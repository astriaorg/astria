use std::{
    io::Write as _,
    net::SocketAddr,
};

use astria_bridge_withdrawer::{
    bridge_withdrawer::ShutdownHandle,
    BridgeWithdrawer,
    Config,
};
use astria_core::{
    bridge::Ics20WithdrawalFromRollupMemo,
    primitive::v1::asset::{
        self,
        Denom,
    },
    protocol::transaction::v1alpha1::{
        action::{
            BridgeUnlockAction,
            Ics20Withdrawal,
        },
        Action,
    },
};
use ibc_types::core::client::Height as IbcHeight;
use once_cell::sync::Lazy;
use sequencer_client::Address;
use tempfile::NamedTempFile;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::MockSequencerServer;

const DEFAULT_LAST_ROLLUP_HEIGHT: u64 = 1;
const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";
const DEFAULT_DENOM: &str = "nria";
const SEQUENCER_CHAIN_ID: &str = "test-sequencer";
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

    /// A handle to issue a shutdown to the bridge withdrawer.
    bridge_withdrawer_shutdown_handle: Option<ShutdownHandle>,
    bridge_withdrawer: JoinHandle<()>,

    pub config: Config,
}

impl TestBridgeWithdrawer {
    pub async fn spawn() -> Self {
        Lazy::force(&TELEMETRY);

        let shutdown_token = CancellationToken::new();

        // sequencer signer key
        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(
                "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90".as_bytes(),
            )
            .unwrap();
        let sequencer_key_path = keyfile.path().to_str().unwrap().to_string();

        let cometbft_mock = wiremock::MockServer::start().await;

        let sequencer_mock = MockSequencerServer::spawn().await;
        let sequencer_grpc_endpoint = format!("http://{}", sequencer_mock.local_addr);

        let config = Config {
            sequencer_cometbft_endpoint: cometbft_mock.uri(),
            sequencer_grpc_endpoint,
            sequencer_chain_id: SEQUENCER_CHAIN_ID.into(),
            sequencer_key_path,
            fee_asset_denomination: DEFAULT_DENOM.parse().unwrap(),
            min_expected_fee_asset_balance: 100,
            rollup_asset_denomination: DEFAULT_DENOM.parse().unwrap(),
            sequencer_bridge_address: default_bridge_address().to_string(),
            ethereum_contract_address: todo!(),
            ethereum_rpc_endpoint: todo!(),
            sequencer_address_prefix: ASTRIA_ADDRESS_PREFIX.into(),
            api_addr: "0.0.0.0".into(),
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

        Self {
            api_address,
            cometbft_mock,
            sequencer_mock,
            bridge_withdrawer_shutdown_handle: Some(bridge_withdrawer_shutdown_handle),
            bridge_withdrawer,
            config,
        }
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
        memo: serde_json::to_vec(&BridgeUnlockMemo {
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT.into(),
            transaction_hash: [1u8; 32].into(),
        })
        .unwrap(),
        fee_asset: denom,
        bridge_address: None,
    };
    Action::BridgeUnlock(inner)
}

pub(crate) fn default_native_asset() -> asset::Denom {
    "nria".parse().unwrap()
}

fn default_bridge_address() -> Address {
    astria_address([1u8; 20])
}

/// Constructs an [`Address`] prefixed by `"astria"`.
pub(crate) fn astria_address(
    array: [u8; astria_core::primitive::v1::ADDRESS_LEN],
) -> astria_core::primitive::v1::Address {
    astria_core::primitive::v1::Address::builder()
        .array(array)
        .prefix(ASTRIA_ADDRESS_PREFIX)
        .try_build()
        .unwrap()
}
