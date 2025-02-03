use astria_core::protocol::{
    auctioneer::v1::EnshrinedAuctioneerEntry,
    transaction::v1::action::{
        EnshrineAuctioneer,
        Transfer,
    },
};
use astria_eyre::eyre::{
    ensure,
    Context,
};
use async_trait::async_trait;
use cnidarium::StateWrite;

use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
    },
    action_handler::{
        check_transfer,
        execute_transfer,
        ActionHandler,
    },
    address::StateReadExt as _,
    auctioneer::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    authority::StateReadExt as _,
    transaction::StateReadExt as _,
};

// TODO - we should set this in genesis and write it to state during `InitChain`
const AMOUNT_TO_STAKE: u128 = 10000;

#[async_trait]
impl ActionHandler for EnshrineAuctioneer {
    async fn check_stateless(&self) -> astria_eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> astria_eyre::Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        // ensure from is sudo address
        // check that the sender of this tx is the authorized sudo address for the bridge account
        let sudo_address = state.get_sudo_address().await?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        state
            .ensure_base_prefix(&self.auctioneer_address)
            .await
            .wrap_err("failed check for base prefix of auctioneer address")?;
        state
            .ensure_base_prefix(&self.staker_address)
            .await
            .wrap_err("failed check for base prefix of auctioneer address")?;

        let is_auctioneer_enshrined = state
            .is_an_enshrined_auctioneer(&self.auctioneer_address)
            .await?;
        ensure!(!is_auctioneer_enshrined, "auctioneer is already enshrined");

        ensure!(
            self.amount == AMOUNT_TO_STAKE,
            "amount to stake is not correct"
        );

        // get staker address balance
        let account_balance = state
            .get_account_balance(self.staker_address.address_bytes(), &self.asset)
            .await?;
        ensure!(
            account_balance >= self.amount,
            "staker does not have enough balance to stake"
        );

        // now transfer from staker address to auctioneer address
        let transfer_action = Transfer {
            to: self.auctioneer_address,
            asset: self.asset.clone(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &self.staker_address, &state).await?;
        execute_transfer(&transfer_action, &self.staker_address, &mut state).await?;

        let enshrined_auctioneer_entry = EnshrinedAuctioneerEntry {
            auctioneer_address: self.auctioneer_address,
            staker_address: self.staker_address,
            staked_amount: self.amount,
            fee_asset: self.fee_asset.clone(),
            asset: self.asset.clone(),
        };
        state
            .put_enshrined_auctioneer_entry(&self.auctioneer_address, enshrined_auctioneer_entry)?;

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
        protocol::transaction::v1::action::EnshrineAuctioneer,
    };
    use cnidarium::StateDelta;

    use crate::{
        accounts::{
            AddressBytes,
            StateReadExt,
            StateWriteExt as _,
        },
        action_handler::{
            impls::enshrine_auctioneer::AMOUNT_TO_STAKE,
            ActionHandler,
        },
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
    async fn enshrine_auctioneer_ok() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let trace_asset = "trace_asset"
            .parse::<asset::denom::TracePrefixed>()
            .unwrap();
        let ibc_asset = trace_asset.to_ibc_prefixed();
        let amount_to_stake = AMOUNT_TO_STAKE;
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
            amount: AMOUNT_TO_STAKE,
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
            enshrined_auctioneer_entry.staked_amount, AMOUNT_TO_STAKE,
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
    }
}
