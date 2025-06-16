//! To run the benchmark, from the root of the monorepo, run:
//! ```sh
//! cargo bench --features=benchmark -qp astria-sequencer mempool
//! ```
#![expect(non_camel_case_types, reason = "for benchmark")]

use std::{
    collections::HashSet,
    sync::{
        Arc,
        OnceLock,
    },
    time::Duration,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::TransactionId,
};
use sha2::{
    Digest as _,
    Sha256,
};
use telemetry::Metrics;

use crate::{
    accounts::StateWriteExt as _,
    benchmark_utils::{
        new_fixture,
        SIGNER_COUNT,
    },
    checked_transaction::CheckedTransaction,
    mempool::{
        Mempool,
        RemovalReason,
    },
    test_utils::{
        dummy_balances,
        dummy_tx_costs,
    },
};

/// The max time for any benchmark.
const MAX_TIME: Duration = Duration::from_secs(30);

/// This trait exists so we can get better output from `divan` by configuring the various mempool
/// sizes as types rather than consts. With types we get output like:
/// ```text
/// ╰─ insert_new_tx
///    ├─ mempool_with_100_txs
///  ...
///    ╰─ mempool_with_100000_txs
/// ```
/// rather than:
/// ```text
/// ╰─ insert_new_tx
///    ├─ 100
///  ...
///    ╰─ 100000
/// ```
trait MempoolSize {
    fn size() -> usize;

    fn checked_size() -> usize {
        assert!(Self::size() <= transactions().len());
        Self::size()
    }
}

struct mempool_with_100_txs;

struct mempool_with_1000_txs;

struct mempool_with_10000_txs;

struct mempool_with_100000_txs;

impl MempoolSize for mempool_with_100_txs {
    fn size() -> usize {
        100
    }
}

impl MempoolSize for mempool_with_1000_txs {
    fn size() -> usize {
        1_000
    }
}

impl MempoolSize for mempool_with_10000_txs {
    fn size() -> usize {
        10_000
    }
}

impl MempoolSize for mempool_with_100000_txs {
    fn size() -> usize {
        100_000
    }
}

fn transactions() -> &'static Vec<Arc<CheckedTransaction>> {
    crate::benchmark_utils::transactions(crate::benchmark_utils::TxTypes::AllRollupDataSubmissions)
}

/// Returns a new `Mempool` initialized with the number of transactions specified by `T::size()`
/// taken from the static `transactions()`, and with a full `comet_bft_removal_cache`.
fn init_mempool<T: MempoolSize>() -> Mempool {
    static CELL_100: OnceLock<Mempool> = OnceLock::new();
    static CELL_1_000: OnceLock<Mempool> = OnceLock::new();
    static CELL_10_000: OnceLock<Mempool> = OnceLock::new();
    static CELL_100_000: OnceLock<Mempool> = OnceLock::new();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let init = || {
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, T::size());
        let account_balances = dummy_balances(0, 0);
        let tx_costs = dummy_tx_costs(0, 0, 0);
        runtime.block_on(async {
            for tx in transactions().iter().take(T::checked_size()) {
                mempool
                    .insert(tx.clone(), 0, &account_balances.clone(), tx_costs.clone())
                    .await
                    .unwrap();
            }
            for i in 0..super::REMOVAL_CACHE_SIZE {
                let tx_id = TransactionId::new(Sha256::digest(i.to_le_bytes()).into());
                mempool
                    .inner
                    .write()
                    .await
                    .comet_bft_removal_cache
                    .add(tx_id, RemovalReason::Expired);
            }
        });
        mempool
    };

    let mempool = match T::checked_size() {
        100 => CELL_100.get_or_init(init),
        1_000 => CELL_1_000.get_or_init(init),
        10_000 => CELL_10_000.get_or_init(init),
        100_000 => CELL_100_000.get_or_init(init),
        _ => unreachable!(),
    };
    runtime.block_on(async { mempool.deep_clone().await })
}

/// Returns the first transaction from the static `transactions()` not included in the initialized
/// mempool, i.e. the one at index `T::size()`.
fn get_unused_tx<T: MempoolSize>() -> Arc<CheckedTransaction> {
    transactions().get(T::checked_size()).unwrap().clone()
}

/// This is not really a benchmark test, rather a means to memoize the data used in the "real"
/// benchmark tests below.
///
/// It should always be named so that it is alphabetically first in the suite of tests, since the
/// tests are run in that order.
///
/// This means that all the real tests are able to have a meaningful number of iterations, making
/// the results more accurate, rather than one test only having a single iteration due to the time
/// taken to memoize the data.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn a_warmup<T: MempoolSize>(bencher: divan::Bencher) {
    init_mempool::<T>();
    bencher.bench(|| ());
}

