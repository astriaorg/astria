use std::time::Duration;

use astria_core::{
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::SequenceAction,
};
use astria_eyre::{
    eyre,
    eyre::{
        eyre,
        WrapErr as _,
    },
};
use ed25519_consensus::SigningKey;
use secrecy::{
    ExposeSecret,
    SecretString,
    Zeroize,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::{
    executor,
    executor::Status,
};

pub(crate) struct Builder {
    pub(crate) sequencer_url: String,
    pub(crate) sequencer_chain_id: String,
    pub(crate) private_key: SecretString,
    pub(crate) block_time_ms: u64,
    pub(crate) max_bytes_per_bundle: usize,
    pub(crate) shutdown_token: CancellationToken,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<(super::Executor, executor::Handle)> {
        let Self {
            sequencer_url,
            sequencer_chain_id,
            private_key,
            block_time_ms,
            max_bytes_per_bundle,
            shutdown_token,
        } = self;
        let sequencer_client = sequencer_client::HttpClient::new(sequencer_url.as_str())
            .wrap_err("failed constructing sequencer client")?;
        let (status, _) = watch::channel(Status::new());
        let mut private_key_bytes: [u8; 32] = hex::decode(private_key.expose_secret())
            .wrap_err("failed to decode private key bytes from hex string")?
            .try_into()
            .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
        let sequencer_key = SigningKey::from(private_key_bytes);
        private_key_bytes.zeroize();

        let sequencer_address = Address::from_verification_key(sequencer_key.verification_key());

        let (serialized_rollup_transaction_tx, serialized_rollup_transaction_rx) =
            tokio::sync::mpsc::channel::<SequenceAction>(256);

        Ok((
            super::Executor {
                status,
                serialized_rollup_transactions: serialized_rollup_transaction_rx,
                sequencer_client,
                sequencer_chain_id,
                sequencer_key,
                address: sequencer_address,
                block_time: Duration::from_millis(block_time_ms),
                max_bytes_per_bundle,
                shutdown_token,
            },
            executor::Handle::new(serialized_rollup_transaction_tx),
        ))
    }
}
