use std::{
    fmt::Write as _,
    pin::pin,
    sync::{
        atomic::AtomicBool,
        Arc,
    },
};

use tokio::sync::{
    Notify,
    RwLock,
};
use tracing::debug;

use super::clone_request;
use crate::{
    erase_request,
    mock::Mock,
    mock_set::{
        MockId,
        MockSet,
    },
    verification::VerificationOutcome,
    AnyRequest,
};

#[derive(Clone, Default)]
pub struct MockServer {
    state: Arc<RwLock<MockServerState>>,
}

impl MockServer {
    #[must_use = "the mock server must be used to be useful"]
    pub fn new() -> Self {
        let state = MockServerState {
            mock_set: MockSet::new(),
            received_requests: Some(Vec::new()),
        };
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub async fn disable_request_recording(&self) {
        self.state.write().await.received_requests = None;
    }

    pub async fn handle_request<
        T: erased_serde::Serialize + prost::Name + Clone + Send + Sync + 'static,
        U: Send + Sync + 'static,
    >(
        &self,
        rpc: &'static str,
        req: tonic::Request<T>,
    ) -> tonic::Result<tonic::Response<U>> {
        self.state.write().await.handle_request(rpc, req)
    }

    pub async fn register(&self, mock: Mock) {
        self.state.write().await.mock_set.register(mock);
    }

    pub async fn register_as_scoped(&self, mock: Mock) -> MockGuard {
        let (notify, mock_id) = self.state.write().await.mock_set.register(mock);
        MockGuard {
            notify,
            mock_id,
            server_state: self.state.clone(),
        }
    }

    pub async fn verify(&self) {
        debug!("verifying mock expectations");
        if let VerificationOutcome::Failure(failed_verifications) = self.state.read().await.verify()
        {
            let received_requests_message =
                received_requests_message(&self.state.read().await.received_requests);

            let verifications_errors: String =
                failed_verifications.iter().fold(String::new(), |mut s, m| {
                    _ = writeln!(s, "- {}", m.error_message());
                    s
                });
            let error_message = format!(
                "Verifications failed:\n{verifications_errors}\n{received_requests_message}",
            );
            if std::thread::panicking() {
                debug!("{}", &error_message);
            } else {
                panic!("{}", &error_message);
            }
        }
    }
}

impl Drop for MockServer {
    // Clean up when the `MockServer` instance goes out of scope.
    fn drop(&mut self) {
        futures::executor::block_on(self.verify());
        // The sender half of the channel, `shutdown_trigger`, gets dropped here
        // Triggering the graceful shutdown of the server itself.
    }
}

pub struct MockGuard {
    notify: Arc<(Notify, AtomicBool)>,
    mock_id: MockId,
    server_state: Arc<RwLock<MockServerState>>,
}

impl MockGuard {
    pub async fn wait_until_satisfied(self) {
        let (notify, flag) = &*self.notify;
        let mut notification = pin!(notify.notified());

        // listen for events of satisfaction.
        notification.as_mut().enable();

        // check if satisfaction has previously been recorded
        if flag.load(std::sync::atomic::Ordering::Acquire) {
            return;
        }

        // await event
        notification.await;
    }
}

struct MockRequest {
    inner: AnyRequest,
}

impl MockRequest {
    fn print(&self, mut buffer: impl std::fmt::Write) -> std::fmt::Result {
        // TODO: Print the metadata map

        // for name in self.inner.iter() {
        //     let values = self
        //         .headers
        //         .get_all(name)
        //         .iter()
        //         .map(|value| String::from_utf8_lossy(value.as_bytes()))
        //         .collect::<Vec<_>>();
        //     let values = values.join(",");
        //     writeln!(buffer, "{}: {}", name, values)?;
        // }

        if let Ok(body) = serde_json::to_string_pretty(self.inner.get_ref().as_serialize()) {
            writeln!(buffer, "{body}")
        } else {
            writeln!(buffer, "Could not map the gRPC body to JSON",)
        }
    }
}

impl From<AnyRequest> for MockRequest {
    fn from(value: AnyRequest) -> Self {
        Self {
            inner: value,
        }
    }
}

#[derive(Default)]
struct MockServerState {
    mock_set: MockSet,
    received_requests: Option<Vec<(&'static str, MockRequest)>>,
}

impl MockServerState {
    fn handle_request<
        T: erased_serde::Serialize + prost::Name + Clone + Send + Sync + 'static,
        U: Send + Sync + 'static,
    >(
        &mut self,
        rpc: &'static str,
        req: tonic::Request<T>,
    ) -> tonic::Result<tonic::Response<U>> {
        if let Some(received_requests) = &mut self.received_requests {
            received_requests.push((rpc, erase_request(clone_request(&req)).into()));
        }
        self.mock_set.handle_request(rpc, req)
    }

    fn verify(&self) -> VerificationOutcome {
        self.mock_set.verify_all()
    }
}

impl Drop for MockGuard {
    fn drop(&mut self) {
        let future = async move {
            let MockGuard {
                mock_id,
                server_state,
                ..
            } = self;
            let mut state = server_state.write().await;
            let report = state.mock_set.verify(*mock_id);

            if report.is_satisfied() {
                state.mock_set.deactivate(*mock_id);
            } else {
                let received_requests_message = received_requests_message(&state.received_requests);

                let verifications_error = format!("- {}\n", report.error_message());
                let error_message = format!(
                    "Verification failed for a scoped \
                     mock:\n{verifications_error}\n{received_requests_message}",
                );
                if std::thread::panicking() {
                    debug!("{}", &error_message);
                } else {
                    panic!("{}", &error_message);
                }
            }
        };
        futures::executor::block_on(future);
    }
}

fn received_requests_message(
    received_requests: &Option<Vec<(&'static str, MockRequest)>>,
) -> String {
    if let Some(received_requests) = received_requests {
        if received_requests.is_empty() {
            "The server did not receive any request.".into()
        } else {
            received_requests.iter().enumerate().fold(
                "Received requests:\n".to_string(),
                |mut message, (index, (rpc, request))| {
                    _ = writeln!(message, "- Request #{index}");
                    _ = writeln!(message, "\tRPC name: {rpc}");
                    _ = writeln!(message, "\tRequests Protobuf as JSON");
                    _ = request.print(indenter::indented(&mut message).with_str("\t"));
                    message
                },
            )
        }
    } else {
        "Enable request recording on the mock server to get the list of incoming requests as part \
         of the panic message."
            .into()
    }
}
