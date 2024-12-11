#![allow(
    unreachable_pub,
    clippy::pedantic,
    clippy::arithmetic_side_effects,
    reason = "this crate is for testing only"
)]

#[expect(
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    reason = "cannot prevent generated files from having allow attributes"
)]
pub mod service {
    include!("generated/grpc.mock_service.v1.rs");
    include!("generated/grpc.mock_service.v1.serde.rs");
}
