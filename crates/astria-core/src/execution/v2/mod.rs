use std::num::NonZeroU64;

use pbjson_types::Timestamp;

use crate::{
    generated::astria::execution::v2 as raw,
    primitive::v1::{
        IncorrectRollupIdLength,
        RollupId,
    },
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error("failed validating Protobuf `astria.execution.v2.ExecutionSession`")]
pub struct ExecutionSessionError(ExecutionSessionErrorKind);

impl ExecutionSessionError {
    fn execution_session_parameters(source: ExecutionSessionParametersError) -> Self {
        Self(
            ExecutionSessionErrorKind::InvalidExecutionSessionParameters {
                source,
            },
        )
    }

    fn commitment_state(source: CommitmentStateError) -> Self {
        Self(ExecutionSessionErrorKind::InvalidCommitmentState {
            source,
        })
    }

    fn missing_execution_session_parameters() -> Self {
        Self(ExecutionSessionErrorKind::MissingExecutionSessionParameters)
    }

    fn missing_commitment_state() -> Self {
        Self(ExecutionSessionErrorKind::MissingCommitmentState)
    }
}

#[derive(Debug, thiserror::Error)]
enum ExecutionSessionErrorKind {
    #[error("invalid field `.execution_session_parameters`")]
    InvalidExecutionSessionParameters {
        source: ExecutionSessionParametersError,
    },
    #[error("invalid field `.commitment_state`")]
    InvalidCommitmentState { source: CommitmentStateError },
    #[error("field `.execution_session_parameters` was not set")]
    MissingExecutionSessionParameters,
    #[error("field `.commitment_state` was not set")]
    MissingCommitmentState,
}

/// `ExecutionSession` contains the information needed to drive the full execution
/// of a rollup chain in the rollup.
///
/// The execution session is only valid for the execution config params with
/// which it was created. Once all blocks within the session have been executed,
/// the execution client must request a new session. The `session_id` is used to
/// to track which session is being used.
#[derive(Debug)]
pub struct ExecutionSession {
    /// An ID for the session.
    session_id: String,
    /// The configuration for the execution session.
    parameters: ExecutionSessionParameters,
    /// The commitment state for executing client to start from.
    commitment_state: CommitmentState,
}

impl ExecutionSession {
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn execution_session_parameters(&self) -> &ExecutionSessionParameters {
        &self.parameters
    }

    #[must_use]
    pub fn commitment_state(&self) -> &CommitmentState {
        &self.commitment_state
    }
}

impl Protobuf for ExecutionSession {
    type Error = ExecutionSessionError;
    type Raw = raw::ExecutionSession;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::ExecutionSession {
            session_id,
            execution_session_parameters,
            commitment_state,
        } = raw;
        let execution_session_parameters = execution_session_parameters
            .as_ref()
            .ok_or_else(Self::Error::missing_execution_session_parameters)?;
        let execution_session_parameters =
            ExecutionSessionParameters::try_from_raw_ref(execution_session_parameters)
                .map_err(Self::Error::execution_session_parameters)?;
        let commitment_state = commitment_state
            .as_ref()
            .ok_or_else(Self::Error::missing_commitment_state)?;
        let commitment_state = CommitmentState::try_from_raw_ref(commitment_state)
            .map_err(Self::Error::commitment_state)?;
        Ok(Self {
            session_id: session_id.clone(),
            parameters: execution_session_parameters,
            commitment_state,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            session_id,
            parameters: execution_session_parameters,
            commitment_state,
        } = self;
        let execution_session_parameters = execution_session_parameters.to_raw();
        let commitment_state = commitment_state.to_raw();
        Self::Raw {
            session_id: session_id.clone(),
            execution_session_parameters: Some(execution_session_parameters),
            commitment_state: Some(commitment_state),
        }
    }
}

