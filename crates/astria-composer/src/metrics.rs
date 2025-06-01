use std::{
    collections::HashMap,
    time::Duration,
};

use astria_core::primitive::v1::RollupId;
use telemetry::{
    metric_names,
    metrics::{
        Counter,
        Error,
        Gauge,
        Histogram,
        Recorder,
        RegisteringBuilder,
    },
};
use tracing::error;

type GethCounters = HashMap<String, Counter>;
type GrpcCounters = HashMap<RollupId, Counter>;

const ROLLUP_CHAIN_NAME_LABEL: &str = "rollup_chain_name";
const ROLLUP_ID_LABEL: &str = "rollup_id";
const COLLECTOR_TYPE_LABEL: &str = "collector_type";

pub struct Metrics {
    geth_txs_received: GethCounters,
    geth_txs_dropped: GethCounters,
    grpc_txs_received: GrpcCounters,
    grpc_txs_dropped: GrpcCounters,
    txs_dropped_too_large: HashMap<RollupId, Counter>,
    nonce_fetch_count: Counter,
    nonce_fetch_failure_count: Counter,
    nonce_fetch_latency: Histogram,
    current_nonce: Gauge,
    sequencer_submission_latency: Histogram,
    sequencer_submission_failure_count: Counter,
    txs_per_submission: Histogram,
    bytes_per_submission: Histogram,
}

impl Metrics {
    pub(crate) fn geth_txs_received(&self, id: &String) -> Option<&Counter> {
        self.geth_txs_received.get(id)
    }

    pub(crate) fn geth_txs_dropped(&self, id: &String) -> Option<&Counter> {
        self.geth_txs_dropped.get(id)
    }

    pub(crate) fn increment_grpc_txs_received(&self, id: &RollupId) {
        let Some(counter) = self.grpc_txs_received.get(id) else {
            error!(rollup_id = %id, "failed to get grpc transactions_received counter");
            return;
        };
        counter.increment(1);
    }

    pub(crate) fn increment_grpc_txs_dropped(&self, id: &RollupId) {
        let Some(counter) = self.grpc_txs_dropped.get(id) else {
            error!(rollup_id = %id, "failed to get grpc transactions_dropped counter");
            return;
        };
        counter.increment(1);
    }

    pub(crate) fn increment_txs_dropped_too_large(&self, id: &RollupId) {
        let Some(counter) = self.txs_dropped_too_large.get(id) else {
            error!(rollup_id = %id, "failed to get transactions_dropped_too_large counter");
            return;
        };
        counter.increment(1);
    }

    pub(crate) fn increment_nonce_fetch_count(&self) {
        self.nonce_fetch_count.increment(1);
    }

    pub(crate) fn increment_nonce_fetch_failure_count(&self) {
        self.nonce_fetch_failure_count.increment(1);
    }

    pub(crate) fn record_nonce_fetch_latency(&self, latency: Duration) {
        self.nonce_fetch_latency.record(latency);
    }

    pub(crate) fn set_current_nonce(&self, nonce: u32) {
        self.current_nonce.set(nonce);
    }

    pub(crate) fn record_sequencer_submission_latency(&self, latency: Duration) {
        self.sequencer_submission_latency.record(latency);
    }

    pub(crate) fn increment_sequencer_submission_failure_count(&self) {
        self.sequencer_submission_failure_count.increment(1);
    }

    pub(crate) fn record_txs_per_submission(&self, count: usize) {
        self.txs_per_submission.record(count);
    }

    pub(crate) fn record_bytes_per_submission(&self, byte_count: usize) {
        self.bytes_per_submission.record(byte_count);
    }
}

impl telemetry::Metrics for Metrics {
    type Config = crate::Config;

