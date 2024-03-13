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

/// A type for tracking the heights within the block stream.
///
/// Exists primarily because calling methods in a pinned type is tedious compared to calling methods
/// on its fields.
struct Heights {
    next_expected_height: Height,
    greatest_requested_height: Option<Height>,
    latest_sequencer_height: Height,
    max_ahead: u64,
}

impl Heights {
    /// Returns the next height to fetch, if any.
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

    /// Sets the latest height observed from sequencer if greater than what was previously set.
    ///
    /// Returns `true` is greater, `false` if not.
    fn set_greatest_if_greater(&mut self, height: Height) -> bool {
        let greater = self
            .greatest_requested_height
            .map_or(true, |old| height > old);
        if greater {
            self.greatest_requested_height.replace(height);
        }
        greater
    }

    /// Sets the latest height observed from sequencer if greater than what was previously set.
    ///
    /// Returns `true` is greater, `false` if not.
    pub(super) fn set_latest_observed_if_greater(&mut self, height: Height) -> bool {
        let greater = height > self.latest_sequencer_height;
        if greater {
            self.latest_sequencer_height = height;
        }
        greater
    }

    /// Sets the latest height expected by the rollup if greater than what was previously set.
    ///
    /// Returns `true` is greater, `false` if not.
    pub(super) fn set_next_expected_if_greater(&mut self, height: Height) -> bool {
        let greater = height > self.next_expected_height;
        if greater {
            self.next_expected_height = height;
        }
        greater
    }
}

pin_project! {
    pub(super) struct BlocksFromHeightStream {
        rollup_id: RollupId,
        heights: Heights,
        in_progress: FuturesMap<Height, eyre::Result<FilteredSequencerBlock>>,
        client: SequencerGrpcClient,
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
            latest_height.recorded = %self.heights.latest_sequencer_height,
        )
    )]
    pub(super) fn set_latest_observed_height_if_greater(&mut self, height: Height) {
        if !self.heights.set_latest_observed_if_greater(height) {
            info!("observed latest sequencer height older than previous; ignoring it");
        }
    }

    /// Records the next expected height expected by the rollup.
    ///
    /// Ignores it if its older than what was previously expected.
    #[instrument(
        skip_all,
        fields(
            next_height.expected = %height,
            next_height.recorded = %self.heights.next_expected_height,
        )
    )]
    pub(super) fn set_next_expected_height_if_greater(&mut self, height: Height) {
        if !self.heights.set_next_expected_if_greater(height) {
            info!("next expected sequencer height older than previous; ignoring it",);
        }
    }

    pub(super) fn new(
        rollup_id: RollupId,
        next_expected_height: Height,
        latest_sequencer_height: Height,
        client: SequencerGrpcClient,
        max_in_flight: usize,
    ) -> Self {
        let heights = Heights {
            next_expected_height,
            latest_sequencer_height,
            greatest_requested_height: None,
            max_ahead: 128,
        };
        Self {
            rollup_id,
            heights,
            in_progress: FuturesMap::new(std::time::Duration::from_secs(10), max_in_flight),
            client,
        }
    }
}

impl Stream for BlocksFromHeightStream {
    type Item = eyre::Result<FilteredSequencerBlock>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        use futures_bounded::PushError;

        let this = self.project();

        // Try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while let Some(next_height) = this.heights.next_height_to_fetch() {
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
            if !this.heights.set_greatest_if_greater(next_height) {
                error!(
                    "attempted to set the greatest requested height, but it was smaller than what \
                     was previously recorded"
                );
            }
        }

        // Attempt to pull the next value from the in_progress_queue
        let (height, res) = futures::ready!(this.in_progress.poll_unpin(cx));

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
        if this.heights.next_height_to_fetch().is_none() {
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
