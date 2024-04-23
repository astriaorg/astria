use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        Mutex,
    },
};

use astria_core::{
    execution::v1alpha2::{
        Block,
        CommitmentState,
        GenesisInfo,
    },
    generated::{
        execution::v1alpha2::{
            self as raw,
            execution_service_server::{
                ExecutionService,
                ExecutionServiceServer,
            },
            BatchGetBlocksRequest,
            BatchGetBlocksResponse,
            ExecuteBlockRequest,
            GetBlockRequest,
            GetCommitmentStateRequest,
            GetGenesisInfoRequest,
            UpdateCommitmentStateRequest,
        },
        sequencerblock::v1alpha1::RollupData as RawRollupData,
    },
    protocol::test_utils::{
        make_cometbft_block,
        ConfigureCometBftBlock,
    },
    sequencerblock::v1alpha1::{
        block::RollupData,
        SequencerBlock,
    },
    Protobuf as _,
};
use bytes::Bytes;
use prost::Message;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;

use super::{
    Client,
    Executor,
    ReconstructedBlock,
    RollupId,
    SequencerHeight,
};

// Bytes provides an escape hatch for interior mutability.
// That's not good in general but acceptable in these tests.
#[allow(clippy::declare_interior_mutable_const)]
const GENESIS_HASH: Bytes = Bytes::from_static(&[0u8; 32]);

const ROLLUP_ID: RollupId = RollupId::new([42u8; 32]);

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

fn make_genesis_block() -> raw::Block {
    raw::Block {
        number: 0,
        hash: GENESIS_HASH,
        parent_block_hash: GENESIS_HASH,
        timestamp: Some(chrono::Utc::now().into()),
    }
}

struct ExecutionServiceImpl {
    hash_to_number: Mutex<HashMap<Bytes, u32>>,
    commitment_state: Mutex<raw::CommitmentState>,
    genesis_info: Mutex<raw::GenesisInfo>,
}

