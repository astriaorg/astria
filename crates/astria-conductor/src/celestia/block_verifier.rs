use std::collections::HashMap;

use celestia_client::CelestiaSequencerBlob;
use color_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use ed25519_consensus::{
    Signature,
    VerificationKey,
};
use prost::Message;
use sequencer_client::{
    tendermint,
    tendermint_rpc,
    Client as _,
    WebSocketClient,
};
use tracing::instrument;

/// `BlockVerifier` is verifying blocks received from celestia.
#[derive(Clone)]
pub(super) struct BlockVerifier {
    pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
}

impl BlockVerifier {
    pub(super) fn new(
        pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
    ) -> Self {
        Self {
            pool,
        }
    }

    #[instrument(skip_all, fields(
        height.in_blob = %blob.height(),
        block_hash.in_blob = %telemetry::display::hex(&blob.block_hash()),
    ))]
    pub(super) async fn verify_blob(&self, blob: &CelestiaSequencerBlob) -> eyre::Result<()> {
        let client =
            self.pool.get().await.wrap_err(
                "failed getting a client from the pool to get the current validator set",
            )?;
        Verify::at_height(&client, blob.height())
            .await
            .wrap_err("failed getting the required objects to verify blob")?
            .verify(blob)
    }
}
struct Verify {
    block_hash: tendermint::Hash,
    commit_header: tendermint::block::signed_header::SignedHeader,
}

impl Verify {
    async fn at_height(
        client: &WebSocketClient,
        height: tendermint::block::Height,
    ) -> eyre::Result<Self> {
        use futures::TryFutureExt as _;
        ensure!(
            height != tendermint::block::Height::from(0u32),
            "cannot validate sequencer blocks at height 0",
        );
        // the validators at height h-1 vote for the block at height h.
        let prev_height: tendermint::block::Height = (height.value() - 1).try_into().wrap_err(
            "failed converting decremented height tendermint value to tendermint height type",
        )?;
        let (block, commit, validator_set) = tokio::try_join!(
            client
                .block(height)
                .map_err(|e| eyre::Report::new(e).wrap_err("failed fetching sequencer block")),
            client
                .commit(height)
                .map_err(|e| eyre::Report::new(e).wrap_err("failed fetching commit")),
            client
                .validators(prev_height, tendermint_rpc::Paging::Default)
                .map_err(|e| eyre::Report::new(e).wrap_err("failed fetching validator set")),
        )?;
        let block_hash = block.block_id.hash;
        let commit_header = commit.signed_header;
        Self::new(block_hash, commit_header, validator_set)
    }

    fn new(
        block_hash: tendermint::Hash,
        commit_header: tendermint::block::signed_header::SignedHeader,
        validator_set: tendermint_rpc::endpoint::validators::Response,
    ) -> eyre::Result<Self> {
        ensure!(
            block_hash == commit_header.header.hash(),
            "block hash stored in commit header does not match block hash of sequencer block",
        );
        ensure_commit_has_quorum(
            &commit_header.commit,
            &validator_set,
            &commit_header.header.chain_id,
        )
        .wrap_err("unable to verify that commit had quorum")?;
        Ok(Self {
            block_hash,
            commit_header,
        })
    }

    fn verify(&self, blob: &CelestiaSequencerBlob) -> eyre::Result<()> {
        ensure!(
            &self.commit_header.header.chain_id == blob.cometbft_chain_id(),
            "expected cometbft chain ID `{}`, got {}",
            self.commit_header.header.chain_id,
            blob.cometbft_chain_id(),
        );
        ensure!(
            blob.block_hash() == self.block_hash.as_bytes(),
            "block hash in blob does not match block hash of sequencer block",
        );

        Ok(())
    }
}

