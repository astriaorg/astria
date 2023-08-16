use std::collections::{
    BTreeMap,
    HashMap,
};

use astria_proto::execution::v1alpha1::DoBlockResponse;
use astria_sequencer_relayer::types::{
    get_namespace,
    Namespace,
    SequencerBlockData,
};
use color_eyre::eyre::{
    Result,
    WrapErr as _,
};
use prost_types::Timestamp as ProstTimestamp;
use tendermint::{
    block::Height,
    hash::Hash,
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
};
mod queue;

pub(crate) type JoinHandle = task::JoinHandle<Result<()>>;

/// The channel for sending commands to the executor task.
pub(crate) type Sender = UnboundedSender<ExecutorCommand>;
/// The channel the executor task uses to listen for commands.
type Receiver = UnboundedReceiver<ExecutorCommand>;

/// The ExecutorCommitLevel specifies the behavior of the Conductor when sending
/// transaction data and fork choice updates to the execution layer.
enum ExecutorCommitLevel {
    /// The default commit level. The Conductor will send all incoming sequencer blocks to the
    /// execution layer to allow the rollup to view pending state. All incoming
    /// blocks at height N are marked as `head`. When a block at height N+1 is
    /// received, that block is marked as the new `head` and the block's parent at
    /// height N is marked as `soft`/`safe`. Blocks are marked as `firm`/`final`
    /// when they are seen in the DA layer.
    Head,
    /// The most recent block in the chain is both `soft` and `head`. The
    /// conductor will not send a block (at height N) to the rollup until a new
    /// block (at height N+1) is received. Effectively all blocks seen by the
    /// rollup will not be reverted. Blocks are marked as `firm`/`final` when
    /// they are seen in the DA layer.
    Soft,
    /// The Conductor ignores all blocks gossiped from the sequencer and only
    /// pulls data from the DA. All blocks seen this way are immediately marked
    /// as `firm`/`final`.
    Firm,
}

impl Default for ExecutorCommitLevel {
    fn default() -> Self {
        Self::Head
    }
}

