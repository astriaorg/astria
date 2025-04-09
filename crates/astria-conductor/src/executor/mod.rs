use std::{
    collections::HashMap,
    time::Duration,
};

use astria_core::{
    execution::v1::{
        Block,
        CommitmentState,
    },
    primitive::v1::RollupId,
    protocol::price_feed::v1::ExtendedCommitInfoWithCurrencyPairMapping,
    sequencerblock::v1::block::{
        self,
        FilteredSequencerBlock,
        FilteredSequencerBlockParts,
        PriceFeedData,
        RollupData,
    },
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    eyre,
    WrapErr as _,
};
use bytes::Bytes;
use sequencer_client::{
    tendermint::{
        block::Height as SequencerHeight,
        Time as TendermintTime,
    },
    HttpClient,
};
use tokio::{
    select,
    sync::mpsc,
    task::JoinError,
};
use tokio_util::{
    sync::CancellationToken,
    task::JoinMap,
};
use tracing::{
    debug,
    debug_span,
    error,
    info,
    instrument,
    warn,
};

use crate::{
    celestia::ReconstructedBlock,
    config::CommitLevel,
    metrics::Metrics,
};

mod builder;

pub(crate) use builder::Builder;

mod client;
mod state;
#[cfg(test)]
mod tests;

pub(super) use client::Client;
use state::State;
pub(crate) use state::StateReceiver;

use self::state::StateSender;

type CelestiaHeight = u64;

pub(crate) struct Executor {
    config: crate::Config,

    /// The execution client driving the rollup.
    client: Client,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,

    metrics: &'static Metrics,
}

impl Executor {
    const CELESTIA: &'static str = "celestia";
    const SEQUENCER: &'static str = "sequencer";

    pub(crate) async fn run_until_stopped(self) -> eyre::Result<()> {
        let initialized = select!(
            () = self.shutdown.clone().cancelled_owned() => {
                return report_exit(Ok(
                    "received shutdown signal while initializing task; \
                    aborting intialization and exiting"
                ), "");
            }
            res = self.init() => {
                res.wrap_err("initialization failed")?
            }
        );

        initialized.run().await
    }

    /// Runs the init logic that needs to happen before [`Executor`] can enter its main loop.
    #[instrument(skip_all, err)]
    async fn init(self) -> eyre::Result<Initialized> {
        let state = self
            .create_initial_node_state()
            .await
            .wrap_err("failed setting initial rollup node state")?;

        let sequencer_cometbft_client = HttpClient::new(&*self.config.sequencer_cometbft_url)
            .wrap_err("failed constructing sequencer cometbft RPC client")?;

        let reader_cancellation_token = self.shutdown.child_token();

        let (firm_blocks_tx, firm_blocks_rx) = tokio::sync::mpsc::channel(16);
        let (soft_blocks_tx, soft_blocks_rx) =
            tokio::sync::mpsc::channel(state.calculate_max_spread());

        let mut reader_tasks = JoinMap::new();
        if self.config.is_with_firm() {
            let celestia_token = if self.config.no_celestia_auth {
                None
            } else {
                Some(self.config.celestia_bearer_token.clone())
            };

            let reader = crate::celestia::Builder {
                celestia_http_endpoint: self.config.celestia_node_http_url.clone(),
                celestia_token,
                celestia_block_time: Duration::from_millis(self.config.celestia_block_time_ms),
                firm_blocks: firm_blocks_tx,
                rollup_state: state.subscribe(),
                sequencer_cometbft_client: sequencer_cometbft_client.clone(),
                sequencer_requests_per_second: self.config.sequencer_requests_per_second,
                expected_celestia_chain_id: self.config.expected_celestia_chain_id.clone(),
                expected_sequencer_chain_id: self.config.expected_sequencer_chain_id.clone(),
                shutdown: reader_cancellation_token.child_token(),
                metrics: self.metrics,
            }
            .build()
            .wrap_err("failed to build Celestia Reader")?;
            reader_tasks.spawn(Self::CELESTIA, reader.run_until_stopped());
        }

        if self.config.is_with_soft() {
            let sequencer_grpc_client =
                crate::sequencer::SequencerGrpcClient::new(&self.config.sequencer_grpc_url)
                    .wrap_err("failed constructing grpc client for Sequencer")?;

            let sequencer_reader = crate::sequencer::Builder {
                sequencer_grpc_client,
                sequencer_cometbft_client: sequencer_cometbft_client.clone(),
                sequencer_block_time: Duration::from_millis(self.config.sequencer_block_time_ms),
                expected_sequencer_chain_id: self.config.expected_sequencer_chain_id.clone(),
                shutdown: reader_cancellation_token.child_token(),
                soft_blocks: soft_blocks_tx,
                rollup_state: state.subscribe(),
            }
            .build();
            reader_tasks.spawn(Self::SEQUENCER, sequencer_reader.run_until_stopped());
        };

        Ok(Initialized {
            config: self.config,
            client: self.client,
            firm_blocks: firm_blocks_rx,
            soft_blocks: soft_blocks_rx,
            shutdown: self.shutdown,
            state,
            blocks_pending_finalization: HashMap::new(),
            metrics: self.metrics,
            reader_tasks,
            reader_cancellation_token,
        })
    }

