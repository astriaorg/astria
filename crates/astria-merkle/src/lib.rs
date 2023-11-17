//! A RFC 6962 compliant merkle tree with a flat representation.
//!
//! This Merkle tree avoids unnecessary allocations by pushing all node hashes
//! into a byte buffer of 32-byte sha256 hashes. This crate also provides the
//! `LeafBuilder` and `AuditLeafBuilder` APIs for constructing leaves ad-hoc
//! if they consist of the concatenation of many bytes so that they too need not
//! be pre-allocated.
//!
//! This library only supports sha256 hashing, and thus only 32 byte leaf hashes.
//! It also does not store the tree's leaves, only their hashes. RFC 6962 consistency
//! proofs are not yet implemented.
//!
//! # Usage and examples
//! Add this to your `Cargo.toml` dependencies (it is encouraged to use a fixed `rev` or `tag`
//! to ensure that a build does not randomly stop failing because upstream changed):
//! ```toml
//! [dependencies]
//! astria-merkle = { git = "https://github.com/astriaorg/astria/", rev = "<fixed-commit>" }
//! ```
//!
//! Then use it like so:
//! ```
//! use astria_merkle::Tree;
//! // Construct a tree from an iterable yielding byte slices
//! let mut tree = Tree::from_leaves(&[&[1; 32][..], &[4, 4, 4], b"helloworld"]);
//!
//! // Push a single leaf into the tree
//! tree.push(&[64; 32]);
//!
//! // Construct a tree from different sources
//! tree.build_leaf()
//!     .write(&[42; 1])
//!     .write(&[1, 1])
//!     .write(&vec![42; 3])
//!     .write(b"42");
//!
//! let root = tree.root();
//! let proof = tree
//!     .construct_proof(4)
//!     .expect("leaf 5 must be inside the tree");
//!
//! assert!(
//!     proof
//!         .audit()
//!         .with_root(root)
//!         .with_leaf_builder()
//!         .write(&[42; 1])
//!         .write(&[1, 1])
//!         .write(&vec![42; 3])
//!         .write(b"42")
//!         .finish_leaf()
//!         .perform()
//! );
//! ```
//!
//! # Indexing scheme
//! The in-memory representation alternates between leaf nodes and branch nodes
//! with a indexing shown below (the node with index 3 is the root).
//! ```text
//! 0
//!   1
//! 2
//!    3
//! 4
//!   5
//! 6
//! ```
//!
//! # Indexing
//! All indexing into the merkle tree rely on the following observation:
//!
//! A tree of depth 1 has the following addressing scheme:
//! ```text
//! 0       00
//!   1     01
//! 2       10
//! ```
//! And a tree of depth 2
//! ```text
//! 0      000
//!   1    001
//! 2      010
//!    3   011
//! 4      100
//!   5    101
//! 6      110
//! ```
//! And a subtree of depth 3
//! ```text
//! 0        0000
//!    1     0001  <-- depth of subtree at index 1: 1
//! 2        0010
//!      3   0011  <-- depth of subtree at index 3: 2
//! 4        0100
//!    5     0101
//! 6        0110
//!        7 0111  <-- depth of subtree at root: 3
//! 8        1000
//!    9     1001
//! 10       1010
//!      11  1011  <-- depth of subtree at index 11: 2
//! 12       1100
//!    13    1101  <-- depth of subtree at index 13: 1
//! 14       1110
//! ```
//! For the general case of a tree of depth `D+1` we then observe:
//! ```text
//! 011...1: root at index 2^D - 1
//!  \____/
//!    D number of ones
//! \_____/
//!   (D+1)-tree
//!
//! 0xxxxxx:
//!  \____/
//!    D-tree to the left of the root
//!
//! 1xxxxxx:
//!  \____/
//!    D-tree to the right of the root
//! ```
//! See [flat in order trees](https://www.ietf.org/archive/id/draft-ietf-mls-protocol-14.html) for a full discussion and proof.
//!
//!
//! # Further reading:
//!
//! + RFC 6962: <https://datatracker.ietf.org/doc/html/rfc6962>
//! + RFC 7574 surpassing 6962: <https://datatracker.ietf.org/doc/rfc7574>
//! + Hypercore whitepaper introducing flat binary trees: <https://www.datprotocol.com/deps/0002-hypercore/>
//! + Traversing rachet trees: <https://www.ietf.org/archive/id/draft-ietf-mls-protocol-14.html>
//! + Flat in-order trees (the blog post this crate is based on): <https://mmapped.blog/posts/22-flat-in-order-trees>

