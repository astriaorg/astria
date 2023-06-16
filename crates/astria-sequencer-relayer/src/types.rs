use astria_proto::sequencer::v1::{
    Block as RawBlock,
    BlockId as RawBlockId,
    Commit as RawCommit,
    CommitSig as RawCommitSig,
    Consensus as RawVersion,
    Data as RawData,
    Header as RawHeader,
    PartSetHeader as RawPartSetHeader,
};
use eyre::{
    bail,
    eyre,
    Context,
};
use serde::{
    Deserialize,
    Serialize,
};
use tendermint::{
    self,
    account::Id as AccountId,
    block::{
        header::Version as TmVersion,
        parts::Header as TmPartSetHeader,
        Commit as TmCommit,
        CommitSig as TmCommitSig,
        Header as TmHeader,
        Height as TmHeight,
        Id as TmBlockId,
    },
    chain::Id as TmChainId,
    AppHash,
    Hash as TmHash,
    Time,
};

use crate::base64_string::Base64String;

/// cosmos-sdk (Tendermint) RPC types.
/// see https://v1.cosmos.network/rpc/v0.41.4
#[derive(Serialize, Debug)]
pub struct EmptyRequest {}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Serialize)]
pub struct BlockResponse {
    pub block_id: BlockId,
    pub block: Block,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Serialize)]
pub struct Block {
    pub header: Header,
    pub data: Data,
    pub last_commit: Commit,
}

impl Block {
    pub fn from_proto(proto: RawBlock) -> eyre::Result<Self> {
        Ok(Self {
            header: Header::from_proto(
                proto
                    .header
                    .ok_or(eyre!("Block from_proto failed: no header"))?,
            )?,
            data: Data::from_proto(
                proto
                    .data
                    .ok_or(eyre!("Block from_proto failed: no data"))?,
            ),
            last_commit: Commit::from_proto(
                proto
                    .last_commit
                    .ok_or(eyre!("Block from_proto failed: no last_commit"))?,
            )?,
        })
    }

    pub fn to_proto(&self) -> eyre::Result<RawBlock> {
        Ok(RawBlock {
            header: Some(Header::to_proto(&self.header)?),
            data: Some(Data::to_proto(&self.data)),
            last_commit: Some(Commit::to_proto(&self.last_commit)),
        })
    }
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
            ),
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
            hash: Base64String::from_bytes(tm_block_id.hash.as_bytes()),
            part_set_header: Parts {
                total: tm_block_id.part_set_header.total,
                hash: Base64String::from_bytes(tm_block_id.part_set_header.hash.as_bytes()),
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
    pub fn from_proto(proto: RawPartSetHeader) -> Self {
        Self {
            total: proto.total,
            hash: Base64String::from_bytes(&proto.hash),
        }
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
                    signature
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
                    signature
                        .clone()
                        .ok_or(eyre!("CommitSig from_tm_commit_sig failed: no signature"))?
                        .as_bytes(),
                ),
            }),
        }
    }
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq, Serialize)]
pub struct Data {
    pub txs: Vec<Base64String>,
}

impl Data {
    pub fn from_proto(proto: RawData) -> Self {
        Self {
            txs: proto
                .txs
                .iter()
                .map(|tx| Base64String::from_bytes(tx))
                .collect(),
        }
    }

    pub fn to_proto(&self) -> RawData {
        RawData {
            txs: self.txs.iter().map(|tx| tx.0.clone()).collect(),
        }
    }
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq, Serialize)]
pub struct Version {
    pub block: u64,
    pub app: u64,
}

impl Version {
    pub fn from_proto(proto: RawVersion) -> Self {
        Self {
            block: proto.block,
            app: proto.app,
        }
    }

    pub fn to_proto(&self) -> RawVersion {
        RawVersion {
            block: self.block,
            app: self.app,
        }
    }
}

#[derive(Clone, Deserialize, Debug, Eq, PartialEq, Serialize)]
pub struct Header {
    pub version: Version,
    pub chain_id: String,
    pub height: TmHeight,
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
                block: 0,
                app: 0,
            },
            chain_id: "default".to_string(),
            height: TmHeight::from(0u8),
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
    pub(crate) fn to_tendermint_header(&self) -> eyre::Result<TmHeader> {
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
                block: self.version.block,
                app: self.version.app,
            },
            chain_id: TmChainId::try_from(self.chain_id.clone())?,
            height: self.height,
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

    pub fn from_proto(proto: RawHeader) -> eyre::Result<Self> {
        let version = Version::from_proto(
            proto
                .version
                .ok_or(eyre!("Header from_proto failed: no version"))?,
        );
        let last_block_id =
            Some(BlockId::from_proto(proto.last_block_id.ok_or(eyre!(
                "Header from_proto failed: no last_block_id"
            ))?)?);
        let last_commit_hash = Some(Base64String::from_bytes(&proto.last_commit_hash));
        let data_hash = Some(Base64String::from_bytes(&proto.data_hash));
        let validators_hash = Base64String::from_bytes(&proto.validators_hash);
        let next_validators_hash = Base64String::from_bytes(&proto.next_validators_hash);
        let consensus_hash = Base64String::from_bytes(&proto.consensus_hash);
        let app_hash = Base64String::from_bytes(&proto.app_hash);
        let last_results_hash = Some(Base64String::from_bytes(&proto.last_results_hash));
        let evidence_hash = Some(Base64String::from_bytes(&proto.evidence_hash));
        let proposer_address = Base64String::from_bytes(&proto.proposer_address);

        Ok(Self {
            version,
            chain_id: proto.chain_id,
            height: TmHeight::try_from(proto.height)?,
            time: proto.time,
            last_block_id,
            last_commit_hash,
            data_hash,
            validators_hash,
            next_validators_hash,
            consensus_hash,
            app_hash,
            last_results_hash,
            evidence_hash,
            proposer_address,
        })
    }

    pub fn to_proto(&self) -> eyre::Result<RawHeader> {
        Ok(RawHeader {
            version: Some(Version::to_proto(&self.version)),
            chain_id: self.chain_id.clone(),
            height: self.height.into(),
            time: self.time.clone(),
            last_block_id: self.last_block_id.clone().map(|h| BlockId::to_proto(&h)),
            last_commit_hash: self
                .last_commit_hash
                .clone()
                .ok_or(eyre!("Header to_proto failed: no last_commit_hash"))?
                .0,
            data_hash: self
                .data_hash
                .clone()
                .ok_or(eyre!("Header to_proto failed: no data_hash"))?
                .0,
            validators_hash: self.validators_hash.0.clone(),
            next_validators_hash: self.next_validators_hash.0.clone(),
            consensus_hash: self.consensus_hash.0.clone(),
            app_hash: self.app_hash.0.clone(),
            last_results_hash: self
                .last_results_hash
                .clone()
                .ok_or(eyre!("Header to_proto failed: no last_results_hash"))?
                .0,
            evidence_hash: self
                .evidence_hash
                .clone()
                .ok_or(eyre!("Header to_proto failed:: no evidence_hash"))?
                .0,
            proposer_address: self.proposer_address.0.clone(),
        })
    }
}