    #[instrument(skip_all, err)]
    async fn create_initial_node_state(&self) -> eyre::Result<StateSender> {
        let genesis_info = {
            async {
                self.client
                    .clone()
                    .get_genesis_info_with_retry()
                    .await
                    .wrap_err("failed getting genesis info")
            }
        };
        let commitment_state = {
            async {
                self.client
                    .clone()
                    .get_commitment_state_with_retry()
                    .await
                    .wrap_err("failed getting commitment state")
            }
        };
        let (genesis_info, commitment_state) = tokio::try_join!(genesis_info, commitment_state)?;

        let (state, _) = state::channel(
            State::try_from_genesis_info_and_commitment_state(genesis_info, commitment_state)
                .wrap_err(
                    "failed to construct initial state gensis and commitment info received from \
                     rollup",
                )?,
        );

        self.metrics
            .absolute_set_executed_firm_block_number(state.firm_number());
        self.metrics
            .absolute_set_executed_soft_block_number(state.soft_number());
        info!(
            initial_state = serde_json::to_string(&*state.get())
                .expect("writing json to a string should not fail"),
            "received genesis info from rollup",
        );
        Ok(state)
    }
}

struct Initialized {
    config: crate::Config,

    /// The execution client driving the rollup.
    client: Client,

    /// The channel of which this executor receives blocks for executing
    /// firm commitments.
    firm_blocks: mpsc::Receiver<Box<ReconstructedBlock>>,

    /// The channel of which this executor receives blocks for executing
    /// soft commitments.
    soft_blocks: mpsc::Receiver<FilteredSequencerBlock>,

    /// Token to listen for Conductor being shut down.
    shutdown: CancellationToken,

    /// Tracks the status of the execution chain.
    state: StateSender,

    /// Tracks rollup blocks by their rollup block numbers.
    ///
    /// Required to mark firm blocks received from celestia as executed
    /// without re-executing on top of the rollup node.
    blocks_pending_finalization: HashMap<u32, Block>,

    metrics: &'static Metrics,

    /// The tasks reading block data off Celestia or Sequencer.
    reader_tasks: JoinMap<&'static str, eyre::Result<()>>,

    /// The cancellation token specifically for signaling the `reader_tasks` to shut down.
    reader_cancellation_token: CancellationToken,
}

impl Initialized {
    async fn run(mut self) -> eyre::Result<()> {
        let reason = select!(
            biased;

            () = self.shutdown.clone().cancelled_owned() => {
                Ok("received shutdown signal")
            }

            res = self.run_event_loop() => {
                res
            }
        );

        self.shutdown(reason).await
    }