/// This function ensures that the given Commit has quorum, ie that the Commit contains >2/3 voting
/// power. It performs the following checks:
/// - the height of the commit matches the block height of the validator set
/// - each validator in the commit is in the validator set
/// - for each signature in the commit, the validator public key matches the validator address in
///   the commit
/// - for each signature in the commit, the validator signature in the commit is valid
/// - the total voting power of the commit is >2/3 of the total voting power of the validator set
///
/// # Errors
///
/// If any of the above conditions are not satisfied, an error is returned.
#[instrument]
fn ensure_commit_has_quorum(
    commit: &tendermint::block::Commit,
    validator_set: &tendermint_rpc::endpoint::validators::Response,
    chain_id: &tendermint::chain::Id,
) -> eyre::Result<()> {
    // Validator set at Block N-1 is used for block N
    let expected_height = validator_set.block_height.increment();
    let actual_height = commit.height;
    ensure!(
        expected_height == actual_height,
        "commit height mismatch; expected `{expected_height}`, got `{actual_height}`"
    );

    let Some(total_voting_power) = validator_set
        .validators
        .iter()
        .try_fold(0u64, |acc, validator| acc.checked_add(validator.power()))
    else {
        bail!("total voting power exceeded u64:MAX");
    };

    let validator_map = validator_set
        .validators
        .iter()
        .map(|v| {
            let address = tendermint::account::Id::from(v.pub_key);
            (address, v)
        })
        .collect::<HashMap<_, _>>();

    let mut commit_voting_power = 0u64;
    for vote in &commit.signatures {
        // we only care about votes that are for the Commit.BlockId (ignore absent validators and
        // votes for nil)
        let tendermint::block::CommitSig::BlockIdFlagCommit {
            validator_address,
            signature,
            timestamp,
        } = vote
        else {
            continue;
        };

        let Some(signature) = signature else {
            bail!(
                "signature should not be empty for commit with validator {}",
                validator_address
            )
        };

        // verify validator exists in validator set
        let Some(validator) = validator_map.get(validator_address) else {
            bail!("validator {} not found in validator set", validator_address);
        };

        // verify address in signature matches validator pubkey
        let address_from_pubkey = tendermint::account::Id::from(validator.pub_key);

        ensure!(
            &address_from_pubkey == validator_address,
            format!(
                "validator address mismatch: expected {}, got {}",
                validator_address, address_from_pubkey
            )
        );

        // verify vote signature
        verify_vote_signature(
            *timestamp,
            commit,
            chain_id,
            &validator.pub_key,
            signature.as_bytes(),
        )
        .wrap_err("failed to verify vote signature")?;

        commit_voting_power += validator.power();
    }

    ensure!(
        commit_voting_power <= total_voting_power,
        format!(
            "commit voting power is greater than total voting power: {} > {}",
            commit_voting_power, total_voting_power
        )
    );

    ensure!(
        does_commit_voting_power_have_quorum(commit_voting_power, total_voting_power),
        format!(
            "commit voting power is less than 2/3 of total voting power: {} <= {}",
            commit_voting_power,
            total_voting_power * 2 / 3,
        )
    );

    Ok(())
}

fn does_commit_voting_power_have_quorum(commited: u64, total: u64) -> bool {
    if total < 3 {
        return commited * 3 > total * 2;
    }

    commited > total / 3 * 2
}

