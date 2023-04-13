use color_eyre::eyre::Result;
use log::{info, warn};
use prost_types::Timestamp;
use sequencer_relayer::proto::SequencerMsg;
use sequencer_relayer::sequencer_block::{
    cosmos_tx_body_to_sequencer_msgs, get_namespace, parse_cosmos_tx, Namespace, SequencerBlock,
};
use tendermint::Time;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task,
};

use crate::alert::{Alert, AlertSender};
use crate::config::Config;
use crate::execution_client::ExecutionRpcClient;

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
type Receiver = UnboundedReceiver<ExecutorCommand>;

/// spawns a executor task and returns a tuple with the task's join handle
/// and the channel for sending commands to this executor
pub(crate) async fn spawn(conf: &Config, alert_tx: AlertSender) -> Result<(JoinHandle, Sender)> {
    log::info!("Spawning executor task.");
    let (mut executor, executor_tx) = Executor::new(
        &conf.execution_rpc_url,
        get_namespace(conf.chain_id.as_bytes()),
        alert_tx,
    )
    .await?;
    let join_handle = task::spawn(async move { executor.run().await });
    log::info!("Spawned executor task.");
    Ok((join_handle, executor_tx))
}

// Given a string, convert to protobuf timestamp
fn time_conversion(value: &str) -> Option<Timestamp> {
    let time = Time::parse_from_rfc3339(value).expect("Could not get timestamp");
    let seconds = time.unix_timestamp();
    let all_nanos = time.unix_timestamp_nanos();
    let nanos = (all_nanos - (seconds as i128 * 10_i128.pow(9))) as i32; // Largest this can be is order of 10^9, ok to cast
    Some(Timestamp { seconds, nanos })
}

#[derive(Debug)]
pub(crate) enum ExecutorCommand {
    /// Command for when a block is received
    BlockReceived {
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
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> Result<()> {
        log::info!("Starting executor event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ExecutorCommand::BlockReceived { block } => {
                    log::info!(
                        "ExecutorCommand::BlockReceived height={}",
                        block.header.height
                    );
                    self.alert_tx.send(Alert::BlockReceived {
                        block_height: block.header.height.parse::<u64>()?,
                    })?;
                    self.execute_block(*block).await?;
                }
                ExecutorCommand::Shutdown => {
                    log::info!("Shutting down executor event loop.");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Uses RPC to send block to execution service
    async fn execute_block(&mut self, block: SequencerBlock) -> Result<()> {
        let prev_block_hash = self.execution_state.clone();
        info!(
            "executing block {} with parent block hash {}",
            block.header.height,
            hex::encode(&prev_block_hash)
        );

        // get transactions for our namespace
        let Some(txs) = block.rollup_txs.get(&self.namespace) else {
            info!("sequencer block {} did not contains txs for namespace", block.header.height);
            return Ok(());
        };

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

        let timestamp = time_conversion(&block.header.time);

        let response = self
            .execution_rpc_client
            .call_do_block(prev_block_hash, txs, timestamp)
            .await?;
        self.execution_state = response.block_hash;

        Ok(())
    }
}
