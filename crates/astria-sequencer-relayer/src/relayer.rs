use std::time::Duration;

use astria_sequencer_client::Client as SequencerClient;
use eyre::{
    Result,
    WrapErr as _,
};
use tokio::{
    sync::{
        mpsc::UnboundedSender,
        watch::{
            self,
            Receiver,
        },
    },
    time,
};
use tracing::{
    debug,
    info,
    warn,
};

use crate::{
    data_availability::CelestiaClient,
    serde::NamespaceToTxCount,
    types::SequencerBlockData,
    validator::Validator,
};

pub struct Relayer {
    sequencer_client: SequencerClient,
    data_availability_client: Option<CelestiaClient>,
    validator: Validator,
    sequencer_poll_period: Duration,
    block_tx: UnboundedSender<SequencerBlockData>,
    state_tx: watch::Sender<State>,
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
    /// + failed to construct a client to the sequencer;
    /// + failed to construct a client to the data availability layer (unless `cfg.disable_writing`
    ///   is set).
    pub fn new(
        cfg: &crate::config::Config,
        block_tx: UnboundedSender<SequencerBlockData>,
    ) -> Result<Self> {
        let validator = Validator::from_path(&cfg.validator_key_file)
            .wrap_err("failed to get validator info from file")?;

        let sequencer_client = SequencerClient::new(&cfg.sequencer_endpoint)
            .wrap_err("failed to create sequencer client")?;

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
            sequencer_client,
            data_availability_client,
            sequencer_poll_period: Duration::from_millis(cfg.block_time),
            validator,
            block_tx,
            state_tx,
        })
    }

    pub(crate) fn subscribe_to_state(&self) -> Receiver<State> {
        self.state_tx.subscribe()
    }

    async fn get_and_submit_latest_block(&self) -> eyre::Result<State> {
        let mut new_state = (*self.state_tx.borrow()).clone();
        let resp = self.sequencer_client.get_latest_block().await?;

        let height = resp.block.header.height.value();
        if height <= *new_state.current_sequencer_height.get_or_insert(height) {
            return Ok(new_state);
        }

        info!(height = ?height, tx_count = resp.block.data.len(), "got block from sequencer");
        new_state.current_sequencer_height.replace(height);

        if resp.block.header.proposer_address.as_ref() != self.validator.address.as_ref() {
            let proposer_address = resp.block.header.proposer_address;
            info!(
                %proposer_address,
                validator_address = %self.validator.address,
                "ignoring block: proposer address is not ours",
            );
            return Ok(new_state);
        }

        let sequencer_block = match SequencerBlockData::from_tendermint_block(resp.block) {
            Ok(block) => block,
            Err(e) => {
                warn!(error = ?e, "failed to convert block to DA block");
                return Ok(new_state);
            }
        };

        info!(
            sequencer_block = height,
            proposer = ?sequencer_block.header.proposer_address,
            namespaces_to_tx_count = %NamespaceToTxCount(&sequencer_block.rollup_txs),
            "submitting sequencer block to DA layer",
        );

        self.block_tx.send(sequencer_block.clone())?;
        let namespace_count = sequencer_block.rollup_txs.len();
        if let Some(client) = &self.data_availability_client {
            match client
                .submit_block(
                    sequencer_block,
                    &self.validator.signing_key,
                    self.validator.verification_key,
                )
                .await
            {
                Ok(resp) => {
                    new_state
                        .current_data_availability_height
                        .replace(resp.height);
                    info!(
                        sequencer_block = height,
                        da_layer_block = resp.height,
                        namespace_count = namespace_count,
                        "submitted sequencer block to DA layer",
                    );
                }
                Err(e) => warn!(error = ?e, "failed to submit block to DA layer"),
            }
        }
        Ok(new_state)
    }

    /// Runs the relayer worker.
    ///
    /// # Errors
    ///
    /// `Relayer::run` never returns an error. The return type is
    /// only set to `eyre::Result` for convenient use in `SequencerRelayer`.
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let mut interval = time::interval(self.sequencer_poll_period);
        loop {
            interval.tick().await;
            match self.get_and_submit_latest_block().await {
                Err(e) => warn!(error = ?e, "failed to get latest block from sequencer"),
                Ok(new_state) if new_state != *self.state_tx.borrow() => {
                    _ = self.state_tx.send_replace(new_state);
                }
                Ok(_) => {}
            }
        }
        // Return Ok to make the types align (see the method's doc comment why this is necessary).
        // Allow unreachable code to quiet warnings
        #[allow(unreachable_code)]
        Ok(())
    }
}
