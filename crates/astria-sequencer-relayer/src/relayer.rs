use bech32::{
    self,
    ToBase32,
    Variant,
};
use serde::Deserialize;
use tendermint::{
    account,
    block::Height,
};
use tokio::{
    sync::{
        mpsc::UnboundedSender,
        watch,
    },
    task::JoinHandle,
    time::Interval,
};
use tracing::{
    info,
    warn,
};

use crate::{
    base64_string::Base64String,
    data_availability::CelestiaClient,
    keys::private_key_bytes_to_keypair,
    sequencer::SequencerClient,
    sequencer_block::SequencerBlock,
};

#[derive(Deserialize)]
pub struct ValidatorPrivateKeyFile {
    pub address: String,
    pub pub_key: KeyWithType,
    pub priv_key: KeyWithType,
}

#[derive(Deserialize)]
pub struct KeyWithType {
    #[serde(rename = "type")]
    pub key_type: String,
    pub value: String,
}

pub struct Relayer {
    sequencer_client: SequencerClient,
    da_client: CelestiaClient,
    disable_writing: bool,
    keypair: ed25519_dalek::Keypair,
    validator_address: account::Id,
    interval: Interval,
    block_tx: UnboundedSender<SequencerBlock>,

    state: watch::Sender<State>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub(crate) current_sequencer_height: Option<Height>,
    pub(crate) current_data_availability_height: Option<Height>,
}

impl State {
    pub fn is_ready(&self) -> bool {
        self.current_sequencer_height.is_some() && self.current_data_availability_height.is_some()
    }
}

impl Relayer {
    pub fn new(
        sequencer_client: SequencerClient,
        da_client: CelestiaClient,
        key_file: ValidatorPrivateKeyFile,
        interval: Interval,
        block_tx: UnboundedSender<SequencerBlock>,
    ) -> eyre::Result<Self> {
        // generate our private-public keypair
        let keypair = private_key_bytes_to_keypair(
            &Base64String::from_string(key_file.priv_key.value)
                .expect("failed to decode validator private key; must be base64 string")
                .0,
        )
        .expect("failed to convert validator private key to keypair");

        // generate our validator address
        let validator_address_bytes: [u8; account::LENGTH] = hex::decode(&key_file.address)
            .expect("failed to decode validator address; must be hex string")
            .try_into()
            .map_err(|e| {
                eyre::eyre!("failed to convert validator address to account::Id:\n{e:?}")
            })?;
        let validator_address = account::Id::new(validator_address_bytes);

        let (state, _) = watch::channel(State::default());

        Ok(Self {
            sequencer_client,
            da_client,
            disable_writing: false,
            keypair,
            validator_address,
            interval,
            block_tx,
            state,
        })
    }

    pub fn disable_writing(&mut self) {
        self.disable_writing = true;
    }

    pub fn subscribe_to_state(&self) -> watch::Receiver<State> {
        self.state.subscribe()
    }

    async fn get_and_submit_latest_block(&self) -> eyre::Result<State> {
        let mut new_state = (*self.state.borrow()).clone();
        let resp = self.sequencer_client.get_latest_block().await?;
        let block = resp.block;

        let height = block.header.height;
        if height
            <= *new_state
                .current_sequencer_height
                .get_or_insert(block.header.height)
        {
            return Ok(new_state);
        }

        info!(
            "got block with height {} from sequencer",
            block.header.height
        );
        new_state
            .current_sequencer_height
            .replace(block.header.height);

        let proposer_address_string = bech32::encode(
            "metrovalcons",
            Base64String::from_bytes(&block.header.proposer_address.0)
                .0
                .to_base32(),
            Variant::Bech32,
        )
        .expect("should encode block proposer address");
        if proposer_address_string != self.validator_address.to_string() {
            info!(
                %proposer_address_string,
                validator_address = %self.validator_address,
                "ignoring block: proposer address is not ours",
            );
            return Ok(new_state);
        }

        let sequencer_block = match SequencerBlock::from_cosmos_block(block) {
            Ok(block) => block,
            Err(e) => {
                warn!(error = ?e, "failed to convert block to DA block");
                return Ok(new_state);
            }
        };

        self.block_tx.send(sequencer_block.clone())?;

        let tx_count = sequencer_block.rollup_transactions.len()
            + sequencer_block.sequencer_transactions.len();
        if self.disable_writing {
            return Ok(new_state);
        }

        match self
            .da_client
            .submit_block(sequencer_block, &self.keypair)
            .await
        {
            Ok(resp) => {
                let height = Height::try_from(resp.height)?;
                new_state.current_data_availability_height.replace(height);
                info!(
                    sequencer_block = u64::from(height),
                    da_layer_block = resp.height,
                    tx_count,
                    "submitted sequencer block to DA layer",
                );
            }
            Err(e) => warn!(error = ?e, "failed to submit block to DA layer"),
        }
        Ok(new_state)
    }

    pub fn run(mut self) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            loop {
                self.interval.tick().await;
                match self.get_and_submit_latest_block().await {
                    Err(e) => warn!(error = ?e, "failed to get latest block from sequencer"),
                    Ok(new_state) if new_state != *self.state.borrow() => {
                        _ = self.state.send_replace(new_state);
                    }
                    Ok(_) => {}
                }
            }
        })
    }
}