    async fn run_event_loop(&mut self) -> eyre::Result<&'static str> {
        loop {
            select!(
                biased;

                Some(block) = self.firm_blocks.recv() =>
                {
                    debug_span!("conductor::Executor::run_until_stopped").in_scope(||debug!(
                        block.height = %block.sequencer_height(),
                        block.hash = %block.block_hash(),
                        "received block from celestia reader",
                    ));
                    if let Err(error) = self.execute_firm(block).await {
                        break Err(error).wrap_err("failed executing firm block");
                    }
                }

                Some(block) = self.soft_blocks.recv(), if !self.is_spread_too_large() =>
                {
                    debug_span!("conductor::Executor::run_until_stopped").in_scope(||debug!(
                        block.height = %block.height(),
                        block.hash = %block.block_hash(),
                        "received block from sequencer reader",
                    ));
                    if let Err(error) = self.execute_soft(block).await {
                        break Err(error).wrap_err("failed executing soft block");
                    }
                }

                Some((task, res)) = self.reader_tasks.join_next() => {
                    break handle_task_exit(task, res);
                }

                else => break Ok("all channels are closed")
            );
        }
    }

    /// Returns if the spread between firm and soft commitment heights in the tracked state is too
    /// large.
    ///
    /// Always returns `false` if this executor was configured to run without firm commitments.
    ///
    /// # Panics
    ///
    /// Panics if called before [`Executor::init`] because `max_spread` must be set.
    fn is_spread_too_large(&self) -> bool {
        if !self.config.is_with_firm() {
            return false;
        }
        let (next_firm, next_soft) = {
            let next_firm = self.state.next_expected_firm_sequencer_height().value();
            let next_soft = self.state.next_expected_soft_sequencer_height().value();
            (next_firm, next_soft)
        };

        let is_too_far_ahead = usize::try_from(next_soft.saturating_sub(next_firm))
            .map(|spread| spread >= self.state.calculate_max_spread())
            .unwrap_or(false);

        if is_too_far_ahead {
            debug!("soft blocks are too far ahead of firm; skipping soft blocks");
        }
        is_too_far_ahead
    }

    #[instrument(skip_all, fields(
        block.hash = %block.block_hash(),
        block.height = block.height().value(),
        err,
    ))]
    async fn execute_soft(&mut self, block: FilteredSequencerBlock) -> eyre::Result<()> {
        // TODO(https://github.com/astriaorg/astria/issues/624): add retry logic before failing hard.
        let executable_block = ExecutableBlock::from_sequencer(block, self.state.rollup_id());

        let expected_height = self.state.next_expected_soft_sequencer_height();
        match executable_block.height.cmp(&expected_height) {
            std::cmp::Ordering::Less => {
                info!(
                    expected_height.sequencer_block = %expected_height,
                    "block received was stale because firm blocks were executed first; dropping",
                );
                return Ok(());
            }
            std::cmp::Ordering::Greater => bail!(
                "block received was out-of-order; was a block skipped? expected: \
                 {expected_height}, actual: {}",
                executable_block.height
            ),
            std::cmp::Ordering::Equal => {}
        }

        let genesis_height = self.state.sequencer_genesis_block_height();
        let block_height = executable_block.height;
        let Some(block_number) =
            state::map_sequencer_height_to_rollup_height(genesis_height, block_height)
        else {
            bail!(
                "failed to map block height rollup number. This means the operation
                `sequencer_height - sequencer_genesis_height` underflowed or was not a valid
                cometbft height. Sequencer height: `{block_height}`, sequencer genesis height: \
                 `{genesis_height}`",
            )
        };

        // The parent hash of the next block is the hash of the block at the current head.
        let parent_hash = self.state.soft_hash();
        let executed_block = self
            .execute_block(parent_hash, executable_block)
            .await
            .wrap_err("failed to execute block")?;

        self.does_block_response_fulfill_contract(ExecutionKind::Soft, &executed_block)
            .wrap_err("execution API server violated contract")?;

        self.update_commitment_state(Update::OnlySoft(executed_block.clone()))
            .await
            .wrap_err("failed to update soft commitment state")?;

        self.blocks_pending_finalization
            .insert(block_number, executed_block);

        // XXX: We set an absolute number value here to avoid any potential issues of the remote
        // rollup state and the local state falling out of lock-step.
        self.metrics
            .absolute_set_executed_soft_block_number(block_number);

        Ok(())
    }

    #[instrument(skip_all, fields(
        block.hash = %block.block_hash(),
        block.height = block.sequencer_height().value(),
        err,
    ))]
    async fn execute_firm(&mut self, block: Box<ReconstructedBlock>) -> eyre::Result<()> {
        let celestia_height = block.celestia_height;
        let executable_block = ExecutableBlock::from_reconstructed(*block);
        let expected_height = self.state.next_expected_firm_sequencer_height();
        let block_height = executable_block.height;
        ensure!(
            block_height == expected_height,
            "expected block at sequencer height {expected_height}, but got {block_height}",
        );

        let genesis_height = self.state.sequencer_genesis_block_height();
        let Some(block_number) =
            state::map_sequencer_height_to_rollup_height(genesis_height, block_height)
        else {
            bail!(
                "failed to map block height rollup number. This means the operation
                `sequencer_height - sequencer_genesis_height` underflowed or was not a valid
                cometbft height. Sequencer height: `{block_height}`, sequencer genesis height: \
                 `{genesis_height}`",
            )
        };

        let update = if self.should_execute_firm_block() {
            let parent_hash = self.state.firm_hash();
            let executed_block = self
                .execute_block(parent_hash, executable_block)
                .await
                .wrap_err("failed to execute block")?;
            self.does_block_response_fulfill_contract(ExecutionKind::Firm, &executed_block)
                .wrap_err("execution API server violated contract")?;
            Update::ToSame(executed_block, celestia_height)
        } else if let Some(block) = self.blocks_pending_finalization.remove(&block_number) {
            debug!(
                block_number,
                "found pending block in cache; updating state but not not re-executing it"
            );
            Update::OnlyFirm(block, celestia_height)
        } else {
            debug!(
                block_number,
                "pending block not found for block number in cache. THIS SHOULD NOT HAPPEN. \
                 Trying to fetch the already-executed block from the rollup before giving up."
            );
            match self.client.get_block_with_retry(block_number).await {
                Ok(block) => Update::OnlyFirm(block, celestia_height),
                Err(error) => {
                    error!(
                        block_number,
                        %error,
                        "failed to fetch block from rollup and can will not be able to update \
                        firm commitment state. Giving up."
                    );
                    return Err(error).wrap_err_with(|| {
                        format!("failed to get block at number `{block_number}` from rollup")
                    });
                }
            }
        };

        self.update_commitment_state(update)
            .await
            .wrap_err("failed to setting both commitment states to executed block")?;

        // XXX: We set an absolute number value here to avoid any potential issues of the remote
        // rollup state and the local state falling out of lock-step.
        self.metrics
            .absolute_set_executed_soft_block_number(block_number);

        Ok(())
    }

    /// Executes `block` on top of its `parent_hash`.
    ///
    /// This function is called via [`Executor::execute_firm`] or [`Executor::execute_soft`],
    /// and should not be called directly.
    #[instrument(skip_all, fields(
        block.hash = %block.hash,
        block.height = block.height.value(),
        block.num_of_transactions = block.transactions.len(),
        rollup.parent_hash = %telemetry::display::base64(&parent_hash),
        err
    ))]
    async fn execute_block(
        &mut self,
        parent_hash: Bytes,
        block: ExecutableBlock,
    ) -> eyre::Result<Block> {
        let ExecutableBlock {
            hash,
            transactions,
            timestamp,
            ..
        } = block;

        let n_transactions = transactions.len();
        let sequencer_block_hash = hash.as_bytes().to_vec().into();

        let executed_block = self
            .client
            .execute_block_with_retry(parent_hash, transactions, timestamp, sequencer_block_hash)
            .await
            .wrap_err("failed to run execute_block RPC")?;

        self.metrics
            .record_transactions_per_executed_block(n_transactions);

        info!(
            executed_block.hash = %telemetry::display::base64(&executed_block.hash()),
            executed_block.number = executed_block.number(),
            "executed block",
        );

        Ok(executed_block)
    }

    #[instrument(skip_all, err)]
    async fn update_commitment_state(&mut self, update: Update) -> eyre::Result<()> {
        use Update::{
            OnlyFirm,
            OnlySoft,
            ToSame,
        };
        let (firm, soft, celestia_height) = match update {
            OnlyFirm(firm, celestia_height) => (firm, self.state.soft(), celestia_height),
            OnlySoft(soft) => (
                self.state.firm(),
                soft,
                self.state.celestia_base_block_height(),
            ),
            ToSame(block, celestia_height) => (block.clone(), block, celestia_height),
        };
        let commitment_state = CommitmentState::builder()
            .firm(firm)
            .soft(soft)
            .base_celestia_height(celestia_height)
            .build()
            .wrap_err("failed constructing commitment state")?;
        let new_state = self
            .client
            .update_commitment_state_with_retry(commitment_state)
            .await
            .wrap_err("failed updating remote commitment state")?;
        info!(
            soft.number = new_state.soft().number(),
            soft.hash = %telemetry::display::base64(&new_state.soft().hash()),
            firm.number = new_state.firm().number(),
            firm.hash = %telemetry::display::base64(&new_state.firm().hash()),
            "updated commitment state",
        );
        self.state
            .try_update_commitment_state(new_state)
            .wrap_err("failed updating internal state tracking rollup state; invalid?")?;
        Ok(())
    }

    fn does_block_response_fulfill_contract(
        &mut self,
        kind: ExecutionKind,
        block: &Block,
    ) -> Result<(), ContractViolation> {
        does_block_response_fulfill_contract(&mut self.state, kind, block)
    }

    /// Returns whether a firm block should be executed.
    ///
    /// Firm blocks should be executed if:
    /// 1. executor runs in firm only mode (blocks are always executed).
    /// 2. executor runs in soft-and-firm mode and the soft and firm rollup numbers are equal.
    fn should_execute_firm_block(&self) -> bool {
        should_execute_firm_block(
            self.state.next_expected_firm_sequencer_height().value(),
            self.state.next_expected_soft_sequencer_height().value(),
            self.config.execution_commit_level,
        )
    }

    #[instrument(skip_all, err)]
    async fn shutdown(mut self, reason: eyre::Result<&'static str>) -> eyre::Result<()> {
        info!("signaling all reader tasks to exit");
        self.reader_cancellation_token.cancel();
        while let Some((task, exit_status)) = self.reader_tasks.join_next().await {
            match crate::utils::flatten(exit_status) {
                Ok(()) => info!(task, "task exited"),
                Err(error) => warn!(task, %error, "task exited with error"),
            }
        }
        report_exit(reason, "shutting down")
    }
}

