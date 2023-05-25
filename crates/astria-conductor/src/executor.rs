use std::collections::HashMap;

use astria_proto::sequencer::v1::SequencerMsg;
use astria_sequencer_relayer::{
    base64_string::Base64String,
    sequencer_block::{
        cosmos_tx_body_to_sequencer_msgs,
        get_namespace,
        parse_cosmos_tx,
        Namespace,
        SequencerBlock,
    },
};
use color_eyre::eyre::{
    Result,
    WrapErr as _,
};
use prost_types::Timestamp as ProstTimestamp;
use tendermint::Time;
use tokio::{
    sync::mpsc::{
        self,
        UnboundedReceiver,
        UnboundedSender,
    },
    task,
};
use tracing::{
    error,
    info,
    warn,
};

use crate::{
    alert::{
        Alert,
        AlertSender,
    },
    config::Config,
    execution_client::ExecutionRpcClient,
};

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
type Receiver = UnboundedReceiver<ExecutorCommand>;

/// spawns a executor task and returns a tuple with the task's join handle
/// and the channel for sending commands to this executor
pub(crate) async fn spawn(conf: &Config, alert_tx: AlertSender) -> Result<(JoinHandle, Sender)> {
    info!("Spawning executor task.");
    let (mut executor, executor_tx) = Executor::new(
        &conf.execution_rpc_url,
        get_namespace(conf.chain_id.as_bytes()),
        alert_tx,
    )
    .await?;
    let join_handle = task::spawn(async move { executor.run().await });
    info!("Spawned executor task.");
    Ok((join_handle, executor_tx))
}

// Given a string, convert to protobuf timestamp
fn convert_str_to_prost_timestamp(value: &str) -> Result<ProstTimestamp> {
    let time =
        Time::parse_from_rfc3339(value).wrap_err("failed parsing string as rfc3339 datetime")?;
    use tendermint_proto::google::protobuf::Timestamp as TendermintTimestamp;
    let TendermintTimestamp {
        seconds,
        nanos,
    } = time.into();
    Ok(ProstTimestamp {
        seconds,
        nanos,
    })
}

#[derive(Debug)]
pub enum ExecutorCommand {
    /// used when a block is received from the gossip network
    BlockReceivedFromGossipNetwork {
        block: Box<SequencerBlock>,
    },
    /// used when a block is received from the reader (Celestia)
    BlockReceivedFromDataAvailability {
        block: Box<SequencerBlock>,
    },
    Shutdown,
}

struct Executor {
    /// Channel on which executor commands are received.
    cmd_rx: Receiver,
    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: ExecutionRpcClient,
    /// Namespace ID
    namespace: Namespace,

    /// The channel on which the driver and tasks in the driver can post alerts
    /// to the consumer of the driver.
    alert_tx: AlertSender,
    /// Tracks the state of the execution chain
    execution_state: Vec<u8>,

    /// map of sequencer block hash to execution block hash
    ///
    /// this is required because when we receive sequencer blocks (from network or DA),
    /// we only know the sequencer block hash, but not the execution block hash,
    /// as the execution block hash is created by executing the block.
    /// as well, the execution layer is not aware of the sequencer block hash.
    /// we need to track the mapping of sequencer block hash -> execution block hash
    /// so that we can mark the block as final on the execution layer when
    /// we receive a finalized sequencer block.
    sequencer_hash_to_execution_hash: HashMap<Base64String, Vec<u8>>,
}

