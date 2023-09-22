use tendermint::abci::Code;

#[derive(Clone, Copy, Debug)]
pub struct AbciCode(u32);

#[rustfmt::skip]
impl AbciCode {
    pub const OK: Self = Self(0);
    pub const UNKNOWN_PATH: Self = Self(1);
    pub const INVALID_PARAMETER: Self = Self(2);
    pub const INTERNAL_ERROR: Self = Self(3);
    pub const INVALID_NONCE: Self = Self(4);
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