// An error when transforming a [`raw::ExecutionSessionParameters`] into a
// [`ExecutionSessionParameters`].
#[derive(Debug, thiserror::Error)]
#[error("failed to validate Protobuf `astria.execution.v2.ExecutionSessionParameters`")]
pub struct ExecutionSessionParametersError(ExecutionSessionParametersErrorKind);

impl ExecutionSessionParametersError {
    fn incorrect_rollup_id_length(source: IncorrectRollupIdLength) -> Self {
        Self(
            ExecutionSessionParametersErrorKind::IncorrectRollupIdLength {
                source,
            },
        )
    }

    fn no_rollup_id() -> Self {
        Self(ExecutionSessionParametersErrorKind::NoRollupId)
    }

    fn invalid_sequencer_start_block_height(source: tendermint::Error) -> Self {
        Self(
            ExecutionSessionParametersErrorKind::InvalidSequencerStartBlockHeight {
                source,
            },
        )
    }
}

#[derive(Debug, thiserror::Error)]
enum ExecutionSessionParametersErrorKind {
    #[error("field `.rollup_id` was invalid")]
    IncorrectRollupIdLength { source: IncorrectRollupIdLength },
    #[error("field `.rollup_id` was not set")]
    NoRollupId,
    #[error("field `.sequencer_start_block_height` was invalid")]
    InvalidSequencerStartBlockHeight { source: tendermint::Error },
}

/// Genesis Info required from a rollup to start an execution client.
///
/// Contains information about the rollup id, and base heights for both sequencer & celestia.
///
/// Usually constructed its [`Protobuf`] implementation from a
/// [`raw::ExecutionSessionParameters`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::astria::execution::v2::ExecutionSessionParameters")
)]
pub struct ExecutionSessionParameters {
    /// The `rollup_id` is the unique identifier for the rollup chain.
    rollup_id: RollupId,
    /// The first rollup block number to be executed. This is mapped to
    /// `sequencer_first_block_height`. The minimum first block number is 1, since 0 represents
    /// the genesis block. Implementors should reject a value of 0.
    ///
    /// Servers implementing this API should reject execution of blocks below this
    /// value with an `OUT_OF_RANGE` error code.
    rollup_start_block_number: u64,
    /// The final rollup block number to execute as part of a session.
    ///
    /// If not set or set to 0, the execution session does not have an upper bound.
    ///
    /// Servers implementing this API should reject execution of blocks past this
    /// value with an `OUT_OF_RANGE` error code.
    rollup_end_block_number: Option<NonZeroU64>,
    /// The ID of the Astria Sequencer network to retrieve Sequencer blocks from.
    /// Conductor implementations should verify that the Sequencer network they are
    /// connected to have this chain ID (if fetching soft Sequencer blocks), and verify
    /// that the Sequencer metadata blobs retrieved from Celestia contain this chain
    /// ID (if extracting firm Sequencer blocks from Celestia blobs).
    sequencer_chain_id: String,
    /// The first block height on the sequencer chain to use for rollup transactions.
    /// This is mapped to `rollup_start_block_number`.
    sequencer_start_block_height: tendermint::block::Height,
    /// The ID of the Celestia network to retrieve blobs from.
    /// Conductor implementations should verify that the Celestia network they are
    /// connected to have this chain ID (if extracting firm Sequencer blocks from
    /// Celestia blobs).
    celestia_chain_id: String,
    /// The maximum number of Celestia blocks which can be read above
    /// `CommitmentState.lowest_celestia_search_height` in search of the next firm
    /// block.
    ///
    /// Cannot be set to 0 if Conductor is configured to use firm commitments. If
    /// Conductor is in soft-only mode, this value is ignored.
    celestia_search_height_max_look_ahead: u64,
}

