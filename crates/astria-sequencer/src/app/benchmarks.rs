use std::time::Duration;

use astria_core::sequencer::{
    Account,
    AddressPrefixes,
    GenesisState,
    UncheckedGenesisState,
};
use cnidarium::Storage;
use penumbra_ibc::params::IBCParameters;

use crate::{
    app::{
        test_utils,
        App,
    },
    benchmark_utils::{
        self,
        TxTypes,
        SIGNER_COUNT,
    },
    proposal::block_size_constraints::BlockSizeConstraints,
    test_utils::{
        astria_address,
        nria,
        ASTRIA_PREFIX,
    },
};

/// The max time for any benchmark.
const MAX_TIME: Duration = Duration::from_secs(120);

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
            .collect::<Vec<_>>();
        let address_prefixes = AddressPrefixes {
            base: ASTRIA_PREFIX.into(),
        };
        let first_address = accounts.first().unwrap().address;
        let unchecked_genesis_state = UncheckedGenesisState {
            accounts,
            address_prefixes,
            authority_sudo_address: first_address,
            ibc_sudo_address: first_address,
            ibc_relayer_addresses: vec![],
            native_asset_base_denomination: nria(),
            ibc_params: IBCParameters::default(),
            allowed_fee_assets: vec!["nria".parse().unwrap()],
            fees: test_utils::default_fees(),
        };
        let genesis_state = GenesisState::try_from(unchecked_genesis_state).unwrap();

        let (app, storage) =
            test_utils::initialize_app_with_storage(Some(genesis_state), vec![]).await;

        for tx in benchmark_utils::transactions(TxTypes::AllTransfers) {
            app.mempool.insert(tx.clone(), 0).await.unwrap();
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
        .with_inputs(|| BlockSizeConstraints::new(22_019_254).unwrap())
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
