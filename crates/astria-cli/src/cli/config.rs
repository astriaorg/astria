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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub sequencer_chain_id: String,
    pub sequencer_url: String,
    pub asset: String,
    pub fee_asset: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            sequencer_chain_id: "astria-dusk-10".to_string(),
            sequencer_url: "https://rpc.sequencer.dusk-10.devnet.astria.org".to_string(),
            asset: "TIA".to_string(),
            fee_asset: "TIA".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Config {
    networks: HashMap<String, NetworkConfig>,
}
impl Config {
    /// Validate if the parsed input for choosing a network config exists and is valid
    pub(crate) fn validate_network(&self, network: String) -> bool {
        for (network_name, _) in &self.networks {
            if *network_name == network {
                return true;
            }
        }
        false
    }

    /// Get a list of valid network names from the config
    pub(crate) fn get_valid_networks(&self) -> Vec<String> {
        let mut valid_networks = Vec::new();
        for (network_name, _) in &self.networks {
            valid_networks.push(network_name.clone());
        }
        valid_networks
    }

    /// Get the network config for the selected network
    pub(crate) fn get_network(&self, network: String) -> Option<&NetworkConfig> {
        self.networks.get(&network)
    }
}

pub(crate) fn get_networks_config() -> eyre::Result<Config> {
    // try to get the home directory and build the path to the config file
    let mut path = home_dir().expect("Could not determine the home directory.");
    path.push(".astria");
    path.push("sequencer-networks-config.toml");

    // Read the TOML file
    let toml_str = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&toml_str)?;

    // for (network_name, network_config) in &config.networks {
    //     println!("Network: {}", network_name);
    //     println!(
    //         "  Sequencer Chain ID: {}",
    //         network_config.sequencer_chain_id
    //     );
    //     println!("  Sequencer URL: {}", network_config.sequencer_url);
    //     println!("  Asset: {}", network_config.asset);
    //     println!("  Fee Asset: {}", network_config.fee_asset);
    //     println!();
    // }

    Ok(config)
}
