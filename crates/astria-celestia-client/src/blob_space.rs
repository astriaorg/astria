//! The blobs of data that are are submitted to celestia.

use celestia_types::nmt::{
    Namespace,
    NS_ID_V0_SIZE,
};
use ed25519_consensus::{
    Signature,
    SigningKey,
    VerificationKey,
};
use sequencer_types::ChainId;
use sequencer_validation::InclusionProof;
use serde::{
    de::DeserializeOwned,
    Deserialize,
    Serialize,
};
use sha2::{
    Digest as _,
    Sha256,
};
use tendermint::{
    block::Header,
    Hash,
};

/// Utility to create a v0 celestia namespace from the sha256 of `bytes`.
#[must_use]
#[allow(clippy::missing_panics_doc)] // OK because this is checked with a const assertion
pub fn celestia_namespace_v0_from_hashed_bytes(bytes: &[u8]) -> Namespace {
    // ensure that the conversion to `id` does not fail.
    // clippy: `NS_ID_V0_SIZE` is imported from a foreign crate. Catches
    // breaking changes.
    #[allow(clippy::assertions_on_constants)]
    const _: () = assert!(NS_ID_V0_SIZE < 32);
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let id = <[u8; NS_ID_V0_SIZE]>::try_from(&result[0..NS_ID_V0_SIZE])
        .expect("must not fail as hash is always 32 bytes and NS_ID_V0_SIZE < 32");
    Namespace::const_v0(id)
}

fn hash_json_serialized_bytes<T: Serialize>(val: &T) -> [u8; 32] {
    let mut hasher = Sha256::new();
    let bytes = serde_json::to_vec(val).expect(
        "should not fail because all types called with this do not contain maps and hence \
         non-unicode keys that would trigger to_vec()'s only error case",
    );
    hasher.update(bytes);
    hasher.finalize().into()
}

/// A wrapper of some abstract namespace data together with the public key
/// derived from the signing key that created the signature.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(try_from = "UnverifiedSignedNamespaceData::<D>")]
#[serde(into = "UnverifiedSignedNamespaceData::<D>")]
#[serde(bound = "D: Clone + DeserializeOwned + Serialize")]
pub struct SignedNamespaceData<D> {
    data: D,
    public_key: VerificationKey,
    signature: Signature,
}

impl<D> SignedNamespaceData<D> {
    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn public_key(&self) -> VerificationKey {
        self.public_key
    }

    pub fn signature(&self) -> Signature {
        self.signature
    }

    fn from_unverified_unchecked(unverified: UnverifiedSignedNamespaceData<D>) -> Self {
        let UnverifiedSignedNamespaceData {
            data,
            public_key,
            signature,
        } = unverified;
        Self {
            data,
            public_key,
            signature,
        }
    }

    pub fn into_unverified(self) -> UnverifiedSignedNamespaceData<D> {
        let Self {
            data,
            public_key,
            signature,
        } = self;
        UnverifiedSignedNamespaceData {
            data,
            public_key,
            signature,
        }
    }
}

impl<D: Serialize> SignedNamespaceData<D> {
    /// Constructs a `SignedNamespaceData` by signing the sha256 hashed JSON serialization
    /// of `data`.
    ///
    /// NOTE: Json is unstable under serialization. This must be replaced ASAP.
    pub fn from_data_and_key(data: D, key: &SigningKey) -> Self {
        let hash = hash_json_serialized_bytes(&data);
        let signature = key.sign(&hash);
        let public_key = key.verification_key();
        SignedNamespaceData {
            data,
            public_key,
            signature,
        }
    }

    /// Creates a signed namespace data object from an unsigned one.
    ///
    /// # Errors
    /// Returns an error if `unverified.public_key` cannot verify the
    /// the sha256 hash of the serialized and sha256-hashed `unverified.data`
    /// against `unverified.signature`.
    pub fn try_from_unverified(
        unverified: UnverifiedSignedNamespaceData<D>,
    ) -> Result<Self, SignedNamespaceDataVerification> {
        let UnverifiedSignedNamespaceData {
            data,
            public_key,
            signature,
        } = &unverified;
        let data_bytes = hash_json_serialized_bytes(data);
        public_key
            .verify(signature, &data_bytes)
            .map_err(SignedNamespaceDataVerification::Verification)?;
        Ok(Self::from_unverified_unchecked(unverified))
    }
}

impl<D: Serialize> TryFrom<UnverifiedSignedNamespaceData<D>> for SignedNamespaceData<D> {
    type Error = SignedNamespaceDataVerification;

