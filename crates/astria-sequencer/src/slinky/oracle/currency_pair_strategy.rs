use astria_core::slinky::types::v1::CurrencyPair;
use astria_eyre::eyre::{
    ensure,
    Result,
    WrapErr as _,
};

use crate::slinky::oracle::state_ext::StateReadExt;

/// see <https://github.com/skip-mev/slinky/blob/793b2e874d6e720bd288e82e782502e41cf06f8c/abci/strategies/currencypair/default.go>
pub(crate) struct DefaultCurrencyPairStrategy;

impl DefaultCurrencyPairStrategy {
    pub(crate) async fn id<S: StateReadExt>(
        state: &S,
        currency_pair: &CurrencyPair,
    ) -> Result<u64> {
        state.get_currency_pair_id(currency_pair).await
    }

    pub(crate) async fn from_id<S: StateReadExt>(
        state: &S,
        id: u64,
    ) -> Result<Option<CurrencyPair>> {
        state.get_currency_pair(id).await
    }

    pub(crate) fn get_encoded_price<S: StateReadExt>(_state: &S, price: u128) -> Vec<u8> {
        price.to_be_bytes().to_vec()
    }

    pub(crate) fn get_decoded_price<S: StateReadExt>(
        _state: &S,
        encoded_price: &[u8],
    ) -> Result<u128> {
        ensure!(encoded_price.len() == 16, "invalid encoded price length");
        let mut bytes = [0; 16];
        bytes.copy_from_slice(encoded_price);
        Ok(u128::from_be_bytes(bytes))
    }

    pub(crate) async fn get_max_num_currency_pairs<S: StateReadExt>(
        state: &S,
        is_proposal_phase: bool,
    ) -> Result<u64> {
        let current = state
            .get_num_currency_pairs()
            .await
            .wrap_err("failed to get number of currency pairs")?;

        if is_proposal_phase {
            let removed = state
                .get_num_removed_currency_pairs()
                .await
                .wrap_err("failed to get number of removed currency pairs")?;
            Ok(current.saturating_add(removed))
        } else {
            Ok(current)
        }
    }
}