impl ExecutionSessionParameters {
    #[must_use]
    pub fn new(
        rollup_id: RollupId,
        rollup_start_block_number: u64,
        rollup_end_block_number: u64,
        sequencer_chain_id: String,
        sequencer_start_block_height: tendermint::block::Height,
        celestia_chain_id: String,
        celestia_search_height_max_look_ahead: u64,
    ) -> Self {
        Self {
            rollup_id,
            rollup_start_block_number,
            rollup_end_block_number: NonZeroU64::new(rollup_end_block_number),
            sequencer_chain_id,
            sequencer_start_block_height,
            celestia_chain_id,
            celestia_search_height_max_look_ahead,
        }
    }

    #[must_use]
    pub fn rollup_id(&self) -> RollupId {
        self.rollup_id
    }

    #[must_use]
    pub fn rollup_start_block_number(&self) -> u64 {
        self.rollup_start_block_number
    }

    #[must_use]
    pub fn rollup_end_block_number(&self) -> Option<NonZeroU64> {
        self.rollup_end_block_number
    }

    #[must_use]
    pub fn sequencer_start_block_height(&self) -> u64 {
        self.sequencer_start_block_height.into()
    }

    #[must_use]
    pub fn sequencer_chain_id(&self) -> &String {
        &self.sequencer_chain_id
    }

    #[must_use]
    pub fn celestia_chain_id(&self) -> &String {
        &self.celestia_chain_id
    }

    #[must_use]
    pub fn celestia_search_height_max_look_ahead(&self) -> u64 {
        self.celestia_search_height_max_look_ahead
    }
}

impl From<ExecutionSessionParameters> for raw::ExecutionSessionParameters {
    fn from(value: ExecutionSessionParameters) -> Self {
        value.to_raw()
    }
}

impl Protobuf for ExecutionSessionParameters {
    type Error = ExecutionSessionParametersError;
    type Raw = raw::ExecutionSessionParameters;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::ExecutionSessionParameters {
            rollup_id,
            rollup_start_block_number,
            rollup_end_block_number,
            sequencer_chain_id,
            sequencer_start_block_height,
            celestia_chain_id,
            celestia_search_height_max_look_ahead,
        } = raw;
        let Some(rollup_id) = rollup_id else {
            return Err(Self::Error::no_rollup_id());
        };
        let rollup_id = RollupId::try_from_raw_ref(rollup_id)
            .map_err(Self::Error::incorrect_rollup_id_length)?;
        let sequencer_start_block_height =
            tendermint::block::Height::try_from(*sequencer_start_block_height)
                .map_err(Self::Error::invalid_sequencer_start_block_height)?;

        Ok(Self {
            rollup_id,
            rollup_start_block_number: *rollup_start_block_number,
            rollup_end_block_number: NonZeroU64::new(*rollup_end_block_number),
            sequencer_chain_id: sequencer_chain_id.clone(),
            sequencer_start_block_height,
            celestia_chain_id: celestia_chain_id.clone(),
            celestia_search_height_max_look_ahead: *celestia_search_height_max_look_ahead,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            rollup_id,
            rollup_start_block_number,
            rollup_end_block_number,
            sequencer_chain_id,
            sequencer_start_block_height,
            celestia_chain_id,
            celestia_search_height_max_look_ahead,
        } = self;

        Self::Raw {
            rollup_id: Some(rollup_id.to_raw()),
            rollup_start_block_number: *rollup_start_block_number,
            rollup_end_block_number: rollup_end_block_number.map(NonZeroU64::get).unwrap_or(0),
            sequencer_chain_id: sequencer_chain_id.clone(),
            sequencer_start_block_height: sequencer_start_block_height.value(),
            celestia_chain_id: celestia_chain_id.clone(),
            celestia_search_height_max_look_ahead: *celestia_search_height_max_look_ahead,
        }
    }
}

/// An error when transforming a [`raw::ExecutedBlockMetadata`] into a [`ExecutedBlockMetadata`].
#[derive(Debug, thiserror::Error)]
#[error("failed to validate Protobuf `astria.execution.v2.ExecutedBlockMetadata`")]
pub struct ExecutedBlockMetadataError(ExecutedBlockMetadataErrorKind);

