use rs_cnc::Client;

mod common;

#[test]
fn test_data_roundtrip() {
    // FIXME - do we even need this?
    common::setup();

    let base_url = String::from("http://192.167.10.0:26657");
    let client = Client::new(base_url);

    let namespace_id: u8 = 53;
    let random_data: String = String::from("random data");
    let _result: Result<(), reqwest::Error> = client.submit_pfd(
        namespace_id,
        random_data,
        10000,
        100000);


}
