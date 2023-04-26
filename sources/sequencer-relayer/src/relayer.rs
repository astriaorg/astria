use bech32::{self, ToBase32, Variant};
use serde::Deserialize;
use std::str::FromStr;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::base64_string::Base64String;
use crate::da::CelestiaClient;
use crate::keys::{private_key_bytes_to_keypair, validator_hex_to_address};
use crate::sequencer::SequencerClient;
use crate::sequencer_block::SequencerBlock;

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
    keypair: ed25519_dalek::Keypair,
    validator_address: String,
    validator_address_bytes: Vec<u8>,

    state: watch::Sender<State>,
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
    pub fn new(
        sequencer_client: SequencerClient,
        da_client: CelestiaClient,
        key_file: ValidatorPrivateKeyFile,
    ) -> Self {
        // generate our private-public keypair
        let keypair = private_key_bytes_to_keypair(
            &Base64String::from_string(key_file.priv_key.value)
                .expect("failed to decode validator private key; must be base64 string")
                .0,
        )
        .expect("failed to convert validator private key to keypair");

        // generate our bech32 validator address
        let validator_address = validator_hex_to_address(&key_file.address)
            .expect("failed to convert validator address to bech32");

        // generate our validator address bytes
        let validator_address_bytes = hex::decode(&key_file.address)
            .expect("failed to decode validator address; must be hex string");

        let (state, _) = watch::channel(State::default());

        Self {
            sequencer_client,
            da_client,
            keypair,
            validator_address,
            validator_address_bytes,
            state,
        }
    }

    pub fn subscribe_to_state(&self) -> watch::Receiver<State> {
        self.state.subscribe()
    }

    async fn get_latest_block(&self) -> eyre::Result<State> {
        let mut new_state = (*self.state.borrow()).clone();
        let resp = self.sequencer_client.get_latest_block().await?;

        let maybe_height: Result<u64, <u64 as FromStr>::Err> = resp.block.header.height.parse();
        if let Err(e) = maybe_height {
            warn!(
                error = ?e,
                "got invalid block height {} from sequencer",
                resp.block.header.height,
            );
            return Ok(new_state);
        }

        let height = maybe_height.unwrap();
        if height <= *new_state.current_sequencer_height.get_or_insert(height) {
            return Ok(new_state);
        }

        info!("got block with height {} from sequencer", height);
        new_state.current_sequencer_height.replace(height);

        if resp.block.header.proposer_address.0 != self.validator_address_bytes {
            let proposer_address = bech32::encode(
                "metrovalcons",
                resp.block.header.proposer_address.0.to_base32(),
                Variant::Bech32,
            )
            .expect("should encode block proposer address");
            info!(
                %proposer_address,
                validator_address = %self.validator_address,
                "ignoring block: proposer address is not ours",
            );
            return Ok(new_state);
        }

        let sequencer_block = match SequencerBlock::from_cosmos_block(resp.block) {
            Ok(block) => block,
            Err(e) => {
                warn!(error = ?e, "failed to convert block to DA block");
                return Ok(new_state);
            }
        };

        let tx_count = sequencer_block.rollup_txs.len() + sequencer_block.sequencer_txs.len();
        match self
            .da_client
            .submit_block(sequencer_block, &self.keypair)
            .await
        {
            Ok(resp) => {
                new_state
                    .current_data_availability_height
                    .replace(resp.height);
                info!(
                    sequencer_block = height,
                    da_layer_block = resp.height,
                    tx_count,
                    "submitted sequencer block to DA layer",
                );
            }
            Err(e) => warn!(error = ?e, "failed to submit block to DA layer"),
        }
        Ok(new_state)
    }

    pub async fn run(&self) {
        match self.get_latest_block().await {
            Err(e) => warn!(error = ?e, "failed to get latest block from sequencer"),
            Ok(new_state) if new_state != *self.state.borrow() => {
                _ = self.state.send_replace(new_state);
            }
            Ok(_) => {}
        }
    }
}
