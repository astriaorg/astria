use astria_sequencer_client::BlockResponse;
use eyre::{
    bail,
    Result,
    WrapErr as _,
};
use tokio::{
    select,
    sync::{
        mpsc::{
            UnboundedReceiver,
            UnboundedSender,
        },
        watch::{
            self,
            Receiver,
        },
    },
    task::{
        self,
        JoinError,
    },
};
use tracing::{
    debug,
    instrument,
    warn,
};

use crate::{
    data_availability::CelestiaClient,
    macros::report_err,
    types::SequencerBlockData,
    validator::Validator,
};

pub struct Relayer {
    data_availability_client: Option<CelestiaClient>,
    validator: Validator,
    sequencer_blocks_rx: UnboundedReceiver<BlockResponse>,
    gossip_block_tx: UnboundedSender<SequencerBlockData>,
    state_tx: watch::Sender<State>,
    queued_blocks: Vec<SequencerBlockData>,
    conversion_workers: task::JoinSet<eyre::Result<Option<SequencerBlockData>>>,
    submission_task: Option<task::JoinHandle<eyre::Result<u64>>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub(crate) current_sequencer_height: Option<u64>,
    pub(crate) current_data_availability_height: Option<u64>,
}

impl State {
    pub fn is_ready(&self) -> bool {
        self.current_sequencer_height.is_some() && self.current_data_availability_height.is_some()
    }
}

