use std::fmt::{
    self,
    Display,
    Formatter,
    Write,
};

/// The default sequencer asset base denomination.
pub const DEFAULT_NATIVE_ASSET_DENOM: &str = "nria";

/// Returns the default sequencer asset [`Denom`].
// allow: parsing single-segment assets is unit tested
#[allow(clippy::missing_panics_doc)]
// allow: used in many places already
#[allow(clippy::module_name_repetitions)]
#[must_use]
pub fn default_native_asset() -> Denom {
    DEFAULT_NATIVE_ASSET_DENOM
        .parse::<Denom>()
        .expect("parsing a single segment string must work")
}

struct DenomFmt<'a> {
    prefix_segments: &'a [String],
    base_denom: &'a str,
}

/// Represents a denomination of a sequencer asset.
///
/// This can be either an IBC-bridged asset or a native asset.
/// If it's a native asset, the prefix will be empty.
///
/// Note that the full denomination trace of the token is `prefix/base_denom`,
/// in the case that a prefix is present.
/// This is hashed to create the ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denom {
    id: Id,
    prefix_segments: Vec<String>,
    base_denom: String,
}

impl Denom {
    #[must_use]
    pub fn is_prefixed(&self) -> bool {
        !self.prefix_segments.is_empty()
    }

    /// Returns the asset ID, which is the hash of the denomination trace.
    #[must_use]
    pub fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub fn base_denom(&self) -> &str {
        &self.base_denom
    }

    #[must_use]
    pub fn channel(&self) -> Option<&str> {
        if self.prefix_segments.len() < 2 {
            return None;
        }
        self.prefix_segments.last().map(|seg| &**seg)
    }

    /// Returns the number of segments in `prefix` if they all match `self.prefix_segments`, or
    /// `None` if there is any mismatch.
    ///
    /// Returns `Some(0)` if `prefix` and `self.prefix_segments` are both empty.
    fn count_matching_prefix_segments(&self, prefix: &str) -> Option<usize> {
        let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
        let mut arg_segment_count = 0;
        for argument_segment in prefix.split('/') {
            match self.prefix_segments.get(arg_segment_count) {
                Some(denom_segment) if denom_segment == argument_segment => {
                    arg_segment_count = arg_segment_count.saturating_add(1);
                }
                _ => return None,
            }
        }
        Some(arg_segment_count)
    }

    #[must_use]
    pub fn prefix_matches_exactly(&self, prefix: &str) -> bool {
        self.count_matching_prefix_segments(prefix)
            .is_some_and(|count| count == self.prefix_segments.len())
    }

    #[must_use]
    pub fn is_prefixed_by(&self, prefix: &str) -> bool {
        self.count_matching_prefix_segments(prefix).is_some()
    }

    #[must_use]
    pub fn remove_prefix(&self, prefix: &str) -> Option<Self> {
        let segments_to_drop = self.count_matching_prefix_segments(prefix)?;
        let prefix_segments = self.prefix_segments[segments_to_drop..].to_vec();
        let id = Id::from_denom_fmt(&DenomFmt {
            prefix_segments: &prefix_segments,
            base_denom: &self.base_denom,
        });
        Some(Self {
            id,
            prefix_segments,
            base_denom: self.base_denom.clone(),
        })
    }

    #[must_use]
    pub fn denomination_trace(&self) -> String {
        self.to_string()
    }

    // /// Create a new [`Denom`] with the same base denomination,
    // /// but without the prefix of the original.
    // #[must_use]
    // pub fn to_base_denom(&self) -> Self {
    //     Self::from_base_denom(&self.base_denom)
    // }

    fn as_fmt(&self) -> DenomFmt<'_> {
        DenomFmt {
            prefix_segments: &self.prefix_segments,
            base_denom: &self.base_denom,
        }
    }
}

impl std::fmt::Display for Denom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_fmt().fmt(f)
    }
}

