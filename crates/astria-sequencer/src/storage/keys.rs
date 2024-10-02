use std::{
    borrow::Cow,
    fmt::{
        self,
        Display,
        Formatter,
    },
    str::FromStr,
};

use astria_core::primitive::v1::asset::IbcPrefixed;

use crate::accounts::AddressBytes;

/// Helper struct whose `Display` impl outputs the prefix followed by the hex-encoded address.
pub(crate) struct AddressPrefixer<'a, T> {
    prefix: &'static str,
    address: &'a T,
}

impl<'a, T> AddressPrefixer<'a, T> {
    pub(crate) fn new(prefix: &'static str, address: &'a T) -> Self {
        Self {
            prefix,
            address,
        }
    }
}

impl<'a, T: AddressBytes> Display for AddressPrefixer<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            self.prefix,
            hex::encode(self.address.address_bytes())
        )
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

    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl<'a> Display for Asset<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&hex::encode(self.0.as_bytes()))
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

#[derive(Debug, thiserror::Error)]
#[error("failed to parse input as asset key")]
pub(crate) struct ParseAssetKeyError {
    #[from]
    source: hex::FromHexError,
}

impl<'a> FromStr for Asset<'a> {
    type Err = ParseAssetKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use hex::FromHex as _;
        let bytes = <[u8; IbcPrefixed::LENGTH]>::from_hex(s)?;
        Ok(Self(Cow::Owned(IbcPrefixed::new(bytes))))
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
