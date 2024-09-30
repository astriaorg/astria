//! To run the benchmark, from the root of the monorepo, run:
//! ```sh
//! cargo bench --features=benchmark -qp astria-sequencer mempool
//! ```
#![expect(non_camel_case_types, reason = "for benchmark")]

use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::{
    crypto::SigningKey,
    protocol::transaction::v1alpha1::SignedTransaction,
};
use sha2::{
    Digest as _,
    Sha256,
};

use crate::{
    app::test_utils::{
        mock_balances,
        mock_state_getter,
        mock_state_put_account_balances,
        mock_state_put_account_nonce,
        mock_tx_cost,
    },
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

fn transactions() -> &'static Vec<Arc<SignedTransaction>> {
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
    let account_mock_balance = mock_balances(0, 0);
    let tx_mock_cost = mock_tx_cost(0, 0, 0);
    runtime.block_on(async {
        for tx in transactions().iter().take(T::checked_size()) {
            mempool
                .insert(
                    tx.clone(),
                    0,
                    account_mock_balance.clone(),
                    tx_mock_cost.clone(),
                )
                .await
                .unwrap();
        }
        for i in 0..super::REMOVAL_CACHE_SIZE {
            let hash = Sha256::digest(i.to_le_bytes()).into();
            mempool
                .comet_bft_removal_cache
                .write()
                .await
                .add(hash, RemovalReason::Expired);
        }
    });
    mempool
}

/// Returns the first transaction from the static `transactions()` not included in the initialized
/// mempool, i.e. the one at index `T::size()`.
fn get_unused_tx<T: MempoolSize>() -> Arc<SignedTransaction> {
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
    let mock_balances = mock_balances(0, 0);
    let mock_tx_cost = mock_tx_cost(0, 0, 0);
    bencher
        .with_inputs(|| {
            (
                init_mempool::<T>(),
                get_unused_tx::<T>(),
                mock_balances.clone(),
                mock_tx_cost.clone(),
            )
        })
        .bench_values(move |(mempool, tx, mock_balances, mock_tx_cost)| {
            runtime.block_on(async {
                mempool
                    .insert(tx, 0, mock_balances, mock_tx_cost)
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
        .enable_all()
        .build()
        .unwrap();

    let mut mock_state = runtime.block_on(mock_state_getter());

    // iterate over all signers and put their balances and nonces into the mock state
    for i in 0..SIGNER_COUNT {
        let signing_key = SigningKey::from([i; 32]);
        let signing_address = signing_key.address_bytes();
        mock_state_put_account_nonce(&mut mock_state, &signing_address, 0);
    }

    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool.builder_queue(&mock_state).await.unwrap();
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
        .enable_all()
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
    // `comet_bft_removal_cache` are filled (assuming this test case has enough txs).
    let new_nonce = u32::try_from(super::REMOVAL_CACHE_SIZE)
        .unwrap()
        .checked_div(u32::from(SIGNER_COUNT))
        .and_then(|res| res.checked_add(1))
        .unwrap();
    let mock_balances = mock_balances(0, 0);
    let mut mock_state = runtime.block_on(mock_state_getter());

    // iterate over all signers and put their balances and nonces into the mock state
    for i in 0..SIGNER_COUNT {
        let signing_key = SigningKey::from([i; 32]);
        let signing_address = signing_key.address_bytes();
        mock_state_put_account_balances(&mut mock_state, &signing_address, mock_balances.clone());
        mock_state_put_account_nonce(&mut mock_state, &signing_address, new_nonce);
    }

    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool.run_maintenance(&mock_state, false).await;
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
        .enable_all()
        .build()
        .unwrap();
    // Set the new nonce so that the entire `REMOVAL_CACHE_SIZE` entries in the
    // `comet_bft_removal_cache` are filled (assuming this test case has enough txs).
    let new_nonce = u32::try_from(super::REMOVAL_CACHE_SIZE)
        .unwrap()
        .checked_div(u32::from(SIGNER_COUNT))
        .and_then(|res| res.checked_add(1))
        .unwrap();
    let mock_balances = mock_balances(0, 0);
    let mut mock_state = runtime.block_on(mock_state_getter());

    // iterate over all signers and put their balances and nonces into the mock state
    for i in 0..SIGNER_COUNT {
        let signing_key = SigningKey::from([i; 32]);
        let signing_address = signing_key.address_bytes();
        mock_state_put_account_balances(&mut mock_state, &signing_address, mock_balances.clone());
        mock_state_put_account_nonce(&mut mock_state, &signing_address, new_nonce);
    }

    bencher
        .with_inputs(|| init_mempool::<T>())
        .bench_values(move |mempool| {
            runtime.block_on(async {
                mempool.run_maintenance(&mock_state, true).await;
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