    fn register<R: Recorder>(
        builder: &mut RegisteringBuilder<R>,
        config: &Self::Config,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let rollups = config
            .parse_rollups()
            .map_err(|error| Error::External(Box::new(error)))?;
        let (geth_txs_received, grpc_txs_received) =
            register_txs_received(builder, rollups.keys())?;
        let (geth_txs_dropped, grpc_txs_dropped) = register_txs_dropped(builder, rollups.keys())?;
        let txs_dropped_too_large = register_txs_dropped_too_large(builder, rollups.keys())?;

        let nonce_fetch_count = builder
            .new_counter_factory(
                NONCE_FETCH_COUNT,
                "The number of times we have attempted to fetch the nonce",
            )?
            .register()?;

        let nonce_fetch_failure_count = builder
            .new_counter_factory(
                NONCE_FETCH_FAILURE_COUNT,
                "The number of times we have failed to fetch the nonce",
            )?
            .register()?;

        let nonce_fetch_latency = builder
            .new_histogram_factory(
                NONCE_FETCH_LATENCY,
                "The latency of fetching the nonce, in seconds",
            )?
            .register()?;

        let current_nonce = builder
            .new_gauge_factory(CURRENT_NONCE, "The current nonce")?
            .register()?;

        let sequencer_submission_latency = builder
            .new_histogram_factory(
                SEQUENCER_SUBMISSION_LATENCY,
                "The latency of submitting a transaction to the sequencer, in seconds",
            )?
            .register()?;

        let sequencer_submission_failure_count = builder
            .new_counter_factory(
                SEQUENCER_SUBMISSION_FAILURE_COUNT,
                "The number of failed transaction submissions to the sequencer",
            )?
            .register()?;

        let txs_per_submission = builder
            .new_histogram_factory(
                TRANSACTIONS_PER_SUBMISSION,
                "The number of rollup transactions successfully sent to the sequencer in a single \
                 submission",
            )?
            .register()?;

        let bytes_per_submission = builder
            .new_histogram_factory(
                BYTES_PER_SUBMISSION,
                "The total bytes successfully sent to the sequencer in a single submission",
            )?
            .register()?;

        Ok(Self {
            geth_txs_received,
            geth_txs_dropped,
            grpc_txs_received,
            grpc_txs_dropped,
            txs_dropped_too_large,
            nonce_fetch_count,
            nonce_fetch_failure_count,
            nonce_fetch_latency,
            current_nonce,
            sequencer_submission_latency,
            sequencer_submission_failure_count,
            txs_per_submission,
            bytes_per_submission,
        })
    }
}

fn register_txs_received<'a, R: Recorder>(
    builder: &mut RegisteringBuilder<R>,
    rollup_chain_names: impl Iterator<Item = &'a String>,
) -> Result<(GethCounters, GrpcCounters), Error> {
    let mut factory = builder.new_counter_factory(
        TRANSACTIONS_RECEIVED,
        "The number of transactions successfully received from collectors and bundled, labelled \
         by rollup and collector type",
    )?;

    let mut geth_counters = HashMap::new();
    let mut grpc_counters = HashMap::new();

    for chain_name in rollup_chain_names {
        let rollup_id = RollupId::from_unhashed_bytes(chain_name.as_bytes());

        let geth_counter = factory.register_with_labels(&[
            (ROLLUP_CHAIN_NAME_LABEL, chain_name.clone()),
            (ROLLUP_ID_LABEL, rollup_id.to_string()),
            (COLLECTOR_TYPE_LABEL, "geth".to_string()),
        ])?;
        geth_counters.insert(chain_name.clone(), geth_counter);

        let grpc_counter = factory.register_with_labels(&[
            (ROLLUP_CHAIN_NAME_LABEL, chain_name.clone()),
            (ROLLUP_ID_LABEL, rollup_id.to_string()),
            (COLLECTOR_TYPE_LABEL, "grpc".to_string()),
        ])?;
        grpc_counters.insert(rollup_id, grpc_counter);
    }
    Ok((geth_counters, grpc_counters))
}

fn register_txs_dropped<'a, R: Recorder>(
    builder: &mut RegisteringBuilder<R>,
    rollup_chain_names: impl Iterator<Item = &'a String>,
) -> Result<(GethCounters, GrpcCounters), Error> {
    let mut factory = builder.new_counter_factory(
        TRANSACTIONS_DROPPED,
        "The number of transactions dropped by the collectors before bundling, labelled by rollup \
         and collector type",
    )?;

    let mut geth_counters = HashMap::new();
    let mut grpc_counters = HashMap::new();

    for chain_name in rollup_chain_names {
        let rollup_id = RollupId::from_unhashed_bytes(chain_name.as_bytes());

        let geth_counter = factory.register_with_labels(&[
            (ROLLUP_CHAIN_NAME_LABEL, chain_name.clone()),
            (ROLLUP_ID_LABEL, rollup_id.to_string()),
            (COLLECTOR_TYPE_LABEL, "geth".to_string()),
        ])?;
        geth_counters.insert(chain_name.clone(), geth_counter);

        let grpc_counter = factory.register_with_labels(&[
            (ROLLUP_CHAIN_NAME_LABEL, chain_name.clone()),
            (ROLLUP_ID_LABEL, rollup_id.to_string()),
            (COLLECTOR_TYPE_LABEL, "grpc".to_string()),
        ])?;
        grpc_counters.insert(rollup_id, grpc_counter);
    }
    Ok((geth_counters, grpc_counters))
}