/// Wraps a task result to explain why it exited.
///
/// Right now only the err-branch is populated because tasks should
/// never exit. Still returns an `eyre::Result` to line up with the
/// return type of [`Executor::run_until_stopped`].
///
/// Executor should `break handle_task_exit` immediately after calling
/// this method.
fn handle_task_exit(
    task: &'static str,
    res: Result<eyre::Result<()>, JoinError>,
) -> eyre::Result<&'static str> {
    match res {
        Ok(Ok(())) => Err(eyre!("task `{task}` finished unexpectedly")),
        Ok(Err(err)) => Err(err).wrap_err_with(|| format!("task `{task}` exited with error")),
        Err(err) => Err(err).wrap_err_with(|| format!("task `{task}` panicked")),
    }
}

#[instrument(skip_all)]
fn report_exit(reason: eyre::Result<&str>, message: &str) -> eyre::Result<()> {
    // XXX: explicitly setting the message (usually implicitly set by tracing)
    match reason {
        Ok(reason) => {
            info!(%reason, message);
            Ok(())
        }
        Err(error) => {
            error!(%error, message);
            Err(error)
        }
    }
}

enum Update {
    OnlyFirm(Block, CelestiaHeight),
    OnlySoft(Block),
    ToSame(Block, CelestiaHeight),
}

