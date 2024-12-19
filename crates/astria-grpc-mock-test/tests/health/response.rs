use std::time::Duration;

use astria_grpc_mock::{
    matcher,
    response,
    Mock,
};
use astria_grpc_mock_test::service::{
    service_client::ServiceClient,
    MockRequest,
    MockResponse,
};
use tokio::time::timeout;

use crate::utils::start_mock_server;

#[tokio::test]
async fn default_response_works() {
    let server = start_mock_server().await;
    let mut client = ServiceClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<MockRequest>())
        .respond_with(response::default_response::<MockResponse>());
    server.mocked.register(mock).await;
    let rsp = client
        .check(MockRequest {
            service: "helloworld".to_string(),
            additional_info: String::new(),
        })
        .await
        .unwrap();
    assert_eq!(&MockResponse::default(), rsp.get_ref());
}

#[tokio::test]
async fn constant_response_works() {
    let server = start_mock_server().await;
    let mut client = ServiceClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_response = MockResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_type::<MockRequest>())
        .respond_with(response::constant_response(expected_response.clone()));
    server.mocked.register(mock).await;
    let rsp = client
        .check(MockRequest {
            service: "helloworld".to_string(),
            additional_info: String::new(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
async fn dynamic_response_works() {
    let server = start_mock_server().await;
    let mut client = ServiceClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mut expected_response = MockResponse {
        status: 1,
    };

    let mock = Mock::for_rpc_given("check", matcher::message_type::<MockRequest>())
        .respond_with(response::dynamic_response(dynamic_responder));
    server.mocked.register(mock).await;
    let rsp_1 = client
        .check(MockRequest {
            service: "1".to_string(),
            additional_info: String::new(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp_1.get_ref());

    expected_response.status = 2;
    let rsp_2 = client
        .check(MockRequest {
            service: "2".to_string(),
            additional_info: String::new(),
        })
        .await
        .unwrap();
    assert_eq!(&expected_response, rsp_2.get_ref());
}

fn dynamic_responder(request: &MockRequest) -> MockResponse {
    let status_return = request
        .service
        .chars()
        .next()
        .unwrap()
        .to_digit(10)
        .unwrap();
    MockResponse {
        status: i32::try_from(status_return).unwrap(),
    }
}

#[tokio::test]
async fn response_delay_works_as_expected() {
    const DELAY: Duration = Duration::from_millis(250);
    const FIFTY_MILLIS: Duration = Duration::from_millis(50);

    let server = start_mock_server().await;
    let mut err_client = ServiceClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mut ok_client = ServiceClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let mock = Mock::for_rpc_given("check", matcher::message_type::<MockRequest>())
        .respond_with(response::default_response::<MockResponse>().set_delay(DELAY));
    mock.mount(&server.mocked).await;

    let rsp_fut_expect_err = err_client.check(MockRequest {
        service: "helloworld".to_string(),
        additional_info: String::new(),
    });
    let rsp_fut_expect_ok = ok_client.check(MockRequest {
        service: "helloworld".to_string(),
        additional_info: String::new(),
    });

    timeout(DELAY - FIFTY_MILLIS, rsp_fut_expect_err)
        .await
        .unwrap_err();
    let ok_rsp = timeout(DELAY + FIFTY_MILLIS, rsp_fut_expect_ok)
        .await
        .unwrap();

    assert!(ok_rsp.is_ok());
    assert_eq!(&MockResponse::default(), ok_rsp.unwrap().get_ref());
}
