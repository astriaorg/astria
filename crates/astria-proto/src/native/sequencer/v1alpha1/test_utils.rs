//! Utilities to create objects used in various tests of the Astria codebase.

use prost::Message as _;

use super::{
    // FIXME(github....): bring these assets under v1alpha1
    super::asset::default_native_asset_id,
    derive_merkle_tree_from_rollup_txs,
    merge_sequence_actions_in_signed_transaction_transactions_by_rollup_id,
    RollupId,
    SequenceAction,
    UnsignedTransaction,
};

pub fn make_cometbft_block() -> tendermint::Block {
    use rand::rngs::OsRng;
    let height = 1;
    let signing_key = ed25519_consensus::SigningKey::new(OsRng);
    ConfigureCometBftBlock {
        height,
        signing_key,
        proposer_address: None,
    }
    .make()
}

pub struct ConfigureCometBftBlock {
    pub height: u32,
    pub proposer_address: Option<tendermint::account::Id>,
    pub signing_key: ed25519_consensus::SigningKey,
}

impl ConfigureCometBftBlock {
    pub fn make(self) -> tendermint::Block {
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
        let Self {
            height,
            signing_key,
            proposer_address,
        } = self;

        let proposer_address = proposer_address.unwrap_or_else(|| {
            let public_key: tendermint::crypto::ed25519::VerificationKey =
                signing_key.verification_key().as_ref().try_into().unwrap();
            tendermint::account::Id::from(public_key)
        });

        let suffix = height.to_string().into_bytes();
        let rollup_id = RollupId::from_unhashed_bytes([b"test_chain_id_", &*suffix].concat());
        let unsigned_transaction = UnsignedTransaction {
            nonce: 1,
            actions: vec![
                SequenceAction {
                    rollup_id,
                    transactions: vec![[b"hello_world_id_", &*suffix].concat()],
                }
                .into(),
            ],
            fee_asset_id: default_native_asset_id(),
        };

        let signed_transaction = unsigned_transaction.into_signed(&signing_key);
        let rollup_transactions =
            merge_sequence_actions_in_signed_transaction_transactions_by_rollup_id(&[
                signed_transaction.clone(),
            ]);
        let rollup_transactions_tree = derive_merkle_tree_from_rollup_txs(&rollup_transactions);

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
