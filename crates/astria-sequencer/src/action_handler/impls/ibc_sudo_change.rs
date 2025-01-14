use astria_core::protocol::transaction::v1::action::IbcSudoChange;
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};
use async_trait::async_trait;
use cnidarium::StateWrite;
use tracing::{
    instrument,
    Level,
};

use crate::{
    action_handler::ActionHandler,
    address::StateReadExt as _,
    authority::StateReadExt as _,
    ibc::StateWriteExt as _,
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for IbcSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .wrap_err("desired new ibc sudo address has an unsupported prefix")?;
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        state
            .put_ibc_sudo_address(self.new_address)
            .wrap_err("failed to put ibc sudo address in state")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::primitive::v1::TransactionId;

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        authority::StateWriteExt as _,
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            ASTRIA_PREFIX,
        },
        ibc::StateReadExt as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn ibc_sudo_change_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let old_ibc_sudo_address = astria_address(&[0; 20]);
        let new_ibc_sudo_address = astria_address(&[1; 20]);
        let sudo_address = astria_address(&[2; 20]);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_sudo_address(sudo_address).unwrap();
        state.put_ibc_sudo_address(old_ibc_sudo_address).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        assert_eq!(
            state.get_ibc_sudo_address().await.unwrap(),
            *old_ibc_sudo_address.address_bytes()
        );

        let action = IbcSudoChange {
            new_address: new_ibc_sudo_address,
        };

        action.check_and_execute(&mut state).await.unwrap();

        assert_eq!(
            state.get_ibc_sudo_address().await.unwrap(),
            *new_ibc_sudo_address.address_bytes()
        );
    }

    #[tokio::test]
    async fn ibc_sudo_change_fails_if_new_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let new_ibc_sudo_address = astria_address(&[1; 20]);

        let different_prefix = "different_prefix";
        state.put_base_prefix(different_prefix.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [2; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = IbcSudoChange {
            new_address: new_ibc_sudo_address,
        };

        assert_eyre_error(
            &action.check_and_execute(state).await.unwrap_err(),
            &format!(
                "address has prefix `{ASTRIA_PREFIX}` but only `{different_prefix}` is permitted"
            ),
        );
    }

    #[tokio::test]
    async fn ibc_sudo_change_fails_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let sudo_address = astria_address(&[0; 20]);
        let signer = astria_address(&[1; 20]);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *signer.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_sudo_address(sudo_address).unwrap();

        let action = IbcSudoChange {
            new_address: astria_address(&[2; 20]),
        };

        assert_eyre_error(
            &action.check_and_execute(state).await.unwrap_err(),
            "signer is not the sudo key",
        );
    }
}
