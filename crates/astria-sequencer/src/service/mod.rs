pub(crate) mod consensus;
pub(crate) mod info;
pub(crate) mod mempool;
pub(crate) mod snapshot;

pub(crate) use consensus::Consensus;
pub(crate) use info::Info;
pub(crate) use mempool::Mempool;
pub(crate) use snapshot::Snapshot;
use tendermint::abci::Code;

#[derive(Clone, Copy, Debug)]
pub(crate) struct AbciCode(u32);

#[rustfmt::skip]
impl AbciCode {
    pub(crate) const OK: Self = Self(0);
    pub(crate) const UNKNOWN_PATH: Self = Self(1);
    pub(crate) const INVALID_PARAMETER: Self = Self(2);
    pub(crate) const INTERNAL_ERROR: Self = Self(3);
    pub(crate) const INVALID_NONCE: Self = Self(4);
    pub(crate) const INVALID_SIGNATURE: Self = Self(5);
}

impl AbciCode {
    pub(crate) fn info(self) -> Option<&'static str> {
        match self.0 {
            0 => Some("Ok"),
            1 => Some("provided path is unknown"),
            2 => Some("one or more path parameters were invalid"),
            3 => Some("an internal server error occured"),
            4 => Some("the provided nonce was invalid"),
            5 => Some("the provided signature was invalid"),
            _ => None,
        }
    }
}

impl std::fmt::Display for AbciCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info().unwrap_or("<unknown abci code>"))
    }
}

impl From<AbciCode> for Code {
    fn from(value: AbciCode) -> Self {
        value.0.into()
    }
}
