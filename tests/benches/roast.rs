use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main, BatchSize};

// Import necessary types and modules.
use std::collections::BTreeMap; // BTreeMap is a sorted map, useful for managing signers and commitments by their identifiers.
use std::fs::File; // Required for file operations
use std::io::{self, Read, BufReader}; // Required for file I/O and buffered reading

use frost_ed25519::round1::SigningCommitments; // Represents the nonces (commitments) a signer creates in round 1.
use frost_ed25519::Identifier; // A unique identifier for each participant in the FROST protocol.
use frost_ed25519::Signature; // The final, aggregated signature object.
use frost_ed25519::keys::PublicKeyPackage; // Required for verification context

use old_rand; // Trait for random number generation.

use roast::coordinator; // The central coordinator module for the ROAST protocol.
use roast::frost::Frost; // A wrapper or adapter for the underlying FROST implementation.
use roast::signer; // The signer logic module for the ROAST protocol.
use roast::signer::RoastSigner; // The state machine for a single participant in ROAST.

use bincode; // Required for deserializing binary signatures
use serde::Deserialize; // Required for deserializing Signature objects

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
    // NOTE: This MUST match the message used in `generate_signatures.rs`
    let message = b"this is a test message for ROAST signature generation";

    // Create a benchmark function for the full ROAST protocol run.
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
                    // Break here as soon as the signature is combined.
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

    // --- NEW BENCHMARK: Measuring only Round 2 until aggregation ---
    group.bench_function(BenchmarkId::new("roast_round2_aggregate", "5_of_7"), |b| {
        b.iter_batched(
            // Setup function: This part runs before each iteration of the routine.
            // It includes all steps up to the completion of Round 1.
            || {
                let frost = Frost::new();
                let mut rng = old_rand::thread_rng(); // Fresh RNG for this setup phase.

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
                        &mut rng, // Use the fresh RNG for signer creation
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

                // ROUND 1: Commitment Exchange
                for (id, commitment) in &commitments {
                    let response = roast.receive(*id, None, commitment.clone()).unwrap();
                    if let Some(nonce_set) = response.nonce_set.clone() {
                        nonce_response = Some(nonce_set);
                    }
                }

                let sign_session_nonces = nonce_response.expect("Did not receive enough nonces");

                // Return the necessary state for the routine.
                // We clone signers here to ensure each iteration gets a distinct set of mutable signers.
                (roast, signers, sign_session_nonces, rng)
            },
            // Routine: This part is actually measured. It performs Round 2 and aggregation.
            |(mut roast, mut signers, sign_session_nonces, mut rng)| {
                let mut final_signature: Option<Signature> = None;

                // --- ROUND 2: Signing ---
                // Iterate through signers and process their signature shares.
                for (id, signer) in &mut signers {
                    // Only consider signers who contributed to the nonce set.
                    if !sign_session_nonces.iter().any(|(i, _)| i == id) {
                        continue;
                    }

                    // Signer generates their partial signature and a new nonce (for next round if applicable).
                    let (sig_share, new_nonce) = signer.sign(&mut rng, sign_session_nonces.clone());
                    
                    // Coordinator receives the signature share.
                    let response = roast.receive(*id, Some(sig_share), new_nonce).unwrap();

                    // Check if the combined signature has been formed.
                    if let Some(sig) = response.combined_signature {
                        final_signature = Some(sig);
                        // Stop immediately once the signature is aggregated, as requested.
                        break;
                    }
                }

                // Use `black_box` to prevent the compiler from optimizing away the result,
                // ensuring the aggregation logic is fully executed and measured.
                std::hint::black_box(final_signature.expect("should have combined signature"));
            },
            // BatchSize::PerIteration ensures the setup function runs for each measured iteration.
            criterion::BatchSize::PerIteration,
        );
    });


    // --- Setup for the VERIFICATION benchmark: Load signatures from file ---
    let file_path = "signatures.bin";
    let loaded_signatures: Vec<Signature> = {
        let file = File::open(file_path).unwrap_or_else(|e| {
            eprintln!("Could not open signatures.bin for verification benchmark: {:?}", e);
            eprintln!("Please ensure 'generate_signatures' has been run to create 'signatures.bin'.");
            std::process::exit(1); // Exit if file cannot be opened
        });

        let mut reader = BufReader::new(file);
        let mut signatures = Vec::new();

        loop {
            let mut len_bytes = [0u8; 8];
            match reader.read_exact(&mut len_bytes) {
                Ok(_) => {
                    let len = u64::from_le_bytes(len_bytes);
                    let mut sig_bytes = vec![0u8; len as usize];
                    reader.read_exact(&mut sig_bytes).unwrap_or_else(|e| {
                        eprintln!("Error reading signature bytes from signatures.bin: {:?}", e);
                        std::process::exit(1); // Exit on read error
                    });
                    let deserialized_sig: Signature = bincode::deserialize(&sig_bytes).unwrap_or_else(|e| {
                        eprintln!("Error deserializing signature from signatures.bin: {:?}", e);
                        std::process::exit(1); // Exit on deserialization error
                    });
                    signatures.push(deserialized_sig);
                },
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break; // Reached end of file
                },
                Err(e) => {
                    eprintln!("An unexpected error occurred while reading signatures.bin: {:?}", e);
                    std::process::exit(1); // Exit on other I/O errors
                }
            }
        }
        if signatures.is_empty() {
            eprintln!("No signatures found in 'signatures.bin'. Please run 'generate_signatures' first.");
            std::process::exit(1); // Exit if no signatures were loaded
        }
        println!("Loaded {} signatures from '{}' for verification benchmark.", signatures.len(), file_path);
        signatures
    };

    // Keep track of the index for cycling through loaded_signatures
    // This `static mut` is generally discouraged but can be used in benchmarks if carefully managed
    // and if parallelism isn't a concern for the counter itself.
    // For Criterion, the benchmark functions are typically run sequentially within a single thread.
    static mut SIG_IDX_COUNTER: usize = 0;


    // Benchmark for verifying the loaded signatures.
    group.bench_function(BenchmarkId::new("roast_verify_from_file", "5_of_7"), |b| {
        b.iter_batched(
            // Setup function: This is called before each batch of iterations.
            // It prepares the data for the routine.
            || {
                // Safely access and increment the static counter.
                let current_sig_index = unsafe {
                    let idx = SIG_IDX_COUNTER;
                    SIG_IDX_COUNTER = (SIG_IDX_COUNTER + 1) % loaded_signatures.len();
                    idx
                };
                // Clone the signature to ensure each verification is independent.
                // This `clone` time is part of the setup for each verification, not the verification itself.
                loaded_signatures[current_sig_index].clone()
            },
            // Routine: This part is actually measured. It performs the verification.
            |sig_to_verify| {
                // The verification is the only part being benchmarked here.
                // `black_box` prevents the compiler from optimizing away the verification call.
                std::hint::black_box(
                    pubkey_package
                        .verifying_key()
                        .verify(message, &sig_to_verify)
                        .is_ok(),
                );
            },
            BatchSize::SmallInput, // BatchSize::SmallInput is appropriate here because the setup (cloning one signature) is minimal.
        );
    });

    group.finish(); // Finish the benchmark group
}

fn benchmarks(c: &mut Criterion) {
    roast_bench(c);
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
