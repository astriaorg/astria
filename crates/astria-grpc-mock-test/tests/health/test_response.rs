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
            service: "1".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp_1.get_ref());

    expected_response.status = 2;
    let rsp_2 = client
        .check(HealthCheckRequest {
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
    let mut err_client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mut ok_client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(
            response::default_response::<HealthCheckResponse>()
                .set_delay(Duration::from_millis(250)),
        );
    mock.mount(&server.mocked).await;

    let rsp_fut_expect_err = err_client.check(HealthCheckRequest {
        service: "helloworld".to_string(),
    });
    let rsp_fut_expect_ok = ok_client.check(HealthCheckRequest {
        service: "helloworld".to_string(),
    });

    timeout(Duration::from_millis(200), rsp_fut_expect_err)
        .await
        .unwrap_err(); // should be error
    let ok_rsp = timeout(Duration::from_millis(300), rsp_fut_expect_ok)
        .await
        .unwrap(); // should be ok

    assert!(ok_rsp.is_ok());
    assert_eq!(&HealthCheckResponse::default(), ok_rsp.unwrap().get_ref());
}
