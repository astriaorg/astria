use std::{
    sync::Arc,
    time::Duration,
};

use astria_bridge_contracts::{
    GetWithdrawalActions,
    GetWithdrawalActionsBuilder,
};
use astria_core::{
    primitive::v1::{
        asset,
        Address,
    },
    protocol::transaction::v1::Action,
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
    debug,
    info,
    info_span,
    instrument,
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
    pub(crate) use_compat_address: bool,
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
            use_compat_address,
            submitter_handle,
        } = self;

        let contract_address = address_from_string(&ethereum_contract_address)
            .wrap_err("failed to parse ethereum contract address")?;

        Ok(Watcher {
            contract_address,
            ethereum_rpc_endpoint: ethereum_rpc_endpoint.to_string(),
            rollup_asset_denom,
            bridge_address,
            use_compat_address,
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
    use_compat_address: bool,
    state: Arc<State>,
}

struct FullyInitialized {
    shutdown_token: CancellationToken,
    submitter_handle: submitter::Handle,
    state: Arc<State>,
    provider: Arc<Provider<Ws>>,
    action_fetcher: GetWithdrawalActions<Provider<Ws>>,
    starting_rollup_height: u64,
}

impl Watcher {
    pub(crate) async fn run(self) -> Result<()> {
        let fully_init = self
            .startup()
            .await
            .wrap_err("watcher failed to start up")?;

        fully_init.state.set_watcher_ready();

        fully_init.run().await
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
    #[instrument(skip_all, err)]
    async fn startup(self) -> eyre::Result<FullyInitialized> {
        let Self {
            shutdown_token,
            mut startup_handle,
            submitter_handle,
            contract_address,
            ethereum_rpc_endpoint,
            rollup_asset_denom,
            bridge_address,
            use_compat_address,
            state,
        } = self;

        let startup::Info {
            fee_asset,
            starting_rollup_height,
            ..
        } = select! {
            () = shutdown_token.cancelled() => {
                return Err(eyre!("watcher received shutdown signal while waiting for startup"));
            }

            startup_info = startup_handle.get_info() => {
                startup_info.wrap_err("failed to receive startup info")?
            }
        };

        debug!(
            fee_asset = %fee_asset,
            starting_rollup_height = starting_rollup_height,
            "received startup info"
        );

        // connect to eth node
        let retry_config = tryhard::RetryFutureConfig::new(1024)
            .exponential_backoff(Duration::from_millis(500))
            .max_delay(Duration::from_secs(60))
            .on_retry(
                |attempt, next_delay: Option<Duration>, error: &ProviderError| {
                    let wait_duration = next_delay
                        .map(telemetry::display::format_duration)
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
            let url = ethereum_rpc_endpoint.clone();
            async move {
                let websocket_client = Ws::connect_with_reconnects(url, 0).await?;
                Ok(Provider::new(websocket_client))
            }
        })
        .with_config(retry_config)
        .await
        .wrap_err("failed connecting to rollup after several retries; giving up")?;

        let provider = Arc::new(provider);
        let ics20_asset_to_withdraw = if rollup_asset_denom.leading_channel().is_some() {
            info!(
                %rollup_asset_denom,
                "configured rollup asset contains an ics20 channel; ics20 withdrawals will be emitted"
            );
            Some(rollup_asset_denom.clone())
        } else {
            info!(
                %rollup_asset_denom,
                "configured rollup asset does not contain an ics20 channel; ics20 withdrawals will not be emitted"
            );
            None
        };

        let action_fetcher = GetWithdrawalActionsBuilder::new()
            .provider(provider.clone())
            .fee_asset(fee_asset)
            .contract_address(contract_address)
            .bridge_address(bridge_address)
            .sequencer_asset_to_withdraw(rollup_asset_denom.clone().into())
            .set_ics20_asset_to_withdraw(ics20_asset_to_withdraw)
            .use_compat_address(use_compat_address)
            .try_build()
            .await
            .wrap_err("failed to construct contract event to sequencer action fetcher")?;

        Ok(FullyInitialized {
            shutdown_token,
            submitter_handle,
            state,
            provider,
            action_fetcher,
            starting_rollup_height,
        })
    }
}

impl FullyInitialized {
    async fn run(self) -> eyre::Result<()> {
        tokio::select! {
            res = watch_for_blocks(
                self.provider,
                self.action_fetcher,
                self.starting_rollup_height,
                self.submitter_handle,
                self.shutdown_token.clone(),
            ) => {
                res.context("block handler exited")
            }
           () = self.shutdown_token.cancelled() => {
                Ok(())
            }
        }
    }
}

#[instrument(skip_all, fields(from_rollup_height, to_rollup_height), err)]
async fn sync_unprocessed_rollup_heights(
    provider: Arc<Provider<Ws>>,
    action_fetcher: &GetWithdrawalActions<Provider<Ws>>,
    submitter_handle: &submitter::Handle,
    from_rollup_height: u64,
    to_rollup_height: u64,
) -> Result<()> {
    for i in from_rollup_height..=to_rollup_height {
        let block = provider
            .get_block(i)
            .await
            .map_err(eyre::Report::new)
            .and_then(|block| block.ok_or_eyre("block is missing"))
            .wrap_err_with(|| format!("failed to get block at rollup height `{i}`"))?;
        get_and_forward_block_events(action_fetcher, block, submitter_handle)
            .await
            .wrap_err("failed to get and send events at block")?;
    }
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

    info_span!("watch_for_blocks").in_scope(|| {
        info!(
            block.height = current_rollup_block_height.as_u64(),
            block.hash = current_rollup_block.hash.map(tracing::field::display),
            "got current block"
        );
    });

    // sync any blocks missing between `next_rollup_block_height` and the current latest
    // (inclusive).
    sync_unprocessed_rollup_heights(
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
                info_span!("watch_for_blocks").in_scope(|| info!("block watcher shutting down"));
                return Ok(());
            }
            block = block_rx.next() => {
                if let Some(block) = block {
                    get_and_forward_block_events(
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

#[instrument(skip_all, fields(
    block.hash = block.hash.map(tracing::field::display),
    block.number = block.number.map(tracing::field::display),
), err)]
async fn get_and_forward_block_events(
    actions_fetcher: &GetWithdrawalActions<Provider<Ws>>,
    block: Block<H256>,
    submitter_handle: &submitter::Handle,
) -> Result<()> {
    let block_hash = block.hash.ok_or_eyre("block did not contain a hash")?;
    let rollup_height = block
        .number
        .ok_or_eyre("block did not contain a rollup height")?
        .as_u64();
    let actions: Vec<Action> = actions_fetcher
        .get_for_block_hash(block_hash)
        .await
        .wrap_err("failed getting actions for block")?
        .into_iter()
        .filter_map(|r| {
            r.map_err(|e| {
                warn!(
                    error = %eyre::Report::new(e),
                    "failed to convert rollup withdrawal event to sequencer action; dropping"
                );
            })
            .ok()
        })
        .collect();

    if actions.is_empty() {
        info!(
            "no withdrawal actions found for block `{block_hash}` at rollup height \
             `{rollup_height}"
        );
    }
    submitter_handle
        .send_batch(Batch {
            actions,
            rollup_height,
        })
        .await
        .wrap_err("failed to send batched events; receiver dropped?")?;

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
