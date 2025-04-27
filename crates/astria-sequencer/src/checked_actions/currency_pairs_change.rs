use astria_core::{
    oracles::price_feed::{
        oracle::v2::CurrencyPairState,
        types::v2::{
            CurrencyPair,
            CurrencyPairNonce,
        },
    },
    primitive::v1::{
        asset::IbcPrefixed,
        ADDRESS_LEN,
    },
    protocol::transaction::v1::action::CurrencyPairsChange,
};
use astria_eyre::eyre::{
    ensure,
    OptionExt as _,
    Result,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    StateWrite,
};
use tracing::{
    debug,
    instrument,
    Level,
};

use super::{
    AssetTransfer,
    TransactionSignerAddressBytes,
};
use crate::{
    authority::StateReadExt as _,
    oracles::price_feed::oracle::state_ext::{
        StateReadExt as _,
        StateWriteExt as _,
    },
};

#[derive(Debug)]
pub(crate) struct CheckedCurrencyPairsChange {
    action: CurrencyPairsChange,
    tx_signer: TransactionSignerAddressBytes,
}

impl CheckedCurrencyPairsChange {
    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn new<S: StateRead>(
        action: CurrencyPairsChange,
        tx_signer: [u8; ADDRESS_LEN],
        state: S,
    ) -> Result<Self> {
        let checked_action = Self {
            action,
            tx_signer: tx_signer.into(),
        };
        checked_action.run_mutable_checks(state).await?;

        Ok(checked_action)
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn run_mutable_checks<S: StateRead>(&self, state: S) -> Result<()> {
        // Ensure the tx signer is the current sudo address.
        let sudo_address = state
            .get_sudo_address()
            .await
            .wrap_err("failed to read sudo address from storage")?;
        ensure!(
            &sudo_address == self.tx_signer.as_bytes(),
            "transaction signer not authorized to change currency pairs",
        );
        Ok(())
    }

    #[instrument(skip_all, err(level = Level::DEBUG))]
    pub(super) async fn execute<S: StateWrite>(&self, state: S) -> Result<()> {
        self.run_mutable_checks(&state).await?;

        match &self.action {
            CurrencyPairsChange::Addition(currency_pairs) => {
                execute_currency_pairs_addition(state, currency_pairs).await
            }
            CurrencyPairsChange::Removal(currency_pairs) => {
                execute_currency_pairs_removal(state, currency_pairs).await
            }
        }
    }

    pub(super) fn action(&self) -> &CurrencyPairsChange {
        &self.action
    }
}

impl AssetTransfer for CheckedCurrencyPairsChange {
    fn transfer_asset_and_amount(&self) -> Option<(IbcPrefixed, u128)> {
        None
    }
}

async fn execute_currency_pairs_addition<S: StateWrite>(
    mut state: S,
    currency_pairs: &[CurrencyPair],
) -> Result<()> {
    let mut next_currency_pair_id = state
        .get_next_currency_pair_id()
        .await
        .wrap_err("failed to read next currency pair id from storage")?;
    let mut num_currency_pairs = state
        .get_num_currency_pairs()
        .await
        .wrap_err("failed to read number of currency pairs from storage")?;

    for pair in currency_pairs {
        if state
            .get_currency_pair_state(pair)
            .await
            .wrap_err("failed to read currency pair state from storage")?
            .is_some()
        {
            debug!(%pair, "currency pair already exists, skipping");
            continue;
        }

        let currency_pair_state = CurrencyPairState {
            price: None,
            nonce: CurrencyPairNonce::new(0),
            id: next_currency_pair_id,
        };
        state
            .put_currency_pair_state(pair.clone(), currency_pair_state)
            .wrap_err("failed to write currency pair state to storage")?;
        num_currency_pairs = num_currency_pairs
            .checked_add(1)
            .ok_or_eyre("overflow when incrementing number of currency pairs")?;
        next_currency_pair_id = next_currency_pair_id
            .increment()
            .ok_or_eyre("overflow when incrementing next currency pair id")?;
    }

    state
        .put_next_currency_pair_id(next_currency_pair_id)
        .wrap_err("failed to write next currency pair id to storage")?;
    state
        .put_num_currency_pairs(num_currency_pairs)
        .wrap_err("failed to write number of currency pairs to storage")
}

async fn execute_currency_pairs_removal<S: StateWrite>(
    mut state: S,
    currency_pairs: &[CurrencyPair],
) -> Result<()> {
    let mut num_currency_pairs = state
        .get_num_currency_pairs()
        .await
        .wrap_err("failed to read number of currency pairs from storage")?;

    for pair in currency_pairs {
        if state
            .remove_currency_pair(pair)
            .await
            .wrap_err("failed to delete currency pair from storage")?
        {
            num_currency_pairs = num_currency_pairs
                .checked_sub(1)
                .ok_or_eyre("failed to decrement number of currency pairs")?;
        }
    }

    state
        .put_num_currency_pairs(num_currency_pairs)
        .wrap_err("failed to write number of currency pairs to storage")
}

#[cfg(test)]
mod tests {
    use astria_core::{
        oracles::price_feed::types::v2::CurrencyPairId,
        protocol::transaction::v1::action::SudoAddressChange,
    };

    use super::*;
    use crate::{
        checked_actions::CheckedSudoAddressChange,
        test_utils::{
            assert_error_contains,
            astria_address,
            Fixture,
            SUDO_ADDRESS_BYTES,
        },
    };

    fn new_addition<'a, I: IntoIterator<Item = &'a str>>(pairs: I) -> CurrencyPairsChange {
        CurrencyPairsChange::Addition(pairs.into_iter().map(|s| s.parse().unwrap()).collect())
    }

