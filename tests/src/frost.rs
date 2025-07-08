use frost::keys::{KeyPackage, PublicKeyPackage};
use frost::round1::{SigningCommitments, SigningNonces};
use frost::round2::SignatureShare;
use frost_ed25519::{self as frost, Identifier, SigningPackage};
use old_rand::{CryptoRng, RngCore};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrostSettings {
    pub system_size: u16,
    pub threshold: u16,
}

impl crate::Settings for FrostSettings {
    fn system_size(&self) -> u16 {
        self.system_size
    }

    fn threshold(&self) -> u16 {
        self.threshold
    }
}

#[derive(Clone, Debug)]
pub struct FrostPackage {
    pub(crate) secret: BTreeMap<Identifier, KeyPackage>,
    pub(crate) public: PublicKeyPackage,
}

impl FrostPackage {
    pub fn secret(&self) -> &BTreeMap<Identifier, KeyPackage> {
        &self.secret
    }
    pub fn public(&self) -> &PublicKeyPackage {
        &self.public
    }
}

pub struct FrostRound1 {
    pub(crate) nonces: BTreeMap<Identifier, SigningNonces>,
    pub(crate) commitments: BTreeMap<Identifier, SigningCommitments>,
}

impl FrostRound1 {
    pub fn nonces(&self) -> &BTreeMap<Identifier, SigningNonces> {
        &self.nonces
    }
    pub fn commitments(&self) -> &BTreeMap<Identifier, SigningCommitments> {
        &self.commitments
    }
}

pub struct FrostRound2 {
    pub(crate) signing_package: SigningPackage,
    pub(crate) signature_shares: BTreeMap<Identifier, SignatureShare>,
}

impl FrostRound2 {
    pub fn signing_package(&self) -> &SigningPackage {
        &self.signing_package
    }
    pub fn signature_shares(&self) -> &BTreeMap<Identifier, SignatureShare> {
        &self.signature_shares
    }
}

pub fn setup<RNG>(settings: &FrostSettings, rng: &mut RNG) -> Result<FrostPackage, frost::Error>
where
    RNG: RngCore + CryptoRng,
{
    let max_signers = settings.system_size;
    let min_signers = settings.threshold;
    let (shares, pubkey_package) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        rng,
    )?;

    // Verifies the secret shares from the dealer and store them in a BTreeMap.
    // In practice, the KeyPackages must be sent to its respective participants
    // through a confidential and authenticated channel.
    let mut key_packages: BTreeMap<_, _> = BTreeMap::new();

    for (identifier, secret_share) in shares {
        // ANCHOR: tkg_verify
        let key_package = frost::keys::KeyPackage::try_from(secret_share)?;
        // ANCHOR_END: tkg_verify
        key_packages.insert(identifier, key_package);
    }
    Ok(FrostPackage {
        secret: key_packages,
        public: pubkey_package,
    })
}

pub fn vote_commitments<RNG>(
    settings: &FrostSettings,
    packages: &FrostPackage,
    rng: &mut RNG,
) -> Result<FrostRound1, frost::Error>
where
    RNG: RngCore + CryptoRng,
{
    let mut nonces_map = BTreeMap::new();
    let mut commitments_map = BTreeMap::new();

    ////////////////////////////////////////////////////////////////////////////
    // Round 1: generating nonces and signing commitments for each participant
    ////////////////////////////////////////////////////////////////////////////

    // In practice, each iteration of this loop will be executed by its respective participant.
    for participant_index in 1..=settings.threshold {
        let participant_identifier = participant_index.try_into().expect("should be nonzero");
        let key_package = &packages.secret[&participant_identifier];
        // Generate one (1) nonce and one SigningCommitments instance for each
        // participant, up to _threshold_.
        // ANCHOR: round1_commit
        let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), rng);
        // ANCHOR_END: round1_commit
        // In practice, the nonces must be kept by the participant to use in the
        // next round, while the commitment must be sent to the coordinator
        // (or to every other participant if there is no coordinator) using
        // an authenticated channel.
        nonces_map.insert(participant_identifier, nonces);
        commitments_map.insert(participant_identifier, commitments);
    }
    let nonces = nonces_map;
    let commitments = commitments_map;
    Ok(FrostRound1 {
        nonces,
        commitments,
    })
}

pub fn sign_message(
    _settings: &FrostSettings,
    packages: &FrostPackage,
    round1: &FrostRound1,
    message: &[u8],
) -> Result<FrostRound2, frost::Error> {
    // This is what the signature aggregator / coordinator needs to do:
    // - decide what message to sign
    // - take one (unused) commitment per signing participant
    let mut signature_shares = BTreeMap::new();
    // ANCHOR: round2_package

    // In practice, the SigningPackage must be sent to all participants
    // involved in the current signing (at least min_signers participants),
    // using an authenticate channel (and confidential if the message is secret).
    let signing_package = frost::SigningPackage::new(round1.commitments.clone(), message);
    // ANCHOR_END: round2_package

    ////////////////////////////////////////////////////////////////////////////
    // Round 2: each participant generates their signature share
    ////////////////////////////////////////////////////////////////////////////

    // In practice, each iteration of this loop will be executed by its respective participant.
    for participant_identifier in round1.nonces.keys() {
        let key_package = &packages.secret[participant_identifier];

        let nonces = &round1.nonces[participant_identifier];

        // Each participant generates their signature share.
        // ANCHOR: round2_sign
        let signature_share = frost::round2::sign(&signing_package, nonces, key_package)?;
        // ANCHOR_END: round2_sign

        // In practice, the signature share must be sent to the Coordinator
        // using an authenticated channel.
        signature_shares.insert(*participant_identifier, signature_share);
    }
    Ok(FrostRound2 {
        signing_package,
        signature_shares,
    })
}

pub fn aggregate_verify(
    _settings: &FrostSettings,
    packages: &FrostPackage,
    _round1: &FrostRound1,
    round2: &FrostRound2,
    message: &[u8],
) -> Result<(), frost::Error> {
    // Aggregate (also verifies the signature shares)
    // ANCHOR: aggregate
    let group_signature = frost::aggregate(
        &round2.signing_package,
        &round2.signature_shares,
        &packages.public,
    )?;
    // ANCHOR_END: aggregate

    // Check that the threshold signature can be verified by the group public
    // key (the verification key).
    // ANCHOR: verify
    let is_signature_valid = packages
        .public
        .verifying_key()
        .verify(message, &group_signature)
        .is_ok();
    // ANCHOR_END: verify
    assert!(is_signature_valid);
    Ok(())
}

pub fn frost_example(max_faulty: u16) -> Result<(), frost::Error> {
    let settings = FrostSettings {
        system_size: 3 * max_faulty + 1,
        threshold: 2 * max_faulty + 1,
    };
    let mut rng = old_rand::thread_rng();

    let package = setup(&settings, &mut rng)?;
    let round1 = vote_commitments(&settings, &package, &mut rng)?;

    let message = b"message to sign";

    let round2 = sign_message(&settings, &package, &round1, message)?;
    aggregate_verify(&settings, &package, &round1, &round2, message)?;

    Ok(())
}
