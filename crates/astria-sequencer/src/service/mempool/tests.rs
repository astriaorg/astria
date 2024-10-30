use std::num::NonZeroU32;

use astria_core::{
    protocol::transaction::v1::{
        action::Transfer,
        TransactionBody,
    },
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
        test_utils::{
            get_alice_signing_key,
            MockTxBuilder,
            ALICE_ADDRESS,
            BOB_ADDRESS,
        },
        benchmark_and_test_utils::genesis_state,
        test_utils::MockTxBuilder,
        App,
    },
    mempool::{
        Mempool,
        RemovalReason,
    },
    test_utils::astria_address_from_hex_string,
};

#[tokio::test]
async fn future_nonces_are_accepted() {
    // The mempool should allow future nonces.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let mut app = App::new(snapshot, mempool.clone(), metrics).await.unwrap();
    let genesis_state = genesis_state();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

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
async fn too_expensive_txs_are_replaceable() {
    // The mempool should allow replacement of transactions that an account does
    // not have enough balance to afford.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let mut app = App::new(snapshot, mempool.clone(), metrics).await.unwrap();
    let chain_id = "test".to_string();
    let genesis_state = crate::app::test_utils::genesis_state();

    // get balance higher than alice's
    let alice_balance = genesis_state
        .accounts()
        .iter()
        .find(|a| a.address == astria_address_from_hex_string(ALICE_ADDRESS))
        .unwrap()
        .balance;
    let too_expensive_amount = alice_balance + 10;

    app.init_chain(storage.clone(), genesis_state, vec![], chain_id.clone())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

    let tx_too_expensive = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: astria_address_from_hex_string(BOB_ADDRESS),
                amount: too_expensive_amount,
                asset: crate::test_utils::nria().into(),
                fee_asset: crate::test_utils::nria().into(),
            }
            .into(),
        ])
        .nonce(0)
        .chain_id(chain_id.clone())
        .try_build()
        .unwrap()
        .sign(&get_alice_signing_key());

    let tx_ok = TransactionBody::builder()
        .actions(vec![
            Transfer {
                to: astria_address_from_hex_string(BOB_ADDRESS),
                amount: 1,
                asset: crate::test_utils::nria().into(),
                fee_asset: crate::test_utils::nria().into(),
            }
            .into(),
        ])
        .nonce(0)
        .chain_id(chain_id.clone())
        .try_build()
        .unwrap()
        .sign(&get_alice_signing_key());
    let req_too_expensive = CheckTx {
        tx: tx_too_expensive.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };
    let req_ok = CheckTx {
        tx: tx_ok.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::New,
    };

    // too expensive should enter the mempool
    let rsp_too_expensive = super::handle_check_tx(
        req_too_expensive,
        storage.latest_snapshot(),
        &mut mempool,
        metrics,
    )
    .await;
    assert_eq!(rsp_too_expensive.code, Code::Ok, "{rsp_too_expensive:#?}");

    // ok should enter the mempool
    let rsp_ok =
        super::handle_check_tx(req_ok, storage.latest_snapshot(), &mut mempool, metrics).await;
    assert_eq!(rsp_ok.code, Code::Ok, "{rsp_ok:#?}");

    // recheck on too expensive should be nonce replacement
    let req_too_expensive = CheckTx {
        tx: tx_too_expensive.to_raw().encode_to_vec().into(),
        kind: CheckTxKind::Recheck,
    };
    let rsp_too_expensive_recheck = super::handle_check_tx(
        req_too_expensive,
        storage.latest_snapshot(),
        &mut mempool,
        metrics,
    )
    .await;
    assert_eq!(
        rsp_too_expensive_recheck.code,
        Code::Err(NonZeroU32::new(18).unwrap()),
        "{rsp_too_expensive_recheck:#?}"
    );

    // mempool should contain single transaction
    assert_eq!(mempool.len().await, 1);
}

#[tokio::test]
async fn rechecks_pass() {
    // The mempool should not fail rechecks of transactions.
    let storage = cnidarium::TempStorage::new().await.unwrap();
    let snapshot = storage.latest_snapshot();

    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mut mempool = Mempool::new(metrics, 100);
    let mut app = App::new(snapshot, mempool.clone(), metrics).await.unwrap();
    let genesis_state = genesis_state();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

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
    let mut app = App::new(snapshot, mempool.clone(), metrics).await.unwrap();
    let genesis_state = genesis_state();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

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
    let mut app = App::new(snapshot, mempool.clone(), metrics).await.unwrap();
    let genesis_state = genesis_state();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();
    app.commit(storage.clone()).await;

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
