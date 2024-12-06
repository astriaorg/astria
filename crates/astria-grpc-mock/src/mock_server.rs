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

/// A mock server that can have [`Mock`]s mounted to it to simulate gRPC responses.
///
/// # Examples
///
/// ```rust
/// use astria_grpc_mock::{
///     matcher,
///     response,
///     AnyMessage,
///     Mock,
///     MockServer,
/// };
/// use futures::executor::block_on;
///
/// // `MockMessage` implementation hidden for brevity
/// # #[derive(serde::Serialize, ::prost::Message, Clone, PartialEq)]
/// # struct MockMessage {
/// #     #[prost(string, tag = "1")]
/// #     name: String,
/// # }
/// # impl ::prost::Name for MockMessage {
/// #     const NAME: &'static str = "MockMessage";
/// #     const PACKAGE: &'static str = "test";
/// #
/// #     fn full_name() -> ::prost::alloc::string::String {
/// #         ::prost::alloc::format!("test.{}", Self::NAME)
/// #     }
/// # }
///
/// let mock_message = MockMessage {
///     name: "test".to_string(),
/// };
///
/// let server = MockServer::new();
/// block_on(server.disable_request_recording());
///
/// let mock = Mock::for_rpc_given("rpc", matcher::message_exact_pbjson(&mock_message))
///     .respond_with(response::constant_response(mock_message.clone()));
/// let _mock_guard = block_on(server.register_as_scoped(mock));
/// // or: block_on(server.register(mock));
/// let rsp = block_on(server.handle_request::<MockMessage, MockMessage>(
///     "rpc",
///     tonic::Request::new(mock_message.clone()),
/// ));
///
/// assert_eq!(rsp.unwrap().into_inner(), mock_message);
/// ```
#[derive(Clone, Default)]
pub struct MockServer {
    state: Arc<RwLock<MockServerState>>,
}

impl MockServer {
    /// Creates a new [`MockServer`] instance. See [`MockServer`] docs for example usage.
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

    /// Disables recording of incoming requests on the server. This will delete all previously
    /// recorded requests and prevent any new requests from being recorded. There is no option
    /// to enable recording again. It will not prevent the server from recording failed
    /// verifications, and will only prevent the received requests from being printed in the panic
    /// message upon failed verification. See [`MockServer`] docs for example usage.
    pub async fn disable_request_recording(&self) {
        self.state.write().await.received_requests = None;
    }

    /// Takes an RPC name and [`tonic::Request`] and returns a [`tonic::Response`] based on the
    /// first [`Mock`] that matches the RPC name and request. If no mock matches, it will
    /// return a status with [`tonic::Code::NotFound`]. See [`MockServer`] docs for example usage.
    pub async fn handle_request<
        T: erased_serde::Serialize + prost::Name + Clone + Send + Sync + 'static,
        U: Send + Sync + 'static,
    >(
        &self,
        rpc: &'static str,
        req: tonic::Request<T>,
    ) -> tonic::Result<tonic::Response<U>> {
        let (response, delay) = self.state.write().await.handle_request(rpc, req);
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }
        response
    }

    /// Mounts a [`Mock`] to the server. Once mounted, the server will respond to calls to
    /// [`handle_request`](`MockServer::handle_request`) that match one of the mounted mocks. See
    /// [`MockServer`] docs for example usage.
    pub async fn register(&self, mock: Mock) {
        self.state.write().await.mock_set.register(mock);
    }

    /// Mounts a [`Mock`] to the server, returning a [`MockGuard`] that can be evaluated for
    /// satisfaction of the mock. Once mounted, the server will respond to calls to
    /// [`handle_request`](`MockServer::handle_request`) that match one of the mounted mocks. See
    /// [`MockServer`] docs for example usage.
    pub async fn register_as_scoped(&self, mock: Mock) -> MockGuard {
        let (notify, mock_id) = self.state.write().await.mock_set.register(mock);
        MockGuard {
            notify,
            mock_id,
            server_state: self.state.clone(),
        }
    }

    /// Verifies that all mounted [`Mock`]s have been satisfied and that there were no bad responses
    /// from them.
    ///
    /// # Panics
    ///
    /// Panics with information about the failed verifications along with recieved requests if any
    /// of the mounted mocks were not satisfied, or if any of them returned a bad response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use astria_grpc_mock::{
    ///     matcher,
    ///     response,
    ///     AnyMessage,
    ///     Mock,
    ///     MockServer,
    /// };
    /// use futures::executor::block_on;
    ///
    /// let server = MockServer::new();
    /// block_on(
    ///     Mock::for_rpc_given("rpc", matcher::message_type::<AnyMessage>())
    ///         .respond_with(response::error_response(0.into()))
    ///         .expect(0)
    ///         .mount(&server),
    /// );
    /// block_on(server.verify());
    ///
    /// // The code below will panic, since the mock expects 1 request and receives 0.
    /// // block_on(Mock::for_rpc_given("rpc", matcher::message_type::<AnyMessage>())
    /// //    .respond_with(response::error_response(0.into()))
    /// //    .expect(1)
    /// //    .mount(&server));
    /// // block_on(server.verify());
    /// ```
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

