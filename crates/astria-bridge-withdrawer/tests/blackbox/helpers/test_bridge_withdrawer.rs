use std::{
    io::Write as _,
    mem,
    net::SocketAddr,
    time::Duration,
};

use astria_bridge_withdrawer::{
    BridgeWithdrawer,
    Config,
    ShutdownHandle,
};
use astria_core::{
    bridge::{
        Ics20WithdrawalFromRollupMemo,
        UnlockMemo,
    },
    primitive::v1::asset::{
        self,
        Denom,
    },
    protocol::{
        bridge::v1alpha1::BridgeAccountLastTxHashResponse,
        transaction::v1alpha1::{
            action::{
                BridgeUnlockAction,
                Ics20Withdrawal,
            },
            Action,
        },
    },
};
use futures::Future;
use ibc_types::core::client::Height as IbcHeight;
use once_cell::sync::Lazy;
use sequencer_client::{
    Address,
    NonceResponse,
};
use tempfile::NamedTempFile;
use tokio::task::JoinHandle;
use tracing::{
    debug,
    error,
    info,
};

use super::{
    make_tx_commit_success_response,
    mock_cometbft::{
        mount_default_chain_id,
        mount_default_fee_assets,
        mount_default_min_expected_fee_asset_balance,
        mount_get_nonce_response,
    },
    mount_broadcast_tx_commit_response_as_scoped,
    mount_last_bridge_tx_hash_response,
    MockSequencerServer,
};
use crate::helpers::ethereum::{
    AstriaWithdrawerDeployerConfig,
    TestEthereum,
    TestEthereumConfig,
};

const DEFAULT_LAST_ROLLUP_HEIGHT: u64 = 1;
const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";
pub(crate) const SEQUENCER_CHAIN_ID: &str = "test-sequencer";
const ASTRIA_ADDRESS_PREFIX: &str = "astria";

static TELEMETRY: Lazy<()> = Lazy::new(|| {
    if std::env::var_os("TEST_LOG").is_some() {
        let filter_directives = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::stdout)
            .set_pretty_print(true)
            .filter_directives(&filter_directives)
            .try_init()
            .unwrap();
    } else {
        telemetry::configure()
            .no_otel()
            .stdout_writer(std::io::sink)
            .try_init()
            .unwrap();
    }
});

pub struct TestBridgeWithdrawer {
    /// The address of the public API server (health, ready).
    pub api_address: SocketAddr,

    /// The mock cometbft server.
    pub cometbft_mock: wiremock::MockServer,

    /// The mock sequencer server.
    pub sequencer_mock: MockSequencerServer,

    /// The rollup-side ethereum smart contract
    pub ethereum: TestEthereum,

    /// A handle to issue a shutdown to the bridge withdrawer.
    bridge_withdrawer_shutdown_handle: Option<ShutdownHandle>,

    /// The bridge withdrawer task.
    bridge_withdrawer: JoinHandle<()>,

    /// The config used to initialize the bridge withdrawer.
    pub config: Config,
}

impl Drop for TestBridgeWithdrawer {
    fn drop(&mut self) {
        debug!("dropping TestBridgeWithdrawer");

        // Drop the shutdown handle to cause the bridge withdrawer to shutdown.
        let _ = self.bridge_withdrawer_shutdown_handle.take();

        let bridge_withdrawer = mem::replace(&mut self.bridge_withdrawer, tokio::spawn(async {}));
        let _ = futures::executor::block_on(async move {
            tokio::time::timeout(Duration::from_secs(2), bridge_withdrawer)
                .await
                .unwrap_or_else(|_| {
                    error!("timeout out waiting for bridge withdrawer to shut down");
                    Ok(())
                })
        });
    }
}

impl TestBridgeWithdrawer {
    pub async fn spawn() -> Self {
        Lazy::force(&TELEMETRY);

        // sequencer signer key
        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(
                "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90".as_bytes(),
            )
            .unwrap();
        let sequencer_key_path = keyfile.path().to_str().unwrap().to_string();

        // TODO: option for configuring this
        let ethereum =
            TestEthereumConfig::AstriaWithdrawer(AstriaWithdrawerDeployerConfig::default())
                .spawn()
                .await;

        let cometbft_mock = wiremock::MockServer::start().await;

        let sequencer_mock = MockSequencerServer::spawn().await;
        let sequencer_grpc_endpoint = sequencer_mock.local_addr.to_string();

        let config = Config {
            sequencer_cometbft_endpoint: cometbft_mock.uri(),
            sequencer_grpc_endpoint,
            sequencer_chain_id: SEQUENCER_CHAIN_ID.into(),
            sequencer_key_path,
            fee_asset_denomination: default_native_asset(),
            rollup_asset_denomination: default_native_asset().to_string(),
            sequencer_bridge_address: default_bridge_address().to_string(),
            ethereum_contract_address: ethereum.contract_address(),
            ethereum_rpc_endpoint: ethereum.ws_endpoint(),
            sequencer_address_prefix: ASTRIA_ADDRESS_PREFIX.into(),
            api_addr: "0.0.0.0:0".into(),
            log: String::new(),
            force_stdout: false,
            no_otel: false,
            no_metrics: false,
            metrics_http_listener_addr: String::new(),
            pretty_print: true,
        };

        info!(config = serde_json::to_string(&config).unwrap());
        let (bridge_withdrawer, bridge_withdrawer_shutdown_handle) =
            BridgeWithdrawer::new(config.clone()).unwrap();
        let api_address = bridge_withdrawer.local_addr();
        let bridge_withdrawer = tokio::task::spawn(bridge_withdrawer.run());

        let mut test_bridge_withdrawer = Self {
            api_address,
            ethereum,
            cometbft_mock,
            sequencer_mock,
            bridge_withdrawer_shutdown_handle: Some(bridge_withdrawer_shutdown_handle),
            bridge_withdrawer,
            config,
        };

        test_bridge_withdrawer.mount_startup_responses().await;

        test_bridge_withdrawer
    }

