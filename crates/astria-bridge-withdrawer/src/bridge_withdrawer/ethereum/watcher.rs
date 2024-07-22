use std::{
    sync::Arc,
    time::Duration,
};

use astria_bridge_contracts::{
    GetWithdrawalActions,
    GetWithdrawalActionsBuilder,
};
use astria_core::primitive::v1::{
    asset,
    Address,
};
use astria_eyre::{
    eyre::{
        self,
        bail,
        eyre,
        OptionExt as _,
        WrapErr as _,
    },
    Result,
};
use ethers::{
    core::types::Block,
    providers::{
        Middleware,
        Provider,
        ProviderError,
        StreamExt as _,
        Ws,
    },
    types::H256,
    utils::hex,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    info,
    warn,
};

use crate::bridge_withdrawer::{
    batch::Batch,
    startup,
    state::State,
    submitter,
};

pub(crate) struct Builder {
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) startup_handle: startup::InfoHandle,
    pub(crate) ethereum_contract_address: String,
    pub(crate) ethereum_rpc_endpoint: String,
    pub(crate) state: Arc<State>,
    pub(crate) rollup_asset_denom: asset::TracePrefixed,
    pub(crate) bridge_address: Address,
    pub(crate) submitter_handle: submitter::Handle,
}

impl Builder {
    pub(crate) fn build(self) -> Result<Watcher> {
        let Builder {
            ethereum_contract_address,
            ethereum_rpc_endpoint,
            shutdown_token,
            startup_handle,
            state,
            rollup_asset_denom,
            bridge_address,
            submitter_handle,
        } = self;

        let contract_address = address_from_string(&ethereum_contract_address)
            .wrap_err("failed to parse ethereum contract address")?;

        Ok(Watcher {
            contract_address,
            ethereum_rpc_endpoint: ethereum_rpc_endpoint.to_string(),
            rollup_asset_denom,
            bridge_address,
            state,
            shutdown_token: shutdown_token.clone(),
            startup_handle,
            submitter_handle,
        })
    }
}

/// Watches for withdrawal events emitted by the `AstriaWithdrawer` contract.
pub(crate) struct Watcher {
    shutdown_token: CancellationToken,
    startup_handle: startup::InfoHandle,
    submitter_handle: submitter::Handle,
    contract_address: ethers::types::Address,
    ethereum_rpc_endpoint: String,
    rollup_asset_denom: asset::TracePrefixed,
    bridge_address: Address,
    state: Arc<State>,
}

impl Watcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let (provider, action_fetcher, next_rollup_block_height) = self
            .startup()
            .await
            .wrap_err("watcher failed to start up")?;

        let Self {
            state,
            shutdown_token,
            submitter_handle,
            ..
        } = self;

        state.set_watcher_ready();

        tokio::select! {
            res = watch_for_blocks(
                provider,
                action_fetcher,
                next_rollup_block_height,
                submitter_handle,
                shutdown_token.clone(),
            ) => {
                info!("block handler exited");
                res.context("block handler exited")
            }
           () = shutdown_token.cancelled() => {
                info!("watcher shutting down");
                Ok(())
            }
        }
    }

    /// Gets the startup data from the submitter and connects to the Ethereum node.
    ///
    /// Returns the contract handle, the asset ID of the fee asset, the divisor for the asset
    /// withdrawal amount, and the rollup block height to watch from.
    ///
    /// # Errors
    /// - If the fee asset ID provided in the config is not a valid fee asset on the sequencer.
    /// - If the Ethereum node cannot be connected to after several retries.
    /// - If the asset withdrawal decimals cannot be fetched.
    async fn startup(
        &mut self,
    ) -> eyre::Result<(Arc<Provider<Ws>>, GetWithdrawalActions<Provider<Ws>>, u64)> {
        let startup::Info {
            fee_asset,
            starting_rollup_height,
            ..
        } = select! {
            () = self.shutdown_token.cancelled() => {
                return Err(eyre!("watcher received shutdown signal while waiting for startup"));
            }

            startup_info = self.startup_handle.get_info() => {
                startup_info.wrap_err("failed to receive startup info")?
            }
        };

        // connect to eth node
        let retry_config = tryhard::RetryFutureConfig::new(1024)
            .exponential_backoff(Duration::from_millis(500))
            .max_delay(Duration::from_secs(60))
            .on_retry(
                |attempt, next_delay: Option<Duration>, error: &ProviderError| {
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    warn!(
                        attempt,
                        wait_duration,
                        error = error as &dyn std::error::Error,
                        "attempt to connect to rollup node failed; retrying after backoff",
                    );
                    futures::future::ready(())
                },
            );

        let provider = tryhard::retry_fn(|| {
            let url = self.ethereum_rpc_endpoint.clone();
            async move {
                let websocket_client = Ws::connect_with_reconnects(url, 0).await?;
                Ok(Provider::new(websocket_client))
            }
        })
        .with_config(retry_config)
        .await
        .wrap_err("failed connecting to rollup after several retries; giving up")?;

        let provider = Arc::new(provider);
        let ics20_asset_to_withdraw = if self.rollup_asset_denom.last_channel().is_some() {
            info!(
                rollup_asset_denom = %self.rollup_asset_denom,
                "configured rollup asset contains an ics20 channel; ics20 withdrawals will be emitted"
            );
            Some(self.rollup_asset_denom.clone())
        } else {
            info!(
                rollup_asset_denom = %self.rollup_asset_denom,
                "configured rollup asset does not contain an ics20 channel; ics20 withdrawals will not be emitted"
            );
            None
        };
        let action_fetcher = GetWithdrawalActionsBuilder::new()
            .provider(provider.clone())
            .fee_asset(fee_asset)
            .contract_address(self.contract_address)
            .bridge_address(self.bridge_address)
            .sequencer_asset_to_withdraw(self.rollup_asset_denom.clone().into())
            .set_ics20_asset_to_withdraw(ics20_asset_to_withdraw)
            .try_build()
            .await
            .wrap_err("failed to construct contract event to sequencer action fetcher")?;

        self.state.set_watcher_ready();

        Ok((provider.clone(), action_fetcher, starting_rollup_height))
    }
}

