//! The raw JSON payload sent on a `eth_sendBundle` JSONRPC request.
//!
//! The design of the `RawBundle` object follows that of [beaverbuild]
//! and [titanbuilder]. Since beaverbuild is a superset of titanbuilder,
//! it is used as the reference.
//!
//! [beaverbuild]: https://beaverbuild.org/docs.html
//! [titanbuilder]: https://docs.titanbuilder.xyz/api/eth_sendbundle

// NOTE: These types are heavily geared toward use in JSONRPC server.
// It's unclear right now if the orderpool itself is the right place for
// them or if they should be located in closer to that server.

use std::{
    num::NonZeroU64,
    sync::Arc,
};

use alloy_primitives::{
    Bytes,
    B256,
    U64,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_with::{
    serde_as,
    DefaultOnNull,
};
use tracing::error;
use uuid::Uuid;

use super::{
    Cancellation,
    Order,
};
use crate::bundle::Bundle;

/// The raw bundle as received over a `eth_sendBundle` API call.
///
/// The fields mostly follow [rbuilder], which itself is a superset of
/// [beaverbuild] and [titanbuilder].
///
/// Fields that were removed compared to [rbuilder] because they are
/// not currently backed by any functionality.
///
/// + replacement_nonce
/// + first_seen_at
/// + signing_address
/// + refund_percent
/// + refund_recipient
/// + refund_tx_hashes
/// + min_timestamp
/// + max_timestamp
///
/// [beaverbuild]: https://beaverbuild.org/docs.html
/// [titanbuilder]: https://docs.titanbuilder.xyz/api/eth_sendbundle
/// [rbuilder]]: https://github.com/flashbots/rbuilder/blob/942f4013a53a2a0fb99d9e82a020df032ea87b2e/crates/rbuilder/src/primitives/serialize.rs#L91
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawBundle {
    /// txs `Array[String]`, A list of signed transactions to execute in an atomic bundle, list can
    /// be empty for bundle cancellations
    pub(crate) txs: Vec<Bytes>,
    /// blockNumber (Optional) `String`, a hex encoded block number for which this bundle is valid
    /// on. If nil or 0, blockNumber will default to the current pending block
    pub(crate) block_number: Option<U64>,

    /// revertingTxHashes (Optional) `Array[String]`, A list of tx hashes that are allowed to
    /// revert
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub(crate) reverting_tx_hashes: Vec<B256>,
    /// droppingTxHashes (Optional) `Array[String]` A list of tx hashes that are allowed to be
    /// discarded, but may not revert on chain.
    #[serde_as(deserialize_as = "DefaultOnNull")]
    pub(crate) dropping_tx_hashes: Vec<B256>,
    /// a UUID v4 that can be used to replace or cancel this bundle
    /// `replacement_uuid` is a deprecated alias for `uuid` used by beaver builder.
    /// titan builder still uses `replacement_uuid`.
    #[serde(alias = "replacement_uuid", skip_serializing_if = "Option::is_none")]
    pub(crate) uuid: Option<Uuid>,
}

impl RawBundle {
    pub(crate) fn interpret_as_order(self) -> Result<Order, RawBundleToOrderError> {
        if self.txs.is_empty() {
            // TODO: should we also explicitly deal with Uuid::nil and
            // treat that as unset? Leaning toward "no"
            let Some(uuid) = self.uuid else {
                return Err(RawBundleToOrderError::NoTxsNoUuid);
            };
            return Ok(Order::Cancel(Cancellation::new(uuid)));
        }
        let txs = self
            .txs
            .iter()
            .enumerate()
            .map(|(idx, bytes)| {
                crate::bundle::Transaction::decode_2718(bytes).map_err(|source| {
                    RawBundleToOrderError::DecodeTransaction {
                        idx,
                        source,
                    }
                })
            })
            .collect::<Result<_, _>>()?;

        let mut bundle = Bundle::new();
        bundle
            .set_txs(txs)
            .set_raw_txs(self.txs)
            .set_block(
                self.block_number
                    .and_then(|block| NonZeroU64::new(block.to())),
            )
            .collect_reverting_tx_hashes(self.reverting_tx_hashes)
            .collect_dropping_tx_hashes(self.dropping_tx_hashes)
            // XXX: Uuid::nil corresponds to Uuid::default, but we choose to be explicit here.
            .set_uuid(self.uuid.unwrap_or_else(|| Uuid::nil()));
        Ok(Order::New(Arc::new(bundle)))
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum RawBundleToOrderError {
    #[error("failed to decode item `.txs[{idx}]` as an EIP 2718 typed transaction envelope")]
    DecodeTransaction {
        idx: usize,
        source: crate::bundle::TransactionDecode2718Error,
    },
    #[error(
        "fields .txs and .uuid were empty or not set, which need to be set to dermine if the \
         bundle is an order, replacement order, or cancellation"
    )]
    NoTxsNoUuid,
}