impl Relayer {
    /// Instantiates a `Relayer`.
    ///
    /// # Errors
    ///
    /// Returns one of the following errors:
    /// + failed to read the validator keys from the path in cfg;
    /// + failed to construct a client to the data availability layer (unless `cfg.disable_writing`
    ///   is set).
    pub fn new(
        cfg: &crate::config::Config,
        sequencer_blocks_rx: UnboundedReceiver<BlockResponse>,
        gossip_block_tx: UnboundedSender<SequencerBlockData>,
    ) -> Result<Self> {
        let validator = Validator::from_path(&cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let data_availability_client = if cfg.disable_writing {
            debug!("disabling writing to data availability layer requested; disabling");
            None
        } else {
            let client = CelestiaClient::builder()
                .endpoint(&cfg.celestia_endpoint)
                .bearer_token(&cfg.celestia_bearer_token)
                .gas_limit(cfg.gas_limit)
                .build()
                .wrap_err("failed to create data availability client")?;
            Some(client)
        };

        let (state_tx, _) = watch::channel(State::default());

        Ok(Self {
            data_availability_client,
            validator,
            sequencer_blocks_rx,
            gossip_block_tx,
            state_tx,
            queued_blocks: Vec::new(),
            conversion_workers: task::JoinSet::new(),
            submission_task: None,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> Receiver<State> {
        self.state_tx.subscribe()
    }

    fn handle_new_block(&mut self, block: BlockResponse) {
        let current_height = self.state_tx.borrow().current_sequencer_height;
        let validator = self.validator.clone();
        // Start the costly conversion; note that the current height at
        // the time of receipt matters. The internal state might have advanced
        // past the height recorded in the block while it was converting, but
        // that's ok.
        self.conversion_workers.spawn_blocking(move || {
            convert_block_response_to_sequencer_block_data(block, current_height, validator)
        });
    }

    /// Handle the result
    #[instrument(skip_all)]
    fn handle_conversion_completed(
        &mut self,
        join_result: Result<eyre::Result<Option<SequencerBlockData>>, JoinError>,
    ) -> HandleConversionCompletedResult {
        // First check if the join task panicked
        let conversion_result = match join_result {
            Ok(conversion_result) => conversion_result,
            // Report if the task failed, i.e. panicked
            Err(e) => {
                // TODO: inject the correct tracing span
                report_err!(e, "conversion task failed");
                return HandleConversionCompletedResult::Handled;
            }
        };
        // Then handle the actual result of the computation
        match conversion_result {
            // Gossip and collect successfully converted sequencer responses
            Ok(Some(sequencer_block_data)) => {
                if let Err(_) = self.gossip_block_tx.send(sequencer_block_data.clone()) {
                    return HandleConversionCompletedResult::GossipChannelClosed;
                }
                // Update the internal state if the block was admitted
                let height = sequencer_block_data.header.height.value();
                self.state_tx.send_if_modified(|state| {
                    if Some(height) > state.current_sequencer_height {
                        state.current_sequencer_height = Some(height);
                        return true;
                    }
                    false
                });
                // Store the converted data
                self.queued_blocks.push(sequencer_block_data);
            }
            // Ignore sequencer responses that were filtered out
            Ok(None) => (),
            // Report if the conversion failed
            Err(e) => {
                // TODO: inject the correct tracing span
                report_err!(
                    e,
                    "failed converting sequencer block response to block data"
                );
            }
        }
        HandleConversionCompletedResult::Handled
    }

    fn handle_submission_completed(&mut self, join_result: Result<eyre::Result<u64>, JoinError>) {
        self.submission_task = None;
        // First check if the join task panicked
        let submission_result = match join_result {
            Ok(submission_result) => submission_result,
            // Report if the task failed, i.e. panicked
            Err(e) => {
                // TODO: inject the correct tracing span
                report_err!(e, "submission task failed");
                return ();
            }
        };
        // Then report update the internal state or report if submission failed
        match submission_result {
            Ok(height) => self.state_tx.send_modify(|state| {
                state.current_data_availability_height.replace(height);
            }),
            // TODO: add more context to this error, maybe inject a span?
            Err(e) => {
                report_err!(e, "submitting blocks to data availability layer failed");
            }
        }
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// `Relayer::run` never returns an error. The return type is
    /// only set to `eyre::Result` for convenient use in `SequencerRelayer`.
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        let stop_msg = loop {
            select!(
                // Receive new blocks from sequencer poller
                maybe_new_block = self.sequencer_blocks_rx.recv() => {
                    match maybe_new_block {
                        Some(new_block) => self.handle_new_block(new_block),
                        None => break "sequencer block channel closed unexpectedly",
                    }
                }

                // Distribute and store converted/admitted blocks
                Some(res) = self.conversion_workers.join_next() => {
                    if self.handle_conversion_completed(res)
                           .is_gossip_channel_closed()
                    {
                        break "gossip block channel closed unexpectedly";
                    }
                }

                // Record the current height of the data availability layer if a submission
                // was in flight.
                //
                // NOTE: + wrapping the task in an async block makes this lazy;
                //       + `unwrap`ping can't fail because this branch is disabled if `None`
                res = async { self.submission_task.as_mut().unwrap().await }, if self.submission_task.is_some() => {
                    self.handle_submission_completed(res);
                }
            );
            // Try to submit new blocks
            //
            // This will immediately and eagerly try to submit to the data availability
            // layer if no submission is in flight.
            if self.data_availability_client.is_some()
                && !self.queued_blocks.is_empty()
                && !self.submission_task.is_none()
            {
                let client = self.data_availability_client.clone().expect(
                    "this should not fail because the if condition of this block checked that a \
                     client is present",
                );
                self.submission_task = Some(task::spawn(submit_blocks_to_data_availability_layer(
                    client,
                    self.queued_blocks.clone(),
                    self.validator.clone(),
                )));
                self.queued_blocks.clear();
            }
        };
        self.conversion_workers.abort_all();
        self.submission_task.as_mut().map(|task| task.abort());
        bail!(stop_msg);
    }
}

#[instrument(skip_all)]
fn convert_block_response_to_sequencer_block_data(
    res: BlockResponse,
    current_height: Option<u64>,
    validator: Validator,
) -> eyre::Result<Option<SequencerBlockData>> {
    if Some(res.block.header.height.value()) > current_height {
        debug!(
            "sequencer block response contained height at or below the current height tracked in \
             relayer"
        );
        return Ok(None);
    }
    if res.block.header.proposer_address != validator.address {
        debug!("proposer recorded in sequencer block response does not match internal validator");
        return Ok(None);
    }
    let sequencer_block_data = SequencerBlockData::from_tendermint_block(res.block)
        .wrap_err("failed converting sequencer block response to sequencer block data")?;
    Ok(Some(sequencer_block_data))
}

#[instrument(skip_all)]
async fn submit_blocks_to_data_availability_layer(
    client: CelestiaClient,
    sequencer_block_data: Vec<SequencerBlockData>,
    validator: Validator,
) -> eyre::Result<u64> {
    let rsp = client
        .submit_all_blocks(
            sequencer_block_data,
            &validator.signing_key,
            validator.verification_key,
        )
        .await?;
    Ok(rsp.height)
}

enum HandleConversionCompletedResult {
    Handled,
    GossipChannelClosed,
}

impl HandleConversionCompletedResult {
    fn is_gossip_channel_closed(self) -> bool {
        matches!(self, Self::GossipChannelClosed)
    }
}
