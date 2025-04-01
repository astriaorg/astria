use std::time::Duration;

use astria_bridge_signer::{
    BridgeSigner,
    Config,
};
use astria_core::{
    generated::astria::signer::v1::{
        frost_participant_service_client::FrostParticipantServiceClient,
        CommitmentWithIdentifier,
        ExecuteRoundOneRequest,
        ExecuteRoundTwoRequest,
        GetVerifyingShareRequest,
        RoundOneResponse,
        RoundTwoResponse,
        VerifyingShare,
    },
    protocol::transaction::v1::{
        action::Ics20Withdrawal,
        TransactionBody,
    },
    Protobuf as _,
};
use prost::Message as _;
use telemetry::metrics;
use tokio::{
    net::TcpListener,
    time::sleep,
};
use tonic::{
    transport::Channel,
    Request,
    Response,
    Status,
};

use super::mock_rollup::MockRollup;

pub(crate) struct TestBridgeSigner {
    mock_evm: MockRollup,
    frost_client: FrostParticipantServiceClient<Channel>,
}

impl TestBridgeSigner {
    pub(crate) async fn spawn() -> Self {
        let mock_evm = MockRollup::new().await;

        // We need to create a temporary listener to randomly assign a port, this way when tests are
        // being run concurrently they aren't on the same port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cfg = Config {
            grpc_endpoint: listener.local_addr().unwrap().to_string(),
            frost_secret_key_package_path: "./tests/blackbox/helpers/key-package/\
                                            test_secret_key_package.json"
                .to_string(),
            rollup_rpc_endpoint: mock_evm.get_url(),
            log: "debug".to_string(),
            force_stdout: false,
            no_otel: false,
            no_metrics: true,
            metrics_http_listener_addr: String::new(),
            pretty_print: false,
        };
        drop(listener);

        let (metrics, _handle) = metrics::ConfigBuilder::new()
            .set_global_recorder(false)
            .build(&())
            .unwrap();
        let metrics = Box::leak(Box::new(metrics));

        let signer = BridgeSigner::from_config(cfg.clone(), metrics)
            .expect("creating BridgeSigner from test config should succeed");

        // Wait for signer to become ready before connecting the client. TODO: health endpoint?
        sleep(Duration::from_millis(500)).await;
        let frost_client =
            FrostParticipantServiceClient::connect(format!("http://{}", cfg.grpc_endpoint))
                .await
                .expect("connecting to FrostParticipantServiceClient should succeed");

        tokio::spawn(signer.run_until_stopped());
        Self {
            mock_evm,
            frost_client,
        }
    }

    pub(crate) async fn get_verifying_share(&mut self) -> Result<VerifyingShare, Status> {
        let request = Request::new(GetVerifyingShareRequest {});
        self.frost_client
            .get_verifying_share(request)
            .await
            .map(Response::into_inner)
    }

    pub(crate) async fn execute_round_one(&mut self) -> Result<RoundOneResponse, Status> {
        let request = Request::new(ExecuteRoundOneRequest {});
        self.frost_client
            .execute_round_one(request)
            .await
            .map(Response::into_inner)
    }

    pub(crate) async fn execute_round_two(
        &mut self,
        commitments: Vec<CommitmentWithIdentifier>,
        tx_body: TransactionBody,
        request_identifier: u32,
    ) -> Result<RoundTwoResponse, Status> {
        let message = tx_body.into_raw().encode_to_vec().into();
        let request = Request::new(ExecuteRoundTwoRequest {
            commitments,
            message,
            request_identifier,
        });
        self.frost_client
            .execute_round_two(request)
            .await
            .map(Response::into_inner)
    }

    pub(crate) async fn mount_ics20_withdrawal_verification(&mut self, act: &Ics20Withdrawal) {
        self.mock_evm.mount_ics20_withdrawal_verification(act).await;
    }

    pub(crate) async fn mount_bridge_unlock_verification(
        &mut self,
        act: &astria_core::protocol::transaction::v1::action::BridgeUnlock,
    ) {
        self.mock_evm.mount_bridge_unlock_verification(act).await;
    }
}
