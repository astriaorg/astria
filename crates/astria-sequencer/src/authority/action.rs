use astria_core::protocol::transaction::v1::action::{
    IbcSudoChange,
    SudoAddressChange,
    ValidatorUpdate,
};
use astria_eyre::eyre::{
    bail,
    ensure,
    Result,
    WrapErr as _,
};
use cnidarium::StateWrite;

use crate::{
    address::StateReadExt as _,
    app::ActionHandler,
    authority::{
        StateReadExt as _,
        StateWriteExt as _,
    },
    ibc::StateWriteExt as _,
};

#[async_trait::async_trait]
impl ActionHandler for ValidatorUpdate {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
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

#[async_trait::async_trait]
impl ActionHandler for SudoAddressChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    /// check that the signer of the transaction is the current sudo address,
    /// as only that address can change the sudo address
    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .wrap_err("desired new sudo address has an unsupported prefix")?;
        state
            .put_sudo_address(self.new_address)
            .wrap_err("failed to put sudo address in state")?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActionHandler for IbcSudoChange {
    async fn check_stateless(&self) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        state
            .ensure_base_prefix(&self.new_address)
            .await
            .wrap_err("desired new ibc sudo address has an unsupported prefix")?;
        state
            .put_ibc_sudo_address(self.new_address)
            .wrap_err("failed to put ibc sudo address in state")?;
        Ok(())
    }
}
