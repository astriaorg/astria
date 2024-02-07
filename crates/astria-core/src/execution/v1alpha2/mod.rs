use prost_types::Timestamp;

use crate::{
    generated::execution::v1alpha2 as raw,
    sequencer::v1alpha1::{
        IncorrectRollupIdLength,
        RollupId,
    },
    Protobuf,
};

// An error when transforming a [`raw::GenesisInfo`] into a [`GenesisInfo`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct GenesisInfoError(GenesisInfoErrorKind);

impl GenesisInfoError {
    fn incorrect_rollup_id_length(inner: IncorrectRollupIdLength) -> Self {
        Self(GenesisInfoErrorKind::IncorrectRollupIdLength(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum GenesisInfoErrorKind {
    #[error("`rollup_id` field did not contain a valid rollup ID")]
    IncorrectRollupIdLength(IncorrectRollupIdLength),
}

/// Genesis Info required from a rollup to start a an execution client.
///
/// Contains information about the rollup id, and base heights for both sequencer & celestia.
///
/// Usually constructed its [`Protobuf`] implementation from a
/// [`raw::GenesisInfo`].
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GenesisInfo {
    /// The rollup id which is used to identify the rollup txs.
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serde::string::display")
    )]
    rollup_id: RollupId,
    /// The first height of sequencer which is used to identify the first block of the rollup.
    sequencer_genesis_block_height: tendermint::block::Height,
    /// The first height of celestia which to look for sequencer blocks.
    celestia_base_block_height: celestia_tendermint::block::Height,
    /// The allowed variance in the block height of celestia when looking for sequencer blocks.
    celestia_block_variance: u32,
}

impl GenesisInfo {
    #[must_use]
    pub fn rollup_id(&self) -> RollupId {
        self.rollup_id
    }

    #[must_use]
    pub fn sequencer_genesis_block_height(&self) -> tendermint::block::Height {
        self.sequencer_genesis_block_height
    }

    #[must_use]
    pub fn celestia_base_block_height(&self) -> celestia_tendermint::block::Height {
        self.celestia_base_block_height
    }

    #[must_use]
    pub fn celestia_block_variance(&self) -> u32 {
        self.celestia_block_variance
    }
}

impl Protobuf for GenesisInfo {
    type Error = GenesisInfoError;
    type Raw = raw::GenesisInfo;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::GenesisInfo {
            rollup_id,
            sequencer_genesis_block_height,
            celestia_base_block_height,
            celestia_block_variance,
        } = raw;
        let rollup_id =
            RollupId::try_from_slice(rollup_id).map_err(Self::Error::incorrect_rollup_id_length)?;

        Ok(Self {
            rollup_id,
            sequencer_genesis_block_height: (*sequencer_genesis_block_height).into(),
            celestia_base_block_height: (*celestia_base_block_height).into(),
            celestia_block_variance: *celestia_block_variance,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            rollup_id,
            sequencer_genesis_block_height,
            celestia_base_block_height,
            celestia_block_variance,
        } = self;

        let sequencer_genesis_block_height: u32 =
            (*sequencer_genesis_block_height).value().try_into().expect(
                "block height overflow, this should not happen since tendermint heights are i64 \
                 under the hood",
            );
        let celestia_base_block_height: u32 =
            (*celestia_base_block_height).value().try_into().expect(
                "block height overflow, this should not happen since tendermint heights are i64 \
                 under the hood",
            );
        Self::Raw {
            rollup_id: rollup_id.to_vec(),
            sequencer_genesis_block_height,
            celestia_base_block_height,
            celestia_block_variance: *celestia_block_variance,
        }
    }
}

/// An error when transforming a [`raw::Block`] into a [`Block`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BlockError(BlockErrorKind);

impl BlockError {
    fn field_not_set(field: &'static str) -> Self {
        Self(BlockErrorKind::FieldNotSet(field))
    }

    fn incorrect_block_hash_length(wrong_hash: &[u8]) -> Self {
        Self(BlockErrorKind::IncorrectBlockHashLength(wrong_hash.len()))
    }

    fn incorrect_parent_block_hash_length(wrong_hash: &[u8]) -> Self {
        Self(BlockErrorKind::IncorrectParentBlockHashLength(
            wrong_hash.len(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
enum BlockErrorKind {
    #[error("{0} field not set")]
    FieldNotSet(&'static str),
    #[error(".hash field contained wrong number of bytes; expected 32, got {0}")]
    IncorrectBlockHashLength(usize),
    #[error(".parent_block_hash field contained wrong number of bytes; expected 32, got {0}")]
    IncorrectParentBlockHashLength(usize),
}

/// An Astria execution block on a rollup.
///
/// Contains information about the block number, its hash,
/// its parent block's hash, and timestamp.
///
/// Usually constructed its [`Protobuf`] implementation from a
/// [`raw::Block`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Block {
    /// The block number
    number: u32,
    /// The hash of the block
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serde::string::hex"))]
    hash: [u8; 32],
    /// The hash from the parent block
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::serde::string::hex"))]
    parent_block_hash: [u8; 32],
    /// Timestamp on the block, standardized to google protobuf standard.
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serde::string::display")
    )]
    timestamp: Timestamp,
}

impl Block {
    #[must_use]
    pub fn number(&self) -> u32 {
        self.number
    }

    #[must_use]
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }

    #[must_use]
    pub fn parent_block_hash(&self) -> [u8; 32] {
        self.parent_block_hash
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        // prost_types::Timestamp is a (i64, i32) tuple, so this is
        // effectively just a copy
        self.timestamp.clone()
    }
}

impl Protobuf for Block {
    type Error = BlockError;
    type Raw = raw::Block;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::Block {
            number,
            hash,
            parent_block_hash,
            timestamp,
        } = raw;
        let hash = hash
            .as_slice()
            .try_into()
            .map_err(|_| Self::Error::incorrect_block_hash_length(hash))?;
        let parent_block_hash = parent_block_hash
            .as_slice()
            .try_into()
            .map_err(|_| Self::Error::incorrect_parent_block_hash_length(parent_block_hash))?;

        // Clone'ing timestamp is effectively a copy because timestamp is just a (i32, i64) tuple
        let timestamp = timestamp
            .clone()
            .ok_or(Self::Error::field_not_set(".timestamp"))?;

        Ok(Self {
            number: *number,
            hash,
            parent_block_hash,
            timestamp,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            number,
            hash,
            parent_block_hash,
            timestamp,
        } = self;
        Self::Raw {
            number: *number,
            hash: hash.to_vec(),
            parent_block_hash: parent_block_hash.to_vec(),
            // Clone'ing timestamp is effectively a copy because timestamp is just a (i32, i64)
            // tuple
            timestamp: Some(timestamp.clone()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct CommitmentStateError(CommitmentStateErrorKind);

impl CommitmentStateError {
    fn field_not_set(field: &'static str) -> Self {
        Self(CommitmentStateErrorKind::FieldNotSet(field))
    }

    fn firm(source: BlockError) -> Self {
        Self(CommitmentStateErrorKind::Firm(source))
    }

    fn soft(source: BlockError) -> Self {
        Self(CommitmentStateErrorKind::Soft(source))
    }

    fn firm_exceeds_soft(source: FirmExceedsSoft) -> Self {
        Self(CommitmentStateErrorKind::FirmExceedsSoft(source))
    }
}

#[derive(Debug, thiserror::Error)]
enum CommitmentStateErrorKind {
    #[error("{0} field not set")]
    FieldNotSet(&'static str),
    #[error(".firm field did not contain a valid block")]
    Firm(#[source] BlockError),
    #[error(".soft field did not contain a valid block")]
    Soft(#[source] BlockError),
    #[error(transparent)]
    FirmExceedsSoft(FirmExceedsSoft),
}

#[derive(Debug, thiserror::Error)]
#[error("firm commitment at `{firm} exceeds soft commitment at `{soft}")]
pub struct FirmExceedsSoft {
    firm: u32,
    soft: u32,
}

pub struct NoFirm;
pub struct NoSoft;
pub struct WithFirm(Block);
pub struct WithSoft(Block);

#[derive(Default)]
pub struct CommitmentStateBuilder<TFirm = NoFirm, TSoft = NoSoft> {
    firm: TFirm,
    soft: TSoft,
}

impl CommitmentStateBuilder<NoFirm, NoSoft> {
    fn new() -> Self {
        Self {
            firm: NoFirm,
            soft: NoSoft,
        }
    }
}

impl<TFirm, TSoft> CommitmentStateBuilder<TFirm, TSoft> {
    pub fn firm(self, firm: Block) -> CommitmentStateBuilder<WithFirm, TSoft> {
        let Self {
            soft, ..
        } = self;
        CommitmentStateBuilder {
            firm: WithFirm(firm),
            soft,
        }
    }

    pub fn soft(self, soft: Block) -> CommitmentStateBuilder<TFirm, WithSoft> {
        let Self {
            firm, ..
        } = self;
        CommitmentStateBuilder {
            firm,
            soft: WithSoft(soft),
        }
    }
}

impl CommitmentStateBuilder<WithFirm, WithSoft> {
    /// Finalize the commitment state.
    ///
    /// # Errors
    /// Returns an error if the firm block exceeds the soft one.
    pub fn build(self) -> Result<CommitmentState, FirmExceedsSoft> {
        let Self {
            firm: WithFirm(firm),
            soft: WithSoft(soft),
        } = self;
        if firm.number() > soft.number() {
            return Err(FirmExceedsSoft {
                firm: firm.number(),
                soft: soft.number(),
            });
        }
        Ok(CommitmentState {
            soft,
            firm,
        })
    }
}

/// Information about the [`Block`] at each sequencer commitment level.
///
/// A commitment state is valid if:
/// - Block numbers are such that soft >= firm (upheld by this type).
/// - No blocks ever decrease in block number.
/// - The chain defined by soft is the head of the canonical chain the firm block must belong to.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CommitmentState {
    /// Soft commitment is the rollup block matching latest sequencer block.
    soft: Block,
    /// Firm commitment is achieved when data has been seen in DA.
    firm: Block,
}

impl CommitmentState {
    #[must_use = "a commitment state must be built to be useful"]
    pub fn builder() -> CommitmentStateBuilder {
        CommitmentStateBuilder::new()
    }

    #[must_use]
    pub fn firm(&self) -> &Block {
        &self.firm
    }

    #[must_use]
    pub fn soft(&self) -> &Block {
        &self.soft
    }
}

impl Protobuf for CommitmentState {
    type Error = CommitmentStateError;
    type Raw = raw::CommitmentState;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            soft,
            firm,
        } = raw;
        let soft = 'soft: {
            let Some(soft) = soft else {
                break 'soft Err(Self::Error::field_not_set(".soft"));
            };
            Block::try_from_raw_ref(soft).map_err(Self::Error::soft)
        }?;
        let firm = 'firm: {
            let Some(firm) = firm else {
                break 'firm Err(Self::Error::field_not_set(".firm"));
            };
            Block::try_from_raw_ref(firm).map_err(Self::Error::firm)
        }?;
        Self::builder()
            .firm(firm)
            .soft(soft)
            .build()
            .map_err(Self::Error::firm_exceeds_soft)
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            soft,
            firm,
        } = self;
        let soft = soft.to_raw();
        let firm = firm.to_raw();
        Self::Raw {
            soft: Some(soft),
            firm: Some(firm),
        }
    }
}
