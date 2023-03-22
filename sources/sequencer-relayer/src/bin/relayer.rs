use bech32::{self, FromBase32, ToBase32, Variant};
use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use std::{str::FromStr, time};

use sequencer_relayer::{
    da::CelestiaClient, sequencer::SequencerClient, sequencer_block::SequencerBlock,
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

    /// Validator address of sequencer node using this relayer.
    /// Address can be found with `metro tendermint show-address`.
    #[arg(short, long)]
    validator_address: String,

    /// Log level. One of debug, info, warn, or error
    #[arg(short, long, default_value = "info")]
    log: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(args.log)),
        )
        .init();

    let sequencer_client =
        SequencerClient::new(args.sequencer_endpoint).expect("failed to create sequencer client");
    let da_client = CelestiaClient::new(args.celestia_endpoint)
        .expect("failed to create data availability client");

    let (hrp, data, variant) = bech32::decode(&args.validator_address)
        .expect("failed to decode validator address; address must be a valid bech32 string");
    assert_eq!(hrp, "metrovalcons", "address must start with metrovalcons");
    assert_eq!(variant, Variant::Bech32, "expected bech32 address");
    let address_bytes = Vec::<u8>::from_base32(&data).unwrap();

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
                        validator_address = %args.validator_address,
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
                match da_client.submit_block(sequencer_block).await {
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
