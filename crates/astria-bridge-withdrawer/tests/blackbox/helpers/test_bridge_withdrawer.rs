use astria_bridge_withdrawer::{bridge_withdrawer, BridgeWithdrawer};
use astria_core::{primitive::v1::asset::default_native_asset, protocol::transaction::v1alpha1::action::BridgeUnlockAction};
use astria_eyre::eyre;
use once_cell::sync::Lazy;
use tokio::task::JoinHandle;
use wiremock::MockServer;

const DEFAULT_LAST_ROLLUP_HEIGHT: u64 = 1;
const DEFAULT_IBC_DENOM: &str = "transfer/channel-0/utia";

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

struct TestBridgeWithdrawer {
    bridge_withdrawer: Option<BridgeWithdrawer>,
    cometbft_mock: MockServer,
    grpc_mock: MockServer,
    bridge_withdrawer_task_handle: Option<JoinHandle<Result<(), eyre::Report>>>,
}

impl TestBridgeWithdrawer {
    async fn setup() -> Self {
        Lazy::force(&TELEMETRY);

        // set up external resources
        let shutdown_token = CancellationToken::new();

        // sequencer signer key
        let keyfile = NamedTempFile::new().unwrap();
        (&keyfile)
            .write_all(
                "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90".as_bytes(),
            )
            .unwrap();
        let sequencer_key_path = keyfile.path().to_str().unwrap().to_string();

        // cometbft
        let cometbft_mock = MockServer::start().await;
        let sequencer_cometbft_endpoint = format!("http://{}", cometbft_mock.address());
        // TODO: use grpc mock
        let sequencer_grpc_endpoint = format!("http://{}", cometbft_mock.address());

        // withdrawer state
        let state = Arc::new(state::State::new());
        // not testing watcher here so just set it to ready
        state.set_watcher_ready();
        let (startup_tx, startup_rx) = oneshot::channel();
        let startup_handle = startup::SubmitterHandle::new(startup_rx);

        let metrics = Box::leak(Box::new(Metrics::new()));

        let (submitter, submitter_handle) = submitter::Builder {
            shutdown_token: shutdown_token.clone(),
            startup_handle,
            sequencer_key_path,
            sequencer_address_prefix: "astria".into(),
            sequencer_cometbft_endpoint,
            sequencer_grpc_endpoint,
            state,
            metrics,
        }
        .build()
        .unwrap();

        Self {
            todo!()
        }
    }

    async fn startup(&mut self) {
        let submitter = self.submitter.take().unwrap();

        let mut state = submitter.state.subscribe();

        self.bridge_withdrawer_task_handle = Some(tokio::spawn(submitter.run()));

        self.startup_tx
            .take()
            .expect("should only send startup info once")
            .send(startup::SubmitterInfo {
                sequencer_chain_id: SEQUENCER_CHAIN_ID.to_string(),
            })
            .unwrap();

        // wait for the submitter to be ready
        state
            .wait_for(state::StateSnapshot::is_ready)
            .await
            .unwrap();
    }

    async fn spawn() -> Self {
        let mut submitter = Self::setup().await;
        submitter.startup().await;
        submitter
    }
}

fn make_ics20_withdrawal_action() -> Action {
    let denom = DEFAULT_IBC_DENOM.parse::<Denom>().unwrap();
    let destination_chain_address = "address".to_string();
    let inner = Ics20Withdrawal {
        denom: denom.clone(),
        destination_chain_address,
        return_address: bridge_withdrawer::astria_address([0u8; 20]),
        amount: 99,
        memo: serde_json::to_string(&Ics20WithdrawalFromRollupMemo {
            memo: "hello".to_string(),
            bridge_address: bridge_withdrawer::astria_address([0u8; 20]),
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT,
            transaction_hash: [2u8; 32],
        })
        .unwrap(),
        fee_asset_id: denom.id(),
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
        to: bridge_withdrawer::astria_address([0u8; 20]),
        amount: 99,
        memo: serde_json::to_vec(&BridgeUnlockMemo {
            block_number: DEFAULT_LAST_ROLLUP_HEIGHT.into(),
            transaction_hash: [1u8; 32].into(),
        })
        .unwrap(),
        fee_asset_id: denom.id(),
        bridge_address: None,
    };
    Action::BridgeUnlock(inner)
}

/// Convert a `Request` object to a `SignedTransaction`
fn signed_tx_from_request(request: &Request) -> SignedTransaction {
    use astria_core::generated::protocol::transaction::v1alpha1::SignedTransaction as RawSignedTransaction;
    use prost::Message as _;

    let wrapped_tx_sync_req: request::Wrapper<tx_sync::Request> =
        serde_json::from_slice(&request.body)
            .expect("deserialize to JSONRPC wrapped tx_sync::Request");
    let raw_signed_tx = RawSignedTransaction::decode(&*wrapped_tx_sync_req.params().tx)
        .expect("can't deserialize signed sequencer tx from broadcast jsonrpc request");
    let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx)
        .expect("can't convert raw signed tx to checked signed tx");
    debug!(?signed_tx, "sequencer mock received signed transaction");

    signed_tx
}
