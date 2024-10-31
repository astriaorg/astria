use astria_core::protocol::transaction::v1::Action;
use astria_eyre::{
    eyre,
    eyre::WrapErr as _,
};
use cnidarium::StateWrite;

#[async_trait::async_trait]
pub(crate) trait ActionHandler {
    async fn check_stateless(&self) -> eyre::Result<()>;

    async fn check_and_execute<S: StateWrite>(
        &self,
        mut state: S,
        context: crate::transaction::Context,
    ) -> astria_eyre::eyre::Result<()>;
}

#[async_trait::async_trait]
impl ActionHandler for Action {
    async fn check_stateless(&self) -> eyre::Result<()> {
        match self {
            Action::Transfer(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for TransferAction"),
            Action::RollupDataSubmission(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for SequenceAction"),
            Action::ValidatorUpdate(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for ValidatorUpdateAction"),
            Action::SudoAddressChange(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for SudoAddressChangeAction"),
            Action::IbcSudoChange(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for IbcSudoChangeAction"),
            Action::FeeChange(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for FeeChangeAction"),
            Action::Ibc(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for IbcRelay action"),
            Action::Ics20Withdrawal(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for Ics20WithdrawalAction"),
            Action::IbcRelayerChange(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for IbcRelayerChangeAction"),
            Action::FeeAssetChange(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for FeeAssetChangeAction"),
            Action::InitBridgeAccount(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for InitBridgeAccountAction"),
            Action::BridgeLock(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for BridgeLockAction"),
            Action::BridgeUnlock(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for BridgeUnlockAction"),
            Action::BridgeSudoChange(act) => act
                .check_stateless()
                .await
                .wrap_err("stateless check failed for BridgeSudoChangeAction"),
        }
    }

    async fn check_and_execute<S: StateWrite>(
        &self,
        state: S,
        context: crate::transaction::Context,
    ) -> astria_eyre::eyre::Result<()> {
        match self {
            Action::Transfer(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing transfer action failed"),
            Action::RollupDataSubmission(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing sequence action failed"),
            Action::ValidatorUpdate(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing validor update"),
            Action::SudoAddressChange(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing sudo address change failed"),
            Action::IbcSudoChange(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing ibc sudo change failed"),
            Action::FeeChange(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing fee change failed"),
            Action::Ibc(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("executing ibc relay failed"),
            Action::Ics20Withdrawal(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing ics20 withdrawal"),
            Action::IbcRelayerChange(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing ibc relayer change"),
            Action::FeeAssetChange(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing fee asseet change"),
            Action::InitBridgeAccount(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing init bridge account"),
            Action::BridgeLock(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing bridge lock"),
            Action::BridgeUnlock(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing bridge unlock"),
            Action::BridgeSudoChange(act) => act
                .check_and_execute(state, context)
                .await
                .wrap_err("failed executing bridge sudo change"),
        }
    }
}
