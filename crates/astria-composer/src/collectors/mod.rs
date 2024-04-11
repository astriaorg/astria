pub(crate) mod geth;
pub(crate) mod grpc;

const EXECUTOR_SEND_TIMEOUT: Duration = Duration::from_millis(500);

use std::time::Duration;

pub(crate) use geth::Geth;
pub(crate) use grpc::Grpc;
