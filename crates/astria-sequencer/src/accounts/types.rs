use anyhow::anyhow;
use astria_proto::generated::primitive::v1::Uint128 as ProtoBalance;
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
pub struct Address(pub(crate) [u8; ADDRESS_LEN]);

impl Address {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Decodes an address from a hex string.
    ///
    /// # Errors
    ///
    /// - if the string is not valid hex
    /// - if the string is not 20 bytes long
    pub fn try_from_str(s: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(s)?;
        let arr: [u8; ADDRESS_LEN] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid address hex length"))?;
        Ok(Self(arr))
    }

    /// Returns the address of the account associated with the given verification key,
    /// which is calculated as the first 20 bytes of the sha256 hash of the verification key.
    #[must_use]
    pub fn from_verification_key(public_key: &crate::crypto::VerificationKey) -> Self {
        let bytes = crate::hash(public_key.as_bytes());
        let arr: [u8; ADDRESS_LEN] = bytes[0..ADDRESS_LEN]
            .try_into()
            .expect("can convert 32 byte hash to 20 byte array");
        Address(arr)
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
