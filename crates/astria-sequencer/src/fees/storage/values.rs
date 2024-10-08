use astria_core::protocol::transaction::v1alpha1::action::FeeComponents as ActionFeeComponents;
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Value(ValueImpl);

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum ValueImpl {
    Fees(FeeComponents),
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::fees) struct FeeComponents(ActionFeeComponents);

impl From<ActionFeeComponents> for FeeComponents {
    fn from(fees: ActionFeeComponents) -> Self {
        Self(fees)
    }
}

impl From<FeeComponents> for ActionFeeComponents {
    fn from(fees: FeeComponents) -> Self {
        fees.0
    }
}

impl<'a> From<FeeComponents> for crate::storage::StoredValue<'a> {
    fn from(fees: FeeComponents) -> Self {
        crate::storage::StoredValue::Fees(Value(ValueImpl::Fees(fees)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for FeeComponents {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Fees(Value(ValueImpl::Fees(fees))) = value else {
            bail!("fees stored value type mismatch: expected FeeComponents, found {value:?}");
        };
        Ok(fees)
    }
}
