use astria_core::protocol::transaction::v1alpha1::action::{
    BridgeLockFeeComponents,
    BridgeSudoChangeFeeComponents,
    BridgeUnlockFeeComponents,
    Ics20WithdrawalFeeComponents,
    InitBridgeAccountFeeComponents,
    SequenceFeeComponents,
    TransferFeeComponents,
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
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct TransferFeeComponentsStorage(TransferFeeComponents);

impl From<TransferFeeComponents> for TransferFeeComponentsStorage {
    fn from(fees: TransferFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<TransferFeeComponentsStorage> for TransferFeeComponents {
    fn from(fees: TransferFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<TransferFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: TransferFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::TransferFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for TransferFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::TransferFees(fees))) = value else {
            bail!(
                "fees stored value type mismatch: expected TransferFeeComponents, found {value:?}"
            );
        };
        Ok(fees)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct SequenceFeeComponentsStorage(SequenceFeeComponents);

impl From<SequenceFeeComponents> for SequenceFeeComponentsStorage {
    fn from(fees: SequenceFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<SequenceFeeComponentsStorage> for SequenceFeeComponents {
    fn from(fees: SequenceFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<SequenceFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: SequenceFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::SequenceFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for SequenceFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::SequenceFees(fees))) = value else {
            bail!(
                "fees stored value type mismatch: expected SequenceFeeComponents, found {value:?}"
            );
        };
        Ok(fees)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct Ics20WithdrawalFeeComponentsStorage(Ics20WithdrawalFeeComponents);

impl From<Ics20WithdrawalFeeComponents> for Ics20WithdrawalFeeComponentsStorage {
    fn from(fees: Ics20WithdrawalFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<Ics20WithdrawalFeeComponentsStorage> for Ics20WithdrawalFeeComponents {
    fn from(fees: Ics20WithdrawalFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<Ics20WithdrawalFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: Ics20WithdrawalFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::Ics20WithdrawalFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Ics20WithdrawalFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::Ics20WithdrawalFees(fees))) = value
        else {
            bail!(
                "fees stored value type mismatch: expected Ics20WithdrawalFeeComponents, found \
                 {value:?}"
            );
        };
        Ok(fees)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct InitBridgeAccountFeeComponentsStorage(InitBridgeAccountFeeComponents);

impl From<InitBridgeAccountFeeComponents> for InitBridgeAccountFeeComponentsStorage {
    fn from(fees: InitBridgeAccountFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<InitBridgeAccountFeeComponentsStorage> for InitBridgeAccountFeeComponents {
    fn from(fees: InitBridgeAccountFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<InitBridgeAccountFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: InitBridgeAccountFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::InitBridgeAccountFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for InitBridgeAccountFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::InitBridgeAccountFees(fees))) =
            value
        else {
            bail!(
                "fees stored value type mismatch: expected InitBridgeAccountFees, found {value:?}"
            );
        };
        Ok(fees)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct BridgeLockFeeComponentsStorage(BridgeLockFeeComponents);

impl From<BridgeLockFeeComponents> for BridgeLockFeeComponentsStorage {
    fn from(fees: BridgeLockFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<BridgeLockFeeComponentsStorage> for BridgeLockFeeComponents {
    fn from(fees: BridgeLockFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<BridgeLockFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: BridgeLockFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::BridgeLockFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BridgeLockFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::BridgeLockFees(fees))) = value
        else {
            bail!(
                "fees stored value type mismatch: expected BridgeLockFeeComponents, found \
                 {value:?}"
            );
        };
        Ok(fees)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct BridgeUnlockFeeComponentsStorage(BridgeUnlockFeeComponents);

impl From<BridgeUnlockFeeComponents> for BridgeUnlockFeeComponentsStorage {
    fn from(fees: BridgeUnlockFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<BridgeUnlockFeeComponentsStorage> for BridgeUnlockFeeComponents {
    fn from(fees: BridgeUnlockFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<BridgeUnlockFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: BridgeUnlockFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::BridgeUnlockFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BridgeUnlockFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::BridgeUnlockFees(fees))) = value
        else {
            bail!(
                "fees stored value type mismatch: expected BridgeUnlockFeeComponents, found \
                 {value:?}"
            );
        };
        Ok(fees)
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct BridgeSudoChangeFeeComponentsStorage(BridgeSudoChangeFeeComponents);

impl From<BridgeSudoChangeFeeComponents> for BridgeSudoChangeFeeComponentsStorage {
    fn from(fees: BridgeSudoChangeFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<BridgeSudoChangeFeeComponentsStorage> for BridgeSudoChangeFeeComponents {
    fn from(fees: BridgeSudoChangeFeeComponentsStorage) -> Self {
        fees.0
    }
}

impl<'a> From<BridgeSudoChangeFeeComponentsStorage> for crate::storage::StoredValue<'a> {
    fn from(fees: BridgeSudoChangeFeeComponentsStorage) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::BridgeSudoChangeFees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BridgeSudoChangeFeeComponentsStorage {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::BridgeSudoChangeFees(fees))) = value
        else {
            bail!(
                "fees stored value type mismatch: expected BridgeSudoChangeFeeComponents, found \
                 {value:?}"
            );
        };
        Ok(fees)
    }
}
