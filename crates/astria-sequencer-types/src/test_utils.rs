//! Utilities that are intended to be used in tests but not outside of it.

#![allow(clippy::missing_panics_doc)]

use tendermint::block::Header;

#[must_use]
/// Returns a default tendermint block header for test purposes.
pub fn default_header() -> Header {
    use tendermint::{
        account,
        block::{
            header::Version,
            Height,
        },
        chain,
        hash::AppHash,
        Hash,
        Time,
    };

    Header {
        version: Version {
            block: 0,
            app: 0,
        },
        chain_id: chain::Id::try_from("test").unwrap(),
        height: Height::from(1u32),
        time: Time::now(),
        last_block_id: None,
        last_commit_hash: None,
        data_hash: None,
        validators_hash: Hash::Sha256([0; 32]),
        next_validators_hash: Hash::Sha256([0; 32]),
        consensus_hash: Hash::Sha256([0; 32]),
        app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
        last_results_hash: None,
        evidence_hash: None,
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
    }
}

pub fn create_tendermint_block() -> tendermint::Block {
    use proto::{
        native::sequencer::v1alpha1::{
            SequenceAction,
            UnsignedTransaction,
        },
        Message as _,
    };
    use rand::rngs::OsRng;
    use sha2::Digest as _;
    use tendermint::{
        block,
        chain,
        evidence,
        hash::AppHash,
        merkle::simple_hash_from_byte_vectors,
        Hash,
        Time,
    };

    let height = 1u32;

    let signing_key = ed25519_consensus::SigningKey::new(OsRng);
    let public_key: tendermint::crypto::ed25519::VerificationKey =
        signing_key.verification_key().as_ref().try_into().unwrap();
    let proposer_address = tendermint::account::Id::from(public_key);

    let suffix = height.to_string().into_bytes();
    let chain_id = [b"test_chain_id_", &*suffix].concat();
    let signed_tx_bytes = UnsignedTransaction {
        nonce: 1,
        actions: vec![
            SequenceAction {
                chain_id: chain_id.clone(),
                data: [b"hello_world_id_", &*suffix].concat(),
            }
            .into(),
        ],
    }
    .into_signed(&signing_key)
    .into_raw()
    .encode_to_vec();
    let action_tree = merkle::Tree::from_leaves(std::iter::once(&signed_tx_bytes));
    let chain_ids_commitment = merkle::Tree::from_leaves(std::iter::once(chain_id)).root();
    let data = vec![
        action_tree.root().to_vec(),
        chain_ids_commitment.to_vec(),
        signed_tx_bytes,
    ];
    let data_hash = Some(Hash::Sha256(simple_hash_from_byte_vectors::<sha2::Sha256>(
        &data.iter().map(sha2::Sha256::digest).collect::<Vec<_>>(),
    )));

    let (last_commit_hash, last_commit) = crate::test_utils::make_test_commit_and_hash();

    tendermint::Block::new(
        block::Header {
            version: block::header::Version {
                block: 0,
                app: 0,
            },
            chain_id: chain::Id::try_from("test").unwrap(),
            height: block::Height::from(height),
            time: Time::now(),
            last_block_id: None,
            last_commit_hash: (height > 1).then_some(last_commit_hash),
            data_hash,
            validators_hash: Hash::Sha256([0; 32]),
            next_validators_hash: Hash::Sha256([0; 32]),
            consensus_hash: Hash::Sha256([0; 32]),
            app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
            last_results_hash: None,
            evidence_hash: None,
            proposer_address,
        },
        data,
        evidence::List::default(),
        // The first height must not, every height after must contain a last commit
        (height > 1).then_some(last_commit),
    )
    .unwrap()
}

// Returns a tendermint commit and hash for testing purposes.
#[must_use]
pub fn make_test_commit_and_hash() -> (tendermint::Hash, tendermint::block::Commit) {
    let commit = tendermint::block::Commit {
        height: 1u32.into(),
        ..Default::default()
    };
    (crate::calculate_last_commit_hash(&commit), commit)
}
