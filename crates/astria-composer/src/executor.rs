use std::time::Duration;

use astria_sequencer::{
    sequence::Action as SequenceAction,
    transaction::{
        Action as SequencerAction, Signed as SequencerTxSigned, Unsigned as UnsignedSequencerTx,
    },
};
use color_eyre::eyre::Context;
use sequencer_client::{Nonce as SequencerNonce, SequencerClientExt};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::ds::{RollupTx, RollupTxExt, SequencerClient, WireFormat};
use color_eyre::eyre;

/// The Executor module executes the strategies that are forwarded to it by the bundler
pub(crate) struct Executor {
    seq_client: SequencerClient,
    seq_tx_recv_channel: UnboundedReceiver<Vec<RollupTxExt>>,
}

impl Executor {
    pub(crate) async fn new(
        seq_url: &str,
        seq_tx_recv_channel: UnboundedReceiver<Vec<RollupTxExt>>,
    ) -> Result<Self, eyre::Error> {
        let seq_client = SequencerClient::new(seq_url)
        .wrap_err("Failed to initialize Sequencer Client")?;

        seq_client.wait_for_sequencer(5, Duration::from_secs(5), 2.0).await?;

        Ok(Self {
            seq_client: seq_client,
            seq_tx_recv_channel,
        })
    }

    async fn sign_bundle(bundle: Vec<RollupTxExt>) -> Result<SequencerTxSigned, eyre::Error> {
        // FIXME(https://github.com/astriaorg/astria/issues/215): need to set and track
        // nonces for an actual fixed funded key/account.
        // For now, each transaction is transmitted from a new account with nonce 0
        let sequencer_key = ed25519_consensus::SigningKey::new(rand::thread_rng());
        let nonce = SequencerNonce::from(0);

        // Transform the rollup transactions to sequencer actions
        let actions: Vec<SequencerAction> = bundle
            .into_iter()
            .map(|(tx, chain_id)| {
                let bytes = match tx {
                    RollupTx::EthersTx(tx) => tx.serialize(),
                    _ => unreachable!(),
                };

                SequencerAction::SequenceAction(SequenceAction::new(
                    chain_id.into_bytes(),
                    bytes.to_vec(),
                ))
            })
            .collect();

        let unsigned_sequencer_tx = UnsignedSequencerTx::new_with_actions(nonce, actions);

        Ok(unsigned_sequencer_tx.into_signed(&sequencer_key))
    }

    async fn sign_and_submit_bundle(
        &mut self,
        bundle: Vec<RollupTxExt>,
    ) -> Result<(), eyre::Error> {
        let signed = Self::sign_bundle(bundle).await?;
        self.seq_client
            .inner
            .submit_transaction_sync(signed)
            .await
            .wrap_err("Failed to submit sequencer transaction")?;

        Ok(())
    }

    pub(crate) async fn start(&mut self) -> Result<(), eyre::Error> {
        while let Some(bundle) = self.seq_tx_recv_channel.recv().await {
            // NOTE: because nonces are serialized, each submission is blocked by the success
            //       of the previous submission
            self.sign_and_submit_bundle(bundle).await?;
        }

        Ok(())
    }
}
