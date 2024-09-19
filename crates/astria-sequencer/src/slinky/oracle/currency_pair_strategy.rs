use anyhow::Context as _;
use astria_core::slinky::types::v1::{
    CurrencyPair,
    CurrencyPairId,
};

use crate::slinky::oracle::state_ext::StateReadExt;

/// see <https://github.com/skip-mev/slinky/blob/793b2e874d6e720bd288e82e782502e41cf06f8c/abci/strategies/currencypair/default.go>
pub(crate) struct DefaultCurrencyPairStrategy;

impl DefaultCurrencyPairStrategy {
    pub(crate) async fn id<S: StateReadExt>(
        state: &S,
        currency_pair: &CurrencyPair,
    ) -> anyhow::Result<Option<CurrencyPairId>> {
        state.get_currency_pair_id(currency_pair).await
    }

    pub(crate) async fn from_id<S: StateReadExt>(
        state: &S,
        id: CurrencyPairId,
    ) -> anyhow::Result<Option<CurrencyPair>> {
        state.get_currency_pair(id).await
    }

    pub(crate) async fn get_max_num_currency_pairs<S: StateReadExt>(
        state: &S,
        is_proposal_phase: bool,
    ) -> anyhow::Result<u64> {
        let current = state
            .get_num_currency_pairs()
            .await
            .context("failed to get number of currency pairs")?;

        if is_proposal_phase {
            let removed = state
                .get_num_removed_currency_pairs()
                .await
                .context("failed to get number of removed currency pairs")?;
            Ok(current.saturating_add(removed))
        } else {
            Ok(current)
        }
    }
}
