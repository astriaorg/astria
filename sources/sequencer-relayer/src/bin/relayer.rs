use bech32::{self, ToBase32, Variant};
use clap::Parser;
use dirs::home_dir;
use serde::Deserialize;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use std::{str::FromStr, time};

use sequencer_relayer::{
    base64_string::Base64String,
    da::CelestiaClient,
    keys::{private_key_bytes_to_keypair, validator_hex_to_address},
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
};

pub const DEFAULT_SEQUENCER_ENDPOINT: &str = "http://localhost:1317";
pub const DEFAULT_CELESTIA_ENDPOINT: &str = "http://localhost:26659";

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Sequencer node RPC endpoint. Default: http://localhost:1317
    #[arg(short, long, default_value = DEFAULT_SEQUENCER_ENDPOINT)]
    sequencer_endpoint: String,

    /// Celestia node RPC endpoint. Default: http://localhost:26659
    #[arg(short, long, default_value = DEFAULT_CELESTIA_ENDPOINT)]
    celestia_endpoint: String,

    /// Expected block time of the sequencer in milliseconds;
    /// ie. how often we should poll the sequencer.
    #[arg(short, long, default_value = "1000")]
    block_time: u64,

    /// Path to validator private key file.
    #[arg(short, long, default_value = ".metro/config/priv_validator_key.json")]
    validator_key_file: String,

    /// Log level. One of debug, info, warn, or error
    #[arg(short, long, default_value = "info")]
    log: String,
}

#[derive(Deserialize)]
pub struct ValidatorPrivateKeyFile {
    pub address: String,
    pub pub_key: KeyWithType,
    pub priv_key: KeyWithType,
}

#[derive(Deserialize)]
pub struct KeyWithType {
    #[serde(rename = "type")]
    pub key_type: String,
    pub value: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(args.log)),
        )
        .init();

    // unmarshal validator private key file
    let home_dir = home_dir().unwrap();
    let file_path = home_dir.join(&args.validator_key_file);
    info!("using validator keys located at {}", file_path.display());

    let key_file =
        std::fs::read_to_string(file_path).expect("failed to read validator private key file");
    let key_file: ValidatorPrivateKeyFile =
        serde_json::from_str(&key_file).expect("failed to unmarshal validator key file");

    // generate our private-public keypair
    let keypair = private_key_bytes_to_keypair(
        &Base64String::from_string(key_file.priv_key.value)
            .expect("failed to decode validator private key; must be base64 string")
            .0,
    )
    .expect("failed to convert validator private key to keypair");

    // generate our bech32 validator address
    let address = validator_hex_to_address(&key_file.address)
        .expect("failed to convert validator address to bech32");

    // generate our validator address bytes
    let address_bytes = hex::decode(&key_file.address)
        .expect("failed to decode validator address; must be hex string");

    let sequencer_client =
        SequencerClient::new(args.sequencer_endpoint).expect("failed to create sequencer client");
    let da_client = CelestiaClient::new(args.celestia_endpoint)
        .expect("failed to create data availability client");

    let sleep_duration = time::Duration::from_millis(args.block_time);
    let mut interval = tokio::time::interval(sleep_duration);
    let mut highest_block_number = 0u64;

    loop {
        interval.tick().await;
        match sequencer_client.get_latest_block().await {
            Ok(resp) => {
                let maybe_height: Result<u64, <u64 as FromStr>::Err> =
                    resp.block.header.height.parse();
                if let Err(e) = maybe_height {
                    warn!(
                        error = ?e,
                        "got invalid block height {} from sequencer",
                        resp.block.header.height,
                    );
                    continue;
                }

                let height = maybe_height.unwrap();
                if height <= highest_block_number {
                    continue;
                }

                info!("got block with height {} from sequencer", height);
                highest_block_number = height;

                if resp.block.header.proposer_address.0 != address_bytes {
                    let proposer_address = bech32::encode(
                        "metrovalcons",
                        resp.block.header.proposer_address.0.to_base32(),
                        Variant::Bech32,
                    )
                    .expect("should encode block proposer address");
                    info!(
                        %proposer_address,
                        validator_address = %address,
                        "ignoring block: proposer address is not ours",
                    );
                    continue;
                }

                let sequencer_block = match SequencerBlock::from_cosmos_block(resp.block) {
                    Ok(block) => block,
                    Err(e) => {
                        warn!(error = ?e, "failed to convert block to DA block");
                        continue;
                    }
                };

                let tx_count =
                    sequencer_block.rollup_txs.len() + sequencer_block.sequencer_txs.len();
                match da_client.submit_block(sequencer_block, &keypair).await {
                    Ok(_) => info!(
                        "submitted block {} to DA layer: tx count={}",
                        height, &tx_count
                    ),
                    Err(e) => warn!(error = ?e, "failed to submit block to DA layer"),
                }
            }
            Err(e) => warn!(error = ?e, "failed to get latest block from sequencer"),
        }
    }
}
