use std::{
    error::Error as StdError,
    pin::Pin,
    task::Poll,
};

use astria_core::sequencer::v1alpha1::{
    block::FilteredSequencerBlock,
    RollupId,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use futures::Stream;
use futures_bounded::FuturesMap;
use pin_project_lite::pin_project;
use sequencer_client::tendermint::block::Height;
use telemetry::display::json;
use tracing::{
    error,
    info,
    instrument,
    warn,
};

use super::SequencerGrpcClient;
use crate::sequencer::reporting::ReportFilteredSequencerBlock;

pin_project! {
    pub(super) struct BlocksFromHeightStream {
        rollup_id: RollupId,
        next_expected_height: Height,
        greatest_requested_height: Option<Height>,
        latest_sequencer_height: Height,
        in_progress: FuturesMap<Height, eyre::Result<FilteredSequencerBlock>>,
        client: SequencerGrpcClient,
        max_ahead: u64,
    }
}

impl BlocksFromHeightStream {
    /// Records the latest height observed from sequencer.
    ///
    /// Ignores it if its older than what was previously observed.
    #[instrument(
        skip_all,
        fields(
            latest_height.observed = %height,
            latest_height.recorded = %self.latest_sequencer_height,
        )
    )]
    pub(super) fn record_latest_height(&mut self, height: Height) {
        if height < self.latest_sequencer_height {
            info!("observed latest sequencer height older than previous; ignoring it",);
        }
        self.latest_sequencer_height = height;
    }

    /// Records the latest height observed from sequencer.
    ///
    /// Ignores it if its older than what was previously observed.
    #[instrument(
        skip_all,
        fields(
            next_height.observed = %height,
            next_height.recorded = %self.next_expected_height,
        )
    )]
    pub(super) fn record_next_expected_height(&mut self, height: Height) {
        if height < self.next_expected_height {
            info!("next expected sequencer height older than previous; ignoring it",);
        }
        self.next_expected_height = height;
    }

    /// The stream can yield more if the greatest requested height isn't too far
    /// ahead of the next expected height and not ahead of the latest observed sequencer height.
    fn next_height_to_fetch(&self) -> Option<Height> {
        let potential_height = match self.greatest_requested_height {
            None => self.next_expected_height,
            Some(greatest_requested_height) => greatest_requested_height.increment(),
        };
        let not_too_far_ahead =
            potential_height.value() < (self.next_expected_height.value() + self.max_ahead);
        let height_exists_on_sequencer = potential_height <= self.latest_sequencer_height;
        if not_too_far_ahead && height_exists_on_sequencer {
            Some(potential_height)
        } else {
            None
        }
    }

    pub(super) fn new(
        rollup_id: RollupId,
        next_expected_height: Height,
        latest_sequencer_height: Height,
        client: SequencerGrpcClient,
        max_in_flight: usize,
    ) -> Self {
        Self {
            rollup_id,
            next_expected_height,
            latest_sequencer_height,
            greatest_requested_height: None,
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), max_in_flight),
            client,
            max_ahead: 128,
        }
    }
}

impl Stream for BlocksFromHeightStream {
    type Item = eyre::Result<FilteredSequencerBlock>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        use futures_bounded::PushError;

        // Try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while let Some(next_height) = self.as_ref().get_ref().next_height_to_fetch() {
            let this = self.as_mut().project();
            match this.in_progress.try_push(
                next_height,
                fetch_block(this.client.clone(), next_height, *this.rollup_id),
            ) {
                Err(PushError::BeyondCapacity(_)) => break,
                Err(PushError::Replaced(_)) => {
                    error!(
                        height = %next_height,
                        "scheduled to fetch block, but a fetch for the same height was already in-flight",
                    );
                }
                Ok(()) => {}
            }
            this.greatest_requested_height.replace(next_height);
        }

        // Attempt to pull the next value from the in_progress_queue
        let (height, res) = futures::ready!(self.as_mut().project().in_progress.poll_unpin(cx));

        // Ok branch (contains the block or a fetch error): propagate the error up
        //
        // Err branch (timeout): a fetch timing out is not a problem: we can just reschedule it.
        match res {
            Ok(fetch_result) => {
                return Poll::Ready(Some(fetch_result.wrap_err_with(|| {
                    format!("failed fetching sequencer block at height `{height}`")
                })));
            }
            Err(timed_out) => {
                warn!(
                    %height,
                    error = &timed_out as &dyn StdError,
                    "request for height timed out, rescheduling",
                );
                let res = {
                    let this = self.as_mut().project();
                    this.in_progress.try_push(
                        height,
                        fetch_block(this.client.clone(), height, *this.rollup_id),
                    )
                };
                assert!(
                    res.is_ok(),
                    "there must be space in the map after a future timed out"
                );
            }
        }

        // We only reach this part if the `futures::ready!` didn't short circuit,
        // if no result was ready.
        if self.as_ref().get_ref().next_height_to_fetch().is_none() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.in_progress.len(), None)
    }
}

#[instrument(
    skip_all,
    fields(%height, %rollup_id),
    err,
)]
async fn fetch_block(
    mut client: SequencerGrpcClient,
    height: Height,
    rollup_id: RollupId,
) -> eyre::Result<FilteredSequencerBlock> {
    let filtered_block = client
        .get(height.value(), rollup_id)
        .await
        .wrap_err("failed fetching filtered sequencer block")?;
    info!(
        block = %json(&ReportFilteredSequencerBlock(&filtered_block)),
        "received block from Sequencer gRPC service",
    );
    Ok(filtered_block)
}

// TODO: Bring these back, but probably on a dedicated `Heights` types tracking the requested
// heights inside the stream. #[cfg(test)]
// mod tests {
//     use futures_bounded::FuturesMap;
//     use sequencer_client::tendermint::block::Height;

//     use super::BlocksFromHeightStream;

//     async fn make_stream() -> BlocksFromHeightStream {
//         let pool = crate::client_provider::mock::TestPool::setup().await;
//         BlocksFromHeightStream {
//             next_expected_height: Height::from(1u32),
//             greatest_requested_height: None,
//             latest_sequencer_height: Height::from(2u32),
//             in_progress: FuturesMap::new(std::time::Duration::from_secs(10), 10),
//             pool: pool.pool.clone(),
//             max_ahead: 3,
//         }
//     }

//     #[tokio::test]
//     async fn stream_next_blocks() {
//         let mut stream = make_stream().await;
//         assert_eq!(
//             Some(stream.next_expected_height),
//             stream.next_height_to_fetch(),
//             "an unset greatest requested height should lead to the next expected height",
//         );

//         stream.greatest_requested_height = Some(Height::from(1u32));
//         assert_eq!(
//             Some(stream.latest_sequencer_height),
//             stream.next_height_to_fetch(),
//             "the greated requested height is right before the latest observed height, which \
//              should give the observed height",
//         );
//         stream.greatest_requested_height = Some(Height::from(2u32));
//         assert!(
//             stream.next_height_to_fetch().is_none(),
//             "the greatest requested height being the latest observed height should give nothing",
//         );
//         stream.greatest_requested_height = Some(Height::from(4u32));
//         stream.latest_sequencer_height = Height::from(5u32);
//         assert!(
//             stream.next_height_to_fetch().is_none(),
//             "a greatest height before the latest observed height but too far ahead of the next \
//              expected height should give nothing",
//         );
//     }
// }
