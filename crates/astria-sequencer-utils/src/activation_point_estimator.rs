#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::arithmetic_side_effects,
    reason = "casts between f64 and u64 will involve values where these lints are not a problem"
)]

use std::time::Duration;

use astria_eyre::eyre::{
    bail,
    ensure,
    eyre,
    Result,
    WrapErr as _,
};
use jiff::{
    Span,
    SpanTotal,
    Timestamp,
    Unit,
};
use serde_json::Value;

const TIMEOUT_DURATION: Duration = Duration::from_secs(5);

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The URL of the Sequencer node
    #[arg(long, short = 'u', value_name = "URL")]
    sequencer_url: String,

    #[command(flatten)]
    action: Action,

    /// The number of blocks to use to estimate a mean block time [minimum: 1]
    #[arg(
        long,
        short = 's',
        default_value = "43200",
        value_name = "INTEGER",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    sample_size: u64,

    /// Print verbose output
    #[arg(long, short = 'v')]
    verbose: bool,
}

#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
pub struct Action {
    /// Estimate an activation point (block height) for the given duration in the future. Enter as
    /// e.g. "3d 4h 5m"
    #[arg(
        long,
        short = 'd',
        value_name = "DURATION",
        value_parser = clap::value_parser!(Span)
    )]
    desired_duration: Option<Span>,

    /// Estimate an activation point (block height) for the given instant. Enter as e.g.
    /// "2025-08-17 16:00:00Z" where the `Z` suffix denotes UTC, or the same instant with a -5 hour
    /// offset from UTC is "2025-08-17 11:00:00-05:00"
    #[arg(
        long,
        short = 'i',
        value_name = "TIMESTAMP",
        value_parser = clap::value_parser!(Timestamp)
    )]
    desired_instant: Option<Timestamp>,

    /// Predict block time for given height
    #[arg(long, short = 't', value_name = "BLOCK HEIGHT")]
    predict_block_time: Option<u64>,
}

impl Args {
    async fn get_current_height(&self) -> Result<(u64, Timestamp)> {
        let blocking_getter = async {
            reqwest::get(format!("{}/block", self.sequencer_url))
                .await
                .wrap_err("failed to get latest block")?
                .text()
                .await
                .wrap_err("failed to parse block response as UTF-8 string")
        };

        let response = tokio::time::timeout(TIMEOUT_DURATION, blocking_getter)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to fetch block within {}s",
                    TIMEOUT_DURATION.as_secs_f32()
                )
            })??
            .trim()
            .to_string();
        let json_rpc_response: Value = serde_json::from_str(&response)
            .wrap_err_with(|| format!("failed to parse block response `{response}` as json"))?;
        let header = json_rpc_response
            .get("result")
            .and_then(|value| value.get("block"))
            .and_then(|value| value.get("header"))
            .ok_or_else(|| {
                eyre!("expected block response `{response}` to have field `result.block.header`")
            })?;
        let height_str = header
            .get("height")
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("expected header `{header}` to have string field `height`"))?;
        let height: u64 = height_str
            .parse()
            .wrap_err_with(|| format!("expected height `{height_str}` to convert to `u64`"))?;
        let time_str = header
            .get("time")
            .and_then(Value::as_str)
            .ok_or_else(|| eyre!("expected header `{header}` to have string field `time`"))?;
        let timestamp: Timestamp = time_str
            .parse()
            .wrap_err_with(|| format!("expected time `{time_str}` to convert to `Timestamp`"))?;

        Ok((height, timestamp))
    }

    async fn get_timestamp_at_height(&self, height: u64) -> Result<Timestamp> {
        let blocking_getter = async {
            reqwest::get(format!("{}/block?height={height}", self.sequencer_url))
                .await
                .wrap_err_with(|| format!("failed to get block at height {height}"))?
                .text()
                .await
                .wrap_err("failed to parse block response as UTF-8 string")
        };

        let response = tokio::time::timeout(TIMEOUT_DURATION, blocking_getter)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to fetch block at height {height} within {}s",
                    TIMEOUT_DURATION.as_secs_f32()
                )
            })??
            .trim()
            .to_string();
        let json_rpc_response: Value = serde_json::from_str(&response)
            .wrap_err_with(|| format!("failed to parse block response `{response}` as json"))?;
        let time_str = json_rpc_response
            .get("result")
            .and_then(|value| value.get("block"))
            .and_then(|value| value.get("header"))
            .and_then(|value| value.get("time"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                eyre!(
                    "expected block response `{response}` to have string field \
                     `result.block.header.time`"
                )
            })?;
        time_str
            .parse()
            .wrap_err_with(|| format!("expected time `{time_str}` to convert to `Timestamp`"))
    }

    async fn get_network_name(&self) -> Result<String> {
        let blocking_getter = async {
            reqwest::get(format!("{}/status", self.sequencer_url))
                .await
                .wrap_err("failed to get status")?
                .text()
                .await
                .wrap_err("failed to parse status response as UTF-8 string")
        };

        let response = tokio::time::timeout(TIMEOUT_DURATION, blocking_getter)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to fetch status within {}s",
                    TIMEOUT_DURATION.as_secs_f32()
                )
            })??
            .trim()
            .to_string();
        let json_rpc_response: Value = serde_json::from_str(&response)
            .wrap_err_with(|| format!("failed to parse status response `{response}` as json"))?;
        Ok(json_rpc_response
            .get("result")
            .and_then(|value| value.get("node_info"))
            .and_then(|value| value.get("network"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                eyre!(
                    "expected status response `{response}` to have string field \
                     `result.node_info.network`"
                )
            })?
            .to_string())
    }
}

