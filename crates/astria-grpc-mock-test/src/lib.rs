#![allow(unreachable_pub, clippy::pedantic)]

pub mod health {
    include!("generated/grpc.health.v1.rs");
    include!("generated/grpc.health.v1.serde.rs");
}
