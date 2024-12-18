use astria_core::protocol::transaction::v1::action::RemoveMarketAuthorities;
use astria_eyre::eyre::{
    self,
    ensure,
    Context as _,
    OptionExt as _,
};
use cnidarium::StateWrite;

use crate::{
    action_handler::ActionHandler,
    address::StateReadExt as _,
    connect::market_map::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for RemoveMarketAuthorities {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> eyre::Result<()> {
        let from = state
            .try_base_prefixed(
                &state
                    .get_transaction_context()
                    .expect("transaction source must be present in state when executing an action")
                    .address_bytes(),
            )
            .await
            .wrap_err("failed to convert signer address to base prefixed address")?;
        let mut params = state
            .get_params()
            .await
            .wrap_err("failed to obtain market map params from state")?
            .ok_or_eyre("market map params not found in state")?;
        ensure!(params.admin == from, "signer is not the market map admin");
        for address in &self.remove_addresses {
            params.market_authorities.retain(|a| a != address);
        }
        state
            .put_params(params)
            .wrap_err("failed to put params into state")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        connect::market_map::v2::Params,
        primitive::v1::TransactionId,
        protocol::transaction::v1::action::RemoveMarketAuthorities,
    };

    use super::*;
    use crate::{
        accounts::AddressBytes as _,
        address::StateWriteExt as _,
        benchmark_and_test_utils::{
            astria_address,
            ASTRIA_PREFIX,
        },
        transaction::{
            StateWriteExt as _,
            TransactionContext,
        },
    };

    #[tokio::test]
    async fn remove_market_authorities_executes_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let admin_address = astria_address(&[0; 20]);
        let address_1 = astria_address(&[1; 20]);
        let address_2 = astria_address(&[2; 20]);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *admin_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![address_1, address_2],
            admin: admin_address,
        };
        state.put_params(params.clone()).unwrap();

        assert_eq!(state.get_params().await.unwrap().unwrap(), params);

        let action = RemoveMarketAuthorities {
            remove_addresses: vec![address_1],
        };
        action.check_and_execute(&mut state).await.unwrap();
        let params = state.get_params().await.unwrap().unwrap();
        assert_eq!(params.market_authorities, vec![address_2]);
    }

    #[tokio::test]
    async fn remove_market_authorities_fails_if_admin_address_is_invalid() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let admin_address = astria_address(&[0; 20]);
        let other_address = astria_address(&[1; 20]);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *other_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![],
            admin: admin_address,
        };
        state.put_params(params.clone()).unwrap();

        let action = RemoveMarketAuthorities {
            remove_addresses: vec![],
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert_eq!(res.to_string(), "signer is not the market map admin");
    }

    #[tokio::test]
    async fn remove_market_authorities_skips_missing_addresses() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let admin_address = astria_address(&[0; 20]);
        let address_1 = astria_address(&[1; 20]);
        let address_2 = astria_address(&[2; 20]);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: *admin_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let params = Params {
            market_authorities: vec![address_2],
            admin: admin_address,
        };
        state.put_params(params.clone()).unwrap();

        assert_eq!(state.get_params().await.unwrap().unwrap(), params);

        let action = RemoveMarketAuthorities {
            remove_addresses: vec![address_1],
        };
        action.check_and_execute(&mut state).await.unwrap();
        let params = state.get_params().await.unwrap().unwrap();
        assert_eq!(params.market_authorities, vec![address_2]);
    }

    #[tokio::test]
    async fn remove_market_authorities_fails_if_params_not_found() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_transaction_context(TransactionContext {
            address_bytes: [0; 20],
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let action = RemoveMarketAuthorities {
            remove_addresses: vec![],
        };
        let res = action.check_and_execute(&mut state).await.unwrap_err();
        assert!(
            res.to_string()
                .contains("market map params not found in state")
        );
        assert!(state.get_market_map().await.unwrap().is_none());
    }
}
