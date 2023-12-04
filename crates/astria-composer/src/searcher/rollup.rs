//! Parsing strings of the form `<rollup_name>::<url>`

use std::fmt;

use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub(super) struct Rollup {
    rollup_name: String,
    url: String,
}

#[derive(Debug)]
pub(super) struct ParseError {}

impl ParseError {
    fn new() -> Self {
        Self {}
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(
            "invalid rollup definition, must be `<rollup_name>::<url>, with \
            <rollup_name> being lowercase alphanumeric ascii and -",
        )
    }
}

impl std::error::Error for ParseError {}

impl Rollup {
    pub(super) fn parse(from: &str) -> Result<Self, ParseError> {
        static ROLLUP_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r"(?x)
                ^(?P<rollup_name>[[:alnum:]-]+?)
                    # lazily match all alphanumeric ascii and dash;
                    # case insignificant, but we will lowercase later
                ::
                (?P<url>.+)
                    # treat all following chars as the url without any verification;
                    # if there are bad chars, the downstream URL parser should
                    # handle that
                $
            ",
            )
            .unwrap()
        });
        let caps = ROLLUP_RE.captures(from).ok_or_else(ParseError::new)?;
        // Note that this will panic on invalid indices. However, these
        // accesses will always be correct because the regex will only
        // match when these capture groups match.
        let rollup_name = caps["rollup_name"].to_string().to_lowercase();
        let url = caps["url"].to_string();
        Ok(Self {
            rollup_name,
            url,
        })
    }

    pub(super) fn into_parts(self) -> (String, String) {
        let Self {
            rollup_name: chain_id,
            url,
        } = self;
        (chain_id, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_parse_rollups(rollups: impl AsRef<str>) -> Vec<Rollup> {
        rollups
            .as_ref()
            .split(',')
            .map(|s| {
                Rollup::parse(s)
                    .unwrap_or_else(|err| panic!("rollup '{s:?}' should parse: {err:?}"))
            })
            .collect()
    }

    #[test]
    fn parse_single_rollup_valid() {
        let rollups = expect_parse_rollups("chain-1::http://some.url");
        assert_eq!(rollups.len(), 1, "\nparsed: {rollups:#?}");
        assert_eq!(rollups[0].rollup_name, "chain-1");
        assert_eq!(rollups[0].url, "http://some.url");
    }

    #[test]
    fn parse_mixed_case_rollup_name_is_lowercased() {
        let rollups = expect_parse_rollups("ChAiN-1::http://some.url");
        assert_eq!(rollups.len(), 1, "\nparsed: {rollups:#?}");
        assert_eq!(rollups[0].rollup_name, "chain-1");
        assert_eq!(rollups[0].url, "http://some.url");
    }

    #[test]
    fn parse_three_rollups_valid() {
        let rollups =
            expect_parse_rollups("chain-1::http://some.url,another::ws://ws.domain,last::foo.bar");
        assert_eq!(rollups.len(), 3, "\nparsed: {rollups:#?}");
        assert_eq!(rollups[0].rollup_name, "chain-1");
        assert_eq!(rollups[0].url, "http://some.url");
        assert_eq!(rollups[1].rollup_name, "another");
        assert_eq!(rollups[1].url, "ws://ws.domain");
        assert_eq!(rollups[2].rollup_name, "last");
        assert_eq!(rollups[2].url, "foo.bar");
    }

    #[should_panic]
    #[test]
    fn parse_with_non_alnum_non_dash_rollup_name_fails() {
        expect_parse_rollups("chain_1::http://some.url");
    }

    #[should_panic]
    #[test]
    fn parse_without_double_colon_fails() {
        expect_parse_rollups("chain-1:http://some.url");
    }

    #[test]
    fn parse_with_triple_colon_is_valid() {
        let rollups = expect_parse_rollups("chain-1:::http://some.url");
        assert_eq!(rollups[0].rollup_name, "chain-1");
        assert_eq!(rollups[0].url, ":http://some.url");
    }
}
