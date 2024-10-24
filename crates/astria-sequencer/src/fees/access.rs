use astria_core::protocol::fees::v1::{
    BridgeLockFeeComponents,
    BridgeSudoChangeFeeComponents,
    BridgeUnlockFeeComponents,
    FeeAssetChangeFeeComponents,
    FeeChangeFeeComponents,
    IbcRelayFeeComponents,
    IbcRelayerChangeFeeComponents,
    IbcSudoChangeFeeComponents,
    Ics20WithdrawalFeeComponents,
    InitBridgeAccountFeeComponents,
    RollupDataSubmissionFeeComponents,
    SudoAddressChangeFeeComponents,
    TransferFeeComponents,
    ValidatorUpdateFeeComponents,
};

pub(crate) trait FeeComponents {
    fn base(&self) -> u128;
    fn multiplier(&self) -> u128;
}

macro_rules! impl_fee_components {
    ($($fee_components:tt),* $(,)?) => {
        $(
            impl FeeComponents for $fee_components {
                fn base(&self) -> u128 {
                    self.base
                }

                fn multiplier(&self) -> u128 {
                    self.multiplier
                }
            }
        )*
    };
}

impl_fee_components!(
    TransferFeeComponents,
    RollupDataSubmissionFeeComponents,
    Ics20WithdrawalFeeComponents,
    InitBridgeAccountFeeComponents,
    BridgeLockFeeComponents,
    BridgeUnlockFeeComponents,
    BridgeSudoChangeFeeComponents,
    ValidatorUpdateFeeComponents,
    IbcRelayerChangeFeeComponents,
    IbcRelayFeeComponents,
    FeeAssetChangeFeeComponents,
    FeeChangeFeeComponents,
    SudoAddressChangeFeeComponents,
    IbcSudoChangeFeeComponents,
);
