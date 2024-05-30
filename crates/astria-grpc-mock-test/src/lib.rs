#![allow(unreachable_pub, clippy::pedantic, clippy::arithmetic_side_effects)]

pub mod health {
    include!("generated/grpc.health.v1.rs");
    include!("generated/grpc.health.v1.serde.rs");
}
