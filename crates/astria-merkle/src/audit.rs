//! Proving that a leaf is part of a tree.

use std::num::NonZeroUsize;

use sha2::{
    Digest as _,
    Sha256,
};

/// Builder to construct a complex leaf ad-hoc without needing to allocate it.
///
/// See `[Audit::with_leaf_builder]` for how to construct it.
pub struct LeafBuilder<'a, TLeaf, TRoot> {
    audit: Option<Audit<'a, TLeaf, TRoot>>,
    hasher: Option<Sha256>,
}

impl<'a, TLeaf, TRoot> LeafBuilder<'a, TLeaf, TRoot> {
    /// Finish constructing a leaf.
    ///
    /// Returns the internal [`Audit`] with its `TLeaf` typestate set.
    ///
    /// # Panics
    /// This method must only be called once. Calling it again will result
    /// in a panic.
    pub fn finish_leaf(&mut self) -> Audit<'a, WithLeafHash, TRoot> {
        let Audit {
            proof,
            root,
            ..
        } = self
            .audit
            .take()
            .expect("LeafBuilder::finish_leaf must not be used twice");
        let leaf_hash = self
            .hasher
            .take()
            .expect("LeafBuilder::finish_leaf must not be used twice")
            .finalize()
            .into();
        Audit {
            leaf_hash: WithLeafHash {
                leaf_hash,
            },
            proof,
            root,
        }
    }

    /// Write `bytes` into the leaf builder.
    ///
    /// # Panics
    /// This method must not be used after [`LeafBuilder::finish_leaf`] has been called
    /// and will panic otherwise.
    pub fn write(&mut self, bytes: &[u8]) -> &mut Self {
        self.hasher
            .as_mut()
            .expect("audit leaf builder must no be used after the leaf is finished")
            .update(bytes);
        self
    }
}

/// The default type-state for the `Audit` APIs leaf hash source.
pub struct NoLeafHash;

/// The default type-state for the `Audit` APIs root hash source.
pub struct NoRoot;

/// The type-state for the `Audit` API after seting a leaf hash.
pub struct WithLeafHash {
    leaf_hash: [u8; 32],
}

/// The type-state for the `Audit` API after setting a root hash.
pub struct WithRoot {
    root: [u8; 32],
}

/// The low level API to perform an audit on a leaf given a proof.
///
/// This type follows the type-state builder pattern.
pub struct Audit<'a, TLeaf = NoLeafHash, TRoot = NoRoot> {
    leaf_hash: TLeaf,
    proof: &'a Proof,
    root: TRoot,
}

impl<'a> Audit<'a> {
    /// Construct a new `Audit`.
    ///
    /// See [`Proof::audit`] for how to use this.
    fn new(proof: &'a Proof) -> Self {
        Self {
            leaf_hash: NoLeafHash,
            proof,
            root: NoRoot,
        }
    }
}

impl<'a, TLeaf, TRoot> Audit<'a, TLeaf, TRoot> {
    /// Construct an ad-hoc leaf using the [`LeafBuilder`] API.
    pub fn with_leaf_builder(self) -> LeafBuilder<'a, TLeaf, TRoot> {
        let hasher = crate::init_leaf_hasher();
        LeafBuilder {
            audit: Some(self),
            hasher: Some(hasher),
        }
    }

    /// Audit `leaf` by hashing it.
    ///
    /// Returns a new `Audit` with its `TLeaf` type-state to [`WithLeafHash`].
    pub fn with_leaf(self, leaf: &[u8]) -> Audit<'a, WithLeafHash, TRoot> {
        self.with_leaf_hash(crate::hash_leaf(leaf))
    }

    /// Audit `leaf` by directly using the provided `leaf_hash`.
    ///
    /// Returns a new `Audit` with its `TLeaf` type-state to [`WithLeafHash`].
    pub fn with_leaf_hash(self, leaf_hash: [u8; 32]) -> Audit<'a, WithLeafHash, TRoot> {
        let Self {
            proof,
            root,
            ..
        } = self;
        let leaf_hash = WithLeafHash {
            leaf_hash,
        };
        Audit {
            leaf_hash,
            proof,
            root,
        }
    }

    /// Perform an audit against the provided `root` hash.
    ///
    /// Returns a new `Audit` with its `TRoot` type-state to [`WithRoot`].
    pub fn with_root(self, root: [u8; 32]) -> Audit<'a, TLeaf, WithRoot> {
        let Self {
            proof,
            leaf_hash,
            ..
        } = self;
        Audit {
            leaf_hash,
            proof,
            root: WithRoot {
                root,
            },
        }
    }
}

