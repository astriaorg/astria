use std::{
    collections::VecDeque,
    str::FromStr,
};

/// Represents a denomination of a sequencer asset.
///
/// This can be either an IBC-bridged asset or a native asset.
/// If it's a native asset, the prefix will be empty.
///
/// Note that the full denomination trace of the token is `prefix/base_denom`,
/// in the case that a prefix is present.
/// This is hashed to create the ID.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Denom {
    TracePrefixed(TracePrefixed),
    IbcPrefixed(IbcPrefixed),
}

impl Denom {
    #[must_use]
    pub fn is_ibc_prefixed(&self) -> bool {
        self.as_ibc_prefixed().is_some()
    }

    #[must_use]
    pub fn is_trace_prefixed(&self) -> bool {
        self.as_trace_prefixed().is_some()
    }

    #[must_use]
    pub fn as_ibc_prefixed(&self) -> Option<&IbcPrefixed> {
        match self {
            Denom::TracePrefixed(_) => None,
            Denom::IbcPrefixed(ibc_prefixed) => Some(ibc_prefixed),
        }
    }

    #[must_use]
    pub fn as_trace_prefixed(&self) -> Option<&TracePrefixed> {
        match self {
            Denom::TracePrefixed(trace) => Some(trace),
            Denom::IbcPrefixed(_) => None,
        }
    }

    #[must_use]
    pub fn to_ibc_prefixed(&self) -> IbcPrefixed {
        match self {
            Denom::TracePrefixed(trace) => trace.to_ibc_prefixed(),
            Denom::IbcPrefixed(ibc) => *ibc,
        }
    }

    /// Unwraps the inner ibc prefixed denom.
    ///
    /// # Panics
    /// Panics if the self value equals [`Self::TracePrefixed`].
    #[must_use]
    pub fn unwrap_ibc_prefixed(self) -> IbcPrefixed {
        let Self::IbcPrefixed(ibc) = self else {
            panic!("not ibc prefixed");
        };
        ibc
    }

    /// Unwraps the inner trace prefixed denom.
    ///
    /// # Panics
    /// Panics if the self value equals [`Self::IbcPrefixed`].
    #[must_use]
    pub fn unwrap_trace_prefixed(self) -> TracePrefixed {
        let Self::TracePrefixed(trace) = self else {
            panic!("not trace prefixed");
        };
        trace
    }

    /// Calculates the length of the display formatted [Denom] without allocating a String.
    #[must_use]
    pub fn display_len(&self) -> usize {
        match self {
            Denom::TracePrefixed(trace) => trace.display_len(),
            Denom::IbcPrefixed(ibc) => ibc.display_len(),
        }
    }
}

impl From<IbcPrefixed> for Denom {
    fn from(value: IbcPrefixed) -> Self {
        Self::IbcPrefixed(value)
    }
}

impl From<TracePrefixed> for Denom {
    fn from(value: TracePrefixed) -> Self {
        Self::TracePrefixed(value)
    }
}

impl<'a> From<&'a IbcPrefixed> for Denom {
    fn from(value: &IbcPrefixed) -> Self {
        Self::IbcPrefixed(*value)
    }
}

impl<'a> From<&'a TracePrefixed> for Denom {
    fn from(value: &TracePrefixed) -> Self {
        Self::TracePrefixed(value.clone())
    }
}

impl From<TracePrefixed> for IbcPrefixed {
    fn from(value: TracePrefixed) -> Self {
        IbcPrefixed::from(&value)
    }
}

impl<'a> From<&'a TracePrefixed> for IbcPrefixed {
    fn from(value: &TracePrefixed) -> Self {
        value.to_ibc_prefixed()
    }
}

impl From<Denom> for IbcPrefixed {
    fn from(value: Denom) -> Self {
        value.to_ibc_prefixed()
    }
}

impl<'a> From<&'a Denom> for IbcPrefixed {
    fn from(value: &Denom) -> Self {
        value.to_ibc_prefixed()
    }
}

