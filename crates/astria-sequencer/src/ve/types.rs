use std::collections::HashMap;

use bytes::Bytes;
use prost::{DecodeError, EncodeError, Message};

/// OracleVoteExtension represents the vote extension structure for oracle prices.
/// It contains a map of currency pair IDs to their corresponding price bytes.
/// The ID function is determined by the CurrencyPairIDStrategy used in the VoteExtensionHandler.
#[derive(Clone, PartialEq, ::prost::Message)]
pub(crate) struct OracleVoteExtension {
    #[prost(map = "uint64, bytes", tag = "1")]
    pub prices: HashMap<u64, Bytes>,
}
