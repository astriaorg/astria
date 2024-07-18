#![allow(non_camel_case_types)]

use std::{
    collections::HashMap,
    sync::{
        Arc,
        OnceLock,
    },
    time::Duration,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        asset::{
            Denom,
            IbcPrefixed,
        },
        Address,
        RollupId,
    },
    protocol::transaction::v1alpha1::{
        action::{
            Action,
            SequenceAction,
        },
        SignedTransaction,
        TransactionParams,
        UnsignedTransaction,
    },
};
use sha2::{
    Digest as _,
    Sha256,
};

use super::{
    Mempool,
    RemovalReason,
};

/// The maximum number of transactions with which to initialize the mempool.
const MAX_INITIAL_TXS: usize = 100_000;
/// The max time for any benchmark.
const MAX_TIME: Duration = Duration::from_secs(30);
/// The number of different signers of transactions, and also the number of different chain IDs.
const SIGNER_COUNT: u8 = 10;

/// Returns an endlessly-repeating iterator over `SIGNER_COUNT` separate signing keys.
fn signing_keys() -> impl Iterator<Item = &'static SigningKey> {
    static SIGNING_KEYS: OnceLock<Vec<SigningKey>> = OnceLock::new();
    SIGNING_KEYS
        .get_or_init(|| {
            (0..SIGNER_COUNT)
                .map(|i| SigningKey::from([i; 32]))
                .collect()
        })
        .iter()
        .cycle()
}

/// Returns a static ref to a collection of `MAX_INITIAL_TXS + 1` transactions.
fn transactions() -> &'static Vec<Arc<SignedTransaction>> {
    static TXS: OnceLock<Vec<Arc<SignedTransaction>>> = OnceLock::new();
    TXS.get_or_init(|| {
        crate::address::initialize_base_prefix("benchmarks").unwrap();
        let mut nonces_and_chain_ids = HashMap::new();
        signing_keys()
            .map(move |signing_key| {
                let verification_key = signing_key.verification_key();
                let (nonce, chain_id) = nonces_and_chain_ids
                    .entry(verification_key)
                    .or_insert_with(|| {
                        (0_u32, format!("chain-{}", signing_key.verification_key()))
                    });
                *nonce = (*nonce).wrapping_add(1);
                let params = TransactionParams::builder()
                    .nonce(*nonce)
                    .chain_id(chain_id.as_str())
                    .build();
                let sequence_action = SequenceAction {
                    rollup_id: RollupId::new([1; 32]),
                    data: vec![2; 1000],
                    fee_asset: Denom::IbcPrefixed(IbcPrefixed::new([3; 32])),
                };
                let tx = UnsignedTransaction {
                    actions: vec![Action::Sequence(sequence_action)],
                    params,
                }
                .into_signed(signing_key);
                Arc::new(tx)
            })
            .take(MAX_INITIAL_TXS + 1)
            .collect()
    })
}

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
        assert!(Self::size() <= MAX_INITIAL_TXS);
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
    let current_account_nonce_getter = |_: Address| async { Ok(new_nonce) };
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
            let address = crate::address::base_prefixed(
                transactions()
                    .first()
                    .unwrap()
                    .verification_key()
                    .address_bytes(),
            );
            (init_mempool::<T>(), address)
        })
        .bench_values(move |(mempool, address)| {
            runtime.block_on(async {
                mempool.pending_nonce(&address).await.unwrap();
            });
        });
}
