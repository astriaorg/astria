//! To run the benchmark, from the root of the monorepo, run:
//! ```sh
//! cargo bench --features=benchmark -qp astria-sequencer app
//! ```

use std::time::Duration;

use astria_core::{
    protocol::genesis::v1::{
        Account,
        GenesisAppState,
    },
    Protobuf,
};

use crate::{
    app::{
        benchmark_and_test_utils::{
            mock_balances,
            mock_tx_cost,
        },
        App,
    },
    benchmark_and_test_utils::astria_address,
    benchmark_utils::{
        self,
        TxTypes,
        SIGNER_COUNT,
    },
    proposal::block_size_constraints::BlockSizeConstraints,
    storage::Storage,
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

impl Fixture {
    /// Initializes a new `App` instance with the genesis accounts derived from the secret keys of
    /// `benchmark_utils::signing_keys()`, and inserts transactions into the app mempool.
    async fn new() -> Fixture {
        let accounts = benchmark_utils::signing_keys()
            .enumerate()
            .take(usize::from(SIGNER_COUNT))
            .map(|(index, signing_key)| Account {
                address: astria_address(&signing_key.address_bytes()),
                balance: 10u128
                    .pow(19)
                    .saturating_add(u128::try_from(index).unwrap()),
            })
            .map(Protobuf::into_raw)
            .collect::<Vec<_>>();
        let first_address = accounts.first().cloned().unwrap().address;
        let genesis_state = GenesisAppState::try_from_raw(
            astria_core::generated::protocol::genesis::v1::GenesisAppState {
                accounts,
                authority_sudo_address: first_address.clone(),
                ibc_sudo_address: first_address.clone(),
                ..crate::app::benchmark_and_test_utils::proto_genesis_state()
            },
        )
        .unwrap();

        let (app, storage) = crate::app::benchmark_and_test_utils::initialize_app_with_storage(
            Some(genesis_state),
            vec![],
        )
        .await;

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