impl<'a, TRoot> Audit<'a, WithLeafHash, TRoot> {
    /// Reconstruct the root hash using the leaf hash stored in the [`WithLeafHash`] state.
    ///
    /// # Examples
    /// ```
    /// let mut tree = astria_merkle::Tree::from_leaves(&[&[1][..], &[2, 2], &[3, 3]]);
    /// tree.build_leaf().write(&[4, 2]).write(b"answer");
    /// let root = tree.root();
    /// let proof = tree.construct_proof(3).expect("leaf 4 is inside the tree");
    /// let reconstructed_root = proof
    ///     .audit()
    ///     .with_leaf_builder()
    ///     .write(&[4, 2])
    ///     .write(b"answer")
    ///     .finish_leaf()
    ///     .reconstruct_root();
    /// assert_eq!(root, reconstructed_root);
    /// ```
    pub fn reconstruct_root(&self) -> [u8; 32] {
        let Self {
            leaf_hash: WithLeafHash {
                leaf_hash,
            },
            proof,
            ..
        } = self;
        proof.reconstruct_root_with_leaf_hash(*leaf_hash)
    }
}

impl<'a> Audit<'a, WithLeafHash, WithRoot> {
    /// Check if the leaf is included in the tree using the internal proof.
    ///
    /// This method reconstructs a Merkle tree root starting from the
    /// leaf hash stored in the [`WithLeafHash`] and audit's internal proof
    /// and returns if it matches the root hash stored in the `[WithRoot]`
    /// state.
    ///
    /// This method is useful if you need to construct a more complex leaf
    /// and want to avoid allocating a buffer for it. Prefer `[Proof::is_leaf_in_tree]`
    /// if the leaf is not complex or already allocated.
    ///
    /// # Examples
    /// ```
    /// let mut tree = astria_merkle::Tree::from_leaves(&[&[1][..], &[2, 2], &[3, 3]]);
    /// tree.build_leaf().write(&[4, 2]).write(b"answer");
    /// let root = tree.root();
    /// let proof = tree.construct_proof(3).expect("leaf 4 is inside the tree");
    /// assert!(
    ///     proof
    ///         .audit()
    ///         .with_root(root)
    ///         .with_leaf_builder()
    ///         .write(&[4, 2])
    ///         .write(b"answer")
    ///         .finish_leaf()
    ///         .perform()
    /// );
    /// ```
    #[must_use = "verify the audit result"]
    pub fn perform(&self) -> bool {
        let Self {
            leaf_hash: WithLeafHash {
                leaf_hash,
            },
            proof,
            root: WithRoot {
                root,
            },
        } = self;
        *root == proof.reconstruct_root_with_leaf_hash(*leaf_hash)
    }
}

#[derive(Debug)]
pub struct InvalidProof {
    kind: InvalidProofKind,
}

impl InvalidProof {
    fn audit_path_not_multiple_of_32(len: usize) -> Self {
        Self {
            kind: InvalidProofKind::AuditPathNotMultipleOf32 {
                len,
            },
        }
    }

    fn leaf_index_outside_tree(leaf_index: usize, tree_size: NonZeroUsize) -> Self {
        Self {
            kind: InvalidProofKind::LeafIndexOutsideTree {
                leaf_index,
                tree_size,
            },
        }
    }

    fn zero_tree_size() -> Self {
        Self {
            kind: InvalidProofKind::ZeroTreeSize,
        }
    }
}

impl std::fmt::Display for InvalidProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("the unchecked proof is not a valid proof")
    }
}

impl std::error::Error for InvalidProof {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

#[derive(Debug)]
enum InvalidProofKind {
    AuditPathNotMultipleOf32 {
        len: usize,
    },
    LeafIndexOutsideTree {
        leaf_index: usize,
        tree_size: NonZeroUsize,
    },
    ZeroTreeSize,
}

impl std::fmt::Display for InvalidProofKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidProofKind::AuditPathNotMultipleOf32 {
                len,
            } => f.write_fmt(format_args!(
                "audit path byte buffer length must be a multiple of 32 bytes, but was {len} bytes"
            )),
            InvalidProofKind::LeafIndexOutsideTree {
                leaf_index,
                tree_size,
            } => {
                let tree_index = crate::leaf_index_to_tree_index(*leaf_index);
                f.write_fmt(format_args!(
                    "leaf index {leaf_index} corresponding to tree index {tree_index} exceeds \
                     tree of size {tree_size}"
                ))
            }
            InvalidProofKind::ZeroTreeSize => f.pad("proof is undefined for trees of size zero"),
        }
    }
}

impl std::error::Error for InvalidProofKind {}

/// A builder pattern shadowing [`Proof`] with unchecked fields.
///
/// Mainly useful when serializing a [`Proof`].
///
/// # Examples
/// ```rust
/// use astria_merkle::Proof;
/// let proof = Proof::unchecked()
///     .audit_path(vec![42u8; 128])
///     .leaf_index(3)
///     .tree_size(15)
///     .try_into_proof()
///     .expect("is a valid proof");
/// ```
#[derive(Debug, Default)]
pub struct UncheckedProof {
    pub audit_path: Vec<u8>,
    pub leaf_index: usize,
    pub tree_size: usize,
}

