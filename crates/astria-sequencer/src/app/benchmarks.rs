//! To run the benchmark, from the root of the monorepo, run:
//! ```sh
//! cargo bench --features=benchmark -qp astria-sequencer app
//! ```

use std::time::Duration;

use cnidarium::Storage;

use crate::{
    app::App,
    benchmark_utils::{
        self,
        TxTypes,
        SIGNER_COUNT,
    },
    proposal::block_size_constraints::BlockSizeConstraints,
    test_utils::{
        astria_address,
        dummy_balances,
        dummy_tx_costs,
    },
};

/// The max time for any benchmark.
const MAX_TIME: Duration = Duration::from_secs(120);
/// The value provided to `BlockSizeConstraints::new` to constrain block sizes.
///
/// Taken from the actual value seen in `prepare_proposal.max_tx_bytes` when handling
/// `prepare_proposal` during stress testing using spamoor.
const COMETBFT_MAX_TX_BYTES: i64 = 22_019_254;

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
            .map(|(index, signing_key)| {
                (
                    astria_address(&signing_key.address_bytes()),
                    10_u128
                        .pow(19)
                        .saturating_add(u128::try_from(index).unwrap()),
                )
            });
        let first_signing_key = benchmark_utils::signing_keys().next().unwrap();
        let first_address = astria_address(&first_signing_key.address_bytes());
        let mut fixture = crate::test_utils::Fixture::uninitialized(None).await;
        fixture
            .chain_initializer()
            .with_genesis_accounts(accounts)
            .with_authority_sudo_address(first_address)
            .with_ibc_sudo_address(first_address)
            .init()
            .await;

        let (app, storage) = fixture.destructure();

        let dummy_balances = dummy_balances(0, 0);
        let dummy_tx_costs = dummy_tx_costs(0, 0, 0);

        for tx in benchmark_utils::transactions(TxTypes::AllTransfers) {
            dbg!(tx);
            dbg!(&dummy_balances);
            dbg!(&dummy_tx_costs);
            // app.mempool
            //     .insert(
            //         tx.clone(),
            //         0,
            //         dummy_balances.clone(),
            //         dummy_tx_costs.clone(),
            //     )
            //     .await
            //     .unwrap();
        }
        Fixture {
            app,
            _storage: storage,
        }
    }
}

#[divan::bench(max_time = MAX_TIME)]
fn prepare_proposal_tx_execution(bencher: divan::Bencher) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut fixture = runtime.block_on(async { Fixture::new().await });
    bencher
        .with_inputs(|| BlockSizeConstraints::new(COMETBFT_MAX_TX_BYTES, true).unwrap())
        .bench_local_refs(|constraints| {
            let included_txs = runtime.block_on(async {
                fixture
                    .app
                    .prepare_proposal_tx_execution(*constraints)
                    .await
                    .unwrap()
            });
            // Ensure we actually processed some txs.  This will trip if execution fails for all
            // txs, or more likely, if the mempool becomes exhausted of txs.
            assert!(!included_txs.is_empty());
        });
}
