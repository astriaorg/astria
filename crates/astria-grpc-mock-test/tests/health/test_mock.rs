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
use tokio::join;

use crate::test_utils::start_mock_server;

#[tokio::test]
async fn mock_expect_two_works() {
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
                name: "helloworld".to_string(),
                service: "helloworld".to_string(),
            })
            .await?;

        let res_two = client
            .check(HealthCheckRequest {
                name: "helloworld".to_string(),
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
async fn response_guard_wait_until_satisfied_works() {
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
        name: "helloworld".to_string(),
        service: "helloworld".to_string(),
    });

    let ((), check_res) = join!(guard.wait_until_satisfied(), check);
    let rsp = check_res.unwrap();
    assert_eq!(&expected_response, rsp.get_ref());
}

#[tokio::test]
async fn up_to_n_times_works_as_expected() {
    let server = start_mock_server().await;
    let mut client = HealthClient::connect(format!("http://{}", server.local_addr))
        .await
        .unwrap();
    let expected_response = HealthCheckResponse {
        status: 1,
    };
    let mock = Mock::for_rpc_given("check", matcher::message_type::<HealthCheckRequest>())
        .respond_with(response::constant_response(expected_response.clone()))
        .up_to_n_times(1);

    let guard = server.mocked.register_as_scoped(mock).await;
    let check = client.check(HealthCheckRequest {
        name: "helloworld".to_string(),
        service: "helloworld".to_string(),
    });

    let ((), check_res) = join!(guard.wait_until_satisfied(), check);
    let rsp = check_res.unwrap();
    assert_eq!(&expected_response, rsp.get_ref());

    let err_rsp = client
        .check(HealthCheckRequest {
            name: "helloworld".to_string(),
            service: "helloworld".to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(err_rsp.code(), tonic::Code::NotFound);
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
            name: "helloworld".to_string(),
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
        name: "helloworld".to_string(),
        service: "helloworld".to_string(),
    });

    let _ = join!(guard.wait_until_satisfied(), check);
}