impl UncheckedProof {
    fn new() -> Self {
        Self::default()
    }

    /// Sets the audit path the proof will use to reconstruct
    /// the Merkle Tree Hash.
    ///
    /// The `audit_path` byte buffer's length must be a multiple of 32.
    ///
    /// The builder does not currently verify that the length of the audit path
    /// is plausible, i.e. that it has exactly the right number of segments
    /// for walking the path from the leaf index to the root for a tree of
    /// the given size.
    ///
    /// This will simply result in an incorrect Merkle Tree Hash being reconstructed
    /// from the proof.
    pub fn audit_path(self, audit_path: Vec<u8>) -> Self {
        Self {
            audit_path,
            ..self
        }
    }

    /// Sets the index of the leaf that this proof is for.
    ///
    /// The leaf index must fall inside the tree size set by
    /// [`ProofBuilder::tree_size`]. The leaf index `i` maps
    /// to a tree index `j = 2 * i`.
    pub fn leaf_index(self, leaf_index: usize) -> Self {
        Self {
            leaf_index,
            ..self
        }
    }

    /// Sets the tree size of the proof.
    ///
    /// The tree size must be `tree_size > 0` because proves are
    /// not defined for empty trees.
    pub fn tree_size(self, tree_size: usize) -> Self {
        Self {
            tree_size,
            ..self
        }
    }

    /// Constructs the [`Proof`] from the builder inputs.
    ///
    /// # Errors
    ///
    /// Returns the following errors conditions:
    /// + if the tree size is zero, see [`ProofBuilder::tree_size`];
    /// + if the leaf index falls outside the tree, see [`ProofBuilder::leaf_index`];
    /// + if the audit path length is not a multiple of 32, see [`ProofBuilder::audit_path`].
    pub fn try_into_proof(self) -> Result<Proof, InvalidProof> {
        let Self {
            audit_path,
            leaf_index,
            tree_size,
        } = self;

        let Some(tree_size) = NonZeroUsize::new(tree_size) else {
            return Err(InvalidProof::zero_tree_size());
        };

        if !crate::is_leaf_index_in_tree(leaf_index, tree_size.get()) {
            return Err(InvalidProof::leaf_index_outside_tree(leaf_index, tree_size));
        }

        if audit_path.len() % 32 != 0 {
            return Err(InvalidProof::audit_path_not_multiple_of_32(
                audit_path.len(),
            ));
        }

        Ok(Proof {
            audit_path,
            leaf_index,
            tree_size,
        })
    }
}

/// The proof that a node is included in a Merkle tree.
///
/// The proof is the concatenation of all sibling hashes required to reconstruct
/// the Merkle tree from a leaf. This is also called the audit path.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Proof {
    pub(super) audit_path: Vec<u8>,
    pub(super) leaf_index: usize,
    pub(super) tree_size: NonZeroUsize,
}

impl Proof {
    pub fn unchecked() -> UncheckedProof {
        UncheckedProof::new()
    }

    pub fn into_unchecked(self) -> UncheckedProof {
        let Self {
            audit_path,
            leaf_index,
            tree_size,
        } = self;
        UncheckedProof {
            audit_path,
            leaf_index,
            tree_size: tree_size.get(),
        }
    }

    /// Returns the audit path of the proof.
    ///
    /// This is the concatenation of all hashes to reconstruct merkle tree root
    /// from the node under proof.
    #[must_use]
    #[inline]
    pub fn audit_path(&self) -> &[u8] {
        &self.audit_path
    }

    /// Returns the leaf index
    #[must_use]
    #[inline]
    pub fn leaf_index(&self) -> usize {
        self.leaf_index
    }