    fn new_removal<'a, I: IntoIterator<Item = &'a str>>(pairs: I) -> CurrencyPairsChange {
        CurrencyPairsChange::Removal(pairs.into_iter().map(|s| s.parse().unwrap()).collect())
    }

    fn pairs(action: CurrencyPairsChange) -> Vec<CurrencyPair> {
        match action {
            CurrencyPairsChange::Addition(pairs) | CurrencyPairsChange::Removal(pairs) => pairs,
        }
    }

    #[tokio::test]
    async fn should_fail_construction_if_signer_is_not_sudo_address() {
        let fixture = Fixture::default_initialized().await;

        let tx_signer = [2_u8; ADDRESS_LEN];
        assert_ne!(*SUDO_ADDRESS_BYTES, tx_signer);

        let addition = new_addition(Some("BTC/USD"));
        let err = fixture
            .new_checked_action(addition, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change currency pairs",
        );

        let removal = new_removal(Some("BTC/USD"));
        let err = fixture
            .new_checked_action(removal, tx_signer)
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change currency pairs",
        );
    }

    #[tokio::test]
    async fn should_fail_execution_if_signer_is_not_sudo_address() {
        let mut fixture = Fixture::default_initialized().await;

        // Construct the addition and removal checked actions while the sudo address is still the
        // tx signer so construction succeeds.
        let addition_action = new_addition(Some("BTC/USD"));
        let checked_addition_action: CheckedCurrencyPairsChange = fixture
            .new_checked_action(addition_action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        let removal_action = new_removal(Some("BTC/USD"));
        let checked_removal_action: CheckedCurrencyPairsChange = fixture
            .new_checked_action(removal_action, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        // Change the sudo address to something other than the tx signer.
        let sudo_address_change = SudoAddressChange {
            new_address: astria_address(&[2; ADDRESS_LEN]),
        };
        let checked_sudo_address_change: CheckedSudoAddressChange = fixture
            .new_checked_action(sudo_address_change, *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();
        checked_sudo_address_change
            .execute(fixture.state_mut())
            .await
            .unwrap();
        let new_sudo_address = fixture.state().get_sudo_address().await.unwrap();
        assert_ne!(*SUDO_ADDRESS_BYTES, new_sudo_address);

        // Try to execute the two checked actions now - should fail due to signer no longer being
        // authorized.
        let err = checked_addition_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change currency pairs",
        );

        let err = checked_removal_action
            .execute(fixture.state_mut())
            .await
            .unwrap_err();
        assert_error_contains(
            &err,
            "transaction signer not authorized to change currency pairs",
        );
    }

    #[tokio::test]
    async fn should_execute_addition() {
        // `Fixture::default_initialized` executes the Aspen upgrade, which adds currency pairs
        // "BTC/USD" and "ETH/USD", so we'll use different ones for this test.
        let mut fixture = Fixture::default_initialized().await;

        // Ensure providing duplicate pairs succeeds.
        let action = new_addition(["TIA/USD", "TIA/ETH", "TIA/USD"]);
        let checked_action: CheckedCurrencyPairsChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        let pairs = pairs(action);
        assert_eq!(
            fixture
                .state()
                .get_currency_pair_state(&pairs[0])
                .await
                .unwrap()
                .unwrap(),
            CurrencyPairState {
                price: None,
                nonce: CurrencyPairNonce::new(0),
                id: CurrencyPairId::new(2),
            }
        );
        assert_eq!(
            fixture
                .state()
                .get_currency_pair_state(&pairs[1])
                .await
                .unwrap()
                .unwrap(),
            CurrencyPairState {
                price: None,
                nonce: CurrencyPairNonce::new(0),
                id: CurrencyPairId::new(3),
            }
        );
        assert_eq!(
            fixture.state().get_next_currency_pair_id().await.unwrap(),
            CurrencyPairId::new(4)
        );
        assert_eq!(fixture.state().get_num_currency_pairs().await.unwrap(), 4);
    }

    #[tokio::test]
    async fn should_execute_removal() {
        // `Fixture::default_initialized` executes the Aspen upgrade, which adds currency pairs
        // "BTC/USD" and "ETH/USD", so we'll use these for this test.
        let mut fixture = Fixture::default_initialized().await;
        assert!(fixture
            .state()
            .get_currency_pair_state(&"BTC/USD".parse::<CurrencyPair>().unwrap())
            .await
            .unwrap()
            .is_some());
        assert!(fixture
            .state()
            .get_currency_pair_state(&"ETH/USD".parse::<CurrencyPair>().unwrap())
            .await
            .unwrap()
            .is_some());

        // Ensure removing duplicate pairs succeeds, and removing a non-existent pair succeeds.
        let action = new_removal(["BTC/USD", "TIA/USD", "BTC/USD"]);
        let checked_action: CheckedCurrencyPairsChange = fixture
            .new_checked_action(action.clone(), *SUDO_ADDRESS_BYTES)
            .await
            .unwrap()
            .into();

        checked_action.execute(fixture.state_mut()).await.unwrap();

        assert!(fixture
            .state()
            .get_currency_pair_state(&"BTC/USD".parse::<CurrencyPair>().unwrap())
            .await
            .unwrap()
            .is_none());
        assert!(fixture
            .state()
            .get_currency_pair_state(&"ETH/USD".parse::<CurrencyPair>().unwrap())
            .await
            .unwrap()
            .is_some());
        assert!(fixture
            .state()
            .get_currency_pair_state(&"TIA/USD".parse::<CurrencyPair>().unwrap())
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            fixture.state().get_next_currency_pair_id().await.unwrap(),
            CurrencyPairId::new(2)
        );
        assert_eq!(fixture.state().get_num_currency_pairs().await.unwrap(), 1);
    }
}
