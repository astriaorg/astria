pub(crate) mod geth;
pub(crate) mod grpc;

use std::{
    fmt::{
        Display,
        Formatter,
    },
    time::Duration,
};

const EXECUTOR_SEND_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub(crate) enum CollectorType {
    Grpc,
    Geth,
}

impl Display for CollectorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CollectorType::Grpc => write!(f, "grpc"),
            CollectorType::Geth => write!(f, "geth"),
        }
    }
}

pub(crate) use geth::Geth;
pub(crate) use grpc::Grpc;
