//! Utilities that are intended to be used in tests but not outside of it.

#![allow(clippy::missing_panics_doc)]

use proto::native::sequencer::v1alpha1::RollupId;
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
            asset::{
                Denom,
                DEFAULT_NATIVE_ASSET_DENOM,
            },
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
    let rollup_id = RollupId::from_unhashed_bytes([b"test_chain_id_", &*suffix].concat());
    let asset = Denom::from_base_denom(DEFAULT_NATIVE_ASSET_DENOM);
    let signed_transaction = UnsignedTransaction {
        nonce: 1,
        actions: vec![
            SequenceAction {
                rollup_id,
                data: [b"hello_world_id_", &*suffix].concat(),
            }
            .into(),
        ],
        fee_asset_id: asset.id(),
    }
    .into_signed(&signing_key);
    let rollup_transactions = proto::native::sequencer::v1alpha1::merge_sequence_actions_in_signed_transaction_transactions_by_rollup_id(&[signed_transaction.clone()]);
    let rollup_transactions_tree =
        proto::native::sequencer::v1alpha1::derive_merkle_tree_from_rollup_txs(
            &rollup_transactions,
        );

    let rollup_ids_root = merkle::Tree::from_leaves(std::iter::once(rollup_id)).root();
    let data = vec![
        rollup_transactions_tree.root().to_vec(),
        rollup_ids_root.to_vec(),
        signed_transaction.into_raw().encode_to_vec(),
    ];
    let data_hash = Some(Hash::Sha256(simple_hash_from_byte_vectors::<sha2::Sha256>(
        &data.iter().map(sha2::Sha256::digest).collect::<Vec<_>>(),
    )));

    let (last_commit_hash, last_commit) = make_test_commit_and_hash();

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
    (calculate_last_commit_hash(&commit), commit)
}

// Calculates the `last_commit_hash` given a Tendermint [`Commit`].
//
// It merkleizes the commit and returns the root. The leaves of the merkle tree
// are the protobuf-encoded [`CommitSig`]s; ie. the signatures that the commit consist of.
//
// See https://github.com/cometbft/cometbft/blob/539985efc7d461668ffb46dff88b3f7bb9275e5a/types/block.go#L922
#[must_use]
fn calculate_last_commit_hash(commit: &tendermint::block::Commit) -> tendermint::Hash {
    use prost::Message as _;
    use tendermint::{
        crypto,
        merkle,
    };
    use tendermint_proto::types::CommitSig;

    let signatures = commit
        .signatures
        .iter()
        .map(|commit_sig| CommitSig::from(commit_sig.clone()).encode_to_vec())
        .collect::<Vec<_>>();
    tendermint::Hash::Sha256(merkle::simple_hash_from_byte_vectors::<
        crypto::default::Sha256,
    >(&signatures))
}
