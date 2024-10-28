use astria_core::{
    primitive::v1::{
        asset::Denom,
        Address,
        Bech32,
    },
    protocol::{
        memos::v1::Ics20WithdrawalFromRollup,
        transaction::v1::action::{
            self,
            BridgeLock,
            BridgeSudoChange,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            InitBridgeAccount,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
    },
    sequencerblock::v1::block::Deposit,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        self,
        bail,
        ensure,
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::StreamExt as _;
use ibc_proto::ibc::apps::transfer::v2::FungibleTokenPacketData;
use ibc_types::core::channel::{
    ChannelId,
    PortId,
};
use penumbra_ibc::component::packet::{
    IBCPacket,
    SendPacketRead as _,
    SendPacketWrite as _,
    Unchecked,
};
use tokio::pin;

use super::ActionHandler;
use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
        StateWriteExt as _,
    },
    address::StateReadExt as _,
    app::StateReadExt as _,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    bridge::{
        StateReadExt as _,
        StateWriteExt,
    },
    fees::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    transaction::StateReadExt as _,
    utils::create_deposit_event,
};

#[async_trait]
impl ActionHandler for BridgeLock {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;
        // ensure the recipient is a bridge account.
        let rollup_id = state
            .get_bridge_account_rollup_id(&self.to)
            .await
            .wrap_err("failed to get bridge account rollup id")?
            .ok_or_eyre("bridge lock must be sent to a bridge account")?;

        let allowed_asset = state
            .get_bridge_account_ibc_asset(&self.to)
            .await
            .wrap_err("failed to get bridge account asset ID")?;
        ensure!(
            allowed_asset == self.asset.to_ibc_prefixed(),
            "asset ID is not authorized for transfer to bridge account",
        );

        let source_transaction_id = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .transaction_id;
        let source_action_index = state
            .get_transaction_context()
            .expect("current source should be set before executing action")
            .source_action_index;

        let deposit = Deposit {
            bridge_address: self.to,
            rollup_id,
            amount: self.amount,
            asset: self.asset.clone(),
            destination_chain_address: self.destination_chain_address.clone(),
            source_transaction_id,
            source_action_index,
        };
        let deposit_abci_event = create_deposit_event(&deposit);

        let transfer_action = Transfer {
            to: self.to,
            asset: self.asset.clone(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &from, &state).await?;
        execute_transfer(&transfer_action, &from, &mut state).await?;

        state.cache_deposit_event(deposit);
        state.record(deposit_abci_event);
        Ok(())
    }
}

#[async_trait]
impl ActionHandler for BridgeSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;
        if let Some(new_sudo_address) = &self.new_sudo_address {
            state
                .ensure_base_prefix(new_sudo_address)
                .await
                .wrap_err("failed check for base prefix of new sudo address")?;
        }
        if let Some(new_withdrawer_address) = &self.new_withdrawer_address {
            state
                .ensure_base_prefix(new_withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of new withdrawer address")?;
        }

        // check that the sender of this tx is the authorized sudo address for the bridge account
        let Some(sudo_address) = state
            .get_bridge_account_sudo_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account sudo address")?
        else {
            // TODO: if the sudo address is unset, should we still allow this action
            // if the sender if the bridge address itself?
            bail!("bridge account does not have an associated sudo address");
        };

        ensure!(
            sudo_address == from,
            "unauthorized for bridge sudo change action",
        );

        if let Some(sudo_address) = self.new_sudo_address {
            state
                .put_bridge_account_sudo_address(&self.bridge_address, sudo_address)
                .wrap_err("failed to put bridge account sudo address")?;
        }

        if let Some(withdrawer_address) = self.new_withdrawer_address {
            state
                .put_bridge_account_withdrawer_address(&self.bridge_address, withdrawer_address)
                .wrap_err("failed to put bridge account withdrawer address")?;
        }

        Ok(())
    }
}

