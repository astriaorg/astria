use super::{
    is_perfect,
    perfect_root,
};
use crate::{
    complete_parent,
    complete_parent_and_sibling,
};

#[track_caller]
fn assert_is_perfect(n: usize) {
    assert!(is_perfect(n));
}

#[test]
fn all_perfect_trees_are_found() {
    assert_is_perfect((1 << 1) - 1);
    assert_is_perfect((1 << 2) - 1);
    assert_is_perfect((1 << 3) - 1);
    assert_is_perfect((1 << 4) - 1);
    assert_is_perfect((1 << 5) - 1);
    assert_is_perfect((1 << 6) - 1);
    assert_is_perfect((1 << 7) - 1);
    assert_is_perfect((1 << 8) - 1);
    assert_is_perfect((1 << 9) - 1);
    assert_is_perfect((1 << 10) - 1);
    assert_is_perfect((1 << 11) - 1);
    assert_is_perfect((1 << 12) - 1);
    assert_is_perfect((1 << 13) - 1);
    assert_is_perfect((1 << 14) - 1);
    assert_is_perfect((1 << 15) - 1);
    assert_is_perfect((1 << 16) - 1);
    assert_is_perfect((1 << 17) - 1);
    assert_is_perfect((1 << 18) - 1);
    assert_is_perfect((1 << 19) - 1);
    assert_is_perfect((1 << 20) - 1);
    assert_is_perfect((1 << 21) - 1);
    assert_is_perfect((1 << 22) - 1);
    assert_is_perfect((1 << 23) - 1);
    assert_is_perfect((1 << 24) - 1);
    assert_is_perfect((1 << 25) - 1);
    assert_is_perfect((1 << 26) - 1);
    assert_is_perfect((1 << 27) - 1);
    assert_is_perfect((1 << 28) - 1);
    assert_is_perfect((1 << 29) - 1);
    assert_is_perfect((1 << 30) - 1);
    assert_is_perfect((1 << 31) - 1);
    assert_is_perfect((1 << 32) - 1);
    assert_is_perfect((1 << 33) - 1);
    assert_is_perfect((1 << 34) - 1);
    assert_is_perfect((1 << 35) - 1);
    assert_is_perfect((1 << 36) - 1);
    assert_is_perfect((1 << 37) - 1);
    assert_is_perfect((1 << 38) - 1);
    assert_is_perfect((1 << 39) - 1);
    assert_is_perfect((1 << 40) - 1);
    assert_is_perfect((1 << 41) - 1);
    assert_is_perfect((1 << 42) - 1);
    assert_is_perfect((1 << 43) - 1);
    assert_is_perfect((1 << 44) - 1);
    assert_is_perfect((1 << 45) - 1);
    assert_is_perfect((1 << 46) - 1);
    assert_is_perfect((1 << 47) - 1);
    assert_is_perfect((1 << 48) - 1);
    assert_is_perfect((1 << 49) - 1);
    assert_is_perfect((1 << 50) - 1);
    assert_is_perfect((1 << 51) - 1);
    assert_is_perfect((1 << 52) - 1);
    assert_is_perfect((1 << 53) - 1);
    assert_is_perfect((1 << 54) - 1);
    assert_is_perfect((1 << 55) - 1);
    assert_is_perfect((1 << 56) - 1);
    assert_is_perfect((1 << 57) - 1);
    assert_is_perfect((1 << 58) - 1);
    assert_is_perfect((1 << 59) - 1);
    assert_is_perfect((1 << 60) - 1);
    assert_is_perfect((1 << 61) - 1);
    assert_is_perfect((1 << 62) - 1);
    assert_is_perfect((1 << 63) - 1);
}

#[track_caller]
fn assert_perfect_root(n: usize, index: usize) {
    assert_eq!(index, perfect_root(n));
}

#[test]
fn root_indices_of_perfect_trees_are_correct() {
    assert_perfect_root(1, 0);
    assert_perfect_root(3, 1);
    assert_perfect_root(7, 3);
}

#[test]
fn last_set_bit_is_correct() {
    let x = 0b1011_0100;
    assert_eq!(0b100, super::last_set_bit(x));
}

#[test]
fn last_zero_bit_is_correct() {
    let x = 0b1011_0011;
    assert_eq!(0b100, super::last_zero_bit(x));
}

#[test]
fn parent_is_as_expected() {
    // Assuming a tree with 8 leaves, giving a total of 15 nodes in the tree.
    // Leaf index 7 corresponds to an internal tree index of 14.
    // According to the indexing scheme, the parenet of the leaf at index 14
    // is at index 13.
    let leaf_idx = 7 * 2;
    let size = 15;
    let parent = complete_parent(leaf_idx, size);
    assert_eq!(13, parent);
}

#[test]
fn parent_and_sibling_are_as_expected() {
    // Assuming a tree with 8 leaves, giving a total of 15 nodes in the tree.
    // Leaf index 7 corresponds to an internal tree index of 14.
    // According to the indexing scheme, the sibling of leaf 7 is leaf 6 at index 12,
    // and their parent at 13.
    let leaf_idx = 7 * 2;
    let size = 15;
    let (parent, sibling) = complete_parent_and_sibling(leaf_idx, size);
    assert_eq!(13, parent);
    assert_eq!(12, sibling);
}
