use std::fmt::{
    self,
    Debug,
    Formatter,
};

use astria_eyre::eyre::{
    bail,
    Result,
};
use bytes::Bytes;
use tendermint::{
    abci::{
        self,
        types::{
            CommitInfo,
            Misbehavior,
        },
    },
    account,
    block,
    Hash,
    Time,
};

/// Details of a proposal that has been handled via a `PrepareProposal` or `ProcessProposal`
/// request.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct CachedProposal {
    time: Time,
    proposer_address: account::Id,
    txs: Vec<Bytes>,
    proposed_last_commit: Option<CommitInfo>,
    misbehavior: Vec<Misbehavior>,
    next_validators_hash: Hash,
    height: block::Height,
}

impl Default for CachedProposal {
    fn default() -> Self {
        Self {
            time: Time::unix_epoch(),
            proposer_address: account::Id::new([0; 20]),
            txs: vec![],
            proposed_last_commit: None,
            misbehavior: vec![],
            height: block::Height::from(0_u8),
            next_validators_hash: Hash::default(),
        }
    }
}

impl Debug for CachedProposal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachedProposal")
            .field("time", &self.time)
            .field("proposer_address", &self.proposer_address)
            .field("txs", &self.txs.iter().map(hex::encode).collect::<Vec<_>>())
            .field("proposed_last_commit", &self.proposed_last_commit)
            .field("misbehavior", &self.misbehavior)
            .field("height", &self.height)
            .field("next_validators_hash", &self.next_validators_hash)
            .finish()
    }
}

/// The various execution states the app can be in for a given block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ExecutionState {
    /// No `CachedProposal` has been set.
    ///
    /// Transitions to `Prepared` or `ExecutedBlock`.
    Unset,
    /// State after preparing a `ProcessProposal` request.
    ///
    /// Transitions to `PreparedValid` or `CheckedPreparedMismatch`.
    Prepared(CachedProposal),
    /// State after comparing a `Prepared` state to a `ProcessProposal` request if it matched.
    ///
    /// Transitions to `ExecutedBlock`.
    PreparedValid(CachedProposal),
    /// State after a `ProcessProposal` request failed comparison against a `Prepared` state.
    ///
    /// End state.
    CheckedPreparedMismatch(CachedProposal),
    /// State after executing a complete block.
    ///
    /// Transitions to `CheckedExecutedBlockMismatch`.
    ExecutedBlock {
        cached_block_hash: [u8; 32],
        cached_proposal: Option<CachedProposal>,
    },
    /// State after a `FinalizeBlock` request failed comparison against an `ExecutedBlock` state.
    ///
    /// End state.
    CheckedExecutedBlockMismatch {
        cached_block_hash: [u8; 32],
        cached_proposal: Option<CachedProposal>,
    },
}

/// State machine for tracking what state the app has executed
/// data in. This is used to check if transactions have been executed
/// in different ABCI calls across requests, and whether the cached state can be
/// used or should be reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionStateMachine(ExecutionState);

impl ExecutionStateMachine {
    pub(super) fn new() -> Self {
        Self(ExecutionState::Unset)
    }

    pub(super) fn data(&self) -> &ExecutionState {
        &self.0
    }

    #[cfg(test)]
    // This is used just to make testing easier for transitions without needing to fully setup
    fn create_with_data(data: ExecutionState) -> Self {
        Self(data)
    }

    /// Called at the end of `prepare_proposal`, it caches info from the request and response.
    ///
    /// Returns an error if the current state is not `Unset`, and `self` is left unchanged. On
    /// success, the state transitions to `Prepared`.
    pub(super) fn set_prepared_proposal(
        &mut self,
        request: abci::request::PrepareProposal,
        response: abci::response::PrepareProposal,
    ) -> Result<()> {
        if self.0 != ExecutionState::Unset {
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
            Some(CommitInfo {
                round: local_last_commit.round,
                votes: vote_info,
            })
        } else {
            None
        };
        let prepared_proposal = CachedProposal {
            time: request.time,
            proposer_address: request.proposer_address,
            txs: response.txs,
            proposed_last_commit,
            misbehavior: request.misbehavior,
            height: request.height,
            next_validators_hash: request.next_validators_hash,
        };
        self.0 = ExecutionState::Prepared(prepared_proposal);
        Ok(())
    }

