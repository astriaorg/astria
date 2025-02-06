use astria_telemetry::{
    metric_names,
    metrics::{
        self,
        Counter,
        Gauge,
        IntoF64,
        RegisteringBuilder,
    },
};

const AUCTION_BIDS_PROCESSED_LABEL: &str = "auction_bids_processed";
const AUCTION_BIDS_PROCESSED_ADMITTED: &str = "admitted";
const AUCTION_BIDS_PROCESSED_DROPPED: &str = "dropped";

pub struct Metrics {
    auction_bids_admitted_gauge: Gauge,
    auction_bids_dropped_gauge: Gauge,
    auction_bids_received_count: Counter,
    auctions_cancelled_count: Counter,
    auctions_submitted_count: Counter,
    block_commitments_received_count: Counter,
    executed_blocks_received_count: Counter,
    optimistic_blocks_received_count: Counter,
}

impl Metrics {
    pub(crate) fn increment_auction_bids_admitted_gauge(&self) {
        self.auction_bids_admitted_gauge.increment(1);
    }

    pub(crate) fn increment_auction_bids_received_counter(&self) {
        self.auction_bids_received_count.increment(1);
    }

    pub(crate) fn increment_auctions_cancelled_count(&self) {
        self.auctions_cancelled_count.increment(1);
    }

    pub(crate) fn increment_auctions_submitted_count(&self) {
        self.auctions_submitted_count.increment(1);
    }

    pub(crate) fn increment_block_commitments_received_counter(&self) {
        self.block_commitments_received_count.increment(1);
    }

    pub(crate) fn increment_executed_blocks_received_counter(&self) {
        self.executed_blocks_received_count.increment(1);
    }

    pub(crate) fn increment_optimistic_blocks_received_counter(&self) {
        self.optimistic_blocks_received_count.increment(1);
    }

    pub(crate) fn reset_auction_bids_admitted_gauge(&self) {
        self.auction_bids_admitted_gauge.set(0);
    }

    pub(crate) fn set_auction_bids_dropped_gauge(&self, val: impl IntoF64) {
        self.auction_bids_dropped_gauge.set(val);
    }
}

impl astria_telemetry::metrics::Metrics for Metrics {
    type Config = ();

    fn register(
        builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        let block_commitments_received_count = builder
            .new_counter_factory(
                BLOCK_COMMITMENTS_RECEIVED,
                "the number of block commitments received from the Sequencer node",
            )?
            .register()?;

        let executed_blocks_received_count = builder
            .new_counter_factory(
                EXECUTED_BLOCKS_RECEIVED,
                "the number of executed blocks received from the Rollup node",
            )?
            .register()?;

        let optimistic_blocks_received_count = builder
            .new_counter_factory(
                OPTIMISTIC_BLOCKS_RECEIVED,
                "the number of optimistic blocks received from the Sequencer node",
            )?
            .register()?;

        let auction_bids_received_count = builder
            .new_counter_factory(
                AUCTION_BIDS_RECEIVED,
                "the number of auction bids received from the Rollup node (total number over the \
                 runtime of auctioneer)",
            )?
            .register()?;

        let mut auction_bids_processed_factory = builder.new_gauge_factory(
            AUCTION_BIDS_PROCESSED,
            "the number of auction bids processed during an auction (either admitted or dropped \
             because the time was up or due to some other issue)",
        )?;
        let auction_bids_admitted_gauge =
            auction_bids_processed_factory.register_with_labels(&[(
                AUCTION_BIDS_PROCESSED_LABEL,
                AUCTION_BIDS_PROCESSED_ADMITTED.to_string(),
            )])?;
        let auction_bids_dropped_gauge =
            auction_bids_processed_factory.register_with_labels(&[(
                AUCTION_BIDS_PROCESSED_LABEL,
                AUCTION_BIDS_PROCESSED_DROPPED.to_string(),
            )])?;

        let auctions_cancelled_count = builder
            .new_counter_factory(
                AUCTIONS_CANCELLED,
                "the number of auctions that were cancelled due to a proposed block pre-empting a \
                 prior proposed block",
            )?
            .register()?;

        let auctions_submitted_count = builder
            .new_counter_factory(
                AUCTIONS_SUBMITTED,
                "the number of successfully submitted auctions",
            )?
            .register()?;

        Ok(Self {
            auction_bids_admitted_gauge,
            auction_bids_dropped_gauge,
            auction_bids_received_count,
            auctions_cancelled_count,
            auctions_submitted_count,
            block_commitments_received_count,
            executed_blocks_received_count,
            optimistic_blocks_received_count,
        })
    }
}

metric_names!(const METRICS_NAMES:
    BLOCK_COMMITMENTS_RECEIVED,
    EXECUTED_BLOCKS_RECEIVED,
    OPTIMISTIC_BLOCKS_RECEIVED,
    AUCTIONS_CANCELLED,
    AUCTIONS_SUBMITTED,
    AUCTION_BIDS_PROCESSED,
    AUCTION_BIDS_RECEIVED,
);
