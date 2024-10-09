use astria_core::protocol::fees::v1alpha1::{
    BridgeLockFeeComponents,
    BridgeSudoChangeFeeComponents,
    BridgeUnlockFeeComponents,
    FeeAssetChangeFeeComponents,
    FeeChangeFeeComponents,
    IbcRelayFeeComponents,
    IbcRelayerChangeFeeComponents,
    IbcSudoChangeFeeComponents,
    Ics20WithdrawalFeeComponents,
    InitBridgeAccountFeeComponents,
    SequenceFeeComponents,
    SudoAddressChangeFeeComponents,
    TransferFeeComponents,
    ValidatorUpdateFeeComponents,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value(ValueImpl);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[expect(
    clippy::enum_variant_names,
    reason = "want to make it clear that these are fees and not actions"
)]
enum ValueImpl {
    TransferFees(TransferFeeComponentsStorage),
    SequenceFees(SequenceFeeComponentsStorage),
    Ics20WithdrawalFees(Ics20WithdrawalFeeComponentsStorage),
    InitBridgeAccountFees(InitBridgeAccountFeeComponentsStorage),
    BridgeLockFees(BridgeLockFeeComponentsStorage),
    BridgeUnlockFees(BridgeUnlockFeeComponentsStorage),
    BridgeSudoChangeFees(BridgeSudoChangeFeeComponentsStorage),
    IbcRelayFees(IbcRelayFeeComponentsStorage),
    ValidatorUpdateFees(ValidatorUpdateFeeComponentsStorage),
    FeeAssetChangeFees(FeeAssetChangeFeeComponentsStorage),
    FeeChangeFees(FeeChangeFeeComponentsStorage),
    IbcRelayerChangeFees(IbcRelayerChangeFeeComponentsStorage),
    IbcSudoChangeFees(IbcSudoChangeFeeComponentsStorage),
    SudoAddressChangeFees(SudoAddressChangeFeeComponentsStorage),
}

macro_rules! impl_from_for_fee_component{
    ( $( $domain_ty:ty => $storage_ty:ty),* $(,)? ) => {
        $(
            impl From<$domain_ty> for $storage_ty {
                fn from(val: $domain_ty) -> Self {
                    Self{base_fee: val.base_fee, computed_cost_multiplier: val.computed_cost_multiplier}
                }
            }
            impl From<$storage_ty> for $domain_ty {
                fn from(val: $storage_ty) -> Self {
                    Self{base_fee: val.base_fee, computed_cost_multiplier: val.computed_cost_multiplier}
                }
            }
        )*
    }
}

macro_rules! impl_from_for_fee_storage {
    ( $( $storage_ty:ty => $value_impl:ident),* $(,)? ) => {
        $(
            impl<'a> From<$storage_ty> for crate::storage::StoredValue<'a> {
                fn from(fees: $storage_ty) -> Self {
                    crate::storage::StoredValue::Fees(Value(ValueImpl::$value_impl(fees)))
                }
            }
            impl<'a> TryFrom<crate::storage::StoredValue<'a>> for $storage_ty {
                type Error = astria_eyre::eyre::Error;

            fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
                let crate::storage::StoredValue::Fees(Value(ValueImpl::$value_impl(fees))) = value else {
                    let value_impl_ty = concat!("ValueImpl::", stringify!($value_impl));
                    bail!(
                        "fees stored value type mismatch: expected {value_impl_ty}, found {value:?}"
                    );
                };
                Ok(fees)
            }
            }
        )*
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct TransferFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct SequenceFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct Ics20WithdrawalFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct InitBridgeAccountFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct BridgeLockFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct BridgeUnlockFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct BridgeSudoChangeFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct IbcRelayFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct ValidatorUpdateFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct FeeAssetChangeFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct FeeChangeFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct IbcRelayerChangeFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct IbcSudoChangeFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct SudoAddressChangeFeeComponentsStorage {
    pub base_fee: u128,
    pub computed_cost_multiplier: u128,
}

impl_from_for_fee_component!(
    TransferFeeComponents => TransferFeeComponentsStorage,
    SequenceFeeComponents => SequenceFeeComponentsStorage,
    Ics20WithdrawalFeeComponents => Ics20WithdrawalFeeComponentsStorage,
    InitBridgeAccountFeeComponents => InitBridgeAccountFeeComponentsStorage,
    BridgeLockFeeComponents => BridgeLockFeeComponentsStorage,
    BridgeUnlockFeeComponents => BridgeUnlockFeeComponentsStorage,
    BridgeSudoChangeFeeComponents => BridgeSudoChangeFeeComponentsStorage,
    IbcRelayFeeComponents => IbcRelayFeeComponentsStorage,
    ValidatorUpdateFeeComponents => ValidatorUpdateFeeComponentsStorage,
    FeeAssetChangeFeeComponents => FeeAssetChangeFeeComponentsStorage,
    FeeChangeFeeComponents => FeeChangeFeeComponentsStorage,
    IbcRelayerChangeFeeComponents => IbcRelayerChangeFeeComponentsStorage,
    IbcSudoChangeFeeComponents => IbcSudoChangeFeeComponentsStorage,
    SudoAddressChangeFeeComponents => SudoAddressChangeFeeComponentsStorage,
);

impl_from_for_fee_storage!(
    TransferFeeComponentsStorage => TransferFees,
    SequenceFeeComponentsStorage => SequenceFees,
    Ics20WithdrawalFeeComponentsStorage => Ics20WithdrawalFees,
    InitBridgeAccountFeeComponentsStorage => InitBridgeAccountFees,
    BridgeLockFeeComponentsStorage => BridgeLockFees,
    BridgeUnlockFeeComponentsStorage => BridgeUnlockFees,
    BridgeSudoChangeFeeComponentsStorage => BridgeSudoChangeFees,
    IbcRelayFeeComponentsStorage => IbcRelayFees,
    ValidatorUpdateFeeComponentsStorage => ValidatorUpdateFees,
    FeeAssetChangeFeeComponentsStorage => FeeAssetChangeFees,
    FeeChangeFeeComponentsStorage => FeeChangeFees,
    IbcRelayerChangeFeeComponentsStorage => IbcRelayerChangeFees,
    IbcSudoChangeFeeComponentsStorage => IbcSudoChangeFees,
    SudoAddressChangeFeeComponentsStorage => SudoAddressChangeFees,
);
