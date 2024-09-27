use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
};

use astria_grpc_mock_test::health::{
    health_server::{
        Health,
        HealthServer,
    },
    HealthCheckRequest,
    HealthCheckResponse,
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
                .add_service(HealthServer::new(HealthService {
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

struct HealthService {
    mock_server: astria_grpc_mock::MockServer,
}

#[tonic::async_trait]
impl Health for HealthService {
    type WatchStream =
        Pin<Box<dyn Stream<Item = Result<HealthCheckResponse, Status>> + Send + 'static>>;

    async fn check(
        self: Arc<Self>,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        self.mock_server.handle_request("check", request).await
    }

    async fn watch(
        self: Arc<Self>,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        unimplemented!()
    }
}
