use std::{
    collections::HashMap,
    str::FromStr,
};

use astria_sequencer_relayer::{
    data_availability::{
        CelestiaClient,
        CelestiaClientBuilder,
        SequencerNamespaceData,
        SignedNamespaceData,
    },
    keys::public_key_to_address,
    sequencer_block::SequencerBlock,
    types::{
        Commit,
        CommitSig,
    },
};
use bech32::{
    ToBase32,
    Variant,
};
use color_eyre::eyre::{
    bail,
    ensure,
    eyre,
    Result,
    WrapErr,
};
use ed25519_dalek::Verifier;
use prost::Message;
use tendermint::{
    account::{self,},
    block::{
        parts,
        CommitSig as TmCommitSig,
        Height,
        Id as BlockId,
        Round,
    },
    chain,
    crypto,
    merkle,
    vote::{
        self,
        CanonicalVote,
    },
    Hash,
    PublicKey,
    Signature,
    Time,
};
use tendermint_proto::types::CommitSig as RawCommitSig;
use tokio::{
    sync::mpsc::{
        self,
        UnboundedReceiver,
        UnboundedSender,
    },
    task,
};
use tracing::{
    debug,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    config::Config,
    executor,
    tendermint::{
        TendermintClient,
        ValidatorSet,
    },
};

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the reader task.
pub type Sender = UnboundedSender<ReaderCommand>;
/// The channel the reader task uses to listen for commands.
type Receiver = UnboundedReceiver<ReaderCommand>;

/// spawns a reader task and returns a tuple with the task's join handle
/// and the channel for sending commands to this reader
pub(crate) async fn spawn(
    conf: &Config,
    executor_tx: executor::Sender,
) -> Result<(JoinHandle, Sender)> {
    info!("Spawning reader task.");
    let (mut reader, reader_tx) =
        Reader::new(&conf.celestia_node_url, &conf.tendermint_url, executor_tx).await?;
    let join_handle = task::spawn(async move { reader.run().await });
    info!("Spawned reader task.");
    Ok((join_handle, reader_tx))
}

#[derive(Debug)]
pub enum ReaderCommand {
    /// Get new blocks
    GetNewBlocks,

    Shutdown,
}

pub struct Reader {
    /// Channel on which reader commands are received.
    cmd_rx: Receiver,

    /// The channel used to send messages to the executor task.
    executor_tx: executor::Sender,

    /// The client used to communicate with Celestia.
    celestia_client: CelestiaClient,

    /// the last block height fetched from Celestia
    curr_block_height: u64,

    tendermint_client: TendermintClient,
}

impl Reader {
    /// Creates a new Reader instance and returns a command sender and an alert receiver.
    pub async fn new(
        celestia_node_url: &str,
        tendermint_url: &str,
        executor_tx: executor::Sender,
    ) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let celestia_client = CelestiaClientBuilder::new(celestia_node_url.to_owned())
            .build()
            .wrap_err("failed creating celestia client")?;