impl Executor {
    /// Creates a new Executor instance and returns a command sender and an alert receiver.
    async fn new(
        rpc_address: &str,
        namespace: Namespace,
        alert_tx: AlertSender,
    ) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let mut execution_rpc_client = ExecutionRpcClient::new(rpc_address).await?;
        let init_state_response = execution_rpc_client.call_init_state().await?;
        let execution_state = init_state_response.block_hash;
        Ok((
            Self {
                cmd_rx,
                execution_rpc_client,
                namespace,
                alert_tx,
                execution_state,
                sequencer_hash_to_execution_hash: HashMap::new(),
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        info!("Starting executor event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ExecutorCommand::BlockReceivedFromGossipNetwork {
                    block,
                } => {
                    self.alert_tx.send(Alert::BlockReceivedFromGossipNetwork {
                        block_height: block.header.height.parse::<u64>()?,
                    })?;
                    if let Err(e) = self.execute_block(*block).await {
                        error!("failed to execute block: {e:?}");
                    }
                }

                ExecutorCommand::BlockReceivedFromDataAvailability {
                    block,
                } => {
                    self.alert_tx
                        .send(Alert::BlockReceivedFromDataAvailability {
                            block_height: block.header.height.parse::<u64>()?,
                        })?;

                    if let Err(e) = self
                        .handle_block_received_from_data_availability(*block)
                        .await
                    {
                        error!("failed to finalize block: {}", e);
                    }
                }

                ExecutorCommand::Shutdown => {
                    info!("Shutting down executor event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Uses RPC to send block to execution service
    /// returns the resulting execution block hash
    async fn execute_block(&mut self, block: SequencerBlock) -> Result<Option<Vec<u8>>> {
        // get transactions for our namespace
        let Some(txs) = block.rollup_txs.get(&self.namespace) else {
            info!("sequencer block {} did not contains txs for namespace", block.header.height);
            return Ok(None);
        };

        let prev_block_hash = self.execution_state.clone();

        info!(
            "executing block {} with parent block hash {}",
            block.header.height,
            hex::encode(&prev_block_hash)
        );

        // parse cosmos sequencer transactions into rollup transactions
        // by converting them to SequencerMsgs and extracting the `data` field
        let txs = txs
            .iter()
            .filter_map(|tx| {
                let body = parse_cosmos_tx(&tx.transaction).ok()?;
                let msgs: Vec<SequencerMsg> = cosmos_tx_body_to_sequencer_msgs(body).ok()?;
                if msgs.len() > 1 {
                    // this should not happen and is a bug in the sequencer relayer
                    warn!(
                        "ignoring cosmos tx with more than one sequencer message: {:#?}",
                        msgs
                    );
                    return None;
                }
                let Some(msg) = msgs.first() else {
                    return None;
                };
                Some(msg.data.clone())
            })
            .collect::<Vec<_>>();

        let timestamp = convert_str_to_prost_timestamp(&block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        let response = self
            .execution_rpc_client
            .call_do_block(prev_block_hash, txs, Some(timestamp))
            .await?;
        self.execution_state = response.block_hash.clone();

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            "executed sequencer block {} (height={}) with execution block hash {}",
            block.block_hash,
            block.header.height,
            hex::encode(&response.block_hash)
        );
        self.sequencer_hash_to_execution_hash
            .insert(block.block_hash, response.block_hash.clone());

        Ok(Some(response.block_hash))
    }

    async fn handle_block_received_from_data_availability(
        &mut self,
        block: SequencerBlock,
    ) -> Result<()> {
        let sequencer_block_hash = block.block_hash.clone();
        let maybe_execution_block_hash = self
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .cloned();
        match maybe_execution_block_hash {
            Some(execution_block_hash) => {
                self.finalize_block(execution_block_hash, &sequencer_block_hash)
                    .await?;
            }
            None => {
                // this means either:
                // - we didn't receive the block from the gossip layer yet, or
                // - we received it, but the sequencer block didn't contain
                // any transactions for this rollup namespace, thus nothing was executed
                // on receiving this block.

                // try executing the block as it hasn't been executed before
                // execute_block will check if our namespace has txs; if so, it'll return the
                // resulting execution block hash, otherwise None
                let Some(execution_block_hash) = self.execute_block(block).await? else {
                    // no txs for our namespace, nothing to do
                    return Ok(());
                };

                // finalize the block after it's been executed
                self.finalize_block(execution_block_hash, &sequencer_block_hash)
                    .await?;
            }
        };
        Ok(())
    }

    async fn finalize_block(
        &mut self,
        execution_block_hash: Vec<u8>,
        sequencer_block_hash: &Base64String,
    ) -> Result<()> {
        self.execution_rpc_client
            .call_finalize_block(execution_block_hash.clone())
            .await?;
        info!(
            "finalized execution block {}",
            hex::encode(execution_block_hash)
        );
        self.sequencer_hash_to_execution_hash
            .remove(sequencer_block_hash);
        Ok(())
    }
}
