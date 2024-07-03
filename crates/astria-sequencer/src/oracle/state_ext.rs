use anyhow::{
    Context,
    Result,
};
use astria_core::generated::slinky::{
    marketmap::v1::{
        MarketMap,
        Params,
    },
    types::v1::CurrencyPair,
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

const CURRENCY_PAIR_TO_ID_PREFIX: &str = "oraclecpid";
const ID_TO_CURRENCY_PAIR_PREFIX: &str = "oracleidcp";

// TODO: should these values be in nonverifiable storage?
const NUM_CURRENCY_PAIRS_KEY: &str = "oraclenumcps";
const NUM_REMOVED_CURRENCY_PAIRS_KEY: &str = "oraclenumremovedcps";

fn currency_pair_to_id_storage_key(currency_pair: &CurrencyPair) -> String {
    format!(
        "{}/{}/{}",
        CURRENCY_PAIR_TO_ID_PREFIX, currency_pair.base, currency_pair.quote
    )
}

fn id_to_currency_pair_storage_key(id: u64) -> String {
    format!("{}/{}", ID_TO_CURRENCY_PAIR_PREFIX, id)
}

/// Newtype wrapper to read and write a u64 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Id(u64);

/// Newtype wrapper to read and write a u64 from rocksdb.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Count(u64);

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip(self))]
    async fn get_currency_pair_id(&self, currency_pair: &CurrencyPair) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(&currency_pair_to_id_storage_key(currency_pair))
            .await
            .context("failed reading currency pair id from state")?
        else {
            return Ok(0);
        };
        let Id(id) = Id::try_from_slice(&bytes).context("invalid currency pair id bytes")?;
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn get_currency_pair(&self, id: u64) -> Result<Option<CurrencyPair>> {
        let bytes = self
            .get_raw(&id_to_currency_pair_storage_key(id))
            .await
            .context("failed to get currency pair from state")?;
        match bytes {
            Some(bytes) => {
                let currency_pair = serde_json::from_slice(&bytes)
                    .context("failed to deserialize currency pair")?;
                Ok(Some(currency_pair))
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_num_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(NUM_CURRENCY_PAIRS_KEY)
            .await
            .context("failed reading number of currency pairs from state")?
        else {
            return Ok(0);
        };
        let Count(num_currency_pairs) =
            Count::try_from_slice(&bytes).context("invalid number of currency pairs bytes")?;
        Ok(num_currency_pairs)
    }

    #[instrument(skip(self))]
    async fn get_num_removed_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(NUM_REMOVED_CURRENCY_PAIRS_KEY)
            .await
            .context("failed reading number of removed currency pairs from state")?
        else {
            return Ok(0);
        };
        let Count(num_removed_currency_pairs) = Count::try_from_slice(&bytes)
            .context("invalid number of removed currency pairs bytes")?;
        Ok(num_removed_currency_pairs)
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip(self))]
    fn put_currency_pair_id(&mut self, currency_pair: &CurrencyPair, id: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Id(id)).context("failed to serialize currency pair id")?;
        self.put_raw(currency_pair_to_id_storage_key(currency_pair), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_currency_pair(&mut self, id: u64, currency_pair: CurrencyPair) -> Result<()> {
        let bytes =
            serde_json::to_vec(&currency_pair).context("failed to serialize currency pair")?;
        self.put_raw(id_to_currency_pair_storage_key(id), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_num_currency_pairs(&mut self, num_currency_pairs: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Count(num_currency_pairs))
            .context("failed to serialize number of currency pairs")?;
        self.put_raw(NUM_CURRENCY_PAIRS_KEY.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip(self))]
    fn put_num_removed_currency_pairs(&mut self, num_removed_currency_pairs: u64) -> Result<()> {
        let bytes = borsh::to_vec(&Count(num_removed_currency_pairs))
            .context("failed to serialize number of removed currency pairs")?;
        self.put_raw(NUM_REMOVED_CURRENCY_PAIRS_KEY.to_string(), bytes);
        Ok(())
    }
}

impl<T: StateWrite> StateWriteExt for T {}