impl FromStr for Denom {
    type Err = ParseDenomError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let this = if s.starts_with("ibc/") {
            Self::IbcPrefixed(s.parse()?)
        } else {
            Self::TracePrefixed(s.parse()?)
        };
        Ok(this)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParseDenomError(ParseDenomErrorKind);

impl std::fmt::Display for Denom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::TracePrefixed(p) => p.fmt(f),
            Self::IbcPrefixed(i) => i.fmt(f),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ParseDenomErrorKind {
    #[error("failed to parse input as ibc denom")]
    IbcPrefixed {
        #[from]
        source: ParseIbcPrefixedError,
    },
    #[error("failed to parse input as denom trace denom")]
    Prefixed {
        #[from]
        source: ParseTracePrefixedError,
    },
}

impl From<ParseIbcPrefixedError> for ParseDenomError {
    fn from(value: ParseIbcPrefixedError) -> Self {
        Self(value.into())
    }
}

impl From<ParseTracePrefixedError> for ParseDenomError {
    fn from(value: ParseTracePrefixedError) -> Self {
        Self(value.into())
    }
}

/// An ICS20 denomination of the form `[port/channel/..]base_denom`.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TracePrefixed {
    trace: TraceSegments,
    base_denom: String,
}

impl TracePrefixed {
    #[must_use]
    pub fn to_ibc_prefixed(&self) -> IbcPrefixed {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        for segment in &self.trace.inner {
            hasher.update(segment.port().as_bytes());
            hasher.update(b"/");
            hasher.update(segment.channel().as_bytes());
            hasher.update(b"/");
        }
        hasher.update(self.base_denom.as_bytes());
        let id = hasher.finalize().into();
        IbcPrefixed {
            id,
        }
    }

    #[must_use]
    pub fn trace_is_empty(&self) -> bool {
        self.trace.is_empty()
    }

    /// Checks if the trace prefixed denom has `port` in left-most position.
    ///
    /// # Examples
    ///
    /// ```
    /// use astria_core::primitive::v1::asset::denom::TracePrefixed;
    /// let denom = "four/segments/of/a/denom".parse::<TracePrefixed>().unwrap();
    /// assert!(denom.has_leading_port("four"));
    /// assert!(!denom.has_leading_port("segments"));
    /// assert!(!denom.has_leading_port("of"));
    /// assert!(!denom.has_leading_port("a"));
    /// assert!(!denom.has_leading_port("denom"));
    /// assert!(!denom.has_leading_port(""));
    /// ```
    #[must_use]
    pub fn has_leading_port<T: AsRef<str>>(&self, port: T) -> bool {
        self.trace.leading_port() == Some(port.as_ref())
    }

    /// Checks if the trace prefixed denom has `channel` in left-most position.
    ///
    /// # Examples
    ///
    /// ```
    /// use astria_core::primitive::v1::asset::denom::TracePrefixed;
    /// let denom = "four/segments/of/a/denom".parse::<TracePrefixed>().unwrap();
    /// assert!(!denom.has_leading_channel("four"));
    /// assert!(denom.has_leading_channel("segments"));
    /// assert!(!denom.has_leading_channel("of"));
    /// assert!(!denom.has_leading_channel("a"));
    /// assert!(!denom.has_leading_channel("denom"));
    /// assert!(!denom.has_leading_channel(""));
    /// ```
    #[must_use]
    pub fn has_leading_channel<T: AsRef<str>>(&self, channel: T) -> bool {
        self.trace.leading_channel() == Some(channel.as_ref())
    }

    #[must_use]
    pub fn last_channel(&self) -> Option<&str> {
        self.trace.last_channel()
    }

    pub fn pop_leading_port_and_channel(&mut self) -> Option<PortAndChannel> {
        self.trace.pop()
    }

    pub fn push_trace_segment(&mut self, segment: PortAndChannel) {
        self.trace.push(segment);
    }

