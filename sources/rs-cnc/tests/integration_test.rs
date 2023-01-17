use rs_cnc::Client;

#[test]
fn test_data_roundtrip() {
    let base_url = String::from("http://localhost:26658");
    let client = Client::new(base_url).unwrap();

    // let namespace_id: u8 = 53;
    let namespace_id: String = String::from("random data");
    let random_data: String = String::from("random data");
    let res: Result<(), reqwest::Error> = client.submit_pfd(
        namespace_id,
        random_data,
        2_000,
        60_000);

    if res.is_err() {

    }

}
