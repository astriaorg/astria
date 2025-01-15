use std::time::Duration;

use astria_core::upgrades::v1::Upgrades;
use astria_eyre::{
    anyhow_to_eyre,
    eyre,
    eyre::{
        bail,
        eyre,
        OptionExt,
        Result,
        WrapErr,
    },
};
use cnidarium::Snapshot;
use isahc::AsyncReadResponseExt as _;
use serde_json::Value;
use tracing::warn;

use crate::app::{
    ShouldShutDown,
    StateReadExt as _,
};

mod state_ext;
pub(crate) mod storage;
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

/// Returns `ShouldShutDown::ShutDownForUpgrade` if any scheduled upgrade is due to be applied
/// during the next block and this binary is not hard-coded to apply that upgrade.  Otherwise,
/// returns `ShouldShutDown::ContinueRunning`.
pub(crate) async fn should_shut_down(
    upgrades: &Upgrades,
    snapshot: &Snapshot,
) -> Result<ShouldShutDown> {
    let next_block_height = next_block_height(snapshot).await?;

    for upgrade in upgrades.iter() {
        // The upgrades are ordered from lowest activation height to highest, so once we find an
        // upgrade with an activation height > `next_block_height`, we can stop iterating as all
        // subsequent upgrades have activation heights > `next_block_height`.
        if upgrade.activation_height() < next_block_height {
            continue;
        }
        if upgrade.activation_height() > next_block_height {
            return Ok(ShouldShutDown::ContinueRunning);
        }

        if !upgrade.shutdown_required() {
            continue;
        }

        let block_time = snapshot
            .get_block_timestamp()
            .await
            .wrap_err("failed getting latest block time from snapshot")?;
        let app_hash = snapshot
            .root_hash()
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get current root hash from snapshot")?;
        let hex_encoded_app_hash = hex::encode(app_hash.0);

        return Ok(ShouldShutDown::ShutDownForUpgrade {
            upgrade_activation_height: upgrade.activation_height(),
            block_time,
            hex_encoded_app_hash,
        });
    }

    Ok(ShouldShutDown::ContinueRunning)
}

/// Verifies that all historical upgrades have been applied.
///
/// Returns an error if any has not been applied.
pub(crate) async fn ensure_historical_upgrades_applied(
    upgrades: &Upgrades,
    snapshot: &Snapshot,
) -> Result<()> {
    let next_block_height = next_block_height(snapshot).await?;

    for upgrade in upgrades.iter() {
        // The upgrades are ordered from lowest activation height to highest, so once we find an
        // upgrade with an activation height >= `next_block_height`, we can stop iterating as all
        // subsequent upgrades have activation heights > `next_block_height`.  In the case where
        // activation height == `next_block_height`, the upgrade is not a historical one and is not
        // expected to have been applied at the point of calling this function.
        if upgrade.activation_height() >= next_block_height {
            return Ok(());
        }

        let upgrade_name = upgrade.name();
        for change in upgrade.changes() {
            let change_name = change.name();
            let Some(stored_change_info) = snapshot
                .get_upgrade_change_info(&upgrade_name, &change_name)
                .await
                .wrap_err("failed to get upgrade change hash")?
            else {
                bail!(
                    "historical upgrade change `{upgrade_name}/{change_name}` has not been \
                     applied (wrong upgrade.json file provided?)",
                );
            };
            let actual_info = change.info();
            if actual_info != stored_change_info {
                bail!(
                    "upgrade change `{actual_info}` does not match stored info \
                     `{stored_change_info}` for `{upgrade_name}/{change_name}`",
                );
            }
        }
    }
    Ok(())
}

async fn next_block_height(snapshot: &Snapshot) -> Result<u64> {
    snapshot
        .get_block_height()
        .await
        .unwrap_or_default()
        .checked_add(1)
        .ok_or_eyre("overflowed getting next block height")
}

/// Returns the CometBFT consensus params by querying the given CometBFT endpoint.
///
/// We need to specify the current block height as a query arg since otherwise CometBFT tries to use
/// its view of current block, and since FinalizeBlock has not yet been called, this results in an
/// error response.
#[expect(clippy::doc_markdown, reason = "false positive")]
pub(crate) async fn get_consensus_params_from_cometbft(
    cometbft_addr: &str,
    block_height: u64,
) -> Result<tendermint::consensus::Params> {
    if cfg!(test) {
        bail!(
            "cannot query cometbft in tests; consensus params should be available to `App` as \
             they are provided in `App::init_chain`"
        );
    }

    let uri = format!("{cometbft_addr}/consensus_params?height={block_height}");

    let max_retries = 66;
    let retry_config = tryhard::RetryFutureConfig::new(max_retries)
        .exponential_backoff(Duration::from_millis(10))
        .max_delay(Duration::from_secs(1))
        .on_retry(
            |attempt, next_delay: Option<Duration>, error: &eyre::Report| {
                let wait_duration = next_delay
                    .map(humantime::format_duration)
                    .map(tracing::field::display);
                warn!(
                    error = error.as_ref() as &dyn std::error::Error,
                    attempt,
                    wait_duration,
                    "failed to get consensus params from cometbft; retrying after backoff",
                );
                async {}
            },
        );

    tryhard::retry_fn(|| try_get_consensus_params_from_cometbft(&uri))
        .with_config(retry_config)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to get consensus params from {uri} after {max_retries} retries; giving up"
            )
        })
}

async fn try_get_consensus_params_from_cometbft(
    uri: &str,
) -> Result<tendermint::consensus::Params> {
    let blocking_getter = async {
        isahc::get_async(uri)
            .await
            .wrap_err("failed to get consensus params")?
            .text()
            .await
            .wrap_err("failed to parse consensus params response as UTF-8 string")
    };

    let response = tokio::time::timeout(Duration::from_secs(1), blocking_getter)
        .await
        .wrap_err("timed out fetching consensus params")??
        .trim()
        .to_string();
    let json_rpc_response: Value = serde_json::from_str(&response).wrap_err_with(|| {
        format!("failed to parse consensus params response `{response}` as json")
    })?;
    let json_params = json_rpc_response
        .get("result")
        .and_then(|result| result.get("consensus_params"))
        .ok_or_else(|| eyre!("missing `result` in consensus params response `{response}`"))?
        .clone();
    serde_json::from_value::<tendermint::consensus::Params>(json_params).wrap_err_with(|| {
        format!(
            "failed to parse `result.consensus_params` as consensus params in response \
             `{response}`"
        )
    })
}
