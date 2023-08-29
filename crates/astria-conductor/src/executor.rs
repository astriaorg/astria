use std::collections::HashMap;

use astria_sequencer_types::{
    Namespace,
    SequencerBlockData,
};
use color_eyre::eyre::{
    Result,
    WrapErr as _,
};
use prost_types::Timestamp as ProstTimestamp;
use tendermint::{
    Hash,
    Time,
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
    instrument,
};

use crate::{
    alert::{
        Alert,
        AlertSender,
    },
    config::Config,
    execution_client::{
        ExecutionClient,
        ExecutionRpcClient,
    },
    types::SequencerBlockSubset,
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
    let execution_rpc_client = ExecutionRpcClient::new(&conf.execution_rpc_url).await?;
    let (mut executor, executor_tx) = Executor::new(
        execution_rpc_client,
        Namespace::with_hashed_bytes(conf.chain_id.as_bytes()),
        alert_tx,
    )
    .await?;
    let join_handle = task::spawn(async move { executor.run().await });
    info!("Spawned executor task.");
    Ok((join_handle, executor_tx))
}

// Given `Time`, convert to protobuf timestamp
fn convert_tendermint_to_prost_timestamp(value: Time) -> Result<ProstTimestamp> {
    use tendermint_proto::google::protobuf::Timestamp as TendermintTimestamp;
    let TendermintTimestamp {
        seconds,
        nanos,
    } = value.into();
    Ok(ProstTimestamp {
        seconds,
        nanos,
    })
}

#[derive(Debug)]
pub enum ExecutorCommand {
    /// used when a block is received from the gossip network
    BlockReceivedFromGossipNetwork {
        block: Box<SequencerBlockData>,
    },
    /// used when a block is received from the reader (Celestia)
    BlockReceivedFromDataAvailability {
        block: Box<SequencerBlockSubset>,
    },
    Shutdown,
}

struct Executor<C> {
    /// Channel on which executor commands are received.
    cmd_rx: Receiver,

    /// The execution rpc client that we use to send messages to the execution service
    execution_rpc_client: C,

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
    sequencer_hash_to_execution_hash: HashMap<Hash, Vec<u8>>,
}

