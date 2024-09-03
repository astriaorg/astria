use std::{
    collections::HashSet,
    net::AddrParseError,
};

use itertools::Itertools;
use thiserror::Error;

#[cfg(doc)]
use super::Metrics;

/// An error related to registering or initializing metrics.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// The metric has already been registered.
    #[error("{metric_type} `{metric_name}` has already been registered")]
    MetricAlreadyRegistered {
        metric_type: &'static str,
        metric_name: &'static str,
    },

    /// The metric with the given labels has already been registered.
    #[error("{metric_type} `{metric_name}` has already been registered with the given labels")]
    MetricWithLabelsAlreadyRegistered {
        metric_type: &'static str,
        metric_name: &'static str,
    },

    /// The metric has a duplicate label.
    #[error(
        "{metric_type} `{metric_name}` has a duplicate of label `{label_name}=\"{label_value}\"`"
    )]
    DuplicateLabel {
        metric_type: &'static str,
        metric_name: &'static str,
        label_name: String,
        label_value: String,
    },

    /// Failed to set the given histogram's buckets.
    #[error("the buckets for histogram `{0}` have already been set")]
    BucketsAlreadySet(&'static str),

    /// The given histogram's buckets are empty.
    #[error("the buckets for histogram `{0}` must have at least one value")]
    EmptyBuckets(&'static str),

    /// The given histograms were assigned buckets, but never registered.
    #[error(
        "histogram(s) [{}] had buckets assigned via `Metrics::set_buckets` but were never \
        registered via `Metrics::register`",
        .0.iter().join(", ")
    )]
    BucketsNotAssigned(HashSet<String>),

    /// Failed to parse the metrics exporter listening address.
    #[error("failed to parse metrics exporter listening address")]
    ParseListeningAddress(#[from] AddrParseError),

    /// Failed to start the metrics exporter server.
    #[error("failed to start the metrics exporter server")]
    StartListening(#[source] StartListeningError),

    /// Failed to set the global metrics recorder.
    #[error("the global metrics recorder has already been set")]
    GlobalMetricsRecorderAlreadySet,

    /// External error, intended for use in implementations of [`Metrics`].
    #[error(transparent)]
    External(Box<dyn std::error::Error + Send + Sync>),
}

/// An error while starting the metrics exporter server.
#[derive(Error, Debug)]
#[error(transparent)]
// allow: the name correctly reflects the type.
#[allow(clippy::module_name_repetitions)]
pub struct StartListeningError(#[from] metrics_exporter_prometheus::BuildError);