/// Benchmarks `Mempool::insert` for a single new transaction on a mempool with the given number of
/// existing entries.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn insert<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let balances = dummy_balances(0, 0);
    let tx_costs = dummy_tx_costs(0, 0, 0);
    bencher
        .with_inputs(|| {
            (
                init_mempool::<T>(),
                get_unused_tx::<T>(),
                balances.clone(),
                tx_costs.clone(),
            )
        })
        .bench_values(move |(mempool, tx, mock_balances, mock_tx_cost)| {
            runtime.block_on(async {
                mempool
                    .insert(tx, 0, &mock_balances, mock_tx_cost)
                    .await
                    .unwrap();
            });
        });
}

/// Benchmarks `Mempool::builder_queue` on a mempool with the given number of existing entries.
///
/// Note: this benchmark doesn't capture the nuances of dealing with parked vs pending transactions.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn builder_queue<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool.builder_queue().await;
            });
        });
}

/// Benchmarks `Mempool::remove_tx_invalid` for a single transaction on a mempool with the given
/// number of existing entries.
///
/// Note about this benchmark: `remove_tx_invalid()` will remove all higher nonces. To keep this
/// benchmark comparable with the previous mempool, we're removing the highest nonce. In the future
/// it would be better to have this bench remove the midpoint.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn remove_tx_invalid<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| {
            let signed_tx = transactions()
                .get(T::checked_size().saturating_sub(1))
                .cloned()
                .unwrap();
            (init_mempool::<T>(), signed_tx)
        })
        .bench_values(move |(mempool, signed_tx)| {
            runtime.block_on(async {
                mempool
                    .remove_tx_invalid(signed_tx, RemovalReason::Expired)
                    .await;
            });
        });
}

/// Benchmarks `Mempool::check_removed_comet_bft` for a single transaction on a mempool with the
/// `comet_bft_removal_cache` filled.
///
/// Note that the number of entries in the main cache is irrelevant here.
#[divan::bench(max_time = MAX_TIME)]
fn check_removed_comet_bft(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| {
            let tx_id = TransactionId::new(Sha256::digest(0_usize.to_le_bytes()).into());
            (init_mempool::<mempool_with_100_txs>(), tx_id)
        })
        .bench_values(move |(mempool, tx_id)| {
            runtime.block_on(async {
                mempool.remove_from_removal_cache(&tx_id).await;
            });
        });
}

/// Benchmarks `Mempool::run_maintenance` on a mempool with the given number of existing entries.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn run_maintenance<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    // Set the new nonce so that the entire `REMOVAL_CACHE_SIZE` entries in the
    // `comet_bft_removal_cache` are filled (assuming this test case has enough txs).
    let new_nonce = u32::try_from(super::REMOVAL_CACHE_SIZE)
        .unwrap()
        .checked_div(u32::from(SIGNER_COUNT))
        .and_then(|res| res.checked_add(1))
        .unwrap();

    // iterate over all signers and put their nonces into the mock state
    let mut fixture = new_fixture();
    for i in 0..SIGNER_COUNT {
        let signing_key = SigningKey::from([i; 32]);
        let signing_address = signing_key.address_bytes();
        fixture
            .state_mut()
            .put_account_nonce(&signing_address, new_nonce)
            .unwrap();
    }
    let state = fixture.state();

    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool
                    .run_maintenance(state, false, &HashSet::new(), 1)
                    .await;
            });
        });
}

/// Benchmarks `Mempool::run_maintenance` on a mempool with the given number of existing entries.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn run_maintenance_tx_recosting<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    // Set the new nonce so that the entire `REMOVAL_CACHE_SIZE` entries in the
    // `comet_bft_removal_cache` are filled (assuming this test case has enough txs).
    let new_nonce = u32::try_from(super::REMOVAL_CACHE_SIZE)
        .unwrap()
        .checked_div(u32::from(SIGNER_COUNT))
        .and_then(|res| res.checked_add(1))
        .unwrap();

    // iterate over all signers and put their nonces into the mock state
    let mut fixture = new_fixture();
    for i in 0..SIGNER_COUNT {
        let signing_key = SigningKey::from([i; 32]);
        let signing_address = signing_key.address_bytes();
        fixture
            .state_mut()
            .put_account_nonce(&signing_address, new_nonce)
            .unwrap();
    }
    let state = fixture.state();

    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool
                    .run_maintenance(state, true, &HashSet::new(), 1)
                    .await;
            });
        });
}

/// Benchmarks `Mempool::pending_nonce` on a mempool with the given number of existing entries.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn pending_nonce<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| {
            let address = transactions()
                .first()
                .unwrap()
                .verification_key()
                .address_bytes();
            (init_mempool::<T>(), address)
        })
        .bench_values(move |(mempool, address)| {
            runtime.block_on(async {
                mempool.pending_nonce(address).await.unwrap();
            });
        });
}
