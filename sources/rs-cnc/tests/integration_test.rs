use rand::Rng;
use rs_cnc::{Client, NamespacedDataResponse, PayForDataResponse};

#[test]
fn test_data_roundtrip() {
    // let base_url = String::from("http://localhost:26658");
    let base_url = String::from("http://localhost:26659"); // v0.6.1
    let client = Client::new(base_url).unwrap();

    // generate some random bytes for namespace_id
    let random_namespace_id = rand::thread_rng().gen::<[u8; 8]>();

    let mut random_data = Vec::new();
    random_data.extend_from_slice(b"some random data");

    let res: Result<PayForDataResponse, reqwest::Error> = client.submit_pay_for_data(
        &random_namespace_id,
        &random_data,
        2_000,
        90_000);

    assert!(res.is_ok());

    if let Some(height) = res.unwrap().height {
        let res: Result<NamespacedDataResponse, reqwest::Error> = client.namespaced_data(random_namespace_id, height);
        assert!(res.is_ok());

        if let namespaced_data_response = res.unwrap() {
            assert_eq!(namespaced_data_response.height.unwrap(), height);

            // let random_data = hex::encode(random_data);
            // let unwrapped = namespaced_data_response.data.unwrap();
            // println!("{:#?}", unwrapped);
            // assert!(unwrapped.contains(&random_data));
        }
    }

}