/// spawns a executor task and returns a tuple with the task's join handle
/// and the channel for sending commands to this executor
pub(crate) async fn spawn(conf: &Config, alert_tx: AlertSender) -> Result<(JoinHandle, Sender)> {
    info!("Spawning executor task.");
    let execution_rpc_client = ExecutionRpcClient::new(&conf.execution_rpc_url).await?;
    let (mut executor, executor_tx) = Executor::new(
        execution_rpc_client,
        get_namespace(conf.chain_id.as_bytes()),
        alert_tx,
        // TODO (GHI 250: https://github.com/astriaorg/astria/issues/250): make this configurable using values from the config
        ExecutorCommitLevel::default(),
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

// enum NextBlockStatus {
//     IsNext,
//     NotNext,
//     NoParent,
// }

#[derive(Debug)]
pub enum ExecutorCommand {
    /// used when a block is received from the gossip network
    BlockReceivedFromGossipNetwork {
        block: Box<SequencerBlockData>,
    },
    /// used when a block is received from the reader (Celestia)
    BlockReceivedFromDataAvailability {
        block: Box<SequencerBlockData>,
    },
    Shutdown,
}

struct Executor<C> {
    /// The commit level of the executor.
    commit_level: ExecutorCommitLevel,

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
    sequencer_hash_to_execution_hash: HashMap<Vec<u8>, Vec<u8>>,

    /// most recently executed sequencer block hash
    // TODO pending (GHI-220: https://github.com/astriaorg/astria/issues/220): reevaluate this after 205 gets merged
    last_executed_seq_block_hash: Hash,

    /// block queue for blocks that have been recieved but their parent has not been executed yet
    // block_queue: BTreeMap<Height, SequencerBlockData>,
    block_queue: ExecutorQueue,
}

impl<C: ExecutionClient> Executor<C> {
    async fn new(
        mut execution_rpc_client: C,
        namespace: Namespace,
        alert_tx: AlertSender,
        commit_level: ExecutorCommitLevel,
    ) -> Result<(Self, Sender)> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let init_state_response = execution_rpc_client.call_init_state().await?;
        let execution_state = init_state_response.block_hash;
        Ok((
            Self {
                commit_level,
                cmd_rx,
                execution_rpc_client,
                namespace,
                alert_tx,
                execution_state,
                sequencer_hash_to_execution_hash: HashMap::new(),
                last_executed_seq_block_hash: Hash::default(),
                // block_queue: BTreeMap::new(),
                block_queue: ExecutorQueue::new(),
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
                        block_height: block.header.height.value(),
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

    /// This function checks if the given block has already been executed by checking the
    /// `sequencer_hash_to_execution_hash` map.
    fn check_if_previously_executed(&mut self, block: SequencerBlockData) -> Option<Vec<u8>> {
        if let Some(execution_hash) = self.sequencer_hash_to_execution_hash.get(&block.block_hash) {
            debug!(
                height = block.header.height.value(),
                execution_hash = hex::encode(execution_hash),
                "block already executed"
            );
            return Some(execution_hash.clone());
        }
        None
    }

    /// This function checks if the given block can be executed because its parent is the last
    /// executed block.
    // fn is_next_block(&mut self, block: SequencerBlockData) -> NextBlockStatus {
    //     // before a block can be added to the queue, it must have a parent block Id
    //     if let Some(parent_block) = block.header.last_block_id {
    //         if parent_block.hash != self.last_executed_seq_block_hash {
    //             NextBlockStatus::NotNext
    //         } else {
    //             NextBlockStatus::IsNext
    //         }
    //     } else {
    //         NextBlockStatus::NoParent
    //     }
    // }

    /// This function takes a given sequencer block and returns the relevant transactions for the
    /// executor's namespace.
    fn get_transacions(&mut self, mut block: SequencerBlockData) -> Option<Vec<Vec<u8>>> {
        let Some(txs) = block.rollup_txs.remove(&self.namespace) else {
            info!(
                height = block.header.height.value(),
                "sequencer block did not contains txs for namespace"
            );
            return None;
        };
        Some(txs.into_iter().map(|tx| tx.transaction).collect::<Vec<_>>())
    }

    /// This function takes a given sequencer block, filters out the relevant transactions, and
    /// sends it to the execution client.
    async fn execute_single_block(
        &mut self,
        block: SequencerBlockData,
    ) -> Result<Option<DoBlockResponse>> {
        // get transactions for the executor's namespace
        let Some(txs) = self.get_transacions(block.clone()) else {
            return Ok(None);
        };

        // get the previous execution block hash
        let prev_execution_block_hash = self.execution_state.clone();

        // get the block timestamp
        let timestamp = convert_tendermint_to_prost_timestamp(block.header.time)
            .wrap_err("failed parsing str as protobuf timestamp")?;

        info!(
            height = block.header.height.value(),
            parent_block_hash = hex::encode(&prev_execution_block_hash),
            "executing block with given parent block",
        );

        // send transactions to the execution client
        let response = self
            .execution_rpc_client
            // TODO pending (GHI-205: https://github.com/astriaorg/astria/issues/202): update for api upgrade
            .call_do_block(prev_execution_block_hash, txs, Some(timestamp))
            .await?;

        // get the execution state from the response and save it
        self.execution_state = response.block_hash.clone();

        // set the last executed sequencer block hash
        // TODO pending (GHI-220: https://github.com/astriaorg/astria/issues/220): reevaluate this after 205 gets merged
        if let Ok(last_hash) = Hash::try_from(block.block_hash.clone()) {
            self.last_executed_seq_block_hash = last_hash;
        };

        // store block hash returned by execution client, as we need it to finalize the block later
        self.sequencer_hash_to_execution_hash
            .insert(block.block_hash, response.block_hash.clone());

        Ok(Some(response))
    }

    /// This function tries to execute all blocks in the block queue that have been recieved.
    // async fn try_execute_queue(&mut self) -> Result<Option<DoBlockResponse>> {
    //     let mut response = DoBlockResponse::default();
    //     if !self.block_queue.is_empty() {
    //         // try executing all blocks in the block queue that have been recieved
    //         while let Some((&height, qblock)) = self.block_queue.first_key_value() {
    //             // can use unwrap here because block can't be added to the queue without a parent
    //             // hash. see is_next_block()
    //             if qblock.header.last_block_id.unwrap().hash == self.last_executed_seq_block_hash
    // {                 response = match self.execute_single_block((*qblock).clone()).await? {
    //                     Some(response) => response,
    //                     None => return Ok(None),
    //                 };

    //                 while self.block_queue.remove(&height).is_some() {}
    //             } else {
    //                 break;
    //             }
    //         }
    //         Ok(Some(response))
    //     } else {
    //         Ok(None)
    //     }
    // }

    /// This function takes a given sequencer block, filters out the relevant transactions, and
    /// sends it to the execution client.
    ///
    /// Relevant transations are pulled from the block and they are sent via the
    /// execution service's DoBlock function for attempted execution. If the transactions
    /// successfully execute, it returns the resulting execution block hash. If multiple blocks are
    /// executed is returns the most recent execution hash. If the block has already been
    /// executed, it returns the previously-computed execution block hash. If there
    /// are no relevant transactions in the SequencerBlock, it returns None.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - the call to the execution service's DoBlock function fails
    async fn execute_block(&mut self, block: SequencerBlockData) -> Result<Option<Vec<u8>>> {
        // check if the block has already been executed
        if let Some(execution_hash) = self.check_if_previously_executed(block.clone()) {
            return Ok(Some(execution_hash));
        }
        // check if the incoming block is the next block. if it's not, add it to
        // the execution queue
        // TODO: probably best to just always add stuff to the queue and then
        // always pull from the queue when trying to execute blocks. that will
        // probably keep the logic fairly simple but may incur some slight
        // performance hits?
        // match self.is_next_block(block.clone()) {
        //     NextBlockStatus::IsNext => {}
        //     NextBlockStatus::NotNext => {
        //         self.block_queue.insert(block.header.height, block.clone());
        //         debug!(
        //             height = block.header.height.value(),
        //             "parent block not yet executed, adding to pending queue, execution state not
        // \              updated"
        //         );
        //         return Ok(Some(self.execution_state.clone()));
        //     }
        //     // TODO: not sure what to do with this... but it's needed to pass preexisting tests
        //     NextBlockStatus::NoParent => {}
        // }
        self.block_queue.insert(block.clone());

        // execute the block that just arrived
        // let mut response = match self.execute_single_block(block.clone()).await? {
        //     Some(response) => response,
        //     None => return Ok(None),
        // };
        // TODO: this is causing a bug where the thing returns zero
        let mut response = DoBlockResponse::default();

        // try executing blocks in the queue now that we have a new block that may have filled gaps
        // in the blocks received
        if !self.block_queue.is_empty() {
            if let Some(blocks_for_execution) = self.block_queue.get_blocks() {
                for block in blocks_for_execution {
                    response = match self.execute_single_block(block.clone()).await? {
                        Some(response) => response,
                        None => return Ok(None),
                    };
                }
            }
            // response = match self.try_execute_queue().await? {
            //     Some(response) => response,
            //     None => return Ok(None),
            // };
        }

        info!(
            sequencer_block_hash = ?block.block_hash,
            sequencer_block_height = block.header.height.value(),
            execution_block_hash = hex::encode(&response.block_hash),
            "executed sequencer block",
        );

        Ok(Some(response.block_hash))
    }

    async fn handle_block_received_from_data_availability(
        &mut self,
        block: SequencerBlockData,
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
                self.finalize_block(execution_block_hash, &sequencer_block_hash)
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
        sequencer_block_hash: &[u8],
    ) -> Result<()> {
        self.execution_rpc_client
            .call_finalize_block(execution_block_hash)
            .await
            .wrap_err("failed to finalize block")?;
        self.sequencer_hash_to_execution_hash
            .remove(sequencer_block_hash);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::HashSet,
        sync::Arc,
    };

    use astria_proto::execution::v1alpha1::{
        DoBlockResponse,
        InitStateResponse,
    };
    use astria_sequencer_relayer::types::IndexedTransaction;
    use prost_types::Timestamp;
    use sha2::Digest as _;
    use tendermint::block::Id as BlockId;
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

    fn get_test_block() -> SequencerBlockData {
        SequencerBlockData {
            block_hash: hash(b"block1"),
            header: astria_sequencer_relayer::utils::default_header(),
            last_commit: None,
            rollup_txs: HashMap::new(),
        }
    }

    fn get_test_block_vec(num_blocks: u32) -> Vec<SequencerBlockData> {
        let namespace = get_namespace(b"test");
        let mut block = get_test_block();
        block.rollup_txs.insert(
            namespace,
            vec![IndexedTransaction {
                block_index: 0,
                transaction: b"test_transaction".to_vec(),
            }],
        );

        let mut blocks = vec![];

        block.header.height = 1_u32.into();
        blocks.push(block);

        for i in 2..=num_blocks {
            let current_hash_string = String::from("block") + &i.to_string();
            let prev_hash_string = String::from("block") + &(i - 1).to_string();
            let current_byte_hash: &[u8] = &current_hash_string.into_bytes();
            let prev_byte_hash: &[u8] = &prev_hash_string.into_bytes();

            let mut block = SequencerBlockData {
                block_hash: hash(current_byte_hash),
                header: astria_sequencer_relayer::utils::default_header(),
                last_commit: None,
                rollup_txs: HashMap::new(),
            };
            block.rollup_txs.insert(
                namespace,
                vec![IndexedTransaction {
                    block_index: 0,
                    transaction: b"test_transaction".to_vec(),
                }],
            );
            block.header.height = i.into();
            let block_id = BlockId {
                hash: Hash::try_from(hash(prev_byte_hash)).unwrap(),
                ..Default::default()
            };
            block.header.last_block_id = Some(block_id);

            blocks.push(block);
        }
        blocks
    }

    #[tokio::test]
    async fn test_block_queue() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = get_namespace(b"test");
        let (mut executor, _) = Executor::new(
            MockExecutionClient::new(),
            namespace,
            alert_tx,
            ExecutorCommitLevel::default(),
        )
        .await
        .unwrap();

        let blocks = get_test_block_vec(10);

        // executing a block like normal
        let mut expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash_0 = executor
            .execute_block(blocks[0].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_0);

        // adding a block without a parent in the execution chain doesn't change the execution
        // state and adds it to the queue
        let execution_block_hash_2 = executor
            .execute_block(blocks[2].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_2);
        assert_eq!(executor.block_queue.len(), 1);

        // adding another block without a parent in the current chain (but does have parent in the
        // queue). also doesn't change execution state
        let execution_block_hash_3 = executor
            .execute_block(blocks[3].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_3);
        assert_eq!(executor.block_queue.len(), 2);

        // adding the actual next block updates the execution state
        // using hash() 3 times here because adding the new block and executing the queue updates
        // the state 3 times. one for each block.
        expected_exection_hash = hash(&hash(&hash(&executor.execution_state)));
        let execution_block_hash_1 = executor
            .execute_block(blocks[1].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_1);
        // and the queue gets executed and cleared
        assert_eq!(executor.block_queue.len(), 0);

        // a new block with a parent appears and is executed
        expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash_4 = executor
            .execute_block(blocks[4].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_4);

        // add another block that doesn't have a parent
        let execution_block_hash_6 = executor
            .execute_block(blocks[6].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        // exectuion hash not updated
        assert_eq!(expected_exection_hash, execution_block_hash_6);
        assert_eq!(executor.block_queue.len(), 1);
        // add in the same block again with a newer timestamp
        let mut newer_6_block = blocks[6].clone();
        newer_6_block.header.time = Time::now();
        let execution_block_hash_6 = executor
            .execute_block(newer_6_block)
            .await
            .unwrap()
            .expect("expected execution block hash");
        // exectuion hash not updated
        assert_eq!(expected_exection_hash, execution_block_hash_6);
        // the newer block replaces the block of the same height in the queue so the queue doesn't
        // grow
        assert_eq!(executor.block_queue.len(), 1);

        // add another block that doesn't have a parent and also a gap between the last block added
        // to the queue that doesn't have a parent
        let execution_block_hash_8 = executor
            .execute_block(blocks[8].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        // exectuion hash not updated
        assert_eq!(expected_exection_hash, execution_block_hash_8);
        assert_eq!(executor.block_queue.len(), 2);

        // add a block that fills the first gap
        // in this case there are two 6 blocks but one is newer
        // only the latest block gets executed here and the old 6 block just gets deleted
        expected_exection_hash = hash(&hash(&executor.execution_state)); // 2 blocks executed
        let execution_block_hash_5 = executor
            .execute_block(blocks[5].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_5);
        // only one block in the queue gets executed because there is still a gap
        assert_eq!(executor.block_queue.len(), 1);

        // add a block that fills the second gap
        expected_exection_hash = hash(&hash(&executor.execution_state));
        let execution_block_hash_7 = executor
            .execute_block(blocks[7].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_7);
        // the rest of the queue is executed because all gaps are filled
        assert_eq!(executor.block_queue.len(), 0);

        // one final block executed like normal
        expected_exection_hash = hash(&executor.execution_state);
        let execution_block_hash_9 = executor
            .execute_block(blocks[9].clone())
            .await
            .unwrap()
            .expect("expected execution block hash");
        assert_eq!(expected_exection_hash, execution_block_hash_9);
        assert_eq!(executor.block_queue.len(), 0);
    }

    #[tokio::test]
    async fn execute_block_with_relevant_txs() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = get_namespace(b"test");
        let (mut executor, _) = Executor::new(
            MockExecutionClient::new(),
            namespace,
            alert_tx,
            ExecutorCommitLevel::default(),
        )
        .await
        .unwrap();

        let expected_exection_hash = hash(&executor.execution_state);
        let mut block = get_test_block();
        block.rollup_txs.insert(
            namespace,
            vec![IndexedTransaction {
                block_index: 0,
                transaction: b"test_transaction".to_vec(),
            }],
        );

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
        let namespace = get_namespace(b"test");
        let (mut executor, _) = Executor::new(
            MockExecutionClient::new(),
            namespace,
            alert_tx,
            ExecutorCommitLevel::default(),
        )
        .await
        .unwrap();

        let block = get_test_block();
        let execution_block_hash = executor.execute_block(block).await.unwrap();
        assert!(execution_block_hash.is_none());
    }

    #[tokio::test]
    async fn handle_block_received_from_data_availability_not_yet_executed() {
        let (alert_tx, _) = mpsc::unbounded_channel();
        let namespace = get_namespace(b"test");
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(
            execution_client,
            namespace,
            alert_tx,
            ExecutorCommitLevel::default(),
        )
        .await
        .unwrap();

        let mut block: SequencerBlockData = get_test_block();
        block.rollup_txs.insert(
            namespace,
            vec![IndexedTransaction {
                block_index: 0,
                transaction: b"test_transaction".to_vec(),
            }],
        );

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
        let namespace = get_namespace(b"test");
        let finalized_blocks = Arc::new(Mutex::new(HashSet::new()));
        let execution_client = MockExecutionClient {
            finalized_blocks: finalized_blocks.clone(),
        };
        let (mut executor, _) = Executor::new(
            execution_client,
            namespace,
            alert_tx,
            ExecutorCommitLevel::default(),
        )
        .await
        .unwrap();

        let block: SequencerBlockData = get_test_block();
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