    /// Returns if the proof is empty.
    ///
    /// This can happen if the proof was constructed for a tree containing only
    /// one leaf.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.audit_path.is_empty()
    }

    /// Returns the number of segments in the proof.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.audit_path.len() / 32
    }

    /// Returns the size of the tree this proof was derived from.
    #[must_use]
    #[inline]
    pub fn tree_size(&self) -> NonZeroUsize {
        self.tree_size
    }

    /// Starts an audit using the [`Audit`] API.
    ///
    /// This method is the entry point to the [`Audit`] API to either
    /// reconstruct a merkle tree root, or to test if a leaf can be verified
    /// to be part of a tree using the given proof.
    ///
    /// It is particularly useful for verifying that a more complex leaf without
    /// needing to allocate a buffer for it.
    ///
    /// Use [`Proof::is_leaf_in_tree`] if the leaf is not complex or already allocated.
    ///
    /// # Examples
    /// ```
    /// # use astria_merkle::Tree;
    /// let mut tree = Tree::from_leaves(&[&[1; 32][..], &[4, 4, 4], b"helloworld"]);
    /// tree.build_leaf()
    ///     .write(&[42; 1])
    ///     .write(&[1, 1])
    ///     .write(&vec![42; 3])
    ///     .write(b"42");
    /// let root = tree.root();
    /// let proof = tree.construct_proof(3).expect("leaf 4 is in the tree");
    /// assert!(
    ///     proof
    ///         .audit()
    ///         .with_root(root)
    ///         .with_leaf_builder()
    ///         .write(&[42; 1])
    ///         .write(&[1, 1])
    ///         .write(&vec![42; 3])
    ///         .write(b"42")
    ///         .finish_leaf()
    ///         .perform()
    /// );
    /// ```
    #[must_use = "an audit must be performed to be useful"]
    pub fn audit(&self) -> Audit {
        Audit::new(self)
    }

    /// Walks the audit path to reconstruct the root hash starting from a node hash.
    ///
    /// Use this method if the starting point is the hash of a node (like the leaf hash)
    /// which does not need to be hashed prior to combining with its sibling.
    ///
    /// # Examples
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
    /// let root = tree.root();
    /// let proof = tree
    ///     .construct_proof(6)
    ///     .expect("leaf 7 must be inside the tree");
    /// let leaf_hash = tree.leaf(6).expect("leaf 7 must be inside the tree");
    /// let reconstructed_root = proof.reconstruct_root_with_leaf_hash(leaf_hash);
    /// assert_eq!(root, reconstructed_root);
    /// ```
    #[must_use]
    pub fn reconstruct_root_with_leaf_hash(&self, leaf_hash: [u8; 32]) -> [u8; 32] {
        let Self {
            audit_path,
            leaf_index,
            tree_size,
        } = self;
        let mut i = crate::leaf_index_to_tree_index(*leaf_index);
        let mut acc = leaf_hash;
        for sibling in audit_path.chunks(32) {
            let parent = crate::complete_parent(i, tree_size.get());
            if parent > i {
                acc = crate::combine(&acc, sibling);
            } else {
                acc = crate::combine(sibling, &acc);
            }
            i = parent;
        }
        acc
    }

    /// Walks the audit path to reconstruct the root hash starting from a node.
    ///
    /// Use this method if the starting point is a leaf, which will
    /// be hashed prior to combining with its sibling.
    ///
    /// Use the [`Audit::reconstruct_root`] via [`Proof::audit`] if the leaf
    /// is more complex and you want to avoid allocating a buffer for it.
    ///
    /// # Examples
    /// ```
    /// let mut tree = astria_merkle::Tree::new();
    /// tree.push(&[1; 32]);
    /// tree.push(&[2; 32]);
    /// tree.push(&[3; 32]);
    /// tree.push(&[4; 32]);
    /// tree.push(&[5; 32]);
    /// tree.push(&[6; 32]);
    /// tree.push(&[7; 32]);
    /// tree.push(&[8; 32]);
    /// let root = tree.root();
    /// let proof = tree
    ///     .construct_proof(6)
    ///     .expect("leaf 7 must be inside the tree");
    /// let reconstructed_root = proof.reconstruct_root_with_leaf(&[7_u8; 32]);
    /// assert_eq!(root, reconstructed_root);
    /// ```
    #[must_use]
    pub fn reconstruct_root_with_leaf(&self, leaf: &[u8]) -> [u8; 32] {
        self.audit().with_leaf(leaf).reconstruct_root()
    }

    /// Returns if `leaf` is part of the merkle tree identified by its `root_hash`.
    ///
    /// This is a utility function that walks the audit path in `Proof` starting
    /// from `leaf` and compares with `root_hash`.
    ///
    /// Use the [`Audit`] API via [`Proof::audit`] if `leaf` is more complex
    /// and you want to avoid allocating a buffer for it.
    ///
    /// # Examples
    /// ```
    /// let mut tree = astria_merkle::Tree::new();
    /// tree.push(&[1; 32]);
    /// tree.push(&[2; 32]);
    /// tree.push(&[3; 32]);
    /// tree.push(&[4; 32]);
    /// tree.push(&[5; 32]);
    /// tree.push(&[6; 32]);
    /// tree.push(&[7; 32]);
    /// tree.push(&[8; 32]);
    /// let root = tree.root();
    /// let proof = tree
    ///     .construct_proof(6)
    ///     .expect("leaf 7 must be inside the tree");
    /// assert!(proof.verify(&[7; 32], root));
    /// ```
    #[must_use]
    pub fn verify(&self, leaf: &[u8], root_hash: [u8; 32]) -> bool {
        self.audit().with_leaf(leaf).with_root(root_hash).perform()
    }
}
