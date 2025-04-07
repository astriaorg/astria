use std::num::NonZeroU32;

use astria_core::{
    upgrades::v1::Upgrades,
    Protobuf as _,
};
use prost::Message as _;
use telemetry::Metrics;
use tendermint::{
    abci::{
        request::CheckTxKind,
        Code,
    },
    v0_38::abci::request::CheckTx,
};

use crate::{
    app::{
        benchmark_and_test_utils::get_test_genesis_state,
        test_utils::MockTxBuilder,
        App,
    },
    mempool::{
        Mempool,
        RemovalReason,
    },
};

#[tokio::test]
async fn future_nonces_are_accepted() {
    // The mempool should allow future nonces.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let ve_handler = crate::app::vote_extension::Handler::new(None);
    let mut app = App::new(
        snapshot,
        mempool.clone(),
        Upgrades::default().into(),
        ve_handler,
        metrics,
    )
    .await
    .unwrap();

    app.init_chain(
        storage.clone(),
        get_test_genesis_state(),
        vec![],
        "test".to_string(),
    )
    .await
    .unwrap();
    app.commit(storage.clone()).await.unwrap();

    let the_future_nonce = 10;
    let tx = MockTxBuilder::new().nonce(the_future_nonce).build();
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };

    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");

    // mempool should contain single transaction still
    assert_eq!(mempool.len().await, 1);
}

#[tokio::test]
async fn rechecks_pass() {
    // The mempool should not fail rechecks of transactions.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let ve_handler = crate::app::vote_extension::Handler::new(None);
    let mut app = App::new(
        snapshot,
        mempool.clone(),
        Upgrades::default().into(),
        ve_handler,
        metrics,
    )
    .await
    .unwrap();

    app.init_chain(
        storage.clone(),
        get_test_genesis_state(),
        vec![],
        "test".to_string(),
    )
    .await
    .unwrap();
    app.commit(storage.clone()).await.unwrap();

    let tx = MockTxBuilder::new().nonce(0).build();
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };
    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");

    // recheck also passes
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::Recheck,
    };
    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");

    // mempool should contain single transaction still
    assert_eq!(mempool.len().await, 1);
}

#[tokio::test]
async fn can_reinsert_after_recheck_fail() {
    // The mempool should be able to re-insert a transaction after a recheck fails due to the
    // transaction being removed from the appside mempool. This is to allow users to re-insert
    // if they wish to do so.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let ve_handler = crate::app::vote_extension::Handler::new(None);
    let mut app = App::new(
        snapshot,
        mempool.clone(),
        Upgrades::default().into(),
        ve_handler,
        metrics,
    )
    .await
    .unwrap();

    app.init_chain(
        storage.clone(),
        get_test_genesis_state(),
        vec![],
        "test".to_string(),
    )
    .await
    .unwrap();
    app.commit(storage.clone()).await.unwrap();

    let tx = MockTxBuilder::new().nonce(0).build();
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };
    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");

    // remove the transaction from the mempool to make recheck fail
    mempool
        .remove_tx_invalid(tx.clone(), RemovalReason::Expired)
        .await;

    // see recheck fails
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::Recheck,
    };
    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Err(NonZeroU32::new(9).unwrap()), "{rsp:#?}");

    // can re-insert the transaction after first recheck fail
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };
    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");
}

#[tokio::test]
async fn recheck_adds_non_tracked_tx() {
    // The mempool should be able to insert a transaction on recheck if it isn't in the mempool.
    // This could happen in the case of a sequencer restart as the cometbft mempool persists but
    // the appside one does not.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let ve_handler = crate::app::vote_extension::Handler::new(None);
    let mut app = App::new(
        snapshot,
        mempool.clone(),
        Upgrades::default().into(),
        ve_handler,
        metrics,
    )
    .await
    .unwrap();

    app.init_chain(
        storage.clone(),
        get_test_genesis_state(),
        vec![],
        "test".to_string(),
    )
    .await
    .unwrap();
    app.commit(storage.clone()).await.unwrap();

    let tx = MockTxBuilder::new().nonce(0).build();
    let req = CheckTx {
        tx: tx.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::Recheck,
    };

    // recheck should pass and add transaction to mempool
    let rsp = super::handle_check_tx(req, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp.code, Code::Ok, "{rsp:#?}");

    // mempool should contain single transaction still
    assert_eq!(mempool.len().await, 1);
}
