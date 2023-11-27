use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use prost::Message;

use crate::{
    generated::block_relay::v1alpha1 as raw,
    native::sequencer::v1alpha1::{
        TransferAction,
        UnsignedTransaction,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum SignedBundleError {
    #[error("converting from proto failed")]
    ProtoDecodeFailed,
}

pub struct SignedBundle {
    pub signature: Signature,
    pub verification_key: VerificationKey,
    pub bundle: Bundle,
}

impl SignedBundle {
    // into_raw
    // to_raw
    // try_from_raw

    pub fn try_from_raw(raw: raw::SignedBundle) -> Result<Self, SignedBundleError> {
        let signature = Signature::try_from(&*raw.signature)
            .map_err(|_| SignedBundleError::ProtoDecodeFailed)?;
        let verification_key = VerificationKey::try_from(&*raw.public_key)
            .map_err(|_| SignedBundleError::ProtoDecodeFailed)?;
        let bundle = Bundle::try_from_raw(raw.bundle.ok_or(SignedBundleError::ProtoDecodeFailed)?)?;
        Ok(Self {
            signature,
            verification_key,
            bundle,
        })
    }
    // into_parts
}

pub struct Bundle {
    pub block_height: u64,
    pub bid: TransferAction,
    pub payload: UnsignedTransaction,
}

impl Bundle {
    // into_signed

    // Convert the bundle into an `OpaqueBid` that can be sent to the Proposer by hashing the
    // payload with SHA256.
    pub fn to_opaque_bid(&self) -> OpaqueBid {
        use sha2::Digest as _;

        let bytes = self.payload.to_raw().encode_to_vec();
        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        let payload_hash: [u8; 64] = hasher.finalize().into();

        OpaqueBid {
            block_height: self.block_height,
            bid: self.bid.clone(),
            payload_hash,
        }
    }

    // into_raw
    // to_raw
    // try_from_raw
    pub fn try_from_raw(raw: raw::Bundle) -> Result<Self, SignedBundleError> {
        let bid =
            TransferAction::try_from_raw(raw.bid.ok_or(SignedBundleError::ProtoDecodeFailed)?)
                .map_err(|_| SignedBundleError::ProtoDecodeFailed)?;
        let payload = UnsignedTransaction::try_from_raw(
            raw.bundle.ok_or(SignedBundleError::ProtoDecodeFailed)?,
        )
        .map_err(|_| SignedBundleError::ProtoDecodeFailed)?;
        Ok(Self {
            block_height: raw.block_height,
            bid,
            payload,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpaqueBidError {
    #[error("converting from proto failed")]
    ProtoDecodeFailed,
}

pub struct OpaqueBid {
    pub block_height: u64,
    pub bid: TransferAction,
    pub payload_hash: [u8; 64],
}

impl OpaqueBid {
    // into_raw
    pub fn into_raw(self) -> raw::OpaqueBid {
        raw::OpaqueBid {
            block_height: self.block_height,
            bid: Some(self.bid.into_raw()),
            payload_hash: self.payload_hash.to_vec(),
        }
    }

    // try_from_raw
    pub fn try_from_raw(raw: raw::OpaqueBid) -> Result<Self, OpaqueBidError> {
        let bid = TransferAction::try_from_raw(raw.bid.ok_or(OpaqueBidError::ProtoDecodeFailed)?)
            .map_err(|_| OpaqueBidError::ProtoDecodeFailed)?;
        let payload_hash = raw.payload_hash.as_slice();
        if payload_hash.len() != 32 {
            return Err(OpaqueBidError::ProtoDecodeFailed);
        }
        let mut payload_hash_bytes = [0u8; 64];
        payload_hash_bytes.copy_from_slice(payload_hash);
        Ok(Self {
            block_height: raw.block_height,
            bid,
            payload_hash: payload_hash_bytes,
        })
    }

    /// Construct the commitment to the bid using the provided signer by signing `payload_hash`.
    pub fn commitment(&self, signer: SigningKey) -> Commitment {
        Commitment::from_bytes(&signer.sign(&self.payload_hash).to_bytes())
    }

    // verify_commitment
    pub fn verify_commitment(
        &self,
        commitment: Commitment,
        verification_key: VerificationKey,
    ) -> bool {
        verification_key
            .verify(&self.payload_hash.into(), &commitment.0)
            .is_ok()
    }
}

pub struct Commitment(pub(self) [u8; 64]);

impl Commitment {
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        Self(bytes.clone())
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}