use sha2::{
    Digest as _,
    Sha256,
};

pub mod audit;
#[cfg(test)]
mod tests;

pub use audit::{
    Audit,
    Proof,
};

/// Calculates `SHA256(0x00 | leaf)`
#[must_use]
pub fn hash_leaf(leaf: &[u8]) -> [u8; 32] {
    let mut hasher = init_leaf_hasher();
    hasher.update(leaf);
    hasher.finalize().into()
}

/// Calculates `SHA256(0x01 || left || right)`.
#[must_use]
pub fn combine(left: &[u8], right: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01_u8]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn init_leaf_hasher() -> Sha256 {
    let mut hasher = Sha256::new();
    hasher.update([0x00_u8]);
    hasher
}

/// A low-level API to construct a leaf-hash ad-hoc without needing to allocate it.
///
/// [`LeafBuilder`] can only be created by using the [`Tree::build_push_leaf`] method.
/// The leaf is built and the tree updated once `LeafBuilder` goes out of scope.
///
/// No two leaf builders can exist at the same time because a builder holds a mutable
/// reference to its tree during its lifetime.
///
/// See [`Tree::build_leaf`] for usage.
pub struct LeafBuilder<'a> {
    tree: &'a mut Tree,
    hasher: Option<Sha256>,
}

impl<'a> LeafBuilder<'a> {
    /// Takes ownership of the builder, dropping it.
    ///
    /// This method causes the leaf builder to go out of scope, causing it
    /// to be dropped and triggering an update of the merkle tree. Use this
    /// if the builder is explicitly bound.
    ///
    /// Usually calling this is not necessary because writes to the builder can
    /// be chained, but this is still useful if some other operation has to happen
    /// between writes, requiring the builder to be finalized explicitly.
    ///
    /// # Examples
    /// ```
    /// # use astria_merkle::Tree;
    /// let mut tree_1 = Tree::new();
    /// let mut builder = tree_1.build_leaf();
    /// builder.write(b"hello");
    /// builder.write(b"world");
    /// builder.finish();
    ///
    /// let mut tree_2 = Tree::new();
    /// tree_2.build_leaf().write(b"hello").write(b"world");
    ///
    /// let mut tree_3 = Tree::new();
    /// tree_3.push(b"helloworld");
    ///
    /// assert_eq!(tree_1.root(), tree_2.root());
    /// assert_eq!(tree_1.root(), tree_3.root());
    /// ```
    pub fn finish(self) {}

    /// Writes `bytes` into the builder, appending to the leaf.
    ///
    /// See [`Tree::build_leaf`] for example usage.
    #[allow(clippy::missing_panics_doc)] // invariant of the system
    pub fn write(&mut self, bytes: &[u8]) -> &mut Self {
        let hasher = self
            .hasher
            .as_mut()
            .expect("hasher is set during the lifetime of the leaf builder");
        hasher.update(bytes);
        self
    }
}

impl<'a> Drop for LeafBuilder<'a> {
    fn drop(&mut self) {
        let Self {
            tree,
            hasher,
        } = self;
        let leaf_hash: [u8; 32] = hasher
            .take()
            .expect("hasher is set during the leaf builder's lifetime and only taken on drop")
            .finalize()
            .into();
        if tree.nodes.is_empty() {
            tree.nodes.extend_from_slice(&leaf_hash);
            return;
        }
        // append 2 * 32 = 64 zeros
        tree.nodes.extend_from_slice(&[0; 64]);
        let size = tree.len();
        tree.set_node(size - 1, leaf_hash);
        let mut idx = tree.len() - 1;
        let root = complete_root(tree.len());
        loop {
            idx = complete_parent(idx, size);
            let left = complete_left_child(idx);
            let right = complete_right_child(idx, size);
            let new_value = tree.combine_nodes(left, right);
            tree.set_node(idx, new_value);
            if idx == root {
                break;
            }
        }
    }
}

/// An append-only Merkle tree with a flat binary representation.
pub struct Tree {
    nodes: Vec<u8>,
}

impl Tree {
    /// Calculates `SHA256(0x01 || MHT_i || MHT_j)`, where
    /// `MHT_i` is merkle tree hash of the i-th node.
    fn combine_nodes(&self, i: usize, j: usize) -> [u8; 32] {
        let left = self.get_node(i);
        let right = self.get_node(j);
        combine(&left, &right)
    }

