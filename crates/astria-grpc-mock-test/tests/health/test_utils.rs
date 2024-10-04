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

#[derive(::prost::Message, Clone, PartialEq)]
pub(crate) struct MockMessage {
    #[prost(string, tag = "1")]
    pub(crate) field_one: String,
    #[prost(string, tag = "2")]
    pub(crate) field_two: String,
}

impl ::prost::Name for MockMessage {
    const NAME: &'static str = "MockMessage";
    const PACKAGE: &'static str = "test_utils";

    fn full_name() -> String {
        "test_utils.MockMessage".to_string()
    }
}

impl serde::Serialize for MockMessage {
    #[allow(clippy::arithmetic_side_effects)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.field_one.is_empty() {
            len += 1;
        }
        if !self.field_two.is_empty() {
            len += 1;
        }
        let mut struct_ser =
            serializer.serialize_struct("grpc.health.v1.HealthCheckRequest", len)?;
        if !self.field_one.is_empty() {
            struct_ser.serialize_field("field_one", &self.field_one)?;
        }
        if !self.field_two.is_empty() {
            struct_ser.serialize_field("field_two", &self.field_two)?;
        }
        struct_ser.end()
    }
}
