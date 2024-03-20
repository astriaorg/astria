#![allow(unreachable_pub, clippy::pedantic)]

/// Files generated using [`tonic-build`] and [`buf`] via the [`tools/protobuf-compiler`]
/// build tool.
///
/// [`tonic-build`]: https://docs.rs/tonic-build
/// [`buf`]: https://buf.build
/// [`tools/protobuf-compiler`]: ../../../../tools/protobuf-compiler
#[path = ""]
pub mod execution {
    #[path = "astria.execution.v1alpha1.rs"]
    pub mod v1alpha1;

    #[path = "astria.execution.v1alpha2.rs"]
    pub mod v1alpha2;
}

#[path = ""]
pub mod primitive {
    #[path = "astria.primitive.v1.rs"]
    pub mod v1;
}

#[path = ""]
pub mod sequencer {
    #[path = "astria.sequencer.v1.rs"]
    pub mod v1;
}

#[path = ""]
pub mod composer {
    #[path = "astria.composer.v1alpha1.rs"]
    pub mod v1alpha1;
}

