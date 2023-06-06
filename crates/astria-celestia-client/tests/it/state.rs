use astria_celestia_client::Blob;
use jsonrpsee::core::traits::ToRpcParams;

#[tokio::test]
async fn submit_pay_for_blob_works() {
    let client = crate::make_client();
    let blob = Blob {
        namespace_id: *b"shrdsueq",
        data: b"helloworld".to_vec(),
    };
    let array_params = jsonrpsee::rpc_params!("42", 42, &[blob.clone()]);
    println!("{}", array_params.to_rpc_params().unwrap().unwrap());
    client
        .submit_pay_for_blob("42".into(), 42, &[blob])
        .await
        .unwrap();
}
