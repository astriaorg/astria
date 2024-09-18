use std::{
    collections::HashMap,
    fs,
};

use color_eyre::eyre;
use home::home_dir;
use serde::{
    Deserialize,
    Serialize,
};
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct NetworkConfig {
    sequencer_chain_id: String,
    sequencer_url: String,
    asset: String,
    fee_asset: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Config {
    networks: HashMap<String, NetworkConfig>,
}

pub(crate) fn get_networks_config() -> eyre::Result<Config> {
    // try to get the home directory and build the path to the config file
    let mut path = home_dir().expect("Could not determine the home directory.");
    path.push(".astria");
    path.push("sequencer-networks-config.toml");

    // Read the TOML file
    let toml_str = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&toml_str)?;

    for (network_name, network_config) in &config.networks {
        println!("Network: {}", network_name);
        println!(
            "  Sequencer Chain ID: {}",
            network_config.sequencer_chain_id
        );
        println!("  Sequencer URL: {}", network_config.sequencer_url);
        println!("  Asset: {}", network_config.asset);
        println!("  Fee Asset: {}", network_config.fee_asset);
        println!();
    }

    Ok(config)
}
