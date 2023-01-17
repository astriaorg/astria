use rand::Rng;
use rs_cnc::{Client, SubmitPFDResponse};

#[test]
fn test_data_roundtrip() {
    // let base_url = String::from("http://localhost:26658");
    let base_url = String::from("http://localhost:26659"); // v0.6.1
    let client = Client::new(base_url).unwrap();

    // generate some random bytes for namespace_id
    let random_namespace_id = rand::thread_rng().gen::<[u8; 8]>();

    let mut random_data = Vec::new();
    random_data.extend_from_slice(b"some random data");

    let res: Result<SubmitPFDResponse, reqwest::Error> = client.submit_pfd(
        random_namespace_id,
        random_data,
        2_000,
        90_000);

    assert_eq!(res.is_ok(), true);
}
