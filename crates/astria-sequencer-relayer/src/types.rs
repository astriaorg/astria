use astria_proto::sequencer::v1::{
    BlockId as RawBlockId,
    Commit as RawCommit,
    CommitSig as RawCommitSig,
    PartSetHeader as RawPartSetHeader,
};
use eyre::{
    bail,
    eyre,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::block::{
    Commit as TmCommit,
    CommitSig as TmCommitSig,
    Id as TmBlockId,
};
// use tendermint::Block;
use tendermint_proto::types::{
    Block as RawBlock,
    BlockId as TmRawBlockId,
};

use crate::base64_string::Base64String;

/// cosmos-sdk (Tendermint) RPC types.
/// see https://v1.cosmos.network/rpc/v0.41.4

#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Deserialize, Debug)]
pub struct BlockResponse {
    pub block_id: TmRawBlockId,
    pub block: RawBlock,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Serialize)]
pub struct BlockId {
    pub hash: Base64String,
    pub part_set_header: Parts,
}

impl BlockId {
    pub fn from_proto(proto: RawBlockId) -> eyre::Result<Self> {
        Ok(Self {
            hash: Base64String::from_bytes(&proto.hash),
            part_set_header: Parts::from_proto(
                proto
                    .part_set_header
                    .ok_or(eyre!("BlockId from_proto failed: no part_set_header"))?,
            )?,
        })
    }

    pub fn to_proto(&self) -> RawBlockId {
        RawBlockId {
            hash: self.hash.0.clone(),
            part_set_header: Some(self.part_set_header.to_proto()),
        }
    }

    pub fn from_tm_block_id(tm_block_id: &TmBlockId) -> Self {
        Self {
            hash: Base64String::from_bytes(&Into::<Vec<u8>>::into(tm_block_id.hash)),
            part_set_header: Parts {
                total: tm_block_id.part_set_header.total,
                hash: Base64String::from_bytes(&Into::<Vec<u8>>::into(
                    tm_block_id.part_set_header.hash,
                )),
            },
        }
    }
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Serialize)]
pub struct Parts {
    pub total: u32,
    pub hash: Base64String,
}

impl Parts {
    pub fn from_proto(proto: RawPartSetHeader) -> eyre::Result<Self> {
        Ok(Self {
            total: proto.total,
            hash: Base64String::from_bytes(&proto.hash),
        })
    }

    pub fn to_proto(&self) -> RawPartSetHeader {
        RawPartSetHeader {
            total: self.total,
            hash: self.hash.0.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Commit {
    pub height: String,
    pub round: u32,
    pub block_id: BlockId,
    pub signatures: Vec<CommitSig>,
}

impl Commit {
    pub fn from_proto(proto: RawCommit) -> eyre::Result<Self> {
        let signatures = proto
            .signatures
            .into_iter()
            .map(CommitSig::from_proto)
            .collect::<eyre::Result<Vec<_>>>()?;

        Ok(Self {
            height: proto.height,
            round: proto.round,
            block_id: BlockId::from_proto(
                proto
                    .block_id
                    .ok_or(eyre!("Commit from_proto failed: no block_id"))?,
            )?,
            signatures,
        })
    }

    pub fn to_proto(&self) -> RawCommit {
        let signatures = self
            .signatures
            .iter()
            .map(CommitSig::to_proto)
            .collect::<Vec<_>>();

        RawCommit {
            height: self.height.clone(),
            round: self.round,
            block_id: Some(self.block_id.to_proto()),
            signatures,
        }
    }

    pub fn from_tm_commit(tm_commit: &TmCommit) -> eyre::Result<Self> {
        let height = tm_commit.height.value().to_string();
        let round = tm_commit.round.into();
        let block_id = BlockId::from_tm_block_id(&tm_commit.block_id);
        let signatures = tm_commit
            .signatures
            .iter()
            .map(CommitSig::from_tm_commit_sig)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            height,
            round,
            block_id,
            signatures,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct CommitSig {
    pub block_id_flag: String,
    pub validator_address: Base64String,
    pub timestamp: String,
    pub signature: Base64String,
}

impl CommitSig {
    pub fn from_proto(proto: RawCommitSig) -> eyre::Result<Self> {
        Ok(Self {
            block_id_flag: proto.block_id_flag,
            validator_address: Base64String::from_bytes(&proto.validator_address),
            timestamp: proto.timestamp,
            signature: Base64String::from_bytes(&proto.signature),
        })
    }

    pub fn to_proto(&self) -> RawCommitSig {
        RawCommitSig {
            block_id_flag: self.block_id_flag.clone(),
            validator_address: self.validator_address.0.clone(),
            timestamp: self.timestamp.clone(),
            signature: self.signature.0.clone(),
        }
    }

    pub fn from_tm_commit_sig(tm_commit_sig: &TmCommitSig) -> eyre::Result<Self> {
        match tm_commit_sig {
            TmCommitSig::BlockIdFlagAbsent => bail!("BlockIDFlagAbsent is not supported"),
            TmCommitSig::BlockIdFlagCommit {
                validator_address,
                timestamp,
                signature,
            } => Ok(Self {
                block_id_flag: "BLOCK_ID_FLAG_COMMIT".to_string(),
                validator_address: Base64String::from_string(validator_address.to_string())?,
                timestamp: timestamp.to_string(),
                signature: Base64String::from_bytes(
                    &signature
                        .clone()
                        .ok_or(eyre!("CommitSig from_tm_commit_sig failed: no signature"))?
                        .as_bytes(),
                ),
            }),
            TmCommitSig::BlockIdFlagNil {
                validator_address,
                timestamp,
                signature,
            } => Ok(Self {
                block_id_flag: "BLOCK_ID_FLAG_NIL".to_string(),
                validator_address: Base64String::from_string(validator_address.to_string())?,
                timestamp: timestamp.to_string(),
                signature: Base64String::from_bytes(
                    &signature
                        .clone()
                        .ok_or(eyre!("CommitSig from_tm_commit_sig failed: no signature"))?
                        .as_bytes(),
                ),
            }),
        }
    }
}
