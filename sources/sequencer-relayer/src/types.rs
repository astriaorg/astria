use base64::{engine::general_purpose, Engine as _};
use hex;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

/// cosmos-sdk RPC types.
/// see https://v1.cosmos.network/rpc/v0.41.4

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Clone, PartialEq)]
pub struct Base64String(pub Vec<u8>);

impl fmt::Debug for Base64String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(&self.0))
    }
}

impl Serialize for Base64String {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&general_purpose::STANDARD.encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Base64String {
    fn deserialize<D>(deserializer: D) -> Result<Base64String, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(Base64StringVisitor)
    }
}

struct Base64StringVisitor;

impl Base64StringVisitor {
    fn decode_string<E>(self, value: &str) -> Result<Base64String, E>
    where
        E: de::Error,
    {
        general_purpose::STANDARD
            .decode(value)
            .map(Base64String)
            .map_err(|e| {
                E::custom(format!(
                    "failed to decode string {} from base64: {:?}",
                    value, e
                ))
            })
    }
}

impl<'de> Visitor<'de> for Base64StringVisitor {
    type Value = Base64String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a base64-encoded string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.decode_string(value)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.decode_string(&value)
    }
}

#[derive(Deserialize, Debug)]
pub struct BlockResponse {
    pub block_id: BlockId,
    pub block: Block,
}

#[derive(Deserialize, Debug)]
pub struct BlockId {
    pub hash: Base64String,
    // TODO: part_set_header
}

#[derive(Deserialize, Debug)]
pub struct Block {
    pub header: Header,
    pub data: Data,
    // TODO: evidence
}

#[derive(Deserialize, Debug)]
pub struct Header {
    // TODO: version
    pub chain_id: String,
    pub height: String,
    pub time: String,
    // TODO: last_block_id
    pub last_commit_hash: Base64String,
    pub data_hash: Base64String,
    pub validators_hash: Base64String,
    pub next_validators_hash: Base64String,
    pub consensus_hash: Base64String,
    pub app_hash: Base64String,
    pub last_results_hash: Base64String,
    pub evidence_hash: Base64String,
    pub proposer_address: Base64String,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    pub txs: Vec<Base64String>,
}
