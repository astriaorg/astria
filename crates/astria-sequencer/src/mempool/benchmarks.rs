#![allow(non_camel_case_types)]

use std::time::Duration;

use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use sha2::{
    Digest as _,
    Sha256,
};

use crate::{
    benchmark_utils::SIGNER_COUNT,
    mempool::{
        Mempool,
        RemovalReason,
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

fn transactions() -> &'static Vec<SignedTransaction> {
    crate::benchmark_utils::transactions(crate::benchmark_utils::TxTypes::AllSequenceActions)
}

/// Returns a new `Mempool` initialized with the number of transactions specified by `T::size()`
/// taken from the static `transactions()`, and with a full `comet_bft_removal_cache`.
fn init_mempool<T: MempoolSize>() -> Mempool {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mempool = Mempool::new();
    runtime.block_on(async {
        for tx in transactions().iter().take(T::checked_size()) {
            mempool.insert(tx.clone(), 0).await.unwrap();
        }
        for i in 0..super::REMOVAL_CACHE_SIZE {
            let hash = Sha256::digest(i.to_le_bytes()).into();
            mempool
                .track_removal_comet_bft(hash, RemovalReason::Expired)
                .await;
        }
    });
    mempool
}

/// Returns the first transaction from the static `transactions()` not included in the initialized
/// mempool, i.e. the one at index `T::size()`.
fn get_unused_tx<T: MempoolSize>() -> SignedTransaction {
    transactions().get(T::checked_size()).unwrap().clone()
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
        .enable_all()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| (init_mempool::<T>(), get_unused_tx::<T>()))
        .bench_values(move |(mempool, tx)| {
            runtime.block_on(async {
                mempool.insert(tx, 0).await.unwrap();
            });
        });
}

/// Benchmarks `Mempool::pop` on a mempool with the given number of existing entries.
#[divan::bench(
    max_time = MAX_TIME,
    types = [
        mempool_with_100_txs,
        mempool_with_1000_txs,
        mempool_with_10000_txs,
        mempool_with_100000_txs
    ]
)]
fn pop<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool.pop().await.unwrap();
            });
        });
}

/// Benchmarks `Mempool::remove` for a single transaction on a mempool with the given number of
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
fn remove<T: MempoolSize>(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| {
            let tx_hash = transactions().first().unwrap().sha256_of_proto_encoding();
            (init_mempool::<T>(), tx_hash)
        })
        .bench_values(move |(mempool, tx_hash)| {
            runtime.block_on(async {
                mempool.remove(tx_hash).await;
            });
        });
}

/// Benchmarks `Mempool::track_removal_comet_bft` for a single new transaction on a mempool with
/// the `comet_bft_removal_cache` filled.
///
/// Note that the number of entries in the main cache is irrelevant here.
#[divan::bench(max_time = MAX_TIME)]
fn track_removal_comet_bft(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| {
            let tx_hash = transactions().first().unwrap().sha256_of_proto_encoding();
            (init_mempool::<mempool_with_100_txs>(), tx_hash)
        })
        .bench_values(move |(mempool, tx_hash)| {
            runtime.block_on(async {
                mempool
                    .track_removal_comet_bft(tx_hash, RemovalReason::Expired)
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
        .enable_all()
        .build()
        .unwrap();
    bencher
        .with_inputs(|| {
            let tx_hash = Sha256::digest(0_usize.to_le_bytes()).into();
            (init_mempool::<mempool_with_100_txs>(), tx_hash)
        })
        .bench_values(move |(mempool, tx_hash)| {
            runtime.block_on(async {
                mempool.check_removed_comet_bft(tx_hash).await.unwrap();
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
        .enable_all()
        .build()
        .unwrap();
    // Set the new nonce so that the entire `REMOVAL_CACHE_SIZE` entries in the
    // `comet_bft_removal_cache` are replaced (assuming this test case has enough txs).
    // allow: this is test-only code, using small values, and where the result is not critical.
    #[allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]
    let new_nonce = (super::REMOVAL_CACHE_SIZE as u32 / u32::from(SIGNER_COUNT)) + 1;
    // Although in production this getter will be hitting the state store and will be slower than
    // this test one, it's probably insignificant as the getter is only called once per address,
    // and we don't expect a high number of discrete addresses in the mempool entries.
    let current_account_nonce_getter = |_: [u8; 20]| async { Ok(new_nonce) };
    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool
                    .run_maintenance(current_account_nonce_getter)
                    .await
                    .unwrap();
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
        .enable_all()
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
