use astria_core::connect::market_map::v2::{
    MarketMap,
    Params,
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_market_map(&self) -> Result<Option<MarketMap>> {
        let Some(bytes) = self
            .get_raw(keys::MARKET_MAP)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get market map from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::MarketMap::try_from(value)
                    .map(|market_map| Some(MarketMap::from(market_map)))
            })
            .wrap_err("invalid market map bytes")
    }

    #[instrument(skip_all)]
    async fn get_market_map_last_updated_height(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(keys::LAST_UPDATED)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading market map last updated height from state")?
        else {
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::BlockHeight::try_from(value).map(u64::from))
            .wrap_err("invalid updated height bytes")
    }

    #[instrument(skip_all)]
    async fn get_params(&self) -> Result<Option<Params>> {
        let Some(bytes) = self
            .get_raw(keys::PARAMS)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get params from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::Params::try_from(value).map(|params| Some(Params::from(params)))
            })
            .wrap_err("invalid params bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_market_map(&mut self, market_map: MarketMap) -> Result<()> {
        let bytes = StoredValue::from(storage::MarketMap::from(&market_map))
            .serialize()
            .wrap_err("failed to serialize market map")?;
        self.put_raw(keys::MARKET_MAP.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_market_map_last_updated_height(&mut self, height: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::BlockHeight::from(height))
            .serialize()
            .wrap_err("failed to serialize last updated height")?;
        self.put_raw(keys::LAST_UPDATED.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_params(&mut self, params: Params) -> Result<()> {
        let bytes = StoredValue::from(storage::Params::from(&params))
            .serialize()
            .wrap_err("failed to serialize params")?;
        self.put_raw(keys::PARAMS.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