impl ExecutedBlockMetadataError {
    fn field_not_set(field: &'static str) -> Self {
        Self(ExecutedBlockMetadataErrorKind::FieldNotSet(field))
    }
}

#[derive(Debug, thiserror::Error)]
enum ExecutedBlockMetadataErrorKind {
    #[error("field `.{0}` not set")]
    FieldNotSet(&'static str),
}

/// An Astria execution block on a rollup.
///
/// Contains information about the block number, its hash,
/// its parent block's hash, and timestamp.
///
/// Usually constructed its [`Protobuf`] implementation from a
/// [`raw::ExecutedBlockMetadata`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::astria::execution::v2::ExecutedBlockMetadata")
)]
pub struct ExecutedBlockMetadata {
    /// The block number
    number: u64,
    /// The hash of the block
    hash: String,
    /// The hash of the parent block
    parent_hash: String,
    /// Timestamp of the block, taken from the sequencer block that this rollup block
    /// was constructed from.
    timestamp: Timestamp,
    /// The hash of the sequencer block that this rollup block was constructed from.
    sequencer_block_hash: String,
}

impl ExecutedBlockMetadata {
    #[must_use]
    pub fn number(&self) -> u64 {
        self.number
    }

    #[must_use]
    pub fn hash(&self) -> &str {
        &self.hash
    }

    #[must_use]
    pub fn parent_hash(&self) -> &str {
        &self.parent_hash
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        // prost_types::Timestamp is a (i64, i32) tuple, so this is
        // effectively just a copy
        self.timestamp
    }

    #[must_use]
    pub fn sequencer_block_hash(&self) -> &str {
        &self.sequencer_block_hash
    }
}

impl From<ExecutedBlockMetadata> for raw::ExecutedBlockMetadata {
    fn from(value: ExecutedBlockMetadata) -> Self {
        value.to_raw()
    }
}

impl Protobuf for ExecutedBlockMetadata {
    type Error = ExecutedBlockMetadataError;
    type Raw = raw::ExecutedBlockMetadata;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::ExecutedBlockMetadata {
            number,
            hash,
            parent_hash,
            timestamp,
            sequencer_block_hash,
        } = raw;
        // Cloning timestamp is effectively a copy because timestamp is just a (i32, i64) tuple
        let timestamp = timestamp.ok_or(Self::Error::field_not_set(".timestamp"))?;