    /// Returns the hash for a node at index `i`.
    ///
    /// # Panics
    /// Panics if `i` is outside the tree, i.e. if `i >= self.len()`.
    #[inline]
    fn get_node(&self, i: usize) -> [u8; 32] {
        assert!(self.is_in_tree(i));
        self.nodes[i * 32..(i + 1) * 32].try_into().unwrap()
    }

    /// Returns `true` if the index `i` falls inside the Merkle tree.
    #[inline]
    fn is_in_tree(&self, i: usize) -> bool {
        i < self.len()
    }

    /// Assigns `val` to the node at index `i`.
    ///
    /// # Panics
    /// Panics if `i >= self.len()`.
    #[inline]
    fn set_node(&mut self, i: usize, val: [u8; 32]) {
        assert!(self.is_in_tree(i));
        self.nodes[i * 32..(i + 1) * 32].copy_from_slice(&val);
    }

    /// Constructs the inclusion proof for the i-th leaf of the tree.
    ///
    /// Returns `None` if `i` is outside the tree.
    ///
    /// # Examples
    /// Constructing a proof for a tree without leaves returns no proof (as leaf `0` is not
    /// inside the tree), while a tree with a single leaf returns an empty proof:
    /// ```
    /// # use astria_merkle::Tree;
    /// let tree = Tree::new();
    /// assert!(tree.construct_proof(0).is_none());
    ///
    /// let mut tree = Tree::new();
    /// tree.push(&[1u8]);
    /// let proof = tree.construct_proof(0).expect("leaf 0 is inside the tree");
    /// assert!(proof.is_empty());
    /// ```
    /// A proof for a perfect tree of 8 leaves:
    /// ```
    /// # use astria_merkle::Tree;
    /// let mut tree = Tree::new();
    /// tree.push(&[1; 32]);
    /// tree.push(&[2; 32]);
    /// tree.push(&[3; 32]);
    /// tree.push(&[4; 32]);
    /// tree.push(&[5; 32]);
    /// tree.push(&[6; 32]);
    /// tree.push(&[7; 32]);
    /// tree.push(&[8; 32]);
    /// let proof = tree
    ///     .construct_proof(7)
    ///     .expect("leaf 7 must be inside the tree");
    /// assert_eq!(3, proof.len());
    /// ```
    #[must_use]
    pub fn construct_proof(&self, leaf_index: usize) -> Option<Proof> {
        let mut tree_index = leaf_index_to_tree_index(leaf_index);
        if !self.is_in_tree(leaf_index) {
            return None;
        }
        let mut audit_path = Vec::new();
        let tree_size = self.len();
        let root = complete_root(tree_size);
        while tree_index != root {
            let sibling;
            (tree_index, sibling) = complete_parent_and_sibling(tree_index, tree_size);
            audit_path.extend_from_slice(&self.get_node(sibling));
        }
        Some(Proof {
            audit_path,
            leaf_index,
            tree_size,
        })
    }

    /// Returns `MTH_i`, the merkle tree hash of the i-th leaf.
    ///
    /// Returns `None` if `i` falls outside the tree.
    ///
    /// # Examples
    /// ```
    /// use astria_merkle::{
    ///     combine,
    ///     hash_leaf,
    ///     Tree,
    /// };
    /// let mut tree = Tree::new();
    /// tree.push(&[1; 32]);
    /// tree.push(&[2; 32]);
    /// assert_eq!(Some(hash_leaf(&[1; 32])), tree.leaf(0));
    /// assert_eq!(Some(hash_leaf(&[2; 32])), tree.leaf(1));
    /// assert!(tree.leaf(2).is_none());
    /// ```
    #[must_use]
    pub fn leaf(&self, i: usize) -> Option<[u8; 32]> {
        let idx = leaf_index_to_tree_index(i);
        self.is_in_tree(idx).then(|| self.get_node(idx))
    }