// see https://github.com/tendermint/tendermint/blob/35581cf54ec436b8c37fabb43fdaa3f48339a170/types/vote.go#L147
fn verify_vote_signature(
    timestamp: tendermint::Time,
    commit: &tendermint::block::Commit,
    chain_id: &tendermint::chain::Id,
    public_key: &tendermint::PublicKey,
    signature_bytes: &[u8],
) -> eyre::Result<()> {
    let public_key = VerificationKey::try_from(public_key.to_bytes().as_slice())
        .wrap_err("failed to create public key from vote")?;
    let signature =
        Signature::try_from(signature_bytes).wrap_err("failed to create signature from vote")?;

    let canonical_vote = tendermint::vote::CanonicalVote {
        vote_type: tendermint::vote::Type::Precommit,
        height: commit.height,
        round: commit.round,
        block_id: Some(tendermint::block::Id {
            hash: commit.block_id.hash,
            part_set_header: commit.block_id.part_set_header,
        }),
        timestamp: Some(timestamp),
        chain_id: chain_id.clone(),
    };

    public_key
        .verify(
            &signature,
            &tendermint_proto::types::CanonicalVote::try_from(canonical_vote)
                .wrap_err("failed to turn commit canonical vote into proto type")?
                .encode_length_delimited_to_vec(),
        )
        .wrap_err("failed to verify vote signature")?;
    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use astria_core::sequencer::v1alpha1::{
        celestia::UncheckedCelestiaSequencerBlob,
        test_utils::make_cometbft_block,
        RollupId,
    };
    use prost::Message as _;
    use sequencer_client::tendermint_rpc::endpoint::validators;
    // use sequencer_types::ChainId;
    use tendermint::{
        account,
        block::Commit,
        validator,
        validator::Info as Validator,
        Hash,
    };

    use super::ensure_commit_has_quorum;
    use crate::celestia::block_verifier::does_commit_voting_power_have_quorum;

    /// Constructs a `[merkle::Tree]` from an iterator yielding byte slices.
    ///
    /// This hashes each item before pushing it into the Merkle Tree, which
    /// effectively causes a double hashing. The leaf hash of an item `d_i`
    /// is then `MTH(d_i) = SHA256(0x00 || SHA256(d_i))`.
    fn merkle_tree_from_transactions<I, B>(iter: I) -> merkle::Tree
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        use sha2::{
            Digest as _,
            Sha256,
        };
        merkle::Tree::from_leaves(iter.into_iter().map(|item| Sha256::digest(&item)))
    }

    fn make_test_validator_set_and_commit(
        height: u32,
        chain_id: tendermint::chain::Id,
    ) -> (validators::Response, account::Id, Commit) {
        use rand::rngs::OsRng;

        let signing_key = ed25519_consensus::SigningKey::new(OsRng);
        let pub_key = tendermint::public_key::PublicKey::from_raw_ed25519(
            signing_key.verification_key().as_ref(),
        )
        .unwrap();
        let address = tendermint::account::Id::from(pub_key);

        let validator = validator::Info {
            address,
            pub_key,
            power: 10u32.into(),
            proposer_priority: 0.into(),
            name: None,
        };

        let round = 0u16;
        let timestamp = tendermint::Time::unix_epoch();
        let canonical_vote = tendermint::vote::CanonicalVote {
            vote_type: tendermint::vote::Type::Precommit,
            height: height.into(),
            round: round.into(),
            block_id: None,
            timestamp: Some(timestamp),
            chain_id,
        };

        let message = tendermint_proto::types::CanonicalVote::try_from(canonical_vote)
            .unwrap()
            .encode_length_delimited_to_vec();

        let signature = signing_key.sign(&message);

        let commit = tendermint::block::Commit {
            height: height.into(),
            round: round.into(),
            signatures: vec![tendermint::block::CommitSig::BlockIdFlagCommit {
                validator_address: address,
                timestamp,
                signature: Some(signature.into()),
            }],
            ..Default::default()
        };

        (
            validators::Response::new((height - 1).into(), vec![validator], 1),
            address,
            commit,
        )
    }

    #[test]
    fn validate_sequencer_blob_last_commit_none_ok() {
        let rollup_transactions_root = merkle::Tree::from_leaves([[1, 2, 3], [4, 5, 6]]).root();
        let chain_ids_commitment = merkle::Tree::new().root();

        let tree = merkle_tree_from_transactions([rollup_transactions_root, chain_ids_commitment]);
        let data_hash = tree.root();
        let rollup_transactions_proof = tree.construct_proof(0).unwrap();
        let rollup_ids_proof = tree.construct_proof(1).unwrap();

        let mut header = make_cometbft_block().header;
        let height = header.height.value().try_into().unwrap();
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let (validator_set, proposer_address, commit) =
            make_test_validator_set_and_commit(height, header.chain_id.clone());
        header.proposer_address = proposer_address;
        let sequencer_blob = UncheckedCelestiaSequencerBlob {
            header,
            rollup_ids: vec![],
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
        .try_into_celestia_sequencer_blob()
        .unwrap();

        ensure_commit_has_quorum(&commit, &validator_set, sequencer_blob.cometbft_chain_id())
            .unwrap();
    }

    #[tokio::test]
    async fn validate_sequencer_blob_with_chain_ids() {
        let test_tx = b"test-tx".to_vec();
        let rollup_id = RollupId::from_unhashed_bytes(b"test-chain");
        let grouped_txs = BTreeMap::from([(rollup_id, vec![test_tx.clone()])]);
        let rollup_transactions_tree =
            astria_core::sequencer::v1alpha1::derive_merkle_tree_from_rollup_txs(&grouped_txs);
        let rollup_transactions_root = rollup_transactions_tree.root();
        let rollup_ids_root = merkle::Tree::from_leaves(std::iter::once(rollup_id)).root();

        let tree = merkle_tree_from_transactions([rollup_transactions_root, rollup_ids_root]);
        let data_hash = tree.root();
        let rollup_transactions_proof = tree.construct_proof(0).unwrap();
        let rollup_ids_proof = tree.construct_proof(1).unwrap();

        let mut header = make_cometbft_block().header;
        let height = header.height.value().try_into().unwrap();
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let (validator_set, proposer_address, commit) =
            make_test_validator_set_and_commit(height, header.chain_id.clone());
        header.proposer_address = proposer_address;

        let sequencer_blob = UncheckedCelestiaSequencerBlob {
            header,
            rollup_ids: vec![rollup_id],
            rollup_transactions_root,
            rollup_transactions_proof,
            rollup_ids_proof,
        }
        .try_into_celestia_sequencer_blob()
        .unwrap();

        ensure_commit_has_quorum(&commit, &validator_set, sequencer_blob.cometbft_chain_id())
            .unwrap();
    }

    #[test]
    fn test_does_commit_voting_power_have_quorum() {
        assert!(does_commit_voting_power_have_quorum(3, 4));
        assert!(does_commit_voting_power_have_quorum(101, 150));
        assert!(does_commit_voting_power_have_quorum(
            u64::MAX / 3,
            u64::MAX / 3
        ));
        assert!(does_commit_voting_power_have_quorum(
            u64::MAX / 3,
            u64::MAX / 2 - 1
        ));
        assert!(does_commit_voting_power_have_quorum(u64::MAX, u64::MAX));

        assert!(!does_commit_voting_power_have_quorum(0, 1));
        assert!(!does_commit_voting_power_have_quorum(1, 2));
        assert!(!does_commit_voting_power_have_quorum(2, 3));
        assert!(!does_commit_voting_power_have_quorum(100, 150));
        assert!(!does_commit_voting_power_have_quorum(
            u64::MAX / 3 - 1,
            u64::MAX / 2
        ));
        assert!(does_commit_voting_power_have_quorum(
            u64::MAX / 3,
            u64::MAX / 2
        ));
        assert!(!does_commit_voting_power_have_quorum(0, 0));
    }

    #[test]
    fn ensure_commit_has_quorum_ok() {
        // these values were retrieved by running the sequencer node and requesting the following:
        // curl http://localhost:26657/validators
        // curl http://localhost:26657/commit?height=79
        let validator_set_str = r#"{
            "block_height":"78",
            "validators":[
                {
                    "address":"D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9",
                    "pub_key":{"type":"tendermint/PubKeyEd25519", "value": "tyPnz5GGblrx3PBjQRxZOHbzsPEI1E8lOh62QoPSWLw="},
                    "voting_power":"10",
                    "proposer_priority":"0"
                }
            ],
            "count":"1",
            "total":"1"
        }"#;
        let commit_str = r#"{
            "height":"79",
            "round":0,
            "block_id":{
                "hash": "74BD4E7F7EF902A84D55589F2AA60B332F1C2F34DDE7652C80BFEB8E7471B1DA",
                "parts":{
                    "total":1,
                    "hash":"7632FFB5D84C3A64279BC9EA86992418ED23832C66E0C3504B7025A9AF42C8C4"
                }
            },
            "signatures":[
                {
                    "block_id_flag":2,
                    "validator_address":"D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9",
                    "timestamp": "2023-07-05T19:02:55.206600022Z",
                    "signature": "qy9vEjqSrF+8sD0K0IAXA398xN1s3QI2rBBDbBMWf0rw0L+B9Z92DZEptf6bPYWuKUFdEc0QFKhUMQA8HjBaAw=="
                }
            ]
        }"#;
        let validator_set =
            serde_json::from_str::<validators::Response>(validator_set_str).unwrap();
        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        ensure_commit_has_quorum(
            &commit,
            &validator_set,
            &tendermint::chain::Id::try_from("test-chain-g3ejvw").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn ensure_commit_has_quorum_not_ok() {
        use base64::engine::{
            general_purpose::STANDARD,
            Engine as _,
        };
        let validator_set = validators::Response::new(
            78u32.into(),
            vec![Validator {
                name: None,
                address: "D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9"
                    .parse::<tendermint::account::Id>()
                    .unwrap(),
                pub_key: tendermint::PublicKey::from_raw_ed25519(
                    &STANDARD
                        .decode("tyPnz5GGblrx3PBjQRxZOHbzsPEI1E8lOh62QoPSWLw=")
                        .unwrap(),
                )
                .unwrap(),
                power: 10u32.into(),
                proposer_priority: 0.into(),
            }],
            1,
        );

        let commit = Commit {
            height: 79u32.into(),
            round: 0u16.into(),
            block_id: tendermint::block::Id {
                hash: "74BD4E7F7EF902A84D55589F2AA60B332F1C2F34DDE7652C80BFEB8E7471B1DA"
                    .parse::<Hash>()
                    .unwrap(),
                part_set_header: tendermint::block::parts::Header::new(
                    1,
                    "7632FFB5D84C3A64279BC9EA86992418ED23832C66E0C3504B7025A9AF42C8C4"
                        .parse::<Hash>()
                        .unwrap(),
                )
                .unwrap(),
            },
            signatures: vec![],
        };

        let result = ensure_commit_has_quorum(
            &commit,
            &validator_set,
            &tendermint::chain::Id::try_from("test-chain-g3ejvw").unwrap(),
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("commit voting power is less than 2/3 of total voting power")
        );
    }
}