    /// Given a `ProcessProposal` request, returns whether the `ProcessProposal` matches the current
    /// cached proposal or not.
    ///
    /// Returns `false` if the current state is not `Prepared` or `PreparedValid`, and `self` is
    /// left unchanged.
    ///
    /// If the request matches the cached info, the state transitions to `PreparedValid` and `true`
    /// is returned. Otherwise the state transitions to `CheckedPreparedMismatch` and `false` is
    /// returned.
    pub(super) fn check_if_prepared_proposal(
        &mut self,
        request: abci::request::ProcessProposal,
    ) -> bool {
        let cached_proposal = match &mut self.0 {
            ExecutionState::Unset
            | ExecutionState::CheckedPreparedMismatch(_)
            | ExecutionState::CheckedExecutedBlockMismatch {
                ..
            }
            | ExecutionState::ExecutedBlock {
                ..
            } => return false,
            ExecutionState::PreparedValid(cached_proposal)
            | ExecutionState::Prepared(cached_proposal) => std::mem::take(cached_proposal),
        };

        let new_proposal = CachedProposal {
            time: request.time,
            proposer_address: request.proposer_address,
            txs: request.txs,
            proposed_last_commit: request.proposed_last_commit,
            misbehavior: request.misbehavior,
            height: request.height,
            next_validators_hash: request.next_validators_hash,
        };

        if cached_proposal != new_proposal {
            self.0 = ExecutionState::CheckedPreparedMismatch(cached_proposal);
            return false;
        }

        self.0 = ExecutionState::PreparedValid(cached_proposal);
        true
    }

    /// Called after executing a block during `process_proposal` or `finalize_block`, it caches the
    /// executed CometBFT block hash.
    ///
    /// Returns an error if the current state is not `PreparedValid` or `Unset`, and `self` is left
    /// unchanged. On success, the state transitions to `ExecutedBlock`.
    pub(super) fn set_executed_block(&mut self, block_hash: [u8; 32]) -> Result<()> {
        match &mut self.0 {
            ExecutionState::Unset => {
                self.0 = ExecutionState::ExecutedBlock {
                    cached_block_hash: block_hash,
                    cached_proposal: None,
                };
            }
            ExecutionState::PreparedValid(cached_proposal) => {
                self.0 = ExecutionState::ExecutedBlock {
                    cached_block_hash: block_hash,
                    cached_proposal: Some(std::mem::take(cached_proposal)),
                };
            }
            ExecutionState::Prepared(_) => {
                bail!(
                    "executed block state attempted to be set before prepared proposal state \
                     validated",
                );
            }
            ExecutionState::ExecutedBlock {
                ..
            } => {
                bail!("executed block state attempted to be set again");
            }
            ExecutionState::CheckedPreparedMismatch(_)
            | ExecutionState::CheckedExecutedBlockMismatch {
                ..
            } => {
                bail!("executed block state shouldn't be set after invalid check");
            }
        }

        Ok(())
    }

