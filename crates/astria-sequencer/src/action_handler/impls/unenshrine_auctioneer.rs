use astria_core::protocol::transaction::v1::action::{
    Transfer,
    UnenshrineAuctioneer,
};
use astria_eyre::eyre::{
    bail,
    ensure,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    action_handler::{
        check_transfer,
        execute_transfer,
        ActionHandler,
    },
    auctioneer::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
};

#[async_trait]
impl ActionHandler for UnenshrineAuctioneer {
    async fn check_stateless(&self) -> astria_eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> astria_eyre::Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        let is_enshrined_auctioneer = state
            .is_an_enshrined_auctioneer(&self.auctioneer_address)
            .await?;
        ensure!(is_enshrined_auctioneer, "auctioneer not enshrined");

        let enshrined_auctioneer_entry = match state
            .get_enshrined_auctioneer_entry(&self.auctioneer_address)
            .await?
        {
            Some(entry) => entry,
            None => bail!("auctioneer not enshrined"),
        };

        ensure!(
            from == self.staker_address.bytes(),
            "only staker can unenshrine auctioneer"
        );

        // transfer money back to staker
        // now transfer from staker address to auctioneer address
        let transfer_action = Transfer {
            to: enshrined_auctioneer_entry.staker_address,
            asset: enshrined_auctioneer_entry.asset.clone(),
            amount: enshrined_auctioneer_entry.staked_amount,
            fee_asset: enshrined_auctioneer_entry.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &self.auctioneer_address, &state).await?;
        execute_transfer(&transfer_action, &self.auctioneer_address, &mut state).await?;

        state.delete_enshrined_auctioneer_entry(&self.auctioneer_address)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        primitive::v1::{
            asset,
            TransactionId,
        },
        protocol::transaction::v1::action::{
            EnshrineAuctioneer,
            UnenshrineAuctioneer,
        },
    };
    use cnidarium::StateDelta;

    use crate::{
        accounts::{
            AddressBytes,
            StateReadExt,
            StateWriteExt as _,
        },
        action_handler::ActionHandler,
        address::StateWriteExt as _,
        auctioneer::StateReadExt as _,
        authority::StateWriteExt as _,
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
    async fn unenshrine_auctioneer_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let trace_asset = "trace_asset"
            .parse::<asset::denom::TracePrefixed>()
            .unwrap();
        let ibc_asset = trace_asset.to_ibc_prefixed();
        let amount_to_stake = 10000;
        let staker_address = astria_address(&[3; 20]);
        let auctioneer_address = astria_address(&[4; 20]);
        let from_address = astria_address(&[1; 20]);

        state.put_transaction_context(TransactionContext {
            address_bytes: *from_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });
        state.put_base_prefix(ASTRIA_PREFIX.to_string()).unwrap();
        state.put_sudo_address(from_address).unwrap();
        state
            .put_account_balance(&staker_address, &ibc_asset.clone(), amount_to_stake + 100)
            .unwrap();

        let account_balance = state
            .get_account_balance(staker_address.address_bytes(), &ibc_asset)
            .await
            .unwrap();
        assert_eq!(account_balance, amount_to_stake + 100);

        let enshrine_auctioneer = EnshrineAuctioneer {
            staker_address,
            auctioneer_address,
            asset: ibc_asset.into(),
            fee_asset: ibc_asset.into(),
            amount: amount_to_stake,
        };

        enshrine_auctioneer
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let is_enshrined_auctioneer = state
            .is_an_enshrined_auctioneer(&auctioneer_address)
            .await
            .unwrap();
        assert!(is_enshrined_auctioneer, "auctioneer should be enshrined");

        let enshrined_auctioneer_entry = state
            .get_enshrined_auctioneer_entry(&auctioneer_address)
            .await
            .expect("enshrined auctioneer entry should exist")
            .expect("enshrined auctioneer entry should exist");
        assert_eq!(
            enshrined_auctioneer_entry.auctioneer_address, auctioneer_address,
            "auctioneer address should match"
        );
        assert_eq!(
            enshrined_auctioneer_entry.staker_address, staker_address,
            "staker address should match"
        );
        assert_eq!(
            enshrined_auctioneer_entry.staked_amount, amount_to_stake,
            "staked amount should match"
        );
        assert_eq!(
            enshrined_auctioneer_entry.fee_asset,
            ibc_asset.into(),
            "fee assets should match"
        );
        assert_eq!(
            enshrined_auctioneer_entry.asset,
            ibc_asset.into(),
            "asset should match"
        );

        state.put_transaction_context(TransactionContext {
            address_bytes: *staker_address.address_bytes(),
            transaction_id: TransactionId::new([0; 32]),
            position_in_transaction: 0,
        });

        let unenshrine_auctioneer = UnenshrineAuctioneer {
            auctioneer_address,
            staker_address,
            fee_asset: ibc_asset.into(),
            asset: ibc_asset.into(),
        };

        unenshrine_auctioneer
            .check_and_execute(&mut state)
            .await
            .unwrap();

        let is_enshrined_auctioneer = state
            .is_an_enshrined_auctioneer(&auctioneer_address)
            .await
            .unwrap();
        assert!(!is_enshrined_auctioneer, "auctioneer should be enshrined");
    }
}
