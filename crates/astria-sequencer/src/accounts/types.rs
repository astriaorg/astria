use anyhow::anyhow;
use astria_proto::primitive::v1::Uint128 as ProtoBalance;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use serde::{
    Deserialize,
    Serialize,
};

/// The length of an account address in bytes.
pub(crate) const ADDRESS_LEN: usize = 20;

/// Represents an account address.
#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct Address([u8; ADDRESS_LEN]);

impl Address {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[allow(dead_code)]
    pub(crate) fn from_array(arr: [u8; ADDRESS_LEN]) -> Self {
        Self(arr)
    }
}

impl hex::FromHex for Address {
    type Error = anyhow::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> std::result::Result<Self, Self::Error> {
        let bytes = hex::decode(hex)?;
        let arr: [u8; ADDRESS_LEN] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid address hex length"))?;
        Ok(Self(arr))
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        let arr: [u8; ADDRESS_LEN] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid address length"))?;
        Ok(Address(arr))
    }
}

impl TryFrom<&str> for Address {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        let bytes = hex::decode(s)?;
        let arr: [u8; ADDRESS_LEN] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid address hex length"))?;
        Ok(Address(arr))
    }
}

impl TryFrom<&crate::crypto::VerificationKey> for Address {
    type Error = anyhow::Error;

    fn try_from(public_key: &crate::crypto::VerificationKey) -> Result<Self, Self::Error> {
        let bytes = crate::hash(public_key.as_bytes());
        let arr: [u8; ADDRESS_LEN] = bytes[0..ADDRESS_LEN]
            .try_into()
            .map_err(|_| anyhow!("invalid address hex length"))?;
        Ok(Address(arr))
    }
}

impl TryFrom<&crate::crypto::SigningKey> for Address {
    type Error = anyhow::Error;

    fn try_from(secret_key: &crate::crypto::SigningKey) -> Result<Self, Self::Error> {
        let bytes = crate::hash(secret_key.verification_key().as_bytes());
        let arr: [u8; ADDRESS_LEN] = bytes[0..ADDRESS_LEN]
            .try_into()
            .map_err(|_| anyhow!("invalid address hex length"))?;
        Ok(Address(arr))
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
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

impl From<&ProtoBalance> for Balance {
    fn from(proto: &ProtoBalance) -> Self {
        Self(proto.into())
    }
}

impl From<Balance> for ProtoBalance {
    fn from(balance: Balance) -> Self {
        balance.0.into()
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
