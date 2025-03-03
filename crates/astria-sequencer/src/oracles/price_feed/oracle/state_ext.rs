use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::oracles::price_feed::{
    oracle::v2::{
        CurrencyPairState,
        QuotePrice,
    },
    types::v2::{
        CurrencyPair,
        CurrencyPairId,
    },
};
use astria_eyre::{
    anyhow_to_eyre,
    eyre::{
        OptionExt as _,
        Result,
        WrapErr as _,
    },
};
use async_trait::async_trait;
use cnidarium::{
    StateRead,
    StateWrite,
};
use futures::Stream;
use pin_project_lite::pin_project;
use tracing::instrument;

use super::storage::{
    self,
    keys,
};
use crate::storage::StoredValue;

pin_project! {
    pub(crate) struct CurrencyPairsWithIdsStream<St> {
        #[pin]
        underlying: St,
    }
}

#[derive(PartialEq)]
pub(crate) struct CurrencyPairWithId {
    pub(crate) id: u64,
    pub(crate) currency_pair: CurrencyPair,
}

impl<St> Stream for CurrencyPairsWithIdsStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<(String, Vec<u8>)>>,
{
    type Item = Result<CurrencyPairWithId>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let (key, bytes) = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(item)) => item,
            Some(Err(err)) => {
                return Poll::Ready(Some(
                    Err(anyhow_to_eyre(err)).wrap_err("failed reading from state"),
                ));
            }
            None => return Poll::Ready(None),
        };
        let id = StoredValue::deserialize(&bytes)
            .and_then(|value| storage::CurrencyPairId::try_from(value).map(CurrencyPairId::from))
            .wrap_err_with(|| format!("invalid currency pair id bytes under key `{key}`"))?;

        let currency_pair = match keys::extract_currency_pair_from_pair_to_id_key(&key) {
            Err(err) => {
                return Poll::Ready(Some(Err(err).with_context(|| {
                    format!("failed to extract currency pair from key `{key}`")
                })));
            }
            Ok(parsed) => parsed,
        };
        Poll::Ready(Some(Ok(CurrencyPairWithId {
            id: id.get(),
            currency_pair,
        })))
    }
}

pin_project! {
    pub(crate) struct CurrencyPairsStream<St> {
        #[pin]
        underlying: St,
    }
}

impl<St> Stream for CurrencyPairsStream<St>
where
    St: Stream<Item = astria_eyre::anyhow::Result<String>>,
{
    type Item = Result<CurrencyPair>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let key = match ready!(this.underlying.as_mut().poll_next(cx)) {
            Some(Ok(item)) => item,
            Some(Err(err)) => {
                return Poll::Ready(Some(
                    Err(anyhow_to_eyre(err)).wrap_err("failed reading from state"),
                ));
            }
            None => return Poll::Ready(None),
        };
        let currency_pair = match keys::extract_currency_pair_from_pair_state_key(&key) {
            Err(err) => {
                return Poll::Ready(Some(Err(err).with_context(|| {
                    format!("failed to extract currency pair from key `{key}`")
                })));
            }
            Ok(parsed) => parsed,
        };
        Poll::Ready(Some(Ok(currency_pair)))
    }
}

#[async_trait]
pub(crate) trait StateReadExt: StateRead {
    #[instrument(skip_all)]
    async fn get_currency_pair_id(
        &self,
        currency_pair: &CurrencyPair,
    ) -> Result<Option<CurrencyPairId>> {
        let Some(bytes) = self
            .get_raw(&keys::currency_pair_to_id(currency_pair))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading currency pair id from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::CurrencyPairId::try_from(value).map(|id| Some(CurrencyPairId::from(id)))
            })
            .wrap_err("invalid currency pair id bytes")
    }

    #[instrument(skip_all)]
    async fn get_currency_pair(&self, id: CurrencyPairId) -> Result<Option<CurrencyPair>> {
        let Some(bytes) = self
            .get_raw(&keys::id_to_currency_pair(id))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading currency pair from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::CurrencyPair::try_from(value).map(|pair| Some(CurrencyPair::from(pair)))
            })
            .wrap_err("invalid currency pair bytes")
    }

    #[instrument(skip_all)]
    fn currency_pairs_with_ids(&self) -> CurrencyPairsWithIdsStream<Self::PrefixRawStream> {
        CurrencyPairsWithIdsStream {
            underlying: self.prefix_raw(keys::CURRENCY_PAIR_TO_ID_PREFIX),
        }
    }

    #[instrument(skip_all)]
    fn currency_pairs(&self) -> CurrencyPairsStream<Self::PrefixKeysStream> {
        CurrencyPairsStream {
            underlying: self.prefix_keys(keys::CURRENCY_PAIR_STATE_PREFIX),
        }
    }

    #[instrument(skip_all)]
    async fn get_num_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(keys::NUM_CURRENCY_PAIRS)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading number of currency pairs from state")?
        else {
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Count::try_from(value).map(u64::from))
            .wrap_err("invalid number of currency pairs bytes")
    }

    #[instrument(skip_all)]
    async fn get_currency_pair_state(
        &self,
        currency_pair: &CurrencyPair,
    ) -> Result<Option<CurrencyPairState>> {
        let Some(bytes) = self
            .get_raw(&keys::currency_pair_state(currency_pair))
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed to get currency pair state from state")?
        else {
            return Ok(None);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| {
                storage::CurrencyPairState::try_from(value)
                    .map(|state| Some(CurrencyPairState::from(state)))
            })
            .wrap_err("invalid currency pair state bytes")
    }

    #[instrument(skip_all)]
    async fn get_next_currency_pair_id(&self) -> Result<CurrencyPairId> {
        let Some(bytes) = self
            .get_raw(keys::NEXT_CURRENCY_PAIR_ID)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading next currency pair id from state")?
        else {
            return Ok(CurrencyPairId::new(0));
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::CurrencyPairId::try_from(value).map(CurrencyPairId::from))
            .wrap_err("invalid next currency pair id bytes")
    }
}

