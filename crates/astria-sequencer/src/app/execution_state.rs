use astria_eyre::eyre::{
    bail,
    Result,
};
use tendermint::{
    abci,
    Hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionFingerprintData {
    // No ProposalFingerprint has been set.
    // Transitions to: Prepared, ExecutedBlock
    Unset,
    // State after preparing a ProcessProposal request
    // - data is a fingerprint of the request/response.
    // Transitions to either: PreparedValid or CheckedPreparedMismatch
    Prepared([u8; 32]),
    // State after comparing a `Prepared` fingerprint to a ProcessProposal request if it matched.
    // - data is the fingerprint from the `Prepared` state.
    // Transitions to: ExecutedBlock
    PreparedValid([u8; 32]),
    // The fingerprint failed comparison against a Prepared state
    // - data is a fingerprint from Prepared state.
    // End state.
    CheckedPreparedMismatch([u8; 32]),
    // Fingerprint from after executing a complete block.
    // - first value is the CometBft block hash
    // - second is the `Prepared` fingerprint if transitioned from a `PreparedVerified` state.
    // Transitions to: CheckedExecutedBlockMismatch
    ExecutedBlock([u8; 32], Option<[u8; 32]>),
    // The fingerprint failed comparison against a ExecutedBlock state
    // - data matches that of the ExecutedBlock state which came from
    // End state.
    CheckedExecutedBlockMismatch([u8; 32], Option<[u8; 32]>),
}

// State machine for tracking what state the app has executed
// data in. This is used to check if transactions have been executed
// in different ABCI calls across requests, and whether the cached state can be
// used or should be reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionState(ExecutionFingerprintData);

impl ExecutionState {
    pub(crate) fn new() -> Self {
        Self(ExecutionFingerprintData::Unset)
    }

    pub(crate) fn data(&self) -> ExecutionFingerprintData {
        self.0
    }

    #[cfg(test)]
    // This is used just to make testing easier for transitions without needing to fully setup
    fn create_with_data(data: ExecutionFingerprintData) -> Self {
        Self(data)
    }

    // Called at the end `prepare_proposal`, it takes the request and response
    // to create a partial ProcessProposal message, serializes and hashes that
    // data to create a fingerprint.
    //
    // Can only be run on an unset fingerprint.
    pub(crate) fn set_prepared_proposal(
        &mut self,
        request: abci::request::PrepareProposal,
        response: abci::response::PrepareProposal,
    ) -> Result<()> {
        use prost::Message as _;
        use sha2::{
            Digest as _,
            Sha256,
        };
        use tendermint_proto::v0_38::abci as pb;
        if self.0 != ExecutionFingerprintData::Unset {
            bail!("execution state already set");
        }

        let proposed_last_commit = if let Some(local_last_commit) = request.local_last_commit {
            let vote_info = local_last_commit
                .votes
                .into_iter()
                .map(|vote| abci::types::VoteInfo {
                    validator: vote.validator,
                    sig_info: vote.sig_info,
                })
                .collect();
            Some(abci::types::CommitInfo {
                round: local_last_commit.round,
                votes: vote_info,
            })
        } else {
            None
        };
        let proposal = abci::request::ProcessProposal {
            hash: Hash::default(),
            proposed_last_commit,
            height: request.height,
            time: request.time,
            proposer_address: request.proposer_address,
            next_validators_hash: request.next_validators_hash,
            misbehavior: request.misbehavior,
            txs: response.txs,
        };

        let pb_data = pb::RequestProcessProposal::from(proposal).encode_to_vec();
        let data: [u8; 32] = Sha256::digest(pb_data).into();
        self.0 = ExecutionFingerprintData::Prepared(data);
        Ok(())
    }

    // Given a ProcessProposal request, check the ProcessProposal matches
    // the current executed proposal. If it does not match, the status is set to
    // `CheckedPreparedMismatch`.
    // Will always return false if called on a non `Prepared` or `PreparedValid` state.
    // Returns whether the proposal matches the current fingerprint.
    pub(crate) fn check_if_prepared_proposal(
        &mut self,
        proposal: &abci::request::ProcessProposal,
    ) -> bool {
        use prost::Message as _;
        use sha2::{
            Digest as _,
            Sha256,
        };
        use tendermint_proto::v0_38::abci as pb;
        match self.0 {
            ExecutionFingerprintData::Unset
            | ExecutionFingerprintData::CheckedPreparedMismatch(_)
            | ExecutionFingerprintData::CheckedExecutedBlockMismatch(..)
            | ExecutionFingerprintData::ExecutedBlock(..) => false,
            ExecutionFingerprintData::PreparedValid(proposal_hash)
            | ExecutionFingerprintData::Prepared(proposal_hash) => {
                let partial_proposal = abci::request::ProcessProposal {
                    hash: Hash::default(),
                    ..proposal.clone()
                };
                let pb_data = pb::RequestProcessProposal::from(partial_proposal).encode_to_vec();
                let data: [u8; 32] = Sha256::digest(pb_data).into();
                if proposal_hash != data {
                    self.0 = ExecutionFingerprintData::CheckedPreparedMismatch(proposal_hash);
                    return false;
                }

                self.0 = ExecutionFingerprintData::PreparedValid(proposal_hash);
                true
            }
        }
    }

    // Called after `process_proposal` has been called or `finalize_block` to set
    // to a `ExecutedBlock` fingerprint. Can only be called on a `Prepared`
    // or `Unset` fingerprint, otherwise will error.
    pub(crate) fn set_executed_block(&mut self, block_hash: [u8; 32]) -> Result<()> {
        match self.0 {
            ExecutionFingerprintData::Unset => {
                self.0 = ExecutionFingerprintData::ExecutedBlock(block_hash, None);
            }
            ExecutionFingerprintData::PreparedValid(proposal_hash) => {
                self.0 = ExecutionFingerprintData::ExecutedBlock(block_hash, Some(proposal_hash));
            }
            ExecutionFingerprintData::Prepared(_) => {
                bail!(
                    "executed block fingerprint attempted to be set before prepared proposal \
                     fingerprint validated.",
                );
            }
            ExecutionFingerprintData::ExecutedBlock(..) => {
                bail!("executed block fingerprint attempted to be set again.",);
            }
            ExecutionFingerprintData::CheckedPreparedMismatch(_)
            | ExecutionFingerprintData::CheckedExecutedBlockMismatch(..) => {
                bail!("executed block fingerprint shouldn't be set after invalid check.",);
            }
        }

        Ok(())
    }

    // Given a block hash, check if it matches the current execution state.
    //
    // If checking against an `ExecutedBlock` fingerprint, will compare the hash, update
    // the status to `CheckedExecutedBlockMismatch` if it does not match.
    //
    // Should not be called on a `Prepared` fingerprint, will change status
    // to `CheckedPreparedMismatch`.
    pub(crate) fn check_if_executed_block(&mut self, block_hash: [u8; 32]) -> bool {
        match self.0 {
            ExecutionFingerprintData::Unset
            | ExecutionFingerprintData::CheckedPreparedMismatch(_)
            | ExecutionFingerprintData::CheckedExecutedBlockMismatch(..) => false,
            // Can only call check executed on an executed fingerprint.
            ExecutionFingerprintData::Prepared(proposal_hash)
            | ExecutionFingerprintData::PreparedValid(proposal_hash) => {
                self.0 = ExecutionFingerprintData::CheckedPreparedMismatch(proposal_hash);

                false
            }
            ExecutionFingerprintData::ExecutedBlock(cached_block_hash, proposal_hash) => {
                if block_hash != cached_block_hash {
                    self.0 = ExecutionFingerprintData::CheckedExecutedBlockMismatch(
                        cached_block_hash,
                        proposal_hash,
                    );
                    return false;
                }

                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tendermint::{
        abci::{
            request,
            response,
        },
        Time,
    };

    use super::*;

    fn build_prepare_proposal(
        txs: &[bytes::Bytes],
    ) -> (request::PrepareProposal, response::PrepareProposal) {
        let time = Time::from_unix_timestamp(1_741_740_299, 32).unwrap();
        let request = request::PrepareProposal {
            height: 1u32.into(),
            time,
            proposer_address: [9u8; 20].to_vec().try_into().unwrap(),
            next_validators_hash: Hash::default(),
            local_last_commit: None,
            misbehavior: vec![],
            txs: vec![],
            max_tx_bytes: 1_000_000,
        };
        let response = abci::response::PrepareProposal {
            txs: txs.to_owned(),
        };
        (request, response)
    }

    fn build_process_proposal(
        request: request::PrepareProposal,
        response: response::PrepareProposal,
        matching: bool,
    ) -> request::ProcessProposal {
        // The time is otherwise set to a fixed value so this should
        // always be different.
        // Also avoids using non-matching values in snapshots
        let time = if matching { request.time } else { Time::now() };
        request::ProcessProposal {
            hash: Hash::Sha256([6u8; 32]),
            proposed_last_commit: None,
            height: request.height,
            time,
            proposer_address: request.proposer_address,
            next_validators_hash: request.next_validators_hash,
            misbehavior: request.misbehavior,
            txs: response.txs,
        }
    }

    #[test]
    fn new_execution_state_is_unset() {
        let state = ExecutionState::new();
        assert_eq!(state.data(), ExecutionFingerprintData::Unset);
    }

    #[test]
    fn set_prepared_succeeds() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let mut state = ExecutionState::new();

        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();
        insta::assert_debug_snapshot!(state.data());
    }

    #[test]
    fn set_prepared_fails_on_non_unset() {
        let hash = [1u8; 32];
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);

        let states_to_test = vec![
            ExecutionFingerprintData::Prepared(hash),
            ExecutionFingerprintData::PreparedValid(hash),
            ExecutionFingerprintData::CheckedPreparedMismatch(hash),
            ExecutionFingerprintData::ExecutedBlock(hash, None),
            ExecutionFingerprintData::ExecutedBlock(hash, Some(hash)),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(hash, None),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(hash, Some(hash)),
        ];

        for state in states_to_test {
            let mut state = ExecutionState::create_with_data(state);
            let data_copy = state.data();
            let _ = state
                .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
                .unwrap_err();
            assert_eq!(state.data(), data_copy);
        }
    }

    #[test]
    fn check_if_prepared_validates_successfully() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let matching_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), true);
        let mut state = ExecutionState::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();

        assert!(state.check_if_prepared_proposal(&matching_request));
        insta::assert_debug_snapshot!(state.data());
        let data_copy = state.data();

        // Should validate a second time, don't need a second snapshot
        assert!(state.check_if_prepared_proposal(&matching_request));
        assert_eq!(state.data(), data_copy);
    }

    #[test]
    fn check_if_prepared_on_prepared_valid_will_invalidate_mismatch() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let match_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), true);
        let mismatch_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), false);
        let mut state = ExecutionState::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();

        // First get validated then attempt again with mismatch
        assert!(state.check_if_prepared_proposal(&match_request));
        assert!(!state.check_if_prepared_proposal(&mismatch_request));
        insta::assert_debug_snapshot!(state.data());

        // Shouldn't change back to passing
        assert!(!state.check_if_prepared_proposal(&match_request));
    }

    #[test]
    fn check_if_prepared_invalidates_mismatch() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let process_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), false);
        let mut state = ExecutionState::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();

        assert!(!state.check_if_prepared_proposal(&process_request));
        insta::assert_debug_snapshot!(state.data());
    }

    #[test]
    fn check_if_prepared_false_on_ineligible_states() {
        let block_hash = [1u8; 32];
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let match_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), true);
        let mut state = ExecutionState::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();
        let ExecutionFingerprintData::Prepared(prop_hash) = state.data() else {
            panic!("Expected Prepared state");
        };

        // Each one of these states should return false and not change the state.
        let states_to_test = vec![
            ExecutionFingerprintData::Unset,
            ExecutionFingerprintData::CheckedPreparedMismatch(prop_hash),
            ExecutionFingerprintData::ExecutedBlock(block_hash, None),
            ExecutionFingerprintData::ExecutedBlock(block_hash, Some(prop_hash)),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(block_hash, None),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(block_hash, Some(prop_hash)),
        ];

        for state in states_to_test {
            let mut state = ExecutionState::create_with_data(state);
            let data_copy = state.data();
            assert!(!state.check_if_prepared_proposal(&match_request));
            assert_eq!(state.data(), data_copy);
        }
    }

    #[test]
    fn set_execute_block_without_prepared_succeeds() {
        let block_hash = [1u8; 32];
        let mut state = ExecutionState::new();

        state.set_executed_block(block_hash).unwrap();
        assert_eq!(
            state.data(),
            ExecutionFingerprintData::ExecutedBlock(block_hash, None)
        );
    }

    #[test]
    fn set_execute_block_with_prepared_validated_succeeds() {
        let block_hash = [1u8; 32];
        let mut state =
            ExecutionState::create_with_data(ExecutionFingerprintData::PreparedValid([2u8; 32]));

        state.set_executed_block(block_hash).unwrap();
        assert_eq!(
            state.data(),
            ExecutionFingerprintData::ExecutedBlock(block_hash, Some([2u8; 32]))
        );
    }

    #[test]
    fn set_execute_block_with_non_settable_states_errors() {
        let hash = [1u8; 32];
        let mut errors = vec![];

        // All of these should error, and then we will validate errors don't change with snapshot.
        let mut state =
            ExecutionState::create_with_data(ExecutionFingerprintData::Prepared([1u8; 32]));
        errors.push(state.set_executed_block(hash).unwrap_err());
        state = ExecutionState::create_with_data(
            ExecutionFingerprintData::CheckedPreparedMismatch(hash),
        );
        errors.push(state.set_executed_block(hash).unwrap_err());
        state = ExecutionState::create_with_data(ExecutionFingerprintData::ExecutedBlock(
            hash,
            Some(hash),
        ));
        errors.push(state.set_executed_block(hash).unwrap_err());
        state = ExecutionState::create_with_data(
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(hash, Some(hash)),
        );
        errors.push(state.set_executed_block(hash).unwrap_err());

        insta::assert_debug_snapshot!(errors);
    }

    #[test]
    fn check_if_execute_block_succeeds_on_matching() {
        let block_hash = [1u8; 32];
        let mut state = ExecutionState::create_with_data(ExecutionFingerprintData::ExecutedBlock(
            block_hash, None,
        ));
        let data_copy = state.data();

        assert!(state.check_if_executed_block(block_hash));
        assert_eq!(state.data(), data_copy);

        state = ExecutionState::create_with_data(ExecutionFingerprintData::ExecutedBlock(
            block_hash,
            Some(block_hash),
        ));
        let data_copy = state.data();

        assert!(state.check_if_executed_block(block_hash));
        assert_eq!(state.data(), data_copy);
    }

    #[test]
    fn check_if_execute_block_fails_on_non_matching() {
        let block_hash = [1u8; 32];
        let mut state = ExecutionState::create_with_data(ExecutionFingerprintData::ExecutedBlock(
            block_hash, None,
        ));

        assert!(!state.check_if_executed_block([2u8; 32]));
        assert_eq!(
            state.data(),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(block_hash, None)
        );

        state = ExecutionState::create_with_data(ExecutionFingerprintData::ExecutedBlock(
            block_hash,
            Some([3u8; 32]),
        ));
        assert!(!state.check_if_executed_block([2u8; 32]));
        assert_eq!(
            state.data(),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(block_hash, Some([3u8; 32]))
        );
    }

    #[test]
    fn check_if_execute_block_fails_on_prepared_states() {
        let hash = [1u8; 32];
        let mut state = ExecutionState::create_with_data(ExecutionFingerprintData::Prepared(hash));

        assert!(!state.check_if_executed_block(hash));
        assert_eq!(
            state.data(),
            ExecutionFingerprintData::CheckedPreparedMismatch(hash)
        );

        // Should also fail and state should match what is called on prepared state
        state = ExecutionState::create_with_data(ExecutionFingerprintData::PreparedValid(hash));
        assert!(!state.check_if_executed_block(hash));
        assert_eq!(
            state.data(),
            ExecutionFingerprintData::CheckedPreparedMismatch(hash)
        );
    }

    #[test]
    fn check_if_execute_block_fails_on_ineligible_states() {
        let hash = [1u8; 32];

        // Each one of these states should return false and not change the state.
        let states_to_test = vec![
            ExecutionFingerprintData::Unset,
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(hash, None),
            ExecutionFingerprintData::CheckedExecutedBlockMismatch(hash, Some(hash)),
        ];

        for state in states_to_test {
            let mut state = ExecutionState::create_with_data(state);
            let data_copy = state.data();
            assert!(!state.check_if_executed_block(hash));
            assert_eq!(state.data(), data_copy);
        }
    }
}
