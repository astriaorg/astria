//! Contains the `ActionHandler` trait, which houses all stateless/stateful checks and execution, as
//! well as all of its implementations.

use astria_core::protocol::transaction::v1::action::Transfer;
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};

use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    bridge::StateReadExt as _,
    transaction::StateReadExt as _,
};

pub(crate) mod impls;

/// This trait is a verbatim copy of `cnidarium_component::ActionHandler`.
///
/// It's duplicated here because all actions are foreign types, forbidding
/// the implementation of [`cnidarium_component::ActionHandler`][1] for
/// these types due to Rust orphan rules.
///
/// [1]: https://github.com/penumbra-zone/penumbra/blob/14959350abcb8cfbf33f9aedc7463fccfd8e3f9f/crates/cnidarium-component/src/action_handler.rs#L30
#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    // Commenting out for the time being as this is currently not being used. Leaving this in
    // for reference as this is copied from cnidarium_component.
    // ```
    // type CheckStatelessContext: Clone + Send + Sync + 'static;
    // async fn check_stateless(&self, context: Self::CheckStatelessContext) -> anyhow::Result<()>;
    // async fn check_historical<S: StateRead + 'static>(&self, _state: Arc<S>) -> anyhow::Result<()> {
    //     Ok(())
    // }
    // ```

    async fn check_stateless(&self) -> astria_eyre::eyre::Result<()>;

    async fn check_and_execute<S: StateWrite>(&self, mut state: S)
        -> astria_eyre::eyre::Result<()>;
}

async fn execute_transfer<S, TAddress>(
    action: &Transfer,
    from: &TAddress,
    mut state: S,
) -> Result<()>
where
    S: StateWrite,
    TAddress: AddressBytes,
{
    let from = from.address_bytes();
    state
        .decrease_balance(from, &action.asset, action.amount)
        .await
        .wrap_err("failed decreasing `from` account balance")?;
    state
        .increase_balance(&action.to, &action.asset, action.amount)
        .await
        .wrap_err("failed increasing `to` account balance")?;

    Ok(())
}

async fn check_transfer<S, TAddress>(action: &Transfer, from: &TAddress, state: &S) -> Result<()>
where
    S: StateRead,
    TAddress: AddressBytes,
{
    state.ensure_base_prefix(&action.to).await.wrap_err(
        "failed ensuring that the destination address matches the permitted base prefix",
    )?;

    // check that the sender is the withdrawer for the bridge account, if the transfer is from a
    // bridge account
    if state
        .is_a_bridge_account(from)
        .await
        .wrap_err("failed to check if from address is a bridge account")?
    {
        let signer = state
            .get_transaction_context()
            .ok_or_eyre("failed to get transaction context")?
            .address_bytes();
        let withdrawer = state
            .get_bridge_account_withdrawer_address(from)
            .await
            .wrap_err("failed to get bridge account withdrawer address")?
            .ok_or_eyre("bridge account must have a withdrawer address set")?;
        ensure!(
            signer == withdrawer,
            "signer is not the authorized withdrawer for the bridge account",
        );
    }

    let transfer_asset = &action.asset;

    let from_transfer_balance = state
        .get_account_balance(from, transfer_asset)
        .await
        .wrap_err("failed to get account balance in transfer check")?;
    ensure!(
        from_transfer_balance >= action.amount,
        "insufficient funds for transfer"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::{
        Address,
        TransactionId,
    };
    use cnidarium::{
        StateDelta,
        TempStorage,
    };

    use super::*;
    use crate::{
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            nria,
            ASTRIA_PREFIX,
        },
        bridge::StateWriteExt as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn check_transfer_fails_if_destination_is_not_base_prefixed() {
        let storage = TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        let different_prefix = "different_prefix";
        let to_address = Address::builder()
            .prefix(different_prefix.to_string())
            .array([0; 20])
            .try_build()
            .unwrap();
        let action = Transfer {
            to: to_address,
            fee_asset: nria().into(),
            asset: nria().into(),
            amount: 100,
        };

        assert_eyre_error(
            &check_transfer(&action, &astria_address(&[1; 20]), &state)
                .await
                .unwrap_err(),
            &format!(
                "address has prefix `{different_prefix}` but only `{ASTRIA_PREFIX}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn check_transfer_fails_if_insufficient_funds_in_sender_account() {
        let storage = TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        let action = Transfer {
            to: astria_address(&[0; 20]),
            fee_asset: nria().into(),
            asset: nria().into(),
            amount: 100,
        };

        assert_eyre_error(
            &check_transfer(&action, &astria_address(&[1; 20]), &state)
                .await
                .unwrap_err(),
            "insufficient funds for transfer",
        );
    }

    #[tokio::test]
    async fn check_transfer_fails_if_from_is_bridge_account_and_signer_is_not_withdrawer() {
        let storage = TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let from_address = astria_address(&[1; 20]);
        let withdrawer_address = astria_address(&[2; 20]);
        let bridge_address = astria_address(&[3; 20]);

        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state
            .put_bridge_account_rollup_id(&bridge_address, [0; 32].into())
            .unwrap();
        state
            .put_bridge_account_withdrawer_address(&bridge_address, withdrawer_address)
            .unwrap();
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();

        let transfer = Transfer {
            to: astria_address(&[4; 20]),
            fee_asset: nria().into(),
            asset: nria().into(),
            amount: 100,
        };
        let err = check_transfer(&transfer, &bridge_address, &state)
            .await
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("signer is not the authorized withdrawer for the bridge account"));
    }
}