impl<C: ExecutionClient> Executor<C> {
    async fn new(
        mut execution_rpc_client: C,
        namespace: Namespace,
        alert_tx: AlertSender,
    ) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
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
                        block_height: block.header().height.value(),
                    })?;
                    let Some(block_subset) =
                        SequencerBlockSubset::from_sequencer_block_data(*block, self.namespace)
                    else {
                        info!(namespace = %self.namespace, "block did not contain data for namespace; skiping");
                        continue;
                    };
                    if let Err(e) = self.execute_block(block_subset).await {
                        error!("failed to execute block: {e:?}");
                    }
                }

                ExecutorCommand::BlockReceivedFromDataAvailability {
                    block,
                } => {
                    self.alert_tx
                        .send(Alert::BlockReceivedFromDataAvailability {
                            block_height: block.header.height.value(),
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

    /// checks for relevant transactions in the SequencerBlock and attempts
    /// to execute them via the execution service function DoBlock.
    /// if there are relevant transactions that successfully execute,
    /// it returns the resulting execution block hash.
    /// if the block has already been executed, it returns the previously-computed
    /// execution block hash.
    /// if there are no relevant transactions in the SequencerBlock, it returns None.
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Option<Vec<u8>>> {
        if block.rollup_transactions.is_empty() {
            debug!(
                height = block.header.height.value(),
                "no transactions in block"
            );
            return Ok(None);
        }

        if let Some(execution_hash) = self.sequencer_hash_to_execution_hash.get(&block.block_hash) {
            debug!(
                height = block.header.height.value(),
                execution_hash = hex::encode(execution_hash),
                "block already executed"
            );
            return Ok(Some(execution_hash.clone()));
        }

        let prev_block_hash = self.execution_state.clone();
        info!(
            height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_block_hash),
            "executing block with given parent block",
        );

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        let response = self
            .execution_rpc_client
            .call_do_block(prev_block_hash, block.rollup_transactions, Some(timestamp))
            .await?;
        self.execution_state = response.block_hash.clone();

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            sequencer_block_hash = ?block.block_hash,
            sequencer_block_height = block.header.height.value(),
            execution_block_hash = hex::encode(&response.block_hash),
            "executed sequencer block",
        );
        self.sequencer_hash_to_execution_hash
            .insert(block.block_hash, response.block_hash.clone());

        Ok(Some(response.block_hash))
    }

    async fn handle_block_received_from_data_availability(
        &mut self,
        block: SequencerBlockSubset,
    ) -> Result<()> {
        let sequencer_block_hash = block.block_hash;
        let maybe_execution_block_hash = self
            .sequencer_hash_to_execution_hash
            .get(&sequencer_block_hash)
            .cloned();
        match maybe_execution_block_hash {
            Some(execution_block_hash) => {
                self.finalize_block(execution_block_hash, sequencer_block_hash)
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
                let Some(execution_block_hash) = self
                    .execute_block(block)
                    .await
                    .wrap_err("failed to execute block")?
                else {
                    // no txs for our namespace, nothing to do
                    debug!("execute_block returned None; skipping finalize_block");
                    return Ok(());
                };

                // finalize the block after it's been executed
                self.finalize_block(execution_block_hash, sequencer_block_hash)
                    .await?;
            }
        };
        Ok(())
    }

    /// This function finalizes the given execution block on the execution layer by calling
    /// the execution service's FinalizeBlock function.
    /// note that this function clears the respective entry in the
    /// `sequencer_hash_to_execution_hash` map.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - the call to the execution service's FinalizeBlock function fails
    #[instrument(ret, err, skip_all, fields(
        execution_block_hash = hex::encode(&execution_block_hash),
        sequencer_block_hash = hex::encode(sequencer_block_hash),
    ))]
    async fn finalize_block(
        &mut self,
        execution_block_hash: Vec<u8>,
        sequencer_block_hash: Hash,
    ) -> Result<()> {
        self.execution_rpc_client
            .call_finalize_block(execution_block_hash)
            .await
            .wrap_err("failed to finalize block")?;
        self.sequencer_hash_to_execution_hash
            .remove(&sequencer_block_hash);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashSet,
        sync::Arc,
    };

    use astria_proto::generated::execution::v1alpha1::{
        DoBlockResponse,
        InitStateResponse,
    };
    use prost_types::Timestamp;
    use sha2::Digest as _;
    use tokio::sync::{
        mpsc,
        Mutex,
    };

    use super::*;

    // a mock ExecutionClient used for testing the Executor
    struct MockExecutionClient {
        finalized_blocks: Arc<Mutex<HashSet<Vec<u8>>>>,
    }

    impl MockExecutionClient {
        fn new() -> Self {
            Self {
                finalized_blocks: Arc::new(Mutex::new(HashSet::new())),
            }
        }
    }

    impl crate::private::Sealed for MockExecutionClient {}

    #[async_trait::async_trait]
    impl ExecutionClient for MockExecutionClient {
        // returns the sha256 hash of the prev_block_hash
        // the Executor passes self.execution_state as prev_block_hash
        async fn call_do_block(
            &mut self,
            prev_block_hash: Vec<u8>,
            _transactions: Vec<Vec<u8>>,
            _timestamp: Option<Timestamp>,
        ) -> Result<DoBlockResponse> {
            let res = hash(&prev_block_hash);
            Ok(DoBlockResponse {
                block_hash: res.to_vec(),
            })
        }

        async fn call_finalize_block(&mut self, block_hash: Vec<u8>) -> Result<()> {
            self.finalized_blocks.lock().await.insert(block_hash);
            Ok(())
        }

        async fn call_init_state(&mut self) -> Result<InitStateResponse> {
            let hasher = sha2::Sha256::new();
            Ok(InitStateResponse {
                block_hash: hasher.finalize().to_vec(),
            })
        }
    }

    fn hash(s: &[u8]) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(s);
        hasher.finalize().to_vec()
    }

    fn get_test_block_subset() -> SequencerBlockSubset {
        SequencerBlockSubset {
            block_hash: hash(b"block1").try_into().unwrap(),
            header: astria_sequencer_types::test_utils::default_header(),
            rollup_transactions: vec![],
        }
    }

    #[tokio::test]
    async fn execute_block_with_relevant_txs() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::with_hashed_bytes(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let expected_exection_hash = hash(&executor.execution_state);
        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        let execution_block_hash = executor
            .execute_block(block)
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash);
    }

    #[tokio::test]
    async fn execute_block_without_relevant_txs() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::with_hashed_bytes(b"test");
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), namespace, alert_tx)
            .await
            .unwrap();

        let block = get_test_block_subset();
        let execution_block_hash = executor.execute_block(block).await.unwrap();
        assert!(execution_block_hash.is_none());
    }

    #[tokio::test]
    async fn handle_block_received_from_data_availability_not_yet_executed() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::with_hashed_bytes(b"test");
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(execution_client, namespace, alert_tx)
            .await
            .unwrap();

        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        let expected_exection_hash = hash(&executor.execution_state);

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        // should have executed and finalized the block
        assert!(finalized_blocks.lock().await.len() == 1);
        assert!(
            finalized_blocks
                .lock()
                .await
                .get(&executor.execution_state)
                .is_some()
        );
        assert_eq!(expected_exection_hash, executor.execution_state);
        // should be empty because 1 block was executed and finalized, which deletes it from the map
        assert!(executor.sequencer_hash_to_execution_hash.is_empty());
    }

    #[tokio::test]
    async fn handle_block_received_from_data_availability_no_relevant_transactions() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = Namespace::with_hashed_bytes(b"test");
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(execution_client, namespace, alert_tx)
            .await
            .unwrap();

        let block: SequencerBlockSubset = get_test_block_subset();
        let previous_execution_state = executor.execution_state.clone();

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        // should not have executed or finalized the block
        assert!(finalized_blocks.lock().await.is_empty());
        assert!(
            finalized_blocks
                .lock()
                .await
                .get(&executor.execution_state)
                .is_none()
        );
        assert_eq!(previous_execution_state, executor.execution_state,);
        // should be empty because nothing was executed
        assert!(executor.sequencer_hash_to_execution_hash.is_empty());
    }
}
