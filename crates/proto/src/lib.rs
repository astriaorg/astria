#[cfg(not(target_pointer_width = "64"))]
compile_error!(
    "library is only guaranteed to run on 64 bit machines due to casts from/to u64 and usize"
);

pub use prost::{
    DecodeError,
    EncodeError,
    Message,
};

#[allow(warnings)]
#[allow(unreachable_pub)]
#[rustfmt::skip]
mod proto;

#[cfg(feature = "native")]
pub mod native;

pub use proto::generated;
