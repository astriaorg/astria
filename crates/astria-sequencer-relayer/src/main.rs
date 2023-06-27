use std::{
    net::SocketAddr,
    time,
};

use astria_sequencer_relayer::{
    api,
    config,
    data_availability::CelestiaClientBuilder,
    network::GossipNetwork,
    relayer::Relayer,
    sequencer::SequencerClient,
    writer::Writer,
};
use tracing::{
    info,
    warn,
};

#[tokio::main]
async fn main() {
    let cfg = config::get().expect("failed to read configuration");
    tracing_subscriber::fmt().with_env_filter(&cfg.log).init();
    let cfg_json = serde_json::to_string(&cfg).unwrap_or_else(|e| {
        warn!(
            error = ?e,
            "failed serializing config as json; will use debug formatting"
        );
        format!("{cfg:?}")
    });
    info!(config = cfg_json, "running astria-sequencer-relayer");

    let sequencer_client = SequencerClient::new(cfg.sequencer_endpoint.clone())
        .expect("failed to create sequencer client");
    let da_client = CelestiaClientBuilder::new(cfg.celestia_endpoint.clone())
        .gas_limit(cfg.gas_limit)
        .build()
        .expect("failed to create data availability client");

    let sequencer_interval =
        tokio::time::interval(time::Duration::from_millis(cfg.sequencer_block_time));
    let (block_tx, block_rx) = tokio::sync::watch::channel(None);

    let mut relayer = Relayer::new(cfg.clone(), sequencer_client, sequencer_interval, block_tx)
        .expect("failed to create relayer");

    if cfg.disable_writing {
        relayer.disable_writing();
    }

    let relayer_state = relayer.subscribe_to_state();
    let relayer_handle = relayer.run();

    let _api_server_task = tokio::task::spawn(async move {
        let api_addr = SocketAddr::from(([127, 0, 0, 1], cfg.rpc_port));
        api::start(api_addr, relayer_state).await;
    });

    let writer_interval =
        tokio::time::interval(time::Duration::from_millis(cfg.celestia_block_time));

    if !cfg.disable_writing && !cfg.disable_network {
        let network =
            GossipNetwork::new(cfg.p2p_port, block_rx.clone()).expect("failed to create network");
        let network_handle = network.run();

        let writer = Writer::new(cfg, da_client, writer_interval, block_rx)
            .expect("failed to create writer");
        let writer_handle = writer.run();
        tokio::try_join!(relayer_handle, network_handle, writer_handle)
            .expect("failed to join tasks");
    } else if cfg.disable_writing && !cfg.disable_network {
        let network =
            GossipNetwork::new(cfg.p2p_port, block_rx.clone()).expect("failed to create network");
        let network_handle = network.run();
        tokio::try_join!(relayer_handle, network_handle).expect("failed to join tasks");
    } else if cfg.disable_network && !cfg.disable_writing {
        let writer = Writer::new(cfg, da_client, writer_interval, block_rx)
            .expect("failed to create writer");
        let writer_handle = writer.run();
        tokio::try_join!(relayer_handle, writer_handle).expect("failed to join tasks");
    } else {
        relayer_handle.await.expect("relayer handle error");
    }
}
