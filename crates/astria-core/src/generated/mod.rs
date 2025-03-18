#![allow(
    unreachable_pub,
    clippy::pedantic,
    clippy::needless_borrows_for_generic_args,
    clippy::arithmetic_side_effects,
    clippy::needless_lifetimes
)]
//! Files generated using [`tonic-build`] and [`buf`] via the [`tools/protobuf-compiler`]
//! build tool.
//!
//! [`tonic-build`]: https://docs.rs/tonic-build
//! [`buf`]: https://buf.build
//! [`tools/protobuf-compiler`]: ../../../../tools/protobuf-compiler

#[path = ""]
pub mod astria_vendored {
    #[path = ""]
    pub mod tendermint {
        pub mod abci {
            include!("astria_vendored.tendermint.abci.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria_vendored.tendermint.abci.serde.rs");
            }
        }

        pub mod crypto {
            include!("astria_vendored.tendermint.crypto.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria_vendored.tendermint.crypto.serde.rs");
            }
        }
    }
}

#[path = ""]
pub mod astria {
    #[path = ""]
    pub mod auction {
        pub mod v1alpha1 {
            include!("astria.auction.v1alpha1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria.auction.v1alpha1.serde.rs");
            }
        }
    }

    #[path = ""]
    pub mod execution {
        pub mod v1 {
            include!("astria.execution.v1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria.execution.v1.serde.rs");
            }
        }
        pub mod v2 {
            include!("astria.execution.v2.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria.execution.v2.serde.rs");
            }
        }
    }

    pub mod optimistic_execution {
        pub mod v1alpha1 {
            include!("astria.optimistic_execution.v1alpha1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria.optimistic_execution.v1alpha1.serde.rs");
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
        pub mod accounts {
            #[path = "astria.protocol.accounts.v1.rs"]
            pub mod v1;
        }
        #[path = ""]
        pub mod asset {
            #[path = "astria.protocol.asset.v1.rs"]
            pub mod v1;
        }
        #[path = ""]
        pub mod bridge {
            #[path = "astria.protocol.bridge.v1.rs"]
            pub mod v1;
        }
        #[path = ""]
        pub mod fees {
            #[path = "astria.protocol.fees.v1.rs"]
            pub mod v1 {
                include!("astria.protocol.fees.v1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impls {
                    use super::*;
                    include!("astria.protocol.fees.v1.serde.rs");
                }
            }
        }
        #[path = ""]
        pub mod genesis {
            pub mod v1 {
                include!("astria.protocol.genesis.v1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impls {
                    use super::*;
                    include!("astria.protocol.genesis.v1.serde.rs");
                }
            }
        }
        #[path = ""]
        pub mod memos {
            pub mod v1 {
                include!("astria.protocol.memos.v1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impls {
                    use super::*;
                    include!("astria.protocol.memos.v1.serde.rs");
                }
            }
        }

        #[path = ""]
        pub mod transaction {
            pub mod v1 {
                include!("astria.protocol.transaction.v1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("astria.protocol.transaction.v1.serde.rs");
                }
            }
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

        pub mod v1 {
            include!("astria.sequencerblock.v1.rs");

            #[cfg(feature = "serde")]
            mod _serde_impl {
                use super::*;
                include!("astria.sequencerblock.v1.serde.rs");
            }
        }

        pub mod optimistic {
            pub mod v1alpha1 {
                include!("astria.sequencerblock.optimistic.v1alpha1.rs");

                #[cfg(feature = "serde")]
                mod _serde_impl {
                    use super::*;
                    include!("astria.sequencerblock.optimistic.v1alpha1.serde.rs");
                }
            }
        }
    }

    #[path = ""]
    pub mod composer {
        #[path = "astria.composer.v1.rs"]
        pub mod v1;
    }
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
