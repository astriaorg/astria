use astria_config::astria_config;

/// The single config for creating an astria-sequencer-relayer service.
#[derive(Clone, PartialEq)]
#[astria_config(sequencer_relayer)]
pub struct Config {
    pub sequencer_endpoint: String,
    pub celestia_endpoint: String,
    pub celestia_bearer_token: String,
    pub gas_limit: u64,
    pub block_time: u64,
    pub validator_key_file: String,
    pub rpc_port: u16,
    pub log: String,
}