        Ok(Self {
            number: *number,
            hash: hash.clone(),
            parent_hash: parent_hash.clone(),
            timestamp,
            sequencer_block_hash: sequencer_block_hash.clone(),
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            number,
            hash,
            parent_hash,
            timestamp,
            sequencer_block_hash,
        } = self;
        Self::Raw {
            number: *number,
            hash: hash.clone(),
            parent_hash: parent_hash.clone(),
            // Cloning timestamp is effectively a copy because timestamp is just a (i32, i64)
            // tuple
            timestamp: Some(*timestamp),
            sequencer_block_hash: sequencer_block_hash.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed validating Protobuf `astria.execution.v2.CommitmentState`")]
pub struct CommitmentStateError(CommitmentStateErrorKind);

impl CommitmentStateError {
    fn field_not_set(field: &'static str) -> Self {
        Self(CommitmentStateErrorKind::FieldNotSet(field))
    }

    fn firm(source: ExecutedBlockMetadataError) -> Self {
        Self(CommitmentStateErrorKind::Firm {
            source,
        })
    }

    fn soft(source: ExecutedBlockMetadataError) -> Self {
        Self(CommitmentStateErrorKind::Soft {
            source,
        })
    }

    fn firm_exceeds_soft(source: FirmExceedsSoft) -> Self {
        Self(CommitmentStateErrorKind::FirmExceedsSoft {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum CommitmentStateErrorKind {
    #[error("field `.{0}` not set")]
    FieldNotSet(&'static str),
    #[error("field `.firm` was invalid")]
    Firm { source: ExecutedBlockMetadataError },
    #[error("field `.soft` was invalid")]
    Soft { source: ExecutedBlockMetadataError },
    #[error("firm commitment height exceeded soft commitment height")]
    FirmExceedsSoft { source: FirmExceedsSoft },
}

#[derive(Debug, thiserror::Error)]
#[error("firm commitment at `{firm}` exceeds soft commitment at `{soft}`")]
pub struct FirmExceedsSoft {
    firm: u64,
    soft: u64,
}

pub struct NoFirm;
pub struct NoSoft;
pub struct NoBaseCelestiaHeight;
pub struct WithFirm(ExecutedBlockMetadata);
pub struct WithSoft(ExecutedBlockMetadata);
pub struct WithLowestCelestiaSearchHeight(u64);
#[derive(Default)]
pub struct CommitmentStateBuilder<
    TFirm = NoFirm,
    TSoft = NoSoft,
    TBaseCelestiaHeight = NoBaseCelestiaHeight,
> {
    firm_executed_block_metadata: TFirm,
    soft_executed_block_metadata: TSoft,
    lowest_celestia_search_height: TBaseCelestiaHeight,
}

impl CommitmentStateBuilder<NoFirm, NoSoft, NoBaseCelestiaHeight> {
    fn new() -> Self {
        Self {
            firm_executed_block_metadata: NoFirm,
            soft_executed_block_metadata: NoSoft,
            lowest_celestia_search_height: NoBaseCelestiaHeight,
        }
    }
}

impl<TFirm, TSoft, TCelestiaBaseHeight> CommitmentStateBuilder<TFirm, TSoft, TCelestiaBaseHeight> {
    pub fn firm_executed_block_metadata(
        self,
        firm_executed_block_metadata: ExecutedBlockMetadata,
    ) -> CommitmentStateBuilder<WithFirm, TSoft, TCelestiaBaseHeight> {
        let Self {
            soft_executed_block_metadata,
            lowest_celestia_search_height,
            ..
        } = self;
        CommitmentStateBuilder {
            firm_executed_block_metadata: WithFirm(firm_executed_block_metadata),
            soft_executed_block_metadata,
            lowest_celestia_search_height,
        }
    }

    pub fn soft_executed_block_metadata(
        self,
        soft_executed_block_metadata: ExecutedBlockMetadata,
    ) -> CommitmentStateBuilder<TFirm, WithSoft, TCelestiaBaseHeight> {
        let Self {
            firm_executed_block_metadata,
            lowest_celestia_search_height,
            ..
        } = self;
        CommitmentStateBuilder {
            firm_executed_block_metadata,
            soft_executed_block_metadata: WithSoft(soft_executed_block_metadata),
            lowest_celestia_search_height,
        }
    }

    pub fn lowest_celestia_search_height(
        self,
        lowest_celestia_search_height: u64,
    ) -> CommitmentStateBuilder<TFirm, TSoft, WithLowestCelestiaSearchHeight> {
        let Self {
            firm_executed_block_metadata,
            soft_executed_block_metadata,
            ..
        } = self;
        CommitmentStateBuilder {
            firm_executed_block_metadata,
            soft_executed_block_metadata,
            lowest_celestia_search_height: WithLowestCelestiaSearchHeight(
                lowest_celestia_search_height,
            ),
        }
    }
}

impl CommitmentStateBuilder<WithFirm, WithSoft, WithLowestCelestiaSearchHeight> {
    /// Finalize the commitment state.
    ///
    /// # Errors
    /// Returns an error if the firm block exceeds the soft one.
    pub fn build(self) -> Result<CommitmentState, FirmExceedsSoft> {
        let Self {
            firm_executed_block_metadata: WithFirm(firm_executed_block_metadata),
            soft_executed_block_metadata: WithSoft(soft_executed_block_metadata),
            lowest_celestia_search_height:
                WithLowestCelestiaSearchHeight(lowest_celestia_search_height),
        } = self;
        if firm_executed_block_metadata.number() > soft_executed_block_metadata.number() {
            return Err(FirmExceedsSoft {
                firm: firm_executed_block_metadata.number(),
                soft: soft_executed_block_metadata.number(),
            });
        }
        Ok(CommitmentState {
            soft_executed_block_metadata,
            firm_executed_block_metadata,
            lowest_celestia_search_height,
        })
    }
}

/// The `CommitmentState` holds the block at each stage of sequencer commitment
/// level
///
/// A Valid `CommitmentState`:
/// - Block numbers are such that soft >= firm.
/// - No blocks ever decrease in block number.
/// - The chain defined by soft is the head of the canonical chain the firm block must belong to.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::astria::execution::v2::CommitmentState")
)]
pub struct CommitmentState {
    /// Soft committed block metadata derived directly from an Astria sequencer block.
    soft_executed_block_metadata: ExecutedBlockMetadata,
    /// Firm committed block metadata derived from a Sequencer block that has been
    /// written to the data availability layer (Celestia).
    firm_executed_block_metadata: ExecutedBlockMetadata,
    /// The lowest Celestia height that will be searched for the next firm block.
    /// This information is stored as part of `CommitmentState` so that it will be
    /// routinely updated as new firm blocks are received, and so that the execution
    /// client will not need to search from Celestia genesis.
    lowest_celestia_search_height: u64,
}

impl CommitmentState {
    #[must_use = "a commitment state must be built to be useful"]
    pub fn builder() -> CommitmentStateBuilder {
        CommitmentStateBuilder::new()
    }

    #[must_use]
    pub fn firm(&self) -> &ExecutedBlockMetadata {
        &self.firm_executed_block_metadata
    }

    #[must_use]
    pub fn soft(&self) -> &ExecutedBlockMetadata {
        &self.soft_executed_block_metadata
    }

    #[must_use]
    pub fn lowest_celestia_search_height(&self) -> u64 {
        self.lowest_celestia_search_height
    }
}

impl From<CommitmentState> for raw::CommitmentState {
    fn from(value: CommitmentState) -> Self {
        value.to_raw()
    }
}

impl Protobuf for CommitmentState {
    type Error = CommitmentStateError;
    type Raw = raw::CommitmentState;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            soft_executed_block_metadata,
            firm_executed_block_metadata,
            lowest_celestia_search_height,
        } = raw;
        let soft_executed_block_metadata = 'soft: {
            let Some(soft) = soft_executed_block_metadata else {
                break 'soft Err(Self::Error::field_not_set(".soft"));
            };
            ExecutedBlockMetadata::try_from_raw_ref(soft).map_err(Self::Error::soft)
        }?;
        let firm_executed_block_metadata = 'firm: {
            let Some(firm) = firm_executed_block_metadata else {
                break 'firm Err(Self::Error::field_not_set(".firm"));
            };
            ExecutedBlockMetadata::try_from_raw_ref(firm).map_err(Self::Error::firm)
        }?;

        Self::builder()
            .firm_executed_block_metadata(firm_executed_block_metadata)
            .soft_executed_block_metadata(soft_executed_block_metadata)
            .lowest_celestia_search_height(*lowest_celestia_search_height)
            .build()
            .map_err(Self::Error::firm_exceeds_soft)
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            soft_executed_block_metadata,
            firm_executed_block_metadata,
            lowest_celestia_search_height,
        } = self;
        let soft_executed_block_metadata = soft_executed_block_metadata.to_raw();
        let firm_executed_block_metadata = firm_executed_block_metadata.to_raw();
        let lowest_celestia_search_height = *lowest_celestia_search_height;
        Self::Raw {
            soft_executed_block_metadata: Some(soft_executed_block_metadata),
            firm_executed_block_metadata: Some(firm_executed_block_metadata),
            lowest_celestia_search_height,
        }
    }
}
