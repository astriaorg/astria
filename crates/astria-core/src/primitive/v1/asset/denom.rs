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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn id(&self) -> super::Id {
        match self {
            Self::TracePrefixed(trace) => trace.id(),
            Self::IbcPrefixed(ibc) => ibc.id(),
        }
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracePrefixed {
    trace: TraceSegments,
    base_denom: String,
}

impl TracePrefixed {
    #[must_use]
    pub fn id(&self) -> super::Id {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        for segment in &self.trace.inner {
            hasher.update(segment.port().as_bytes());
            hasher.update(b"/");
            hasher.update(segment.channel().as_bytes());
            hasher.update(b"/");
        }
        hasher.update(self.base_denom.as_bytes());
        super::Id::new(hasher.finalize().into())
    }

    #[must_use]
    pub fn trace_is_empty(&self) -> bool {
        self.trace.is_empty()
    }

    /// Checks if the trace prefixed denom starts with `s`.
    ///
    /// # Examples
    ///
    /// ```
    /// use astria_core::primitive::v1::asset::denom::TracePrefixed;
    /// let denom = "four/segments/of/a/denom".parse::<TracePrefixed>().unwrap();
    ///
    /// // Empty string is always true:
    /// assert!(denom.starts_with_str(""));
    /// // Single slash is always false:
    /// assert!(!denom.starts_with_str("/"));
    /// // Emptry strings are false:
    /// assert!(!denom.starts_with_str(" "));
    ///
    /// // In general, whitespace is not trimmed and leads to false
    /// assert!(!denom.starts_with_str("four/segments /"));
    ///
    /// // Trailing slashes don't change the result if they are part of the trace prefix:
    /// assert!(denom.starts_with_str("four/segments"));
    /// assert!(denom.starts_with_str("four/segments/"));
    ///
    /// // Trailing slashes on the full trace prefix denom however return false:
    /// assert!(!denom.starts_with_str("four/segments/of/a/denom/"));
    ///
    /// // Providing only a port is true
    /// assert!(denom.starts_with_str("four"));
    /// // Providing a full port/channel pair followed by just a port is also true
    /// assert!(denom.starts_with_str("four/segments/of"));
    ///
    /// // Half of a port or channel is false
    /// assert!(!denom.starts_with_str("four/segm"));
    ///
    /// // The full trace prefixed denom is true:
    /// assert!(denom.starts_with_str("four/segments/of/a/denom"));
    /// ```
    #[must_use]
    pub fn starts_with_str(&self, s: &str) -> bool {
        if s.is_empty() {
            return true;
        }
        let mut had_trailing_slash = false;
        let s = s
            .strip_suffix('/')
            .inspect(|_| had_trailing_slash = true)
            .unwrap_or(s);
        if s.is_empty() {
            return false;
        }
        let mut parts = s.split('/');
        for segment in self.trace.iter() {
            // first iteration: we know that s is not empty after stripping the /
            // so that this is not wrongly returning true.
            let Some(port) = parts.next() else {
                return true;
            };
            if segment.port() != port {
                return false;
            }
            let Some(channel) = parts.next() else {
                return true;
            };
            if segment.channel() != channel {
                return false;
            }
        }
        let Some(base_denom) = parts.next() else {
            return true;
        };
        if base_denom != self.base_denom {
            return false;
        }
        if had_trailing_slash {
            return false;
        }
        parts.next().is_none()
    }

    #[must_use]
    pub fn last_channel(&self) -> Option<&str> {
        self.trace.last_channel()
    }

    pub fn pop_trace_segment(&mut self) -> Option<PortAndChannel> {
        self.trace.pop()
    }

    pub fn push_trace_segment(&mut self, segment: PortAndChannel) {
        self.trace.push(segment);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceSegments {
    inner: VecDeque<PortAndChannel>,
}

impl TraceSegments {
    fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
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

    fn iter(&self) -> impl Iterator<Item = &PortAndChannel> {
        self.inner.iter()
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IbcPrefixed {
    id: [u8; 32],
}

impl IbcPrefixed {
    #[must_use]
    pub fn id(&self) -> super::Id {
        super::Id::new(self.id)
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
        let port_and_channel = denom.pop_trace_segment().unwrap();
        assert_eq!("a", port_and_channel.port());
        assert_eq!("long", port_and_channel.channel());

        let port_and_channel = denom.pop_trace_segment().unwrap();
        assert_eq!("path", port_and_channel.port());
        assert_eq!("to", port_and_channel.channel());

        assert_eq!(None, denom.pop_trace_segment());
    }

    #[test]
    fn start_prefixes() {
        let denom = "four/segments/of/a/denom".parse::<TracePrefixed>().unwrap();

        assert!(denom.starts_with_str(""));
        assert!(!denom.starts_with_str("/"));
        assert!(!denom.starts_with_str(" "));

        assert!(!denom.starts_with_str("four/segments /"));

        assert!(denom.starts_with_str("four/segments"));
        assert!(denom.starts_with_str("four/segments/"));

        assert!(!denom.starts_with_str("four/segments/of/a/denom/"));

        assert!(denom.starts_with_str("four"));
        assert!(denom.starts_with_str("four/segments/of"));

        assert!(!denom.starts_with_str("four/segm"));

        assert!(denom.starts_with_str("four/segments/of/a/denom"));
    }
}