#[async_trait]
impl ActionHandler for BridgeUnlock {
    // TODO(https://github.com/astriaorg/astria/issues/1430): move checks to the `BridgeUnlock` parsing.
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.amount > 0, "amount must be greater than zero",);
        ensure!(self.memo.len() <= 64, "memo must not be more than 64 bytes");
        ensure!(
            !self.rollup_withdrawal_event_id.is_empty(),
            "rollup withdrawal event id must be non-empty",
        );
        ensure!(
            self.rollup_withdrawal_event_id.len() <= 256,
            "rollup withdrawal event id must not be more than 256 bytes",
        );
        ensure!(
            self.rollup_block_number > 0,
            "rollup block number must be greater than zero",
        );
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.to)
            .await
            .wrap_err("failed check for base prefix of destination address")?;
        state
            .ensure_base_prefix(&self.bridge_address)
            .await
            .wrap_err("failed check for base prefix of bridge address")?;

        let asset = state
            .get_bridge_account_ibc_asset(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge's asset id, must be a bridge account")?;

        // check that the sender of this tx is the authorized withdrawer for the bridge account
        let Some(withdrawer_address) = state
            .get_bridge_account_withdrawer_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account withdrawer address")?
        else {
            bail!("bridge account does not have an associated withdrawer address");
        };

        ensure!(
            withdrawer_address == from,
            "unauthorized to unlock bridge account",
        );

        let transfer_action = Transfer {
            to: self.to,
            asset: asset.into(),
            amount: self.amount,
            fee_asset: self.fee_asset.clone(),
        };

        check_transfer(&transfer_action, &self.bridge_address, &state).await?;
        state
            .check_and_set_withdrawal_event_block_for_bridge_account(
                &self.bridge_address,
                &self.rollup_withdrawal_event_id,
                self.rollup_block_number,
            )
            .await
            .context("withdrawal event already processed")?;
        execute_transfer(&transfer_action, &self.bridge_address, state).await?;

        Ok(())
    }
}

