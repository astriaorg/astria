use std::{
    path::Path,
    time::Duration,
};

use astria_core::upgrades::v1::{
    ChangeHash,
    Upgrade,
    Upgrades,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        self,
        bail,
        eyre,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use cnidarium::{
    Snapshot,
    StateWrite,
};
use isahc::AsyncReadResponseExt;
use serde_json::Value;
use tendermint::consensus::params::VersionParams;
use tracing::{
    error,
    info,
    warn,
};

use super::{
    StateReadExt as _,
    StateWriteExt as _,
};
use crate::{
    app::{
        ShouldShutDown,
        StateReadExt as _,
        StateWriteExt,
    },
    oracles::price_feed::{
        market_map::component::MarketMapComponent,
        oracle::component::OracleComponent,
    },
};

pub(crate) struct UpgradesHandler {
    upgrades: Upgrades,
    cometbft_rpc_addr: String,
}

impl UpgradesHandler {
    pub(crate) fn new<P: AsRef<Path>>(
        upgrades_filepath: P,
        cometbft_rpc_addr: String,
    ) -> Result<Self> {
        let upgrades = Upgrades::new(upgrades_filepath).wrap_err("failed constructing upgrades")?;
        Ok(Self {
            upgrades,
            cometbft_rpc_addr,
        })
    }

    pub(crate) fn upgrades(&self) -> &Upgrades {
        &self.upgrades
    }

    /// Verifies that all historical upgrades have been applied.
    ///
    /// Returns an error if any has not been applied.
    pub(crate) async fn ensure_historical_upgrades_applied(
        &self,
        snapshot: &Snapshot,
    ) -> Result<()> {
        let next_block_height = next_block_height(snapshot).await?;

        for upgrade in self.upgrades.iter() {
            // The upgrades are ordered from lowest activation height to highest, so once we find
            // an upgrade with an activation height >= `next_block_height`, we can stop iterating
            // as all subsequent upgrades have activation heights > `next_block_height`.  In the
            // case where activation height == `next_block_height`, the upgrade is not a historical
            // one and is not expected to have been applied at the point of calling this function.
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

    /// Returns `ShouldShutDown::ShutDownForUpgrade` if any scheduled upgrade is due to be applied
    /// during the next block and this binary is not hard-coded to apply that upgrade.  Otherwise,
    /// returns `ShouldShutDown::ContinueRunning`.
    pub(crate) async fn should_shut_down(&self, snapshot: &Snapshot) -> Result<ShouldShutDown> {
        let next_block_height = next_block_height(snapshot).await?;

        for upgrade in self.upgrades.iter() {
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

    /// Execute any changes to global state required as part of any upgrade with an activation
    /// height == `block_height`.
    ///
    /// At a minimum, the `info` of each `Change` in such an upgrade must be written to verifiable
    /// storage.
    ///
    /// Returns an empty `Vec` if no upgrade was executed.
    pub(crate) fn execute_upgrade_if_due<S: StateWrite>(
        &mut self,
        mut state: S,
        block_height: tendermint::block::Height,
    ) -> Result<Vec<ChangeHash>> {
        let Some(upgrade) = self
            .upgrades
            .upgrade_activating_at_height(block_height.value())
        else {
            return Ok(vec![]);
        };
        let upgrade_name = upgrade.name();
        let mut change_hashes = vec![];
        for change in upgrade.changes() {
            change_hashes.push(change.calculate_hash());
            state
                .put_upgrade_change_info(&upgrade_name, change)
                .wrap_err("failed to put upgrade change info")?;
            info!(upgrade = %upgrade_name, change = %change.name(), "executed upgrade change");
        }

        // NOTE: any further state changes specific to individual upgrades should be
        //       executed here after matching on the upgrade variant.

        #[expect(
            irrefutable_let_patterns,
            reason = "will become refutable once we have more than one upgrade variant"
        )]
        if let Upgrade::Upgrade1(upgrade_1) = upgrade {
            let genesis_state = upgrade_1.price_feed_change().genesis();
            MarketMapComponent::handle_genesis(&mut state, genesis_state.market_map())
                .wrap_err("failed to handle market map genesis")?;
            info!("handled market map genesis");
            OracleComponent::handle_genesis(state, genesis_state.oracle())
                .wrap_err("failed to handle oracle genesis")?;
            info!("handled oracle genesis");
        }

        Ok(change_hashes)
    }

    /// Updates CometBFT consensus params as required by any upgrade with an activation height ==
    /// `block_height`.
    ///
    /// Returns `None` if no upgrade is due.
    ///
    /// If an upgrade is due, at a minimum, the ABCI application version should be increased.
    pub(crate) async fn end_block<S: StateWriteExt>(
        &self,
        mut state: S,
        block_height: tendermint::block::Height,
    ) -> Result<Option<tendermint::consensus::Params>> {
        let Some(upgrade) = self
            .upgrades
            .upgrade_activating_at_height(block_height.value())
        else {
            return Ok(None);
        };

        let mut params = match state
            .get_consensus_params()
            .await
            .wrap_err("failed to get consensus params from storage")?
        {
            Some(value) => value,
            None => self
                .get_consensus_params_from_cometbft(block_height.value())
                .await
                .wrap_err("failed to get consensus params from cometbft")?,
        };

        let new_app_version = upgrade.app_version();
        if let Some(existing_app_version) = &params.version {
            if new_app_version <= existing_app_version.app {
                error!(
                    "new app version {new_app_version} should be greater than existing version {}",
                    existing_app_version.app
                );
            }
        }
        params.version = Some(VersionParams {
            app: new_app_version,
        });

        // NOTE: any further changes specific to individual upgrades should be applied here after
        //       matching on the upgrade variant.

        #[expect(
            irrefutable_let_patterns,
            reason = "will become refutable once we have more than one upgrade variant"
        )]
        if let Upgrade::Upgrade1(_) = upgrade {
            set_vote_extensions_enable_height_to_next_block_height(block_height, &mut params);
        }

        state
            .put_consensus_params(params.clone())
            .wrap_err("failed to put consensus params to storage")?;

        Ok(Some(params))
    }

    /// Returns the CometBFT consensus params by querying the given CometBFT endpoint.
    ///
    /// We need to specify the current block height as a query arg since otherwise CometBFT tries to
    /// use its view of current block, and since `FinalizeBlock` has not yet been called, this
    /// results in an error response.
    async fn get_consensus_params_from_cometbft(
        &self,
        block_height: u64,
    ) -> Result<tendermint::consensus::Params> {
        if cfg!(test) {
            bail!(
                "cannot query cometbft in tests; consensus params should be available to `App` as \
                 they are provided in `App::init_chain`"
            );
        }

        let uri = format!(
            "{}/consensus_params?height={block_height}",
            self.cometbft_rpc_addr
        );

        let max_retries = 66;
        let retry_config = tryhard::RetryFutureConfig::new(max_retries)
            .exponential_backoff(Duration::from_millis(10))
            .max_delay(Duration::from_secs(1))
            .on_retry(
                |attempt, next_delay: Option<Duration>, error: &eyre::Report| {
                    let wait_duration = next_delay
                        .map(telemetry::display::format_duration)
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
                    "failed to get consensus params from {uri} after {max_retries} retries; \
                     giving up"
                )
            })
    }
}

