use astria_celestia_client::CelestiaHttpClient;

mod state;

const TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                     eyJBbGxvdyI6WyJwdWJsaWMiLCJyZWFkIiwid3JpdGUiLCJhZG1pbiJdfQ.\
                     ZmKuJYDtkAesWhgKxQP6jn2DSf9kScg84rBbhEsfrTE";
pub fn make_client() -> CelestiaHttpClient {
    CelestiaHttpClient::builder()
        .bearer_token(TOKEN)
        .endpoint("http://astria-celestia-client-test.localdev.me")
        .build()
        .unwrap()
}