#[async_trait]
impl ActionHandler for FeeAssetChange {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> eyre::Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        let authority_sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get authority sudo address")?;
        ensure!(
            authority_sudo_address == from,
            "unauthorized address for fee asset change"
        );
        match self {
            FeeAssetChange::Addition(asset) => {
                state
                    .put_allowed_fee_asset(asset)
                    .context("failed to write allowed fee asset to state")?;
            }
            FeeAssetChange::Removal(asset) => {
                state.delete_allowed_fee_asset(asset);

                pin!(
                    let assets = state.allowed_fee_assets();
                );
                ensure!(
                    assets
                        .filter_map(|item| std::future::ready(item.ok()))
                        .next()
                        .await
                        .is_some(),
                    "cannot remove last allowed fee asset",
                );
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ActionHandler for FeeChange {
    async fn check_stateless(&self) -> eyre::Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the fee
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> eyre::Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        match self {
            Self::Transfer(fees) => state
                .put_transfer_fees(*fees)
                .wrap_err("failed to put transfer fees"),
            Self::RollupDataSubmission(fees) => state
                .put_rollup_data_submission_fees(*fees)
                .wrap_err("failed to put sequence fees"),
            Self::Ics20Withdrawal(fees) => state
                .put_ics20_withdrawal_fees(*fees)
                .wrap_err("failed to put ics20 withdrawal fees"),
            Self::InitBridgeAccount(fees) => state
                .put_init_bridge_account_fees(*fees)
                .wrap_err("failed to put init bridge account fees"),
            Self::BridgeLock(fees) => state
                .put_bridge_lock_fees(*fees)
                .wrap_err("failed to put bridge lock fees"),
            Self::BridgeUnlock(fees) => state
                .put_bridge_unlock_fees(*fees)
                .wrap_err("failed to put bridge unlock fees"),
            Self::BridgeSudoChange(fees) => state
                .put_bridge_sudo_change_fees(*fees)
                .wrap_err("failed to put bridge sudo change fees"),
            Self::IbcRelay(fees) => state
                .put_ibc_relay_fees(*fees)
                .wrap_err("failed to put ibc relay fees"),
            Self::ValidatorUpdate(fees) => state
                .put_validator_update_fees(*fees)
                .wrap_err("failed to put validator update fees"),
            Self::FeeAssetChange(fees) => state
                .put_fee_asset_change_fees(*fees)
                .wrap_err("failed to put fee asset change fees"),
            Self::FeeChange(fees) => state
                .put_fee_change_fees(*fees)
                .wrap_err("failed to put fee change fees"),
            Self::IbcRelayerChange(fees) => state
                .put_ibc_relayer_change_fees(*fees)
                .wrap_err("failed to put ibc relayer change fees"),
            Self::SudoAddressChange(fees) => state
                .put_sudo_address_change_fees(*fees)
                .wrap_err("failed to put sudo address change fees"),
            Self::IbcSudoChange(fees) => state
                .put_ibc_sudo_change_fees(*fees)
                .wrap_err("failed to put ibc sudo change fees"),
        }
    }
}

#[async_trait]
impl ActionHandler for IbcRelayerChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

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

#[async_trait]
impl ActionHandler for IbcSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

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

#[async_trait]
impl ActionHandler for action::Ics20Withdrawal {
    // TODO(https://github.com/astriaorg/astria/issues/1430): move checks to the `Ics20Withdrawal` parsing.
    async fn check_stateless(&self) -> Result<()> {
        ensure!(self.timeout_time() != 0, "timeout time must be non-zero",);
        ensure!(self.amount() > 0, "amount must be greater than zero",);
        if self.bridge_address.is_some() {
            let parsed_bridge_memo: Ics20WithdrawalFromRollup = serde_json::from_str(&self.memo)
                .context("failed to parse memo for ICS bound bridge withdrawal")?;

            ensure!(
                !parsed_bridge_memo.rollup_return_address.is_empty(),
                "rollup return address must be non-empty",
            );
            ensure!(
                parsed_bridge_memo.rollup_return_address.len() <= 256,
                "rollup return address must be no more than 256 bytes",
            );
            ensure!(
                !parsed_bridge_memo.rollup_withdrawal_event_id.is_empty(),
                "rollup withdrawal event id must be non-empty",
            );
            ensure!(
                parsed_bridge_memo.rollup_withdrawal_event_id.len() <= 256,
                "rollup withdrawal event id must be no more than 256 bytes",
            );
            ensure!(
                parsed_bridge_memo.rollup_block_number != 0,
                "rollup block number must be non-zero",
            );
        }

        // NOTE (from penumbra): we could validate the destination chain address as bech32 to
        // prevent mistyped addresses, but this would preclude sending to chains that don't
        // use bech32 addresses.
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        state
            .ensure_base_prefix(&self.return_address)
            .await
            .wrap_err("failed to verify that return address address has permitted base prefix")?;

        if let Some(bridge_address) = &self.bridge_address {
            state.ensure_base_prefix(bridge_address).await.wrap_err(
                "failed to verify that bridge address address has permitted base prefix",
            )?;
            let parsed_bridge_memo: Ics20WithdrawalFromRollup = serde_json::from_str(&self.memo)
                .context("failed to parse memo for ICS bound bridge withdrawal")?;

            state
                .check_and_set_withdrawal_event_block_for_bridge_account(
                    self.bridge_address
                        .as_ref()
                        .map_or(&from, Address::as_bytes),
                    &parsed_bridge_memo.rollup_withdrawal_event_id,
                    parsed_bridge_memo.rollup_block_number,
                )
                .await
                .context("withdrawal event already processed")?;
        }

        let withdrawal_target = establish_withdrawal_target(self, &state, &from)
            .await
            .wrap_err("failed establishing which account to withdraw funds from")?;

        let current_timestamp = state
            .get_block_timestamp()
            .await
            .wrap_err("failed to get block timestamp")?;
        let packet = {
            let packet = create_ibc_packet_from_withdrawal(self, &state)
                .await
                .context("failed converting the withdrawal action into IBC packet")?;
            state
                .send_packet_check(packet, current_timestamp)
                .await
                .map_err(anyhow_to_eyre)
                .wrap_err("packet failed send check")?
        };

        state
            .decrease_balance(withdrawal_target, self.denom(), self.amount())
            .await
            .wrap_err("failed to decrease sender or bridge balance")?;

        // if we're the source, move tokens to the escrow account,
        // otherwise the tokens are just burned
        if is_source(packet.source_port(), packet.source_channel(), self.denom()) {
            let channel_balance = state
                .get_ibc_channel_balance(self.source_channel(), self.denom())
                .await
                .wrap_err("failed to get channel balance")?;

            state
                .put_ibc_channel_balance(
                    self.source_channel(),
                    self.denom(),
                    channel_balance
                        .checked_add(self.amount())
                        .ok_or_eyre("overflow when adding to channel balance")?,
                )
                .wrap_err("failed to update channel balance")?;
        }

        state.send_packet_execute(packet).await;
        Ok(())
    }
}

#[async_trait]
impl ActionHandler for InitBridgeAccount {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        if let Some(withdrawer_address) = &self.withdrawer_address {
            state
                .ensure_base_prefix(withdrawer_address)
                .await
                .wrap_err("failed check for base prefix of withdrawer address")?;
        }
        if let Some(sudo_address) = &self.sudo_address {
            state
                .ensure_base_prefix(sudo_address)
                .await
                .wrap_err("failed check for base prefix of sudo address")?;
        }

        // this prevents the address from being registered as a bridge account
        // if it's been previously initialized as a bridge account.
        //
        // however, there is no prevention of initializing an account as a bridge
        // account that's already been used as a normal EOA.
        //
        // the implication is that the account might already have a balance, nonce, etc.
        // before being converted into a bridge account.
        //
        // after the account becomes a bridge account, it can no longer receive funds
        // via `TransferAction`, only via `BridgeLockAction`.
        if state
            .get_bridge_account_rollup_id(&from)
            .await
            .wrap_err("failed getting rollup ID of bridge account")?
            .is_some()
        {
            bail!("bridge account already exists");
        }

        state
            .put_bridge_account_rollup_id(&from, self.rollup_id)
            .wrap_err("failed to put bridge account rollup id")?;
        state
            .put_bridge_account_ibc_asset(&from, &self.asset)
            .wrap_err("failed to put asset ID")?;
        state.put_bridge_account_sudo_address(
            &from,
            self.sudo_address.map_or(from, Address::bytes),
        )?;
        state.put_bridge_account_withdrawer_address(
            &from,
            self.withdrawer_address.map_or(from, Address::bytes),
        )?;

        Ok(())
    }
}

#[async_trait]
impl ActionHandler for RollupDataSubmission {
    async fn check_stateless(&self) -> Result<()> {
        // TODO: do we want to place a maximum on the size of the data?
        // https://github.com/astriaorg/astria/issues/222
        ensure!(
            !self.data.is_empty(),
            "cannot have empty data for sequence action"
        );
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, _state: S) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl ActionHandler for SudoAddressChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the sudo address
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .wrap_err("desired new sudo address has an unsupported prefix")?;
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");
        state
            .put_sudo_address(self.new_address)
            .wrap_err("failed to put sudo address in state")?;
        Ok(())
    }
}