#[cfg(any(test, feature = "benchmark"))]
impl From<Upgrades> for UpgradesHandler {
    fn from(upgrades: Upgrades) -> Self {
        Self {
            upgrades,
            cometbft_rpc_addr: String::new(),
        }
    }
}

async fn next_block_height(snapshot: &Snapshot) -> Result<u64> {
    snapshot
        .get_block_height()
        .await
        .unwrap_or_default()
        .checked_add(1)
        .ok_or_eyre("overflowed getting next block height")
}

fn set_vote_extensions_enable_height_to_next_block_height(
    current_block_height: tendermint::block::Height,
    consensus_params: &mut tendermint::consensus::Params,
) {
    // Set the vote_extensions_enable_height as the next block height (it must be a future
    // height to be valid).
    let new_enable_height = current_block_height.increment();
    if let Some(existing_enable_height) = consensus_params.abci.vote_extensions_enable_height {
        // If vote extensions are already enabled, they cannot be disabled, and the
        // `vote_extensions_enable_height` cannot be changed.
        if existing_enable_height.value() != 0 {
            error!(
                %existing_enable_height, %new_enable_height,
                "vote extensions enable height already set; ignoring update",
            );
            return;
        }
    }
    consensus_params.abci.vote_extensions_enable_height = Some(new_enable_height);
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