    /// Calculates the length of the display formatted [`TracePrefixed`] without allocating a
    /// String.
    #[must_use]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "string derived length should never overflow usize::MAX on 64 bit machines \
                  because of memory constraints"
    )]
    fn display_len(&self) -> usize {
        let mut len: usize = 0;
        for segment in &self.trace.inner {
            len += segment.port.len() + segment.channel.len() + 2; // 2 additional "/" characters
        }
        len + self.base_denom.len()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct TraceSegments {
    inner: VecDeque<PortAndChannel>,
}

impl TraceSegments {
    fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }

    fn leading_port(&self) -> Option<&str> {
        self.inner.front().map(|segment| &*segment.port)
    }

    fn leading_channel(&self) -> Option<&str> {
        self.inner.front().map(|segment| &*segment.channel)
    }

    fn push(&mut self, seg: PortAndChannel) {
        self.inner.push_back(seg);
    }

    fn pop(&mut self) -> Option<PortAndChannel> {
        self.inner.pop_front()
    }

    fn last_channel(&self) -> Option<&str> {
        self.inner.back().map(|segment| &*segment.channel)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl FromStr for TraceSegments {
    type Err = ParseTracePrefixedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(Self::Err::not_ascii());
        }
        if s.as_bytes().iter().any(u8::is_ascii_whitespace) {
            return Err(Self::Err::whitespace());
        }
        let mut split = s.split('/');
        let mut parsed_segments = TraceSegments::new();
        loop {
            let Some(port) = split.next() else {
                break;
            };
            let Some(channel) = split.next() else {
                return Err(Self::Err::port_without_channel());
            };
            if port.is_empty() {
                return Err(Self::Err::port_is_empty());
            }
            if channel.is_empty() {
                return Err(Self::Err::channel_is_empty());
            }
            parsed_segments.push(PortAndChannel {
                port: port.into(),
                channel: channel.into(),
            });
        }
        Ok(parsed_segments)
    }
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PortAndChannel {
    port: String,
    channel: String,
}

impl PortAndChannel {
    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }

    #[must_use]
    pub fn port(&self) -> &str {
        &self.port
    }
}

impl std::fmt::Display for TracePrefixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in &self.trace.inner {
            f.write_str(&segment.port)?;
            f.write_str("/")?;
            f.write_str(&segment.channel)?;
            f.write_str("/")?;
        }
        f.write_str(&self.base_denom)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParseTracePrefixedError(ParseTracePrefixedErrorKind);

impl ParseTracePrefixedError {
    fn base_is_empty() -> Self {
        Self(ParseTracePrefixedErrorKind::BaseIsEmpty)
    }

    fn channel_is_empty() -> Self {
        Self(ParseTracePrefixedErrorKind::ChannelIsEmpty)
    }

    fn port_is_empty() -> Self {
        Self(ParseTracePrefixedErrorKind::PortIsEmpty)
    }

    fn not_ascii() -> Self {
        Self(ParseTracePrefixedErrorKind::NotAscii)
    }

    fn port_without_channel() -> Self {
        Self(ParseTracePrefixedErrorKind::PortWithoutChannel)
    }

    fn whitespace() -> Self {
        Self(ParseTracePrefixedErrorKind::Whitespace)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum ParseTracePrefixedErrorKind {
    #[error("the input itself or its base denom segment is empty")]
    BaseIsEmpty,
    #[error("a port segment was empty")]
    PortIsEmpty,
    #[error("a channel segment was empty")]
    ChannelIsEmpty,
    #[error("input contained non-ascii characters")]
    NotAscii,
    #[error("input contains whitespace")]
    Whitespace,
    #[error(
        "the denom trace path was lopsided, there was one port without matching channel segment"
    )]
    PortWithoutChannel,
}