impl ExecutionServiceImpl {
    fn new() -> Self {
        let mut hash_to_number = HashMap::new();
        // insert a genesis number/block here
        hash_to_number.insert(GENESIS_HASH, 0);
        Self {
            hash_to_number: hash_to_number.into(),
            commitment_state: raw::CommitmentState {
                soft: Some(make_genesis_block()),
                firm: Some(make_genesis_block()),
            }
            .into(),
            genesis_info: raw::GenesisInfo {
                rollup_id: vec![42u8; 32].into(),
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
        self: Arc<Self>,
        _request: tonic::Request<GetBlockRequest>,
    ) -> std::result::Result<tonic::Response<raw::Block>, tonic::Status> {
        unimplemented!("get_block")
    }

    async fn get_genesis_info(
        self: Arc<Self>,
        _request: tonic::Request<GetGenesisInfoRequest>,
    ) -> std::result::Result<tonic::Response<raw::GenesisInfo>, tonic::Status> {
        Ok(tonic::Response::new(
            self.genesis_info.lock().unwrap().clone(),
        ))
    }

    async fn batch_get_blocks(
        self: Arc<Self>,
        _request: tonic::Request<BatchGetBlocksRequest>,
    ) -> std::result::Result<tonic::Response<BatchGetBlocksResponse>, tonic::Status> {
        unimplemented!("batch_get_blocks")
    }

    async fn execute_block(
        self: Arc<Self>,
        request: tonic::Request<ExecuteBlockRequest>,
    ) -> std::result::Result<tonic::Response<raw::Block>, tonic::Status> {
        let request = request.into_inner();
        let parent_block_hash: Bytes = request.prev_block_hash.clone();
        let hash = get_expected_execution_hash(&parent_block_hash, request.transactions);
        let new_number = {
            let mut guard = self.hash_to_number.lock().unwrap();
            let new_number = guard.get(&parent_block_hash).unwrap() + 1;
            guard.insert(hash.clone(), new_number);
            new_number
        };

        let timestamp = request.timestamp.unwrap_or_default();
        Ok(tonic::Response::new(raw::Block {
            number: new_number,
            hash,
            parent_block_hash,
            timestamp: Some(timestamp),
        }))
    }

    async fn get_commitment_state(
        self: Arc<Self>,
        _request: tonic::Request<GetCommitmentStateRequest>,
    ) -> std::result::Result<tonic::Response<raw::CommitmentState>, tonic::Status> {
        Ok(tonic::Response::new(
            self.commitment_state.lock().unwrap().clone(),
        ))
    }

    async fn update_commitment_state(
        self: Arc<Self>,
        request: tonic::Request<UpdateCommitmentStateRequest>,
    ) -> std::result::Result<tonic::Response<raw::CommitmentState>, tonic::Status> {
        let new_state = {
            let mut guard = self.commitment_state.lock().unwrap();
            *guard = request.into_inner().commitment_state.unwrap();
            guard.clone()
        };
        Ok(tonic::Response::new(new_state))
    }
}

fn get_expected_execution_hash(
    parent_block_hash: &Bytes,
    transactions: Vec<RawRollupData>,
) -> Bytes {
    use prost::Message as _;
    use sha2::Digest as _;

    let mut hasher = sha2::Sha256::new();
    hasher.update(parent_block_hash);
    for tx in transactions {
        hasher.update(tx.encode_to_vec());
    }
    Bytes::copy_from_slice(&hasher.finalize())
}

fn make_reconstructed_block(height: u32) -> ReconstructedBlock {
    let block = ConfigureCometBftBlock {
        height,
        ..Default::default()
    }
    .make();
    let block = SequencerBlock::try_from_cometbft(block).unwrap();

    ReconstructedBlock {
        block_hash: block.block_hash(),
        header: block.header().clone(),
        transactions: vec![],
        celestia_height: 1,
    }
}

struct MockEnvironment {
    _server: MockExecutionServer,
    _shutdown: CancellationToken,
    executor: Executor,
    client: Client,
}

async fn start_mock() -> MockEnvironment {
    let server = MockExecutionServer::spawn().await;
    let server_url = format!("http://{}", server.local_addr());

    let shutdown_token = CancellationToken::new();
    let (executor, _) = crate::executor::Builder {
        consider_commitment_spread: false,
        rollup_address: server_url,
        shutdown: shutdown_token.clone(),
    }
    .build()
    .unwrap();

    let client = Client::connect(executor.rollup_address.clone())
        .await
        .unwrap();

    executor
        .set_initial_node_state(client.clone())
        .await
        .unwrap();

    MockEnvironment {
        _server: server,
        _shutdown: shutdown_token,
        executor,
        client,
    }
}

fn make_rollup_data(data: &str) -> RawRollupData {
    RollupData::SequencedData(data.as_bytes().to_vec()).into_raw()
}

#[tokio::test]
async fn firm_blocks_at_expected_heights_are_executed() {
    let mut mock = start_mock().await;

    let height = 100;

    let mut block = make_reconstructed_block(height);
    let rollup_data = make_rollup_data("test_transaction");
    block.transactions.push(rollup_data.encode_to_vec());

    let expected_exection_hash = get_expected_execution_hash(
        mock.executor.state.borrow().firm().hash(),
        vec![rollup_data.clone()],
    );

    mock.executor
        .execute_firm(mock.client.clone(), block)
        .await
        .unwrap();
    assert_eq!(
        expected_exection_hash,
        mock.executor.state.borrow().firm().hash(),
    );

    let mut block = make_reconstructed_block(height + 1);
    block.transactions.push(rollup_data.encode_to_vec());
    let expected_exection_hash = get_expected_execution_hash(
        mock.executor.state.borrow().firm().hash(),
        vec![rollup_data],
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
    let mut mock = start_mock().await;

    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(100u32);
    let block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);
    assert!(
        mock.executor
            .execute_soft(mock.client.clone(), block)
            .await
            .is_ok()
    );

    let mut block = make_cometbft_block();
    block.header.height = SequencerHeight::from(101u32);
    let block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);
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
    let mut mock = start_mock().await;

    let block = ConfigureCometBftBlock {
        height: 100,
        rollup_transactions: vec![(ROLLUP_ID, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let soft_block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);

    let block_hash = soft_block.block_hash();

    let firm_block = ReconstructedBlock {
        block_hash: soft_block.block_hash(),
        header: soft_block.header().clone(),
        transactions: soft_block
            .rollup_transactions()
            .get(&ROLLUP_ID)
            .cloned()
            .unwrap()
            .transactions()
            .to_vec(),
        celestia_height: 1,
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
    let mut mock = start_mock().await;

    let block = ConfigureCometBftBlock {
        height: 100,
        rollup_transactions: vec![(ROLLUP_ID, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let soft_block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);

    let block_hash = soft_block.block_hash();

    let firm_block = ReconstructedBlock {
        block_hash: soft_block.block_hash(),
        header: soft_block.header().clone(),
        transactions: soft_block
            .rollup_transactions()
            .get(&ROLLUP_ID)
            .cloned()
            .unwrap()
            .transactions()
            .to_vec(),
        celestia_height: 1,
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
    let mut mock = start_mock().await;
    let block = ConfigureCometBftBlock {
        height: 99,
        rollup_transactions: vec![(ROLLUP_ID, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let sequencer_block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);

    let firm = mock.executor.state.borrow().firm().clone();
    let soft = mock.executor.state.borrow().soft().clone();

    mock.executor
        .execute_soft(mock.client.clone(), sequencer_block)
        .await
        .unwrap();

    assert_eq!(
        &firm,
        mock.executor.state.borrow().firm(),
        "soft blocks must not advance firm commitments"
    );
    assert_eq!(
        &soft,
        mock.executor.state.borrow().soft(),
        "soft commitment is at height 100, so a block at height 99 must not execute",
    );
}

#[tokio::test]
async fn non_sequential_future_soft_blocks_give_error() {
    let mut mock = start_mock().await;

    let block = ConfigureCometBftBlock {
        height: 101,
        rollup_transactions: vec![(ROLLUP_ID, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let sequencer_block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);
    assert!(
        mock.executor
            .execute_soft(mock.client.clone(), sequencer_block)
            .await
            .is_err()
    );

    let block = ConfigureCometBftBlock {
        height: 100,
        rollup_transactions: vec![(ROLLUP_ID, b"hello_world".to_vec())],
        ..Default::default()
    }
    .make();
    let sequencer_block = SequencerBlock::try_from_cometbft(block)
        .unwrap()
        .into_filtered_block([ROLLUP_ID]);
    assert!(
        mock.executor
            .execute_soft(mock.client.clone(), sequencer_block)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn out_of_order_firm_blocks_are_rejected() {
    let mut mock = start_mock().await;
    let block = make_reconstructed_block(99);
    assert!(
        mock.executor
            .execute_firm(mock.client.clone(), block.clone())
            .await
            .is_err()
    );

    let block = make_reconstructed_block(101);
    assert!(
        mock.executor
            .execute_firm(mock.client.clone(), block.clone())
            .await
            .is_err()
    );

    let block = make_reconstructed_block(100);
    assert!(
        mock.executor
            .execute_firm(mock.client.clone(), block.clone())
            .await
            .is_ok()
    );
}

fn make_block(number: u32) -> raw::Block {
    raw::Block {
        number,
        hash: Bytes::from_static(&[0u8; 32]),
        parent_block_hash: Bytes::from_static(&[0u8; 32]),
        timestamp: Some(pbjson_types::Timestamp {
            seconds: 0,
            nanos: 0,
        }),
    }
}

struct MakeState {
    firm: u32,
    soft: u32,
}

fn make_state(
    MakeState {
        firm,
        soft,
    }: MakeState,
) -> super::State {
    let genesis_info = GenesisInfo::try_from_raw(raw::GenesisInfo {
        rollup_id: Bytes::copy_from_slice(ROLLUP_ID.as_ref()),
        sequencer_genesis_block_height: 1,
        celestia_base_block_height: 1,
        celestia_block_variance: 1,
    })
    .unwrap();
    let commitment_state = CommitmentState::try_from_raw(raw::CommitmentState {
        firm: Some(make_block(firm)),
        soft: Some(make_block(soft)),
    })
    .unwrap();
    let mut state = super::State::new();
    state.init(genesis_info, commitment_state);
    state
}

#[track_caller]
fn assert_contract_fulfilled(kind: super::ExecutionKind, state: MakeState, number: u32) {
    let block = Block::try_from_raw(make_block(number)).unwrap();
    let state = make_state(state);
    super::does_block_response_fulfill_contract(kind, &state, &block)
        .expect("number stored in response block must be one more than in tracked state");
}

#[track_caller]
fn assert_contract_violated(kind: super::ExecutionKind, state: MakeState, number: u32) {
    let block = Block::try_from_raw(make_block(number)).unwrap();
    let state = make_state(state);
    super::does_block_response_fulfill_contract(kind, &state, &block)
        .expect_err("number stored in response block must not be one more than in tracked state");
}

#[test]
fn foo() {
    use super::ExecutionKind::{
        Firm,
        Soft,
    };
    assert_contract_fulfilled(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        3,
    );

    assert_contract_fulfilled(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        4,
    );

    assert_contract_violated(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        1,
    );

    assert_contract_violated(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        2,
    );

    assert_contract_violated(
        Firm,
        MakeState {
            firm: 2,
            soft: 3,
        },
        4,
    );

    assert_contract_violated(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        2,
    );

    assert_contract_violated(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        3,
    );

    assert_contract_violated(
        Soft,
        MakeState {
            firm: 2,
            soft: 3,
        },
        5,
    );
}
