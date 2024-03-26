use std::sync::{
    atomic::AtomicBool,
    Arc,
};

use tokio::sync::Notify;
use tonic::Request;

use super::{
    mock::{
        Match as _,
        Mock,
    },
    response::MockResponse,
    AnyMessage,
};
use crate::{
    clone_request,
    clone_response,
    response::ResponseResult,
    verification::VerificationReport,
};

pub(crate) enum MockResult<U> {
    NoMatch,
    Success(tonic::Result<tonic::Response<U>>),
    BadResponse(tonic::Status),
}

/// A wrapper of erased request to mock response.
///
/// Only used to implement `Clone` because [`Request`] doesn't.
pub(crate) struct BadResponse {
    pub(crate) request: Request<AnyMessage>,
    pub(crate) mock_response: MockResponse,
}

impl BadResponse {
    pub(crate) fn print(&self, mut buffer: impl std::fmt::Write, indent: &str) -> std::fmt::Result {
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

        writeln!(buffer, "{indent}Matched request (Protobuf as JSON)")?;
        writeln!(
            buffer,
            "{indent}Protobuf type name: {}",
            self.request.get_ref().as_name().full_name()
        )?;
        if let Ok(body) = serde_json::to_string_pretty(self.request.get_ref().as_serialize()) {
            writeln!(buffer, "{body}")?;
        } else {
            writeln!(buffer, "<Could not map the gRPC body to JSON>")?;
        }

        writeln!(
            buffer,
            "\n{indent}Bad response (mock returned unexpected protobuf)"
        )?;
        writeln!(
            buffer,
            "{indent}Protobuf type name: {}",
            self.mock_response.inner.get_ref().as_name().full_name()
        )?;
        writeln!(
            buffer,
            "{indent}Rust type name: {}",
            self.mock_response.type_name
        )?;
        writeln!(buffer, "{indent}Protobuf as JSON:")?;
        if let Ok(body) =
            serde_json::to_string_pretty(self.mock_response.inner.get_ref().as_serialize())
        {
            writeln!(buffer, "{body}")
        } else {
            writeln!(buffer, "<Could not map the gRPC body to JSON>")
        }
    }
}

impl From<(Request<AnyMessage>, MockResponse)> for BadResponse {
    fn from(value: (Request<AnyMessage>, MockResponse)) -> Self {
        Self {
            request: value.0,
            mock_response: value.1,
        }
    }
}

impl Clone for BadResponse {
    fn clone(&self) -> Self {
        Self {
            request: clone_request(&self.request),
            mock_response: self.mock_response.clone(),
        }
    }
}

pub(crate) struct MountedMock {
    inner: Mock,
    position_in_set: usize,
    notify: Arc<(Notify, AtomicBool)>,
    successful_response: Vec<(Request<AnyMessage>, ResponseResult)>,
    bad_responses: Vec<BadResponse>,
}

impl MountedMock {
    pub(crate) fn match_and_respond<U: 'static>(
        &mut self,
        rpc: &'static str,
        request: &Request<AnyMessage>,
    ) -> MockResult<U> {
        if self.inner.rpc != rpc
            || !self
                .inner
                .matchers
                .iter()
                .all(|matcher| matcher.matches(request))
        {
            return MockResult::NoMatch;
        }

        let response = match self.inner.response.respond(request) {
            Err(status) => {
                self.successful_response
                    .push((clone_request(request), Err(status.clone())));
                Ok(Err(status))
            }
            Ok(mock_response) => {
                let (metadata, erased_message, extensions) =
                    clone_response(&mock_response.inner).into_parts();
                if let Ok(message) = erased_message.clone_box().into_any().downcast::<U>() {
                    let rsp = tonic::Response::from_parts(metadata, *message, extensions);
                    self.successful_response
                        .push((clone_request(request), Ok(mock_response)));
                    Ok(Ok(rsp))
                } else {
                    let actual = mock_response.type_name;
                    self.bad_responses
                        .push((clone_request(request), mock_response).into());
                    let expected = std::any::type_name::<U>();
                    #[rustfmt::skip]
                        let msg = format!(
                            "failed downcasting mock response to conrete type:\n\
                             required type of gRPC response: `{expected}`\n\
                             type of mock response: `{actual}`\n\
                             JSON serialization:\n\
                             {}",
                            serde_json::to_string_pretty(erased_message.as_serialize())
                                    .expect("can map registered protobuf response to json")
                        );
                    Err(tonic::Status::internal(msg))
                }
            }
        };

        let verification = self.verify();
        // if a bad response was received notify and exist immediately, don't even
        // set the satisfaction flag.
        if verification.has_bad_response() {
            self.notify.0.notify_waiters();
        }
        if verification.is_satisfied() {
            // always set the satisfaction flag **before** raising the event
            self.notify
                .1
                .store(true, std::sync::atomic::Ordering::Release);
            self.notify.0.notify_waiters();
        }
        match response {
            Ok(ok) => MockResult::Success(ok),
            Err(err) => MockResult::BadResponse(err),
        }
    }

    pub(crate) fn new(inner: Mock, position_in_set: usize) -> Self {
        Self {
            inner,
            position_in_set,
            notify: Arc::new((Notify::new(), AtomicBool::new(false))),
            successful_response: Vec::new(),
            bad_responses: Vec::new(),
        }
    }

    pub(crate) fn notify(&self) -> Arc<(Notify, AtomicBool)> {
        self.notify.clone()
    }

    pub(crate) fn verify(&self) -> VerificationReport {
        VerificationReport {
            mock_name: self.inner.name.clone(),
            rpc: self.inner.rpc,
            n_successful_requests: self.successful_response.len() as u64,
            bad_responses: self.bad_responses.clone(),
            expectation_range: self.inner.expectation_range.clone(),
            position_in_set: self.position_in_set,
        }
    }
}
