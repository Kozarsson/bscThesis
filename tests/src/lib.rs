// use multisig::{
//     Committee,
//     SignatureShare,
// };

// use roast::{
//     coordinator::Coordinator,
//     signer::RoastSigner,
//     frost::Frost,
// };

// use rand::{RngCore, CryptoRng, rngs::OsRng, seq::SliceRandom};
// use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_verify_basic_scenarios_roast() {
//         let frost = Frost::new();
//         let mut rng = OsRng;

//         let (frost_key, secret_shares) = frost.simulate_keygen(2, 3, &mut rng);

//         let message = b"test message";
//         let roast = coordinator::Coordinator::new(frost, frost_key.clone(), message, 2, 3);

//         // Create each signer session and create an initial nonce
//         let (mut signer1, nonce1) = signer::RoastSigner::new(
//             &mut rng,
//             Frost::new(),
//             frost_key.clone(),
//             0,
//             secret_shares[0].clone(),
//             message,
//         );
//         let (mut signer2, nonce2) = signer::RoastSigner::new(
//             &mut rng,
//             Frost::new(),
//             frost_key.clone(),
//             1,
//             secret_shares[1].clone(),
//             message,
//         );

//         // Begin with each signer sending a nonce to ROAST, marking these signers as responsive.
//         let response = roast.receive(0, None, nonce1).unwrap();
//         assert!(response.nonce_set.is_none());
//         assert!(response.combined_signature.is_none());

//         let response2 = roast.receive(1, None, nonce2).unwrap();
//         assert!(response2.nonce_set.is_some());

//         // Once ROAST receives the threshold number of nonces, it responds to the group of
//         // responsive signers with a nonce set to the group of responsive signers.
//         assert!(response2.recipients.contains(&0) && response2.recipients.contains(&1));
//         let sign_session_nonces = response2.nonce_set.expect("roast responded with nonces");

//         // The signer signs using this the nonces for this sign session,
//         // and responds to ROAST with a signature share.
//         let (sig_share2, nonce2) = signer2.sign(&mut rng, sign_session_nonces.clone());
//         let response = roast.receive(1, Some(sig_share2), nonce2).unwrap();
//         assert!(response.combined_signature.is_none());

//         // ROAST also sends the nonce set to the other signer, who also signs
//         let (sig_share1, nonce1) = signer1.sign(&mut rng, sign_session_nonces);

//         let response = roast.receive(0, Some(sig_share1), nonce1).unwrap();
//         assert!(response.combined_signature.is_some());

//         // Once the threshold number of signature shares have been received,
//         // ROAST combines the signature shares into the aggregate signature
//         let final_sig = response.combined_signature.expect("should have combined signature");
//         assert!(frost_key.verify(message, &final_sig).is_ok());

//     }

//     #[test]
//     fn test_verify_basic_scenarios_multisig() {
//         // Setup a dummy committee
//         let committee = Committee::new(5); // Example: committee with 5 conceptual members

//         // Setup a dummy message
//         let message = b"Test message content";

//         // Scenario 1: Enough shares to meet the threshold
//         let shares1: Vec<SignatureShare> = vec!
//         [SignatureShare::new(1), 
//         SignatureShare::new(2), 
//         SignatureShare::new(3)
//         ];
//         let threshold1 = 3;
//         let result1 = committee.verify(message, &shares1, threshold1);
//         // println!(
//             "Scenario 1: Message='{}', CertSize={}, Threshold={}, Result={}",
//             String::from_utf8_lossy(message), shares1.len(), threshold1, result1
//         );
//         assert!(result1, "Scenario 1 should pass: verified shares meet threshold.");

//         // Scenario 2: Not enough shares to meet the threshold
//         let shares2: Vec<SignatureShare> = vec![SignatureShare::new(1), SignatureShare::new(2)];
//         let threshold2 = 3;
//         let result2 = committee.verify(message, &shares2, threshold2);
//         // println!(
//             "Scenario 2: Message='{}', CertSize={}, Threshold={}, Result={}",
//             String::from_utf8_lossy(message), shares2.len(), threshold2, result2
//         );
//         assert!(!result2, "Scenario 2 should fail: verified shares below threshold.");

//         // Scenario 3: Empty certificate, threshold > 0
//         let shares_empty: Vec<SignatureShare> = Vec::new();
//         let threshold3 = 1;
//         let result3 = committee.verify(message, &shares_empty, threshold3);
//         // println!(
//             "Scenario 3: Message='{}', CertSize={}, Threshold={}, Result={}",
//             String::from_utf8_lossy(message), shares_empty.len(), threshold3, result3
//         );
//         assert!(!result3, "Scenario 3 should fail: empty certificate, threshold > 0.");

//         // Scenario 4: Empty certificate, threshold = 0
//         // (Assuming 0 verified shares >= threshold 0 is true)
//         let threshold4 = 0;
//         let result4 = committee.verify(message, &shares_empty, threshold4);
//         // println!(
//             "Scenario 4: Message='{}', CertSize={}, Threshold={}, Result={}",
//             String::from_utf8_lossy(message), shares_empty.len(), threshold4, result4
//         );
//         assert!(result4, "Scenario 4 should pass: threshold is 0.");

//         // Scenario 5: Shares provided, threshold = 0
//         let shares5: Vec<SignatureShare> = vec![SignatureShare::new(1)];
//         let threshold5 = 0;
//         let result5 = committee.verify(message, &shares5, threshold5);
//         // println!(
//             "Scenario 5: Message='{}', CertSize={}, Threshold={}, Result={}",
//             String::from_utf8_lossy(message), shares5.len(), threshold5, result5
//         );
//         assert!(result5, "Scenario 5 should pass: threshold is 0, shares provided.");
//     }
// }