    /// Returns the root hash of the Merkle tree.
    ///
    /// If the tree is empty then the root is defined as the hash of the emptry
    /// string, i.e. `MTH({}) = Sha256()`.
    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        if self.is_empty() {
            Sha256::digest(b"").into()
        } else {
            self.get_node(complete_root(self.len()))
        }
    }

    /// Creates a new, empty merkle tree.
    ///
    /// # Examples
    /// ```
    /// # use astria_merkle::Tree;
    /// let tree = Tree::new();
    /// assert_eq!(0, tree.len());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }

    /// Returns the number of nodes in the merkle tree.
    ///
    /// # Examples
    /// ```
    /// # use astria_merkle::Tree;
    /// let mut tree = Tree::new();
    /// tree.push(&[1; 32]);
    /// assert_eq!(1, tree.len());
    /// // Pushing a second leaf will also insert a parent to hold their combined hash.
    /// tree.push(&[2; 32]);
    /// assert_eq!(3, tree.len());
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len() / 32
    }

    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Build a leaf hash ad-hoc by writing its bytes into a [`LeafBuilder`].
    ///
    /// The leaf is added to the tree when [`LeafBuilder`] is dropped.
    ///
    /// If a leaf itself is build up from several byte slices, this method allows
    /// adding a leaf hash to the merkle tree without having to allocate the leaf first.
    ///
    /// # Examples
    /// ```
    /// # use astria_merkle::Tree;
    /// let mut tree_1 = Tree::new();
    /// // The `LeafBuilder` returned by `Tree::build_leaf` is never assigned, so it
    /// // goes out of scope at the end of the statement, updating the tree on drop.
    /// tree_1.build_leaf().write(b"hello").write(b"world");
    ///
    /// let mut leaf = Vec::new();
    /// leaf.extend_from_slice(b"hello");
    /// leaf.extend_from_slice(b"world");
    /// let mut tree_2 = Tree::new();
    /// tree_2.push(&leaf);
    ///
    /// assert_eq!(tree_1.root(), tree_2.root());
    /// ```
    pub fn build_leaf(&mut self) -> LeafBuilder<'_> {
        let hasher = init_leaf_hasher();
        LeafBuilder {
            tree: self,
            hasher: Some(hasher),
        }
    }

    /// Pushes a new leaf into the tree.
    pub fn push(&mut self, leaf: &[u8]) {
        self.build_leaf().write(leaf);
    }

    /// Constructs a Merkle tree from an iterator yielding byte slices.
    ///
    /// This is a utility function to loop over an iterator and pushing
    /// each item into the tree.
    pub fn from_leaves<I, B>(iter: I) -> Self
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        let mut tree = Self::new();
        for item in iter {
            tree.push(item.as_ref());
        }
        tree
    }
}

impl Default for Tree {
    fn default() -> Self {
        Tree::new()
    }
}

/// Calculates the index `i` of the j-th leaf in the tree.
///
/// Since leaves are always indexed with even numbers and branches with
/// odd numbers, this is just the formula `i = 2 * j`.
#[inline]
fn leaf_index_to_tree_index(j: usize) -> usize {
    j * 2
}

/// Isolates last set bit of an unsigned integer `x` as a mask.
#[inline]
fn last_set_bit(x: usize) -> usize {
    x - ((x - 1) & x)
}

/// Isolatest the last unset bit of an unsigned integer `x` as a mask.
#[inline]
fn last_zero_bit(x: usize) -> usize {
    last_set_bit(x + 1)
}

/// Returns the parent index of a node at index `i` in a perfect binary tree.
///
/// Following the indexing scheme, this sets the last unset bit in `i` to 1,
/// and the next most significant bit to 0:
///
/// ```text
///                .-last zero bit
///               /
/// i   = y y y y x 0 1 1 ... 1  <--- node at index i
/// p   = y y y y 0 1 1 1 ... 1  <--- parent of i with x zeroed out.
///
/// Operations:
///       0 0 0 0 1 0 0 0 ... 0  <--- last_zero_bit(i)
///       y y y y x 1 1 1 ... 1  <--- last_zero_bit(i) | i
///       1 1 1 1 0 1 1 1 ... 1  <--- last_zero_bit(i) << 1
///       y y y y 0 1 1 1 ... 1  <--- (last_zero_bit(i) | i) & !(last_zero_bit(i) << 1)
/// ```
#[inline]
fn perfect_parent(i: usize) -> usize {
    let zero = last_zero_bit(i);
    (zero | i) & !(zero << 1)
}

/// Returns the left child index of a node at index `p` in a perfect binary tree.
///
/// Following the indexing scheme, this unsets the bit at position `k-1`
/// to the right of the last zero bit at `k`.
/// ```text
/// p   = y y y y 0 1 1 1 ... 1  <--- parent at index p
/// i   = y y y y 0 0 1 1 ... 1  <--- child at index i
///
/// Operations
///       0 0 0 0 1 0 0 0 ... 0  <--- last_zero_bit(p)
///       0 0 0 0 0 1 0 0 ... 0  <--- last_zero_bit(p) >> 1
///       1 1 1 1 1 0 1 1 ... 1  <--- !(last_zero_bit(p) >> 1)
/// i   = y y y y 0 0 1 1 ... 1  <--- p & !(last_zero_bit(p) >> 1)
/// ```
fn perfect_left_child(p: usize) -> usize {
    assert!(is_branch(p));
    p & !(last_zero_bit(p) >> 1)
}