    /// Given a block hash, returns whether it matches the current one in execution state or not.
    ///
    /// Returns `false` if the current state is not `Prepared`, `PreparedValid` or `ExecutedBlock`,
    /// and `self` is left unchanged.
    ///
    /// Returns `false` if the current state is `Prepared` or `PreparedValid`, and the state
    /// transitions to `CheckedPreparedMismatch`.
    ///
    /// If the block hash matches the cached info in the `ExecutedBlock` state, `self` is left
    /// unchanged and `true` is returned. Otherwise the state transitions to
    /// `CheckedExecutedBlockMismatch` and `false` is returned.
    pub(super) fn check_if_executed_block(&mut self, block_hash: [u8; 32]) -> bool {
        match &mut self.0 {
            ExecutionState::Unset
            | ExecutionState::CheckedPreparedMismatch(_)
            | ExecutionState::CheckedExecutedBlockMismatch {
                ..
            } => false,
            // Can only call check executed on an executed fingerprint.
            ExecutionState::Prepared(cached_proposal)
            | ExecutionState::PreparedValid(cached_proposal) => {
                self.0 = ExecutionState::CheckedPreparedMismatch(std::mem::take(cached_proposal));

                false
            }
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal,
            } => {
                if block_hash != *cached_block_hash {
                    self.0 = ExecutionState::CheckedExecutedBlockMismatch {
                        cached_block_hash: *cached_block_hash,
                        cached_proposal: std::mem::take(cached_proposal),
                    };
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

    impl From<request::PrepareProposal> for CachedProposal {
        fn from(prepare_request: request::PrepareProposal) -> Self {
            CachedProposal {
                time: prepare_request.time,
                proposer_address: prepare_request.proposer_address,
                txs: vec![],
                proposed_last_commit: None,
                misbehavior: prepare_request.misbehavior.clone(),
                next_validators_hash: prepare_request.next_validators_hash,
                height: prepare_request.height,
            }
        }
    }

    fn build_prepare_proposal(
        txs: &[Bytes],
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
        let response = response::PrepareProposal {
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
        let state = ExecutionStateMachine::new();
        assert_eq!(*state.data(), ExecutionState::Unset);
    }

    #[test]
    fn set_prepared_succeeds() {
        let (prepare_request, prepare_response) =
            build_prepare_proposal(&[vec![1, 2, 3].into(), vec![253, 254, 255].into()]);
        let mut state = ExecutionStateMachine::new();

        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();
        insta::assert_debug_snapshot!(state.data());
    }

    #[test]
    fn set_prepared_fails_on_non_unset() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request.clone());
        let cached_block_hash = [1u8; 32];

        let states_to_test = vec![
            ExecutionState::Prepared(cached_proposal.clone()),
            ExecutionState::PreparedValid(cached_proposal.clone()),
            ExecutionState::CheckedPreparedMismatch(cached_proposal.clone()),
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal: None,
            },
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal: Some(cached_proposal.clone()),
            },
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: None,
            },
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: Some(cached_proposal),
            },
        ];

        for state in states_to_test {
            let mut state_machine = ExecutionStateMachine::create_with_data(state);
            let data_copy = state_machine.data().clone();
            let _ = state_machine
                .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
                .unwrap_err();
            assert_eq!(*state_machine.data(), data_copy);
        }
    }

    #[test]
    fn check_if_prepared_validates_successfully() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let matching_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), true);
        let mut state = ExecutionStateMachine::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();

        assert!(state.check_if_prepared_proposal(matching_request.clone()));
        insta::assert_debug_snapshot!(state.data());
        let data_copy = state.data().clone();

        // Should validate a second time, don't need a second snapshot
        assert!(state.check_if_prepared_proposal(matching_request));
        assert_eq!(*state.data(), data_copy);
    }

    #[test]
    fn check_if_prepared_on_prepared_valid_will_invalidate_mismatch() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let match_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), true);
        let mismatch_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), false);
        let mut state = ExecutionStateMachine::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();

        // First get validated then attempt again with mismatch
        assert!(state.check_if_prepared_proposal(match_request.clone()));
        assert!(!state.check_if_prepared_proposal(mismatch_request));
        insta::assert_debug_snapshot!(state.data());

        // Shouldn't change back to passing
        assert!(!state.check_if_prepared_proposal(match_request));
    }

    #[test]
    fn check_if_prepared_invalidates_mismatch() {
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let process_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), false);
        let mut state = ExecutionStateMachine::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();

        assert!(!state.check_if_prepared_proposal(process_request));
        insta::assert_debug_snapshot!(state.data());
    }

    #[test]
    fn check_if_prepared_false_on_ineligible_states() {
        let cached_block_hash = [1u8; 32];
        let (prepare_request, prepare_response) = build_prepare_proposal(&[]);
        let match_request =
            build_process_proposal(prepare_request.clone(), prepare_response.clone(), true);
        let mut state = ExecutionStateMachine::new();
        state
            .set_prepared_proposal(prepare_request.clone(), prepare_response.clone())
            .unwrap();
        let ExecutionState::Prepared(cached_proposal) = state.data().clone() else {
            panic!("Expected Prepared state");
        };

        // Each one of these states should return false and not change the state.
        let states_to_test = vec![
            ExecutionState::Unset,
            ExecutionState::CheckedPreparedMismatch(cached_proposal.clone()),
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal: None,
            },
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal: Some(cached_proposal.clone()),
            },
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: None,
            },
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: Some(cached_proposal),
            },
        ];

        for state in states_to_test {
            let mut state = ExecutionStateMachine::create_with_data(state);
            let data_copy = state.data().clone();
            assert!(!state.check_if_prepared_proposal(match_request.clone()));
            assert_eq!(*state.data(), data_copy);
        }
    }

    #[test]
    fn set_execute_block_without_prepared_succeeds() {
        let cached_block_hash = [1u8; 32];
        let mut state = ExecutionStateMachine::new();

        state.set_executed_block(cached_block_hash).unwrap();
        assert_eq!(
            *state.data(),
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal: None
            }
        );
    }

    #[test]
    fn set_execute_block_with_prepared_validated_succeeds() {
        let cached_block_hash = [1u8; 32];
        let (prepare_request, _prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request);
        let mut state = ExecutionStateMachine::create_with_data(ExecutionState::PreparedValid(
            cached_proposal.clone(),
        ));

        state.set_executed_block(cached_block_hash).unwrap();
        assert_eq!(
            *state.data(),
            ExecutionState::ExecutedBlock {
                cached_block_hash,
                cached_proposal: Some(cached_proposal)
            }
        );
    }

    #[test]
    fn set_execute_block_with_non_settable_states_errors() {
        let cached_block_hash = [1u8; 32];
        let mut errors = vec![];

        // All of these should error, and then we will validate errors don't change with snapshot.
        let (prepare_request, _prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request);
        let mut state = ExecutionStateMachine::create_with_data(ExecutionState::Prepared(
            cached_proposal.clone(),
        ));
        errors.push(state.set_executed_block(cached_block_hash).unwrap_err());
        state = ExecutionStateMachine::create_with_data(ExecutionState::CheckedPreparedMismatch(
            cached_proposal.clone(),
        ));
        errors.push(state.set_executed_block(cached_block_hash).unwrap_err());
        state = ExecutionStateMachine::create_with_data(ExecutionState::ExecutedBlock {
            cached_block_hash,
            cached_proposal: Some(cached_proposal.clone()),
        });
        errors.push(state.set_executed_block(cached_block_hash).unwrap_err());
        state =
            ExecutionStateMachine::create_with_data(ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: Some(cached_proposal),
            });
        errors.push(state.set_executed_block(cached_block_hash).unwrap_err());

        insta::assert_debug_snapshot!(errors);
    }

    #[test]
    fn check_if_execute_block_succeeds_on_matching() {
        let cached_block_hash = [1u8; 32];
        let mut state = ExecutionStateMachine::create_with_data(ExecutionState::ExecutedBlock {
            cached_block_hash,
            cached_proposal: None,
        });
        let data_copy = state.data().clone();

        assert!(state.check_if_executed_block(cached_block_hash));
        assert_eq!(*state.data(), data_copy);

        let (prepare_request, _prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request);
        state = ExecutionStateMachine::create_with_data(ExecutionState::ExecutedBlock {
            cached_block_hash,
            cached_proposal: Some(cached_proposal),
        });
        let data_copy = state.data().clone();

        assert!(state.check_if_executed_block(cached_block_hash));
        assert_eq!(*state.data(), data_copy);
    }

    #[test]
    fn check_if_execute_block_fails_on_non_matching() {
        let cached_block_hash = [1u8; 32];
        let mut state = ExecutionStateMachine::create_with_data(ExecutionState::ExecutedBlock {
            cached_block_hash,
            cached_proposal: None,
        });

        assert!(!state.check_if_executed_block([2u8; 32]));
        assert_eq!(
            *state.data(),
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: None
            }
        );

        let (prepare_request, _prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request);
        state = ExecutionStateMachine::create_with_data(ExecutionState::ExecutedBlock {
            cached_block_hash,
            cached_proposal: Some(cached_proposal.clone()),
        });
        assert!(!state.check_if_executed_block([2u8; 32]));
        assert_eq!(
            *state.data(),
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: Some(cached_proposal)
            }
        );
    }

    #[test]
    fn check_if_execute_block_fails_on_prepared_states() {
        let block_hash = [1u8; 32];
        let (prepare_request, _prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request);
        let mut state = ExecutionStateMachine::create_with_data(ExecutionState::Prepared(
            cached_proposal.clone(),
        ));

        assert!(!state.check_if_executed_block(block_hash));
        assert_eq!(
            *state.data(),
            ExecutionState::CheckedPreparedMismatch(cached_proposal.clone())
        );

        // Should also fail and state should match what is called on prepared state
        state = ExecutionStateMachine::create_with_data(ExecutionState::PreparedValid(
            cached_proposal.clone(),
        ));
        assert!(!state.check_if_executed_block(block_hash));
        assert_eq!(
            *state.data(),
            ExecutionState::CheckedPreparedMismatch(cached_proposal)
        );
    }

    #[test]
    fn check_if_execute_block_fails_on_ineligible_states() {
        let cached_block_hash = [1u8; 32];

        // Each one of these states should return false and not change the state.
        let (prepare_request, _prepare_response) = build_prepare_proposal(&[]);
        let cached_proposal = CachedProposal::from(prepare_request);
        let states_to_test = vec![
            ExecutionState::Unset,
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: None,
            },
            ExecutionState::CheckedExecutedBlockMismatch {
                cached_block_hash,
                cached_proposal: Some(cached_proposal.clone()),
            },
        ];

        for state in states_to_test {
            let mut state = ExecutionStateMachine::create_with_data(state);
            let data_copy = state.data().clone();
            assert!(!state.check_if_executed_block(cached_block_hash));
            assert_eq!(*state.data(), data_copy);
        }
    }
}
