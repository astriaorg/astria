//! Consistency and inclusion tests are taken and adjusted to Rust from
//! [transparency-dev/merkle](https://github.com/transparency-dev/merkle/blob/7d6ba6631786ace38b5ccf5b0cbb31bff5c70e25/testonly/reference_test.go).

use astria_merkle::Tree;
use hex_literal::hex;

const LEAF_INPUTS: &[&[u8]] = &[
    &hex!(""),
    &hex!("00"),
    &hex!("10"),
    &hex!("2021"),
    &hex!("3031"),
    &hex!("40414243"),
    &hex!("5051525354555657"),
    &hex!("606162636465666768696a6b6c6d6e6f"),
];

mod inclusion;

fn make_tree_given_num_leaves(n: usize) -> Tree {
    let mut tree = Tree::new();
    for leaf in &LEAF_INPUTS[0..n] {
        tree.push(leaf);
    }
    tree
}
