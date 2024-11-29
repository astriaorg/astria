use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use futures::{
    Future,
    FutureExt,
};
use tokio::task::JoinHandle;
use tracing::{
    info,
    instrument,
    warn,
};

use crate::{
    block::Commitment,
    bundle::Bundle,
    flatten_join_result,
};

pub(in crate::auctioneer::inner) struct Running {
    pub(super) id: super::Id,
    pub(super) height: u64,
    pub(super) parent_block_of_executed: Option<[u8; 32]>,
    // TODO: Rename this to AuctionSender or smth like that
    pub(super) sender: super::Handle,
    pub(super) task: JoinHandle<eyre::Result<()>>,
}

impl Running {
    pub(in crate::auctioneer::inner) fn abort(&self) {
        self.task.abort();
    }

    #[instrument(skip(self))]
    // pub(in crate::auctioneer::inner) fn start_timer(&mut self, auction_id: Id) ->
    // eyre::Result<()> {
    pub(in crate::auctioneer::inner) fn start_timer(
        &mut self,
        block_commitment: Commitment,
    ) -> eyre::Result<()> {
        let id_according_to_block =
            super::Id::from_sequencer_block_hash(block_commitment.sequencer_block_hash());

        if self.id == id_according_to_block && block_commitment.sequencer_height() == self.height {
            self.sender
                .start_timer()
                .wrap_err("failed to send command to start timer to auction")?;
        } else {
            // TODO: provide better information on the blocks/currently running auction.
            // warn!(
            //     current_block.sequencer_block_hash =
            // %base64(self.current_block.sequencer_block_hash()),
            //     block_commitment.sequencer_block_hash =
            // %base64(block_commitment.sequencer_block_hash()),     "received
            // block commitment for the wrong block" );
            info!(
                "not starting the auction timer because sequencer block hash and height of the \
                 commitment did not match that of the running auction",
            );
        }

        Ok(())
    }

    #[instrument(skip(self))]
    // pub(in crate::auctioneer::inner) fn start_processing_bids(&mut self, auction_id: Id) ->
    // eyre::Result<()> {
    pub(in crate::auctioneer::inner) fn start_processing_bids(
        &mut self,
        block: crate::block::Executed,
    ) -> eyre::Result<()> {
        let id_according_to_block =
            super::Id::from_sequencer_block_hash(block.sequencer_block_hash());

        if self.id == id_according_to_block {
            // TODO: What if it was already set? Overwrite? Replace? Drop?
            let _ = self
                .parent_block_of_executed
                .replace(block.parent_rollup_block_hash());
            self.sender
                .start_processing_bids()
                .wrap_err("failed to send command to start processing bids")?;
        } else {
            // TODO: bring back the fields to track the dropped block and current block
            // warn!(
            //     // TODO: nicer display for the current block
            //     current_block.sequencer_block_hash =
            // %base64(self.current_block.sequencer_block_hash()),
            //     executed_block.sequencer_block_hash =
            // %base64(executed_block.sequencer_block_hash()),
            //     executed_block.rollup_block_hash =
            // %base64(executed_block.rollup_block_hash()),     "received
            // optimistic execution result for wrong sequencer block" );
            warn!(
                "not starting to process bids in the current auction because we received an \
                 executed block from the rollup with a sequencer block hash that does not match \
                 that of the currently running auction; dropping the executed block"
            );
        }

        Ok(())
    }

    pub(in crate::auctioneer::inner) fn forward_bundle_to_auction(
        &mut self,
        bundle: Bundle,
    ) -> eyre::Result<()> {
        let id_according_to_bundle =
            super::Id::from_sequencer_block_hash(bundle.base_sequencer_block_hash());

        // TODO: emit some more information about auctoin ID, expected vs actual parent block hash,
        // tacked block hash, provided block hash, etc.
        let Some(parent_block_of_executed) = self.parent_block_of_executed else {
            eyre::bail!(
                "received a new bundle but the current auction has not yet
                    received an execute block from the rollup; dropping the bundle"
            );
        };
        let ids_match = self.id == id_according_to_bundle;
        let parent_blocks_match = parent_block_of_executed == bundle.parent_rollup_block_hash();
        if ids_match && parent_blocks_match {
            self.sender
                .try_send_bundle(bundle)
                .wrap_err("failed to add bundle to auction")?;
        } else {
            warn!(
                // TODO: Add these fields back in. Is it even necessary to return the error?
                // Can't we just fire the event here? necessary?
                //
                // curent_block.sequencer_block_hash = %base64(self.
                // current_block.sequencer_block_hash()),
                // bundle.sequencer_block_hash = %base64(bundle.base_sequencer_block_hash()),
                // bundle.parent_rollup_block_hash =
                // %base64(bundle.parent_rollup_block_hash()),
                "incoming bundle does not match current block, ignoring"
            );
            eyre::bail!("auction ID and ID according to bundle don't match");
        }
        Ok(())
    }
}

impl Future for Running {
    type Output = eyre::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let res = std::task::ready!(self.task.poll_unpin(cx));
        std::task::Poll::Ready(flatten_join_result(res))
    }
}
