use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
        Formatter,
    },
    str::FromStr,
};

use astria_core::primitive::v1::asset::{
    denom::ParseIbcPrefixedError,
    IbcPrefixed,
};

use crate::accounts::AddressBytes;

/// Helper struct whose `Display` impl outputs the prefix followed by the hex-encoded address.
pub(crate) struct AccountPrefixer<'a, T> {
    prefix: &'static str,
    address: &'a T,
}

impl<'a, T> AccountPrefixer<'a, T> {
    pub(crate) fn new(prefix: &'static str, address: &'a T) -> Self {
        Self {
            prefix,
            address,
        }
    }
}

impl<'a, T: AddressBytes> Display for AccountPrefixer<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use base64::{
            display::Base64Display,
            engine::general_purpose::URL_SAFE,
        };
        f.write_str(self.prefix)?;
        Base64Display::new(self.address.address_bytes(), &URL_SAFE).fmt(f)
    }
}

/// Helper struct whose `Display` impl outputs the hex-encoded ibc-prefixed address, and that can be
/// parsed from such a hex-encoded form.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Asset<'a>(Cow<'a, IbcPrefixed>);

impl<'a> Asset<'a> {
    pub(crate) fn get(self) -> IbcPrefixed {
        self.0.into_owned()
    }
}

impl<'a> Display for Asset<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a, T> From<&'a T> for Asset<'a>
where
    &'a T: Into<Cow<'a, IbcPrefixed>>,
{
    fn from(value: &'a T) -> Self {
        Self(value.into())
    }
}

impl<'a> FromStr for Asset<'a> {
    type Err = ParseIbcPrefixedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Cow::Owned(s.parse()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::Asset;

    #[test]
    fn asset_key_to_string_parse_roundtrip() {
        let asset = "an/asset/with/a/prefix"
            .parse::<astria_core::primitive::v1::asset::Denom>()
            .unwrap();
        let expected = Asset::from(&asset);
        let actual = expected.to_string().parse::<Asset>().unwrap();
        assert_eq!(expected, actual);
    }
}
