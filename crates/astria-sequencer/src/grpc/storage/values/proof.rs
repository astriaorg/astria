use std::{
    borrow::Cow,
    num::NonZeroUsize,
};

use astria_eyre::eyre::bail;
use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
use merkle::{
    audit::UncheckedProof,
    Proof as DomainProof,
};

use super::{
    Value,
    ValueImpl,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub(in crate::grpc) struct Proof<'a> {
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

impl<'a> From<Proof<'a>> for crate::storage::StoredValue<'a> {
    fn from(proof: Proof<'a>) -> Self {
        crate::storage::StoredValue::Grpc(Value(ValueImpl::Proof(proof)))
    }
}

impl<'a> TryFrom<crate::storage::StoredValue<'a>> for Proof<'a> {
    type Error = astria_eyre::eyre::Error;

    fn try_from(value: crate::storage::StoredValue<'a>) -> Result<Self, Self::Error> {
        let crate::storage::StoredValue::Grpc(Value(ValueImpl::Proof(proof))) = value else {
            bail!("grpc stored value type mismatch: expected proof, found {value:?}");
        };
        Ok(proof)
    }
}
