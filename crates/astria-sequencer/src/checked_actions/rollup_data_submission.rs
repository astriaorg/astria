use astria_core::{
    primitive::v1::asset::IbcPrefixed,
    protocol::transaction::v1::action::RollupDataSubmission,
};
use astria_eyre::eyre::{
    ensure,
    Result,
};
use tracing::{
    instrument,
    Level,
};

use super::AssetTransfer;

#[derive(Debug)]
pub(crate) struct CheckedRollupDataSubmission {
    action: RollupDataSubmission,
}

impl CheckedRollupDataSubmission {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) fn new(action: RollupDataSubmission) -> Result<Self> {
        ensure!(
            !action.data.is_empty(),
            "cannot have empty data for rollup data submission action"
        );

        let checked_action = Self {
            action,
        };

        Ok(checked_action)
    }

    pub(crate) fn action(&self) -> &RollupDataSubmission {
        &self.action
    }
}

impl AssetTransfer for CheckedRollupDataSubmission {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::test_utils::{
        assert_error_contains,
        dummy_rollup_data_submission,
    };

    #[tokio::test]
    async fn should_fail_construction_if_data_is_empty() {
        let action = RollupDataSubmission {
            data: Bytes::new(),
            ..dummy_rollup_data_submission()
        };
        let err = CheckedRollupDataSubmission::new(action).unwrap_err();

        assert_error_contains(
            &err,
            "cannot have empty data for rollup data submission action",
        );
    }
}
