// allow just make the tests work for now
#![allow(clippy::should_panic_without_expect)]

use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
};

use astria_grpc_mock::{
    matcher,
    response,
    Mock,
};
use astria_grpc_mock_test::health::{
    health_client::HealthClient,
    health_server::{
        Health,
        HealthServer,
    },
    HealthCheckRequest,
    HealthCheckResponse,
};
use tokio::{
    join,
    task::JoinHandle,
};
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

struct MockServer {
    _server: JoinHandle<()>,
    local_addr: SocketAddr,
    mocked: astria_grpc_mock::MockServer,
}

async fn start_mock_server() -> MockServer {
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

#[tokio::test]
async fn default_response_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::default_response::<HealthCheckResponse>());
    server.mocked.register(mock).await;
    let rsp = client
        .check(HealthCheckRequest {
            service: "helloworld".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&HealthCheckResponse::default(), rsp.get_ref());
}

#[tokio::test]
async fn constant_response_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::constant_response(expected_response.clone()));
    server.mocked.register(mock).await;
    let rsp = client
        .check(HealthCheckRequest {
            service: "helloworld".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
async fn constant_response_expect_two_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::constant_response(expected_response.clone()))
        .expect(2);

    let guard = server.mocked.register_as_scoped(mock).await;
    let two_checks = async move {
        let res_one = client
            .check(HealthCheckRequest {
                service: "helloworld".to_string(),
            })
            .await?;

        let res_two = client
            .check(HealthCheckRequest {
                service: "helloworld".to_string(),
            })
            .await?;
        Ok::<_, tonic::Status>((res_one, res_two))
    };

    let ((), res) = join!(guard.wait_until_satisfied(), two_checks);
    let res = res.unwrap();
    assert_eq!(&expected_response, res.0.get_ref());
    assert_eq!(&expected_response, res.1.get_ref());
}

#[tokio::test]
async fn constant_response_guard_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::constant_response(expected_response.clone()))
        .expect(1);

    let guard = server.mocked.register_as_scoped(mock).await;
    let check = client.check(HealthCheckRequest {
        service: "helloworld".to_string(),
    });

    let ((), check_res) = join!(guard.wait_until_satisfied(), check);
    let rsp = check_res.unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
async fn exact_pbjson_match_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_request = HealthCheckRequest {
        service: "helloworld".to_string(),
    };
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_exact_pbjson(&expected_request))
        .respond_with(response::constant_response(expected_response.clone()));
    server.mocked.register(mock).await;
    let rsp = client
        .check(HealthCheckRequest {
            service: "helloworld".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
async fn partial_pbjson_match_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_request = HealthCheckRequest {
        service: "helloworld".to_string(),
    };
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    // FIXME: Right now this is equivalent to an exact check because the request only has one field.
    let mock = Mock::for_rpc_given("check", matcher::message_partial_pbjson(&expected_request))
        .respond_with(response::constant_response(expected_response.clone()));
    server.mocked.register(mock).await;
    let rsp = client
        .check(HealthCheckRequest {
            service: "helloworld".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
#[should_panic]
async fn incorrect_mock_response_fails_server() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::default_response::<HealthCheckRequest>());
    server.mocked.register(mock).await;
    let _ = client
        .check(HealthCheckRequest {
            service: "helloworld".to_string(),
        })
        .await;
}

#[tokio::test]
#[should_panic]
async fn incorrect_mock_response_fails_guard() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::default_response::<HealthCheckRequest>());

    let guard = server.mocked.register_as_scoped(mock).await;
    let check = client.check(HealthCheckRequest {
        service: "helloworld".to_string(),
    });

    let _ = join!(guard.wait_until_satisfied(), check);
}