        // TODO: we should probably pass in the height we want to start at from some genesis/config
        // file
        let curr_block_height = celestia_client.get_latest_height().await?;
        Ok((
            Self {
                cmd_rx,
                executor_tx,
                celestia_client,
                curr_block_height,
                tendermint_client: TendermintClient::new(tendermint_url.to_owned())?,
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        info!("Starting reader event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ReaderCommand::GetNewBlocks => {
                    let blocks = self
                        .get_new_blocks()
                        .await
                        .map_err(|e| eyre!("failed to get new block: {}", e))?;
                    for block in blocks {
                        self.process_block(block)
                            .await
                            .map_err(|e| eyre!("failed to process block: {}", e))?;
                    }
                }
                ReaderCommand::Shutdown => {
                    info!("Shutting down reader event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// get_new_blocks fetches any new sequencer blocks from Celestia.
    pub async fn get_new_blocks(&mut self) -> Result<Vec<SequencerBlock>> {
        debug!("ReaderCommand::GetNewBlocks");
        let mut blocks = vec![];

        // get the latest celestia block height
        let prev_height = self.curr_block_height;
        self.curr_block_height = self.celestia_client.get_latest_height().await?;
        info!(
            "checking celestia blocks {} to {}",
            prev_height, self.curr_block_height
        );

        // check for any new sequencer blocks written from the previous to current block height
        for height in prev_height..self.curr_block_height {
            let res = self
                .celestia_client
                .get_sequencer_namespace_data(height, None)
                .await;

            match res {
                Ok(datas) => {
                    // continue as celestia block doesn't have a sequencer block
                    if datas.is_empty() {
                        continue;
                    };

                    for data in datas {
                        let block = match self.validate_sequencer_namespace_data(data).await {
                            Ok(block) => block,
                            Err(e) => {
                                // this means someone submitted an invalid block to celestia;
                                // we can ignore it
                                warn!("sequencer block failed validation: {}", e);
                                continue;
                            }
                        };

                        blocks.push(block);
                    }
                }
                Err(e) => {
                    // just log the error for now.
                    // any blocks that weren't fetched will be handled in the next cycle
                    error!("{}", e.to_string());
                }
            }
        }

        // sort blocks by height
        // TODO: there isn't a guarantee that the blocks aren't severely out of order,
        // and we need to ensure that there are no gaps between the block heights before processing.
        blocks.sort_by(|a, b| a.header.height.cmp(&b.header.height));
        Ok(blocks)
    }

    /// performs various validation checks on the SignedNamespaceData that was read from Celestia
    /// and returns the full SequencerBlock corresponding to the given SignedNamespaceData if all
    /// checks pass.
    ///
    /// checks performed:
    /// - the proposer of the sequencer block matches the expected proposer for the block height
    ///   from tendermint
    /// - the signer of the SignedNamespaceData the proposer
    /// - the signature is valid
    /// - the root of the markle tree of all the header fields matches the block's block_hash
    /// - the root of the merkle tree of all transactions in the block matches the block's data_hash
    /// - validate the block was actually finalized; ie >2/3 stake signed off on it
    async fn validate_sequencer_namespace_data(
        &self,
        data: SignedNamespaceData<SequencerNamespaceData>,
    ) -> Result<SequencerBlock> {
        // sequencer block's height
        let height = data.data.header.height;

        // get validator set for this height
        let validator_set = self
            .tendermint_client
            .get_validator_set(height.value() - 1)
            .await?;

        // find proposer address for this height
        let expected_proposer_address = validator_set
            .get_proposer()
            .wrap_err("failed to get proposer from validator set")?
            .address;

        // check if the proposer address matches the sequencer block's proposer
        if data.data.header.proposer_address.to_string() != expected_proposer_address {
            bail!(
                "proposer address mismatch: expected {}, got {}",
                expected_proposer_address,
                data.data.header.proposer_address.to_string()
            );
        }

        // verify the namespace data signing public key matches the proposer address
        let res_address: account::Id = PublicKey::from_raw_ed25519(data.public_key.0.as_slice())
            .ok_or(eyre!(
                "failed to decode address from signed namespace data's public key"
            ))?
            .into();
        if res_address.to_string() != expected_proposer_address {
            bail!(
                "public key mismatch: expected {}, got {}",
                expected_proposer_address,
                res_address
            );
        }

        // validate that commit signatures hash to header.last_commit_hash
        let data = match calculate_last_commit_hash(&data.data.last_commit) {
            Hash::Sha256(calculated_last_commit_hash) => {
                let Some(last_commit_hash) = data.data.header.last_commit_hash.as_ref() else {
                    bail!("last commit hash should not be empty");
                };

                if calculated_last_commit_hash != last_commit_hash.0.as_slice() {
                    bail!("last commit hash in header does not match calculated last commit hash");
                }

                // verify that the validator votes on the previous block have >2/3 voting power
                tokio::task::spawn_blocking(
                    move || -> Result<SignedNamespaceData<SequencerNamespaceData>> {
                        ensure_commit_has_quorum(
                            &data.data.last_commit,
                            &validator_set,
                            &data.data.header.chain_id,
                        )?;
                        Ok(data)
                    },
                )
                .await?
                .wrap_err("failed to ensure commit has quorum")?

                // TODO: commit is for previous block; how do we handle this?
            }
            Hash::None => {
                // this case only happens if the last commit is empty, which should only happen on
                // block 1.
                ensure!(
                    data.data.header.height == Height::from(1_u8),
                    "last commit hash not found"
                );
                ensure!(
                    data.data.header.last_commit_hash.is_none(),
                    "last commit hash should be empty"
                );
                data
            }
        };

        // verify the block signature
        data.verify()?;

        // finally, get the full SequencerBlock
        // the reason the public key type needs to be converted is due to serialization
        // constraints, probably fix this later
        let public_key = ed25519_dalek::PublicKey::from_bytes(&data.public_key.0)?;

        // pass the public key to `get_sequencer_block` which does the signature validation for us
        let block = self
            .celestia_client
            .get_sequencer_block(&data.data, Some(&public_key))
            .await
            .map_err(|e| eyre!("failed to get rollup data: {}", e))?;

        // validate the block header matches the block hash
        block.verify_block_hash()?;

        // finally, validate that the transactions in the block result in the correct data_hash
        block.verify_data_hash()?;

        Ok(block)
    }

    /// Processes an individual block
    async fn process_block(&self, block: SequencerBlock) -> Result<()> {
        self.executor_tx.send(
            executor::ExecutorCommand::BlockReceivedFromDataAvailability {
                block: Box::new(block),
            },
        )?;

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
    commit: &Commit,
    validator_set: &ValidatorSet,
    chain_id: &str,
) -> Result<()> {
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
    .try_fold(0u64, |acc, validator| acc.checked_add(validator.voting_power)) else {
        bail!("total voting power exceeded u64:MAX");
    };

    let validator_map = validator_set
        .validators
        .iter()
        .map(|v| (&v.address, v)) // address is in bech32
        .collect::<HashMap<_, _>>();

    let mut commit_voting_power = 0u64;
    for vote in &commit.signatures {
        // we only care about votes that are for the Commit.BlockId (ignore absent validators and
        // votes for nil)
        // we only care about votes that are for the Commit.BlockId (ignore absent validators and
        // votes for nil)
        if vote.block_id_flag != "BLOCK_ID_FLAG_COMMIT" {
            continue;
        }
        // TODO: unpack into validator_address, signature, timestamp
        // verify validator exists in validator set
        let validator_address: String = bech32::encode(
            "metrovalcons",
            vote.validator_address.0.to_base32(),
            Variant::Bech32,
        )?;
        let Some(validator) = validator_map.get(&validator_address) else {
                bail!("validator {} not found in validator set", validator_address);
            };

        // verify address in signature matches validator pubkey
        let address_from_pubkey = public_key_to_address(&validator.pub_key.key.0)?;
        ensure!(
            address_from_pubkey == validator_address,
            format!(
                "validator address mismatch: expected {}, got {}",
                validator_address, address_from_pubkey
            )
        );

        // verify vote signature
        verify_vote_signature(
            vote,
            commit,
            chain_id,
            &validator.pub_key.key.0,
            &vote.signature.0,
        )?;

        commit_voting_power += validator.voting_power;
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
// TODO: we can change these types (CommitSig and Commit) to be the tendermint types
// after the other relayer types are updated.
fn verify_vote_signature(
    vote: &CommitSig,
    commit: &Commit,
    chain_id: &str,
    public_key_bytes: &[u8],
    signature_bytes: &[u8],
) -> Result<()> {
    let public_key = ed25519_dalek::PublicKey::from_bytes(public_key_bytes)?;
    let signature = ed25519_dalek::Signature::from_bytes(signature_bytes)?;
    let canonical_vote = CanonicalVote {
        vote_type: vote::Type::Precommit,
        height: Height::from_str(&commit.height)?,
        round: Round::from(commit.round as u16),
        block_id: Some(BlockId {
            hash: Hash::try_from(commit.block_id.hash.0.to_vec())?,
            part_set_header: parts::Header::new(
                commit.block_id.part_set_header.total,
                Hash::try_from(commit.block_id.part_set_header.hash.0.to_vec())?,
            )?,
        }),

        timestamp: Some(Time::parse_from_rfc3339(&vote.timestamp)?),
        chain_id: chain::Id::try_from(chain_id)?,
    };
    public_key.verify(
        &tendermint_proto::types::CanonicalVote::try_from(canonical_vote)?
            .encode_length_delimited_to_vec(),
        &signature,
    )?;
    Ok(())
}

// see https://github.com/cometbft/cometbft/blob/539985efc7d461668ffb46dff88b3f7bb9275e5a/types/block.go#L922
// block_id_flag types are: https://github.com/cometbft/cometbft/blob/4e130bde8e85ec78ae81d06aa54df056a8fae43a/spec/core/data_structures.md?plain=1#L251
fn calculate_last_commit_hash(commit: &Commit) -> Hash {
    let signatures = commit
        .signatures
        .iter()
        .filter_map(|v| {
            match v.block_id_flag.as_str() {
                "BLOCK_ID_FLAG_COMMIT" => {
                    let commit_sig = TmCommitSig::BlockIdFlagCommit {
                        signature: Some(Signature::try_from(v.signature.clone().0).ok()?),
                        validator_address: account::Id::try_from(v.validator_address.clone().0)
                            .ok()?,
                        timestamp: Time::parse_from_rfc3339(&v.timestamp).ok()?,
                    };
                    Some(RawCommitSig::try_from(commit_sig).ok()?.encode_to_vec())
                }
                "BLOCK_ID_FLAG_NIL" => {
                    let commit_sig = TmCommitSig::BlockIdFlagNil {
                        signature: Some(Signature::try_from(v.signature.clone().0).ok()?),
                        validator_address: account::Id::try_from(v.validator_address.clone().0)
                            .ok()?,
                        timestamp: Time::parse_from_rfc3339(&v.timestamp).ok()?,
                    };
                    Some(RawCommitSig::try_from(commit_sig).ok()?.encode_to_vec())
                }
                "BLOCK_ID_FLAG_ABSENT" => Some(
                    RawCommitSig::try_from(TmCommitSig::BlockIdFlagAbsent)
                        .ok()?
                        .encode_to_vec(),
                ),
                _ => None, // TODO: could this ever happen?
            }
        })
        .collect::<Vec<_>>();
    Hash::Sha256(merkle::simple_hash_from_byte_vectors::<
        crypto::default::Sha256,
    >(&signatures))
}
#[cfg(test)]
mod test {
    use astria_sequencer_relayer::{
        base64_string::Base64String,
        types::{
            BlockId,
            Parts,
        },
    };

    use super::*;
    use crate::tendermint::{
        KeyWithType,
        Validator,
        ValidatorSet,
    };

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
        let validator_set_str = r#"{
            "block_height": "2082",
            "validators": [
              {
                "address": "metrovalcons1hdu2nzhcyfnhaj9tfrdlekfnfwx895mk83d322",
                "pub_key": {
                  "@type": "/cosmos.crypto.ed25519.PubKey",
                  "key": "MdfFS4MH09Og5y+9SVxpJRqUnZkDGfnPjdyx4qM2Vng="
                },
                "voting_power": "5000",
                "proposer_priority": "0"
              }
            ],
            "pagination": {
              "next_key": null,
              "total": "1"
            }
          }"#;
        let commit_str = r#"{
            "height": "2082",
            "round": 0,
            "block_id": {
                "hash": "5QrZ8fznJw/X1lviA5cyQ2BwLbma8iuvXHqh6BiMJdU=",
                "part_set_header": {
                    "total": 1,
                    "hash": "DUMkxxMa2M0/aMmNyVGkvLn+3w1HTsGZ/YKyAVu+gdc="
                }
            },
            "signatures": [
                {
                    "block_id_flag": "BLOCK_ID_FLAG_COMMIT",
                    "validator_address": "u3ipivgiZ37Iq0jb/NkzS4xy03Y=",
                    "timestamp": "2023-05-29T13:57:32.797060160Z",
                    "signature": "SQdU03IyfHOiTeGrPcbgBnRSpjN7cimaX0XO3jWLIkKL5w8ePx7Lg7V1CaDDTQJ0G5WHtcHVQky2dzq4vmkHBA=="
                }
            ]
        }"#;

        let validator_set = serde_json::from_str::<ValidatorSet>(validator_set_str).unwrap();
        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        ensure_commit_has_quorum(&commit, &validator_set, "private").unwrap();
    }

    #[test]
    fn test_ensure_commit_has_quorum_not_ok() {
        let validator_set = ValidatorSet {
            block_height: "2082".to_string(),
            validators: vec![Validator {
                address: "metrovalcons1hdu2nzhcyfnhaj9tfrdlekfnfwx895mk83d322".to_string(),
                pub_key: KeyWithType {
                    key: Base64String::from_string(
                        "MdfFS4MH09Og5y+9SVxpJRqUnZkDGfnPjdyx4qM2Vng=".to_string(),
                    )
                    .unwrap(),
                    key_type: "/cosmos.crypto.ed25519.PubKey".to_string(),
                },
                voting_power: 5000,
                proposer_priority: 0,
            }],
        };

        let commit = Commit {
            height: "2082".to_string(),
            round: 0,
            block_id: BlockId {
                hash: Base64String::from_string(
                    "5QrZ8fznJw/X1lviA5cyQ2BwLbma8iuvXHqh6BiMJdU=".to_string(),
                )
                .unwrap(),
                part_set_header: Parts {
                    total: 1,
                    hash: Base64String::from_string(
                        "DUMkxxMa2M0/aMmNyVGkvLn+3w1HTsGZ/YKyAVu+gdc=".to_string(),
                    )
                    .unwrap(),
                },
            },
            signatures: vec![],
        };

        let result = ensure_commit_has_quorum(&commit, &validator_set, "private");
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
        let commit_str = r#"{
            "height": "2082",
            "round": 0,
            "block_id": {
                "hash": "5QrZ8fznJw/X1lviA5cyQ2BwLbma8iuvXHqh6BiMJdU=",
                "part_set_header": {
                    "total": 1,
                    "hash": "DUMkxxMa2M0/aMmNyVGkvLn+3w1HTsGZ/YKyAVu+gdc="
                }
            },
            "signatures": [
                {
                    "block_id_flag": "BLOCK_ID_FLAG_COMMIT",
                    "validator_address": "u3ipivgiZ37Iq0jb/NkzS4xy03Y=",
                    "timestamp": "2023-05-29T13:57:32.797060160Z",
                    "signature": "SQdU03IyfHOiTeGrPcbgBnRSpjN7cimaX0XO3jWLIkKL5w8ePx7Lg7V1CaDDTQJ0G5WHtcHVQky2dzq4vmkHBA=="
                }
            ]
        }"#;
        let expected_last_commit_hash =
            Base64String::from_string("rpjY+9Y2ZL9y8RsfcgiKSNw4emL6YyBneMbuztCS9HQ=".to_string())
                .unwrap();

        let commit = serde_json::from_str::<Commit>(commit_str).unwrap();
        let last_commit_hash = calculate_last_commit_hash(&commit);
        assert!(matches!(last_commit_hash, Hash::Sha256(_)));
        assert!(expected_last_commit_hash.0 == last_commit_hash.as_bytes());
    }
}
