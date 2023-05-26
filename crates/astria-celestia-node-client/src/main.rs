use astria_celestia_node_client::{
    HeaderClient,
    StateClient,
};
use jsonrpsee::{
    http_client::HttpClientBuilder,
    ws_client::{
        HeaderMap,
        HeaderValue,
    },
};

#[tokio::main]
async fn main() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_static(
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
             eyJBbGxvdyI6WyJwdWJsaWMiLCJyZWFkIiwid3JpdGUiXX0.\
             gdHm9UFlPcUS-aP13klGWqieX89q99MrisRgZzCGGRA",
        ),
    );
    let client = HttpClientBuilder::default()
        .build("http://test.localdev.me:80/bridgerpc/")
        .unwrap();

    println!("{:?}", client.get_by_height(1).await.unwrap());

    println!(
        "{:?}",
        client
            .submit_pay_for_blob(
                "BW==".into(),
                "Ynl0ZSBhcnJHeQ==".into(),
                "2000".into(),
                80000,
            )
            .await
            .unwrap()
    );
}
