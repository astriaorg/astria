use astria_core::primitive::v1::asset;

pub(crate) mod component;
mod fee_handler;
pub(crate) mod query;
mod state_ext;
pub(crate) mod storage;

#[cfg(test)]
mod tests;

pub(crate) use fee_handler::FeeHandler;
pub(crate) use state_ext::{
    StateReadExt,
    StateWriteExt,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Fee {
    action_name: String,
    asset: asset::Denom,
    amount: u128,
    position_in_transaction: u64,
}

impl Fee {
    pub(crate) fn asset(&self) -> &asset::Denom {
        &self.asset
    }

    pub(crate) fn amount(&self) -> u128 {
        self.amount
    }
}