impl<'a> std::fmt::Display for DenomFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self.prefix_segments {
            f.write_str(segment)?;
            f.write_char('/')?;
        }
        f.write_str(self.base_denom)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParseDenomError(ParseDenomErrorKind);

impl ParseDenomError {
    fn empty_segment() -> Self {
        Self(ParseDenomErrorKind::EmptySegment)
    }

    fn leading_or_trailing_slashes() -> Self {
        Self(ParseDenomErrorKind::LeadingOrTrailingSlashes)
    }

    fn whitespace() -> Self {
        Self(ParseDenomErrorKind::Whitespace)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum ParseDenomErrorKind {
    #[error("one or more segments were empty (only whitespace or neighboring slashes)")]
    EmptySegment,
    #[error("input has leading or trailing slashes")]
    LeadingOrTrailingSlashes,
    #[error("segment contains whitesapce")]
    Whitespace,
}

impl std::str::FromStr for Denom {
    type Err = ParseDenomError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        fn parse_segment(s: &str) -> Result<String, ParseDenomError> {
            if s.as_bytes().iter().any(u8::is_ascii_whitespace) {
                return Err(ParseDenomError::whitespace());
            }
            if s.is_empty() {
                return Err(ParseDenomError::empty_segment());
            }
            Ok(s.to_string())
        }

        // leading/trailing whitespace is ok
        let input = input.trim();
        let trimmed = input.trim_matches('/');

        if trimmed.len() != input.len() {
            return Err(Self::Err::leading_or_trailing_slashes());
        }

        let mut segments = trimmed.split('/').peekable();
        let mut prefix_segments = Vec::new();
        let mut base_denom = String::new();
        while let Some(segment) = segments.next() {
            let parsed = parse_segment(segment)?;
            if segments.peek().is_some() {
                prefix_segments.push(parsed);
            } else {
                base_denom = parsed;
            }
        }
        Ok(Self {
            id: Id::from_denom(trimmed),
            base_denom,
            prefix_segments,
        })
    }
}

/// Asset ID, which is the hash of the denomination trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Id(
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serde::base64_serialize")
    )]
    [u8; 32],
);

impl Id {
    #[must_use]
    pub fn get(self) -> [u8; 32] {
        self.0
    }

    #[must_use]
    pub fn from_denom(denom: &str) -> Self {
        use sha2::Digest as _;
        let hash = sha2::Sha256::digest(denom.as_bytes());
        Self(hash.into())
    }

    fn from_denom_fmt(denom_fmt: &DenomFmt<'_>) -> Self {
        use sha2::Digest as _;
        let denom = denom_fmt.to_string();
        let hash = sha2::Sha256::digest(denom.as_bytes());
        Self(hash.into())
    }

    /// Returns an ID given a 32-byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the slice is not 32 bytes long.
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, IncorrectAssetIdLength> {
        if slice.len() != 32 {
            return Err(IncorrectAssetIdLength {
                received: slice.len(),
            });
        }

        let mut id = [0u8; 32];
        id.copy_from_slice(slice);
        Ok(Self(id))
    }
}

impl From<String> for Id {
    fn from(denom: String) -> Self {
        Self::from_denom(&denom)
    }
}

impl From<[u8; 32]> for Id {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use base64::{
            display::Base64Display,
            prelude::BASE64_STANDARD,
        };
        Base64Display::new(self.as_ref(), &BASE64_STANDARD).fmt(f)
    }
}

/// Indicates that the protobuf response contained an array field that was not 32 bytes long.
#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes, got {received}")]
pub struct IncorrectAssetIdLength {
    received: usize,
}

#[cfg(test)]
mod tests {
    use super::Denom;
    use crate::primitive::v1::asset::ParseDenomErrorKind;

    #[test]
    fn prefixed_input_gives_denom_with_correct_number_of_segments() {
        let input = "path/to/denom";
        let denom = input.parse::<Denom>().unwrap();
        assert_eq!(2, denom.prefix_segments.len());
        assert_eq!("path", denom.prefix_segments[0]);
        assert_eq!("to", denom.prefix_segments[1]);
        assert_eq!("denom", denom.base_denom);
    }

