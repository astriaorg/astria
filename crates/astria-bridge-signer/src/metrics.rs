use telemetry::{
    metric_names,
    metrics::{
        self,
        Counter,
        RegisteringBuilder,
    },
};

pub struct Metrics {
    part_1_request_count: Counter,
    part_2_request_count: Counter,
    valid_message_count: Counter,
    invalid_message_count: Counter,
}

impl metrics::Metrics for Metrics {
    type Config = ();

    fn register(
        builder: &mut RegisteringBuilder,
        _config: &Self::Config,
    ) -> Result<Self, metrics::Error> {
        let part_1_request_count = builder
            .new_counter_factory(
                PART_1_REQUEST_COUNT,
                "The number of times the part1 method of the server was called",
            )?
            .register()?;

        let part_2_request_count = builder
            .new_counter_factory(
                PART_2_REQUEST_COUNT,
                "The number of times the part2 method of the server was called",
            )?
            .register()?;

        let valid_message_count = builder
            .new_counter_factory(
                VALID_MESSAGE_COUNT,
                "The number of valid signing messages that have been received by the server",
            )?
            .register()?;

        let invalid_message_count = builder
            .new_counter_factory(
                INVALID_MESSAGE_COUNT,
                "The number of invalid signing messages that have been received by the server",
            )?
            .register()?;

        Ok(Self {
            part_1_request_count,
            part_2_request_count,
            valid_message_count,
            invalid_message_count,
        })
    }
}

impl Metrics {
    pub(crate) fn increment_part_1_request_count(&self) {
        self.part_1_request_count.increment(1);
    }

    pub(crate) fn increment_part_2_request_count(&self) {
        self.part_2_request_count.increment(1);
    }

    pub(crate) fn increment_valid_message_count(&self) {
        self.valid_message_count.increment(1);
    }

    pub(crate) fn increment_invalid_message_count(&self) {
        self.invalid_message_count.increment(1);
    }
}

metric_names!(const METRICS_NAMES:
    PART_1_REQUEST_COUNT,
    PART_2_REQUEST_COUNT,
    VALID_MESSAGE_COUNT,
    INVALID_MESSAGE_COUNT,
);