    pub async fn mount_startup_responses(&mut self) {
        self.mount_sequencer_config_responses().await;
        self.mount_wait_for_mempool_response().await;
        self.mount_last_bridge_tx_responses().await;
    }

    async fn mount_sequencer_config_responses(&mut self) {
        mount_default_chain_id(&self.cometbft_mock).await;
        mount_default_fee_assets(&self.cometbft_mock).await;
        mount_default_min_expected_fee_asset_balance(&self.cometbft_mock).await;
    }

    async fn mount_wait_for_mempool_response(&mut self) {
        // TODO: add config to allow testing for non-empty mempool
        let empty_mempool_response = NonceResponse {
            height: 0,
            nonce: 1,
        };
        mount_get_nonce_response(&self.cometbft_mock, empty_mempool_response).await;

        self.sequencer_mock
            .mount_pending_nonce_response(1, "startup::wait_for_mempool()")
            .await;
    }

    async fn mount_last_bridge_tx_responses(&mut self) {
        // TODO: add config to allow testing sync
        mount_last_bridge_tx_hash_response(
            &self.cometbft_mock,
            BridgeAccountLastTxHashResponse {
                height: 0,
                tx_hash: None,
            },
        )
        .await
    }

    /// Executes `future` within the specified duration, returning its result.
    ///
    /// If execution takes more than 80% of the allowed time, an error is logged before returning.
    ///
    /// # Panics
    ///
    /// Panics if execution takes longer than the specified duration.
    pub async fn timeout_ms<F: Future>(
        &self,
        num_milliseconds: u64,
        context: &str,
        future: F,
    ) -> F::Output {
        let start = std::time::Instant::now();
        let within = Duration::from_millis(num_milliseconds);
        if let Ok(value) = tokio::time::timeout(within, future).await {
            let elapsed = start.elapsed();
            if elapsed.checked_mul(5).unwrap() > within.checked_mul(4).unwrap() {
                error!(%context,
                    "elapsed time ({} seconds) was over 80% of the specified timeout ({} \
                     seconds) - consider increasing the timeout",
                    elapsed.as_secs_f32(),
                    within.as_secs_f32()
                );
            }
            debug!(context, "future executed without timeout");
            value
        } else {
            // TODO: add handing of failed future using the api server like in sequencer-relayer
            panic!(
                "{} timed out after {} milliseconds",
                context, num_milliseconds
            );
        }
    }

    pub async fn mount_pending_nonce_response_as_scoped(
        &self,
        nonce: u32,
        debug_name: &str,
    ) -> astria_grpc_mock::MockGuard {
        self.sequencer_mock
            .mount_pending_nonce_response_as_scoped(nonce, debug_name)
            .await
    }

    pub async fn mount_broadcast_tx_commit_success_response_as_scoped(
        &self,
    ) -> wiremock::MockGuard {
        mount_broadcast_tx_commit_response_as_scoped(
            &self.cometbft_mock,
            make_tx_commit_success_response(),
        )
        .await
    }
}

fn make_ics20_withdrawal_action() -> Action {
    let denom = DEFAULT_IBC_DENOM.parse::<Denom>().unwrap();
    let destination_chain_address = "address".to_string();
    let inner = Ics20Withdrawal {
        denom: denom.clone(),
        destination_chain_address,
        return_address: astria_address([0u8; 20]),
        amount: 99,
        memo: serde_json::to_string(&Ics20WithdrawalFromRollupMemo {
            memo: "hello".to_string(),
            bridge_address: astria_address([0u8; 20]),
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT,
            transaction_hash: [2u8; 32],
        })
        .unwrap(),
        fee_asset: denom,
        timeout_height: IbcHeight::new(u64::MAX, u64::MAX).unwrap(),
        timeout_time: 0, // zero this for testing
        source_channel: "channel-0".parse().unwrap(),
        bridge_address: None,
    };

    Action::Ics20Withdrawal(inner)
}

fn make_bridge_unlock_action() -> Action {
    let denom = default_native_asset();
    let inner = BridgeUnlockAction {
        to: astria_address([0u8; 20]),
        amount: 99,
        memo: serde_json::to_vec(&UnlockMemo {
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT.into(),
            transaction_hash: [1u8; 32].into(),
        })
        .unwrap(),
        fee_asset: denom,
        bridge_address: None,
    };
    Action::BridgeUnlock(inner)
}

pub fn default_native_asset() -> asset::Denom {
    "nria".parse().unwrap()
}

fn default_bridge_address() -> Address {
    astria_address([1u8; 20])
}

/// Constructs an [`Address`] prefixed by `"astria"`.
pub fn astria_address(
    array: [u8; astria_core::primitive::v1::ADDRESS_LEN],
) -> astria_core::primitive::v1::Address {
    astria_core::primitive::v1::Address::builder()
        .array(array)
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap()
}
