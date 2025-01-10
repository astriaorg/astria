use astria_core::{
    crypto::{
        Signature,
        VerificationKey,
    },
    generated::astria::auction::v1alpha1 as raw,
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::transaction::v1::{
        action::RollupDataSubmission,
        TransactionBody,
    },
    sequencerblock::v1::block,
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use bytes::Bytes;
use prost::Message as _;

use crate::sequencer_key::SequencerKey;

// TODO: this should probably be moved to astria_core::auction?
#[derive(Debug, Clone)]
pub(crate) struct Bid {
    /// The fee that will be charged for this bid.
    fee: u64,
    /// The byte list of transactions fto be included.
    transactions: Vec<Bytes>,
    /// The hash of the rollup block that this bid is based on.
    rollup_parent_block_hash: [u8; 32],
    /// The hash of the sequencer block used to derive the rollup block that this bid is based
    /// on.
    sequencer_parent_block_hash: block::Hash,
}

impl Bid {
    pub(crate) fn try_from_raw(raw: raw::Bid) -> eyre::Result<Self> {
        let raw::Bid {
            fee,
            transactions,
            sequencer_parent_block_hash,
            rollup_parent_block_hash,
        } = raw;
        Ok(Self {
            fee,
            transactions,
            rollup_parent_block_hash: rollup_parent_block_hash
                .as_ref()
                .try_into()
                .wrap_err("invalid prev_rollup_block_hash")?,
            sequencer_parent_block_hash: sequencer_parent_block_hash
                .as_ref()
                .try_into()
                .wrap_err("invalid base_sequencer_block_hash")?,
        })
    }

    fn into_raw(self) -> raw::Bid {
        raw::Bid {
            fee: self.fee,
            transactions: self.transactions,
            sequencer_parent_block_hash: Bytes::copy_from_slice(
                self.sequencer_parent_block_hash.as_bytes(),
            ),
            rollup_parent_block_hash: Bytes::copy_from_slice(&self.rollup_parent_block_hash),
        }
    }

    pub(crate) fn into_transaction_body(
        self,
        nonce: u32,
        rollup_id: RollupId,
        sequencer_key: &SequencerKey,
        fee_asset: asset::Denom,
        chain_id: String,
    ) -> TransactionBody {
        let allocation = Allocation::new(self, sequencer_key);
        let allocation_data = allocation.into_raw().encode_to_vec();

        TransactionBody::builder()
            .actions(vec![
                RollupDataSubmission {
                    rollup_id,
                    data: allocation_data.into(),
                    fee_asset,
                }
                .into(),
            ])
            .nonce(nonce)
            .chain_id(chain_id)
            .try_build()
            .expect("failed to build transaction body")
    }

    pub(crate) fn bid(&self) -> u64 {
        self.fee
    }

    pub(crate) fn rollup_parent_block_hash(&self) -> [u8; 32] {
        self.rollup_parent_block_hash
    }

    pub(crate) fn sequencer_parent_block_hash(&self) -> &block::Hash {
        &self.sequencer_parent_block_hash
    }
}

#[derive(Debug)]
pub(crate) struct Allocation {
    signature: Signature,
    verification_key: VerificationKey,
    payload: Bid,
}

impl Allocation {
    fn new(bid: Bid, sequencer_key: &SequencerKey) -> Self {
        let bid_data = bid.clone().into_raw().encode_to_vec();
        let signature = sequencer_key.signing_key().sign(&bid_data);
        let verification_key = sequencer_key.signing_key().verification_key();
        Self {
            signature,
            verification_key,
            payload: bid,
        }
    }

    fn into_raw(self) -> raw::Allocation {
        let Self {
            signature,
            verification_key,
            payload,
        } = self;

        raw::Allocation {
            signature: Bytes::copy_from_slice(&signature.to_bytes()),
            public_key: Bytes::copy_from_slice(&verification_key.to_bytes()),
            payload: Some(payload.into_raw()),
        }
    }
}
