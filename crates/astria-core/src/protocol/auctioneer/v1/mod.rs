use astria_core_address::Address;

use crate::{
    generated::astria::protocol::auctioneer::v1::EnshrinedAuctioneerEntry as RawEnshrinedAuctioneerEntry,
    primitive::v1::{
        asset,
        AddressError,
    },
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct EnshrinedAuctioneerEntryError(EnshrinedAuctioneerEntryErrorKind);

#[derive(Debug, thiserror::Error)]
enum EnshrinedAuctioneerEntryErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `fee_asset` field was invalid")]
    FeeAsset { source: asset::ParseDenomError },
    #[error("the `asset` field was invalid")]
    Asset { source: asset::ParseDenomError },
}

impl EnshrinedAuctioneerEntryError {
    #[must_use]
    fn field_not_set(field: &'static str) -> Self {
        Self(EnshrinedAuctioneerEntryErrorKind::FieldNotSet(field))
    }

    #[must_use]
    fn address(source: AddressError) -> Self {
        Self(EnshrinedAuctioneerEntryErrorKind::Address {
            source,
        })
    }

    #[must_use]
    fn fee_asset(source: asset::ParseDenomError) -> Self {
        Self(EnshrinedAuctioneerEntryErrorKind::FeeAsset {
            source,
        })
    }

    #[must_use]
    fn asset(source: asset::ParseDenomError) -> Self {
        Self(EnshrinedAuctioneerEntryErrorKind::Asset {
            source,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[must_use]
enum EnshrineAuctioneerErrorKind {
    #[error("the expected field in the raw source type was not set: `{0}`")]
    FieldNotSet(&'static str),
    #[error("the `to` field was invalid")]
    Address { source: AddressError },
    #[error("the `fee_asset` field was invalid")]
    FeeAsset { source: asset::ParseDenomError },
    #[error("the `asset` field was invalid")]
    Asset { source: asset::ParseDenomError },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnshrinedAuctioneerEntry {
    pub auctioneer_address: Address,
    pub staker_address: Address,
    pub staked_amount: u128,
    pub fee_asset: asset::Denom,
    pub asset: asset::Denom,
}

impl Protobuf for EnshrinedAuctioneerEntry {
    type Error = EnshrinedAuctioneerEntryError;
    type Raw = RawEnshrinedAuctioneerEntry;

    #[must_use]
    fn into_raw(self) -> RawEnshrinedAuctioneerEntry {
        RawEnshrinedAuctioneerEntry {
            auctioneer_address: Some(self.auctioneer_address.into_raw()),
            staker_address: Some(self.staker_address.into_raw()),
            fee_asset: self.fee_asset.to_string(),
            asset: self.asset.to_string(),
            staked_amount: Some(self.staked_amount.into()),
        }
    }

    #[must_use]
    fn to_raw(&self) -> RawEnshrinedAuctioneerEntry {
        RawEnshrinedAuctioneerEntry {
            auctioneer_address: Some(self.auctioneer_address.to_raw()),
            staker_address: Some(self.staker_address.to_raw()),
            fee_asset: self.fee_asset.to_string(),
            asset: self.asset.to_string(),
            staked_amount: Some(self.staked_amount.into()),
        }
    }

    /// Convert from a raw, unchecked protobuf
    /// [`crate::generated::astria::protocol::transaction::v1::EnshrineAuctioneer`].
    ///
    /// # Errors
    ///
    /// - if the `auctioneer_address` field is not set
    /// - if the `staker_address` field is invalid
    /// - if the `fee_asset` field is invalid
    /// - if the `asset` field is invalid
    fn try_from_raw(proto: RawEnshrinedAuctioneerEntry) -> Result<Self, Self::Error> {
        let RawEnshrinedAuctioneerEntry {
            auctioneer_address,
            staker_address,
            fee_asset,
            asset,
            staked_amount,
        } = proto;
        let staked_amount =
            staked_amount.ok_or(EnshrinedAuctioneerEntryError::field_not_set("amount"))?;
        let auctioneer_address = auctioneer_address
            .ok_or_else(|| EnshrinedAuctioneerEntryError::field_not_set("auctioneer_address"))
            .and_then(|auctioneer_address| {
                Address::try_from_raw(auctioneer_address)
                    .map_err(EnshrinedAuctioneerEntryError::address)
            })?;
        let staker_address = staker_address
            .ok_or_else(|| EnshrinedAuctioneerEntryError::field_not_set("staker_address"))
            .and_then(|staker_address| {
                Address::try_from_raw(staker_address)
                    .map_err(EnshrinedAuctioneerEntryError::address)
            })?;
        let fee_asset = fee_asset
            .parse()
            .map_err(EnshrinedAuctioneerEntryError::fee_asset)?;
        let asset = asset
            .parse()
            .map_err(EnshrinedAuctioneerEntryError::asset)?;

        Ok(Self {
            auctioneer_address,
            staker_address,
            fee_asset,
            asset,
            staked_amount: staked_amount.into(),
        })
    }

    /// Convert from a reference to a raw, unchecked protobuf
    /// [`crate::generated::astria::protocol::transaction::v1::EnshrineAuctioneer`].
    ///
    /// # Errors
    ///
    /// - if the `auctioneer_address` field is not set
    /// - if the `staker_address` field is invalid
    /// - if the `fee_asset` field is invalid
    /// - if the `asset` field is invalid
    fn try_from_raw_ref(
        proto: &RawEnshrinedAuctioneerEntry,
    ) -> Result<Self, EnshrinedAuctioneerEntryError> {
        Self::try_from_raw(proto.clone())
    }
}