/// Returns the right child index of a of a node at index `p` in a perfect binary tree.
///
/// Following the indexing scheme, in addition to unsetting the bit at position
/// `k-1` as for the left child, this also sets bit `k` to one.
/// ```text
/// p   = y y y y 0 1 1 1 ... 1  <--- parent at index p
/// i   = y y y y 1 0 1 1 ... 1  <--- child at index i
///
/// Operations
///       0 0 0 0 1 0 0 0 ... 0  <--- last_zero_bit(p)
///       0 0 0 0 0 1 0 0 ... 0  <--- last_zero_bit(p) >> 1
///       1 1 1 1 1 0 1 1 ... 1  <--- !(last_zero_bit(p) >> 1)
///       y y y y 1 1 1 1 ... 0  <--- p | last_zero_bit(p)
/// i   = y y y y 1 0 1 1 ... 1  <--- (p | last_zero_bit(p)) & !(last_zero_bit(p) >> 1)
/// ```
fn perfect_right_child(p: usize) -> usize {
    assert!(is_branch(p));
    (p | last_zero_bit(p)) & !(last_zero_bit(p) >> 1)
}

/// Returns the root index of a perfect binary tree.
///
/// Following the indexing scheme, the root is located at `n/2`.
fn perfect_root(n: usize) -> usize {
    assert!(is_perfect(n));
    n >> 1
}

/// Returns the root index of a complete binary tree of size `n`.
///
/// The root of a complete tree is the same as for the smallest perfect binary tree
/// with at least the same number of nodes.
///
/// Note that for the edge case of a perfect binary tree `n = 2^d - 1` for some depth
/// `d` we have `|(2^d - 1) + 1| - 1 = 2^d - 1`.
///
/// Note: `complete binary tree` here refers to a tree in which all left subtrees
///       are perfect, which is a stronger assumption than just "complete".
fn complete_root(n: usize) -> usize {
    perfect_root(n.wrapping_add(1).next_power_of_two().saturating_sub(1))
}

/// Returns the parent index of a node at index `i` in a complete binary tree of size `n`.
///
/// This functions views the tree as a perfect binary but where some nodes to
/// the right of the subtree are removed, and were the subtree is reattached.
/// This algorithm then walks the virtual tree until it hits an index  that falls
/// within the tree.
///
/// Note: `complete binary tree` here refers to a tree in which all left subtrees
///       are perfect, which is a stronger assumption than just "complete".
fn complete_parent(i: usize, n: usize) -> usize {
    let mut i = i;
    loop {
        i = perfect_parent(i);
        if i < n {
            break i;
        }
    }
}

/// Returns the left child index of a node at index `p` of a complete binary tree.
///
/// Note: `complete binary tree` here refers to a tree in which all left subtrees
///       are perfect, which is a stronger assumption than just "complete". Therefore
///       the left child of `p` is the same as for the perfect binary tree.
fn complete_left_child(p: usize) -> usize {
    perfect_left_child(p)
}

/// Returns the right child index of a node at index `p` of a complete binary tree.
///
/// Note: `complete binary tree` here refers to a tree in which all left subtrees
///       are perfect, which is a stronger assumption than just "complete". Therefore
///       the left child of `p` is the same as for the perfect binary tree.
fn complete_right_child(i: usize, n: usize) -> usize {
    assert!(is_branch(i));
    assert!(i < n);
    let right_child = perfect_right_child(i);
    if right_child < n {
        right_child
    } else {
        i + 1 + complete_root(n - i - 1)
    }
}

/// Returns the parent and sibling indices of a node at index `i` of a complete binary tree of size
/// `n`.
///
/// This is a utility to avoid calling `complete_parent` twice in `Tree::construct_proof`.
fn complete_parent_and_sibling(i: usize, n: usize) -> (usize, usize) {
    assert!(i < n);
    let p = complete_parent(i, n);
    let s = if i < p {
        complete_right_child(p, n)
    } else {
        complete_left_child(p)
    };
    (p, s)
}

/// Returns if a node `i` is a branch.
///
/// This relies on the fact that leaves are even-indexed, while
/// branches are odd-indexed.
#[inline]
fn is_branch(i: usize) -> bool {
    i & 0b1 == 1
}

/// Returns if a tree of size `n` is perfect.
#[inline]
fn is_perfect(n: usize) -> bool {
    n == 1 || n.next_power_of_two() == n.wrapping_add(1)
}
