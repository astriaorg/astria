use super::raw;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeAccountLastTxHashResponse {
    pub height: u64,
    pub tx_hash: Option<[u8; 32]>,
}

impl BridgeAccountLastTxHashResponse {
    /// Converts a native [`BridgeAccountLastTxHashResponse`] to a protobuf
    /// [`raw::BridgeAccountLastTxHashResponse`].
    ///
    /// # Errors
    ///
    /// - if the transaction hash is not 32 bytes
    pub fn try_from_raw(
        raw: raw::BridgeAccountLastTxHashResponse,
    ) -> Result<Self, BridgeAccountLastTxHashResponseError> {
        Ok(Self {
            height: raw.height,
            tx_hash: raw
                .tx_hash
                .map(TryInto::<[u8; 32]>::try_into)
                .transpose()
                .map_err(|bytes: Vec<u8>| {
                    BridgeAccountLastTxHashResponseError::invalid_tx_hash(bytes.len())
                })?,
        })
    }

    #[must_use]
    pub fn into_raw(self) -> raw::BridgeAccountLastTxHashResponse {
        raw::BridgeAccountLastTxHashResponse {
            height: self.height,
            tx_hash: self.tx_hash.map(Into::into),
        }
    }
}

impl raw::BridgeAccountLastTxHashResponse {
    /// Converts a protobuf [`raw::BridgeAccountLastTxHashResponse`] to a native
    /// [`BridgeAccountLastTxHashResponse`].
    ///
    /// # Errors
    ///
    /// - if the transaction hash is not 32 bytes
    pub fn try_into_native(
        self,
    ) -> Result<BridgeAccountLastTxHashResponse, BridgeAccountLastTxHashResponseError> {
        BridgeAccountLastTxHashResponse::try_from_raw(self)
    }

    #[must_use]
    pub fn from_native(
        native: BridgeAccountLastTxHashResponse,
    ) -> raw::BridgeAccountLastTxHashResponse {
        native.into_raw()
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BridgeAccountLastTxHashResponseError(BridgeAccountLastTxHashResponseErrorKind);

impl BridgeAccountLastTxHashResponseError {
    #[must_use]
    pub fn invalid_tx_hash(bytes: usize) -> Self {
        Self(BridgeAccountLastTxHashResponseErrorKind::InvalidTxHash(
            bytes,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
enum BridgeAccountLastTxHashResponseErrorKind {
    #[error("invalid tx hash; must be 32 bytes, got {0} bytes")]
    InvalidTxHash(usize),
}
