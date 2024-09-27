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

use crate::test_utils::start_mock_server;

#[tokio::test]
async fn exact_pbjson_match_works() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_request = HealthCheckRequest {
        name: "helloworld".to_string(),
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
            name: "helloworld".to_string(),
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
        name: "helloworld".to_string(),
        service: String::new(),
    };
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_partial_pbjson(&expected_request))
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
async fn and_combinator_works_with_partial_pbjson() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_request_1 = HealthCheckRequest {
        name: "helloworld".to_string(),
        service: String::new(),
    };
    let expected_request_2 = HealthCheckRequest {
        name: String::new(),
        service: "helloworld".to_string(),
    };
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given(
        "check",
        matcher::message_partial_pbjson(&expected_request_1),
    )
    .and(matcher::message_partial_pbjson(&expected_request_2))
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
async fn exact_pbjson_matcher_doesnt_match_incorrect_request() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_request = HealthCheckRequest {
        name: "helloworld".to_string(),
        service: "helloworld".to_string(),
    };
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_exact_pbjson(&expected_request))
        .respond_with(response::constant_response(expected_response.clone()));
    server.mocked.register(mock).await;
    let err_rsp = client
        .check(HealthCheckRequest {
            name: "helloworld_wrong".to_string(),
            service: "helloworld_wrong".to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(err_rsp.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn partial_pbjson_match_doesnt_match_incorrect_request() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_request = HealthCheckRequest {
        name: "helloworld".to_string(),
        service: String::new(),
    };
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_partial_pbjson(&expected_request))
        .respond_with(response::constant_response(expected_response.clone()));
    server.mocked.register(mock).await;
    let err_rsp = client
        .check(HealthCheckRequest {
            name: "helloworld_wrong".to_string(),
            service: "helloworld".to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(err_rsp.code(), tonic::Code::NotFound);
}
