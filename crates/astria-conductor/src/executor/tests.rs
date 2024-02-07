use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        Mutex,
    },
};

use ::optimism::test_utils::deploy_mock_optimism_portal;
use astria_core::{
    generated::execution::v1alpha2::{
        execution_service_server::{
            ExecutionService,
            ExecutionServiceServer,
        },
        BatchGetBlocksRequest,
        BatchGetBlocksResponse,
        Block,
        CommitmentState,
        ExecuteBlockRequest,
        GenesisInfo,
        GetBlockRequest,
        GetCommitmentStateRequest,
        GetGenesisInfoRequest,
        UpdateCommitmentStateRequest,
    },
    sequencer::v1alpha1::test_utils::{
        make_cometbft_block,
        ConfigureCometBftBlock,
    },
};
use ethers::{
    prelude::{
        k256::ecdsa::SigningKey,
        Middleware as _,
    },
    utils::AnvilInstance,
};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
};
use tonic::transport::Server;

use super::{
    optimism,
    Client,
    Executor,
    ReconstructedBlock,
    RollupId,
    SequencerBlock,
    SequencerHeight,
};

const GENESIS_HASH: [u8; 32] = [0u8; 32];

#[derive(Debug)]
struct MockExecutionServer {
    _server_handle: JoinHandle<()>,
    local_addr: SocketAddr,
}

impl MockExecutionServer {
    async fn spawn() -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(ExecutionServiceServer::new(ExecutionServiceImpl::new()))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await;
        });
        Self {
            _server_handle: server_handle,
            local_addr,
        }
    }

    fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

fn make_genesis_block() -> Block {
    Block {
        number: 0,
        hash: GENESIS_HASH.to_vec(),
        parent_block_hash: GENESIS_HASH.to_vec(),
        timestamp: Some(std::time::SystemTime::now().into()),
    }
}

struct ExecutionServiceImpl {
    hash_to_number: Mutex<HashMap<[u8; 32], u32>>,
    commitment_state: Mutex<CommitmentState>,
    genesis_info: Mutex<GenesisInfo>,
}

impl ExecutionServiceImpl {
    fn new() -> Self {
        let mut hash_to_number = HashMap::new();
        // insert a genesis number/block here
        hash_to_number.insert(GENESIS_HASH, 0);
        Self {
            hash_to_number: hash_to_number.into(),
            commitment_state: CommitmentState {
                soft: Some(make_genesis_block()),
                firm: Some(make_genesis_block()),
            }
            .into(),
            genesis_info: GenesisInfo {
                rollup_id: vec![42u8; 32],
                sequencer_genesis_block_height: 100,
                celestia_base_block_height: 1,
                celestia_block_variance: 1,
            }
            .into(),
        }
    }
}

#[tonic::async_trait]
impl ExecutionService for ExecutionServiceImpl {
    async fn get_block(
        &self,
        _request: tonic::Request<GetBlockRequest>,
    ) -> std::result::Result<tonic::Response<Block>, tonic::Status> {
        unimplemented!("get_block")
    }

    async fn get_genesis_info(
        &self,
        _request: tonic::Request<GetGenesisInfoRequest>,
    ) -> std::result::Result<tonic::Response<GenesisInfo>, tonic::Status> {
        Ok(tonic::Response::new(
            self.genesis_info.lock().unwrap().clone(),
        ))
    }

    async fn batch_get_blocks(
        &self,
        _request: tonic::Request<BatchGetBlocksRequest>,
    ) -> std::result::Result<tonic::Response<BatchGetBlocksResponse>, tonic::Status> {
        unimplemented!("batch_get_blocks")
    }

