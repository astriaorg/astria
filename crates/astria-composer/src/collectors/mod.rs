pub(crate) mod geth;
pub(crate) mod grpc;

use std::time::Duration;

const EXECUTOR_SEND_TIMEOUT: Duration = Duration::from_millis(500);

const GETH: &str = "geth";
const GRPC: &str = "grpc";

pub(crate) use geth::Geth;
pub(crate) use grpc::Grpc;
