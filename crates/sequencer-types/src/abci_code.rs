#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbciCode(u32);

#[rustfmt::skip]
impl AbciCode {
    pub const OK: Self = Self(0);
    pub const UNKNOWN_PATH: Self = Self(1);
    pub const INVALID_PARAMETER: Self = Self(2);
    pub const INTERNAL_ERROR: Self = Self(3);
    pub const INVALID_NONCE: Self = Self(4);
    pub const INVALID_SIZE: Self = Self(5);
}

impl AbciCode {
    #[must_use]
    pub fn info(self) -> Option<&'static str> {
        match self.0 {
            0 => Some("Ok"),
            1 => Some("provided path is unknown"),
            2 => Some("one or more path parameters were invalid"),
            3 => Some("an internal server error occured"),
            4 => Some("the provided nonce was invalid"),
            5 => Some("the provided transaction was too large"),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_cometbft(code: tendermint::abci::Code) -> Option<Self> {
        match code.value() {
            0 => Some(Self::OK),
            1 => Some(Self::UNKNOWN_PATH),
            2 => Some(Self::INVALID_PARAMETER),
            3 => Some(Self::INTERNAL_ERROR),
            4 => Some(Self::INVALID_NONCE),
            5 => Some(Self::INVALID_SIZE),
            _ => None,
        }
    }
}

impl std::fmt::Display for AbciCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.info().unwrap_or("<unknown abci code>"))
    }
}

impl From<AbciCode> for tendermint::abci::Code {
    fn from(value: AbciCode) -> Self {
        value.0.into()
    }
}
