use eyre::WrapErr as _;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use tendermint::{
    account::Id as AccountId,
    block::{
        header::Version as TmVersion, parts::Header as TmPartSetHeader, Header as TmHeader,
        Height as TmHeight, Id as TmBlockId,
    },
    chain::Id as TmChainId,
    hash::{AppHash, Hash as TmHash},
    Time,
};

use crate::base64_string::Base64String;

/// cosmos-sdk (Tendermint) RPC types.
/// see https://v1.cosmos.network/rpc/v0.41.4

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Deserialize, Debug)]
pub struct BlockResponse {
    pub block_id: BlockId,
    pub block: Block,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Serialize)]
pub struct BlockId {
    pub hash: Base64String,
    pub part_set_header: Parts,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Serialize)]
pub struct Parts {
    pub total: u32,
    pub hash: Base64String,
}

#[derive(Deserialize, Debug)]
pub struct Block {
    pub header: Header,
    pub data: Data,
    // TODO: evidence
    pub last_commit: Commit,
}

#[derive(Deserialize, Debug)]
pub struct Commit {
    pub height: String,
    pub round: u64,
    pub block_id: BlockId,
    pub signatures: Vec<CommitSig>,
}

#[derive(Deserialize, Debug)]
pub struct CommitSig {
    pub block_id_flag: String,
    pub validator_address: Base64String,
    pub timestamp: String,
    pub signature: Base64String,
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq, Serialize)]
pub struct Version {
    pub block: String,
    pub app: String,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    pub txs: Vec<Base64String>,
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq, Serialize)]
pub struct Header {
    pub version: Version,
    pub chain_id: String,
    pub height: String,
    pub time: String,
    pub last_block_id: Option<BlockId>,
    pub last_commit_hash: Option<Base64String>,
    pub data_hash: Option<Base64String>,
    pub validators_hash: Base64String,
    pub next_validators_hash: Base64String,
    pub consensus_hash: Base64String,
    pub app_hash: Base64String,
    pub last_results_hash: Option<Base64String>,
    pub evidence_hash: Option<Base64String>,
    pub proposer_address: Base64String,
}

impl Default for Header {
    /// default returns an empty header.
    fn default() -> Self {
        Header {
            version: Version {
                block: "0".to_string(),
                app: "0".to_string(),
            },
            chain_id: "default".to_string(),
            height: "0".to_string(),
            time: "1970-01-01T00:00:00Z".to_string(),
            last_block_id: None,
            last_commit_hash: None,
            data_hash: None,
            validators_hash: Base64String(vec![]),
            next_validators_hash: Base64String(vec![]),
            consensus_hash: Base64String(vec![]),
            app_hash: Base64String(vec![]),
            last_results_hash: None,
            evidence_hash: None,
            proposer_address: Base64String(vec![]),
        }
    }
}

impl Header {
    pub fn hash(&self) -> eyre::Result<TmHash> {
        let tm_header = self
            .to_tendermint_header()
            .wrap_err("failed converting header to tendermint header")?;
        Ok(tm_header.hash())
    }

    /// to_tendermint_header converts a Tendermint RPC header to a tendermint-rs header.
    /// FIXME: This looks exactly like the `TryFrom<RawHeader>` definition that tendermint
    /// uses: https://docs.rs/tendermint/0.30.0/tendermint/block/header/struct.Header.html#impl-TryFrom%3CHeader%3E-for-Header
    /// We should use their impl instead of rolling our own.
    pub fn to_tendermint_header(&self) -> eyre::Result<TmHeader> {
        let last_block_id = self
            .last_block_id
            .as_ref()
            .map(|id| {
                Ok(TmBlockId {
                    hash: TmHash::try_from(id.hash.0.clone())?,
                    part_set_header: TmPartSetHeader::new(
                        id.part_set_header.total,
                        TmHash::try_from(id.part_set_header.hash.0.clone())?,
                    )?,
                })
            })
            .map_or(Ok(None), |r: eyre::Result<TmBlockId>| r.map(Some))?;

        let last_commit_hash = self
            .last_commit_hash
            .as_ref()
            .map(|h| TmHash::try_from(h.0.clone()))
            .map_or(Ok(None), |r| r.map(Some))?;

        let data_hash = self
            .data_hash
            .as_ref()
            .map(|h| TmHash::try_from(h.0.clone()))
            .map_or(Ok(None), |r| r.map(Some))?;

        let last_results_hash = self
            .last_results_hash
            .as_ref()
            .map(|h| TmHash::try_from(h.0.clone()))
            .map_or(Ok(None), |r| r.map(Some))?;

        let evidence_hash = self
            .evidence_hash
            .as_ref()
            .map(|h| TmHash::try_from(h.0.clone()))
            .map_or(Ok(None), |r| r.map(Some))?;

        Ok(TmHeader {
            version: TmVersion {
                block: self.version.block.parse::<u64>()?,
                app: self.version.app.parse::<u64>()?,
            },
            chain_id: TmChainId::try_from(self.chain_id.clone())?,
            height: TmHeight::try_from(self.height.parse::<u64>()?)?,
            time: Time::parse_from_rfc3339(&self.time)?,
            last_block_id,
            last_commit_hash,
            data_hash,
            validators_hash: TmHash::try_from(self.validators_hash.0.clone())?,
            next_validators_hash: TmHash::try_from(self.next_validators_hash.0.clone())?,
            consensus_hash: TmHash::try_from(self.consensus_hash.0.clone())?,
            app_hash: AppHash::try_from(self.app_hash.0.clone())?,
            last_results_hash,
            evidence_hash,
            proposer_address: AccountId::try_from(self.proposer_address.0.clone())?,
        })
    }
}
