use ct_merkle::{
    inclusion::InclusionProof as CtInclusionProof,
    CtMerkleTree,
    RootHash,
};
use serde::{
    Deserialize,
    Serialize,
};
use sha2::Sha256;
use tendermint::Hash;

/// A wrapper around [`ct_merkle::CtMerkleTree`], which uses sha256 as the hashing algorithm
/// and Vec<u8> as the leaf type.
#[derive(Debug, Default)]
pub struct MerkleTree(CtMerkleTree<Sha256, Vec<u8>>);

impl MerkleTree {
    #[must_use]
    pub fn from_leaves(leaves: Vec<Vec<u8>>) -> Self {
        let tree = leaves
            .into_iter()
            .fold(CtMerkleTree::new(), |mut tree, leaf| {
                tree.push(leaf);
                tree
            });
        MerkleTree(tree)
    }

    #[must_use]
    pub fn root(&self) -> Hash {
        let digest = sha2::digest::Output::<Sha256>::from_slice(self.0.root().as_bytes())
            .as_slice()
            .to_vec();
        Hash::from_bytes(tendermint::hash::Algorithm::Sha256, &digest)
            .expect("cannot fail since both hashes are 32 bytes")
    }

    /// Returns the inclusion proof for the leaf at the given index.
    ///
    /// # Errors
    ///
    /// - if the index is out of bounds
    pub fn prove_inclusion(&self, idx: usize) -> eyre::Result<InclusionProof> {
        Ok(InclusionProof {
            value: self
                .0
                .get(idx)
                .ok_or(eyre::eyre!("index out of bounds of merkle tree"))?
                .clone(),
            idx,
            num_leaves: self.0.len(),
            inclusion_proof: self.0.prove_inclusion(idx),
        })
    }
}

/// A merkle proof of inclusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct InclusionProof {
    // value to be proven as included
    value: Vec<u8>,
    // leaf index of value to be proven
    idx: usize,
    // total number of leaves in the tree
    num_leaves: usize,
    // the merkle proof itself
    inclusion_proof: CtInclusionProof<Sha256>,
}

impl PartialEq for InclusionProof {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.idx == other.idx
            && self.num_leaves == other.num_leaves
            && self.inclusion_proof.as_bytes() == other.inclusion_proof.as_bytes()
    }
}

impl Eq for InclusionProof {}

impl InclusionProof {
    /// Verify that the proof is valid for the given root hash.
    ///
    /// # Errors
    ///
    /// - if the proof is invalid
    pub fn verify(&self, root_hash: Hash) -> eyre::Result<()> {
        let digest = *sha2::digest::Output::<Sha256>::from_slice(root_hash.as_bytes());
        let ct_root = RootHash::<Sha256>::new(digest, self.num_leaves);
        ct_root
            .verify_inclusion(&self.value, self.idx, &self.inclusion_proof)
            .map_err(|e| eyre::eyre!("failed to verify inclusion: {}", e))
    }
}

#[cfg(test)]
mod test {
    use ct_merkle::{
        self,
        CtMerkleTree,
    };
    use tendermint::merkle::simple_hash_from_byte_vectors;

    use super::*;

    #[test]
    fn ct_merkle_vs_tendermint() {
        // assert that the ct-merkle library is compatible with tendermint
        let data: Vec<Vec<u8>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
            vec![10, 11, 12],
            vec![13, 14, 15],
            vec![16, 17, 18],
            vec![19, 20, 21],
            vec![22, 23, 24],
        ];

        let tm_root = simple_hash_from_byte_vectors::<tendermint::crypto::default::Sha256>(&data);
        let ct_tree = MerkleTree::from_leaves(data);
        let ct_root = ct_tree.root();
        assert_eq!(ct_root.as_bytes(), tm_root.as_slice());
    }

    #[test]
    fn inclusion_proof_serde_roundtrip() {
        let data: Vec<Vec<u8>> = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
            vec![10, 11, 12],
            vec![13, 14, 15],
            vec![16, 17, 18],
            vec![19, 20, 21],
            vec![22, 23, 24],
        ];

        let mut ct_tree: CtMerkleTree<sha2::Sha256, Vec<u8>> = ct_merkle::CtMerkleTree::new();
        for d in &data {
            ct_tree.push(d.clone());
        }
        let ct_root = ct_tree.root();

        let idx = 0;
        let proof = InclusionProof {
            value: data[idx].clone(),
            idx,
            num_leaves: data.len(),
            inclusion_proof: ct_tree.prove_inclusion(idx),
        };

        let json = serde_json::to_string(&proof).unwrap();
        let proof: InclusionProof = serde_json::from_str(&json).unwrap();
        let tm_hash =
            tendermint::Hash::from_bytes(tendermint::hash::Algorithm::Sha256, ct_root.as_bytes())
                .unwrap();
        proof.verify(tm_hash).unwrap();
    }
}
