use crate::ds::{RollupTx, RollupTxExt, WireFormat};
use astria_sequencer::{
    sequence::Action as SequenceAction,
    transaction::{
        Action as SequencerAction, Signed as SignedSequencerTx, Unsigned as UnsignedSequencerTx,
    },
};
use color_eyre::eyre::{self, Context};
use sequencer_client::Nonce as SequencerNonce;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use tokio::sync::mpsc as tokio_mpsc;

pub(crate) struct Strategy {
    collector_receiver: UnboundedReceiver<RollupTxExt>,
    sequencer_sender: UnboundedSender<SignedSequencerTx>,
}

impl Strategy {
    pub(crate) fn new(
        collector_receiver: UnboundedReceiver<RollupTxExt>,
    ) -> (Self, UnboundedReceiver<SignedSequencerTx>) {
        let (stream, sink) = tokio_mpsc::unbounded_channel::<SignedSequencerTx>();
        let new_strategy = Self {
            collector_receiver,
            sequencer_sender: stream,
        };
        (new_strategy, sink)
    }

    fn handle_pending_tx(
        &mut self,
        rollup_txs: Vec<RollupTxExt>,
    ) -> Result<(SignedSequencerTx), eyre::Error> {
        // FIXME(https://github.com/astriaorg/astria/issues/215): need to set and track
        // nonces for an actual fixed funded key/account.
        // For now, each transaction is transmitted from a new account with nonce 0
        let sequencer_key = ed25519_consensus::SigningKey::new(rand::thread_rng());
        let nonce = SequencerNonce::from(0);

        // Transform the rollup transaction to sequencer actions
        let actions: Vec<SequencerAction> = rollup_txs
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

    pub(crate) async fn start(&mut self) -> Result<(), eyre::Error> {
        // For the default strategy, you just pay for every rollup tx received, sign it and wrap it into one
        // sequencer tx. Then you send it over to the block builders
        while let Some(rollup_tx) = self.collector_receiver.recv().await {
            let signed_tx = self.handle_pending_tx(vec![rollup_tx])?;
            self.sequencer_sender
                .send(signed_tx)
                .wrap_err("Failed to forward signed sequencer tx from strategy to builder")?;
        }

        Ok(())
    }
}
