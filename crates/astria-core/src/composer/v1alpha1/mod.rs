use bytes::Bytes;

use crate::{
    sequencerblock::v1alpha1::block::{
        RollupData,
        RollupDataError,
    },
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BuilderBundleError(BuilderBundleErrorKind);

impl BuilderBundleError {
    fn invalid_rollup_data(error: RollupDataError) -> Self {
        Self(BuilderBundleErrorKind::InvalidRollupData(error))
    }
}

#[derive(Debug, thiserror::Error)]
enum BuilderBundleErrorKind {
    #[error("{0} invalid rollup data")]
    InvalidRollupData(#[source] RollupDataError),
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::composer::v1alpha1::BuilderBundle")
)]
pub struct BuilderBundle {
    transactions: Vec<RollupData>,
    parent_hash: Bytes,
}

impl BuilderBundle {
    pub fn transactions(&self) -> &[RollupData] {
        self.transactions.as_slice()
    }

    pub fn parent_hash(&self) -> Bytes {
        self.parent_hash.clone()
    }
}

impl From<BuilderBundle> for crate::generated::composer::v1alpha1::BuilderBundle {
    fn from(value: BuilderBundle) -> Self {
        value.to_raw()
    }
}

impl Protobuf for BuilderBundle {
    type Error = BuilderBundleError;
    type Raw = crate::generated::composer::v1alpha1::BuilderBundle;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let crate::generated::composer::v1alpha1::BuilderBundle {
            transactions,
            parent_hash,
        } = raw;

        let mut rollup_data_transactions = vec![];
        for transaction in transactions {
            let rollup_data = RollupData::try_from_raw_ref(transaction)
                .map_err(BuilderBundleError::invalid_rollup_data)?;
            rollup_data_transactions.push(rollup_data);
        }

        Ok(BuilderBundle {
            transactions: rollup_data_transactions,
            parent_hash: Bytes::from(parent_hash.clone()),
        })
    }

    fn to_raw(&self) -> Self::Raw {
        crate::generated::composer::v1alpha1::BuilderBundle {
            transactions: self.transactions.iter().map(Protobuf::to_raw).collect(),
            parent_hash: self.parent_hash.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BuilderBundlePacketError(BuilderBundlePacketErrorKind);

#[derive(Debug, thiserror::Error)]
enum BuilderBundlePacketErrorKind {
    #[error("{0} field not set")]
    FieldNotSet(&'static str),
    #[error("{0} invalid bundle")]
    InvalidBundle(#[source] BuilderBundleError),
}

impl BuilderBundlePacketError {
    fn field_not_set(field: &'static str) -> Self {
        Self(BuilderBundlePacketErrorKind::FieldNotSet(field))
    }

    fn invalid_bundle(error: BuilderBundleError) -> Self {
        Self(BuilderBundlePacketErrorKind::InvalidBundle(error))
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(into = "crate::generated::composer::v1alpha1::BuilderBundlePacket")
)]
pub struct BuilderBundlePacket {
    bundle: BuilderBundle,
    signature: Bytes,
}

impl BuilderBundlePacket {
    pub fn bundle(&self) -> &BuilderBundle {
        &self.bundle
    }

    pub fn signature(&self) -> Bytes {
        self.signature.clone()
    }
}

impl From<BuilderBundlePacket> for crate::generated::composer::v1alpha1::BuilderBundlePacket {
    fn from(value: BuilderBundlePacket) -> Self {
        value.to_raw()
    }
}

impl Protobuf for BuilderBundlePacket {
    type Error = BuilderBundlePacketError;
    type Raw = crate::generated::composer::v1alpha1::BuilderBundlePacket;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let crate::generated::composer::v1alpha1::BuilderBundlePacket {
            bundle,
            signature,
        } = raw;

        let bundle = {
            let Some(bundle) = bundle else {
                return Err(BuilderBundlePacketError::field_not_set("bundle"));
            };

            BuilderBundle::try_from_raw_ref(bundle)
                .map_err(BuilderBundlePacketError::invalid_bundle)?
        };

        Ok(BuilderBundlePacket {
            bundle,
            signature: Bytes::from(signature.clone()),
        })
    }

    fn to_raw(&self) -> Self::Raw {
        crate::generated::composer::v1alpha1::BuilderBundlePacket {
            bundle: Some(self.bundle.to_raw()),
            signature: self.signature.clone(),
        }
    }
}