#[derive(Debug)]
struct ExecutableBlock {
    hash: block::Hash,
    height: SequencerHeight,
    timestamp: pbjson_types::Timestamp,
    transactions: Vec<Bytes>,
}

impl ExecutableBlock {
    fn from_reconstructed(block: ReconstructedBlock) -> Self {
        let ReconstructedBlock {
            block_hash,
            header,
            transactions,
            extended_commit_info,
            ..
        } = block;
        let timestamp = convert_tendermint_time_to_protobuf_timestamp(header.time());
        let transactions =
            prepend_transactions_by_price_feed_if_exists(transactions, extended_commit_info);
        Self {
            hash: block_hash,
            height: header.height(),
            timestamp,
            transactions,
        }
    }

    fn from_sequencer(block: FilteredSequencerBlock, id: RollupId) -> Self {
        let extended_commit_info = block.extended_commit_info().cloned();
        let FilteredSequencerBlockParts {
            block_hash,
            header,
            mut rollup_transactions,
            ..
        } = block.into_parts();
        let height = header.height();
        let timestamp = convert_tendermint_time_to_protobuf_timestamp(header.time());
        let transactions = rollup_transactions
            .swap_remove(&id)
            .map(|txs| txs.transactions().to_vec())
            .unwrap_or_default();

        let transactions =
            prepend_transactions_by_price_feed_if_exists(transactions, extended_commit_info);
        Self {
            hash: block_hash,
            height,
            timestamp,
            transactions,
        }
    }
}

