use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use crate::{
    generated::protocol::fees::v1alpha1 as raw,
    Protobuf,
};

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct FeeComponentsInner {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FeeComponentsError(FeeComponentsErrorKind);

impl FeeComponentsError {
    pub(crate) fn missing_field(field: &'static str) -> Self {
        Self(FeeComponentsErrorKind::MissingField {
            field,
        })
    }

    pub(crate) fn missing_value_to_change() -> Self {
        Self(FeeComponentsErrorKind::MissingFeeComponent)
    }
}

#[derive(Debug, thiserror::Error)]
enum FeeComponentsErrorKind {
    #[error("the field `{field}` of the fee component was missing")]
    MissingField { field: &'static str },
    #[error("the fee component was missing")]
    MissingFeeComponent,
}

macro_rules! impl_protobuf_for_fee_components {
    ($domain_ty:ty, $raw_ty:ty $(,)?) => {
        impl Protobuf for $domain_ty {
            type Error = FeeComponentsError;
            type Raw = $raw_ty;

            fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
                let Self::Raw {
                    base_fee,
                    computed_cost_multiplier,
                } = raw;
                Ok(Self(FeeComponentsInner {
                    base_fee: base_fee
                        .ok_or_else(|| Self::Error::missing_field("base fee"))?
                        .into(),
                    computed_cost_multiplier: computed_cost_multiplier
                        .ok_or_else(|| Self::Error::missing_field("computed cost multiplier"))?
                        .into(),
                }))
            }

            fn to_raw(&self) -> Self::Raw {
                let FeeComponentsInner {
                    base_fee,
                    computed_cost_multiplier,
                } = self.0;
                Self::Raw {
                    base_fee: Some(base_fee.into()),
                    computed_cost_multiplier: Some(computed_cost_multiplier.into()),
                }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct TransferFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(TransferFeeComponents, raw::TransferFeeComponents);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct SequenceFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(SequenceFeeComponents, raw::SequenceFeeComponents);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct Ics20WithdrawalFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    Ics20WithdrawalFeeComponents,
    raw::Ics20WithdrawalFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct InitBridgeAccountFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    InitBridgeAccountFeeComponents,
    raw::InitBridgeAccountFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct BridgeLockFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(BridgeLockFeeComponents, raw::BridgeLockFeeComponents);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct BridgeUnlockFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(BridgeUnlockFeeComponents, raw::BridgeUnlockFeeComponents);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct BridgeSudoChangeFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    BridgeSudoChangeFeeComponents,
    raw::BridgeSudoChangeFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct IbcRelayFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(IbcRelayFeeComponents, raw::IbcRelayFeeComponents);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct ValidatorUpdateFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    ValidatorUpdateFeeComponents,
    raw::ValidatorUpdateFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct FeeAssetChangeFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    FeeAssetChangeFeeComponents,
    raw::FeeAssetChangeFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct FeeChangeFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(FeeChangeFeeComponents, raw::FeeChangeFeeComponents);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct IbcRelayerChangeFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    IbcRelayerChangeFeeComponents,
    raw::IbcRelayerChangeFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct SudoAddressChangeFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(
    SudoAddressChangeFeeComponents,
    raw::SudoAddressChangeFeeComponents
);

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct IbcSudoChangeFeeComponents(pub FeeComponentsInner);
impl_protobuf_for_fee_components!(IbcSudoChangeFeeComponents, raw::IbcSudoChangeFeeComponents);
