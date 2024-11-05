use astria_core::{
    generated::bundle::v1alpha1::{
        self as raw,
    },
    primitive::v1::{
        asset,
        RollupId,
    },
    protocol::transaction::v1::{
        action::RollupDataSubmission,
        TransactionBody,
    },
};
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use bytes::Bytes;
pub(crate) use client::BundleStream;
use prost::Message as _;

mod client;

// TODO: this should probably be moved to astria_core::bundle
#[derive(Debug, Clone)]
pub(crate) struct Bundle {
    /// The fee that will be charged for this bundle
    fee: u64,
    /// The byte list of transactions fto be included.
    transactions: Vec<Bytes>,
    /// The hash of the rollup block that this bundle is based on.
    // TODO: rename this to `parent_rollup_block_hash` to match execution api
    prev_rollup_block_hash: [u8; 32],
    /// The hash of the sequencer block used to derive the rollup block that this bundle is based
    /// on.
    base_sequencer_block_hash: [u8; 32],
}

impl Bundle {
    fn try_from_raw(raw: raw::Bundle) -> eyre::Result<Self> {
        let raw::Bundle {
            fee,
            transactions,
            base_sequencer_block_hash,
            prev_rollup_block_hash,
        } = raw;
        Ok(Self {
            fee,
            transactions,
            prev_rollup_block_hash: prev_rollup_block_hash
                .as_ref()
                .try_into()
                .wrap_err("invalid prev_rollup_block_hash")?,
            base_sequencer_block_hash: base_sequencer_block_hash
                .as_ref()
                .try_into()
                .wrap_err("invalid base_sequencer_block_hash")?,
        })
    }

    fn into_raw(self) -> raw::Bundle {
        raw::Bundle {
            fee: self.fee,
            transactions: self.transactions,
            base_sequencer_block_hash: Bytes::copy_from_slice(&self.base_sequencer_block_hash),
            prev_rollup_block_hash: Bytes::copy_from_slice(&self.prev_rollup_block_hash),
        }
    }

    pub(crate) fn into_transaction_body(
        self,
        nonce: u32,
        rollup_id: RollupId,
        fee_asset: asset::Denom,
        chain_id: String,
    ) -> TransactionBody {
        let data = self.into_raw().encode_to_vec();

        // TODO: sign the bundle data and put it in a `SignedBundle` message or something (need to
        // update protos for this)

        TransactionBody::builder()
            .actions(vec![
                RollupDataSubmission {
                    rollup_id,
                    data: data.into(),
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

    pub(crate) fn prev_rollup_block_hash(&self) -> [u8; 32] {
        self.prev_rollup_block_hash
    }

    pub(crate) fn base_sequencer_block_hash(&self) -> [u8; 32] {
        self.base_sequencer_block_hash
    }
}
