use bytes::Bytes;
use rs_cnc::CelestiaNodeClient;

#[tokio::test]
async fn test_data_roundtrip() {
    let base_url = "http://localhost:26659";
    let client = CelestiaNodeClient::builder()
        .base_url(base_url)
        .unwrap()
        .build()
        .unwrap();

    let random_namespace_id = String::from("b860ccf0e97fdf6c");

    // create arbitrary vector of bytes
    let data = Bytes::from(&b"some random data"[..]);

    let res = client
        .submit_pay_for_data(&random_namespace_id, &data, 2_000, 90_000)
        .await
        .unwrap();
    assert!(!res.height.is_none());

    // use height from previous response to call namespaced shares/data endpoints
    let height = res.height.unwrap();
    let namespaced_shares_resp = client
        .namespaced_shares(&random_namespace_id, height)
        .await
        .unwrap();
    assert_eq!(height, namespaced_shares_resp.height);

    let namespaced_data_response = client
        .namespaced_data(&random_namespace_id, height)
        .await
        .unwrap();
    let res_data = namespaced_data_response.data.unwrap();
    assert_eq!(res_data.len(), 1);
    assert_eq!(res_data[0].0, data);
    assert_eq!(namespaced_data_response.height.unwrap(), height);
}