impl FromStr for TracePrefixed {
    type Err = ParseTracePrefixedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(Self::Err::not_ascii());
        }
        if s.as_bytes().iter().any(u8::is_ascii_whitespace) {
            return Err(Self::Err::whitespace());
        }
        let (trace, base_denom) = match s.rsplit_once('/') {
            Some((path, base)) => (path.parse::<TraceSegments>()?, base),
            None => (TraceSegments::new(), s),
        };
        if base_denom.is_empty() {
            return Err(Self::Err::base_is_empty());
        }
        Ok(Self {
            base_denom: base_denom.into(),
            trace,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ParseIbcPrefixedError(ParseIbcPrefixedErrorKind);

impl ParseIbcPrefixedError {
    fn hex(source: hex::FromHexError) -> Self {
        Self(ParseIbcPrefixedErrorKind::Hex {
            source,
        })
    }

    fn not_ibc_prefixed() -> Self {
        Self(ParseIbcPrefixedErrorKind::NotIbcPrefixedPrefixed)
    }

    fn too_few_segments() -> Self {
        Self(ParseIbcPrefixedErrorKind::TooFewSegments)
    }

    fn too_many_segments() -> Self {
        Self(ParseIbcPrefixedErrorKind::TooManySegments)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum ParseIbcPrefixedErrorKind {
    #[error("input was not hex encoded or of the wrong length")]
    Hex { source: hex::FromHexError },
    #[error("input was not prefixed by `ibc/`")]
    NotIbcPrefixedPrefixed,
    #[error("input had too few segments")]
    TooFewSegments,
    #[error("input had too many segments")]
    TooManySegments,
}

/// An ICS20 denomination of the form `ibc/<hex-sha256-hash>`.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct IbcPrefixed {
    id: [u8; 32],
}

impl IbcPrefixed {
    #[must_use]
    pub fn new(id: [u8; 32]) -> Self {
        Self {
            id,
        }
    }

    #[must_use]
    pub fn get(&self) -> [u8; 32] {
        self.id
    }

    #[must_use]
    pub fn display_len(&self) -> usize {
        68 // "ibc/" + 64 hex characters
    }
}

impl std::fmt::Display for IbcPrefixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ibc/")?;
        for byte in self.id {
            f.write_fmt(format_args!("{byte:02x}"))?;
        }
        Ok(())
    }
}

impl FromStr for IbcPrefixed {
    type Err = ParseIbcPrefixedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use hex::FromHex as _;
        let mut segments = s.split('/');
        let Some("ibc") = segments.next() else {
            return Err(ParseIbcPrefixedError::not_ibc_prefixed());
        };
        let Some(hex) = segments.next() else {
            return Err(ParseIbcPrefixedError::too_few_segments());
        };
        if segments.next().is_some() {
            return Err(ParseIbcPrefixedError::too_many_segments());
        }
        let id = <[u8; 32]>::from_hex(hex).map_err(Self::Err::hex)?;
        Ok(Self {
            id,
        })
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::{
        Deserialize,
        Deserializer,
        Serialize,
        Serializer,
    };

    macro_rules! impl_serde {
        ($($type:ty),*$(,)?) => {
            $(
                impl<'de> Deserialize<'de> for $type {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        use serde::de::Error as _;
                        let s = std::borrow::Cow::<'_, str>::deserialize(deserializer)?;
                        s.trim().parse().map_err(D::Error::custom)
                    }
                }

                impl Serialize for $type {
                    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where
                        S: Serializer,
                    {
                        serializer.collect_str(self)
                    }
                }
            )*
        }
    }

    impl_serde!(super::Denom, super::TracePrefixed, super::IbcPrefixed);

    #[cfg(test)]
    mod tests {
        use super::super::IbcPrefixed;
        use crate::primitive::v1::asset::{
            denom::TracePrefixed,
            Denom,
        };

        fn trace_prefixed() -> TracePrefixed {
            "a/trace/pre/fixed/denom".parse().unwrap()
        }
        fn ibc_prefixed() -> IbcPrefixed {
            use sha2::{
                Digest as _,
                Sha256,
            };
            let bytes: [u8; 32] = Sha256::digest("a/trace/pre/fixed/denom").into();
            IbcPrefixed::new(bytes)
        }
        #[test]
        fn snapshots() {
            insta::assert_json_snapshot!(ibc_prefixed());
            insta::assert_json_snapshot!(trace_prefixed());
            insta::assert_json_snapshot!(Denom::from(ibc_prefixed()));
            insta::assert_json_snapshot!(Denom::from(trace_prefixed()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Denom,
        IbcPrefixed,
        ParseIbcPrefixedErrorKind,
        ParseTracePrefixedErrorKind,
        TracePrefixed,
    };

    #[test]
    fn parse_ibc_errors() {
        use ParseIbcPrefixedErrorKind::{
            Hex,
            NotIbcPrefixedPrefixed,
            TooFewSegments,
            TooManySegments,
        };
        #[track_caller]
        // allow: silly lint
        #[allow(clippy::needless_pass_by_value)]
        fn assert_error(input: &str, kind: ParseIbcPrefixedErrorKind) {
            let error = input
                .parse::<IbcPrefixed>()
                .expect_err("an error was expected, but a valid denomination was returned");
            assert_eq!(kind, error.0);
        }
        #[track_caller]
        // allow: silly lint
        #[allow(clippy::needless_pass_by_value)]
        fn assert_hex_error(input: &str) {
            let error = input
                .parse::<IbcPrefixed>()
                .expect_err("invalid hex provided, should have returned an error");
            assert!(
                matches!(error.0, Hex { .. }),
                "expected a `Hex {{..}}` error, but got {error:?}"
            );
        }
        assert_error("/ibc/denom", NotIbcPrefixedPrefixed);
        assert_error("notibc/denom", NotIbcPrefixedPrefixed);
        assert_error("ibc", TooFewSegments);
        assert_error("ibc/and/more", TooManySegments);
        assert_hex_error("ibc/nothex");
        assert_hex_error("ibc/");
        assert_hex_error("ibc/ ");
        assert_hex_error(&format!("ibc/{}", hex::encode(vec![42; 31])));
        assert_hex_error(&format!("ibc/{}", hex::encode(vec![42; 33])));
    }

    #[test]
    fn parse_trace_errors() {
        use ParseTracePrefixedErrorKind::{
            BaseIsEmpty,
            ChannelIsEmpty,
            NotAscii,
            PortIsEmpty,
            Whitespace,
        };
        #[track_caller]
        // allow: silly lint
        #[allow(clippy::needless_pass_by_value)]
        fn assert_error(input: &str, kind: ParseTracePrefixedErrorKind) {
            let error = input
                .parse::<TracePrefixed>()
                .expect_err("an error was expected, but a valid denomination was returned");
            assert_eq!(kind, error.0);
        }
        assert_error("path/to/", BaseIsEmpty);
        assert_error("path//denom", ChannelIsEmpty);
        assert_error("/to/denom", PortIsEmpty);
        assert_error("path/ /to/denom", Whitespace);
        assert_error("path/to /denom", Whitespace);
        assert_error("path/to/ denom", Whitespace);
        assert_error(" path/to/denom", Whitespace);
        assert_error("path/to/denom ", Whitespace);
        assert_error("path/🦀/denom", NotAscii);
        assert_error("", BaseIsEmpty);
    }

    #[test]
    fn high_level_parse_and_format() {
        #[track_caller]
        fn assert_formatting(input: &str) {
            let denom = input.parse::<Denom>().unwrap();
            let output = denom.to_string();
            assert_eq!(input, output);
        }
        assert_formatting("path/to/denom");
        assert_formatting("slightly/longer/path/to/denom");
        assert_formatting(&format!("ibc/{}", hex::encode([42u8; 32])));
    }

    #[test]
    fn pop_path() {
        let mut denom = "a/long/path/to/denom".parse::<TracePrefixed>().unwrap();
        let port_and_channel = denom.pop_leading_port_and_channel().unwrap();
        assert_eq!("a", port_and_channel.port());
        assert_eq!("long", port_and_channel.channel());

        let port_and_channel = denom.pop_leading_port_and_channel().unwrap();
        assert_eq!("path", port_and_channel.port());
        assert_eq!("to", port_and_channel.channel());

        assert_eq!(None, denom.pop_leading_port_and_channel());
    }

    #[test]
    fn display_len_outputs_expected_length() {
        assert_correct_display_len("0123456789");
        assert_correct_display_len("path_with-special^characters!@#$%&*()+={}|;:?<>,.`~");

        assert_correct_display_len("MixedCasePath");
        assert_correct_display_len("denom");
        assert_correct_display_len("short/path/denom");
        assert_correct_display_len("a/very/long/path/to/the/denom");
        assert_correct_display_len(&format!("ibc/{}", hex::encode([0u8; 32])));
        assert_correct_display_len(&format!("ibc/{}", hex::encode([1u8; 32])));
        assert_correct_display_len(&format!("ibc/{}", hex::encode([42u8; 32])));
        assert_correct_display_len(&format!("ibc/{}", hex::encode([255u8; 32])));
    }

    #[track_caller]
    fn assert_correct_display_len(denom_str: &str) {
        let denom = denom_str.parse::<Denom>().unwrap();
        assert_eq!(denom_str.len(), denom.display_len());
    }
}
