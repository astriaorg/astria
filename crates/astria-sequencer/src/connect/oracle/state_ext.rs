use std::{
    pin::Pin,
    task::{
        ready,
        Context,
        Poll,
    },
};

use astria_core::connect::{
    oracle::v2::{
        CurrencyPairState,
        QuotePrice,
    },
    types::v2::{
        CurrencyPair,
        CurrencyPairId,
        CurrencyPairNonce,
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

        let currency_pair = match keys::extract_currency_pair_from_key(&key) {
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
        let currency_pair = match keys::extract_currency_pair_from_key(&key) {
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
    async fn get_num_removed_currency_pairs(&self) -> Result<u64> {
        let Some(bytes) = self
            .get_raw(keys::NUM_REMOVED_CURRENCY_PAIRS)
            .await
            .map_err(anyhow_to_eyre)
            .wrap_err("failed reading number of removed currency pairs from state")?
        else {
            return Ok(0);
        };
        StoredValue::deserialize(&bytes)
            .and_then(|value| storage::Count::try_from(value).map(u64::from))
            .wrap_err("invalid number of removed currency pairs bytes")
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
    fn put_num_removed_currency_pairs(&mut self, num_removed_currency_pairs: u64) -> Result<()> {
        let bytes = StoredValue::from(storage::Count::from(num_removed_currency_pairs))
            .serialize()
            .wrap_err("failed to serialize number of removed currency pairs")?;
        self.put_raw(keys::NUM_REMOVED_CURRENCY_PAIRS.to_string(), bytes);
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
        let state = if let Some(mut state) = self
            .get_currency_pair_state(&currency_pair)
            .await
            .wrap_err("failed to get currency pair state")?
        {
            state.price = price;
            state.nonce = state
                .nonce
                .increment()
                .ok_or_eyre("increment nonce overflowed")?;
            state
        } else {
            let id = get_next_currency_pair_id(self)
                .await
                .wrap_err("failed to read next currency pair ID")?;
            CurrencyPairState {
                price,
                nonce: CurrencyPairNonce::new(0),
                id,
            }
        };
        self.put_currency_pair_state(currency_pair, state)
            .wrap_err("failed to put currency pair state")
    }
}

impl<T: StateWrite> StateWriteExt for T {}

#[instrument(skip_all)]
async fn get_next_currency_pair_id<T: StateRead + ?Sized>(state: &T) -> Result<CurrencyPairId> {
    let Some(bytes) = state
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
