use astria_core::oracles::price_feed::types::v2::{
    Base,
    CurrencyPair,
    CurrencyPairId,
    Quote,
};
use astria_eyre::eyre::{
    eyre,
    Result,
};

pub(in crate::oracles::price_feed::oracle) const CURRENCY_PAIR_TO_ID_PREFIX: &str =
    "price_feed/oracle/currency_pair_to_id/";
const ID_TO_CURRENCY_PAIR_PREFIX: &str = "price_feed/oracle/id_to_currency_pair/";
pub(in crate::oracles::price_feed::oracle) const CURRENCY_PAIR_STATE_PREFIX: &str =
    "price_feed/oracle/currency_pair_state/";

pub(in crate::oracles::price_feed::oracle) const NUM_CURRENCY_PAIRS: &str =
    "price_feed/oracle/num_currency_pairs";
pub(in crate::oracles::price_feed::oracle) const NEXT_CURRENCY_PAIR_ID: &str =
    "price_feed/oracle/next_currency_pair_id";

pub(in crate::oracles::price_feed::oracle) fn currency_pair_to_id(
    currency_pair: &CurrencyPair,
) -> String {
    format!(
        "{CURRENCY_PAIR_TO_ID_PREFIX}{}/{}",
        currency_pair.base(),
        currency_pair.quote()
    )
}

pub(in crate::oracles::price_feed::oracle) fn id_to_currency_pair(id: CurrencyPairId) -> String {
    format!("{ID_TO_CURRENCY_PAIR_PREFIX}{id}")
}

pub(in crate::oracles::price_feed::oracle) fn currency_pair_state(
    currency_pair: &CurrencyPair,
) -> String {
    format!("{CURRENCY_PAIR_STATE_PREFIX}{currency_pair}")
}

pub(in crate::oracles::price_feed::oracle) fn extract_currency_pair_from_pair_to_id_key(
    key: &str,
) -> Result<CurrencyPair> {
    extract_currency_pair_from_key(CURRENCY_PAIR_TO_ID_PREFIX, key)
}

pub(in crate::oracles::price_feed::oracle) fn extract_currency_pair_from_pair_state_key(
    key: &str,
) -> Result<CurrencyPair> {
    extract_currency_pair_from_key(CURRENCY_PAIR_STATE_PREFIX, key)
}

fn extract_currency_pair_from_key(prefix: &str, key: &str) -> Result<CurrencyPair> {
    let currency_pair_str = key
        .strip_prefix(prefix)
        .ok_or_else(|| eyre!("key `{key}` did not have prefix `{prefix}`"))?;
    let (base_str, quote_str) = currency_pair_str.split_once('/').ok_or_else(|| {
        eyre!("suffix `{currency_pair_str}` of key `{key}` is not of form <base>/<quote>")
    })?;
    Ok(CurrencyPair::from_parts(
        Base::unchecked_from_parts(base_str.to_string()),
        Quote::unchecked_from_parts(quote_str.to_string()),
    ))
}

#[cfg(test)]
mod tests {
    use astria_core::oracles::price_feed::types::v2::{
        Base,
        Quote,
    };

    use super::*;

    const COMPONENT_PREFIX: &str = "price_feed/oracle/";

    fn currency_pair() -> CurrencyPair {
        CurrencyPair::from_parts(
            Base::unchecked_from_parts("abc".to_string()),
            Quote::unchecked_from_parts("def".to_string()),
        )
    }

    #[test]
    fn keys_should_not_change() {
        insta::assert_snapshot!("num_currency_pairs_key", NUM_CURRENCY_PAIRS);
        insta::assert_snapshot!("next_currency_pair_id_key", NEXT_CURRENCY_PAIR_ID);
        insta::assert_snapshot!(
            "currency_pair_to_id_key",
            currency_pair_to_id(&currency_pair())
        );
        insta::assert_snapshot!(
            "id_to_currency_pair_key",
            id_to_currency_pair(CurrencyPairId::new(9))
        );
        insta::assert_snapshot!(
            "currency_pair_state_key",
            currency_pair_state(&currency_pair())
        );
    }

    #[test]
    fn keys_should_have_component_prefix() {
        assert!(NUM_CURRENCY_PAIRS.starts_with(COMPONENT_PREFIX));
        assert!(NEXT_CURRENCY_PAIR_ID.starts_with(COMPONENT_PREFIX));
        assert!(currency_pair_to_id(&currency_pair()).starts_with(COMPONENT_PREFIX));
        assert!(id_to_currency_pair(CurrencyPairId::new(9)).starts_with(COMPONENT_PREFIX));
        assert!(currency_pair_state(&currency_pair()).starts_with(COMPONENT_PREFIX));
    }

    #[test]
    fn should_extract_currency_pair_from_pair_to_id_key() {
        let currency_pair = currency_pair();
        let key = currency_pair_to_id(&currency_pair);
        let recovered_currency_pair = extract_currency_pair_from_pair_to_id_key(&key).unwrap();
        assert_eq!(currency_pair, recovered_currency_pair);
    }

    #[test]
    fn should_extract_currency_pair_from_pair_state_key() {
        let currency_pair = currency_pair();
        let key = currency_pair_state(&currency_pair);
        let recovered_currency_pair = extract_currency_pair_from_pair_state_key(&key).unwrap();
        assert_eq!(currency_pair, recovered_currency_pair);
    }
}