/// Prepends the price data to the transactions if it can be calculated from `extended_commit_info`.
///
/// Regardless of the order of the returned collection, the rollup can choose whichever execution
/// order suits best for its use case, but it's anticipated that applying the updated price feed
/// before executing transactions would be a common use case, so the price feed is prepended for
/// convenience.
#[instrument(skip_all)]
fn prepend_transactions_by_price_feed_if_exists(
    transactions: Vec<Bytes>,
    extended_commit_info: Option<ExtendedCommitInfoWithCurrencyPairMapping>,
) -> Vec<Bytes> {
    use astria_core::oracles::price_feed::utils::calculate_prices_from_vote_extensions;
    use prost::Message as _;

    let Some(extended_commit_info) = extended_commit_info else {
        return transactions;
    };

    let prices = match calculate_prices_from_vote_extensions(
        &extended_commit_info.extended_commit_info,
        &extended_commit_info.id_to_currency_pair,
    ) {
        Ok(prices) => prices,
        Err(error) => {
            warn!(
                error = %error,
                "failed to calculate prices from vote extensions; continuing without price feed \
                data"
            );
            return transactions;
        }
    };

    let rollup_data = RollupData::PriceFeedData(Box::new(PriceFeedData::new(prices)));
    prepend(rollup_data.into_raw().encode_to_vec().into(), transactions)
}

fn prepend(item_to_prepend: Bytes, txs: Vec<Bytes>) -> Vec<Bytes> {
    std::iter::once(item_to_prepend).chain(txs).collect()
}

/// Converts a [`tendermint::Time`] to a [`prost_types::Timestamp`].
fn convert_tendermint_time_to_protobuf_timestamp(value: TendermintTime) -> pbjson_types::Timestamp {
    let sequencer_client::tendermint_proto::google::protobuf::Timestamp {
        seconds,
        nanos,
    } = value.into();
    pbjson_types::Timestamp {
        seconds,
        nanos,
    }
}

#[derive(Copy, Clone, Debug)]
enum ExecutionKind {
    Firm,
    Soft,
}

impl std::fmt::Display for ExecutionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            ExecutionKind::Firm => "firm",
            ExecutionKind::Soft => "soft",
        };
        f.write_str(kind)
    }
}

#[derive(Debug, thiserror::Error)]
enum ContractViolation {
    #[error(
        "contract violated: execution kind: {kind}, current block number {current}, expected \
         {expected}, received {actual}"
    )]
    WrongBlock {
        kind: ExecutionKind,
        current: u32,
        expected: u32,
        actual: u32,
    },
    #[error("contract violated: current height cannot be incremented")]
    CurrentBlockNumberIsMax { kind: ExecutionKind, actual: u32 },
}

fn does_block_response_fulfill_contract(
    state: &mut StateSender,
    kind: ExecutionKind,
    block: &Block,
) -> Result<(), ContractViolation> {
    let current = match kind {
        ExecutionKind::Firm => state.firm_number(),
        ExecutionKind::Soft => state.soft_number(),
    };
    let actual = block.number();
    let expected = current
        .checked_add(1)
        .ok_or(ContractViolation::CurrentBlockNumberIsMax {
            kind,
            actual,
        })?;
    if actual == expected {
        Ok(())
    } else {
        Err(ContractViolation::WrongBlock {
            kind,
            current,
            expected,
            actual,
        })
    }
}

fn should_execute_firm_block(
    firm_sequencer_height: u64,
    soft_sequencer_height: u64,
    executor_mode: CommitLevel,
) -> bool {
    match executor_mode {
        CommitLevel::SoftAndFirm if firm_sequencer_height == soft_sequencer_height => true,
        CommitLevel::SoftOnly | CommitLevel::SoftAndFirm => false,
        CommitLevel::FirmOnly => true,
    }
}
