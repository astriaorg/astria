use astria_eyre::eyre::{
    bail,
    Result,
};
use tendermint::{
    abci,
    Hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProposalFingerprintData {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProposalFingerprint(ProposalFingerprintData);

impl ProposalFingerprint {
    pub(crate) fn new() -> Self {
        Self(ProposalFingerprintData::Unset)
    }

    pub(crate) fn data(&self) -> ProposalFingerprintData {
        self.0
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
        if self.0 != ProposalFingerprintData::Unset {
            bail!("ProposalFingerprint already set");
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
        self.0 = ProposalFingerprintData::Prepared(data);
        Ok(())
    }

    // Given a ProcessProposal request, check the ProcessProposal matches
    // the current fingerprint. If in `Prepared` state
    // will create a partial ProcessProposal message, serialize and hash that
    // to compare. If it does not match, the status is set to `CheckedPreparedMismatch`.
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
            ProposalFingerprintData::Unset
            | ProposalFingerprintData::CheckedPreparedMismatch(_)
            | ProposalFingerprintData::CheckedExecutedBlockMismatch(..)
            | ProposalFingerprintData::ExecutedBlock(..) => false,
            ProposalFingerprintData::PreparedValid(_) => true,
            ProposalFingerprintData::Prepared(proposal_hash) => {
                let partial_proposal = abci::request::ProcessProposal {
                    hash: Hash::default(),
                    ..proposal.clone()
                };
                let pb_data = pb::RequestProcessProposal::from(partial_proposal).encode_to_vec();
                let data: [u8; 32] = Sha256::digest(pb_data).into();
                if proposal_hash != data {
                    self.0 = ProposalFingerprintData::CheckedPreparedMismatch(proposal_hash);
                    return false;
                }

                self.0 = ProposalFingerprintData::PreparedValid(proposal_hash);
                true
            }
        }
    }

    // Called after `process_proposal` has been called or `finalize_block` to set
    // to a `ExecutedBlock` fingerprint. Can only be called on a `Prepared`
    // or `Unset` fingerprint, otherwise will error.
    pub(crate) fn set_executed_block(&mut self, block_hash: [u8; 32]) -> Result<()> {
        match self.0 {
            ProposalFingerprintData::Unset => {
                self.0 = ProposalFingerprintData::ExecutedBlock(block_hash, None);
            }
            ProposalFingerprintData::PreparedValid(proposal_hash) => {
                self.0 = ProposalFingerprintData::ExecutedBlock(block_hash, Some(proposal_hash));
            }
            ProposalFingerprintData::Prepared(_) => {
                bail!(
                    "executed block fingerprint attempted to be set before prepared proposal \
                     fingerprint validated.",
                );
            }
            ProposalFingerprintData::ExecutedBlock(..) => {
                bail!("executed block fingerprint attempted to be set again.",);
            }
            ProposalFingerprintData::CheckedPreparedMismatch(_)
            | ProposalFingerprintData::CheckedExecutedBlockMismatch(..) => {
                bail!("executed block fingerprint shouldn't be set after invalid check.",);
            }
        }

        Ok(())
    }

    // Given a block hash, check if it matches the current fingerprint.
    //
    // If checking against an `ExecutedBlock` fingerprint, will compare the hash, update
    // the status to `CheckedExecutedBlockMismatch` if it does not match.
    //
    // Should not be called on a `Prepared` fingerprint, will change status
    // to `CheckedPreparedMismatch`.
    pub(crate) fn check_if_executed_block(&mut self, block_hash: [u8; 32]) -> bool {
        match self.0 {
            ProposalFingerprintData::Unset
            | ProposalFingerprintData::CheckedPreparedMismatch(_)
            | ProposalFingerprintData::CheckedExecutedBlockMismatch(..) => false,
            // Can only call check executed on an executed fingerprint.
            ProposalFingerprintData::Prepared(proposal_hash)
            | ProposalFingerprintData::PreparedValid(proposal_hash) => {
                self.0 = ProposalFingerprintData::CheckedPreparedMismatch(proposal_hash);

                false
            }
            ProposalFingerprintData::ExecutedBlock(cached_block_hash, proposal_hash) => {
                if block_hash != cached_block_hash {
                    self.0 = ProposalFingerprintData::CheckedExecutedBlockMismatch(
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
