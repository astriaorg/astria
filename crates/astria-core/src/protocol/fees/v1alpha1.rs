use prost::Name as _;

use crate::{
    generated::protocol::fees::v1alpha1 as raw,
    Protobuf,
};

#[derive(Debug, thiserror::Error)]
#[error("failed validating on-wire type `{on_wire}` as domain type")]
pub struct FeeComponentError {
    on_wire: String,
    inner: FeeComponentErrorKind,
}

impl FeeComponentError {
    fn missing_field(on_wire: String, field: &'static str) -> Self {
        Self {
            on_wire,
            inner: FeeComponentErrorKind::MissingField {
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
                        base_fee,
                        computed_cost_multiplier,
                    } = raw;
                    Ok(Self {
                        base_fee: base_fee
                            .ok_or_else(|| Self::Error::missing_field(Self::Raw::full_name(), "base_fee"))?
                            .into(),
                        computed_cost_multiplier: computed_cost_multiplier
                            .ok_or_else(|| Self::Error::missing_field(Self::Raw::full_name(), "computed_cost_multiplier"))?
                            .into(),
                    })
                }

                fn to_raw(&self) -> Self::Raw {
                    let Self {
                        base_fee,
                        computed_cost_multiplier,
                    } = self;
                    Self::Raw {
                        base_fee: Some(base_fee.into()),
                        computed_cost_multiplier: Some(computed_cost_multiplier.into()),
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
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SequenceFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ics20WithdrawalFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InitBridgeAccountFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeLockFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeUnlockFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeSudoChangeFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IbcRelayFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidatorUpdateFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeeAssetChangeFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeeChangeFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IbcRelayerChangeFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SudoAddressChangeFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IbcSudoChangeFeeComponents {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}
