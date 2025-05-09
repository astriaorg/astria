use astria_core::{
    primitive::v1::asset::Denom,
    protocol::transaction::v1::action::{
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
    Protobuf,
};
use penumbra_ibc::IbcRelay;
use prost::Name as _;

/// The base byte length of a deposit, as determined by
/// [`tests::get_base_deposit_fee()`].
pub(super) const DEPOSIT_BASE_FEE: u128 = 16;

pub(crate) trait FeeHandler: Send {
    /// The Pascal-case type name, e.g. `RollupDataSubmission`.
    fn name(&self) -> &'static str;

    /// The full name including the protobuf package, e.g.
    /// `astria.protocol.transaction.v1.RollupDataSubmission`.
    // NOTE: We only require this function due to `IbcRelay` not implementing `Protobuf`.
    fn full_name() -> String;

    /// The snake-case type name, e.g. `rollup_data_submission`.
    fn snake_case_name() -> &'static str;

    /// The variable value derived from `self` which is multiplied by the `multiplier` of the
    /// `FeeComponents` for this action to produce the variable portion of the total fees for this
    /// action.
    ///
    /// Many actions have fixed fees, meaning this method returns `0`.
    fn variable_component(&self) -> u128;

    /// The asset to be used to pay the fees.
    ///
    /// If this method returns `None`, the action is free.
    fn fee_asset(&self) -> Option<&Denom>;
}

impl FeeHandler for Transfer {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "transfer"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeLock {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_lock"
    }

    fn variable_component(&self) -> u128 {
        base_deposit_fee(&self.asset, &self.destination_chain_address)
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeSudoChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_sudo_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeUnlock {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_unlock"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for BridgeTransfer {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "bridge_transfer"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for InitBridgeAccount {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "init_bridge_account"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for Ics20Withdrawal {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ics20_withdrawal"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for RollupDataSubmission {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "rollup_data_submission"
    }

    fn variable_component(&self) -> u128 {
        u128::try_from(self.data.len())
            .expect("converting a usize to a u128 should work on any currently existing machine")
    }

    fn fee_asset(&self) -> Option<&Denom> {
        Some(&self.fee_asset)
    }
}

impl FeeHandler for ValidatorUpdate {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "validator_update"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for SudoAddressChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "sudo_address_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for FeeChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "fee_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for IbcSudoChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ibc_sudo_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for IbcRelayerChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ibc_relayer_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for FeeAssetChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "fee_asset_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for IbcRelay {
    fn name(&self) -> &'static str {
        penumbra_proto::penumbra::core::component::ibc::v1::IbcRelay::NAME
    }

    fn full_name() -> String {
        penumbra_proto::penumbra::core::component::ibc::v1::IbcRelay::full_name()
    }

    fn snake_case_name() -> &'static str {
        "ibc_relay"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for RecoverIbcClient {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "recover_ibc_client"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for CurrencyPairsChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "currency_pairs_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

impl FeeHandler for MarketsChange {
    fn name(&self) -> &'static str {
        <Self as Protobuf>::Raw::NAME
    }

    fn full_name() -> String {
        <Self as Protobuf>::full_name()
    }

    fn snake_case_name() -> &'static str {
        "markets_change"
    }

    fn variable_component(&self) -> u128 {
        0
    }

    fn fee_asset(&self) -> Option<&Denom> {
        None
    }
}

/// Returns a modified byte length of the deposit event. Length is calculated with reasonable values
/// for all fields except `asset` and `destination_chain_address`, ergo it may not be representative
/// of on-wire length.
pub(super) fn base_deposit_fee(asset: &Denom, destination_chain_address: &str) -> u128 {
    u128::try_from(
        asset
            .display_len()
            .saturating_add(destination_chain_address.len()),
    )
    .expect("converting a usize to a u128 should work on any currently existing machine")
    .saturating_add(DEPOSIT_BASE_FEE)
}
