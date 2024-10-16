use prost::Name as _;

use crate::{
    generated::protocol::fees::v1alpha1 as raw,
    primitive::v1::asset,
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error("failed validating on-wire type `{on_wire}` as domain type")]
pub struct FeeComponentError {
    on_wire: String,
    source: FeeComponentErrorKind,
}

impl FeeComponentError {
    fn missing_field(on_wire: String, field: &'static str) -> Self {
        Self {
            on_wire,
            source: FeeComponentErrorKind::MissingField {
                field,
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeComponentErrorKind {
    #[error("field `{field}` was not set")]
    MissingField { field: &'static str },
}

macro_rules! impl_protobuf_for_fee_components {
    ( $( $domain_ty:ty => $raw_ty:ty ),* $(,)?) => {
        $(
            impl Protobuf for $domain_ty {
                type Error = FeeComponentError;
                type Raw = $raw_ty;

                fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
                    let Self::Raw {
                        base,
                        multiplier,
                    } = raw;
                    Ok(Self {
                        base: base
                            .ok_or_else(|| Self::Error::missing_field(Self::Raw::full_name(), "base"))?
                            .into(),
                        multiplier: multiplier
                            .ok_or_else(|| Self::Error::missing_field(Self::Raw::full_name(), "multiplier"))?
                            .into(),
                    })
                }

                fn to_raw(&self) -> Self::Raw {
                    let Self {
                        base,
                        multiplier,
                    } = self;
                    Self::Raw {
                        base: Some(base.into()),
                        multiplier: Some(multiplier.into()),
                    }
                }
            }
        )*
    };
}
impl_protobuf_for_fee_components!(
    TransferFeeComponents => raw::TransferFeeComponents,
    SequenceFeeComponents => raw::SequenceFeeComponents,
    Ics20WithdrawalFeeComponents => raw::Ics20WithdrawalFeeComponents ,
    InitBridgeAccountFeeComponents => raw::InitBridgeAccountFeeComponents ,
    BridgeLockFeeComponents => raw::BridgeLockFeeComponents,
    BridgeUnlockFeeComponents => raw::BridgeUnlockFeeComponents,
    BridgeSudoChangeFeeComponents => raw::BridgeSudoChangeFeeComponents ,
    ValidatorUpdateFeeComponents => raw::ValidatorUpdateFeeComponents ,
    IbcRelayerChangeFeeComponents => raw::IbcRelayerChangeFeeComponents ,
    IbcRelayFeeComponents => raw::IbcRelayFeeComponents,
    FeeAssetChangeFeeComponents => raw::FeeAssetChangeFeeComponents ,
    FeeChangeFeeComponents => raw::FeeChangeFeeComponents,
    SudoAddressChangeFeeComponents => raw::SudoAddressChangeFeeComponents ,
    IbcSudoChangeFeeComponents => raw::IbcSudoChangeFeeComponents,
);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransferFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SequenceFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ics20WithdrawalFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InitBridgeAccountFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeLockFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeUnlockFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeSudoChangeFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IbcRelayFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidatorUpdateFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeeAssetChangeFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeeChangeFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IbcRelayerChangeFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SudoAddressChangeFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IbcSudoChangeFeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

#[derive(Debug, Clone)]
pub struct TransactionFeeResponse {
    pub height: u64,
    pub fees: Vec<(asset::Denom, u128)>,
}

impl TransactionFeeResponse {
    #[must_use]
    pub fn into_raw(self) -> raw::TransactionFeeResponse {
        raw::TransactionFeeResponse {
            height: self.height,
            fees: self
                .fees
                .into_iter()
                .map(
                    |(asset, fee)| crate::generated::protocol::fees::v1alpha1::TransactionFee {
                        asset: asset.to_string(),
                        fee: Some(fee.into()),
                    },
                )
                .collect(),
        }
    }

    /// Attempt to convert from a raw protobuf [`raw::TransactionFeeResponse`].
    ///
    /// # Errors
    ///
    /// - if the asset ID could not be converted from bytes
    /// - if the fee was unset
    pub fn try_from_raw(
        proto: raw::TransactionFeeResponse,
    ) -> Result<Self, TransactionFeeResponseError> {
        let raw::TransactionFeeResponse {
            height,
            fees,
        } = proto;
        let fees = fees
            .into_iter()
            .map(
                |crate::generated::protocol::fees::v1alpha1::TransactionFee {
                     asset,
                     fee,
                 }| {
                    let asset = asset.parse().map_err(TransactionFeeResponseError::asset)?;
                    let fee = fee.ok_or(TransactionFeeResponseError::unset_fee())?;
                    Ok((asset, fee.into()))
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Self {
            height,
            fees,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct TransactionFeeResponseError(TransactionFeeResponseErrorKind);

impl TransactionFeeResponseError {
    fn unset_fee() -> Self {
        Self(TransactionFeeResponseErrorKind::UnsetFee)
    }

    fn asset(inner: asset::ParseDenomError) -> Self {
        Self(TransactionFeeResponseErrorKind::Asset(inner))
    }
}

#[derive(Debug, thiserror::Error)]
enum TransactionFeeResponseErrorKind {
    #[error("`fee` field is unset")]
    UnsetFee,
    #[error("failed to parse asset denom in the `assets` field")]
    Asset(#[source] asset::ParseDenomError),
}
