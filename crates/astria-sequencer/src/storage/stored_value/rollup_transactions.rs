use std::borrow::Cow;

use astria_core::sequencerblock::v1alpha1::{
    block::RollupTransactionsParts,
    RollupTransactions as DomainRollupTransactions,
};
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use bytes::Bytes;

use super::{
    Proof,
    RollupId,
    StoredValue,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct RollupTransactions<'a> {
    rollup_id: RollupId<'a>,
    transactions: Cow<'a, [Bytes]>,
    proof: Proof<'a>,
}

impl<'a> From<&'a DomainRollupTransactions> for RollupTransactions<'a> {
    fn from(rollup_txs: &'a DomainRollupTransactions) -> Self {
        RollupTransactions {
            rollup_id: rollup_txs.rollup_id().into(),
            transactions: Cow::Borrowed(rollup_txs.transactions()),
            proof: rollup_txs.proof().into(),
        }
    }
}

impl<'a> From<RollupTransactions<'a>> for DomainRollupTransactions {
    fn from(rollup_txs: RollupTransactions<'a>) -> Self {
        DomainRollupTransactions::unchecked_from_parts(RollupTransactionsParts {
            rollup_id: rollup_txs.rollup_id.into(),
            transactions: rollup_txs.transactions.into_owned(),
            proof: rollup_txs.proof.into(),
        })
    }
}

impl<'a> TryFrom<StoredValue<'a>> for RollupTransactions<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::RollupTransactions(rollup_transactions) = value else {
            return Err(super::type_mismatch("rollup transactions", &value));
        };
        Ok(rollup_transactions)
    }
}
