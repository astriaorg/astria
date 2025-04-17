use astria_eyre::eyre::bail;
use borsh::{
    io::{
        Read,
        Write,
    },
    BorshDeserialize,
    BorshSerialize,
};
use tendermint::consensus::Params as DomainConsensusParams;

use super::{
    BlockHeight,
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct BlockSizeParams {
    max_bytes: u64,
    max_gas: i64,
    time_iota_ms: i64,
}

#[derive(Debug)]
#[expect(
    clippy::struct_field_names,
    reason = "matches field names of domain type"
)]
struct EvidenceParams {
    max_age_num_blocks: u64,
    max_age_duration: tendermint::evidence::Duration,
    max_bytes: i64,
}

impl BorshSerialize for EvidenceParams {
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.max_age_num_blocks.serialize(writer)?;
        self.max_age_duration.0.as_secs().serialize(writer)?;
        self.max_age_duration.0.subsec_nanos().serialize(writer)?;
        self.max_bytes.serialize(writer)
    }
}

impl BorshDeserialize for EvidenceParams {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let max_age_num_blocks = u64::deserialize_reader(reader)?;
        let max_age_duration_secs = u64::deserialize_reader(reader)?;
        let max_age_duration_subsec_nanos = u32::deserialize_reader(reader)?;
        let max_age_duration = tendermint::evidence::Duration(std::time::Duration::new(
            max_age_duration_secs,
            max_age_duration_subsec_nanos,
        ));
        let max_bytes = i64::deserialize_reader(reader)?;
        Ok(Self {
            max_age_num_blocks,
            max_age_duration,
            max_bytes,
        })
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum PublicKeyAlgorithm {
    Ed25519,
    Secp256k1,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct ValidatorParams {
    pub_key_types: Vec<PublicKeyAlgorithm>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct VersionParams {
    app: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct AbciParams {
    vote_extensions_enable_height: Option<BlockHeight>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::app) struct ConsensusParams {
    block: BlockSizeParams,
    evidence: EvidenceParams,
    validator: ValidatorParams,
    version: Option<VersionParams>,
    abci: AbciParams,
}

impl From<DomainConsensusParams> for ConsensusParams {
    fn from(params: DomainConsensusParams) -> Self {
        let block = BlockSizeParams {
            max_bytes: params.block.max_bytes,
            max_gas: params.block.max_gas,
            time_iota_ms: params.block.time_iota_ms,
        };
        let evidence = EvidenceParams {
            max_age_num_blocks: params.evidence.max_age_num_blocks,
            max_age_duration: params.evidence.max_age_duration,
            max_bytes: params.evidence.max_bytes,
        };
        let validator = ValidatorParams {
            pub_key_types: params
                .validator
                .pub_key_types
                .into_iter()
                .map(|algo| match algo {
                    tendermint::public_key::Algorithm::Ed25519 => PublicKeyAlgorithm::Ed25519,
                    tendermint::public_key::Algorithm::Secp256k1 => PublicKeyAlgorithm::Secp256k1,
                })
                .collect(),
        };
        let version = params.version.map(|version_params| VersionParams {
            app: version_params.app,
        });
        let abci = AbciParams {
            vote_extensions_enable_height: params
                .abci
                .vote_extensions_enable_height
                .map(|height| BlockHeight::from(height.value())),
        };
        Self {
            block,
            evidence,
            validator,
            version,
            abci,
        }
    }
}

impl From<ConsensusParams> for DomainConsensusParams {
    fn from(params: ConsensusParams) -> Self {
        let block = tendermint::block::Size {
            max_bytes: params.block.max_bytes,
            max_gas: params.block.max_gas,
            time_iota_ms: params.block.time_iota_ms,
        };
        let evidence = tendermint::evidence::Params {
            max_age_num_blocks: params.evidence.max_age_num_blocks,
            max_age_duration: params.evidence.max_age_duration,
            max_bytes: params.evidence.max_bytes,
        };
        let validator = tendermint::consensus::params::ValidatorParams {
            pub_key_types: params
                .validator
                .pub_key_types
                .into_iter()
                .map(|algo| match algo {
                    PublicKeyAlgorithm::Ed25519 => tendermint::public_key::Algorithm::Ed25519,
                    PublicKeyAlgorithm::Secp256k1 => tendermint::public_key::Algorithm::Secp256k1,
                })
                .collect(),
        };
        let version =
            params.version.map(
                |version_params| tendermint::consensus::params::VersionParams {
                    app: version_params.app,
                },
            );
        let abci = tendermint::consensus::params::AbciParams {
            vote_extensions_enable_height: params
                .abci
                .vote_extensions_enable_height
                .map(|height| tendermint::block::Height::try_from(u64::from(height)).unwrap()),
        };
        Self {
            block,
            evidence,
            validator,
            version,
            abci,
        }
    }
}

impl From<ConsensusParams> for crate::storage::StoredValue<'_> {
    fn from(params: ConsensusParams) -> Self {
        crate::storage::StoredValue::App(Value(ValueImpl::ConsensusParams(params)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for ConsensusParams {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::App(Value(ValueImpl::ConsensusParams(params))) = value
        else {
            bail!("app stored value type mismatch: expected consensus params, found {value:?}");
        };
        Ok(params)
    }
}
