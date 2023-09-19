use std::collections::HashMap;

use astria_proto::generated::execution::v1alpha2::{
    Block,
    CommitmentState,
};
use astria_sequencer_types::{
    ChainId,
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
    Instrument,
};

use crate::{
    config::Config,
    execution_client::{
        ExecutionClientV1Alpha1,
        ExecutionClientV1Alpha2,
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
pub(crate) async fn spawn(conf: &Config) -> Result<(JoinHandle, Sender)> {
    info!(
        chain_id = %conf.chain_id,
        execution_rpc_url = %conf.execution_rpc_url,
        "Spawning executor task."
    );
    let execution_rpc_client = ExecutionRpcClient::new(&conf.execution_rpc_url).await?;
    let (mut executor, executor_tx) = Executor::new(
        execution_rpc_client,
        ChainId::new(conf.chain_id.as_bytes().to_vec()),
    )
    .await?;
    let join_handle = task::spawn(async move { executor.run().in_current_span().await });
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
pub(crate) enum ExecutorCommand {
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

    /// Chain ID
    chain_id: ChainId,

    /// Namespace ID, derived from chain ID
    namespace: Namespace,

    /// Tracks SOFT and FIRM on the execution chain
    commitment_state: CommitmentState,

    /// map of sequencer block hash to execution block
    ///
    /// this is required because when we receive sequencer blocks (from network or DA),
    /// we only know the sequencer block hash, but not the execution block hash,
    /// as the execution block hash is created by executing the block.
    /// as well, the execution layer is not aware of the sequencer block hash.
    /// we need to track the mapping of sequencer block hash -> execution block
    /// so that we can mark the block as final on the execution layer when
    /// we receive a finalized sequencer block.
    sequencer_hash_to_execution_block: HashMap<Hash, Block>,
}

impl<C: ExecutionClientV1Alpha1 + ExecutionClientV1Alpha2> Executor<C> {
    async fn new(mut execution_rpc_client: C, chain_id: ChainId) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

        let commitment_state = execution_rpc_client.call_get_commitment_state().await?;

        Ok((
            Self {
                cmd_rx,
                execution_rpc_client,
                chain_id: chain_id.clone(),
                namespace: Namespace::from_slice(chain_id.as_ref()),
                commitment_state,
                sequencer_hash_to_execution_block: HashMap::new(),
            },
            cmd_tx,
        ))
    }

    #[instrument(skip_all)]
    async fn run(&mut self) -> Result<()> {
        info!("Starting executor event loop.");

        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                ExecutorCommand::BlockReceivedFromGossipNetwork {
                    block,
                } => {
                    let height = block.header().height.value();
                    let Some(block_subset) =
                        SequencerBlockSubset::from_sequencer_block_data(*block, &self.chain_id)
                    else {
                        info!(
                            namespace = %self.namespace,
                            "block did not contain data for namespace; skipping"
                        );
                        continue;
                    };
                    match self.execute_block(block_subset.clone()).await {
                        Ok(Some(executed_block)) => {
                            // set soft but leave firm the same
                            self.update_commitment_state(
                                self.commitment_state.firm.clone().unwrap(),
                                executed_block,
                            )
                            .await?;
                        }
                        Err(e) => {
                            error!(
                                height = height,
                                error = ?e,
                                "failed to execute block"
                            );
                        }
                        _ => {}
                    }
                }

                ExecutorCommand::BlockReceivedFromDataAvailability {
                    block,
                } => {
                    let height = block.header.height.value();
                    if let Err(e) = self
                        .handle_block_received_from_data_availability(*block)
                        .await
                    {
                        error!(
                            height = height,
                            error = ?e,
                            "failed to finalize block"
                        );
                    }
                }

                ExecutorCommand::Shutdown => {
                    info!(
                        namespace = %self.namespace,
                        "Shutting down executor event loop."
                    );
                    break;
                }
            }
        }

        Ok(())
    }

    /// checks for relevant transactions in the SequencerBlock and attempts
    /// to execute them via the execution service function DoBlock.
    /// if there are relevant transactions that successfully execute,
    /// it returns the resulting execution block.
    /// if the block has already been executed, it returns the previously-computed
    /// execution block.
    /// if there are no relevant transactions in the SequencerBlock, it returns None.
    async fn execute_block(&mut self, block: SequencerBlockSubset) -> Result<Option<Block>> {
        if block.rollup_transactions.is_empty() {
            debug!(
                height = block.header.height.value(),
                "no transactions in block"
            );
            return Ok(None);
        }

        if let Some(execution_block) = self
            .sequencer_hash_to_execution_block
            .get(&block.block_hash)
        {
            debug!(
                height = block.header.height.value(),
                execution_hash = hex::encode(execution_block.clone().hash),
                "block already executed"
            );
            return Ok(Some(execution_block.clone()));
        }

        let prev_block_hash = self.commitment_state.soft.clone().unwrap().hash;
        info!(
            height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_block_hash),
            "executing block with given parent block",
        );

        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        let executed_block = self
            .execution_rpc_client
            .call_execute_block(prev_block_hash, block.rollup_transactions, Some(timestamp))
            .await?;

        // store block hash returned by execution client, as we need it to finalize the block later
        info!(
            sequencer_block_hash = ?block.block_hash,
            sequencer_block_height = block.header.height.value(),
            execution_block_hash = hex::encode(&executed_block.hash),
            "executed sequencer block",
        );

        self.sequencer_hash_to_execution_block
            .insert(block.block_hash, executed_block.clone());

        Ok(Some(executed_block))
    }

    /// Updates the soft and firm blocks on the execution layer.
    /// Updates the local commitment_state with the new values.
    async fn update_commitment_state(&mut self, firm: Block, soft: Block) -> Result<()> {
        let commitment_state = self
            .execution_rpc_client
            .call_update_commitment_state(CommitmentState {
                firm: Some(firm),
                soft: Some(soft),
            })
            .await?;
        self.commitment_state = commitment_state;
        Ok(())
    }

    async fn handle_block_received_from_data_availability(
        &mut self,
        block: SequencerBlockSubset,
    ) -> Result<()> {
        let sequencer_block_hash = block.block_hash;
        let maybe_execution_block_hash = self
            .sequencer_hash_to_execution_block
            .get(&sequencer_block_hash)
            .cloned();
        match maybe_execution_block_hash {
            Some(executed_block) => {
                // this case means block has already been executed.
                // setting execution chain's FIRM.
                self.update_commitment_state(
                    executed_block,
                    self.commitment_state.soft.clone().unwrap(),
                )
                .await?;
                // remove the sequencer block hash from the map, as it's been executed
                self.sequencer_hash_to_execution_block
                    .remove(&block.block_hash);
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
                let Some(executed_block) = self
                    .execute_block(block.clone())
                    .await
                    .wrap_err("failed to execute block")?
                else {
                    // no txs for our namespace, nothing to do
                    debug!("execute_block returned None; skipping call_update_commitment_state");
                    return Ok(());
                };
                // when we execute a block received from da, nothing else has been executed on top
                // of it, so we set FIRM and SOFT to this executed block
                self.update_commitment_state(executed_block.clone(), executed_block)
                    .await?;
                self.sequencer_hash_to_execution_block
                    .remove(&block.block_hash);
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashSet,
        sync::Arc,
    };

    use astria_proto::generated::execution::{
        v1alpha1::{
            DoBlockResponse,
            InitStateResponse,
        },
        v1alpha2::{
            BatchGetBlocksResponse,
            BlockIdentifier,
            CommitmentState,
        },
    };
    use prost_types::Timestamp;
    use sha2::Digest as _;
    use tokio::sync::Mutex;

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
    impl ExecutionClientV1Alpha1 for MockExecutionClient {
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

    #[async_trait::async_trait]
    impl ExecutionClientV1Alpha2 for MockExecutionClient {
        async fn call_batch_get_blocks(
            &mut self,
            _identifiers: Vec<BlockIdentifier>,
        ) -> Result<BatchGetBlocksResponse> {
            unimplemented!()
        }

        async fn call_execute_block(
            &mut self,
            prev_block_hash: Vec<u8>,
            _transactions: Vec<Vec<u8>>,
            timestamp: Option<ProstTimestamp>,
        ) -> Result<Block> {
            // returns the sha256 of the prev_block_hash
            let fake_next_hash = hash(&prev_block_hash);
            Ok(Block {
                number: 1,
                hash: fake_next_hash,
                parent_block_hash: prev_block_hash,
                timestamp,
            })
        }

        async fn call_get_block(&mut self, _identifier: BlockIdentifier) -> Result<Block> {
            unimplemented!()
        }

        async fn call_get_commitment_state(&mut self) -> Result<CommitmentState> {
            let timestamp = convert_tendermint_to_prost_timestamp(Time::now())
                .wrap_err("failed parsing str as protobuf timestamp")?;
            // NOTE - these are the same right now. we can change this if we want to test
            //  startup on a chain that already has blocks
            let block = Block {
                number: 1,
                hash: hash(b"block1"),
                parent_block_hash: hash(b"block0"),
                timestamp: Some(timestamp),
            };
            Ok(CommitmentState {
                soft: Some(block.clone()),
                firm: Some(block),
            })
        }

        async fn call_update_commitment_state(
            &mut self,
            commitment_state: CommitmentState,
        ) -> Result<CommitmentState> {
            // using `finalized_blocks` as a proxy for the execution state
            // so that we can more easily make assertions in our tests
            self.finalized_blocks
                .lock()
                .await
                .insert(commitment_state.firm.clone().unwrap().hash);
            Ok(CommitmentState {
                soft: commitment_state.soft,
                firm: commitment_state.firm,
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
        let chain_id = ChainId::new(b"test".to_vec());
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), chain_id)
            .await
            .unwrap();

        let expected_execution_hash = hash(&executor.commitment_state.soft.clone().unwrap().hash);
        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        let executed_block = executor
            .execute_block(block)
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_execution_hash, executed_block.hash);
    }

    #[tokio::test]
    async fn execute_block_without_relevant_txs() {
        let chain_id = ChainId::new(b"test".to_vec());
        let (mut executor, _) = Executor::new(MockExecutionClient::new(), chain_id)
            .await
            .unwrap();

        let block = get_test_block_subset();
        let executed_block = executor.execute_block(block).await.unwrap();
        assert!(executed_block.is_none());
    }

    #[tokio::test]
    async fn handle_block_received_from_data_availability_not_yet_executed() {
        let chain_id = ChainId::new(b"test".to_vec());
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(execution_client, chain_id).await.unwrap();

        let mut block = get_test_block_subset();
        block.rollup_transactions.push(b"test_transaction".to_vec());

        // `hash(b"block1")` is the hash defined in the block from
        // `get_test_block_subset`, so we're hashing it again here
        // to mimic the mocked execute_block functionality
        let expected_execution_hash = hash(&hash(b"block1"));

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        let firm_hash = executor.commitment_state.firm.clone().unwrap().hash;
        // should have executed and finalized the block
        assert_eq!(finalized_blocks.lock().await.len(), 1);
        assert!(finalized_blocks.lock().await.get(&firm_hash).is_some());
        assert_eq!(expected_execution_hash, firm_hash);
        // should be empty because 1 block was executed and finalized, which deletes it from the map
        assert!(executor.sequencer_hash_to_execution_block.is_empty());
        // should have updated self.commitment_state.firm and self.commitment_state.soft to the
        // executed block
        assert_eq!(
            executor.commitment_state.firm.unwrap().hash,
            executor.commitment_state.soft.unwrap().hash
        );
    }

    #[tokio::test]
    async fn handle_block_received_from_data_availability_already_executed() {
        let chain_id = ChainId::new(b"test".to_vec());
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(execution_client, chain_id).await.unwrap();

        let block = get_test_block_subset();

        // this insertion simulates the block being executed on a previous run loop
        let initial_block_hash = hash(b"block1");
        let next_block = Block {
            number: 1,
            hash: hash(&initial_block_hash),
            parent_block_hash: initial_block_hash.clone(),
            timestamp: None,
        };
        executor
            .sequencer_hash_to_execution_block
            .insert(initial_block_hash.try_into().unwrap(), next_block);

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        // should be empty because 1 block was finalized, which deletes it from the map
        assert!(executor.sequencer_hash_to_execution_block.is_empty());
        // should have updated self.commitment_state.firm but soft stayed the same
        assert_ne!(
            executor.commitment_state.firm.unwrap().hash,
            executor.commitment_state.soft.unwrap().hash
        );
    }

    #[tokio::test]
    async fn handle_block_received_from_data_availability_no_relevant_transactions() {
        let chain_id = ChainId::new(b"test".to_vec());
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(execution_client, chain_id).await.unwrap();

        let block: SequencerBlockSubset = get_test_block_subset();
        let firm = executor.commitment_state.firm.clone().unwrap();
        let previous_execution_state = firm.hash.clone();

        executor
            .handle_block_received_from_data_availability(block)
            .await
            .unwrap();

        // should not have executed or finalized the block
        assert!(finalized_blocks.lock().await.is_empty());
        assert!(finalized_blocks.lock().await.get(&firm.hash).is_none());
        assert_eq!(
            previous_execution_state,
            executor.commitment_state.firm.unwrap().hash
        );
        // should be empty because nothing was executed
        assert!(executor.sequencer_hash_to_execution_block.is_empty());
    }
}
