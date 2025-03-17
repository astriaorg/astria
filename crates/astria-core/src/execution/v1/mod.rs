use bytes::Bytes;
use pbjson_types::Timestamp;

use crate::{
    generated::astria::execution::v1 as raw,
    primitive::v1::{
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

    fn no_rollup_id() -> Self {
        Self(GenesisInfoErrorKind::NoRollupId)
    }
}

#[derive(Debug, thiserror::Error)]
enum GenesisInfoErrorKind {
    #[error("`rollup_id` field contained an invalid rollup ID")]
    IncorrectRollupIdLength(IncorrectRollupIdLength),
    #[error("`rollup_id` was not set")]
    NoRollupId,
}

/// Genesis Info required from a rollup to start an execution client.
///
/// Contains information about the rollup id, and base heights for both sequencer & celestia.
///
/// Usually constructed its [`Protobuf`] implementation from a
/// [`raw::GenesisInfo`].
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::astria::execution::v1::GenesisInfo")
)]
pub struct GenesisInfo {
    /// The rollup id which is used to identify the rollup txs.
    rollup_id: RollupId,
    /// The Sequencer block height which contains the first block of the rollup.
    sequencer_genesis_block_height: tendermint::block::Height,
    /// The allowed variance in the block height of celestia when looking for sequencer blocks.
    celestia_block_variance: u64,
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
    pub fn celestia_block_variance(&self) -> u64 {
        self.celestia_block_variance
    }
}

impl From<GenesisInfo> for raw::GenesisInfo {
    fn from(value: GenesisInfo) -> Self {
        value.to_raw()
    }
}

impl Protobuf for GenesisInfo {
    type Error = GenesisInfoError;
    type Raw = raw::GenesisInfo;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let raw::GenesisInfo {
            rollup_id,
            sequencer_genesis_block_height,
            celestia_block_variance,
        } = raw;
        let Some(rollup_id) = rollup_id else {
            return Err(Self::Error::no_rollup_id());
        };
        let rollup_id = RollupId::try_from_raw_ref(rollup_id)
            .map_err(Self::Error::incorrect_rollup_id_length)?;

        Ok(Self {
            rollup_id,
            sequencer_genesis_block_height: (*sequencer_genesis_block_height).into(),
            celestia_block_variance: *celestia_block_variance,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            rollup_id,
            sequencer_genesis_block_height,
            celestia_block_variance,
        } = self;

        let sequencer_genesis_block_height: u32 =
            (*sequencer_genesis_block_height).value().try_into().expect(
                "block height overflow, this should not happen since tendermint heights are i64 \
                 under the hood",
            );
        Self::Raw {
            rollup_id: Some(rollup_id.to_raw()),
            sequencer_genesis_block_height,
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
}

#[derive(Debug, thiserror::Error)]
enum BlockErrorKind {
    #[error("{0} field not set")]
    FieldNotSet(&'static str),
}

/// An Astria execution block on a rollup.
///
/// Contains information about the block number, its hash,
/// its parent block's hash, and timestamp.
///
/// Usually constructed its [`Protobuf`] implementation from a
/// [`raw::Block`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::astria::execution::v1::Block")
)]
pub struct Block {
    /// The block number
    number: u32,
    /// The hash of the block
    hash: Bytes,
    /// The hash of the parent block
    parent_block_hash: Bytes,
    /// Timestamp on the block, standardized to google protobuf standard.
    timestamp: Timestamp,
    /// The hash of the sequencer block that this block is derived from.
    sequencer_block_hash: Bytes,
}

impl Block {
    #[must_use]
    pub fn number(&self) -> u32 {
        self.number
    }

    #[must_use]
    pub fn hash(&self) -> &Bytes {
        &self.hash
    }

    #[must_use]
    pub fn parent_block_hash(&self) -> &Bytes {
        &self.parent_block_hash
    }

    #[must_use]
    pub fn timestamp(&self) -> Timestamp {
        // prost_types::Timestamp is a (i64, i32) tuple, so this is
        // effectively just a copy
        self.timestamp.clone()
    }

    #[must_use]
    pub fn sequencer_block_hash(&self) -> &Bytes {
        &self.sequencer_block_hash
    }
}

impl From<Block> for raw::Block {
    fn from(value: Block) -> Self {
        value.to_raw()
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
            sequencer_block_hash,
        } = raw;
        // Cloning timestamp is effectively a copy because timestamp is just a (i32, i64) tuple
        let timestamp = timestamp
            .clone()
            .ok_or(Self::Error::field_not_set(".timestamp"))?;