    async fn execute_block(
        &self,
        request: tonic::Request<ExecuteBlockRequest>,
    ) -> std::result::Result<tonic::Response<Block>, tonic::Status> {
        let request = request.into_inner();
        let parent_block_hash: [u8; 32] = request.prev_block_hash.try_into().unwrap();
        let hash = get_expected_execution_hash(&parent_block_hash, &request.transactions);
        let new_number = {
            let mut guard = self.hash_to_number.lock().unwrap();
            let new_number = guard.get(&parent_block_hash).unwrap() + 1;
            guard.insert(hash, new_number);
            new_number
        };

        let timestamp = request.timestamp.unwrap_or_default();
        Ok(tonic::Response::new(Block {
            number: new_number,
            hash: hash.to_vec(),
            parent_block_hash: parent_block_hash.to_vec(),
            timestamp: Some(timestamp),
        }))
    }

    async fn get_commitment_state(
        &self,
        _request: tonic::Request<GetCommitmentStateRequest>,
    ) -> std::result::Result<tonic::Response<CommitmentState>, tonic::Status> {
        Ok(tonic::Response::new(
            self.commitment_state.lock().unwrap().clone(),
        ))
    }

    async fn update_commitment_state(
        &self,
        request: tonic::Request<UpdateCommitmentStateRequest>,
    ) -> std::result::Result<tonic::Response<CommitmentState>, tonic::Status> {
        let new_state = {
            let mut guard = self.commitment_state.lock().unwrap();
            *guard = request.into_inner().commitment_state.unwrap();
            guard.clone()
        };
        Ok(tonic::Response::new(new_state))
    }
}

fn get_expected_execution_hash(parent_block_hash: &[u8], transactions: &[Vec<u8>]) -> [u8; 32] {
    hash(&[parent_block_hash, &transactions.concat()].concat())
}

fn hash(s: &[u8]) -> [u8; 32] {
    use sha2::{
        Digest as _,
        Sha256,
    };
    Sha256::digest(s).into()
}

fn make_reconstructed_block() -> ReconstructedBlock {
    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(100u32);
    ReconstructedBlock {
        block_hash: hash(b"block1"),
        header: block.header,
        transactions: vec![],
    }
}

struct MockEnvironment {
    _server: MockExecutionServer,
    _shutdown_tx: oneshot::Sender<()>,
    executor: Executor,
    client: Client,
}

async fn start_mock(pre_execution_hook: Option<optimism::Handler>) -> MockEnvironment {
    let server = MockExecutionServer::spawn().await;
    let server_url = format!("http://{}", server.local_addr());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let (executor, _) = Executor::builder()
        .rollup_address(&server_url)
        .unwrap()
        .shutdown(shutdown_rx)
        .set_optimism_hook(pre_execution_hook)
        .build();

    let client = Client::connect(executor.rollup_address.clone())
        .await
        .unwrap();

    executor
        .set_initial_node_state(client.clone())
        .await
        .unwrap();

    MockEnvironment {
        _server: server,
        _shutdown_tx: shutdown_tx,
        executor,
        client,
    }
}

struct MockEnvironmentWithEthereum {
    environment: MockEnvironment,
    optimism_portal_address: ethers::prelude::Address,
    provider: Arc<ethers::prelude::Provider<ethers::prelude::Ws>>,
    wallet: ethers::prelude::Wallet<SigningKey>,
    anvil: AnvilInstance,
}

async fn start_mock_with_optimism_handler() -> MockEnvironmentWithEthereum {
    let (contract_address, provider, wallet, anvil) = deploy_mock_optimism_portal().await;

    let pre_execution_hook = Some(optimism::Handler::new(
        provider.clone(),
        contract_address,
        1,
    ));
    MockEnvironmentWithEthereum {
        environment: start_mock(pre_execution_hook).await,
        optimism_portal_address: contract_address,
        provider,
        wallet,
        anvil,
    }
}