    fn try_from(unverified: UnverifiedSignedNamespaceData<D>) -> Result<Self, Self::Error> {
        Self::try_from_unverified(unverified)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignedNamespaceDataVerification {
    #[error("failed verifying the serialized data with the given verification key and signature")]
    Verification(#[source] ed25519_consensus::Error),
}

/// A shadow of `SignedNamespaceData` whose data has not been verified against its
/// contained pubic key and signature.
///
/// This type's primary use is for serialization and deserialization.
/// Use [`Self::try_into_verified`] to verify its data and obtain a [`SignedNamespaceData`].
#[derive(Serialize, Deserialize, Debug)]
pub struct UnverifiedSignedNamespaceData<D> {
    pub data: D,
    #[serde(with = "crate::blob_space::_serde::verification_key")]
    pub public_key: VerificationKey,
    #[serde(with = "crate::blob_space::_serde::signature")]
    pub signature: Signature,
}

impl<D> UnverifiedSignedNamespaceData<D> {
    fn from_verified(verified: SignedNamespaceData<D>) -> Self {
        let SignedNamespaceData {
            data,
            public_key,
            signature,
        } = verified;
        Self {
            data,
            public_key,
            signature,
        }
    }
}

impl<D: Serialize> UnverifiedSignedNamespaceData<D> {
    /// Convert `Self` to [`SignedNamespaceData`] by verifying it.
    ///
    /// # Errors
    /// Refer to [`SignedNamespaceData::try_from_unverified`] for error conditions.
    pub fn try_into_verified(
        self,
    ) -> Result<SignedNamespaceData<D>, SignedNamespaceDataVerification> {
        SignedNamespaceData::try_from_unverified(self)
    }
}

impl<D> From<SignedNamespaceData<D>> for UnverifiedSignedNamespaceData<D> {
    fn from(verified: SignedNamespaceData<D>) -> Self {
        Self::from_verified(verified)
    }
}

/// Data that is serialized and submitted to celestia as a blob under the sequencer namespace.
///
/// It contains all the other chain IDs (and thus, namespaces) that were also written to in the same
/// block.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SequencerNamespaceData {
    pub block_hash: Hash,
    pub header: Header,
    pub rollup_chain_ids: Vec<ChainId>,
    pub action_tree_root: [u8; 32],
    pub action_tree_root_inclusion_proof: InclusionProof,
    pub chain_ids_commitment: [u8; 32],
    pub chain_ids_commitment_inclusion_proof: InclusionProof,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "failed to verify the rollup transactions and inclusion proof contained in the celestia blob \
     against the provided root hash"
)]
pub struct RollupVerificationFailure {
    #[from]
    source: sequencer_validation::VerificationFailure,
}

/// Data that is serialized and submitted to celestia as a blob under rollup-specific namespaces.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RollupNamespaceData {
    pub block_hash: Hash,
    pub chain_id: ChainId,
    pub rollup_txs: Vec<Vec<u8>>,
    pub inclusion_proof: InclusionProof,
}

impl RollupNamespaceData {
    /// Verifies `self.inclusion_proof` given the chain ID and the root of the merkle tree
    /// constructed from `self.rollup_txs` and the provided `root_hash`.
    ///
    /// # Errors
    /// Returns an error if the inclusion proof could not be verified.
    pub fn verify_inclusion_proof(
        &self,
        root_hash: [u8; 32],
    ) -> Result<(), RollupVerificationFailure> {
        use sequencer_validation::MerkleTree;
        let rollup_data_tree = MerkleTree::from_leaves(self.rollup_txs.clone());
        let rollup_data_root = rollup_data_tree.root();
        let mut leaf = self.chain_id.as_ref().to_vec();
        leaf.append(&mut rollup_data_root.to_vec());
        self.inclusion_proof.verify(&leaf, root_hash)?;
        Ok(())
    }
}

mod _serde {
    use base64_serde::base64_serde_type;
    base64_serde_type!(Base64Standard, base64::engine::general_purpose::STANDARD);

    pub(super) mod signature {
        use ed25519_consensus::Signature;
        use serde::{
            de::Error as _,
            Deserializer,
            Serializer,
        };

        use super::Base64Standard;

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
            deser: D,
        ) -> Result<Signature, D::Error> {
            let bytes: Vec<u8> = Base64Standard::deserialize(deser)?;
            let signature = Signature::try_from(&*bytes).map_err(|err| {
                D::Error::custom(format!(
                    "failed constructing verification key from bytes: {err:?}"
                ))
            })?;
            Ok(signature)
        }

        pub(crate) fn serialize<S: Serializer>(
            signature: &Signature,
            ser: S,
        ) -> Result<S::Ok, S::Error> {
            Base64Standard::serialize(signature.to_bytes(), ser)
        }
    }

    pub(super) mod verification_key {
        use ed25519_consensus::VerificationKey;
        use serde::{
            de::Error as _,
            Deserializer,
            Serializer,
        };

        use super::Base64Standard;

        pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
            deser: D,
        ) -> Result<VerificationKey, D::Error> {
            let bytes: Vec<u8> = Base64Standard::deserialize(deser)?;
            let key = VerificationKey::try_from(&*bytes).map_err(|err| {
                D::Error::custom(format!(
                    "failed constructing verification key from bytes: {err:?}"
                ))
            })?;
            Ok(key)
        }

        pub(crate) fn serialize<S: Serializer>(
            key: &VerificationKey,
            ser: S,
        ) -> Result<S::Ok, S::Error> {
            Base64Standard::serialize(key.as_bytes(), ser)
        }
    }
}
