pub(crate) mod hunks {
    use std::str::FromStr;

    use astria_core::primitive::v1::asset;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) struct Asset(asset::IbcPrefixed);

    impl Asset {
        pub(crate) fn get(self) -> asset::IbcPrefixed {
            self.0
        }
    }

    impl std::fmt::Display for Asset {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for byte in self.get().as_bytes() {
                f.write_fmt(format_args!("{byte:02x}"))?;
            }
            Ok(())
        }
    }

    impl<T: Into<asset::IbcPrefixed>> From<T> for Asset {
        fn from(value: T) -> Self {
            Self(value.into())
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("failed to parse input as asset key")]
    pub(crate) struct ParseAssetKeyError {
        #[from]
        source: hex::FromHexError,
    }

    impl FromStr for Asset {
        type Err = ParseAssetKeyError;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            use hex::FromHex as _;
            let bytes = <[u8; 32]>::from_hex(s)?;
            Ok(Self(asset::IbcPrefixed::new(bytes)))
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
            let expected = Asset::from(asset);
            let actual = expected.to_string().parse::<Asset>().unwrap();
            assert_eq!(expected, actual);
        }
    }
}
