use astria_core::protocol::{
    fees::v1::FeeComponents as DomainFeeComponents,
    transaction::v1::action::{
        BridgeLock,
        BridgeSudoChange,
        BridgeTransfer,
        BridgeUnlock,
        FeeAssetChange,
        FeeChange,
        IbcRelayerChange,
        IbcSudoChange,
        Ics20Withdrawal,
        InitBridgeAccount,
        RecoverIbcClient,
        RollupDataSubmission,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
    },
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use penumbra_ibc::IbcRelay;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value(ValueImpl);

#[expect(
    clippy::enum_variant_names,
    reason = "want to make it clear that these are fees and not actions"
)]
#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl {
    TransferFees(FeeComponents),
    RollupDataSubmissionFees(FeeComponents),
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
    BridgeTransferFees(FeeComponents),
    RecoverIbcClientFees(FeeComponents),
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
    DomainFeeComponents<Transfer> => TransferFees,
    DomainFeeComponents<RollupDataSubmission> => RollupDataSubmissionFees,
    DomainFeeComponents<Ics20Withdrawal> => Ics20WithdrawalFees,
    DomainFeeComponents<InitBridgeAccount> => InitBridgeAccountFees,
    DomainFeeComponents<BridgeLock> => BridgeLockFees,
    DomainFeeComponents<BridgeUnlock> => BridgeUnlockFees,
    DomainFeeComponents<BridgeSudoChange> => BridgeSudoChangeFees,
    DomainFeeComponents<IbcRelay> => IbcRelayFees,
    DomainFeeComponents<ValidatorUpdate> => ValidatorUpdateFees,
    DomainFeeComponents<FeeAssetChange> => FeeAssetChangeFees,
    DomainFeeComponents<FeeChange> => FeeChangeFees,
    DomainFeeComponents<IbcRelayerChange> => IbcRelayerChangeFees,
    DomainFeeComponents<IbcSudoChange> => IbcSudoChangeFees,
    DomainFeeComponents<SudoAddressChange> => SudoAddressChangeFees,
    DomainFeeComponents<BridgeTransfer> => BridgeTransferFees,
    DomainFeeComponents<RecoverIbcClient> => RecoverIbcClientFees,
);
