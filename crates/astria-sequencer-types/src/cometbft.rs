//! Cometbft specific utilities.

/// Constructs a `[merkle::Tree]` from an iterator yielding byte slices.
///
/// This hashes each item before pushing it into the Merkle Tree, which
/// effectively causes a double hashing. The leaf hash of an item `d_i`
/// is then `MTH(d_i) = SHA256(0x00 || SHA256(d_i))`.
pub fn merkle_tree_from_transactions<I, B>(iter: I) -> merkle::Tree
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    use sha2::{
        Digest as _,
        Sha256,
    };
    merkle::Tree::from_leaves(iter.into_iter().map(|item| Sha256::digest(&item)))
}
