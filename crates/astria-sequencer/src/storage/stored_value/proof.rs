use std::{
    borrow::Cow,
    num::NonZeroUsize,
};

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use merkle::{
    audit::UncheckedProof,
    Proof as DomainProof,
};

use super::StoredValue;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(crate) struct Proof<'a> {
    audit_path: Cow<'a, [u8]>,
    leaf_index: usize,
    tree_size: NonZeroUsize,
}

impl<'a> From<&'a DomainProof> for Proof<'a> {
    fn from(proof: &'a DomainProof) -> Self {
        Proof {
            audit_path: Cow::Borrowed(proof.audit_path()),
            leaf_index: proof.leaf_index(),
            tree_size: proof.tree_size(),
        }
    }
}

impl<'a> From<Proof<'a>> for DomainProof {
    fn from(proof: Proof<'a>) -> Self {
        DomainProof::unchecked_from_parts(UncheckedProof {
            audit_path: proof.audit_path.into_owned(),
            leaf_index: proof.leaf_index,
            tree_size: proof.tree_size.get(),
        })
    }
}

impl<'a> TryFrom<StoredValue<'a>> for Proof<'a> {
    type Error = anyhow::Error;

    fn try_from(value: StoredValue<'a>) -> Result<Self, Self::Error> {
        let StoredValue::Proof(proof) = value else {
            return Err(super::type_mismatch("proof", &value));
        };
        Ok(proof)
    }
}