#[async_trait]
impl ActionHandler for Transfer {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();

        ensure!(
            state
                .get_bridge_account_rollup_id(&from)
                .await
                .wrap_err("failed to get bridge account rollup id")?
                .is_none(),
            "cannot transfer out of bridge account; BridgeUnlock must be used",
        );

        check_transfer(self, &from, &state).await?;
        execute_transfer(self, &from, state).await?;

        Ok(())
    }
}

#[async_trait]
impl ActionHandler for ValidatorUpdate {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let from = state
            .get_transaction_context()
            .expect("transaction source must be present in state when executing an action")
            .address_bytes();
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(sudo_address == from, "signer is not the sudo key");

        // ensure that we're not removing the last validator or a validator
        // that doesn't exist, these both cause issues in cometBFT
        if self.power == 0 {
            let validator_set = state
                .get_validator_set()
                .await
                .wrap_err("failed to get validator set from state")?;
            // check that validator exists
            if validator_set
                .get(self.verification_key.address_bytes())
                .is_none()
            {
                bail!("cannot remove a non-existing validator");
            }
            // check that this is not the only validator, cannot remove the last one
            ensure!(validator_set.len() != 1, "cannot remove the last validator");
        }

        // add validator update in non-consensus state to be used in end_block
        let mut validator_updates = state
            .get_validator_updates()
            .await
            .wrap_err("failed getting validator updates from state")?;
        validator_updates.push_update(self.clone());
        state
            .put_validator_updates(validator_updates)
            .wrap_err("failed to put validator updates in state")?;
        Ok(())
    }
}

