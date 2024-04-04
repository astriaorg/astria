use std::fmt::Write as _;

use crate::{
    mock::Times,
    mounted_mock::BadResponse,
};

/// A report returned by an `MountedMock` detailing what the user expectations were and
/// how many calls were actually received since the mock was mounted on the server.
#[derive(Clone)]
pub(crate) struct VerificationReport {
    /// The mock name specified by the user.
    pub(crate) mock_name: Option<String>,
    pub(crate) rpc: &'static str,
    // /// What users specified
    pub(crate) expectation_range: Times,
    /// Actual number of received requests that matched the specification
    pub(crate) n_successful_requests: u64,
    pub(crate) bad_responses: Vec<BadResponse>,
    /// The position occupied by the mock that generated the report within its parent
    /// [`MountedMockSet`](crate::mock_set::MountedMockSet) collection of `MountedMock`s.
    ///
    /// E.g. `0` if it is the first mock that we try to match against an incoming request, `1`
    /// if it is the second, etc.
    pub(crate) position_in_set: usize,
}

impl VerificationReport {
    #[rustfmt::skip]
    pub(crate) fn error_message(&self) -> String {
        let mut msg = if let Some(ref mock_name) = self.mock_name {
            format!(
                "{} for RPC {}.\n\t\
                Expected range of matching incoming requests: {}\n\t\
                Number of matched requests with valid mock responses: {}\n\t\
                Number of matched requests with bad mock responses: {}\n\t",
                mock_name, self.rpc, self.expectation_range, self.n_successful_requests, self.bad_responses.len(),
            )
        } else {
            format!(
                "Mock #{} for RPC {}.\n\t\
                Expected range of matching incoming requests: {}\n\t\
                Number of matched requests with valid mock responses: {}\n\t\
                Number of matched requests with bad mock responses: {}\n\t",
                self.position_in_set, self.rpc, self.expectation_range, self.n_successful_requests, self.bad_responses.len(),
            )
        };

        if !self.bad_responses.is_empty() {
            let _ = writeln!(msg, "Bad responses (mock response had the wrong type):");
            msg = self.bad_responses.iter().enumerate().fold(
                msg,
                |mut msg, (index, response)| {
                    _ = writeln!(msg, "\t - Bad response #{index}");
                    _ = response.print(indenter::indented(&mut msg).with_str("\t\t"));
                    msg
                }
            );
        }
        msg
    }

    pub(crate) fn is_satisfied(&self) -> bool {
        self.expectation_range.contains(self.n_successful_requests) && !self.has_bad_response()
    }

    pub(crate) fn has_bad_response(&self) -> bool {
        !self.bad_responses.is_empty()
    }
}

pub(crate) enum VerificationOutcome {
    /// The expectations set on all active mocks were satisfied.
    Success,
    /// The expectations set for one or more of the active mocks were not satisfied.
    /// All failed expectations are returned.
    Failure(Vec<VerificationReport>),
}
