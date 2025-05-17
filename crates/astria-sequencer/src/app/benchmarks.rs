//! To run the benchmark, from the root of the monorepo, run:
//! ```sh
//! cargo bench --features=benchmark -qp astria-sequencer app
//! ```

use std::time::Duration;

use crate::{
    benchmark_utils::{
        self,
        new_fixture,
        TxTypes,
    },
    proposal::block_size_constraints::BlockSizeConstraints,
    test_utils::{
        dummy_balances,
        dummy_tx_costs,
        Fixture,
    },
};

/// The max time for any benchmark.
const MAX_TIME: Duration = Duration::from_secs(120);
/// The value provided to `BlockSizeConstraints::new` to constrain block sizes.
///
/// Taken from the actual value seen in `prepare_proposal.max_tx_bytes` when handling
/// `prepare_proposal` during stress testing using spamoor.
const COMETBFT_MAX_TX_BYTES: i64 = 22_019_254;

/// Initializes a new `App` instance with the genesis accounts derived from the secret keys of
/// `benchmark_utils::signing_keys()`, and inserts transactions into the app mempool.
fn initialize() -> Fixture {
    let dummy_balances = dummy_balances(0, 0);
    let dummy_tx_costs = dummy_tx_costs(0, 0, 0);
    let txs = benchmark_utils::transactions(TxTypes::AllTransfers);
    let fixture = new_fixture();
    let mempool = fixture.mempool();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        for tx in txs {
            mempool
                .insert(tx.clone(), 0, &dummy_balances, dummy_tx_costs.clone())
                .await
                .unwrap();
        }
    });

    fixture
}

#[divan::bench(max_time = MAX_TIME)]
fn prepare_proposal_tx_execution(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    let mut fixture = initialize();
    bencher
        .with_inputs(|| BlockSizeConstraints::new(COMETBFT_MAX_TX_BYTES, true).unwrap())
        .bench_local_refs(|constraints| {
            let executed_txs = runtime.block_on(async {
                fixture
                    .app
                    .prepare_proposal_tx_execution(*constraints)
                    .await
                    .unwrap()
            });
            // Ensure we actually processed some txs.  This will trip if execution fails for all
            // txs, or more likely, if the mempool becomes exhausted of txs.
            assert!(!executed_txs.is_empty());
        });
}