/// A guard which can be evaluated for satisfation of a [`Mock`].
///
/// Returned by [`MockServer::register_as_scoped`] and [`Mock::mount_as_scoped`], it can be
/// evaluated by calling [`wait_until_satisfied`](`MockGuard::wait_until_satisfied`). If
/// this method is not called, the guard will be evaluated when it is dropped.
pub struct MockGuard {
    notify: Arc<(Notify, AtomicBool)>,
    mock_id: MockId,
    server_state: Arc<RwLock<MockServerState>>,
}

impl MockGuard {
    /// Awaits satisfaction of the associated [`Mock`], which will occur when it has
    /// received the expected number of matching requests. If the mock expects 0 requests, this
    /// method will return immediately, so it is best practice to instead wait until the guard
    /// is dropped so that it can be ensured no matching requests were made.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use astria_grpc_mock::{
    ///     matcher,
    ///     response,
    ///     AnyMessage,
    ///     Mock,
    ///     MockServer,
    /// };
    /// use futures::{
    ///     executor::block_on,
    ///     future::join,
    /// };
    ///
    /// // `MockMessage` implementation hidden for brevity
    /// # #[derive(serde::Serialize, ::prost::Message, Clone, PartialEq)]
    /// # struct MockMessage {
    /// #     #[prost(string, tag = "1")]
    /// #     name: String,
    /// # }
    /// # impl ::prost::Name for MockMessage {
    /// #     const NAME: &'static str = "MockMessage";
    /// #     const PACKAGE: &'static str = "test";
    /// #
    /// #     fn full_name() -> ::prost::alloc::string::String {
    /// #         ::prost::alloc::format!("test.{}", Self::NAME)
    /// #     }
    /// # }
    ///
    /// let mock_message = MockMessage {
    ///     name: "test".to_string(),
    /// };
    /// let server = MockServer::new();
    /// let mock_guard = block_on(
    ///     Mock::for_rpc_given("rpc", matcher::message_exact_pbjson(&mock_message))
    ///         .respond_with(response::constant_response(mock_message.clone()))
    ///         .expect(1)
    ///         .mount_as_scoped(&server),
    /// );
    /// let rsp_fut = server.handle_request::<MockMessage, MockMessage>(
    ///     "rpc",
    ///     tonic::Request::new(mock_message.clone()),
    /// );
    /// let satisfaction_fut = mock_guard.wait_until_satisfied();
    /// let (rsp, ()) = block_on(join(rsp_fut, satisfaction_fut));
    /// assert_eq!(rsp.unwrap().into_inner(), mock_message);
    /// ```
    pub async fn wait_until_satisfied(&self) {
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
        // TODO(https://github.com/astriaorg/astria/issues/900): Print the metadata map

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
    ) -> (
        tonic::Result<tonic::Response<U>>,
        Option<std::time::Duration>,
    ) {
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
