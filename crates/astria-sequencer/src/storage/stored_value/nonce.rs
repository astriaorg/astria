use borsh::{
    BorshDeserialize,
    BorshSerialize,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Nonce(u32);

impl From<u32> for Nonce {
    fn from(nonce: u32) -> Self {
        Nonce(nonce)
    }
}

impl From<Nonce> for u32 {
    fn from(nonce: Nonce) -> Self {
        nonce.0
    }
}

impl<'a> TryFrom<StoredValue<'a>> for Nonce {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::Nonce(nonce) = value else {
            return Err(super::type_mismatch("nonce", &value));
        };
        Ok(nonce)
    }
}
