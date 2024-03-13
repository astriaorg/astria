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
    rollup_expects: u64,
    greatest_requested_height: Option<u64>,
    latest_sequencer_height: u64,
    max_ahead: u64,
}

impl Heights {
    /// Returns the next height to fetch, if any.
    fn next_height_to_fetch(&self) -> Option<u64> {
        let potential_height = match self.greatest_requested_height {
            None => self.rollup_expects,
            Some(greatest_requested_height) => greatest_requested_height.saturating_add(1),
        };
        let not_too_far_ahead =
            potential_height < (self.rollup_expects.saturating_add(self.max_ahead));
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
    fn set_greatest_if_greater(&mut self, height: u64) -> bool {
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
        let height = height.value();
        let greater = height > self.latest_sequencer_height;
        if greater {
            self.latest_sequencer_height = height;
        }
        greater
    }

    /// Sets the latest height expected by the rollup if greater than what was previously set.
    ///
    /// Returns `true` is greater, `false` if not.
    pub(super) fn set_rollup_expects_if_greater(&mut self, height: Height) -> bool {
        let height = height.value();
        let greater = height > self.rollup_expects;
        if greater {
            self.rollup_expects = height;
        }
        greater
    }
}

pin_project! {
    pub(super) struct BlocksFromHeightStream {
        rollup_id: RollupId,
        heights: Heights,
        in_progress: FuturesMap<u64, eyre::Result<FilteredSequencerBlock>>,
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
            rollup_expects.provided = %height,
            rollup_expects.recorded = %self.heights.rollup_expects,
        )
    )]
    pub(super) fn set_next_expected_height_if_greater(&mut self, height: Height) {
        if !self.heights.set_rollup_expects_if_greater(height) {
            info!("next expected sequencer height older than previous; ignoring it",);
        }
    }

    pub(super) fn new(
        rollup_id: RollupId,
        rollup_expects: Height,
        latest_sequencer_height: Height,
        client: SequencerGrpcClient,
    ) -> Self {
        let heights = Heights {
            rollup_expects: rollup_expects.value(),
            latest_sequencer_height: latest_sequencer_height.value(),
            greatest_requested_height: None,
            max_ahead: 128,
        };
        Self {
            rollup_id,
            heights,
            // NOTE: Gives Sequencer 1h to respond, and hard code it to use 20 max in flight
            // requests. XXX: This interacts with the retry-logic in the
            // `SequencerGrpcClient::get` method. We shoud probably remove this
            // FuturesMap in favor of a plain FuturesUnordered and let the client handle
            // retries.
            in_progress: FuturesMap::new(std::time::Duration::from_secs(3600), 20),
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
                    "attempted to set the greatest requested height to the one just obtained, but \
                     it was smaller than what was previously recorded"
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
    height: u64,
    rollup_id: RollupId,
) -> eyre::Result<FilteredSequencerBlock> {
    let filtered_block = client
        .get(height, rollup_id)
        .await
        .wrap_err("failed fetching filtered sequencer block")?;
    info!(
        block = %json(&ReportFilteredSequencerBlock(&filtered_block)),
        "received block from Sequencer gRPC service",
    );
    Ok(filtered_block)
}

#[cfg(test)]
mod tests {
    use super::Heights;

    #[test]
    fn next_gives_what_rollup_expects_if_fresh() {
        let mut heights = Heights {
            rollup_expects: 5,
            greatest_requested_height: None,
            latest_sequencer_height: 6,
            max_ahead: 3,
        };
        let next = heights.next_height_to_fetch();
        assert_eq!(
            Some(5),
            next,
            "the next height exists and should be the same as what the rollup expects"
        );
        assert!(
            heights.set_greatest_if_greater(next.unwrap()),
            "a fresh heights tracker should be updateable"
        );
        assert_eq!(
            Some(6),
            heights.next_height_to_fetch(),
            "the height after what the rollup expects should be one more",
        );
    }

    #[test]
    fn next_height_is_none_if_too_far_ahead() {
        let heights = Heights {
            rollup_expects: 4,
            greatest_requested_height: Some(5),
            latest_sequencer_height: 6,
            max_ahead: 2,
        };
        let next = heights.next_height_to_fetch();
        assert_eq!(None, next);
    }

    #[test]
    fn next_height_is_none_if_at_sequencer_head() {
        let heights = Heights {
            rollup_expects: 4,
            greatest_requested_height: Some(5),
            latest_sequencer_height: 5,
            max_ahead: 2,
        };
        let next = heights.next_height_to_fetch();
        assert_eq!(None, next);
    }
}
