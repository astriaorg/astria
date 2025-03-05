use std::{
    collections::HashMap,
    process::ExitCode,
};

use astria_bridge_withdrawer::{
    BridgeWithdrawer,
    Config,
    BUILD_INFO,
};
use astria_core::generated::astria::signer::v1::{
    frost_participant_service_client::FrostParticipantServiceClient,
    GetVerifyingShareRequest,
};
use astria_eyre::{
    eyre,
    eyre::{
        ensure,
        eyre,
        WrapErr as _,
    },
};
use frost_ed25519::{
    keys::{
        PublicKeyPackage,
        VerifyingShare,
    },
    Identifier,
};
use tokio::signal::unix::{
    signal,
    SignalKind,
};
use tracing::{
    error,
    info,
    warn,
};

#[tokio::main]
async fn main() -> ExitCode {
    astria_eyre::install().expect("astria eyre hook must be the first hook installed");

    eprintln!("{}", telemetry::display::json(&BUILD_INFO));

    let cfg: Config = config::get().expect("failed to read configuration");
    eprintln!("{}", telemetry::display::json(&cfg),);

    let mut telemetry_conf = telemetry::configure()
        .set_no_otel(cfg.no_otel)
        .set_force_stdout(cfg.force_stdout)
        .set_pretty_print(cfg.pretty_print)
        .set_filter_directives(&cfg.log);

    if !cfg.no_metrics {
        telemetry_conf =
            telemetry_conf.set_metrics(&cfg.metrics_http_listener_addr, env!("CARGO_PKG_NAME"));
    }

    let (metrics, _telemetry_guard) = match telemetry_conf
        .try_init(&())
        .wrap_err("failed to setup telemetry")
    {
        Err(e) => {
            eprintln!("initializing bridge withdrawer failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing bridge withdrawer"
    );

    let (frost_participant_clients, frost_public_key_package) = if cfg.no_frost_threshold_signing {
        (None, None)
    } else {
        let public_key_package = match read_frost_key(&cfg.frost_public_key_package_path)
            .wrap_err_with(|| {
                format!(
                    "failed reading frost public key package from file `{}`",
                    cfg.frost_public_key_package_path
                )
            }) {
            Err(error) => {
                error!(%error, "failed to read frost public key package");
                return ExitCode::FAILURE;
            }
            Ok(key) => key,
        };

        let frost_participant_endpoints = cfg
            .frost_participant_endpoints
            .split(',')
            .map(str::to_string)
            .collect();
        let participant_clients = match initialize_frost_participant_clients(
            frost_participant_endpoints,
            &public_key_package,
        )
        .await
        {
            Err(error) => {
                error!(%error, "failed to initialize frost participant clients");
                return ExitCode::FAILURE;
            }
            Ok(clients) => clients,
        };
        (Some(participant_clients), Some(public_key_package))
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");
    let (withdrawer, shutdown_handle) = match BridgeWithdrawer::new(
        cfg,
        metrics,
        frost_participant_clients,
        frost_public_key_package,
    ) {
        Err(error) => {
            error!(%error, "failed initializing bridge withdrawer");
            return ExitCode::FAILURE;
        }
        Ok(handles) => handles,
    };
    let withdrawer_handle = tokio::spawn(withdrawer.run());

    let shutdown_token = shutdown_handle.token();
    tokio::select!(
        _ = sigterm.recv() => {
            // We don't care about the result (i.e. whether there could be more SIGTERM signals
            // incoming); we just want to shut down as soon as we receive the first `SIGTERM`.
            info!("received SIGTERM, issuing shutdown to all services");
            shutdown_handle.shutdown();
        }
        () = shutdown_token.cancelled() => {
            warn!("stopped waiting for SIGTERM");
        }
    );

    if let Err(error) = withdrawer_handle.await {
        error!(%error, "failed to join main withdrawer task");
    }

    info!("withdrawer stopped");
    ExitCode::SUCCESS
}

fn read_frost_key<P: AsRef<std::path::Path>>(
    path: P,
) -> astria_eyre::eyre::Result<PublicKeyPackage> {
    let key_str =
        std::fs::read_to_string(path).wrap_err("failed to read frost public key package")?;
    serde_json::from_str::<PublicKeyPackage>(&key_str)
        .wrap_err("failed to deserialize public key package")
}

async fn initialize_frost_participant_clients(
    endpoints: Vec<String>,
    public_key_package: &PublicKeyPackage,
) -> eyre::Result<HashMap<Identifier, FrostParticipantServiceClient<tonic::transport::Channel>>> {
    let mut participant_clients = HashMap::new();
    for endpoint in endpoints {
        let mut client = FrostParticipantServiceClient::connect(endpoint)
            .await
            .wrap_err("failed to connect to participant")?;
        let resp = client
            .get_verifying_share(GetVerifyingShareRequest {})
            .await
            .wrap_err("failed to get verifying share")?;
        let verifying_share = VerifyingShare::deserialize(&resp.into_inner().verifying_share)
            .wrap_err("failed to deserialize verifying share")?;
        let identifier = public_key_package
            .verifying_shares()
            .iter()
            .find(|(_, vs)| vs == &&verifying_share)
            .map(|(id, _)| id)
            .ok_or_else(|| eyre!("failed to find identifier for verifying share"))?;
        participant_clients.insert(identifier.to_owned(), client);
    }

    ensure!(
        participant_clients.len() == public_key_package.verifying_shares().len(),
        "failed to initialize all participant clients; are there duplicate endpoints?"
    );

    Ok(participant_clients)
}