#[tokio::test]
async fn firm_blocks_at_expected_heights_are_executed() {
    let mut mock = start_mock(None).await;

    let mut block = make_reconstructed_block();
    block.transactions.push(b"test_transaction".to_vec());

    let expected_exection_hash = get_expected_execution_hash(
        &mock.executor.state.borrow().firm().hash(),
        &block.transactions,
    );

    mock.executor
        .execute_firm(mock.client.clone(), block)
        .await
        .unwrap();
    assert_eq!(
        expected_exection_hash,
        mock.executor.state.borrow().firm().hash(),
    );

    let mut block = make_reconstructed_block();
    block.header.height = block.header.height.increment();
    block.transactions.push(b"a new transaction".to_vec());
    let expected_exection_hash = get_expected_execution_hash(
        &mock.executor.state.borrow().firm().hash(),
        &block.transactions,
    );

    mock.executor
        .execute_firm(mock.client.clone(), block)
        .await
        .unwrap();
    assert_eq!(
        expected_exection_hash,
        mock.executor.state.borrow().firm().hash(),
    );
}

#[tokio::test]
async fn soft_blocks_at_expected_heights_are_executed() {
    let mut mock = start_mock(None).await;

    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(100u32);
    let block = SequencerBlock::try_from_cometbft(block).unwrap();
    assert!(
        mock.executor
            .execute_soft(mock.client.clone(), block)
            .await
            .is_ok()
    );

    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(101u32);
    let block = SequencerBlock::try_from_cometbft(block).unwrap();
    mock.executor
        .execute_soft(mock.client.clone(), block)
        .await
        .unwrap();
    assert_eq!(
        SequencerHeight::from(102u32),
        mock.executor.state.borrow().next_soft_sequencer_height()
    );
}

