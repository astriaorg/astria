use std::{
    ops::{
        Index,
        IndexMut,
    },
    sync::{
        atomic::AtomicBool,
        Arc,
    },
};

use tokio::sync::Notify;
use tracing::debug;

use super::{
    mock::Mock,
    mounted_mock::MountedMock,
};
use crate::{
    erase_request,
    mounted_mock::MockResult,
    verification::{
        VerificationOutcome,
        VerificationReport,
    },
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum MountedMockState {
    InScope,
    OutOfScope,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct MockId {
    index: usize,
}

#[derive(Default)]
pub(crate) struct MockSet {
    mocks: Vec<(MountedMock, MountedMockState)>,
}

impl MockSet {
    pub(crate) fn new() -> Self {
        Self {
            mocks: Vec::new(),
        }
    }

    pub(crate) fn handle_request<
        T: erased_serde::Serialize + prost::Name + Clone + Send + Sync + 'static,
        U: Send + Sync + 'static,
    >(
        &mut self,
        rpc: &'static str,
        req: tonic::Request<T>,
    ) -> tonic::Result<tonic::Response<U>> {
        debug!("handling request.");
        // perform erasure here so that it's not done in every single `Mock::matches` call.
        let erased = erase_request(req);
        let mut mock_response: Option<tonic::Result<tonic::Response<U>>> = None;
        for (mock, mock_state) in &mut self.mocks {
            if let MountedMockState::OutOfScope = mock_state {
                continue;
            }
            match mock.match_and_respond::<U>(rpc, &erased) {
                MockResult::NoMatch => continue,
                MockResult::BadResponse(status) => {
                    mock_response.replace(Err(status));
                }
                MockResult::Success(response) => {
                    mock_response.replace(response);
                }
            }
        }

        mock_response
            .ok_or_else(|| {
                let mut msg = "got unexpected request: ".to_string();
                msg.push_str(
                    &serde_json::to_string(erased.get_ref().as_serialize())
                        .expect("can map protobuf message to json"),
                );
                tonic::Status::not_found(msg)
            })
            .and_then(std::convert::identity)
    }

    pub(crate) fn register(&mut self, mock: Mock) -> (Arc<(Notify, AtomicBool)>, MockId) {
        let n_registered_rollups = self.mocks.len();
        let mounted_mock = MountedMock::new(mock, n_registered_rollups);
        let notify = mounted_mock.notify();
        self.mocks.push((mounted_mock, MountedMockState::InScope));
        (
            notify,
            MockId {
                index: self.mocks.len() - 1,
            },
        )
    }

    /// Verify that expectations have been met for the [`MountedMock`] corresponding to the
    /// specified [`MockId`].
    pub(crate) fn verify(&self, mock_id: MockId) -> VerificationReport {
        let (mock, _) = &self[mock_id];
        mock.verify()
    }

    pub(crate) fn verify_all(&self) -> VerificationOutcome {
        let failed_verifications: Vec<VerificationReport> = self
            .mocks
            .iter()
            .filter_map(|(mock, state)| (*state == MountedMockState::InScope).then_some(mock))
            .map(MountedMock::verify)
            .filter(|verification_report| !verification_report.is_satisfied())
            .collect();
        if failed_verifications.is_empty() {
            VerificationOutcome::Success
        } else {
            VerificationOutcome::Failure(failed_verifications)
        }
    }

    pub(crate) fn deactivate(&mut self, mock_id: MockId) {
        let mock = &mut self[mock_id];
        mock.1 = MountedMockState::OutOfScope;
    }
}

impl IndexMut<MockId> for MockSet {
    fn index_mut(&mut self, index: MockId) -> &mut Self::Output {
        &mut self.mocks[index.index]
    }
}

impl Index<MockId> for MockSet {
    type Output = (MountedMock, MountedMockState);

    fn index(&self, index: MockId) -> &Self::Output {
        &self.mocks[index.index]
    }
}
