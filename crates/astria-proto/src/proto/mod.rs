#[path = "generated"]
/// Files generated using [`prost`] and [`tonic`] via [`buf`] and its
/// [`neoeinstein-prost`] and [`neoeinstein-tonic`] plugins.
///
/// [`prost`]:
/// [`tonic`]:
/// [`buf`]: https://buf.build
/// [`neoeinstein-prost`]: https://buf.build/community/neoeinstein-prost
/// [`neoeinstein-tonic`]: https://buf.build/community/neoeinstein-tonic
pub mod generated {
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
        #[path = "astria.sequencer.v1alpha1.rs"]
        pub mod v1alpha1;
    }
}