async fn sync_from_next_rollup_block_height(
    provider: Arc<Provider<Ws>>,
    action_fetcher: &GetWithdrawalActions<Provider<Ws>>,
    submitter_handle: &submitter::Handle,
    next_rollup_block_height_to_check: u64,
    current_rollup_block_height: u64,
) -> Result<()> {
    if current_rollup_block_height < next_rollup_block_height_to_check {
        return Ok(());
    }

    for i in next_rollup_block_height_to_check..=current_rollup_block_height {
        let Some(block) = provider
            .get_block(i)
            .await
            .wrap_err("failed to get block")?
        else {
            bail!("block with number {i} missing");
        };

        get_and_send_events_at_block(action_fetcher, block, submitter_handle)
            .await
            .wrap_err("failed to get and send events at block")?;
    }

    info!("synced from {next_rollup_block_height_to_check} to {current_rollup_block_height}");
    Ok(())
}

async fn watch_for_blocks(
    provider: Arc<Provider<Ws>>,
    action_fetcher: GetWithdrawalActions<Provider<Ws>>,
    next_rollup_block_height: u64,
    submitter_handle: submitter::Handle,
    shutdown_token: CancellationToken,
) -> Result<()> {
    let mut block_rx = provider
        .subscribe_blocks()
        .await
        .wrap_err("failed to subscribe to blocks")?;

    // read latest block height from subscription;
    // use this value for syncing from the next height to submit to current.
    let Some(current_rollup_block) = block_rx.next().await else {
        bail!("failed to get current rollup block from subscription")
    };

    let Some(current_rollup_block_height) = current_rollup_block.number else {
        bail!("current rollup block missing block number")
    };

    // sync any blocks missing between `next_rollup_block_height` and the current latest
    // (inclusive).
    sync_from_next_rollup_block_height(
        provider.clone(),
        &action_fetcher,
        &submitter_handle,
        next_rollup_block_height,
        current_rollup_block_height.as_u64(),
    )
    .await
    .wrap_err("failed to sync from next rollup block height")?;

    loop {
        select! {
            () = shutdown_token.cancelled() => {
                info!("block watcher shutting down");
                return Ok(());
            }
            block = block_rx.next() => {
                if let Some(block) = block {
                    get_and_send_events_at_block(
                        &action_fetcher,
                        block,
                        &submitter_handle,
                    )
                    .await
                    .wrap_err("failed to get and send events at block")?;
                } else {
                    bail!("block subscription ended")
                }
            }
        }
    }
}

async fn get_and_send_events_at_block(
    actions_fetcher: &GetWithdrawalActions<Provider<Ws>>,
    block: Block<H256>,
    submitter_handle: &submitter::Handle,
) -> Result<()> {
    let block_hash = block.hash.ok_or_eyre("block did not contain a hash")?;
    let rollup_height = block
        .number
        .ok_or_eyre("block did not contain a rollup height")?
        .as_u64();
    let actions = actions_fetcher
        .get_for_block_hash(block_hash)
        .await
        .wrap_err_with(|| {
            format!(
                "failed getting actions for block; block hash: `{block_hash}`, block height: \
                 `{rollup_height}`"
            )
        })?;

    if actions.is_empty() {
        info!(
            "no withdrawal actions found for block `{block_hash}` at rollup height \
             `{rollup_height}; skipping"
        );
    } else {
        submitter_handle
            .send_batch(Batch {
                actions,
                rollup_height,
            })
            .await
            .wrap_err("failed to send batched events; receiver dropped?")?;
    }

    Ok(())
}

// converts an ethereum address string to an `ethers::types::Address`.
// the input string may be prefixed with "0x" or not.
fn address_from_string(s: &str) -> Result<ethers::types::Address> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).wrap_err("failed to parse ethereum address as hex")?;
    let address: [u8; 20] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        eyre!(
            "invalid length for {} ethereum address, must be 20 bytes",
            bytes.len()
        )
    })?;
    Ok(address.into())
}
