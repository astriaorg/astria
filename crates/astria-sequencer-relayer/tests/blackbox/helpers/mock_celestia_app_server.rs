use std::{
    net::SocketAddr,
    sync::{
        Arc,
        Mutex,
    },
};

use astria_core::generated::{
    celestia::v1::{
        query_server::{
            Query as BlobQueryService,
            QueryServer as BlobQueryServer,
        },
        Params as BlobParams,
        QueryParamsRequest as QueryBlobParamsRequest,
        QueryParamsResponse as QueryBlobParamsResponse,
    },
    cosmos::{
        auth::v1beta1::{
            query_server::{
                Query as AuthQueryService,
                QueryServer as AuthQueryServer,
            },
            BaseAccount,
            Params as AuthParams,
            QueryAccountRequest,
            QueryAccountResponse,
            QueryParamsRequest as QueryAuthParamsRequest,
            QueryParamsResponse as QueryAuthParamsResponse,
        },
        base::{
            abci::v1beta1::TxResponse,
            node::v1beta1::{
                service_server::{
                    Service as MinGasPriceService,
                    ServiceServer as MinGasPriceServer,
                },
                ConfigRequest as MinGasPriceRequest,
                ConfigResponse as MinGasPriceResponse,
            },
            tendermint::v1beta1::{
                service_server::{
                    Service as NodeInfoService,
                    ServiceServer as NodeInfoServer,
                },
                GetNodeInfoRequest,
                GetNodeInfoResponse,
            },
        },
        tx::v1beta1::{
            service_server::{
                Service as TxService,
                ServiceServer as TxServer,
            },
            BroadcastTxRequest,
            BroadcastTxResponse,
            GetTxRequest,
            GetTxResponse,
        },
    },
    tendermint::{
        p2p::DefaultNodeInfo,
        types::BlobTx,
    },
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use astria_grpc_mock::{
    matcher::message_type,
    response::{
        constant_response,
        dynamic_response,
    },
    Mock,
    MockGuard,
    MockServer,
};
use celestia_types::nmt::Namespace;
use prost::{
    Message,
    Name,
};
use tokio::task::JoinHandle;
use tonic::{
    transport::Server,
    Request,
    Response,
    Status,
};

const CELESTIA_NETWORK_NAME: &str = "test-celestia";
const GET_NODE_INFO_GRPC_NAME: &str = "get_node_info";
const QUERY_ACCOUNT_GRPC_NAME: &str = "query_account";
const QUERY_AUTH_PARAMS_GRPC_NAME: &str = "query_auth_params";
const QUERY_BLOB_PARAMS_GRPC_NAME: &str = "query_blob_params";
const MIN_GAS_PRICE_GRPC_NAME: &str = "min_gas_price";
const GET_TX_GRPC_NAME: &str = "get_tx";
const BROADCAST_TX_GRPC_NAME: &str = "broadcast_tx";

pub struct MockCelestiaAppServer {
    pub _server: JoinHandle<eyre::Result<()>>,
    pub mock_server: MockServer,
    pub local_addr: SocketAddr,
    pub namespaces: Arc<Mutex<Vec<Namespace>>>,
}

impl MockCelestiaAppServer {
    pub async fn spawn() -> Self {
        use tokio_stream::wrappers::TcpListenerStream;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let mock_server = MockServer::new();
        register_get_node_info(&mock_server).await;
        register_query_account(&mock_server).await;
        register_query_auth_params(&mock_server).await;
        register_query_blob_params(&mock_server).await;
        register_min_gas_price(&mock_server).await;

        let server = {
            let service_impl = CelestiaAppServiceImpl(mock_server.clone());
            tokio::spawn(async move {
                Server::builder()
                    .add_service(NodeInfoServer::new(service_impl.clone()))
                    .add_service(AuthQueryServer::new(service_impl.clone()))
                    .add_service(BlobQueryServer::new(service_impl.clone()))
                    .add_service(MinGasPriceServer::new(service_impl.clone()))
                    .add_service(TxServer::new(service_impl))
                    .serve_with_incoming(TcpListenerStream::new(listener))
                    .await
                    .wrap_err("gRPC sequencer server failed")
            })
        };
        Self {
            _server: server,
            mock_server,
            local_addr,
            namespaces: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn mount_broadcast_tx_response(&self, debug_name: impl Into<String>) -> MockGuard {
        let debug_name = debug_name.into();
        let txhash = debug_name.clone();
        let namespaces = self.namespaces.clone();
        let responder = move |request: &BroadcastTxRequest| {
            namespaces
                .lock()
                .unwrap()
                .extend(extract_blob_namespaces(request));
            // We only use the `code` and `txhash` fields in the success case.  The `txhash` would
            // be an actual hex-encoded SHA256 in prod, but here we can just use the
            // debug name for ease of debugging.
            let tx_response = TxResponse {
                txhash: txhash.clone(),
                code: 0,
                ..TxResponse::default()
            };
            BroadcastTxResponse {
                tx_response: Some(tx_response),
            }
        };

        Mock::for_rpc_given(BROADCAST_TX_GRPC_NAME, message_type::<BroadcastTxRequest>())
            .respond_with(dynamic_response(responder))
            .expect(1)
            .up_to_n_times(1)
            .with_name(debug_name)
            .mount_as_scoped(&self.mock_server)
            .await
    }

    pub async fn mount_get_tx_response(
        &self,
        height: i64,
        debug_name: impl Into<String>,
    ) -> MockGuard {
        let debug_name = debug_name.into();
        // We only use the `tx_response.code` and `tx_response.height` fields in the success case.
        // The `txhash` would be an actual hex-encoded SHA256 in prod, but here we can just use the
        // debug name for ease of debugging.
        let tx_response = TxResponse {
            height,
            txhash: debug_name.clone(),
            code: 0,
            ..TxResponse::default()
        };
        let response = GetTxResponse {
            tx: None,
            tx_response: Some(tx_response),
        };
        Mock::for_rpc_given(GET_TX_GRPC_NAME, message_type::<GetTxRequest>())
            .respond_with(constant_response(response))
            .up_to_n_times(1)
            .expect(1)
            .with_name(debug_name)
            .mount_as_scoped(&self.mock_server)
            .await
    }
}

/// Registers a handler for all incoming `GetNodeInfoRequest`s which responds with the same
/// `GetNodeInfoResponse` every time.
async fn register_get_node_info(mock_server: &MockServer) {
    let default_node_info = Some(DefaultNodeInfo {
        network: CELESTIA_NETWORK_NAME.to_string(),
        ..Default::default()
    });
    let response = GetNodeInfoResponse {
        default_node_info,
        ..Default::default()
    };

    Mock::for_rpc_given(
        GET_NODE_INFO_GRPC_NAME,
        message_type::<GetNodeInfoRequest>(),
    )
    .respond_with(constant_response(response))
    .with_name("global get node info")
    .mount(mock_server)
    .await;
}

/// Registers a handler for all incoming `QueryAccountRequest`s which responds with a
/// `QueryAccountResponse` using the received account address, but otherwise the same data every
/// time.
async fn register_query_account(mock_server: &MockServer) {
    let responder = |request: &QueryAccountRequest| {
        let account = BaseAccount {
            address: request.address.clone(),
            pub_key: None, // this field is ignored by the relayer
            account_number: 10,
            sequence: 53,
        };
        let account_as_any = pbjson_types::Any {
            type_url: BaseAccount::type_url(),
            value: account.encode_to_vec().into(),
        };
        QueryAccountResponse {
            account: Some(account_as_any),
        }
    };
    Mock::for_rpc_given(
        QUERY_ACCOUNT_GRPC_NAME,
        message_type::<QueryAccountRequest>(),
    )
    .respond_with(dynamic_response(responder))
    .with_name("global query account")
    .mount(mock_server)
    .await;
}

/// Registers a handler for all incoming `QueryAuthParamsRequest`s which responds with the same
/// `QueryAuthParamsResponse` every time.
///
/// The response is as per current values in Celestia mainnet.
async fn register_query_auth_params(mock_server: &MockServer) {
    let params = AuthParams {
        max_memo_characters: 256,
        tx_sig_limit: 7,
        tx_size_cost_per_byte: 10,
        sig_verify_cost_ed25519: 590,
        sig_verify_cost_secp256k1: 1000,
    };
    let response = QueryAuthParamsResponse {
        params: Some(params),
    };
    Mock::for_rpc_given(
        QUERY_AUTH_PARAMS_GRPC_NAME,
        message_type::<QueryAuthParamsRequest>(),
    )
    .respond_with(constant_response(response))
    .with_name("global query auth params")
    .mount(mock_server)
    .await;
}

/// Registers a handler for all incoming `QueryBlobParamsRequest`s which responds with the same
/// `QueryBlobParamsResponse` every time.
///
/// The response is as per current values in Celestia mainnet.
async fn register_query_blob_params(mock_server: &MockServer) {
    let response = QueryBlobParamsResponse {
        params: Some(BlobParams {
            gas_per_blob_byte: 8,
            gov_max_square_size: 64,
        }),
    };
    Mock::for_rpc_given(
        QUERY_BLOB_PARAMS_GRPC_NAME,
        message_type::<QueryBlobParamsRequest>(),
    )
    .respond_with(constant_response(response))
    .with_name("global query blob params")
    .mount(mock_server)
    .await;
}

/// Registers a handler for all incoming `MinGasPriceRequest`s which responds with the same
/// `MinGasPriceResponse` every time.
///
/// The response is as per the current value in Celestia mainnet.
async fn register_min_gas_price(mock_server: &MockServer) {
    let response = MinGasPriceResponse {
        minimum_gas_price: "0.002000000000000000utia".to_string(),
    };
    Mock::for_rpc_given(
        MIN_GAS_PRICE_GRPC_NAME,
        message_type::<MinGasPriceRequest>(),
    )
    .respond_with(constant_response(response))
    .with_name("global min gas price")
    .mount(mock_server)
    .await;
}

#[derive(Clone)]
struct CelestiaAppServiceImpl(MockServer);

#[async_trait::async_trait]
impl NodeInfoService for CelestiaAppServiceImpl {
    async fn get_node_info(
        self: Arc<Self>,
        request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<GetNodeInfoResponse>, Status> {
        self.0
            .handle_request(GET_NODE_INFO_GRPC_NAME, request)
            .await
    }
}

#[async_trait::async_trait]
impl AuthQueryService for CelestiaAppServiceImpl {
    async fn account(
        self: Arc<Self>,
        request: Request<QueryAccountRequest>,
    ) -> Result<Response<QueryAccountResponse>, Status> {
        self.0
            .handle_request(QUERY_ACCOUNT_GRPC_NAME, request)
            .await
    }

    async fn params(
        self: Arc<Self>,
        request: Request<QueryAuthParamsRequest>,
    ) -> Result<Response<QueryAuthParamsResponse>, Status> {
        self.0
            .handle_request(QUERY_AUTH_PARAMS_GRPC_NAME, request)
            .await
    }
}

#[async_trait::async_trait]
impl BlobQueryService for CelestiaAppServiceImpl {
    async fn params(
        self: Arc<Self>,
        request: Request<QueryBlobParamsRequest>,
    ) -> Result<Response<QueryBlobParamsResponse>, Status> {
        self.0
            .handle_request(QUERY_BLOB_PARAMS_GRPC_NAME, request)
            .await
    }
}

#[async_trait::async_trait]
impl MinGasPriceService for CelestiaAppServiceImpl {
    async fn config(
        self: Arc<Self>,
        request: Request<MinGasPriceRequest>,
    ) -> Result<Response<MinGasPriceResponse>, Status> {
        self.0
            .handle_request(MIN_GAS_PRICE_GRPC_NAME, request)
            .await
    }
}

#[async_trait::async_trait]
impl TxService for CelestiaAppServiceImpl {
    async fn get_tx(
        self: Arc<Self>,
        request: Request<GetTxRequest>,
    ) -> Result<Response<GetTxResponse>, Status> {
        self.0.handle_request(GET_TX_GRPC_NAME, request).await
    }

    async fn broadcast_tx(
        self: Arc<Self>,
        request: Request<BroadcastTxRequest>,
    ) -> Result<Response<BroadcastTxResponse>, Status> {
        self.0.handle_request(BROADCAST_TX_GRPC_NAME, request).await
    }
}

fn extract_blob_namespaces(request: &BroadcastTxRequest) -> Vec<Namespace> {
    let blob_tx = BlobTx::decode(request.tx_bytes.as_ref()).unwrap();
    blob_tx
        .blobs
        .iter()
        .map(|blob| Namespace::new_v0(blob.namespace_id.as_ref()).unwrap())
        .collect()
}