        Ok(Self {
            number: *number,
            hash: hash.clone(),
            parent_block_hash: parent_block_hash.clone(),
            timestamp,
            sequencer_block_hash: sequencer_block_hash.clone(),
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            number,
            hash,
            parent_block_hash,
            timestamp,
            sequencer_block_hash,
        } = self;
        Self::Raw {
            number: *number,
            hash: hash.clone(),
            parent_block_hash: parent_block_hash.clone(),
            // Cloning timestamp is effectively a copy because timestamp is just a (i32, i64)
            // tuple
            timestamp: Some(timestamp.clone()),
            sequencer_block_hash: sequencer_block_hash.clone(),
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
pub struct NoBaseCelestiaHeight;
pub struct WithFirm(Block);
pub struct WithSoft(Block);
pub struct WithCelestiaBaseHeight(u64);
#[derive(Default)]
pub struct CommitmentStateBuilder<
    TFirm = NoFirm,
    TSoft = NoSoft,
    TBaseCelestiaHeight = NoBaseCelestiaHeight,
> {
    firm: TFirm,
    soft: TSoft,
    base_celestia_height: TBaseCelestiaHeight,
}

impl CommitmentStateBuilder<NoFirm, NoSoft, NoBaseCelestiaHeight> {
    fn new() -> Self {
        Self {
            firm: NoFirm,
            soft: NoSoft,
            base_celestia_height: NoBaseCelestiaHeight,
        }
    }
}

impl<TFirm, TSoft, TCelestiaBaseHeight> CommitmentStateBuilder<TFirm, TSoft, TCelestiaBaseHeight> {
    pub fn firm(self, firm: Block) -> CommitmentStateBuilder<WithFirm, TSoft, TCelestiaBaseHeight> {
        let Self {
            soft,
            base_celestia_height,
            ..
        } = self;
        CommitmentStateBuilder {
            firm: WithFirm(firm),
            soft,
            base_celestia_height,
        }
    }

    pub fn soft(self, soft: Block) -> CommitmentStateBuilder<TFirm, WithSoft, TCelestiaBaseHeight> {
        let Self {
            firm,
            base_celestia_height,
            ..
        } = self;
        CommitmentStateBuilder {
            firm,
            soft: WithSoft(soft),
            base_celestia_height,
        }
    }

    pub fn base_celestia_height(
        self,
        base_celestia_height: u64,
    ) -> CommitmentStateBuilder<TFirm, TSoft, WithCelestiaBaseHeight> {
        let Self {
            firm,
            soft,
            ..
        } = self;
        CommitmentStateBuilder {
            firm,
            soft,
            base_celestia_height: WithCelestiaBaseHeight(base_celestia_height),
        }
    }
}

impl CommitmentStateBuilder<WithFirm, WithSoft, WithCelestiaBaseHeight> {
    /// Finalize the commitment state.
    ///
    /// # Errors
    /// Returns an error if the firm block exceeds the soft one.
    pub fn build(self) -> Result<CommitmentState, FirmExceedsSoft> {
        let Self {
            firm: WithFirm(firm),
            soft: WithSoft(soft),
            base_celestia_height: WithCelestiaBaseHeight(base_celestia_height),
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
            base_celestia_height,
        })
    }
}

/// Information about the [`Block`] at each sequencer commitment level.
///
/// A commitment state is valid if:
/// - Block numbers are such that soft >= firm (upheld by this type).
/// - No blocks ever decrease in block number.
/// - The chain defined by soft is the head of the canonical chain the firm block must belong to.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::astria::execution::v1::CommitmentState")
)]
pub struct CommitmentState {
    /// Soft commitment is the rollup block matching latest sequencer block.
    soft: Block,
    /// Firm commitment is achieved when data has been seen in DA.
    firm: Block,
    /// The base height of celestia from which to search for blocks after this
    /// commitment state.
    base_celestia_height: u64,
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

    pub fn base_celestia_height(&self) -> u64 {
        self.base_celestia_height
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
            soft,
            firm,
            base_celestia_height,
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
            .base_celestia_height(*base_celestia_height)
            .build()
            .map_err(Self::Error::firm_exceeds_soft)
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            soft,
            firm,
            base_celestia_height,
        } = self;
        let soft = soft.to_raw();
        let firm = firm.to_raw();
        let base_celestia_height = *base_celestia_height;
        Self::Raw {
            soft: Some(soft),
            firm: Some(firm),
            base_celestia_height,
        }
    }
}
