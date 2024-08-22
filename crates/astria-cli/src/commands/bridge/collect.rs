use std::{
    collections::BTreeMap,
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
    time::Duration,
};

use astria_bridge_contracts::{
    GetWithdrawalActions,
    GetWithdrawalActionsBuilder,
};
use astria_core::{
    primitive::v1::{
        asset::{
            self,
        },
        Address,
    },
    protocol::transaction::v1alpha1::Action,
};
use clap::Args;
use color_eyre::eyre::{
    self,
    bail,
    ensure,
    eyre,
    OptionExt as _,
    WrapErr as _,
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
};
use futures::stream::BoxStream;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

#[derive(Args, Debug)]
pub(crate) struct WithdrawalEvents {
    /// The websocket endpoint of a geth compatible rollup.
    #[arg(long)]
    rollup_endpoint: String,
    /// The eth address of the astria bridge contracts.
    #[arg(long)]
    contract_address: ethers::types::Address,
    /// The start rollup height from which blocks will be checked for withdrawal events.
    #[arg(long)]
    from_rollup_height: u64,
    /// The end rollup height from which blocks will be checked for withdrawal events.
    /// If not set, then this tool will stream blocks until SIGINT is received.
    #[arg(long)]
    to_rollup_height: Option<u64>,
    /// The asset that will be used to pay the Sequencer fees (should the generated
    /// actions be submitted to the Sequencer).
    #[arg(long, default_value = "nria")]
    fee_asset: asset::Denom,
    /// The sequencer asset withdrawn through the bridge.
    #[arg(long)]
    sequencer_asset_to_withdraw: Option<asset::Denom>,
    /// The is20 asset withdrawn through the bridge.
    #[arg(long)]
    ics20_asset_to_withdraw: Option<asset::TracePrefixed>,
    /// The bech32-encoded bridge address corresponding to the bridged rollup
    /// asset on the sequencer. Should match the bridge address in the geth
    /// rollup's bridge configuration for that asset.
    #[arg(long)]
    bridge_address: Address,
    /// The path to write the collected withdrawal events converted
    /// to Sequencer actions.
    #[arg(long, short)]
    output: PathBuf,
    /// Overwrites <output> if it exists
    #[arg(long, short)]
    force: bool,
}

impl WithdrawalEvents {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let Self {
            rollup_endpoint,
            contract_address,
            from_rollup_height,
            to_rollup_height,
            sequencer_asset_to_withdraw,
            ics20_asset_to_withdraw,
            fee_asset,
            bridge_address,
            output,
            force,
        } = self;

        let output = open_output(&output, force).wrap_err("failed to open output for writing")?;

        let block_provider = connect_to_rollup(&rollup_endpoint)
            .await
            .wrap_err("failed to connect to rollup")?;

        let actions_fetcher = GetWithdrawalActionsBuilder::new()
            .provider(block_provider.clone())
            .contract_address(contract_address)
            .fee_asset(fee_asset)
            .set_ics20_asset_to_withdraw(ics20_asset_to_withdraw)
            .set_sequencer_asset_to_withdraw(sequencer_asset_to_withdraw)
            .bridge_address(bridge_address)
            .try_build()
            .await
            .wrap_err("failed to initialize contract events to sequencer actions converter")?;

        let mut incoming_blocks =
            create_stream_of_blocks(&block_provider, from_rollup_height, to_rollup_height)
                .await
                .wrap_err("failed initializing stream of rollup blocks")?;

        let mut actions_by_rollup_height = ActionsByRollupHeight::new();
        loop {
            tokio::select! {
                biased;

                _ = tokio::signal::ctrl_c() => {
                    break;
                }

                block = incoming_blocks.next() => {
                    match block {
                        Some(Ok(block)) => {
                            if let Err(e) = block_to_actions(
                                block,
                                &mut actions_by_rollup_height,
                                &actions_fetcher,
                            ).await {
                                error!(
                                    error = AsRef::<dyn std::error::Error>::as_ref(&e),
                                    "failed converting contract block to sequencer actions;
                                    exiting stream",
                                );
                                break;
                            }
                        }
                        Some(Err(error)) => {
                            error!(
                                error = AsRef::<dyn std::error::Error>::as_ref(&error),
                                "encountered an error getting block; exiting stream",
                            );
                            break;
                        },
                        None => {
                            info!("block subscription ended");
                            break;
                        }
                    }
                }
            }
        }

        info!(
            "collected a total of {} actions across {} rollup heights; writing to file",
            actions_by_rollup_height
                .0
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            actions_by_rollup_height.0.len(),
        );

