use astria_core::protocol::fees::v1::{
    BridgeLockFeeComponents,
    BridgeSudoChangeFeeComponents,
    BridgeUnlockFeeComponents,
    FeeAssetChangeFeeComponents,
    FeeChangeFeeComponents,
    FeeComponents as DomainFeeComponents,
    IbcRelayFeeComponents,
    IbcRelayerChangeFeeComponents,
    IbcSudoChangeFeeComponents,
    Ics20WithdrawalFeeComponents,
    InitBridgeAccountFeeComponents,
    RollupDataSubmissionFeeComponents,
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
    TransferFees(FeeComponents),
    SequenceFees(FeeComponents),
    Ics20WithdrawalFees(FeeComponents),
    InitBridgeAccountFees(FeeComponents),
    BridgeLockFees(FeeComponents),
    BridgeUnlockFees(FeeComponents),
    BridgeSudoChangeFees(FeeComponents),
    IbcRelayFees(FeeComponents),
    ValidatorUpdateFees(FeeComponents),
    FeeAssetChangeFees(FeeComponents),
    FeeChangeFees(FeeComponents),
    IbcRelayerChangeFees(FeeComponents),
    IbcSudoChangeFees(FeeComponents),
    SudoAddressChangeFees(FeeComponents),
}

macro_rules! impl_from_for_fee_storage {
    ( $( $domain_ty:ty => $value_impl:ident),* $(,)? ) => {
        $(
            impl<'a> From<$domain_ty> for crate::storage::StoredValue<'a> {
                fn from(fees: $domain_ty) -> Self {
                    crate::storage::StoredValue::Fees(Value(ValueImpl::$value_impl(fees.into())))
                }
            }
            impl<'a> TryFrom<crate::storage::StoredValue<'a>> for $domain_ty {
                type Error = astria_eyre::eyre::Error;

                fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
                    let crate::storage::StoredValue::Fees(Value(ValueImpl::$value_impl(fees))) = value else {
                        let value_impl_ty = concat!("ValueImpl::", stringify!($value_impl));
                        bail!(
                            "fees stored value type mismatch: expected {value_impl_ty}, found {value:?}"
                        );
                    };
                    Ok(fees.into())
                }
            }
        )*
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct FeeComponents {
    pub base: u128,
    pub multiplier: u128,
}

impl<T> From<DomainFeeComponents<T>> for FeeComponents {
    fn from(fees: DomainFeeComponents<T>) -> Self {
        Self {
            base: fees.base(),
            multiplier: fees.multiplier(),
        }
    }
}

impl<T: ?Sized> From<FeeComponents> for DomainFeeComponents<T> {
    fn from(fees: FeeComponents) -> Self {
        Self::new(fees.base, fees.multiplier)
    }
}

impl_from_for_fee_storage!(
    TransferFeeComponents => TransferFees,
    RollupDataSubmissionFeeComponents => SequenceFees,
    Ics20WithdrawalFeeComponents => Ics20WithdrawalFees,
    InitBridgeAccountFeeComponents => InitBridgeAccountFees,
    BridgeLockFeeComponents => BridgeLockFees,
    BridgeUnlockFeeComponents => BridgeUnlockFees,
    BridgeSudoChangeFeeComponents => BridgeSudoChangeFees,
    IbcRelayFeeComponents => IbcRelayFees,
    ValidatorUpdateFeeComponents => ValidatorUpdateFees,
    FeeAssetChangeFeeComponents => FeeAssetChangeFees,
    FeeChangeFeeComponents => FeeChangeFees,
    IbcRelayerChangeFeeComponents => IbcRelayerChangeFees,
    IbcSudoChangeFeeComponents => IbcSudoChangeFees,
    SudoAddressChangeFeeComponents => SudoAddressChangeFees,
);
