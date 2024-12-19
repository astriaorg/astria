use astria_core::protocol::transaction::v1::action::UpdateMarketMapParams;
use astria_eyre::eyre::{
    self,
    ensure,
    Context,
};

use crate::{
    action_handler::ActionHandler,
    authority::StateReadExt as _,
    connect::market_map::state_ext::StateWriteExt as _,
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for UpdateMarketMapParams {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: cnidarium::StateWrite>(&self, mut state: S) -> eyre::Result<()> {
        let from = &state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == *from, "signer is not the sudo key");
        state
            .put_params(self.params.clone())
            .wrap_err("failed to put params into state")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use astria_core::{
        connect::market_map::v2::Params,
        primitive::v1::TransactionId,
    };

    use super::*;
    use crate::{
        accounts::AddressBytes,
        authority::StateWriteExt as _,
        benchmark_and_test_utils::astria_address,
        connect::market_map::state_ext::StateReadExt as _,
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn update_market_map_params_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);

        state.put_sudo_address(authority_address).unwrap();

        state.put_transaction_context(TransactionContext {
            address_bytes: *authority_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        assert!(state.get_params().await.unwrap().is_none());

        let action = UpdateMarketMapParams {
            params: Params {
                market_authorities: vec![authority_address],
                admin: authority_address,
            },
        };
        action.check_and_execute(&mut state).await.unwrap();
        let params = state
            .get_params()
            .await
            .unwrap()
            .expect("params should be present");
        assert_eq!(params, action.params);
    }

    #[tokio::test]
    async fn update_market_map_params_fails_if_signer_is_invalid() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let authority_address = astria_address(&[0; 20]);
        let invalid_address = astria_address(&[1; 20]);

        state.put_sudo_address(authority_address).unwrap();

        state.put_transaction_context(TransactionContext {
            address_bytes: *invalid_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = UpdateMarketMapParams {
            params: Params {
                market_authorities: vec![invalid_address],
                admin: invalid_address,
            },
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(res.to_string().contains("signer is not the sudo key"));
    }
}
