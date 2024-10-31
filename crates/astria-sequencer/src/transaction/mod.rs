use std::fmt;

use astria_core::{
    primitive::v1::{
        TransactionId,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::Transaction,
};
use astria_eyre::eyre::{
    self,
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
pub(crate) use checks::{
    check_balance_for_total_fees_and_transfers,
    check_chain_id_mempool,
    get_total_transaction_cost,
};
use cnidarium::StateWrite;

mod checks;

struct StrictSequential;

#[async_trait::async_trait]
trait ExecutionStrictness {
    async fn check_execute_and_pay<S: StateWrite>(
        state: S,
        transaction: &Transaction,
    ) -> eyre::Result<()>;
}

#[async_trait::async_trait]
impl ExecutionStrictness for StrictSequential {
    async fn check_execute_and_pay<S: StateWrite>(
        state: S,
        transaction: &Transaction,
    ) -> eyre::Result<()> {
        process_actions_sequential(state, transaction).await
    }
}

pub(crate) async fn check_and_execute_strict<S>(
    transaction: &Transaction,
    state: S,
) -> eyre::Result<()>
where
    S: StateWrite,
{
    check_and_execute_impl::<StrictSequential, _>(transaction, state).await
}

async fn check_and_execute_impl<TExecutionStrictness, TState>(
    transaction: &Transaction,
    mut state: TState,
) -> eyre::Result<()>
where
    TExecutionStrictness: ExecutionStrictness,
    TState: StateWrite,
{
    // Transactions must match the chain id of the node.
    let chain_id = state.get_chain_id().await?;
    ensure!(
        transaction.chain_id() == chain_id.as_str(),
        InvalidChainId(transaction.chain_id().to_string())
    );

    // Nonce should be equal to the number of executed transactions before this tx.
    // First tx has nonce 0.
    let curr_nonce = state
        .get_account_nonce(transaction)
        .await
        .wrap_err("failed to get nonce for transaction signer")?;
    ensure!(
        curr_nonce == transaction.nonce(),
        InvalidNonce(transaction.nonce())
    );

    // Should have enough balance to cover all actions.
    check_balance_for_total_fees_and_transfers(transaction, &state)
        .await
        .wrap_err("failed to check balance for total fees and transfers")?;

    if state
        .get_bridge_account_rollup_id(transaction)
        .await
        .wrap_err("failed to check account rollup id")?
        .is_some()
    {
        state
            .put_last_transaction_id_for_bridge_account(transaction, transaction.id())
            .wrap_err("failed to put last transaction id for bridge account")?;
    }

    let from_nonce = state
        .get_account_nonce(transaction)
        .await
        .wrap_err("failed getting nonce of transaction signer")?;
    let next_nonce = from_nonce
        .checked_add(1)
        .ok_or_eyre("overflow occurred incrementing stored nonce")?;
    state
        .put_account_nonce(transaction.address_bytes(), next_nonce)
        .wrap_err("failed updating `from` nonce")?;

    TExecutionStrictness::check_execute_and_pay(state, transaction).await?;

    Ok(())
}

async fn process_actions_sequential<S: StateWrite>(
    mut state: S,
    transaction: &Transaction,
) -> eyre::Result<()> {
    for (i, action) in (0..).zip(transaction.actions().iter()) {
        let context = Context {
            address_bytes: *transaction.address_bytes(),
            transaction_id: transaction.id(),
            source_action_index: i,
        };
        check_execute_and_pay(action, &mut state, context).await?;
    }
    Ok(())
}

pub(crate) async fn check_stateless(transaction: &Transaction) -> Result<()> {
    ensure!(
        !transaction.actions().is_empty(),
        "must have at least one action"
    );

    for action in transaction.actions() {
        action.check_stateless().await?;
    }
    Ok(())
}

use crate::{
    accounts::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    app::{
        ActionHandler,
        StateReadExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    fees::FeeHandler,
};

#[derive(Debug)]
pub(crate) struct InvalidChainId(pub(crate) String);

impl fmt::Display for InvalidChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "provided chain id {} does not match expected chain id",
            self.0,
        )
    }
}

impl std::error::Error for InvalidChainId {}

#[derive(Debug)]
pub(crate) struct InvalidNonce(pub(crate) u32);

impl fmt::Display for InvalidNonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "provided nonce {} does not match expected next nonce",
            self.0,
        )
    }
}

impl std::error::Error for InvalidNonce {}

async fn check_execute_and_pay<T: ActionHandler + FeeHandler + Sync, S: StateWrite>(
    action: &T,
    mut state: S,
    context: Context,
) -> Result<()> {
    action.check_and_execute(&mut state, context).await?;
    action.check_and_pay_fees(&mut state, context).await?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct Context {
    pub(crate) address_bytes: [u8; ADDRESS_LEN],
    pub(crate) transaction_id: TransactionId,
    pub(crate) source_action_index: u64,
}
