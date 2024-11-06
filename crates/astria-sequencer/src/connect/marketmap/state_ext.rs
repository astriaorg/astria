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
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::instrument;

const MARKET_MAP_KEY: &str = "connectmarketmap";
const PARAMS_KEY: &str = "connectparams";
const MARKET_MAP_LAST_UPDATED_KEY: &str = "connectmarketmaplastupdated";

/// Newtype wrapper to read and write a u64 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Height(u64);

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_market_map(&self) -> Result<Option<MarketMap>> {
        let bytes = self
            .get_raw(MARKET_MAP_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get market map from state")?;
        match bytes {
            Some(bytes) => {
                let market_map =
                    serde_json::from_slice(&bytes).wrap_err("failed to deserialize market map")?;
                Ok(Some(market_map))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip_all)]
    async fn get_market_map_last_updated_height(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(MARKET_MAP_LAST_UPDATED_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading market map last updated height from state")?
        else {
            return Ok(0);
        };
        let Height(height) = Height::try_from_slice(&bytes).wrap_err("invalid height bytes")?;
        Ok(height)
    }

    #[instrument(skip_all)]
    async fn get_params(&self) -> Result<Option<Params>> {
        let bytes = self
            .get_raw(PARAMS_KEY)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get params from state")?;
        match bytes {
            Some(bytes) => {
                let params =
                    serde_json::from_slice(&bytes).wrap_err("failed to deserialize params")?;
                Ok(Some(params))
            }
            None => Ok(None),
        }
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_market_map(&mut self, market_map: MarketMap) -> Result<()> {
        let bytes = serde_json::to_vec(&market_map).wrap_err("failed to serialize market map")?;
        self.put_raw(MARKET_MAP_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_market_map_last_updated_height(&mut self, height: u64) -> Result<()> {
        self.put_raw(
            MARKET_MAP_LAST_UPDATED_KEY.to_string(),
            borsh::to_vec(&Height(height)).wrap_err("failed to serialize height")?,
        );
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_params(&mut self, params: Params) -> Result<()> {
        let bytes = serde_json::to_vec(&params).wrap_err("failed to serialize params")?;
        self.put_raw(PARAMS_KEY.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
