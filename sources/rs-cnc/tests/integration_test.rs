use rs_cnc::Client;

mod common;

#[test]
fn test_data_roundtrip() {
    common::setup();

    let base_url = String::from("http://localhost:26659");
    let client = Client::new(base_url);

    let namespace_id: u8 = 53;
    let random_data: String = String::from("random data");
    let res = client.submit_pfd(
        namespace_id,
        random_data,
        10000,
        100000);

}
