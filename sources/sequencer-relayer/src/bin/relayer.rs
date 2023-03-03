use structopt::StructOpt;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use std::{str::FromStr, thread, time};

use sequencer_relayer::{
    da::{CelestiaClient, DataAvailabilityClient},
    sequencer::SequencerClient,
};

pub const DEFAULT_SEQUENCER_ENDPOINT: &str = "http://localhost:1317";
pub const DEFAULT_CELESTIA_ENDPOINT: &str = "http://localhost:26659";

#[derive(StructOpt)]
struct Options {
    /// Sequencer node RPC endpoint. Default: http://localhost:1317
    #[structopt(short, long, default_value = DEFAULT_SEQUENCER_ENDPOINT)]
    sequencer_endpoint: String,

    /// Celestia node RPC endpoint. Default: http://localhost:26659
    #[structopt(short, long, default_value = DEFAULT_CELESTIA_ENDPOINT)]
    celestia_endpoint: String,

    /// Expected block time of the sequencer in milliseconds;
    /// ie. how often we should poll the sequencer.
    #[structopt(short, long, default_value = "1000")]
    block_time: u64,

    /// Log level. One of debug, info, warn, or error
    #[structopt(short, long, default_value = "info")]
    log: String,
}

#[tokio::main]
async fn main() {
    let options: Options = Options::from_args();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(options.log)),
        )
        .init();

    let sequencer_client = SequencerClient::new(options.sequencer_endpoint)
        .expect("failed to create sequencer client");
    let da_client = CelestiaClient::new(options.celestia_endpoint)
        .expect("failed to create data availability client");

    let sleep_duration = time::Duration::from_millis(options.block_time);
    let mut highest_block_number = 0u64;

    loop {
        match sequencer_client.get_latest_block().await {
            Ok(resp) => {
                let maybe_height: Result<u64, <u64 as FromStr>::Err> =
                    resp.block.header.height.parse();
                if let Err(e) = maybe_height {
                    warn!(
                        "got invalid block height {} from sequencer: {}",
                        resp.block.header.height, e
                    );
                    thread::sleep(sleep_duration);
                    continue;
                }

                let height = maybe_height.unwrap();
                if height <= highest_block_number {
                    thread::sleep(sleep_duration);
                    continue;
                }

                info!("got block with height {} from sequencer", height);
                highest_block_number = height;
                let tx_count = resp.block.data.txs.len();
                match da_client.submit_block(resp.block.into()).await {
                    Ok(_) => info!(
                        "submitted block {} to DA layer: tx count={}",
                        height, &tx_count
                    ),
                    Err(e) => warn!("failed to submit block to DA layer: {:?}", e),
                }
            }
            Err(e) => warn!("failed to get latest block from sequencer: {:?}", e),
        }

        thread::sleep(sleep_duration);
    }
}
