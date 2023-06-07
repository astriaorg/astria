use astria_sequencer_relayer::{
    da::{
        CelestiaClient,
        CelestiaClientBuilder,
        SequencerNamespaceData,
        SignedNamespaceData,
    },
    sequencer_block::SequencerBlock,
};
use bech32::FromBase32;
use color_eyre::eyre::{
    bail,
    eyre,
    Result,
    WrapErr,
};
use tendermint::{
    account,
    PublicKey,
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
    tendermint::TendermintClient,
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
    /// - TODO: validate the block was actually finalized; ie >2/3 stake signed off on it
    /// (see https://github.com/astriaorg/astria/issues/16)
    async fn validate_sequencer_namespace_data(
        &self,
        data: &SignedNamespaceData<SequencerNamespaceData>,
    ) -> Result<SequencerBlock> {
        // sequencer block's height
        let height = data.data.header.height.into();

        // expected proposer address is in bech32 format
        let expected_proposer_address =
            self.tendermint_client
                .get_proposer_address(height)
                .await
                .map_err(|e| eyre!("failed to get proposer address: {}", e))?;

        let expected_proposer_address = bech32_address_to_hex(&expected_proposer_address)?;

        // check if the proposer address matches the sequencer block's proposer
        let received_proposer_address = data.data.header.proposer_address;

        if received_proposer_address.to_string() != expected_proposer_address {
            bail!(
                "proposer address mismatch: expected {}, got {}",
                expected_proposer_address,
                received_proposer_address
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

fn bech32_address_to_hex(address: &str) -> Result<String> {
    let (_, data, _) = bech32::decode(address)?;
    let bytes = Vec::<u8>::from_base32(&data)?;
    Ok(hex::encode(&bytes))
}
