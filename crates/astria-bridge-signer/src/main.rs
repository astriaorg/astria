use std::process::ExitCode;

use astria_bridge_signer::{
    Config,
    Server,
    Verifier,
    BUILD_INFO,
};
use astria_core::generated::astria::signer::v1::frost_participant_service_server::FrostParticipantServiceServer;
use astria_eyre::eyre::WrapErr as _;
use futures::TryFutureExt as _;
use tokio::signal::unix::{
    signal,
    SignalKind,
};
use tracing::{
    error,
    info,
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
            eprintln!("initializing bridge signer failed:\n{e:?}");
            return ExitCode::FAILURE;
        }
        Ok(metrics_and_guard) => metrics_and_guard,
    };

    info!(
        config = serde_json::to_string(&cfg).expect("serializing to a string cannot fail"),
        "initializing bridge withdrawer"
    );

    let verifier = match Verifier::new(cfg.rollup_rpc_endpoint) {
        Err(error) => {
            error!(%error, "failed initializing bridge signer verifier");
            return ExitCode::FAILURE;
        }
        Ok(verifier) => verifier,
    };

    let server = match Server::new(cfg.frost_secret_key_package_path, verifier, metrics) {
        Err(error) => {
            error!(%error, "failed initializing bridge signer gRPC server");
            return ExitCode::FAILURE;
        }
        Ok(server) => server,
    };

    let mut sigterm = signal(SignalKind::terminate())
        .expect("setting a SIGTERM listener should always work on Unix");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let grpc_server =
        tonic::transport::Server::builder().add_service(FrostParticipantServiceServer::new(server));

    let grpc_addr: std::net::SocketAddr = match cfg.grpc_endpoint.parse() {
        Err(error) => {
            error!(%error, "failed to parse grpc endpoint");
            return ExitCode::FAILURE;
        }
        Ok(addr) => addr,
    };
    info!(grpc_addr = grpc_addr.to_string(), "starting grpc server");
    tokio::task::spawn(
        grpc_server.serve_with_shutdown(grpc_addr, shutdown_rx.unwrap_or_else(|_| ())),
    );

    tokio::select!(
        _ = sigterm.recv() => {
            info!("received SIGTERM, issuing shutdown to all services");
            let _  = shutdown_tx.send(());
        }
    );

    info!("bridge signer stopped");
    ExitCode::SUCCESS
}
