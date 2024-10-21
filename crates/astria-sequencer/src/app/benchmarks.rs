//! To run the benchmark, from the root of the monorepo, run:
//! ```sh
//! cargo bench --features=benchmark -qp astria-sequencer app
//! ```

use std::time::Duration;

use astria_core::{
    primitive::v1::Address,
    protocol::genesis::v1::GenesisAppState,
    Protobuf,
};
use cnidarium::{
    StateDelta,
    Storage,
};
use telemetry::Metrics as _;

use crate::{
    accounts::StateWriteExt,
    app::{
        test_utils::{
            mock_balances,
            mock_tx_cost,
        },
        App,
    },
    assets::StateWriteExt as AssetStateWriteExt,
    benchmark_utils::{
        self,
        TxTypes,
        SIGNER_COUNT,
    },
    fees::StateWriteExt as _,
    mempool::Mempool,
    metrics::Metrics,
    proposal::block_size_constraints::BlockSizeConstraints,
    test_utils::{
        astria_address,
        nria,
    },
};

/// The max time for any benchmark.
const MAX_TIME: Duration = Duration::from_secs(120);
/// The value provided to `BlockSizeConstraints::new` to constrain block sizes.
///
/// Taken from the actual value seen in `prepare_proposal.max_tx_bytes` when handling
/// `prepare_proposal` during stress testing using spamoor.
const COMETBFT_MAX_TX_BYTES: usize = 22_019_254;

struct Fixture {
    app: App,
    _storage: Storage,
}

async fn initialize_app_with_storage(accounts: Vec<(Address, u128)>) -> (App, Storage) {
    let storage = cnidarium::TempStorage::new()
        .await
        .expect("failed to create temp storage backing chain state");
    // XXX: We need to provide accounts, fee assets, asset inside the storage before
    // initialization because this happens outside of setting genesis.
    //
    // This is not ideal and should be fixed by a proper intialization flow that transfers
    // in funds after genesis.
    {
        let mut state = StateDelta::new(storage.latest_snapshot());
        state.put_asset(nria()).unwrap();
        state.put_allowed_fee_asset(&nria()).unwrap();

        for (address, balance) in &accounts {
            state
                .put_account_balance(address, &nria(), *balance)
                .unwrap();
        }
        storage.commit(state).await.unwrap();
    }
    let snapshot = storage.latest_snapshot();
    let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
    let mempool = Mempool::new(metrics, 100);
    let mut app = App::new(snapshot, mempool, metrics).await.unwrap();

    let genesis_state = GenesisAppState::try_from_raw(
        astria_core::generated::protocol::genesis::v1::GenesisAppState {
            authority_sudo_address: Some(accounts.first().cloned().unwrap().0.into_raw()),
            ibc_sudo_address: Some(accounts.first().cloned().unwrap().0.into_raw()),
            ..crate::app::test_utils::proto_genesis_state()
        },
    )
    .unwrap();

    app.init_chain(storage.clone(), genesis_state, vec![], "test".to_string())
        .await
        .unwrap();

    app.commit(storage.clone()).await;

    (app, storage.clone())
}

impl Fixture {
    /// Initializes a new `App` instance with the genesis accounts derived from the secret keys of
    /// `benchmark_utils::signing_keys()`, and inserts transactions into the app mempool.
    async fn new() -> Fixture {
        let accounts = benchmark_utils::signing_keys()
            .enumerate()
            .take(usize::from(SIGNER_COUNT))
            .map(|(index, signing_key)| {
                let address = astria_address(&signing_key.address_bytes());
                let balance = 10u128
                    .pow(19)
                    .saturating_add(u128::try_from(index).unwrap());
                (address, balance)
            })
            .collect::<Vec<_>>();

        let (app, storage) = initialize_app_with_storage(accounts).await;

        let mock_balances = mock_balances(0, 0);
        let mock_tx_cost = mock_tx_cost(0, 0, 0);

        for tx in benchmark_utils::transactions(TxTypes::AllTransfers) {
            app.mempool
                .insert(tx.clone(), 0, mock_balances.clone(), mock_tx_cost.clone())
                .await
                .unwrap();
        }
        Fixture {
            app,
            _storage: storage,
        }
    }
}

#[divan::bench(max_time = MAX_TIME)]
fn execute_transactions_prepare_proposal(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut fixture = runtime.block_on(async { Fixture::new().await });
    bencher
        .with_inputs(|| BlockSizeConstraints::new(COMETBFT_MAX_TX_BYTES).unwrap())
        .bench_local_refs(|constraints| {
            let (_tx_bytes, included_txs) = runtime.block_on(async {
                fixture
                    .app
                    .execute_transactions_prepare_proposal(constraints)
                    .await
                    .unwrap()
            });
            // Ensure we actually processed some txs.  This will trip if execution fails for all
            // txs, or more likely, if the mempool becomes exhausted of txs.
            assert!(!included_txs.is_empty());
        });
}
