use criterion::{criterion_group, criterion_main, Criterion};


use std::collections::BTreeMap; 
use old_rand;
use multisig::{Committee, KeypairShare, Signer};
use thesis::frost;
use std::mem;

const SYSTEM_SIZE: usize = 30;
const THRESHOLD: usize = (2 * SYSTEM_SIZE + 1 + 2) / 3;

const MESSAGE: &[u8] = b"HELLO WORLD"; 



fn multisig_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("multisig");
    group.sampling_mode(criterion::SamplingMode::Flat);

    // --- 1. Benchmark: initialisation (Key Generation and Committee Creation) ---
    // This measures the time to generate all participant key shares and build the committee.
    group.bench_function("multisig_initialisation", |b| {
        b.iter(|| {
            // Key generation for all participants
            let participants = (0..SYSTEM_SIZE).map(|_| KeypairShare::default()).collect::<Vec<_>>();
            // Committee creation and adding all verifying shares
            let mut committee = Committee::new();
            for share in participants.iter().map(|keypair| keypair.verifying_share.clone()) {
                committee.add_key(share.clone());
            }
        });
    });

    // --- Setup for subsequent multisig benchmarks ---
    // Generate participants and committee once for use across signing and verification benchmarks.
    let participants: Vec<KeypairShare> = (0..SYSTEM_SIZE).map(|_| KeypairShare::default()).collect();
    let mut committee_builder = Committee::new();
    for share in participants.iter().map(|keypair| keypair.verifying_share.clone()) {
        committee_builder.add_key(share.clone());
    }
    let committee = committee_builder;

    // --- 2. Benchmark: Signing (Single Signer) ---
    // This measures the time for one individual signer to create their signature share.
    let single_signer = &participants[0];
    group.bench_function("multisig_signing", |b| {
        b.iter(|| {
            single_signer.sign(MESSAGE);
        });
    });

    // --- Setup for Verification Benchmark ---
    // Generate a certificate once for the verification benchmark.
    let certificate = participants
        .iter()
        .take(THRESHOLD)
        .map(|keypair| keypair.sign(MESSAGE))
        .collect::<Vec<_>>();

    // --- 4. Benchmark: Verifying the final signature ---
    // This measures the time to verify the collected certificate using the committee's public keys.
    group.bench_function("multisig_verify", |b| {
        b.iter(|| {
            committee.verify(MESSAGE, &certificate, THRESHOLD);
        });
    });

    let mut total_multisig_cert_size = 0;
    if !certificate.is_empty() {
        for sig_share in &certificate {
            total_multisig_cert_size += mem::size_of_val(sig_share);
        }
        println!("Multisig: Total size of certificate ({} shares): {} bytes", certificate.len(), total_multisig_cert_size);
    } else {
         println!("Multisig: Certificate is empty, cannot determine size.");
    }


    group.finish();
}

fn frost_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("frost");
    group.sampling_mode(criterion::SamplingMode::Flat);

    // A single FROST configuration is used for all benchmarks.
    let settings = frost::FrostSettings {
        system_size: SYSTEM_SIZE as u16,
        threshold: THRESHOLD as u16,
    };
    let message = MESSAGE;
    let mut rng = old_rand::thread_rng();

    // 1. Benchmark: FROST Setup (Distributed Key Generation (DKG))
    group.bench_function("frost_initialisation", |b| {
        b.iter(|| {
            let mut iter_rng = old_rand::thread_rng();
            frost::setup(&settings, &mut iter_rng).unwrap();
        });
    });

    // Create a package once to be used as input for the next benchmark.
    let package = frost::setup(&settings, &mut rng).unwrap();

    // // 2. Benchmark: FROST Commitments (Round 1)
    // group.bench_function("commitments", |b| {
    //     b.iter(|| {
    //         let mut iter_rng = old_rand::thread_rng();
    //         frost::vote_commitments(&settings, &package, &mut iter_rng).unwrap();
    //     });
    // });

    // Create round 1 data to be used as input for the signing benchmark.
    let round1 = frost::vote_commitments(&settings, &package, &mut rng).unwrap();

    // --- FROST: Single participant signing benchmark ---
    let participant_identifier = frost_ed25519::Identifier::try_from(1u16).unwrap();
    let key_package = &package.secret()[&participant_identifier];
    let nonces = &round1.nonces()[&participant_identifier];
    let signing_package = frost_ed25519::SigningPackage::new(round1.commitments().clone(), message);

    // 2. Benchmark: FROST Sign (Round 2 for a single participant)
    group.bench_function("frost_signing", |b| {
        b.iter(|| {
            frost_ed25519::round2::sign(&signing_package, nonces, key_package).unwrap()
        });
    });

    // // 3. Benchmark: FROST Sign (Round 2 + Aggregation)
    // group.bench_function("sign", |b| {
    //     b.iter(|| {
    //         frost::sign_message(&settings, &package, &round1, message).unwrap();
    //     });
    // });

    // Prepare signature shares once for use in the aggregation benchmark
    let signing_package = frost_ed25519::SigningPackage::new(round1.commitments().clone(), message);
    let signature_shares: BTreeMap<_, _> = round1.nonces().iter().map(|(id, nonces)| {
        let key_package = &package.secret()[id];
        let sig_share = frost_ed25519::round2::sign(&signing_package, nonces, key_package).unwrap();
        (*id, sig_share)
    }).collect();

    // 3. Benchmark: FROST Aggregation (by one leader)
    group.bench_function("frost_aggregation", |b| {
        b.iter(|| {
            let group_signature = frost_ed25519::aggregate(
                &signing_package,
                &signature_shares,
                package.public(),
            ).unwrap();
        });
    });

    // Prepare the group signature once 
    let group_signature = frost_ed25519::aggregate(
        &signing_package,
        &signature_shares,
        package.public(),
    ).unwrap();
    println!("FROST: Total size of signature: {} bytes", mem::size_of_val(&group_signature));

    // 4. Benchmark: FROST Verification (of the aggregated signature)
    group.bench_function("frost_verify", |b| {
        b.iter(|| {
            assert!(package.public().verifying_key().verify(message, &group_signature).is_ok());
        });
    });

    group.finish();
}

fn benchmarks(c: &mut Criterion) {
    multisig_bench(c);
    frost_bench(c);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);