use std::{
    borrow::Cow,
    fmt::{
        self,
        Debug,
        Formatter,
    },
};

use astria_core::sequencerblock::v1::{
    block::RollupTransactionsParts,
    RollupTransactions as DomainRollupTransactions,
};
use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use bytes::Bytes;

use super::{
    proof::Proof,
    rollup_ids::RollupId,
    Value,
    ValueImpl,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct RollupTransactions<'a> {
    rollup_id: RollupId<'a>,
    transactions: Cow<'a, [Bytes]>,
    proof: Proof<'a>,
}

impl Debug for RollupTransactions<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RollupTransactions")
            .field("rollup_id", &self.rollup_id)
            .field(
                "transactions",
                &format!(
                    "{} txs totalling {} bytes",
                    self.transactions.len(),
                    self.transactions.iter().map(Bytes::len).sum::<usize>()
                ),
            )
            .field("proof", &self.proof)
            .finish()
    }
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

impl<'a> From<RollupTransactions<'a>> for crate::storage::StoredValue<'a> {
    fn from(rollup_txs: RollupTransactions<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::RollupTransactions(rollup_txs)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for RollupTransactions<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::RollupTransactions(rollup_txs))) =
            value
        else {
            bail!("grpc stored value type mismatch: expected rollup transactions, found {value:?}");
        };
        Ok(rollup_txs)
    }
}
