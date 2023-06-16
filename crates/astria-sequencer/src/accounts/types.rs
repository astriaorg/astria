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

pub const ADDRESS_LEN: usize = 20;

/// Address represents an account address.
#[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Address([u8; ADDRESS_LEN]);

impl TryFrom<Vec<u8>> for Address {
    type Error = anyhow::Error;

    fn try_from(bytes: Vec<u8>) -> std::result::Result<Self, Self::Error> {
        let arr: [u8; ADDRESS_LEN] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid address length"))?;
        Ok(Address(arr))
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

impl TryFrom<&crate::crypto::PublicKey> for Address {
    type Error = anyhow::Error;

    fn try_from(public_key: &crate::crypto::PublicKey) -> Result<Self, Self::Error> {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(public_key.as_bytes());
        let hash = hasher.finalize();
        Ok(Address(
            hash[0..ADDRESS_LEN]
                .try_into()
                .map_err(|_| anyhow!("invalid address hex length"))?,
        ))
    }
}

impl TryFrom<&crate::crypto::Keypair> for Address {
    type Error = anyhow::Error;

    fn try_from(keypair: &crate::crypto::Keypair) -> Result<Self, Self::Error> {
        use sha2::Digest as _;
        let mut hasher = sha2::Sha256::new();
        hasher.update(keypair.public.as_bytes());
        let hash = hasher.finalize();
        Ok(Address(
            hash[0..ADDRESS_LEN]
                .try_into()
                .map_err(|_| anyhow!("invalid address hex length"))?,
        ))
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Address {
    /// attempts to decode the given hex string into an address.
    /// WARNING: this function panics on failure; use `try_from` instead.
    pub fn unsafe_from_hex_string(s: &str) -> Self {
        let bytes = hex::decode(s).unwrap();
        let arr: [u8; ADDRESS_LEN] = bytes
            .try_into()
            .map_err(|_| anyhow!("invalid address hex length"))
            .unwrap();
        Address(arr)
    }

    pub fn as_bytes(&self) -> &[u8] {
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
pub struct Balance(u128);

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

impl Balance {
    pub fn as_proto(&self) -> ProtoBalance {
        ProtoBalance {
            hi: (self.0 >> 64) as u64,
            lo: self.0 as u64,
        }
    }

    pub fn from_proto(proto: &ProtoBalance) -> Self {
        Self((proto.hi as u128) << 64 | proto.lo as u128)
    }
}

impl From<ProtoBalance> for Balance {
    fn from(proto: ProtoBalance) -> Self {
        Self::from_proto(&proto)
    }
}

impl From<&ProtoBalance> for Balance {
    fn from(proto: &ProtoBalance) -> Self {
        Self::from_proto(proto)
    }
}

impl From<Balance> for ProtoBalance {
    fn from(balance: Balance) -> Self {
        balance.as_proto()
    }
}

impl From<&Balance> for ProtoBalance {
    fn from(balance: &Balance) -> Self {
        balance.as_proto()
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
pub struct Nonce(u32);

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
