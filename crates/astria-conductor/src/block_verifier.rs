use std::collections::HashMap;

use astria_sequencer_types::calculate_last_commit_hash;
use celestia_client::{
    RollupNamespaceData,
    SequencerNamespaceData,
    SignedNamespaceData,
};
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
    tendermint_rpc::endpoint::validators,
    Client as _,
};
use tendermint::{
    account,
    block,
    chain,
    validator::Info as Validator,
    vote::{
        self,
        CanonicalVote,
    },
};
use tracing::instrument;

/// `BlockVerifier` is verifying blocks received from celestia.
#[derive(Clone)]
pub(crate) struct BlockVerifier {
    pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
}

impl BlockVerifier {
    pub(crate) fn new(
        pool: deadpool::managed::Pool<crate::client_provider::ClientProvider>,
    ) -> Self {
        Self {
            pool,
        }
    }

    /// validates `SignedNamespaceData` received from Celestia.
    /// This function verifies the block signature and checks that the data
    /// was signed by the expected proposer for this block height.
    pub(crate) async fn validate_signed_namespace_data(
        &self,
        data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> eyre::Result<()> {
        // get validator set for this height
        let height: u32 = data.data().header.height.value().try_into().expect(
            "a tendermint height (currently non-negative i32) should always fit into a u32",
        );
        let current_validator_set = self
            .pool
            .get()
            .await
            .wrap_err("failed getting a client from the pool to get the current validator set")?
            .validators(height, sequencer_client::tendermint_rpc::Paging::Default)
            .await
            .wrap_err("failed to get validator set")?;

        validate_signed_namespace_data(&current_validator_set, data)
            .wrap_err("failed validating signed namespace data")?;

        // get validator set for the previous height, as the commit contained
        // in the block is for the previous height
        let parent_validator_set = self
            .pool
            .get()
            .await
            .wrap_err("failed getting a client from the pool to get the previous validator set")?
            .validators(
                height - 1,
                sequencer_client::tendermint_rpc::Paging::Default,
            )
            .await
            .wrap_err("failed to get validator set")?;

        validate_sequencer_namespace_data(
            &current_validator_set,
            &parent_validator_set,
            data.data(),
        )
        .wrap_err("failed validating sequencer data inside signed namespace data")
    }
}

pub(crate) fn validate_rollup_data(
    rollup_data: &RollupNamespaceData,
    action_tree_root: [u8; 32],
) -> eyre::Result<()> {
    rollup_data
        .verify_inclusion_proof(action_tree_root)
        .wrap_err("failed to verify rollup data inclusion proof")
}

fn validate_signed_namespace_data(
    validator_set: &validators::Response,
    data: &SignedNamespaceData<SequencerNamespaceData>,
) -> eyre::Result<()> {
    // find proposer address for this height
    let expected_proposer_public_key = get_proposer(validator_set)
        .wrap_err("failed to get proposer from validator set")?
        .pub_key
        .to_bytes();

    // verify the namespace data signing public key matches the proposer public key
    let key = data.public_key();
    let proposer_public_key = key.as_bytes();
    ensure!(
        &proposer_public_key[..] == &*expected_proposer_public_key,
        "public key mismatch: expected {}, got {}",
        hex::encode(expected_proposer_public_key),
        hex::encode(proposer_public_key),
    );

    Ok(())
}

fn validate_sequencer_namespace_data(
    current_validator_set: &validators::Response,
    parent_validator_set: &validators::Response,
    data: &SequencerNamespaceData,
) -> eyre::Result<()> {
    use sha2::Digest as _;

    let SequencerNamespaceData {
        block_hash,
        header,
        last_commit,
        rollup_chain_ids: _,
        action_tree_root,
        action_tree_root_inclusion_proof,
        chain_ids_commitment,
    } = data;

    // find proposer address for this height
    let expected_proposer_address = account::Id::from(
        get_proposer(current_validator_set)
            .wrap_err("failed to get proposer from validator set")?
            .pub_key,
    );
    // check if the proposer address matches the sequencer block's proposer
    let received_proposer_address = header.proposer_address;
    ensure!(
        received_proposer_address == expected_proposer_address,
        "proposer address mismatch: expected `{expected_proposer_address}`, got \
         `{received_proposer_address}`",
    );

    match &last_commit {
        Some(last_commit) => {
            // validate that commit signatures hash to header.last_commit_hash
            let calculated_last_commit_hash = calculate_last_commit_hash(last_commit);
            let Some(last_commit_hash) = header.last_commit_hash.as_ref() else {
                bail!("last commit hash should not be empty");
            };

            if &calculated_last_commit_hash != last_commit_hash {
                bail!("last commit hash in header does not match calculated last commit hash");
            }

            // verify that the validator votes on the previous block have >2/3 voting power
            let last_commit = last_commit.clone();
            let chain_id = header.chain_id.clone();
            ensure_commit_has_quorum(&last_commit, parent_validator_set, chain_id.as_ref())
                .wrap_err("failed to ensure commit has quorum")?

            // TODO: commit is for previous block; how do we handle this? (#50)
        }
        None => {
            // the last commit can only be empty on block 1
            ensure!(header.height == 1u32.into(), "last commit hash not found");
            ensure!(
                header.last_commit_hash.is_none(),
                "last commit hash should be empty"
            );
        }
    }

    // validate the block header matches the block hash
    let block_hash_from_header = header.hash();
    ensure!(
        block_hash_from_header == *block_hash,
        "block hash calculated from tendermint header does not match block hash stored in \
         sequencer block",
    );

    // validate the action tree root was included inside `data_hash`
    let Some(data_hash) = header.data_hash else {
        bail!("data hash should not be empty");
    };
    let action_tree_root_hash = sha2::Sha256::digest(action_tree_root);
    action_tree_root_inclusion_proof
        .verify(&action_tree_root_hash, data_hash)
        .wrap_err("failed to verify action tree root inclusion proof")?;

    // validate the chain IDs commitment
    let leaves = data
        .rollup_chain_ids
        .iter()
        .map(|chain_id| chain_id.as_ref().to_vec())
        .collect::<Vec<_>>();
    let expected_chain_ids_commitment =
        sequencer_validation::MerkleTree::from_leaves(leaves).root();
    ensure!(
        expected_chain_ids_commitment == *chain_ids_commitment,
        "chain IDs commitment mismatch: expected {}, got {}",
        hex::encode(expected_chain_ids_commitment),
        hex::encode(chain_ids_commitment),
    );

    Ok(())
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
    validator_set: &validators::Response,
    chain_id: &str,
) -> eyre::Result<()> {
    if commit.height != validator_set.block_height {
        bail!(
            "commit height mismatch: expected {}, got {}",
            validator_set.block_height,
            commit.height
        );
    }

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
            let address = account::Id::from(v.pub_key);
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
        let address_from_pubkey = account::Id::from(validator.pub_key);

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
    chain_id: &str,
    public_key: &tendermint::PublicKey,
    signature_bytes: &[u8],
) -> eyre::Result<()> {
    let public_key = VerificationKey::try_from(public_key.to_bytes().as_slice())
        .wrap_err("failed to create public key from vote")?;
    let signature =
        Signature::try_from(signature_bytes).wrap_err("failed to create signature from vote")?;

    let canonical_vote = CanonicalVote {
        vote_type: vote::Type::Precommit,
        height: commit.height,
        round: commit.round,
        block_id: Some(block::Id {
            hash: commit.block_id.hash,
            part_set_header: commit.block_id.part_set_header,
        }),
        timestamp: Some(timestamp),
        chain_id: chain::Id::try_from(chain_id).wrap_err("failed to parse commit chain ID")?,
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

/// returns the proposer given the current set by ordering the validators by proposer priority.
/// the validator with the highest proposer priority is the proposer.
/// TODO: could there ever be two validators with the same priority?
fn get_proposer(validator_set: &validators::Response) -> eyre::Result<Validator> {
    validator_set
        .validators
        .iter()
        .max_by(|v1, v2| v1.proposer_priority.cmp(&v2.proposer_priority))
        .cloned()
        .ok_or_else(|| eyre::eyre!("no proposer found"))
}

#[cfg(test)]
mod test {
    use std::{
        collections::BTreeMap,
        str::FromStr,
    };

    use sequencer_validation::{
        generate_action_tree_leaves,
        MerkleTree,
    };
    use tendermint::{
        account,
        block::Commit,
        validator,
        Hash,
    };

    use super::*;

    fn make_test_validator_set(height: u32) -> (validators::Response, account::Id) {
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

        (
            validators::Response::new(height.into(), vec![validator], 1),
            address,
        )
    }

    #[test]
    fn validate_sequencer_namespace_data_last_commit_none_ok() {
        let action_tree = MerkleTree::from_leaves(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let action_tree_root = action_tree.root();

        let txs = vec![action_tree_root.to_vec()];
        let (data_hash, tx_tree) =
            astria_sequencer_types::sequencer_block_data::calculate_data_hash_and_tx_tree(&txs);
        let action_tree_root_inclusion_proof = tx_tree.prove_inclusion(0).unwrap();

        let mut header = astria_sequencer_types::test_utils::default_header();
        let height = header.height.value() as u32;
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let (validator_set, proposer_address) = make_test_validator_set(height);
        header.proposer_address = proposer_address;
        let block_hash = header.hash();

        let sequencer_namespace_data = SequencerNamespaceData {
            block_hash,
            header,
            last_commit: None,
            rollup_chain_ids: vec![],
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment: MerkleTree::from_leaves(vec![]).root(),
        };

        validate_sequencer_namespace_data(
            &validator_set,
            &make_test_validator_set(height - 1).0,
            &sequencer_namespace_data,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn validate_rollup_data_ok() {
        let test_tx = b"test-tx".to_vec();
        let test_chain_id = b"test-chain";
        let mut btree = BTreeMap::new();
        btree.insert(test_chain_id.to_vec(), vec![test_tx.clone()]);
        let leaves = generate_action_tree_leaves(btree);

        let action_tree = MerkleTree::from_leaves(leaves);
        let action_tree_root = action_tree.root();

        let txs = vec![action_tree_root.to_vec()];
        let (data_hash, tx_tree) =
            astria_sequencer_types::sequencer_block_data::calculate_data_hash_and_tx_tree(&txs);
        let action_tree_root_inclusion_proof = tx_tree.prove_inclusion(0).unwrap();

        let mut header = astria_sequencer_types::test_utils::default_header();
        let height = header.height.value() as u32;
        header.data_hash = Some(Hash::try_from(data_hash.to_vec()).unwrap());

        let (validator_set, proposer_address) = make_test_validator_set(height);
        header.proposer_address = proposer_address;
        let block_hash = header.hash();

        let sequencer_namespace_data = SequencerNamespaceData {
            block_hash,
            header,
            last_commit: None,
            rollup_chain_ids: vec![
                astria_sequencer_types::ChainId::new(test_chain_id.to_vec()).unwrap(),
            ],
            action_tree_root,
            action_tree_root_inclusion_proof,
            chain_ids_commitment: MerkleTree::from_leaves(vec![test_chain_id.to_vec()]).root(),
        };

        let rollup_namespace_data = RollupNamespaceData {
            block_hash,
            chain_id: astria_sequencer_types::ChainId::new(test_chain_id.to_vec()).unwrap(),
            rollup_txs: vec![test_tx],
            inclusion_proof: action_tree.prove_inclusion(0).unwrap(),
        };

        validate_sequencer_namespace_data(
            &validator_set,
            &make_test_validator_set(height - 1).0,
            &sequencer_namespace_data,
        )
        .unwrap();
        rollup_namespace_data
            .verify_inclusion_proof(sequencer_namespace_data.action_tree_root)
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
    fn test_ensure_commit_has_quorum_ok() {
        // these values were retrieved by running the sequencer node and requesting the following:
        // curl http://localhost:26657/validators
        // curl http://localhost:26657/commit?height=79
        let validator_set_str = r#"{"block_height":"79","validators":[{"address":"D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9","pub_key":{"type":"tendermint/PubKeyEd25519","value":"tyPnz5GGblrx3PBjQRxZOHbzsPEI1E8lOh62QoPSWLw="},"voting_power":"10","proposer_priority":"0"}],"count":"1","total":"1"}"#;
        let commit_str = r#"{"height":"79","round":0,"block_id":{"hash":"74BD4E7F7EF902A84D55589F2AA60B332F1C2F34DDE7652C80BFEB8E7471B1DA","parts":{"total":1,"hash":"7632FFB5D84C3A64279BC9EA86992418ED23832C66E0C3504B7025A9AF42C8C4"}},"signatures":[{"block_id_flag":2,"validator_address":"D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9","timestamp":"2023-07-05T19:02:55.206600022Z","signature":"qy9vEjqSrF+8sD0K0IAXA398xN1s3QI2rBBDbBMWf0rw0L+B9Z92DZEptf6bPYWuKUFdEc0QFKhUMQA8HjBaAw=="}]}"#;
        let validator_set =
            serde_json::from_str::<validators::Response>(validator_set_str).unwrap();
        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        ensure_commit_has_quorum(&commit, &validator_set, "test-chain-g3ejvw").unwrap();
    }

    #[test]
    fn test_ensure_commit_has_quorum_not_ok() {
        use base64::engine::{
            general_purpose::STANDARD,
            Engine as _,
        };
        let validator_set = validators::Response::new(
            79u32.into(),
            vec![Validator {
                name: None,
                address: tendermint::account::Id::from_str(
                    "D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9",
                )
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
            block_id: block::Id {
                hash: Hash::from_str(
                    "74BD4E7F7EF902A84D55589F2AA60B332F1C2F34DDE7652C80BFEB8E7471B1DA",
                )
                .unwrap(),
                part_set_header: tendermint::block::parts::Header::new(
                    1,
                    Hash::from_str(
                        "7632FFB5D84C3A64279BC9EA86992418ED23832C66E0C3504B7025A9AF42C8C4",
                    )
                    .unwrap(),
                )
                .unwrap(),
            },
            signatures: vec![],
        };

        let result = ensure_commit_has_quorum(&commit, &validator_set, "test-chain-g3ejvw");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("commit voting power is less than 2/3 of total voting power")
        );
    }
}
