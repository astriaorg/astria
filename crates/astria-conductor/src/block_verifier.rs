use std::collections::HashMap;

use astria_sequencer_relayer::data_availability::{
    RollupNamespaceData,
    SequencerNamespaceData,
    SignedNamespaceData,
};
use astria_sequencer_types::SequencerBlockData;
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
use tendermint::{
    account::Id as AccountId,
    block::{
        Commit,
        Header,
        Id as BlockId,
    },
    chain::Id as ChainId,
    crypto,
    merkle,
    validator::Info as Validator,
    vote::{
        self,
        CanonicalVote,
    },
    Hash,
};
use tendermint_proto::types::CommitSig as RawCommitSig;
use tendermint_rpc::{
    endpoint::validators::Response as ValidatorSet,
    Client,
    HttpClient,
};
use tracing::{
    instrument,
    warn,
};

/// `BlockVerifier` is responsible for verifying the correctness of a block
/// before executing it.
/// It has two public functions: `validate_signed_namespace_data` and `validate_sequencer_block`.
///
/// `validate_signed_namespace_data` is used to validate the data received from the data
/// availability layer. `validate_sequencer_block` is used to validate the blocks received from
/// either the data availability layer or the gossip network.
pub struct BlockVerifier {
    sequencer_client: HttpClient,
}

impl BlockVerifier {
    pub fn new(sequencer_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            sequencer_client: HttpClient::new(sequencer_url)
                .wrap_err("failed to construct sequencer client")?,
        })
    }

    /// validates `SignedNamespaceData` received from Celestia.
    /// This function verifies the block signature and checks that the data
    /// was signed by the expected proposer for this block height.
    pub(crate) async fn validate_signed_namespace_data(
        &self,
        data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> eyre::Result<()> {
        // verify the block signature
        data.verify()
            .wrap_err("failed to verify signature of signed namepsace data")?;

        // get validator set for this height
        let height: u32 = data.data.header.height.value().try_into().expect(
            "a tendermint height (currently non-negative i32) should always fit into a u32",
        );
        let validator_set = self
            .sequencer_client
            .validators(height, tendermint_rpc::Paging::Default)
            .await
            .wrap_err("failed to get validator set")?;

        // find proposer address for this height
        let expected_proposer_public_key = get_proposer(&validator_set)
            .wrap_err("failed to get proposer from validator set")?
            .pub_key
            .to_bytes();

        // verify the namespace data signing public key matches the proposer public key
        let proposer_public_key = &data.public_key;
        ensure!(
            proposer_public_key == &expected_proposer_public_key,
            "public key mismatch: expected {}, got {}",
            hex::encode(expected_proposer_public_key),
            hex::encode(proposer_public_key),
        );

        Ok(())
    }

    pub async fn validate_rollup_data(
        &self,
        block_hash: &[u8],
        header: &Header,
        last_commit: &Option<Commit>,
        _rollup_data: &RollupNamespaceData,
    ) -> eyre::Result<()> {
        self.validate_sequencer_block_header_and_last_commit(block_hash, header, last_commit)
            .await
            .context("failed to validate sequencer block header and last commit")?;
        // TODO: validate rollup data w/ merkle proofs
        // https://github.com/astriaorg/astria/issues/153
        Ok(())
    }

    /// performs various validation checks on the SequencerBlock received from either gossip or
    /// Celestia.
    ///
    /// checks performed:
    /// - the proposer of the sequencer block matches the expected proposer for the block height
    ///   from tendermint
    /// - the signer of the SignedNamespaceData the proposer
    /// - the signature is valid
    /// - the root of the markle tree of all the header fields matches the block's block_hash
    /// - the root of the merkle tree of all transactions in the block matches the block's data_hash
    /// - validate the block was actually finalized; ie >2/3 stake signed off on it
    pub async fn validate_sequencer_block_data(
        &self,
        block: &SequencerBlockData,
    ) -> eyre::Result<()> {
        self.validate_sequencer_block_header_and_last_commit(
            block.block_hash(),
            block.header(),
            block.last_commit(),
        )
        .await?;

        // finally, validate that the transactions in the block result in the correct data_hash
        block
            .verify_data_hash()
            .wrap_err("failed to verify block data_hash")?;

        Ok(())
    }

    async fn validate_sequencer_block_header_and_last_commit(
        &self,
        block_hash: &[u8],
        header: &Header,
        last_commit: &Option<Commit>,
    ) -> eyre::Result<()> {
        // sequencer block's height
        let height: u32 = header.height.value().try_into().expect(
            "a tendermint height (currently non-negative i32) should always fit into a u32",
        );

        // get validator set for the previous height, as the commit contained
        // in the block is for the previous height
        let validator_set = self
            .sequencer_client
            .validators(height - 1, tendermint_rpc::Paging::Default)
            .await
            .wrap_err("failed to get validator set")?;

        // find proposer address for this height
        let expected_proposer_address = public_key_bytes_to_address(
            &get_proposer(&validator_set)
                .wrap_err("failed to get proposer from validator set")?
                .pub_key,
        )
        .wrap_err("failed to convert proposer public key to address")?;

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
                tokio::task::spawn_blocking(move || -> eyre::Result<()> {
                    ensure_commit_has_quorum(&last_commit, &validator_set, chain_id.as_ref())
                })
                .await?
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
            block_hash_from_header.as_bytes() == block_hash,
            "block hash calculated from tendermint header does not match block hash stored in \
             sequencer block",
        );

        Ok(())
    }
}

