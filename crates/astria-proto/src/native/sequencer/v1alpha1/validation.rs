pub use sequencer_validation::{
    ct_merkle::{
        self,
        error::InclusionVerifError,
    },
    InclusionProof,
};

use crate::{
    generated::sequencer::v1alpha1 as raw,
    native::Protobuf,
};

#[derive(Debug, thiserror::Error)]
pub enum InclusionProofError {
    #[error("failed reconstructing inclusion proof from bytes returned in raw protobuf")]
    ProofFromBytes(#[source] InclusionVerifError),
}

impl Protobuf for InclusionProof {
    type Error = InclusionProofError;
    type Raw = raw::InclusionProof;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            index,
            num_leaves,
            inclusion_proof,
        } = raw;
        let index: usize = (*index).try_into().expect(
            "a u64 index should always fit into a usize if run on a 64 bit machine; is this \
             running on a 64 bit machine?",
        );
        let num_leaves: usize = (*num_leaves).try_into().expect(
            "a u64 num_leaves should always fit into a usize if run on a 64 bit machine; is this \
             running on a 64 bit machine?",
        );
        let inclusion_proof =
            ct_merkle::inclusion::InclusionProof::try_from_bytes(inclusion_proof.clone())
                .map_err(Self::Error::ProofFromBytes)?;
        Ok(Self::builder()
            .index(index)
            .num_leaves(num_leaves)
            .inclusion_proof(inclusion_proof)
            .build())
    }

    fn to_raw(&self) -> Self::Raw {
        let (index, num_leaves, inclusion_proof) = self.clone().into_parts();
        let index: u64 = (index).try_into().expect(
            "a usize index should always fit into a u64 if run on a 64 bit machine unless this is \
             running on an u128 machine; is this running on a 64 bit machine?",
        );
        let num_leaves: u64 = (num_leaves).try_into().expect(
            "a usize num_leaves should always fit into a u64 if run on a 64 bit machine unless \
             this is running on an u128 machine; is this running on a 64 bit machine?",
        );
        Self::Raw {
            index,
            num_leaves,
            inclusion_proof: inclusion_proof.as_bytes().to_vec(),
        }
    }
}
