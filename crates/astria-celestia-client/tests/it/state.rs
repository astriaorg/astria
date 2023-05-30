#[tokio::test]
async fn submit_pay_for_blob_works() {
    let client = crate::make_client();
    let namespace = b"shredseq";
    let blob = b"helloworld";
    client
        .submit_pay_for_blob(*namespace, blob, "42".into(), 42)
        .await
        .unwrap();
}