fn public_key_bytes_to_address(public_key: &tendermint::PublicKey) -> eyre::Result<AccountId> {
    let public_key =
        tendermint::crypto::ed25519::VerificationKey::try_from(public_key.to_bytes().as_slice())
            .wrap_err("failed to convert proposer public key bytes")?;
    Ok(AccountId::from(public_key))
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
    validator_set: &ValidatorSet,
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
        .filter_map(|v| {
            let address = public_key_bytes_to_address(&v.pub_key).ok()?;
            Some((address, v))
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
        let address_from_pubkey = public_key_bytes_to_address(&validator.pub_key)
            .wrap_err("failed to convert validator public key to address")?;
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
        block_id: Some(BlockId {
            hash: commit.block_id.hash,
            part_set_header: commit.block_id.part_set_header,
        }),
        timestamp: Some(timestamp),
        chain_id: ChainId::try_from(chain_id).wrap_err("failed to parse commit chain ID")?,
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

// see https://github.com/cometbft/cometbft/blob/539985efc7d461668ffb46dff88b3f7bb9275e5a/types/block.go#L922
// block_id_flag types are: https://github.com/cometbft/cometbft/blob/4e130bde8e85ec78ae81d06aa54df056a8fae43a/spec/core/data_structures.md?plain=1#L251
fn calculate_last_commit_hash(commit: &tendermint::block::Commit) -> Hash {
    let signatures = commit
        .signatures
        .iter()
        .filter_map(|v| Some(RawCommitSig::try_from(v.clone()).ok()?.encode_to_vec()))
        .collect::<Vec<_>>();
    Hash::Sha256(merkle::simple_hash_from_byte_vectors::<
        crypto::default::Sha256,
    >(&signatures))
}

/// returns the proposer given the current set by ordering the validators by proposer priority.
/// the validator with the highest proposer priority is the proposer.
/// TODO: could there ever be two validators with the same priority?
fn get_proposer(validator_set: &ValidatorSet) -> eyre::Result<Validator> {
    validator_set
        .validators
        .iter()
        .max_by(|v1, v2| v1.proposer_priority.cmp(&v2.proposer_priority))
        .cloned()
        .ok_or_else(|| eyre::eyre!("no proposer found"))
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use tendermint::block::Commit;

    use super::*;

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
        let validator_set = serde_json::from_str::<ValidatorSet>(validator_set_str).unwrap();
        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        ensure_commit_has_quorum(&commit, &validator_set, "test-chain-g3ejvw").unwrap();
    }

    #[test]
    fn test_ensure_commit_has_quorum_not_ok() {
        use base64::engine::{
            general_purpose::STANDARD,
            Engine as _,
        };
        let validator_set = ValidatorSet::new(
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
            block_id: BlockId {
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

    #[test]
    fn test_calculate_last_commit_hash() {
        // these values were retrieved by running the sequencer node and requesting the following:
        // curl http://localhost:26657/commit?height=79
        // curl http://localhost:26657/block?height=80 | grep last_commit_hash
        let commit_str = r#"{"height":"79","round":0,"block_id":{"hash":"74BD4E7F7EF902A84D55589F2AA60B332F1C2F34DDE7652C80BFEB8E7471B1DA","parts":{"total":1,"hash":"7632FFB5D84C3A64279BC9EA86992418ED23832C66E0C3504B7025A9AF42C8C4"}},"signatures":[{"block_id_flag":2,"validator_address":"D223B03AE01B4A0296053E01A41AE1E2F9CDEBC9","timestamp":"2023-07-05T19:02:55.206600022Z","signature":"qy9vEjqSrF+8sD0K0IAXA398xN1s3QI2rBBDbBMWf0rw0L+B9Z92DZEptf6bPYWuKUFdEc0QFKhUMQA8HjBaAw=="}]}"#;
        let expected_last_commit_hash =
            Hash::from_str("EF285154CDF29146FF423EB48BC7F88A0B57022C9B63455EC7AE876F4EA45B6F")
                .unwrap();
        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        let last_commit_hash = calculate_last_commit_hash(&commit);
        assert!(matches!(last_commit_hash, Hash::Sha256(_)));
        assert!(expected_last_commit_hash.as_bytes() == last_commit_hash.as_bytes());
    }
}
