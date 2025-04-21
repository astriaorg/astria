use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::oracles::price_feed::market_map) struct BlockHeight(u64);

impl From<u64> for BlockHeight {
    fn from(block_height: u64) -> Self {
        BlockHeight(block_height)
    }
}

impl From<BlockHeight> for u64 {
    fn from(block_height: BlockHeight) -> Self {
        block_height.0
    }
}

impl From<BlockHeight> for crate::storage::StoredValue<'_> {
    fn from(block_height: BlockHeight) -> Self {
        crate::storage::StoredValue::PriceFeedMarketMap(Value(ValueImpl::BlockHeight(block_height)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for BlockHeight {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::PriceFeedMarketMap(Value(ValueImpl::BlockHeight(
            block_height,
        ))) = value
        else {
            bail!(
                "price feed market map stored value type mismatch: expected block height, found \
                 {value:?}"
            );
        };
        Ok(block_height)
    }
}