#[tokio::test]
async fn first_firm_then_soft_leads_to_soft_being_dropped() {
    let mut mock = start_mock(None).await;

    let rollup_id = RollupId::new([42u8; 32]);
    let block = ConfigureCometBftBlock {
        height: 100,
        rollup_transactions: vec![(rollup_id, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let soft_block = SequencerBlock::try_from_cometbft(block).unwrap();

    let block_hash = soft_block.block_hash();

    let firm_block = ReconstructedBlock {
        block_hash: soft_block.block_hash(),
        header: soft_block.header().clone(),
        transactions: soft_block
            .rollup_transactions()
            .get(&rollup_id)
            .cloned()
            .unwrap(),
    };
    mock.executor
        .execute_firm(mock.client.clone(), firm_block)
        .await
        .unwrap();
    assert_eq!(1, mock.executor.state.borrow().firm().number());
    assert_eq!(1, mock.executor.state.borrow().soft().number());
    assert!(
        !mock
            .executor
            .blocks_pending_finalization
            .contains_key(&block_hash)
    );

    mock.executor
        .execute_soft(mock.client.clone(), soft_block)
        .await
        .unwrap();
    assert!(
        !mock
            .executor
            .blocks_pending_finalization
            .contains_key(&block_hash)
    );
    assert_eq!(1, mock.executor.state.borrow().firm().number());
    assert_eq!(1, mock.executor.state.borrow().soft().number());
}

#[tokio::test]
async fn first_soft_then_firm_update_state_correctly() {
    let mut mock = start_mock(None).await;

    let rollup_id = RollupId::new([42u8; 32]);
    let block = ConfigureCometBftBlock {
        height: 100,
        rollup_transactions: vec![(rollup_id, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let soft_block = SequencerBlock::try_from_cometbft(block).unwrap();

    let block_hash = soft_block.block_hash();

    let firm_block = ReconstructedBlock {
        block_hash: soft_block.block_hash(),
        header: soft_block.header().clone(),
        transactions: soft_block
            .rollup_transactions()
            .get(&rollup_id)
            .cloned()
            .unwrap(),
    };
    mock.executor
        .execute_soft(mock.client.clone(), soft_block)
        .await
        .unwrap();
    assert!(
        mock.executor
            .blocks_pending_finalization
            .contains_key(&block_hash)
    );
    assert_eq!(0, mock.executor.state.borrow().firm().number());
    assert_eq!(1, mock.executor.state.borrow().soft().number());
    mock.executor
        .execute_firm(mock.client.clone(), firm_block)
        .await
        .unwrap();
    assert_eq!(1, mock.executor.state.borrow().firm().number());
    assert_eq!(1, mock.executor.state.borrow().soft().number());
    assert!(
        !mock
            .executor
            .blocks_pending_finalization
            .contains_key(&block_hash)
    );
}

#[tokio::test]
async fn old_soft_blocks_are_ignored() {
    let mut mock = start_mock(None).await;
    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(99u32);
    let sequencer_block = SequencerBlock::try_from_cometbft(block).unwrap();

    let firm = mock.executor.state.borrow().firm().clone();
    let soft = mock.executor.state.borrow().soft().clone();

    mock.executor
        .execute_soft(mock.client.clone(), sequencer_block)
        .await
        .unwrap();

    assert_eq!(&firm, mock.executor.state.borrow().firm());
    assert_eq!(&soft, mock.executor.state.borrow().soft());
}

#[tokio::test]
async fn non_sequential_future_soft_blocks_give_error() {
    let mut mock = start_mock(None).await;

    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(101u32);
    let sequencer_block = SequencerBlock::try_from_cometbft(block).unwrap();
    assert!(
        mock.executor
            .execute_soft(mock.client.clone(), sequencer_block)
            .await
            .is_err()
    );

    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(100u32);
    let sequencer_block = SequencerBlock::try_from_cometbft(block).unwrap();
    assert!(
        mock.executor
            .execute_soft(mock.client.clone(), sequencer_block)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn out_of_order_firm_blocks_are_rejected() {
    let mut mock = start_mock(None).await;
    let mut block = make_reconstructed_block();

    block.header.height = SequencerHeight::from(99u32);
    assert!(
        mock.executor
            .execute_firm(mock.client.clone(), block.clone())
            .await
            .is_err()
    );

    block.header.height = SequencerHeight::from(101u32);
    assert!(
        mock.executor
            .execute_firm(mock.client.clone(), block.clone())
            .await
            .is_err()
    );

    block.header.height = SequencerHeight::from(100u32);
    assert!(
        mock.executor
            .execute_firm(mock.client.clone(), block.clone())
            .await
            .is_ok()
    );
}

mod optimism_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "install solc-select and foundry-rs and run with --ignored"]
    async fn deposit_events_are_converted_and_executed() {
        use ::optimism::contract::*;

        // make a deposit transaction
        let MockEnvironmentWithEthereum {
            environment: mut mock,
            optimism_portal_address: contract_address,
            provider,
            wallet,
            anvil: _anvil,
        } = start_mock_with_optimism_handler().await;
        let contract = make_optimism_portal_with_signer(provider.clone(), wallet, contract_address);
        let to = ethers::prelude::Address::zero();
        let value = ethers::prelude::U256::from(100);
        let receipt = make_deposit_transaction(&contract, Some(to), value, None)
            .await
            .unwrap()
            .unwrap();
        assert!(receipt.status.unwrap().as_u64() == 1);

        // get the event and the expected deposit transaction
        let to_block = provider.get_block_number().await.unwrap();
        let event_filter = contract
            .event::<TransactionDepositedFilter>()
            .from_block(1)
            .to_block(to_block);

        let events = event_filter.query_with_meta().await.unwrap();

        let deposit_txs =
            crate::executor::optimism::convert_deposit_events_to_encoded_txs(events).unwrap();

        // calculate the expected mock execution hash, which includes the block txs,
        // thus confirming the deposit tx was executed
        let expected_exection_hash =
            get_expected_execution_hash(&mock.executor.state.borrow().soft().hash(), &deposit_txs);
        let block = make_reconstructed_block();
        mock.executor
            .execute_firm(mock.client.clone(), block)
            .await
            .unwrap();
        assert_eq!(
            expected_exection_hash,
            mock.executor.state.borrow().firm().hash(),
        );
    }
}