        actions_by_rollup_height
            .write_to_output(output)
            .wrap_err("failed to write actions to file")
    }
}

async fn block_to_actions(
    block: Block<H256>,
    actions_by_rollup_height: &mut ActionsByRollupHeight,
    actions_fetcher: &GetWithdrawalActions<Provider<Ws>>,
) -> eyre::Result<()> {
    let block_hash = block
        .hash
        .ok_or_eyre("block did not contain a hash; skipping")?;
    let rollup_height = block
        .number
        .ok_or_eyre("block did not contain a rollup height; skipping")?
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
    actions_by_rollup_height.insert(rollup_height, actions)
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct ActionsByRollupHeight(BTreeMap<u64, Vec<Action>>);

impl ActionsByRollupHeight {
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn into_inner(self) -> BTreeMap<u64, Vec<Action>> {
        self.0
    }

    #[instrument(skip_all, err)]
    fn insert(&mut self, rollup_height: u64, actions: Vec<Action>) -> eyre::Result<()> {
        ensure!(
            self.0.insert(rollup_height, actions).is_none(),
            "already collected actions for block at rollup height `{rollup_height}`; no 2 blocks \
             with the same height should have been seen",
        );
        Ok(())
    }

    #[instrument(skip_all, fields(target = %output.path.display()), err)]
    fn write_to_output(self, output: Output) -> eyre::Result<()> {
        let writer = std::io::BufWriter::new(output.handle);
        serde_json::to_writer(writer, &self.0).wrap_err("failed writing actions to file")
    }
}

/// Constructs a block stream from `start` until `maybe_end`, if `Some`.
/// Constructs an open ended stream from `start` if `None`.
#[instrument(skip_all, fields(start, end = maybe_end), err)]
async fn create_stream_of_blocks(
    block_provider: &Provider<Ws>,
    start: u64,
    maybe_end: Option<u64>,
) -> eyre::Result<BoxStream<'_, eyre::Result<Block<H256>>>> {
    let subscription = if let Some(end) = maybe_end {
        futures::stream::iter(start..=end)
            .then(move |height| async move {
                block_provider
                    .get_block(height)
                    .await
                    .wrap_err("failed to get block")?
                    .ok_or_else(|| eyre!("block with number {height} missing"))
            })
            .boxed()
    } else {
        let mut block_subscription = block_provider
            .subscribe_blocks()
            .await
            .wrap_err("failed to subscribe to blocks from rollup")?
            .boxed();

        let Some(current_rollup_block) = block_subscription.next().await else {
            bail!("failed to get current rollup block from subscription")
        };

        let Some(current_rollup_block_height) = current_rollup_block.number else {
            bail!(
                "couldn't determine current rollup block height; value was not set on current on \
                 most recent block",
            );
        };

        futures::stream::iter(start..current_rollup_block_height.as_u64())
            .then(move |height| async move {
                block_provider
                    .get_block(height)
                    .await
                    .wrap_err("failed to get block")?
                    .ok_or_else(|| eyre!("block with number {height} missing"))
            })
            .chain(futures::stream::once(
                async move { Ok(current_rollup_block) },
            ))
            .chain(block_subscription.map(Ok))
            .boxed()
    };
    Ok(subscription)
}

#[derive(Debug)]
struct Output {
    handle: std::fs::File,
    path: PathBuf,
}

#[instrument(skip(target), fields(target = %target.as_ref().display()), err)]
fn open_output<P: AsRef<Path>>(target: P, overwrite: bool) -> eyre::Result<Output> {
    let handle = if overwrite {
        let mut options = std::fs::File::options();
        options.write(true).create(true).truncate(true);
        options
    } else {
        let mut options = std::fs::File::options();
        options.write(true).create_new(true);
        options
    }
    .open(&target)
    .wrap_err("failed to open specified file for writing")?;
    Ok(Output {
        handle,
        path: target.as_ref().to_path_buf(),
    })
}

#[instrument(err)]
async fn connect_to_rollup(rollup_endpoint: &str) -> eyre::Result<Arc<Provider<Ws>>> {
    let retry_config = tryhard::RetryFutureConfig::new(10)
        .fixed_backoff(Duration::from_secs(2))
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
                std::future::ready(())
            },
        );

    let provider = tryhard::retry_fn(|| Provider::<Ws>::connect(rollup_endpoint))
        .with_config(retry_config)
        .await
        .wrap_err("failed connecting to rollup after several retries; giving up")?;
    Ok(Arc::new(provider))
}
