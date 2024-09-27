use std::time::Duration;

use astria_grpc_mock::{
    matcher,
    response,
    Mock,
};
use astria_grpc_mock_test::health::{
    health_client::HealthClient,
    HealthCheckRequest,
    HealthCheckResponse,
};
use futures::future::join;
use tokio::time::timeout;

use crate::test_utils::start_mock_server;

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
            name: "helloworld".to_string(),
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
            name: "helloworld".to_string(),
            service: "helloworld".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
async fn dynamic_response_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mut expected_response = HealthCheckResponse {
        status: 1,
    };

    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::dynamic_response(dynamic_responder));
    server.mocked.register(mock).await;
    let rsp_1 = client
        .check(HealthCheckRequest {
            name: "helloworld".to_string(),
            service: "1".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp_1.get_ref());

    expected_response.status = 2;
    let rsp_2 = client
        .check(HealthCheckRequest {
            name: "helloworld".to_string(),
            service: "2".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp_2.get_ref());
}

fn dynamic_responder(request: &HealthCheckRequest) -> HealthCheckResponse {
    let status_return = request
        .service
        .chars()
        .next()
        .unwrap()
        .to_digit(10)
        .unwrap();
    HealthCheckResponse {
        status: i32::try_from(status_return).unwrap(),
    }
}

#[tokio::test]
async fn response_delay_works_as_expected() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(
            response::default_response::<HealthCheckResponse>()
                .set_delay(Duration::from_millis(250)),
        );
    let mock_guard = server.mocked.register_as_scoped(mock).await;
    let rsp_fut = client.check(HealthCheckRequest {
        name: "helloworld".to_string(),
        service: "helloworld".to_string(),
    });

    timeout(
        Duration::from_millis(250),
        join(mock_guard.wait_until_satisfied(), rsp_fut),
    )
    .await
    .unwrap_err();

    let rsp = client
        .check(HealthCheckRequest {
            name: "helloworld".to_string(),
            service: "helloworld".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&HealthCheckResponse::default(), rsp.get_ref());
}
