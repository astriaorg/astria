pub(crate) mod grpc;

use std::time::Duration;

const EXECUTOR_SEND_TIMEOUT: Duration = Duration::from_millis(500);

pub(crate) use grpc::Grpc;
