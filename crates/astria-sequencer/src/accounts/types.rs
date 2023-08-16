use astria_proto::generated::primitive::v1::Uint128 as ProtoBalance;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use serde::{
    Deserialize,
    Serialize,
};

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
pub struct Balance(pub(crate) u128);

impl Balance {
    pub(crate) fn into_inner(self) -> u128 {
        self.0
    }

    pub(crate) fn as_proto(&self) -> ProtoBalance {
        ProtoBalance::from(self.0)
    }

    pub(crate) fn from_proto(proto: ProtoBalance) -> Self {
        Self(proto.into())
    }

    pub(crate) fn checked_mul<T: Into<u128>>(self, rhs: T) -> Option<Self> {
        let new_balance = self.0.checked_mul(rhs.into())?;
        Some(Self(new_balance))
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
pub struct Nonce(pub(crate) u32);

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

impl From<Nonce> for u32 {
    fn from(n: Nonce) -> Self {
        n.0
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
