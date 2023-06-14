use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use serde::{
    Deserialize,
    Serialize,
};

/// Address represents an account address.
#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct Address(String);

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Address {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Address {
    pub(crate) fn to_str(&self) -> &str {
        &self.0
    }
}

/// Balance represents an account balance.
#[derive(
    Clone,
    Copy,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
)]
pub(crate) struct Balance(u128);

impl Balance {
    pub(crate) fn into_inner(self) -> u128 {
        self.0
    }
}

impl From<u128> for Balance {
    fn from(n: u128) -> Self {
        Self(n)
    }
}

impl std::ops::Add<u128> for Balance {
    type Output = Self;

    fn add(self, rhs: u128) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Add for Balance {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Balance {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Sub<u128> for Balance {
    type Output = Self;

    fn sub(self, rhs: u128) -> Self::Output {
        Self(self.0 - rhs)
    }
}

// Nonce represents an account nonce.
#[derive(
    Clone,
    Copy,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
)]
pub(crate) struct Nonce(u32);

impl Nonce {
    pub(crate) fn into_inner(self) -> u32 {
        self.0
    }
}

impl From<u32> for Nonce {
    fn from(n: u32) -> Self {
        Self(n)
    }
}

impl std::ops::Add for Nonce {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::cmp::PartialEq<u32> for Nonce {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}
