use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// Import necessary types and modules.
    use std::collections::BTreeMap; // BTreeMap is a sorted map, useful for managing signers and commitments by their identifiers.

    use frost_ed25519::round1::SigningCommitments; // Represents the nonces (commitments) a signer creates in round 1.
    use frost_ed25519::Identifier; // A unique identifier for each participant in the FROST protocol.
    use frost_ed25519::Signature; // The final, aggregated signature object.

    use old_rand; // Trait for random number generation.

    use roast::coordinator; // The central coordinator module for the ROAST protocol.
    use roast::frost::Frost; // A wrapper or adapter for the underlying FROST implementation.
    use roast::signer; // The signer logic module for the ROAST protocol.
    use roast::signer::RoastSigner; // The state machine for a single participant in ROAST.

fn roast_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("roast");

    // Common setup that is expensive and only needs to be done once.
    let n = 7;
    let t = 5;

    // This benchmark will specifically measure the performance of `generate_with_dealer`.
    group.bench_function("dkg_7_of_5", |b| {
        // The `iter` method runs the closure multiple times to get a reliable measurement.
        b.iter(|| {
            // Create a new RNG for each iteration to ensure a fair measurement.
            let mut rng = old_rand::thread_rng();
            // Call the function to be benchmarked.
            // Wrap it in `black_box` to prevent the compiler from optimizing it away,
            // since we don't use the `(shares, pubkey_package)` result here.
            std::hint::black_box(
                frost_ed25519::keys::generate_with_dealer(
                    n,
                    t,
                    frost_ed25519::keys::IdentifierList::Default,
                    &mut rng,
                )
                .unwrap(),
            );
        });
    });

    // We need a separate RNG for the setup phase.
    let mut setup_rng = old_rand::thread_rng();

    // Simulate a trusted dealer generating keys for a t-of-n FROST instance.
    let (shares, pubkey_package) = frost_ed25519::keys::generate_with_dealer(
        n,
        t,
        frost_ed25519::keys::IdentifierList::Default,
        &mut setup_rng,
    )
    .unwrap();

    // Define the message that the group will sign.
    let message = b"test message";

    // Create a benchmark function for the ROAST protocol.
    group.bench_function("roast_5_of_7", |b| {
        // The `iter` method runs the closure multiple times and measures its performance.
        // Each iteration represents a full run of the ROAST signing protocol.
        b.iter(|| {
            // --- Per-iteration setup ---
            // This setup is part of what we want to measure for a full protocol run.

            // Initialize the FROST protocol helper.
            let frost = Frost::new();
            // Initialize a random number generator for this iteration to ensure independence.
            let mut rng = old_rand::thread_rng();

            // Initialize the ROAST coordinator.
            let mut roast =
                coordinator::Coordinator::new(frost, pubkey_package.clone(), message, t as usize, n as usize);

            // Create a map to hold the state for each signer.
            let mut signers: BTreeMap<Identifier, RoastSigner<_, _>> = BTreeMap::new();
            // Create a map to hold the initial signing commitments (nonces) from each signer.
            let mut commitments: BTreeMap<Identifier, SigningCommitments> = BTreeMap::new();

            // For each participant, create a new RoastSigner instance and their initial commitment.
            for (identifier, secret_share) in shares.clone() {
                let (signer, commitment) = signer::RoastSigner::new(
                    &mut rng,
                    Frost::new(),
                    pubkey_package.clone(),
                    identifier,
                    secret_share,
                    message,
                );
                signers.insert(identifier, signer);
                commitments.insert(identifier, commitment);
            }

            let mut nonce_response: Option<BTreeMap<Identifier, SigningCommitments>> = None;

            // --- ROUND 1: Commitment Exchange ---
            for (id, commitment) in &commitments {
                let response = roast.receive(*id, None, commitment.clone()).unwrap();
                if let Some(nonce_set) = response.nonce_set.clone() {
                    nonce_response = Some(nonce_set);
                }
            }

            let sign_session_nonces = nonce_response.expect("Did not receive enough nonces");
            let mut final_signature: Option<Signature> = None;

            // --- ROUND 2: Signing ---
            for (id, signer) in &mut signers {
                if !sign_session_nonces.iter().any(|(i, _)| i == id) {
                    continue;
                }

                let (sig_share, new_nonce) = signer.sign(&mut rng, sign_session_nonces.clone());
                let response = roast.receive(*id, Some(sig_share), new_nonce).unwrap();

                if let Some(sig) = response.combined_signature {
                    final_signature = Some(sig);
                    break;
                }
            }

            // --- VERIFICATION ---
            let final_sig = final_signature.expect("should have combined signature");
            // Verify the signature to ensure the protocol run was successful.
            // The verification is part of the benchmarked operation.
            assert!(pubkey_package
                .verifying_key()
                .verify(message, &final_sig)
                .is_ok());
        });
    });

    // --- Setup for the verification benchmark ---
    // We need to generate a valid signature once, outside the benchmark loop.
    let final_sig = {
        let frost = Frost::new();
        let mut rng = old_rand::thread_rng();
        let mut roast =
            coordinator::Coordinator::new(frost, pubkey_package.clone(), message, t as usize, n as usize);
        let mut signers: BTreeMap<Identifier, RoastSigner<_, _>> = BTreeMap::new();
        let mut commitments: BTreeMap<Identifier, SigningCommitments> = BTreeMap::new();

        for (identifier, secret_share) in shares.clone() {
            let (signer, commitment) = signer::RoastSigner::new(
                &mut rng,
                Frost::new(),
                pubkey_package.clone(),
                identifier,
                secret_share,
                message,
            );
            signers.insert(identifier, signer);
            commitments.insert(identifier, commitment);
        }

        let mut nonce_response: Option<BTreeMap<Identifier, SigningCommitments>> = None;
        for (id, commitment) in &commitments {
            let response = roast.receive(*id, None, commitment.clone()).unwrap();
            if let Some(nonce_set) = response.nonce_set.clone() {
                nonce_response = Some(nonce_set);
            }
        }

        let sign_session_nonces = nonce_response.expect("Did not receive enough nonces");
        let mut final_signature: Option<Signature> = None;
        for (id, signer) in &mut signers {
            if !sign_session_nonces.iter().any(|(i, _)| i == id) {
                continue;
            }
            let (sig_share, new_nonce) = signer.sign(&mut rng, sign_session_nonces.clone());
            let response = roast.receive(*id, Some(sig_share), new_nonce).unwrap();
            if let Some(sig) = response.combined_signature {
                final_signature = Some(sig);
                break;
            }
        }
        final_signature.expect("should have combined signature")
    };

    // Benchmark for verifying the final signature.
    group.bench_function(BenchmarkId::new("roast_verify", "5_of_7"), |b| {
        b.iter(|| {
            // The verification is the only part being benchmarked here.
            // We use black_box to ensure the compiler doesn't optimize away the verification.
            std::hint::black_box(
                pubkey_package
                    .verifying_key()
                    .verify(message, &final_sig)
                    .is_ok(),
            );
        });
    });
}

fn benchmarks(c: &mut Criterion) {
    roast_bench(c);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);