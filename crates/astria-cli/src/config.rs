use std::{
    collections::HashMap,
    fs,
    path::Path,
};

use color_eyre::eyre;
use serde::{
    Deserialize,
    Serialize,
};
use toml;

pub const DEFAULT_SEQUENCER_URL: &str = "https://rpc.sequencer.dawn-0.astria.org";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkConfig {
    pub sequencer_chain_id: String,
    pub sequencer_url: String,
    pub asset: String,
    pub fee_asset: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SequencerNetworksConfig {
    networks: HashMap<String, NetworkConfig>,
}

impl SequencerNetworksConfig {
    /// Load the config from a file
    pub fn load<P: AsRef<Path>>(path: P) -> eyre::Result<Self> {
        let toml_str = fs::read_to_string(path)?;
        let config: SequencerNetworksConfig = toml::from_str(&toml_str)?;
        return Ok(config);
    }

    /// Get the network config for the selected network
    pub fn get_network(&self, network: &String) -> eyre::Result<&NetworkConfig> {
        if let Some(network_config) = self.networks.get(network) {
            Ok(network_config)
        } else {
            let keys = self.networks.keys().collect::<Vec<&String>>();
            Err(eyre::eyre!(
                "'{}' not found: Expected one of the following: {:?}",
                network,
                keys
            ))
        }
    }
}
