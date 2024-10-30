use astria_core::{
    protocol::transaction::v1::{
        action::{
            BridgeLock,
            BridgeSudoChange,
            BridgeUnlock,
            FeeAssetChange,
            FeeChange,
            IbcRelayerChange,
            IbcSudoChange,
            Ics20Withdrawal,
            InitBridgeAccount,
            RollupDataSubmission,
            SudoAddressChange,
            Transfer,
            ValidatorUpdate,
        },
        Action,
        Transaction,
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    bail,
    ensure,
    WrapErr as _,
};
use cnidarium::StateRead;
use penumbra_ibc::IbcRelayWithHandlers;
use tracing::{
    instrument,
    Level,
};

use crate::{
    accounts::AddressBytes,
    authority::StateReadExt as _,
    bridge::state_ext::StateReadExt as _,
    ibc::{
        host_interface::AstriaHost,
        state_ext::StateReadExt as _,
    },
};

#[cfg(test)]
mod tests;

#[async_trait::async_trait]
pub(crate) trait AuthorizationHandler {
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()>;
}

#[async_trait::async_trait]
impl AuthorizationHandler for Transfer {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        _state: &S,
        _from: &T,
    ) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for RollupDataSubmission {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        _state: &S,
        _from: &T,
    ) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for BridgeLock {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        _state: &S,
        _from: &T,
    ) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for BridgeUnlock {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        // check that the sender of this tx is the authorized withdrawer for the bridge account
        let Some(withdrawer_address) = state
            .get_bridge_account_withdrawer_address(&self.bridge_address)
            .await
            .wrap_err("failed to get bridge account withdrawer address")?
        else {
            bail!("bridge account does not have an associated withdrawer address");
        };

        ensure!(
            withdrawer_address == *from.address_bytes(),
            "unauthorized to unlock bridge account",
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for InitBridgeAccount {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        _state: &S,
        _from: &T,
    ) -> eyre::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler
    for IbcRelayWithHandlers<crate::ibc::ics20_transfer::Ics20Transfer, AstriaHost>
{
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        check_ibc_authorization(state, from).await
    }
}

async fn check_ibc_authorization<S: StateRead, T: AddressBytes>(
    state: &S,
    from: &T,
) -> eyre::Result<()> {
    ensure!(
        state
            .is_ibc_relayer(*from.address_bytes())
            .await
            .wrap_err("failed to check if address is IBC relayer")?,
        "only IBC sudo address can execute IBC actions"
    );
    Ok(())
}

#[async_trait::async_trait]
impl AuthorizationHandler for IbcRelayerChange {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        let ibc_sudo_address = state
            .get_ibc_sudo_address()
            .await
            .wrap_err("failed to get IBC sudo address")?;
        ensure!(
            ibc_sudo_address == *from.address_bytes(),
            "unauthorized address for IBC relayer change"
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for IbcSudoChange {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(
            sudo_address == *from.address_bytes(),
            "signer is not the sudo key"
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for SudoAddressChange {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(
            sudo_address == *from.address_bytes(),
            "signer is not the sudo key"
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for ValidatorUpdate {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(
            sudo_address == *from.address_bytes(),
            "signer is not the sudo key"
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for FeeChange {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(
            sudo_address == *from.address_bytes(),
            "signer is not the sudo key"
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for FeeAssetChange {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        // ensure signer is the valid `sudo` key in state
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to get sudo address from state")?;
        ensure!(
            sudo_address == *from.address_bytes(),
            "signer is not the sudo key"
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for BridgeSudoChange {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
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
            sudo_address == *from.address_bytes(),
            "unauthorized for bridge sudo change action",
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl AuthorizationHandler for Ics20Withdrawal {
    #[instrument(skip_all, err)]
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        _state: &S,
        _from: &T,
    ) -> eyre::Result<()> {
        Ok(())
    }
}

#[instrument(skip_all, err(level = Level::WARN))]
async fn check_authorization<T: AuthorizationHandler + Protobuf, F: AddressBytes, S: StateRead>(
    act: &T,
    state: &S,
    from: &F,
) -> eyre::Result<()> {
    act.check_authorization(state, from).await?;
    Ok(())
}

#[async_trait::async_trait]
impl AuthorizationHandler for Transaction {
    async fn check_authorization<S: StateRead, T: AddressBytes>(
        &self,
        state: &S,
        from: &T,
    ) -> eyre::Result<()> {
        for action in self.actions() {
            match action {
                Action::Transfer(transfer) => {
                    transfer.check_authorization(state, from).await?;
                }
                Action::RollupDataSubmission(rollup_data_submission) => {
                    rollup_data_submission
                        .check_authorization(state, from)
                        .await?;
                }
                Action::BridgeLock(bridge_lock) => {
                    bridge_lock.check_authorization(state, from).await?;
                }
                Action::BridgeUnlock(bridge_unlock) => {
                    bridge_unlock.check_authorization(state, from).await?;
                }
                Action::InitBridgeAccount(init_bridge_account) => {
                    init_bridge_account.check_authorization(state, from).await?;
                }
                Action::BridgeSudoChange(bridge_sudo_change) => {
                    bridge_sudo_change.check_authorization(state, from).await?;
                }
                Action::Ics20Withdrawal(ics20_withdrawal) => {
                    ics20_withdrawal.check_authorization(state, from).await?;
                }
                Action::Ibc(_) => {
                    check_ibc_authorization(state, from).await?;
                }
                Action::IbcRelayerChange(ibc_relayer_change) => {
                    ibc_relayer_change.check_authorization(state, from).await?;
                }
                Action::IbcSudoChange(ibc_sudo_change) => {
                    ibc_sudo_change.check_authorization(state, from).await?;
                }
                Action::SudoAddressChange(sudo_address_change) => {
                    sudo_address_change.check_authorization(state, from).await?;
                }
                Action::ValidatorUpdate(validator_update) => {
                    validator_update.check_authorization(state, from).await?;
                }
                Action::FeeChange(fee_change) => {
                    fee_change.check_authorization(state, from).await?;
                }
                Action::FeeAssetChange(fee_asset_change) => {
                    fee_asset_change.check_authorization(state, from).await?;
                }
            }
        }
        Ok(())
    }
}
