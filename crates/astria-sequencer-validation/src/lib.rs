//! This crate contains types and logic for constructing merkle trees and generating proofs of
//! inclusion.
//!
//! This is used by the astria-sequencer to generate a commitment to the rollup data in a block,
//! and for the astria-conductor to validate that the rollup data received was in fact committed to.
mod proof;
pub mod utils;

pub use ct_merkle;
pub use proof::{
    InclusionProof,
    InclusionProofBuilder,
    IndexOutOfBounds,
    MerkleTree,
    VerificationFailure,
};
pub use utils::generate_action_tree_leaves;
