use crate::{
    generated::protocol::enshrinedbuilder::v1alpha1 as raw,
    primitive::v1::{
        asset,
        Address,
        AddressError,
    },
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct StakedBuilderEntryError(StakedBuilderEntryErrorKind);

#[derive(Debug, thiserror::Error)]
enum StakedBuilderEntryErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `asset` field was invalid")]
    InvalidAsset(#[source] asset::ParseDenomError),
}

impl StakedBuilderEntryError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(StakedBuilderEntryErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(StakedBuilderEntryErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(StakedBuilderEntryErrorKind::InvalidAsset(err))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StakedBuilderEntry {
    pub creator_address: Address,
    pub builder_address: Address,
    pub staked_amount: u128,
    pub asset: asset::Denom,
}

impl Protobuf for StakedBuilderEntry {
    type Error = StakedBuilderEntryError;
    type Raw = raw::StakedBuilderEntry;

    #[must_use]
    fn into_raw(self) -> raw::StakedBuilderEntry {
        raw::StakedBuilderEntry {
            creator_address: Some(self.creator_address.to_raw()),
            builder_address: Some(self.builder_address.to_raw()),
            staked_amount: Some(self.staked_amount.into()),
            asset: self.asset.to_string(),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::StakedBuilderEntry {
        raw::StakedBuilderEntry {
            creator_address: Some(self.creator_address.to_raw()),
            builder_address: Some(self.builder_address.to_raw()),
            staked_amount: Some(self.staked_amount.into()),
            asset: self.asset.to_string(),
        }
    }

    fn try_from_raw(proto: raw::StakedBuilderEntry) -> Result<Self, Self::Error> {
        let raw::StakedBuilderEntry {
            creator_address,
            builder_address,
            staked_amount,
            asset,
        } = proto;

        let creator_address = creator_address
            .ok_or_else(|| StakedBuilderEntryError::field_not_set("creator_address"))
            .and_then(|creator_address| {
                Address::try_from_raw(&creator_address).map_err(StakedBuilderEntryError::address)
            })?;

        let builder_address = builder_address
            .ok_or_else(|| StakedBuilderEntryError::field_not_set("builder_address"))
            .and_then(|builder_address| {
                Address::try_from_raw(&builder_address).map_err(StakedBuilderEntryError::address)
            })?;

        let staked_amount =
            staked_amount.ok_or_else(|| StakedBuilderEntryError::field_not_set("staked_amount"))?;
        let asset = asset
            .parse()
            .map_err(StakedBuilderEntryError::invalid_asset)?;

        Ok(Self {
            creator_address,
            builder_address,
            staked_amount: staked_amount.into(),
            asset,
        })
    }

    fn try_from_raw_ref(proto: &raw::StakedBuilderEntry) -> Result<Self, StakedBuilderEntryError> {
        Self::try_from_raw(proto.clone())
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct UnstakedBuilderEntryError(UnstakedBuilderEntryErrorKind);

impl UnstakedBuilderEntryError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(UnstakedBuilderEntryErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(UnstakedBuilderEntryErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn invalid_asset(err: asset::ParseDenomError) -> Self {
        Self(UnstakedBuilderEntryErrorKind::InvalidAsset(err))
    }
}

#[derive(Debug, thiserror::Error)]
enum UnstakedBuilderEntryErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `asset` field was invalid")]
    InvalidAsset(#[source] asset::ParseDenomError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnstakedBuilderEntry {
    pub creator_address: Address,
    pub builder_address: Address,
    pub time: pbjson_types::Timestamp,
    pub asset: asset::Denom,
}

impl Protobuf for UnstakedBuilderEntry {
    type Error = UnstakedBuilderEntryError;
    type Raw = raw::UnstakedBuilderEntry;

    #[must_use]
    fn into_raw(self) -> raw::UnstakedBuilderEntry {
        raw::UnstakedBuilderEntry {
            creator_address: Some(self.creator_address.to_raw()),
            builder_address: Some(self.builder_address.to_raw()),
            time: Some(self.time.into()),
            asset: self.asset.to_string(),
        }
    }

    #[must_use]
    fn to_raw(&self) -> raw::UnstakedBuilderEntry {
        raw::UnstakedBuilderEntry {
            creator_address: Some(self.creator_address.to_raw()),
            builder_address: Some(self.builder_address.to_raw()),
            time: Some(self.time.clone().into()),
            asset: self.asset.to_string(),
        }
    }

    fn try_from_raw(proto: raw::UnstakedBuilderEntry) -> Result<Self, Self::Error> {
        let raw::UnstakedBuilderEntry {
            creator_address,
            builder_address,
            time,
            asset,
        } = proto;

        let creator_address = creator_address
            .ok_or_else(|| UnstakedBuilderEntryError::field_not_set("creator_address"))
            .and_then(|creator_address| {
                Address::try_from_raw(&creator_address).map_err(UnstakedBuilderEntryError::address)
            })?;

        let builder_address = builder_address
            .ok_or_else(|| UnstakedBuilderEntryError::field_not_set("builder_address"))
            .and_then(|builder_address| {
                Address::try_from_raw(&builder_address).map_err(UnstakedBuilderEntryError::address)
            })?;

        let time = time.ok_or_else(|| UnstakedBuilderEntryError::field_not_set("time"))?;
        let asset = asset
            .parse()
            .map_err(UnstakedBuilderEntryError::invalid_asset)?;

        Ok(Self {
            creator_address,
            builder_address,
            time,
            asset,
        })
    }

    fn try_from_raw_ref(
        proto: &raw::UnstakedBuilderEntry,
    ) -> Result<Self, UnstakedBuilderEntryError> {
        Self::try_from_raw(proto.clone())
    }
}
