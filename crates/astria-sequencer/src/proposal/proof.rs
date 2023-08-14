use ct_merkle::{
    inclusion::InclusionProof as CtInclusionProof,
    RootHash,
};
use sha2::Sha256;
use tendermint::Hash;

/// A merkle proof of inclusion.
pub(crate) struct InclusionProof {
    // value to be proven as included
    value: Vec<u8>,
    // leaf index of value to be proven
    idx: usize,
    // total number of leaves in the tree
    num_leaves: usize,
    // the merkle proof itself
    inclusion_proof: CtInclusionProof<Sha256>,
}

impl InclusionProof {
    /// Verify that the proof is valid for the given root hash.
    pub(crate) fn verify(&self, root_hash: Hash) -> anyhow::Result<()> {
        let digest = sha2::digest::Output::<Sha256>::from_slice(root_hash.as_bytes()).to_owned();
        let ct_root = RootHash::<Sha256>::new(digest, self.num_leaves);
        ct_root
            .verify_inclusion(&self.value, self.idx, &self.inclusion_proof)
            .map_err(|e| anyhow::anyhow!("failed to verify inclusion: {}", e))
    }
}

#[cfg(test)]
mod test {
    use ct_merkle::{
        self,
        CtMerkleTree,
    };
    use tendermint::merkle::simple_hash_from_byte_vectors;

    #[test]
    fn ct_merkle_vs_tendermint() {
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

        let mut ct_tree: CtMerkleTree<sha2::Sha256, Vec<u8>> = ct_merkle::CtMerkleTree::new();
        data.iter().for_each(|d| {
            ct_tree.push(d.to_vec());
        });
        let ct_root = ct_tree.root();
        assert_eq!(ct_root.as_bytes().as_slice(), tm_root.as_slice());
    }
}
