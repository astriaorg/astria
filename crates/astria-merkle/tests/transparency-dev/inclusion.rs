use hex_literal::hex;

use super::make_tree_given_num_leaves;

struct InclusionTest {
    num_leaves: usize,
    leaf: usize,
    expected_proof: &'static [u8],
}

const INCLUSION_TESTS: &[InclusionTest] = &[
    InclusionTest {
        num_leaves: 1,
        leaf: 0,
        expected_proof: &[],
    },
    InclusionTest {
        num_leaves: 2,
        leaf: 0,
        expected_proof: &hex!("96a296d224f285c67bee93c30f8a309157f0daa35dc5b87e410b78630a09cfc7"),
    },
    InclusionTest {
        num_leaves: 2,
        leaf: 1,
        expected_proof: &hex!("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"),
    },
    InclusionTest {
        num_leaves: 3,
        leaf: 2,
        expected_proof: &hex!("fac54203e7cc696cf0dfcb42c92a1d9dbaf70ad9e621f4bd8d98662f00e3c125"),
    },
    InclusionTest {
        num_leaves: 5,
        leaf: 1,
        expected_proof: &hex!(
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d\
            5f083f0a1a33ca076a95279832580db3e0ef4584bdff1f54c8a360f50de3031e\
            bc1a0643b12e4d2d7c77918f44e0f4f79a838b6cf9ec5b5c283e1f4d88599e6b"
        ),
    },
    InclusionTest {
        num_leaves: 8,
        leaf: 0,
        expected_proof: &hex!(
            "96a296d224f285c67bee93c30f8a309157f0daa35dc5b87e410b78630a09cfc7\
            5f083f0a1a33ca076a95279832580db3e0ef4584bdff1f54c8a360f50de3031e\
            6b47aaf29ee3c2af9af889bc1fb9254dabd31177f16232dd6aab035ca39bf6e4"
        ),
    },
    InclusionTest {
        num_leaves: 8,
        leaf: 5,
        expected_proof: &hex!(
            "bc1a0643b12e4d2d7c77918f44e0f4f79a838b6cf9ec5b5c283e1f4d88599e6b\
            ca854ea128ed050b41b35ffc1b87b8eb2bde461e9e3b5596ece6b9d5975a0ae0\
            d37ee418976dd95753c1c73862b9398fa2a2cf9b4ff0fdfe8b30cd95209614b7"
        ),
    },
];

#[track_caller]
fn construct_tree_and_assert_expected_proof(num_leaves: usize, i: usize, expected_proof: &[u8]) {
    let tree = make_tree_given_num_leaves(num_leaves);
    let actual_proof = tree.construct_proof(i).unwrap();
    assert_eq!(expected_proof, actual_proof.audit_path());
}

#[test]
fn proofs_from_generated_trees_match_known_proofs() {
    for case in INCLUSION_TESTS {
        let InclusionTest {
            num_leaves,
            leaf,
            expected_proof,
        } = case;
        construct_tree_and_assert_expected_proof(*num_leaves, *leaf, expected_proof);
    }
}
