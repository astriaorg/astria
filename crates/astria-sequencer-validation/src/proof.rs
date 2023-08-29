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

/// A wrapper around [`ct_merkle::CtMerkleTree`], which uses sha256 as the hashing algorithm
/// and Vec<u8> as the leaf type.
///
/// # Examples
///
/// ```
/// use astria_sequencer_validation::MerkleTree;
///
/// let data: Vec<Vec<u8>> = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
///
/// let tree = MerkleTree::from_leaves(data);
/// let root = tree.root();
/// let inclusion_proof = tree.prove_inclusion(0).unwrap();
/// let value = vec![1, 2, 3];
/// inclusion_proof.verify(&value, root).unwrap();
/// ```
#[derive(Debug, Default)]
pub struct MerkleTree<T: AsRef<[u8]>>(CtMerkleTree<Sha256, T>);

impl<T: AsRef<[u8]>> MerkleTree<T> {
    /// Creates a new merkle tree from the given leaves.
    #[must_use]
    pub fn from_leaves(leaves: Vec<T>) -> Self {
        let tree = leaves
            .into_iter()
            .fold(CtMerkleTree::new(), |mut tree, leaf| {
                tree.push(leaf);
                tree
            });
        MerkleTree(tree)
    }

    /// Returns the root hash of the merkle tree as a fixed sized array of 32 bytes.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        (*self.0.root().as_bytes()).into()
    }

    /// Returns the inclusion proof for the leaf at the given index.
    ///
    /// # Errors
    ///
    /// - if the index is out of bounds
    pub fn prove_inclusion(&self, idx: usize) -> eyre::Result<InclusionProof> {
        Ok(InclusionProof {
            idx,
            num_leaves: self.0.len(),
            inclusion_proof: self.0.prove_inclusion(idx),
        })
    }
}

/// A merkle proof of inclusion.
///
/// See [`astria_sequencer_validation::MerkleTree`] for a usage example.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct InclusionProof {
    // leaf index of value to be proven
    idx: usize,
    // total number of leaves in the tree
    num_leaves: usize,
    // the merkle proof itself
    inclusion_proof: CtInclusionProof<Sha256>,
}

impl PartialEq for InclusionProof {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
            && self.num_leaves == other.num_leaves
            && self.inclusion_proof.as_bytes() == other.inclusion_proof.as_bytes()
    }
}

impl Eq for InclusionProof {}

impl InclusionProof {
    /// Verify that the merkle proof is valid for the given root hash and leaf value.
    ///
    /// # Errors
    ///
    /// - if the proof is invalid
    pub fn verify<T: AsRef<[u8]>>(&self, value: &[u8], root_hash: T) -> eyre::Result<()> {
        let digest = *sha2::digest::Output::<Sha256>::from_slice(root_hash.as_ref());
        let ct_root = RootHash::<Sha256>::new(digest, self.num_leaves);
        ct_root
            .verify_inclusion(&value, self.idx, &self.inclusion_proof)
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
    fn merkle_tree_snapshot() {
        // this is a "snapshot" test of the merkle tree.
        // if this test fails, it means the merkle tree is no longer backwards-compatible.
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

        let ct_tree = MerkleTree::from_leaves(data);
        let ct_root = ct_tree.root();
        let expected: [u8; 32] = [
            162, 149, 155, 23, 199, 181, 156, 228, 214, 166, 82, 156, 247, 210, 68, 204, 205, 97,
            8, 44, 132, 29, 172, 126, 208, 219, 21, 169, 19, 135, 55, 46,
        ];
        assert_eq!(ct_root, expected);
    }

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
        assert_eq!(ct_root, tm_root.as_slice());
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
        let value = data[idx].clone();
        let proof = InclusionProof {
            idx,
            num_leaves: data.len(),
            inclusion_proof: ct_tree.prove_inclusion(idx),
        };

        let json = serde_json::to_string(&proof).unwrap();
        let proof: InclusionProof = serde_json::from_str(&json).unwrap();
        let tm_hash =
            tendermint::Hash::from_bytes(tendermint::hash::Algorithm::Sha256, ct_root.as_bytes())
                .unwrap();
        proof.verify(&value, tm_hash).unwrap();
    }
}
