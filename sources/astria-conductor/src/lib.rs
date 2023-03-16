use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Result;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use tokio::{signal, time};

use crate::alert::Alert;
use crate::cli::Cli;
use crate::config::Config;
use crate::driver::{spawn, DriverCommand};

pub(crate) mod alert;
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod driver;
pub(crate) mod execution_client;
pub(crate) mod executor;
pub(crate) mod logger;
pub(crate) mod reader;

pub async fn run() -> Result<()> {
    // logs
    logger::initialize();

    // hierarchical config. cli args override Envars which override toml config values
    let conf: Config = Figment::new()
        .merge(Toml::file("ConductorConfig.toml"))
        .merge(Env::prefixed("ASTRIA_"))
        .merge(Serialized::defaults(Cli::parse()))
        .extract()?;

    log::info!("Using node at {}", conf.celestia_node_url);

    // spawn our driver
    let (mut driver_handle, mut alert_rx) = spawn(conf).await?;

    // NOTE - this will most likely be replaced by an RPC server that will receive gossip
    //  messages from the sequencer
    let mut interval = time::interval(Duration::from_secs(3));

    let mut run = true;
    while run {
        tokio::select! {
            // handle alerts from the driver
            Some(alert) = alert_rx.recv() => {
                match alert {
                    Alert::DriverError(error_string) => {
                        println!("error: {}", error_string);
                        run = false;
                    }
                    Alert::BlockReceived{block_height} => {
                        println!("block received at {}", block_height);
                    }
                }
            }
            // request new blocks every X seconds
            _ = interval.tick() => {
                driver_handle.tx.send(DriverCommand::GetNewBlocks)?;
            }
            // shutdown properly on ctrl-c
            _ = signal::ctrl_c() => {
                driver_handle.shutdown().await?;
            }
        }
        if !run {
            break;
        }
    }

    Ok(())
}
