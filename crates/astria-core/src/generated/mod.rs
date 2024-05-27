#![allow(
    unreachable_pub,
    clippy::pedantic,
    clippy::needless_borrows_for_generic_args
)]

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

    pub mod v1alpha2 {
        include!("astria.execution.v1alpha2.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("astria.execution.v1alpha2.serde.rs");
        }
    }
}

#[path = ""]
pub mod primitive {
    pub mod v1 {
        include!("astria.primitive.v1.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("astria.primitive.v1.serde.rs");
        }
    }
}

#[path = ""]
pub mod protocol {
    #[path = ""]
    pub mod account {
        #[path = "astria.protocol.accounts.v1alpha1.rs"]
        pub mod v1alpha1;
    }
    #[path = ""]
    pub mod asset {
        #[path = "astria.protocol.asset.v1alpha1.rs"]
        pub mod v1alpha1;
    }
    #[path = ""]
    pub mod transaction {
        #[path = "astria.protocol.transactions.v1alpha1.rs"]
        pub mod v1alpha1;
    }
}

#[path = ""]
pub mod sequencerblock {
    pub mod v1alpha1 {
        include!("astria.sequencerblock.v1alpha1.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("astria.sequencerblock.v1alpha1.serde.rs");
        }
    }
}

#[path = ""]
pub mod composer {
    #[path = "astria.composer.v1alpha1.rs"]
    pub mod v1alpha1;
}

#[path = ""]
pub mod celestia {
    #[path = "celestia.blob.v1.rs"]
    pub mod v1 {
        include!("celestia.blob.v1.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("celestia.blob.v1.serde.rs");
        }
    }
}

#[path = ""]
pub mod cosmos {
    pub mod auth {
        pub mod v1beta1 {
            include!("cosmos.auth.v1beta1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("cosmos.auth.v1beta1.serde.rs");
            }
        }
    }

    pub mod base {
        pub mod abci {
            pub mod v1beta1 {
                include!("cosmos.base.abci.v1beta1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("cosmos.base.abci.v1beta1.serde.rs");
                }
            }
        }

        pub mod node {
            pub mod v1beta1 {
                include!("cosmos.base.node.v1beta1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("cosmos.base.node.v1beta1.serde.rs");
                }
            }
        }

        pub mod tendermint {
            pub mod v1beta1 {
                include!("cosmos.base.tendermint.v1beta1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("cosmos.base.tendermint.v1beta1.serde.rs");
                }
            }
        }

        pub mod v1beta1 {
            include!("cosmos.base.v1beta1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("cosmos.base.v1beta1.serde.rs");
            }
        }
    }

    pub mod crypto {
        pub mod multisig {
            pub mod v1beta1 {
                include!("cosmos.crypto.multisig.v1beta1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("cosmos.crypto.multisig.v1beta1.serde.rs");
                }
            }
        }

        pub mod secp256k1 {
            include!("cosmos.crypto.secp256k1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("cosmos.crypto.secp256k1.serde.rs");
            }
        }
    }

    pub mod tx {
        pub mod signing {
            pub mod v1beta1 {
                include!("cosmos.tx.signing.v1beta1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("cosmos.tx.signing.v1beta1.serde.rs");
                }
            }
        }

        pub mod v1beta1 {
            include!("cosmos.tx.v1beta1.rs");
            #[cfg(feature = "serde")]
            use super::signing;

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("cosmos.tx.v1beta1.serde.rs");
            }
        }
    }
}

#[path = ""]
pub mod tendermint {
    pub mod abci {
        include!("tendermint.abci.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("tendermint.abci.serde.rs");
        }
    }

    pub mod p2p {
        include!("tendermint.p2p.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("tendermint.p2p.serde.rs");
        }
    }

    pub mod types {
        include!("tendermint.types.rs");

        #[cfg(feature = "serde")]
        mod _serde_impl {
            use super::*;
            include!("tendermint.types.serde.rs");
        }
    }
}