fn register_txs_dropped_too_large<'a, R: Recorder>(
    builder: &mut RegisteringBuilder<R>,
    rollup_chain_names: impl Iterator<Item = &'a String>,
) -> Result<HashMap<RollupId, Counter>, Error> {
    let mut factory = builder.new_counter_factory(
        TRANSACTIONS_DROPPED_TOO_LARGE,
        "The number of transactions dropped because they were too large, labelled by rollup",
    )?;

    let mut counters = HashMap::new();

    for chain_name in rollup_chain_names {
        let rollup_id = RollupId::from_unhashed_bytes(chain_name.as_bytes());

        let counter = factory.register_with_labels(&[
            (ROLLUP_CHAIN_NAME_LABEL, chain_name.clone()),
            (ROLLUP_ID_LABEL, rollup_id.to_string()),
        ])?;
        counters.insert(rollup_id, counter);
    }
    Ok(counters)
}

metric_names!(pub const METRICS_NAMES:
    TRANSACTIONS_RECEIVED,
    TRANSACTIONS_DROPPED,
    TRANSACTIONS_DROPPED_TOO_LARGE,
    NONCE_FETCH_COUNT,
    NONCE_FETCH_FAILURE_COUNT,
    NONCE_FETCH_LATENCY,
    CURRENT_NONCE,
    SEQUENCER_SUBMISSION_LATENCY,
    SEQUENCER_SUBMISSION_FAILURE_COUNT,
    TRANSACTIONS_PER_SUBMISSION,
    BYTES_PER_SUBMISSION
);

#[cfg(test)]
mod tests {
    use super::{
        BYTES_PER_SUBMISSION,
        CURRENT_NONCE,
        NONCE_FETCH_COUNT,
        NONCE_FETCH_FAILURE_COUNT,
        NONCE_FETCH_LATENCY,
        SEQUENCER_SUBMISSION_FAILURE_COUNT,
        SEQUENCER_SUBMISSION_LATENCY,
        TRANSACTIONS_DROPPED,
        TRANSACTIONS_DROPPED_TOO_LARGE,
        TRANSACTIONS_PER_SUBMISSION,
        TRANSACTIONS_RECEIVED,
    };

    #[track_caller]
    fn assert_const(actual: &'static str, suffix: &str) {
        // XXX: hard-code this so the crate name isn't accidentally changed.
        const CRATE_NAME: &str = "astria_composer";
        let expected = format!("{CRATE_NAME}_{suffix}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn metrics_are_as_expected() {
        assert_const(TRANSACTIONS_RECEIVED, "transactions_received");
        assert_const(TRANSACTIONS_DROPPED, "transactions_dropped");
        assert_const(
            TRANSACTIONS_DROPPED_TOO_LARGE,
            "transactions_dropped_too_large",
        );
        assert_const(NONCE_FETCH_COUNT, "nonce_fetch_count");
        assert_const(NONCE_FETCH_FAILURE_COUNT, "nonce_fetch_failure_count");
        assert_const(NONCE_FETCH_LATENCY, "nonce_fetch_latency");
        assert_const(CURRENT_NONCE, "current_nonce");
        assert_const(SEQUENCER_SUBMISSION_LATENCY, "sequencer_submission_latency");
        assert_const(
            SEQUENCER_SUBMISSION_FAILURE_COUNT,
            "sequencer_submission_failure_count",
        );
        assert_const(TRANSACTIONS_PER_SUBMISSION, "transactions_per_submission");
        assert_const(BYTES_PER_SUBMISSION, "bytes_per_submission");
    }
}
