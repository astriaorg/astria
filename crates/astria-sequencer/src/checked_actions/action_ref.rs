use astria_core::protocol::transaction::v1::{
    action::{
        BridgeLock,
        BridgeSudoChange,
        BridgeTransfer,
        BridgeUnlock,
        CurrencyPairsChange,
        FeeAssetChange,
        FeeChange,
        IbcRelayerChange,
        IbcSudoChange,
        Ics20Withdrawal,
        InitBridgeAccount,
        MarketsChange,
        RecoverIbcClient,
        RollupDataSubmission,
        SudoAddressChange,
        Transfer,
        ValidatorUpdate,
    },
    Action,
};
use penumbra_ibc::IbcRelay;

use super::CheckedAction;

pub(crate) enum ActionRef<'a> {
    RollupDataSubmission(&'a RollupDataSubmission),
    Transfer(&'a Transfer),
    ValidatorUpdate(&'a ValidatorUpdate),
    SudoAddressChange(&'a SudoAddressChange),
    Ibc(&'a IbcRelay),
    IbcSudoChange(&'a IbcSudoChange),
    Ics20Withdrawal(&'a Ics20Withdrawal),
    IbcRelayerChange(&'a IbcRelayerChange),
    FeeAssetChange(&'a FeeAssetChange),
    InitBridgeAccount(&'a InitBridgeAccount),
    BridgeLock(&'a BridgeLock),
    BridgeUnlock(&'a BridgeUnlock),
    BridgeSudoChange(&'a BridgeSudoChange),
    BridgeTransfer(&'a BridgeTransfer),
    FeeChange(&'a FeeChange),
    RecoverIbcClient(&'a RecoverIbcClient),
    CurrencyPairsChange(&'a CurrencyPairsChange),
    MarketsChange(&'a MarketsChange),
}

impl<'a> From<&'a Action> for ActionRef<'a> {
    fn from(action: &'a Action) -> Self {
        match action {
            Action::RollupDataSubmission(action) => ActionRef::RollupDataSubmission(action),
            Action::Transfer(action) => ActionRef::Transfer(action),
            Action::ValidatorUpdate(action) => ActionRef::ValidatorUpdate(action),
            Action::SudoAddressChange(action) => ActionRef::SudoAddressChange(action),
            Action::Ibc(action) => ActionRef::Ibc(action),
            Action::IbcSudoChange(action) => ActionRef::IbcSudoChange(action),
            Action::Ics20Withdrawal(action) => ActionRef::Ics20Withdrawal(action),
            Action::IbcRelayerChange(action) => ActionRef::IbcRelayerChange(action),
            Action::FeeAssetChange(action) => ActionRef::FeeAssetChange(action),
            Action::InitBridgeAccount(action) => ActionRef::InitBridgeAccount(action),
            Action::BridgeLock(action) => ActionRef::BridgeLock(action),
            Action::BridgeUnlock(action) => ActionRef::BridgeUnlock(action),
            Action::BridgeSudoChange(action) => ActionRef::BridgeSudoChange(action),
            Action::BridgeTransfer(action) => ActionRef::BridgeTransfer(action),
            Action::FeeChange(action) => ActionRef::FeeChange(action),
            Action::RecoverIbcClient(action) => ActionRef::RecoverIbcClient(action),
            Action::CurrencyPairsChange(action) => ActionRef::CurrencyPairsChange(action),
            Action::MarketsChange(action) => ActionRef::MarketsChange(action),
        }
    }
}

impl<'a> From<&'a CheckedAction> for ActionRef<'a> {
    fn from(checked_action: &'a CheckedAction) -> Self {
        match checked_action {
            CheckedAction::RollupDataSubmission(checked_action) => {
                ActionRef::RollupDataSubmission(checked_action.action())
            }
            CheckedAction::Transfer(checked_action) => ActionRef::Transfer(checked_action.action()),
            CheckedAction::ValidatorUpdate(checked_action) => {
                ActionRef::ValidatorUpdate(checked_action.action())
            }
            CheckedAction::SudoAddressChange(checked_action) => {
                ActionRef::SudoAddressChange(checked_action.action())
            }
            CheckedAction::IbcRelay(checked_action) => ActionRef::Ibc(checked_action.action()),
            CheckedAction::IbcSudoChange(checked_action) => {
                ActionRef::IbcSudoChange(checked_action.action())
            }
            CheckedAction::Ics20Withdrawal(checked_action) => {
                ActionRef::Ics20Withdrawal(checked_action.action())
            }
            CheckedAction::IbcRelayerChange(checked_action) => {
                ActionRef::IbcRelayerChange(checked_action.action())
            }
            CheckedAction::FeeAssetChange(checked_action) => {
                ActionRef::FeeAssetChange(checked_action.action())
            }
            CheckedAction::InitBridgeAccount(checked_action) => {
                ActionRef::InitBridgeAccount(checked_action.action())
            }
            CheckedAction::BridgeLock(checked_action) => {
                ActionRef::BridgeLock(checked_action.action())
            }
            CheckedAction::BridgeUnlock(checked_action) => {
                ActionRef::BridgeUnlock(checked_action.action())
            }
            CheckedAction::BridgeSudoChange(checked_action) => {
                ActionRef::BridgeSudoChange(checked_action.action())
            }
            CheckedAction::BridgeTransfer(checked_action) => {
                ActionRef::BridgeTransfer(checked_action.action())
            }
            CheckedAction::FeeChange(checked_action) => {
                ActionRef::FeeChange(checked_action.action())
            }
            CheckedAction::RecoverIbcClient(checked_action) => {
                ActionRef::RecoverIbcClient(checked_action.action())
            }
            CheckedAction::CurrencyPairsChange(checked_action) => {
                ActionRef::CurrencyPairsChange(checked_action.action())
            }
            CheckedAction::MarketsChange(checked_action) => {
                ActionRef::MarketsChange(checked_action.action())
            }
        }
    }
}
