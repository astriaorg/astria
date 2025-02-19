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

#[derive(Debug, BorshSerialize, BorshDeserialize)]
#[expect(
    clippy::enum_variant_names,
    reason = "want to make it clear that these are fees and not actions"
)]
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
    DomainFeeComponents<BridgeTransfer> => BridgeTransferFees,
    DomainFeeComponents<BridgeSudoChange> => BridgeSudoChangeFees,
    DomainFeeComponents<IbcRelay> => IbcRelayFees,
    DomainFeeComponents<ValidatorUpdate> => ValidatorUpdateFees,
    DomainFeeComponents<FeeAssetChange> => FeeAssetChangeFees,
    DomainFeeComponents<FeeChange> => FeeChangeFees,
    DomainFeeComponents<IbcRelayerChange> => IbcRelayerChangeFees,
    DomainFeeComponents<IbcSudoChange> => IbcSudoChangeFees,
    DomainFeeComponents<SudoAddressChange> => SudoAddressChangeFees,
);

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use insta::assert_snapshot;

    use super::*;

    macro_rules! value_impl_discriminant {
        ($value_impl:ident) => {{
            format!(
                "{:?}",
                discriminant(&ValueImpl::$value_impl(FeeComponents {
                    base: 1,
                    multiplier: 2,
                }))
            )
        }};
    }

    #[test]
    fn value_impl_transfer_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_transfer_fees_discriminant",
            value_impl_discriminant!(TransferFees),
        );
    }

    #[test]
    fn value_impl_rollup_data_submission_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_rollup_data_submission_fees_discriminant",
            value_impl_discriminant!(RollupDataSubmissionFees),
        );
    }

    #[test]
    fn value_impl_ics20_withdrawal_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_ics20_withdrawal_fees_discriminant",
            value_impl_discriminant!(Ics20WithdrawalFees),
        );
    }

    #[test]
    fn value_impl_init_bridge_account_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_init_bridge_account_fees_discriminant",
            value_impl_discriminant!(InitBridgeAccountFees),
        );
    }

    #[test]
    fn value_impl_bridge_lock_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_bridge_lock_fees_discriminant",
            value_impl_discriminant!(BridgeLockFees),
        );
    }

    #[test]
    fn value_impl_bridge_unlock_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_bridge_unlock_fees_discriminant",
            value_impl_discriminant!(BridgeUnlockFees),
        );
    }

    #[test]
    fn value_impl_bridge_sudo_change_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_bridge_sudo_change_fees_discriminant",
            value_impl_discriminant!(BridgeSudoChangeFees),
        );
    }

    #[test]
    fn value_impl_ibc_relay_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_ibc_relay_fees_discriminant",
            value_impl_discriminant!(IbcRelayFees),
        );
    }

    #[test]
    fn value_impl_validator_update_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_validator_update_fees_discriminant",
            value_impl_discriminant!(ValidatorUpdateFees),
        );
    }

    #[test]
    fn value_impl_fee_asset_change_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_fee_asset_change_fees_discriminant",
            value_impl_discriminant!(FeeAssetChangeFees),
        );
    }

    #[test]
    fn value_impl_fee_change_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_fee_change_fees_discriminant",
            value_impl_discriminant!(FeeChangeFees),
        );
    }

    #[test]
    fn value_impl_ibc_relayer_change_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_ibc_relayer_change_fees_discriminant",
            value_impl_discriminant!(IbcRelayerChangeFees),
        );
    }

    #[test]
    fn value_impl_ibc_sudo_change_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_ibc_sudo_change_fees_discriminant",
            value_impl_discriminant!(IbcSudoChangeFees),
        );
    }

    #[test]
    fn value_impl_sudo_address_change_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_sudo_address_change_fees_discriminant",
            value_impl_discriminant!(SudoAddressChangeFees),
        );
    }

    #[test]
    fn value_impl_bridge_transfer_fees_discriminant_unchanged() {
        assert_snapshot!(
            "value_impl_bridge_transfer_fees_discriminant",
            value_impl_discriminant!(BridgeTransferFees),
        );
    }

    // Note: This test must be here instead of in `crate::storage` since `ValueImpl` is not
    // re-exported.
    #[test]
    fn stored_value_fees_discriminant_unchanged() {
        use crate::storage::StoredValue;
        assert_snapshot!(
            "stored_value_fees_discriminant",
            format!(
                "{:?}",
                discriminant(&StoredValue::Fees(Value(ValueImpl::TransferFees(
                    FeeComponents {
                        base: 1,
                        multiplier: 2,
                    }
                ))))
            )
        );
    }
}
