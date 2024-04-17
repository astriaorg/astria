#[cfg(not(target_pointer_width = "64"))]
compile_error!(
    "library is only guaranteed to run on 64 bit machines due to casts from/to u64 and usize"
);

#[rustfmt::skip]
pub mod generated;

pub mod execution;
pub mod primitive;
pub mod sequencer;
pub mod sequencerblock;

#[cfg(feature = "serde")]
pub(crate) mod serde;
