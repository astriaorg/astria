use anyhow::ensure;

/// Represents the sha256 hash of an encoded transaction.
pub struct TransactionHash([u8; 32]);

impl TryFrom<Vec<u8>> for TransactionHash {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> std::result::Result<Self, Self::Error> {
        ensure!(value.len() == 32, "invalid vector length; must be 32");

        let buf: [u8; 32] = value[..].try_into()?;
        Ok(TransactionHash(buf))
    }
}

impl TryFrom<&[u8]> for TransactionHash {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        ensure!(value.len() == 32, "invalid slice length; must be 32");

        let buf: [u8; 32] = value.try_into()?;
        Ok(TransactionHash(buf))
    }
}

impl TransactionHash {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
