use astria_core::protocol::transaction::v1::action::IbcRelayerChange;
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
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for IbcRelayerChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        match self {
            IbcRelayerChange::Addition(addr) | IbcRelayerChange::Removal(addr) => {
                state.ensure_base_prefix(addr).await.wrap_err(
                    "failed check for base prefix of provided address to be added/removed",
                )?;
            }
        }

        let ibc_sudo_address = state
            .get_ibc_sudo_address()
            .await
            .wrap_err("failed to get IBC sudo address")?;
        ensure!(
            ibc_sudo_address == from,
            "unauthorized address for IBC relayer change"
        );

        match self {
            IbcRelayerChange::Addition(address) => {
                state
                    .put_ibc_relayer_address(address)
                    .wrap_err("failed to put IBC relayer address")?;
            }
            IbcRelayerChange::Removal(address) => {
                state.delete_ibc_relayer_address(address);
            }
        }
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
        benchmark_and_test_utils::{
            assert_eyre_error,
            astria_address,
            ASTRIA_PREFIX,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn ibc_relayer_addition_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let ibc_sudo_address = astria_address(&[1; 20]);
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *ibc_sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_ibc_sudo_address(ibc_sudo_address).unwrap();

        let address_to_add = astria_address(&[0; 20]);
        let action = IbcRelayerChange::Addition(address_to_add);
        action.check_and_execute(&mut state).await.unwrap();

        assert!(state.is_ibc_relayer(address_to_add).await.unwrap());
    }

    #[tokio::test]
    async fn ibc_relayer_removal_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let address_to_remove = astria_address(&[0; 20]);
        let ibc_sudo_address = astria_address(&[1; 20]);
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *ibc_sudo_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_ibc_sudo_address(ibc_sudo_address).unwrap();
        state.put_ibc_relayer_address(&address_to_remove).unwrap();

        assert!(state.is_ibc_relayer(address_to_remove).await.unwrap());

        let action = IbcRelayerChange::Removal(address_to_remove);
        action.check_and_execute(&mut state).await.unwrap();

        assert!(!state.is_ibc_relayer(address_to_remove).await.unwrap());
    }

    #[tokio::test]
    async fn ibc_relayer_addition_fails_if_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let different_prefix = "different_prefix";
        state.put_base_prefix(different_prefix.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = IbcRelayerChange::Addition(astria_address(&[0; 20]));
        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "failed check for base prefix of provided address to be added/removed",
        );
    }

    #[tokio::test]
    async fn ibc_relayer_removal_fails_if_address_is_not_base_prefixed() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let different_prefix = "different_prefix";
        state.put_base_prefix(different_prefix.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = IbcRelayerChange::Removal(astria_address(&[0; 20]));
        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "failed check for base prefix of provided address to be added/removed",
        );
    }

    #[tokio::test]
    async fn ibc_relayer_change_fails_if_signer_is_not_sudo_address() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let ibc_sudo_address = astria_address(&[1; 20]);
        let signer = astria_address(&[2; 20]);
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *signer.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_ibc_sudo_address(ibc_sudo_address).unwrap();

        let action = IbcRelayerChange::Addition(astria_address(&[0; 20]));
        assert_eyre_error(
            &action.check_and_execute(&mut state).await.unwrap_err(),
            "unauthorized address for IBC relayer change",
        );
    }
}
