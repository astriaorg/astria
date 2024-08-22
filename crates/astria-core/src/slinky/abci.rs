pub mod v1 {
    use std::collections::HashMap;

    use crate::generated::astria_vendored::slinky::abci::v1 as raw;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OracleVoteExtension {
        pub prices: HashMap<u64, bytes::Bytes>,
    }

    impl OracleVoteExtension {
        #[must_use]
        pub fn from_raw(raw: raw::OracleVoteExtension) -> Self {
            Self {
                prices: raw.prices,
            }
        }

        #[must_use]
        pub fn into_raw(self) -> raw::OracleVoteExtension {
            raw::OracleVoteExtension {
                prices: self.prices,
            }
        }
    }

    impl From<raw::OracleVoteExtension> for OracleVoteExtension {
        fn from(raw: raw::OracleVoteExtension) -> Self {
            Self::from_raw(raw)
        }
    }
}
