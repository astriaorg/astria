use prost::Message as _;
use tendermint::{
    abci::{
        request::CheckTxKind,
        Code,
    },
    v0_38::abci::request::CheckTx,
};

use crate::{
    app::{
        test_utils::get_mock_tx,
        App,
    },
    mempool::Mempool,
    metrics::Metrics,
};

#[tokio::test]
async fn transaction_with_future_nonce_enters_mempool() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let mempool = Mempool::new();
    let metrics = Box::leak(Box::new(Metrics::new()));
    let mut app = App::new(snapshot, mempool, metrics).await.unwrap();

    let genesis_state = crate::app::test_utils::genesis_state();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

    let mempool = Mempool::new();
    let metrics = Box::leak(Box::new(Metrics::new()));

    let the_future_nonce = 10;
    let tx = get_mock_tx(the_future_nonce);
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };
    let rsp = super::handle_check_tx(req, storage.clone(), mempool.clone(), metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");
    // with the current mempool implementation we can't directly observe the transaction;
    // but we can check the pending nonce of the signer.
    assert_eq!(
        mempool.pending_nonce(tx.address_bytes()).await.expect(
            "signer should have a pending nonce because the transaction should have made it into \
             the pool"
        ),
        the_future_nonce,
    );
}

#[tokio::test]
async fn valid_transaction_enters_mempool() {
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let mempool = Mempool::new();
    let metrics = Box::leak(Box::new(Metrics::new()));
    let mut app = App::new(snapshot, mempool, metrics).await.unwrap();

    let genesis_state = crate::app::test_utils::genesis_state();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

    let mempool = Mempool::new();
    let metrics = Box::leak(Box::new(Metrics::new()));

    let nonce = 0;
    let tx = get_mock_tx(nonce);
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };
    let rsp = super::handle_check_tx(req, storage.clone(), mempool.clone(), metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");
    // with the current mempool implementation we can't directly observe the transaction;
    // but we can check the pending nonce of the signer.
    assert_eq!(
        mempool.pending_nonce(tx.address_bytes()).await.expect(
            "signer should have a pending nonce because the transaction should have made it into \
             the pool"
        ),
        nonce,
    );
}
