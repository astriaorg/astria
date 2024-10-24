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

impl FeeComponents for TransferFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for RollupDataSubmissionFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for Ics20WithdrawalFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for InitBridgeAccountFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for BridgeLockFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for BridgeUnlockFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for BridgeSudoChangeFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for ValidatorUpdateFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for IbcRelayerChangeFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for IbcRelayFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for FeeAssetChangeFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for FeeChangeFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for SudoAddressChangeFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}

impl FeeComponents for IbcSudoChangeFeeComponents {
    fn base(&self) -> u128 {
        self.base
    }

    fn multiplier(&self) -> u128 {
        self.multiplier
    }
}
