use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
};

use astria_grpc_mock_test::service::{
    service_server::{
        Service,
        ServiceServer,
    },
    MockRequest,
    MockResponse,
};
use tokio::task::JoinHandle;
use tokio_stream::{
    wrappers::TcpListenerStream,
    Stream,
};
use tonic::{
    transport::Server,
    Request,
    Response,
    Status,
};

pub(crate) struct MockServer {
    _server: JoinHandle<()>,
    pub(crate) local_addr: SocketAddr,
    pub(crate) mocked: astria_grpc_mock::MockServer,
}

pub(crate) async fn start_mock_server() -> MockServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let mock_server = astria_grpc_mock::MockServer::new();
    let server = tokio::spawn({
        let mock_server = mock_server.clone();
        async move {
            let _ = Server::builder()
                .add_service(ServiceServer::new(MockGRPCService {
                    mock_server,
                }))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await;
        }
    });
    MockServer {
        _server: server,
        local_addr,
        mocked: mock_server,
    }
}

struct MockGRPCService {
    mock_server: astria_grpc_mock::MockServer,
}

#[tonic::async_trait]
impl Service for MockGRPCService {
    type WatchStream = Pin<Box<dyn Stream<Item = Result<MockResponse, Status>> + Send + 'static>>;

    async fn check(
        self: Arc<Self>,
        request: Request<MockRequest>,
    ) -> Result<Response<MockResponse>, Status> {
        self.mock_server.handle_request("check", request).await
    }

    async fn watch(
        self: Arc<Self>,
        _request: Request<MockRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        unimplemented!()
    }
}