pub(in crate::action_handler) async fn create_ibc_packet_from_withdrawal<S: StateRead>(
    withdrawal: &action::Ics20Withdrawal,
    state: S,
) -> Result<IBCPacket<Unchecked>> {
    let sender = if withdrawal.use_compat_address {
        let ibc_compat_prefix = state.get_ibc_compat_prefix().await.context(
            "need to construct bech32 compatible address for IBC communication but failed reading \
             required prefix from state",
        )?;
        withdrawal
            .return_address()
            .to_prefix(&ibc_compat_prefix)
            .context("failed to convert the address to the bech32 compatible prefix")?
            .to_format::<Bech32>()
            .to_string()
    } else {
        withdrawal.return_address.to_string()
    };
    let packet = FungibleTokenPacketData {
        amount: withdrawal.amount.to_string(),
        denom: withdrawal.denom.to_string(),
        sender,
        receiver: withdrawal.destination_chain_address.clone(),
        memo: withdrawal.memo.clone(),
    };

    let serialized_packet_data =
        serde_json::to_vec(&packet).context("failed to serialize fungible token packet as JSON")?;

    Ok(IBCPacket::new(
        PortId::transfer(),
        withdrawal.source_channel().clone(),
        *withdrawal.timeout_height(),
        withdrawal.timeout_time(),
        serialized_packet_data,
    ))
}

/// Establishes the withdrawal target.
///
/// The function returns the following addresses under the following conditions:
/// 1. `action.bridge_address` if `action.bridge_address` is set and `from` is its stored withdrawer
///    address.
/// 2. `from` if `action.bridge_address` is unset and `from` is *not* a bridge account.
///
/// Errors if:
/// 1. Errors reading from DB
/// 2. `action.bridge_address` is set, but `from` is not the withdrawer address.
/// 3. `action.bridge_address` is unset, but `from` is a bridge account.
pub(in crate::action_handler) async fn establish_withdrawal_target<'a, S: StateRead>(
    action: &'a action::Ics20Withdrawal,
    state: &S,
    from: &'a [u8; 20],
) -> Result<&'a [u8; 20]> {
    // If the bridge address is set, the withdrawer on that address must match
    // the from address.
    if let Some(bridge_address) = &action.bridge_address {
        let Some(withdrawer) = state
            .get_bridge_account_withdrawer_address(bridge_address)
            .await
            .wrap_err("failed to get bridge withdrawer")?
        else {
            bail!("bridge address must have a withdrawer address set");
        };

        ensure!(
            &withdrawer == from.address_bytes(),
            "sender does not match bridge withdrawer address; unauthorized"
        );

        return Ok(bridge_address.as_bytes());
    }

    // If the bridge address is not set, the sender must not be a bridge account.
    if state
        .is_a_bridge_account(from)
        .await
        .context("failed to establish whether the sender is a bridge account")?
    {
        bail!("sender cannot be a bridge address if bridge address is not set");
    }

    Ok(from)
}

fn is_source(source_port: &PortId, source_channel: &ChannelId, asset: &Denom) -> bool {
    if let Denom::TracePrefixed(trace) = asset {
        !trace.has_leading_port(source_port) || !trace.has_leading_channel(source_channel)
    } else {
        false
    }
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