impl<T: StateRead + ?Sized> StateReadExt for T {}

#[async_trait]
pub(crate) trait StateWriteExt: StateWrite {
    #[instrument(skip_all)]
    fn put_num_currency_pairs(&mut self, num_currency_pairs: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::Count::from(num_currency_pairs))
            .serialize()
            .wrap_err("failed to serialize number of currency pairs")?;
        self.put_raw(keys::NUM_CURRENCY_PAIRS.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    fn put_currency_pair_state(
        &mut self,
        currency_pair: CurrencyPair,
        currency_pair_state: CurrencyPairState,
    ) -> Result<()> {
        let currency_pair_id = currency_pair_state.id;
        let bytes = StoredValue::from(storage::CurrencyPairState::from(currency_pair_state))
            .serialize()
            .wrap_err("failed to serialize currency pair state")?;
        self.put_raw(keys::currency_pair_state(&currency_pair), bytes);

        put_currency_pair_id(self, &currency_pair, currency_pair_id)
            .wrap_err("failed to put currency pair id")?;
        put_currency_pair(self, currency_pair_id, currency_pair)
            .wrap_err("failed to put currency pair")
    }

    #[instrument(skip_all)]
    fn put_next_currency_pair_id(&mut self, next_currency_pair_id: CurrencyPairId) -> Result<()> {
        let bytes = StoredValue::from(storage::CurrencyPairId::from(next_currency_pair_id))
            .serialize()
            .wrap_err("failed to serialize next currency pair id")?;
        self.put_raw(keys::NEXT_CURRENCY_PAIR_ID.to_string(), bytes);
        Ok(())
    }

    #[instrument(skip_all)]
    async fn put_price_for_currency_pair(
        &mut self,
        currency_pair: CurrencyPair,
        price: QuotePrice,
    ) -> Result<()> {
        let mut state = self
            .get_currency_pair_state(&currency_pair)
            .await
            .wrap_err("failed to get currency pair state")?
            .ok_or_eyre("currency pair state not found")?;

        state.price = Some(price);
        state.nonce = state
            .nonce
            .increment()
            .ok_or_eyre("increment nonce overflowed")?;
        self.put_currency_pair_state(currency_pair, state)
            .wrap_err("failed to put currency pair state")
    }

    #[instrument(skip_all)]
    async fn remove_currency_pair(&mut self, currency_pair: &CurrencyPair) -> Result<bool> {
        let Some(id) = self
            .get_currency_pair_id(currency_pair)
            .await
            .wrap_err("failed to get currency pair ID")?
        else {
            return Ok(false);
        };
        self.delete(keys::currency_pair_to_id(currency_pair));
        self.delete(keys::id_to_currency_pair(id));
        self.delete(keys::currency_pair_state(currency_pair));
        Ok(true)
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[instrument(skip_all)]
fn put_currency_pair_id<T: StateWrite + ?Sized>(
    state: &mut T,
    currency_pair: &CurrencyPair,
    id: CurrencyPairId,
) -> Result<()> {
    let bytes = StoredValue::from(storage::CurrencyPairId::from(id))
        .serialize()
        .wrap_err("failed to serialize currency pair id")?;
    state.put_raw(keys::currency_pair_to_id(currency_pair), bytes);
    Ok(())
}

#[instrument(skip_all)]
fn put_currency_pair<T: StateWrite + ?Sized>(
    state: &mut T,
    id: CurrencyPairId,
    currency_pair: CurrencyPair,
) -> Result<()> {
    let bytes = StoredValue::from(storage::CurrencyPair::from(&currency_pair))
        .serialize()
        .wrap_err("failed to serialize currency pair")?;
    state.put_raw(keys::id_to_currency_pair(id), bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use astria_core::{
        oracles::price_feed::types::v2::{
            CurrencyPair,
            CurrencyPairNonce,
            Price,
        },
        Timestamp,
    };
    use cnidarium::StateDelta;
    use futures::TryStreamExt;

    use super::*;

    fn eth_usd() -> CurrencyPair {
        "ETH/USD".parse::<CurrencyPair>().unwrap()
    }

    fn eth_usd_state(nonce: u64) -> CurrencyPairState {
        currency_pair_state(1, nonce)
    }

    fn btc_usd() -> CurrencyPair {
        "BTC/USD".parse::<CurrencyPair>().unwrap()
    }

    fn btc_usd_state(nonce: u64) -> CurrencyPairState {
        currency_pair_state(2, nonce)
    }

    fn currency_pair_state(id: u64, nonce: u64) -> CurrencyPairState {
        CurrencyPairState {
            price: Some(QuotePrice {
                price: Price::new(123),
                block_timestamp: Timestamp {
                    seconds: 4,
                    nanos: 5,
                },
                block_height: nonce.checked_add(10).unwrap(),
            }),
            nonce: CurrencyPairNonce::new(nonce),
            id: CurrencyPairId::new(id),
        }
    }

    /// Putting the currency pair state also writes the pair and the pair ID, so we'll also check
    /// those getters in this test.
    #[tokio::test]
    async fn should_put_and_get_currency_pair_state() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Getting should return `None` when the pair is not stored.
        assert!(state
            .get_currency_pair_state(&eth_usd())
            .await
            .unwrap()
            .is_none());
        assert!(state
            .get_currency_pair_id(&eth_usd())
            .await
            .unwrap()
            .is_none());
        assert!(state
            .get_currency_pair(CurrencyPairId::new(1))
            .await
            .unwrap()
            .is_none());

        // Putting a currency pair state should succeed.
        state
            .put_currency_pair_state(eth_usd(), eth_usd_state(1))
            .unwrap();

        // Getting the stored state, pair and id should succeed.
        let retrieved_pair_state = state
            .get_currency_pair_state(&eth_usd())
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(eth_usd_state(1), retrieved_pair_state);
        let retrieved_pair = state
            .get_currency_pair(eth_usd_state(1).id)
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(eth_usd(), retrieved_pair);
        let retrieved_pair_id = state
            .get_currency_pair_id(&eth_usd())
            .await
            .expect("should not error")
            .expect("should be `Some`");
        assert_eq!(eth_usd_state(1).id, retrieved_pair_id);
    }

    #[tokio::test]
    async fn should_get_currency_pairs_with_ids() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Should return an empty stream before any currency pair states are stored.
        let collected: Vec<_> = state.currency_pairs_with_ids().try_collect().await.unwrap();
        assert!(collected.is_empty());

        // Store some currency pair states.
        state
            .put_currency_pair_state(eth_usd(), eth_usd_state(2))
            .unwrap();
        state
            .put_currency_pair_state(btc_usd(), btc_usd_state(1))
            .unwrap();
        state
            .put_currency_pair_state(btc_usd(), btc_usd_state(2))
            .unwrap();
        state
            .put_currency_pair_state(eth_usd(), eth_usd_state(1))
            .unwrap();

        // Check we retrieved all expected currency pairs with ids.
        let collected: Vec<_> = state.currency_pairs_with_ids().try_collect().await.unwrap();
        assert_eq!(collected.len(), 2);
        assert!(collected.contains(&CurrencyPairWithId {
            id: eth_usd_state(1).id.get(),
            currency_pair: eth_usd(),
        }));
        assert!(collected.contains(&CurrencyPairWithId {
            id: btc_usd_state(1).id.get(),
            currency_pair: btc_usd(),
        }));
    }

    #[tokio::test]
    async fn should_get_currency_pairs() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Should return an empty stream before any currency pair states are stored.
        let collected: Vec<_> = state.currency_pairs().try_collect().await.unwrap();
        assert!(collected.is_empty());

        // Store some currency pair states.
        state
            .put_currency_pair_state(eth_usd(), eth_usd_state(2))
            .unwrap();
        state
            .put_currency_pair_state(btc_usd(), btc_usd_state(1))
            .unwrap();
        state
            .put_currency_pair_state(btc_usd(), btc_usd_state(2))
            .unwrap();
        state
            .put_currency_pair_state(eth_usd(), eth_usd_state(1))
            .unwrap();

        // Check we retrieved all expected currency pairs.
        let collected: Vec<_> = state.currency_pairs().try_collect().await.unwrap();
        assert_eq!(collected.len(), 2);
        assert!(collected.contains(&eth_usd()));
        assert!(collected.contains(&btc_usd()));
    }

    #[tokio::test]
    async fn should_put_and_get_num_currency_pairs() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        // Getting should return `0` when no count is stored.
        assert_eq!(state.get_num_currency_pairs().await.unwrap(), 0);

        // Putting a count should succeed.
        state.put_num_currency_pairs(1).unwrap();

        // Getting the stored count should succeed.
        let retrieved_count = state
            .get_num_currency_pairs()
            .await
            .expect("should not error");
        assert_eq!(1, retrieved_count);

        // Putting a new count should overwrite the first.
        state.put_num_currency_pairs(2).unwrap();

        let retrieved_count = state
            .get_num_currency_pairs()
            .await
            .expect("should not error");
        assert_eq!(2, retrieved_count);
    }
}
