#![allow(
    unreachable_pub,
    clippy::pedantic,
    clippy::arithmetic_side_effects,
    reason = "this crate is for testing only"
)]

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::needless_lifetimes,
    reason = "cannot prevent generated files from having allow attributes or specific lifetimes"
)]
pub mod health {
    include!("generated/grpc.health.v1.rs");
    include!("generated/grpc.health.v1.serde.rs");
}