struct Estimator {
    current_height: u64,
    current_timestamp: Timestamp,
    estimated_nanoseconds_per_block: f64,
    network_name: String,
    verbose: bool,
}

impl Estimator {
    async fn new(args: &Args) -> Result<Self> {
        let (current_height, current_timestamp) = args.get_current_height().await?;
        ensure!(
            current_height > 1,
            "need current height to be greater than 1"
        );
        let sample_size = std::cmp::min(args.sample_size, current_height.saturating_sub(1));
        let old_timestamp = args
            .get_timestamp_at_height(current_height - sample_size)
            .await?;
        let network_name = args.get_network_name().await?;
        if args.verbose {
            println!("current height on `{network_name}`: {current_height}");
        }
        let time_diff_nanoseconds = (current_timestamp - old_timestamp)
            .total(Unit::Nanosecond)
            .wrap_err("failed to get time difference total nanoseconds")?;
        let estimated_nanoseconds_per_block = time_diff_nanoseconds / sample_size as f64;

        Ok(Self {
            current_height,
            current_timestamp,
            estimated_nanoseconds_per_block,
            network_name,
            verbose: args.verbose,
        })
    }

    fn estimate_height(&self, desired_duration: Span) -> Result<()> {
        ensure!(
            desired_duration.is_positive(),
            "desired activation is earlier than current block time - good luck with the time \
             travel"
        );
        let duration_nanoseconds = desired_duration
            .total(SpanTotal::from(Unit::Nanosecond).days_are_24_hours())
            .wrap_err("failed to get duration total nanoseconds")?;

        let estimated_height_diff =
            (duration_nanoseconds / self.estimated_nanoseconds_per_block).ceil() as u64;
        let estimated_height = self.current_height + estimated_height_diff;
        if self.verbose {
            println!("estimated height difference: {estimated_height_diff}");
        }

        if self.verbose {
            println!(
                "estimated activation instant on `{}`: {}",
                self.network_name,
                self.current_timestamp + Duration::from_nanos(duration_nanoseconds as u64)
            );
            colour::dark_green!("estimated activation height on `{}`: ", self.network_name);
            colour::green_ln_bold!("{estimated_height}");
        } else {
            print!("{estimated_height}");
        }
        Ok(())
    }

    fn predict_block_time(&self, future_height: u64) -> Result<()> {
        ensure!(
            future_height > self.current_height,
            "given height is earlier than current block height - no prediction required"
        );
        let height_diff = (future_height - self.current_height) as f64;
        let estimated_instant = self.current_timestamp
            + Duration::from_nanos((self.estimated_nanoseconds_per_block * height_diff) as u64);

        if self.verbose {
            colour::dark_green!(
                "estimated instant on `{}` for block {future_height}: ",
                self.network_name
            );
            colour::green_ln_bold!("{estimated_instant}");
        } else {
            print!("{estimated_instant}");
        }
        Ok(())
    }
}

/// Estimates the activation height.
///
/// # Errors
///
/// Returns an error if bad stuff happens.
pub async fn run(mut args: Args) -> Result<()> {
    args.sequencer_url = args.sequencer_url.trim_end_matches('/').to_string();
    let estimator = Estimator::new(&args).await?;

    if let Some(desired_duration) = args.action.desired_duration {
        estimator.estimate_height(desired_duration)
    } else if let Some(desired_instant) = args.action.desired_instant {
        estimator.estimate_height(desired_instant - estimator.current_timestamp)
    } else if let Some(future_height) = args.action.predict_block_time {
        estimator.predict_block_time(future_height)
    } else {
        bail!("clap should guarantee exactly one of these args is `Some`")
    }
}
