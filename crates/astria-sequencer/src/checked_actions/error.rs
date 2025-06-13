use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::asset::Denom,
};
use astria_eyre::eyre;
use thiserror::Error;

use crate::accounts::AddressBytes;

#[derive(Debug, Error)]
#[error("`{action_name}` action failed initial check")]
pub(crate) struct CheckedActionInitialCheckError {
    pub(super) action_name: &'static str,
    pub(super) source: eyre::Report,
}

impl CheckedActionInitialCheckError {
    pub(super) fn new(action_name: &'static str, source: eyre::Report) -> Self {
        Self {
            action_name,
            source,
        }
    }
}

#[derive(Debug, Error)]
#[error("`{action_name}` action failed mutable check")]
pub(crate) struct CheckedActionMutableCheckError {
    pub(super) action_name: &'static str,
    pub(super) source: eyre::Report,
}

#[derive(Debug, Error)]
pub(crate) enum CheckedActionFeeError {
    #[error("`{action_name}` action is disabled (fees not set for this action)")]
    ActionDisabled { action_name: &'static str },

    #[error("fee asset {fee_asset} for `{action_name}` action is not allowed")]
    FeeAssetIsNotAllowed {
        fee_asset: Denom,
        action_name: &'static str,
    },

    #[error(
        "insufficient {asset} balance in account {} to pay fee of {amount}",
        account.display_address()
    )]
    InsufficientBalanceToPayFee {
        account: [u8; ADDRESS_LENGTH],
        asset: Denom,
        amount: u128,
    },

    #[error("internal error: {context}")]
    InternalError {
        context: String,
        source: eyre::Report,
    },
}

impl CheckedActionFeeError {
    pub(super) fn internal(context: &str, source: eyre::Report) -> Self {
        Self::InternalError {
            context: context.to_string(),
            source,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum CheckedActionExecutionError {
    #[error("`{action_name}` action failed execution")]
    Execution {
        action_name: &'static str,
        source: eyre::Report,
    },

    #[error("`{action_name}` action failed execution (non-fatal)")]
    NonFatalExecution {
        action_name: &'static str,
        source: eyre::Report,
    },

    #[error(transparent)]
    Fee(#[from] CheckedActionFeeError),
}

impl CheckedActionExecutionError {
    pub(super) fn execution(action_name: &'static str, source: eyre::Report) -> Self {
        Self::Execution {
            action_name,
            source,
        }
    }

    pub(super) fn non_fatal_execution(action_name: &'static str, source: eyre::Report) -> Self {
        Self::NonFatalExecution {
            action_name,
            source,
        }
    }
}
