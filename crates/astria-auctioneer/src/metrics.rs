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

const BIDS_PER_AUCTIONLABEL: &str = "kind";
const AUCTION_BIDS_PROCESSED: &str = "processed";
const AUCTION_BIDS_DROPPED: &str = "dropped";

const AUCTION_WINNER_SUBMISSION_LATENCY_LABEL: &str = "auction_winner_submission_latency";
const AUCTION_WINNER_ERROR: &str = "error";
const AUCTION_WINNER_SUCCESS: &str = "success";

const ORDER_SIMULATION_LATENCY_LABEL: &str = "order_simulation_latency";
const ORDER_SIMULATION_ERROR: &str = "error";
const ORDER_SIMULATION_SUCCESS: &str = "success";

pub struct Metrics {
    bids_per_auction_dropped_histogram: Histogram,
    bids_per_auction_processed_histogram: Histogram,
    in_time_order_simulations: Counter,
    late_order_simulations: Counter,
    auction_winner_submission_error_latency: Histogram,
    auction_winner_submission_success_latency: Histogram,
    auction_winning_bid_histogram: Histogram,
    auctions_cancelled_count: Counter,
    block_commitments_received_count: Counter,
    executed_blocks_received_count: Counter,
    proposed_blocks_received_count: Counter,
    order_simulation_success_latency: Histogram,
    order_simulation_failure_latency: Histogram,
}

impl Metrics {
    pub(crate) fn increment_in_time_order_simulations(&self) {
        self.in_time_order_simulations.increment(1);
    }

    pub(crate) fn increment_auctions_cancelled_count(&self) {
        self.auctions_cancelled_count.increment(1);
    }

    pub(crate) fn increment_late_order_simulations(&self) {
        self.late_order_simulations.increment(1);
    }

    pub(crate) fn increment_block_commitments_received_counter(&self) {
        self.block_commitments_received_count.increment(1);
    }

    pub(crate) fn increment_executed_blocks_received_counter(&self) {
        self.executed_blocks_received_count.increment(1);
    }

    pub(crate) fn increment_proposed_blocks_received_counter(&self) {
        self.proposed_blocks_received_count.increment(1);
    }

    pub(crate) fn record_bids_per_auction_dropped_histogram(&self, val: impl IntoF64) {
        self.bids_per_auction_dropped_histogram.record(val);
    }

    pub(crate) fn record_bids_per_auction_processed_histogram(&self, val: impl IntoF64) {
        self.bids_per_auction_processed_histogram.record(val);
    }

    pub(crate) fn record_auction_winning_bid_histogram(&self, val: impl IntoF64) {
        self.auction_winning_bid_histogram.record(val);
    }

    pub(crate) fn record_auction_winner_submission_error_latency(&self, val: impl IntoF64) {
        self.auction_winner_submission_error_latency.record(val);
    }

    pub(crate) fn record_auction_winner_submission_success_latency(&self, val: impl IntoF64) {
        self.auction_winner_submission_success_latency.record(val);
    }

    pub(crate) fn record_order_simulation_success_latency(&self, val: impl IntoF64) {
        self.order_simulation_success_latency.record(val)
    }

    pub(crate) fn record_order_simulation_failure_latency(&self, val: impl IntoF64) {
        self.order_simulation_failure_latency.record(val)
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

        let proposed_blocks_received_count = builder
            .new_counter_factory(
                PROPOSED_BLOCKS_RECEIVED,
                "the number of proposed blocks received from the Sequencer node",
            )?
            .register()?;

        let in_time_order_simulations = builder
            .new_counter_factory(
                ORDERS_SIMULATED_IN_TIME,
                "the number of order simulations that were performed in-time, i.e. before an \
                 auction ended or was cancelled",
            )?
            .register()?;

        let mut auction_bids_factory = builder.new_histogram_factory(
            AUCTION_BIDS_PROCESSED,
            "the number of auction bids received during an auction (either admitted or dropped \
             because the time was up or due to some other issue)",
        )?;
        let bids_per_auction_processed_histogram = auction_bids_factory
            .register_with_labels(&[(BIDS_PER_AUCTIONLABEL, AUCTION_BIDS_PROCESSED.to_string())])?;
        let bids_per_auction_dropped_histogram = auction_bids_factory
            .register_with_labels(&[(BIDS_PER_AUCTIONLABEL, AUCTION_BIDS_DROPPED.to_string())])?;

        let auctions_cancelled_count = builder
            .new_counter_factory(
                AUCTIONS_CANCELLED,
                "the number of auctions that were cancelled due to a proposed block pre-empting a \
                 prior proposed block",
            )?
            .register()?;

        let auction_winning_bid_histogram = builder
            .new_histogram_factory(AUCTION_WINNING_BID, "the amount bid by the auction winner")?
            .register()?;

        let mut auction_winner_submission_latency_factory = builder.new_histogram_factory(
            AUCTION_WINNER_SUBMISSION_LATENCY,
            "the duration for Sequencer to respond to a auction submission",
        )?;

        let auction_winner_submission_error_latency = auction_winner_submission_latency_factory
            .register_with_labels(&[(
                AUCTION_WINNER_SUBMISSION_LATENCY_LABEL,
                AUCTION_WINNER_ERROR.to_string(),
            )])?;

        let auction_winner_submission_success_latency = auction_winner_submission_latency_factory
            .register_with_labels(&[(
            AUCTION_WINNER_SUBMISSION_LATENCY_LABEL,
            AUCTION_WINNER_SUCCESS.to_string(),
        )])?;

        let late_order_simulations = builder
            .new_counter_factory(
                ORDER_SIMULATIONS_WITHOUT_MATCHING_AUCTION,
                "simulations of orders that returned after an auction was already finished or \
                 cancelled",
            )?
            .register()?;

        let mut order_simulations_latency_factory = builder.new_histogram_factory(
            ORDER_SIMULATION_LATENCY,
            "the duration for the rollup to respond to an eth_simulateV1 from the start of an \
             auction",
        )?;
        let order_simulation_success_latency = order_simulations_latency_factory
            .register_with_labels(&[(
                ORDER_SIMULATION_LATENCY_LABEL,
                ORDER_SIMULATION_SUCCESS.to_string(),
            )])?;
        let order_simulation_failure_latency = order_simulations_latency_factory
            .register_with_labels(&[(
                ORDER_SIMULATION_LATENCY_LABEL,
                ORDER_SIMULATION_ERROR.to_string(),
            )])?;
        Ok(Self {
            bids_per_auction_dropped_histogram,
            bids_per_auction_processed_histogram,
            in_time_order_simulations,
            late_order_simulations,
            auction_winner_submission_error_latency,
            auction_winner_submission_success_latency,
            auction_winning_bid_histogram,
            auctions_cancelled_count,
            block_commitments_received_count,
            executed_blocks_received_count,
            proposed_blocks_received_count,
            order_simulation_success_latency,
            order_simulation_failure_latency,
        })
    }
}

metric_names!(const METRICS_NAMES:
    BLOCK_COMMITMENTS_RECEIVED,
    EXECUTED_BLOCKS_RECEIVED,
    PROPOSED_BLOCKS_RECEIVED,
    AUCTIONS_CANCELLED,
    AUCTION_SIMULATION_DELAY_SINCE_START,
    BIDS_PER_AUCTION,
    ORDERS_SIMULATED_IN_TIME,
    ORDER_SIMULATIONS_WITHOUT_MATCHING_AUCTION,
    ORDER_SIMULATION_LATENCY,
    AUCTION_WINNING_BID,
    AUCTION_WINNER_SUBMISSION_LATENCY,
);