    #[test]
    fn denom_with_slashed_prefix_suffix_is_rejected() {
        use ParseDenomErrorKind::{
            EmptySegment,
            LeadingOrTrailingSlashes,
            Whitespace,
        };
        #[track_caller]
        // allow: silly lint
        #[allow(clippy::needless_pass_by_value)]
        fn assert_error(input: &str, kind: ParseDenomErrorKind) {
            let error = input
                .parse::<Denom>()
                .expect_err("an error was expected, but a valid denomination was returned");
            assert_eq!(kind, error.0);
        }
        assert_error("/path/to/denom", LeadingOrTrailingSlashes);
        assert_error("path/to/denom/", LeadingOrTrailingSlashes);
        assert_error("path//to/denom", EmptySegment);
        assert_error("path/ /to/denom", Whitespace);
        assert_error("path/to /denom", Whitespace);
        assert_error("path/to/ denom", Whitespace);
        assert_error("", EmptySegment);
        assert_error(" ", EmptySegment);
    }

    #[test]
    fn denom_is_correctly_formatted() {
        let input = "path/to/denom";
        let denom = input.parse::<Denom>().unwrap();
        let output = denom.to_string();
        assert_eq!(input, output);
    }

    #[test]
    fn full_prefix_of_denom() {
        let input = "path/to/denom";
        let denom = input.parse::<Denom>().unwrap();
        assert!(denom.prefix_matches_exactly("path/to"));
        assert!(denom.prefix_matches_exactly("path/to/"));
        assert!(!denom.prefix_matches_exactly("path/to/denom"));
        assert!(!denom.prefix_matches_exactly("/path/to"));
        assert!(!denom.prefix_matches_exactly("/path/to/"));
    }

    #[test]
    fn partial_prefix_of_denom() {
        let input = "path/to/denom";
        let denom = input.parse::<Denom>().unwrap();
        assert!(denom.is_prefixed_by("path"));
        assert!(denom.is_prefixed_by("path/"));
        assert!(denom.is_prefixed_by("path/to"));
        assert!(denom.is_prefixed_by("path/to/"));
        assert!(!denom.is_prefixed_by(" path"));
        assert!(!denom.is_prefixed_by(" path/"));
        assert!(!denom.is_prefixed_by(" path/to"));
        assert!(!denom.is_prefixed_by(" path/to/"));
        assert!(!denom.is_prefixed_by("path "));
        assert!(!denom.is_prefixed_by("path/ "));
        assert!(!denom.is_prefixed_by("path/to "));
        assert!(!denom.is_prefixed_by("path/to/ "));
        assert!(!denom.is_prefixed_by("/path"));
        assert!(!denom.is_prefixed_by("/path/"));
        assert!(!denom.is_prefixed_by("/path/to"));
        assert!(!denom.is_prefixed_by("/path/to/"));
    }

    #[test]
    fn prefix_matches_exactly_removed() {
        let input = "path/to/denom";
        let denom = input.parse::<Denom>().unwrap();
        assert_eq!(
            denom.remove_prefix("path"),
            "to/denom".parse::<Denom>().ok()
        );
        assert_eq!(
            denom.remove_prefix("path/"),
            "to/denom".parse::<Denom>().ok()
        );
        assert_eq!(
            denom.remove_prefix("path/to"),
            "denom".parse::<Denom>().ok()
        );
        assert_eq!(
            denom.remove_prefix("path/to/"),
            "denom".parse::<Denom>().ok()
        );
        assert!(denom.remove_prefix("other").is_none());
        assert!(denom.remove_prefix(" path/to").is_none());
        assert!(denom.remove_prefix("path/to ").is_none());
        assert!(denom.remove_prefix("path/to/denom").is_none());
    }
}
