use std::{
    fs,
    path::Path,
    time::Duration,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::Address,
    protocol::transaction::v1alpha1::action::SequenceAction,
};
use astria_eyre::eyre::{
    self,
    eyre,
    WrapErr as _,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::{
    executor,
    executor::Status,
    metrics::Metrics,
};

pub(crate) struct Builder {
    pub(crate) sequencer_url: String,
    pub(crate) sequencer_chain_id: String,
    pub(crate) private_key_file: String,
    pub(crate) sequencer_address_prefix: String,
    pub(crate) block_time_ms: u64,
    pub(crate) max_bytes_per_bundle: usize,
    pub(crate) bundle_queue_capacity: usize,
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) metrics: &'static Metrics,
}

impl Builder {
    pub(crate) fn build(self) -> eyre::Result<(super::Executor, executor::Handle)> {
        let Self {
            sequencer_url,
            sequencer_chain_id,
            private_key_file,
            sequencer_address_prefix,
            block_time_ms,
            max_bytes_per_bundle,
            bundle_queue_capacity,
            shutdown_token,
            metrics,
        } = self;
        let sequencer_client = sequencer_client::HttpClient::new(sequencer_url.as_str())
            .wrap_err("failed constructing sequencer client")?;
        let (status, _) = watch::channel(Status::new());

        let sequencer_key = read_signing_key_from_file(&private_key_file).wrap_err_with(|| {
            format!("failed reading signing key from file at path `{private_key_file}`")
        })?;

        let sequencer_address = Address::builder()
            .prefix(sequencer_address_prefix)
            .array(*sequencer_key.verification_key().address_bytes())
            .try_build()
            .wrap_err("failed constructing a sequencer address from private key")?;

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
                bundle_queue_capacity,
                shutdown_token,
                metrics,
            },
            executor::Handle::new(serialized_rollup_transaction_tx),
        ))
    }
}

fn read_signing_key_from_file<P: AsRef<Path>>(path: P) -> eyre::Result<SigningKey> {
    let private_key_hex = fs::read_to_string(path)?;
    let private_key_bytes: [u8; 32] = hex::decode(private_key_hex.trim())?
        .try_into()
        .map_err(|_| eyre!("invalid private key length; must be 32 bytes"))?;
    Ok(SigningKey::from(private_key_bytes))
}
