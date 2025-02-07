use astria_telemetry::{
    metric_names,
    metrics::{
        self,
        Counter,
        Histogram,
        IntoF64,
        RegisteringBuilder,
    },
};

const AUCTION_BIDS_LABEL: &str = "auction_bids";
const AUCTION_BIDS_PROCESSED: &str = "processed";
const AUCTION_BIDS_DROPPED: &str = "dropped";

pub struct Metrics {
    auction_bid_delay_since_start: Histogram,
    auction_bids_dropped_histogram: Histogram,
    auction_bids_processed_histogram: Histogram,
    auction_bids_received_count: Counter,
    auction_winning_bid_histogram: Histogram,
    auctions_cancelled_count: Counter,
    auctions_submitted_count: Counter,
    block_commitments_received_count: Counter,
    executed_blocks_received_count: Counter,
    optimistic_blocks_received_count: Counter,
}

impl Metrics {
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

    pub(crate) fn record_auction_bids_processed_histogram(&self, val: impl IntoF64) {
        self.auction_bids_processed_histogram.record(val);
    }

    pub(crate) fn record_auction_bids_dropped_histogram(&self, val: impl IntoF64) {
        self.auction_bids_dropped_histogram.record(val);
    }

    pub(crate) fn record_auction_bid_delay_since_start(&self, val: impl IntoF64) {
        self.auction_bid_delay_since_start.record(val);
    }

    pub(crate) fn record_auction_winning_bid_histogram(&self, val: impl IntoF64) {
        self.auction_winning_bid_histogram.record(val);
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

        let mut auction_bids_factory = builder.new_histogram_factory(
            AUCTION_BIDS_PROCESSED,
            "the number of auction bids received during an auction (either admitted or dropped \
             because the time was up or due to some other issue)",
        )?;
        let auction_bids_processed_histogram = auction_bids_factory
            .register_with_labels(&[(AUCTION_BIDS_LABEL, AUCTION_BIDS_PROCESSED.to_string())])?;
        let auction_bids_dropped_histogram = auction_bids_factory
            .register_with_labels(&[(AUCTION_BIDS_LABEL, AUCTION_BIDS_DROPPED.to_string())])?;

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

        let auction_winning_bid_histogram = builder
            .new_histogram_factory(AUCTION_WINNING_BID, "the amount bid by the auction winner")?
            .register()?;

        let auction_bid_delay_since_start = builder
            .new_histogram_factory(
                AUCTION_BID_DELAY_SINCE_START,
                "the duration from the start of an auction to when a bid for that auction was \
                 received",
            )?
            .register()?;

        Ok(Self {
            auction_bid_delay_since_start,
            auction_bids_dropped_histogram,
            auction_bids_processed_histogram,
            auction_bids_received_count,
            auction_winning_bid_histogram,
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
    AUCTION_BID_DELAY_SINCE_START,
    AUCTION_BIDS,
    AUCTION_BIDS_RECEIVED,
    AUCTION_WINNING_BID,
);
