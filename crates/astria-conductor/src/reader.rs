use std::{
    collections::HashMap,
    str::FromStr,
};

use astria_sequencer_relayer::{
    da::{
        CelestiaClient,
        CelestiaClientBuilder,
        SequencerNamespaceData,
        SignedNamespaceData,
    },
    keys::public_key_to_address,
    sequencer_block::SequencerBlock,
    types::Commit,
};
use bech32::{
    self,
    ToBase32,
    Variant,
};
use color_eyre::eyre::{
    bail,
    eyre,
    Result,
    WrapErr,
};
use ed25519_dalek::Verifier;
use prost::Message;
use tendermint::{
    block::CommitSig,
    crypto,
    merkle,
    Hash,
};
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
                        let block = match self.validate_sequencer_namespace_data(&data).await {
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
    /// (see https://github.com/astriaorg/astria/issues/16)
    async fn validate_sequencer_namespace_data(
        &self,
        data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> Result<SequencerBlock> {
        // sequencer block's height
        let height = data.data.header.height.parse::<u64>()?;

        // get validator set for this height
        let mut validator_set = self.tendermint_client.get_validator_set(height).await?;

        // find proposer address for this height
        let expected_proposer_address = validator_set.get_proposer()?.address;

        // check if the proposer address matches the sequencer block's proposer
        let received_proposer_address = bech32::encode(
            "metrovalcons",
            data.data.header.proposer_address.0.to_base32(),
            Variant::Bech32,
        )
        .wrap_err("failed converting bytes to bech32 address")?;

        if received_proposer_address != expected_proposer_address {
            bail!(
                "proposer address mismatch: expected {}, got {}",
                expected_proposer_address,
                received_proposer_address
            );
        }

        // verify the namespace data signing public key matches the proposer address
        let res_address = public_key_to_address(&data.public_key.0)?;
        if res_address != expected_proposer_address {
            bail!(
                "public key mismatch: expected {}, got {}",
                expected_proposer_address,
                res_address
            );
        }

        // validate that commit signatures hash to header.last_commit_hash
        match calculate_last_commit_hash(&data.data.last_commit) {
            Hash::Sha256(calculated_last_commit_hash) => {
                let Some(last_commit_hash) = data.data.header.last_commit_hash.as_ref() else {
                    bail!("last commit hash should not be empty");
                };

                if calculated_last_commit_hash.to_vec() != last_commit_hash.0 {
                    bail!("last commit hash mismatch");
                }

                // verify that the validator votes on the previous block have >2/3 voting power
                verify_commit(
                    &data.data.last_commit,
                    &validator_set,
                    &data.data.header.chain_id,
                )?;

                // TODO: commit is for previous block; how do we handle this?
            }
            Hash::None => {
                // this case only happens if the last commit is empty, which should only happen on
                // block 1.
                if data.data.header.height != "1" {
                    bail!("last commit hash not found");
                }

                if data.data.header.last_commit_hash.is_some() {
                    bail!("last commit hash should be empty");
                }
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

fn verify_commit(commit: &Commit, validator_set: &ValidatorSet, chain_id: &str) -> Result<()> {
    if commit.height != validator_set.block_height {
        bail!(
            "commit height mismatch: expected {}, got {}",
            validator_set.block_height,
            commit.height
        );
    }

    // TODO: assert the commit was not for nil

    let mut total_voting_power = 0u64;
    validator_set
        .validators
        .iter()
        .try_for_each(|v| -> Result<()> {
            total_voting_power += v.voting_power.parse::<u64>()?;
            Ok(())
        })?;

    let validator_map = validator_set
        .validators
        .iter()
        .map(|v| (v.address.to_owned(), v)) // address is in bech32
        .collect::<HashMap<_, _>>();

    let mut commit_voting_power = 0u64;
    for vote in &commit.signatures {
        // we only care about votes that are for the Commit.BlockId (ignore absent validators and
        // votes for nil)
        if vote.block_id_flag != "BLOCK_ID_FLAG_COMMIT" {
            continue;
        }

        // verify validator exists in validator set
        let validator_address = bech32::encode(
            "metrovalcons",
            vote.validator_address.0.to_base32(),
            Variant::Bech32,
        )?;
        let Some(validator) = validator_map.get(&validator_address) else {
            bail!("validator {} not found in validator set", validator_address);
        };

        // verify address in signature matches validator pubkey
        let address_from_pubkey = public_key_to_address(&validator.pub_key.key.0)?;
        if address_from_pubkey != validator_address {
            bail!(
                "validator address mismatch: expected {}, got {}",
                validator_address,
                address_from_pubkey
            );
        }

        // verify vote signature
        verify_vote_signature(
            vote,
            commit,
            chain_id,
            &validator.pub_key.key.0,
            &vote.signature.0,
        )?;

        commit_voting_power += validator.voting_power.parse::<u64>()?;
    }

    if commit_voting_power <= total_voting_power * 2 / 3 {
        bail!(
            "total voting power in votes is less than 2/3 of total voting power: {} <= {}",
            commit_voting_power,
            total_voting_power * 2 / 3,
        );
    }

    Ok(())
}

// see https://github.com/tendermint/tendermint/blob/35581cf54ec436b8c37fabb43fdaa3f48339a170/types/vote.go#L147
fn verify_vote_signature(
    vote: &astria_sequencer_relayer::types::CommitSig,
    commit: &Commit,
    chain_id: &str,
    public_key_bytes: &[u8],
    signature_bytes: &[u8],
) -> Result<()> {
    let public_key = ed25519_dalek::PublicKey::from_bytes(public_key_bytes)?;
    let signature = ed25519_dalek::Signature::from_bytes(signature_bytes)?;
    let canonical_vote = tendermint::vote::CanonicalVote {
        vote_type: tendermint::vote::Type::Precommit,
        height: tendermint::block::Height::from_str(&commit.height)?,
        round: tendermint::block::Round::from(commit.round as u16),
        block_id: Some(tendermint::block::Id {
            hash: tendermint::Hash::try_from(commit.block_id.hash.0.to_vec())?,
            part_set_header: tendermint::block::parts::Header::new(
                commit.block_id.part_set_header.total,
                tendermint::Hash::try_from(commit.block_id.part_set_header.hash.0.to_vec())?,
            )?,
        }),
        timestamp: Some(tendermint::Time::parse_from_rfc3339(&vote.timestamp)?),
        chain_id: tendermint::chain::Id::try_from(chain_id)?,
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
                    let commit_sig = CommitSig::BlockIdFlagCommit {
                        signature: Some(
                            tendermint::Signature::try_from(v.signature.clone().0).ok()?,
                        ),
                        validator_address: tendermint::account::Id::try_from(
                            v.validator_address.clone().0,
                        )
                        .ok()?,
                        timestamp: tendermint::Time::parse_from_rfc3339(&v.timestamp).ok()?,
                    };
                    Some(
                        tendermint_proto::types::CommitSig::try_from(commit_sig)
                            .ok()?
                            .encode_to_vec(),
                    )
                }
                "BLOCK_ID_FLAG_NIL" => None,
                "BLOCK_ID_FLAG_ABSENT" => None,
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
    use astria_sequencer_relayer::base64_string::Base64String;

    use super::*;

    #[test]
    fn test_verify_commit() {
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
        verify_commit(&commit, &validator_set, "private").unwrap();
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
        assert!(&expected_last_commit_hash.0 == last_commit_hash.as_bytes());
    }
}
