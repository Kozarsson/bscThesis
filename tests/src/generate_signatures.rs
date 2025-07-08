use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write; // Required for the `write_all` and `writeln` methods

use frost_ed25519::round1::SigningCommitments; // Represents the nonces (commitments) a signer creates in round 1.
use frost_ed25519::Identifier; // A unique identifier for each participant in the FROST protocol.
use frost_ed25519::Signature; // The final, aggregated signature object.

use old_rand::thread_rng; // A cryptographically secure random number generator.

use roast::coordinator; // The central coordinator module for the ROAST protocol.
use roast::frost::Frost; // A wrapper or adapter for the underlying FROST implementation.
use roast::signer; // The signer logic module for the ROAST protocol.
use roast::signer::RoastSigner; // The state machine for a single participant in ROAST.

/// This program generates 100 FROST/ROAST signatures and saves them to a file.
///
/// It performs the following steps:
/// 1. Initializes the FROST parameters (t-of-n).
/// 2. Generates key shares for participants using a trusted dealer (this is done once).
/// 3. For each of 100 iterations:
///    a. Initializes a new ROAST coordinator and signers.
///    b. Runs Round 1 of the FROST protocol (commitment exchange).
///    c. Runs Round 2 of the FROST protocol (signature share generation and aggregation).
///    d. Verifies the generated signature (optional, but good for sanity checks).
///    e. Converts the final signature to a hex string and stores it.
/// 4. Writes all generated hex-encoded signatures to `signatures.txt`, one per line.
fn main() {
    // Define the number of signatures to generate.
    const NUM_SIGNATURES: usize = 100;

    // Define the FROST threshold parameters:
    // `n`: total number of participants
    // `t`: minimum number of participants required to sign
    let n = 31;
    let t = 21;

    // Define the message that the group will collectively sign.
    // This message remains constant across all signature generations.
    let message = b"Hello, world!";

    // --- Key Generation Phase (Performed only once) ---
    // Simulate a trusted dealer generating keys for a t-of-n FROST instance.
    // `shares` contains the secret shares for each participant.
    // `pubkey_package` contains the group public key and participant public keys.
    let mut setup_rng = thread_rng(); // RNG for key generation.
    let (shares, pubkey_package) = frost_ed25519::keys::generate_with_dealer(
        n,
        t,
        frost_ed25519::keys::IdentifierList::Default, // Default identifiers (1, 2, 3...)
        &mut setup_rng,
    )
    .unwrap_or_else(|e| {
        eprintln!("Failed to generate FROST keys: {:?}", e);
        std::process::exit(1);
    });
    println!("FROST keys generated for {} participants with threshold {}.", n, t);

    // Vector to store the hex-encoded strings of the generated signatures.
     let mut serialized_signatures: Vec<Vec<u8>> = Vec::with_capacity(NUM_SIGNATURES);

    // --- Signature Generation Loop (NUM_SIGNATURES times) ---
    for i in 0..NUM_SIGNATURES {
        println!("Generating signature {}/{}", i + 1, NUM_SIGNATURES);

        // Initialize FROST and a new random number generator for this signature generation.
        // It's important to use a new RNG or re-seed for each signature to ensure
        // fresh nonces and prevent signature reuse attacks.
        let frost = Frost::new();
        let mut rng = thread_rng();

        // Initialize the ROAST coordinator for this signing session.
        // The coordinator orchestrates the signing process.
        let mut roast = coordinator::Coordinator::new(
            frost,
            pubkey_package.clone(), // Clone the public key package for each new session.
            message,
            t as usize,
            n as usize,
        );

        // BTreeMaps to hold the state of each signer and their initial commitments.
        let mut signers: BTreeMap<Identifier, RoastSigner<_, _>> = BTreeMap::new();
        let mut commitments: BTreeMap<Identifier, SigningCommitments> = BTreeMap::new();

        // Create a RoastSigner instance for each participant and their initial commitments.
        // Each signer holds their secret share and generates nonces (commitments) for Round 1.
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

        /// --- ROUND 1: Commitment Exchange ---
        // Simulate each signer sending their initial commitment to the coordinator.
        for (id, commitment) in &commitments {
            // The coordinator receives the commitment. `None` is passed for the signature share because this is the first round.
            let response = roast.receive(*id, None, commitment.clone()).unwrap();

            // The coordinator's response will include a `nonce_set` once it has received `t` commitments.
            if let Some(nonce_set) = response.nonce_set.clone() {
                // If the nonce_set is present, it means the threshold has been met. Store it.
                nonce_response = Some(nonce_set);
            }
        }

        let sign_session_nonces = nonce_response.expect("Did not receive enough nonces to proceed to Round 2.");
        let mut final_signature: Option<Signature> = None;

        // --- ROUND 2: Signing ---
        // Simulate each of the `t` responsive signers creating and sending their partial signatures.
        for (id, signer) in &mut signers {
            // Check if the current signer was part of the group that the coordinator selected for this signing session.
            if !sign_session_nonces.iter().any(|(i, _)| i == id) {
                // If not, this signer was not in the first `t` to respond, so they skip this round.
                continue;
            }

            // The signer uses the nonce set from the coordinator to generate their partial signature.
            let (sig_share, new_nonce) = signer.sign(&mut rng, sign_session_nonces.clone());
            // println!("make partial sig {:?}", id); // Log that a partial signature was created.
                                                   // The signer sends their partial signature (`sig_share`) and a new commitment (`new_nonce`) for a potential future round to the coordinator.
            let response = roast.receive(*id, Some(sig_share), new_nonce).unwrap();

            // The coordinator's response will include the final `combined_signature` once it has `t` partial signatures.
            if let Some(sig) = response.combined_signature {
                // If the final signature is present, store it.
                final_signature = Some(sig);
                // The signature is complete, so we can stop the process.
                break;
            }
        }

        let final_sig = final_signature.expect("Failed to combine signature: Not enough valid shares received.");

        // --- Verification (Optional, but highly recommended for sanity checking) ---
        // Verify the aggregated signature against the group public key and message.
        assert!(pubkey_package
            .verifying_key()
            .verify(message, &final_sig)
            .is_ok(), "Signature verification failed for signature {}!", i + 1);

        // Convert the `Signature` object to its byte representation, then hex-encode it.
        // This makes it easy to save to a text file.
        //generated_signatures.push(hex::encode(final_sig.as_bytes()));

        // Serialize the Signature object directly using bincode.
        let encoded_sig = bincode::serialize(&final_sig).unwrap_or_else(|e| {
            eprintln!("Failed to serialize signature {}: {:?}", i + 1, e);
            std::process::exit(1);
        });
        serialized_signatures.push(encoded_sig);
    }

    // --- Save Signatures to File ---
    let file_path = "signatures.bin"; // Changed file extension to .bin for binary data
    let mut file = File::create(file_path).unwrap_or_else(|e| {
        eprintln!("Could not create file '{}': {:?}", file_path, e);
        std::process::exit(1);
    });

    for sig_bytes in serialized_signatures {
        // For bincode, it's common to write the length of the serialized data first,
        // then the data itself, to make deserialization easier later.
        let len = sig_bytes.len() as u64;
        file.write_all(&len.to_le_bytes()).unwrap_or_else(|e| {
            eprintln!("Could not write length to file: {:?}", e);
            std::process::exit(1);
        });
        file.write_all(&sig_bytes).unwrap_or_else(|e| {
            eprintln!("Could not write signature bytes to file: {:?}", e);
            std::process::exit(1);
        });
    }

    println!("\nSuccessfully generated {} signatures and saved them to '{}' in binary format.", NUM_SIGNATURES, file_path);
    println!("To deserialize and read these signatures, you would typically use `bincode::deserialize_from`.");
}