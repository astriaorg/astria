use std::time::Duration;

use astria_core::protocol::transaction::v1alpha1::SignedTransaction;
use astria_eyre::eyre::{
    self,
    eyre,
    Context,
};
use tokio::{
    select,
    sync::{
        mpsc,
        oneshot,
    },
};
use tokio_util::sync::CancellationToken;

use super::{
    bid::Bid,
    Auction,
};
use crate::Metrics;

pub(crate) struct Handle {
    executed_block_tx: Option<oneshot::Sender<()>>,
    block_commitments_tx: Option<oneshot::Sender<()>>,
    reorg_tx: Option<oneshot::Sender<()>>,
}

impl Handle {
    pub(crate) async fn send_bundle(&self) -> eyre::Result<()> {
        unimplemented!()
    }

    pub(crate) fn executed_block(&mut self) -> eyre::Result<()> {
        let _ = self
            .executed_block_tx
            .take()
            .expect("should only send executed signal to a given auction once")
            .send(());
        Ok(())
    }

    pub(crate) fn block_commitment(&mut self) -> eyre::Result<()> {
        let _ = self
            .block_commitments_tx
            .take()
            .expect("should only send block commitment signal to a given auction once")
            .send(());

        Ok(())
    }

    pub(crate) fn reorg(&mut self) -> eyre::Result<()> {
        let _ = self
            .reorg_tx
            .take()
            .expect("should only send reorg signal to a given auction once");

        Ok(())
    }
}

pub(crate) struct Driver {
    #[allow(dead_code)]
    metrics: &'static Metrics,
    shutdown_token: CancellationToken,

    /// The time between receiving a block commitment
    latency_margin: Duration,
    /// Channel for receiving the executed block signal to start processing bundles
    executed_block_rx: oneshot::Receiver<()>,
    /// Channel for receiving the block commitment signal to start the latency margin timer
    block_commitments_rx: oneshot::Receiver<()>,
    /// Channel for receiving the reorg signal
    reorg_rx: oneshot::Receiver<()>,
    /// Channel for receiving new bundles
    new_bids_rx: mpsc::Receiver<Bid>,
    /// The current state of the auction
    auction: Auction,
}

impl Driver {
    pub(crate) async fn run(mut self) -> eyre::Result<()> {
        // TODO: should the timer be inside the auction so that we only have one option?
        let mut latency_margin_timer = None;
        let mut auction: Option<Auction> = None;

        let mut nonce_fetch: Option<tokio::task::JoinHandle<eyre::Result<u64>>> = None;

        let auction_result = loop {
            select! {
                biased;

                () = self.shutdown_token.cancelled() => break Err(eyre!("received shutdown signal")),

                signal = &mut self.reorg_rx => {
                    match signal {
                        Ok(()) => {
                            break Err(eyre!("reorg signal received"))
                        }
                        Err(_) => {
                            return Err(eyre!("reorg signal channel closed"));
                        }
                    }
                    //
                }

                // get the auction winner when the timer expires
                // TODO: should this also be conditioned on auction.is_some()? this feels redundant as we only populate the timer if the auction isnt none
                _ = async { latency_margin_timer.as_mut().unwrap() }, if latency_margin_timer.is_some() => {
                    break Ok(auction.unwrap().winner());
                }

                signal = &mut self.executed_block_rx, if auction.is_none() => {
                    if let Err(e) = signal {
                        break Err(eyre!("exec signal channel closed")).wrap_err(e);
                    }
                    // set auction to open so it starts collecting bids
                    auction = Some(Auction::new());
                }

                signal = &mut self.block_commitments_rx, if auction.is_some() => {
                    if let Err(e) = signal {
                        break Err(eyre!("commit signal channel closed")).wrap_err(e);
                    }
                    // set the timer
                    latency_margin_timer = Some(tokio::time::sleep(self.latency_margin));

                    // TODO: also want to fetch the pending nonce here (we wait for commit because we want the pending nonce from after the commit)
                    nonce_fetch = Some(tokio::task::spawn(async {
                        // TODO: fetch the pending nonce using the sequencer client with tryhard
                        Ok(0)
                    }));
                }

                //  TODO: new bundles from the bundle stream if auction exists?
                //      - add the bid to the auction if executed

            }
            // submit the auction result to the sequencer/wait for cancellation signal
            //  1. result from submit_fut if !submission.terminated()
        };

        // handle auction result
        let transaction = match auction_result {
            Ok(winner) => winner.into_transaction(),
            Err(e) => {
                return Err(e);
            }
        };

        // await the nonce fetch result
        // TODO: flatten this or get rid of the option somehow
        let nonce = nonce_fetch
            .expect("should have received commit to exit the bid loop")
            .await
            .wrap_err("task failed")?
            .wrap_err("failed to fetch nonce")?;

        let submission_result = select! {
            biased;

            // TODO: should this be Ok() or something?
            () = self.shutdown_token.cancelled() => Err(eyre!("received shutdown signal")),

            // submit the transaction to the sequencer
            result = self.submit_transaction(transaction) => {
                // TODO: handle submission failure better?
                result
            }
        };

        submission_result
    }

    async fn submit_transaction(&self, _transaction: SignedTransaction) -> eyre::Result<()> {
        unimplemented!()
    }
}
