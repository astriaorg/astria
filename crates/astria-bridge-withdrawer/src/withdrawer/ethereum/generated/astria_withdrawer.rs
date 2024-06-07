pub use astria_withdrawer::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms,
    clippy::type_complexity,
    dead_code,
    non_camel_case_types
)]
pub mod astria_withdrawer {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                    name: ::std::borrow::ToOwned::to_owned("_baseChainAssetPrecision",),
                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
                    internal_type: ::core::option::Option::Some(::std::borrow::ToOwned::to_owned(
                        "uint32"
                    ),),
                },],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("BASE_CHAIN_ASSET_PRECISION"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("BASE_CHAIN_ASSET_PRECISION",),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint32"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawToIbcChain"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("withdrawToIbcChain"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("destinationChainAddress",),
                                kind: ::ethers::core::abi::ethabi::ParamType::String,
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("string"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("memo"),
                                kind: ::ethers::core::abi::ethabi::ParamType::String,
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("string"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawToSequencer"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("withdrawToSequencer",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("destinationChainAddress",),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                    },],
                ),
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("Ics20Withdrawal"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("Ics20Withdrawal"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("sender"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: true,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: true,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("destinationChainAddress",),
                                kind: ::ethers::core::abi::ethabi::ParamType::String,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("memo"),
                                kind: ::ethers::core::abi::ethabi::ParamType::String,
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SequencerWithdrawal"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("SequencerWithdrawal",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("sender"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: true,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: true,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("destinationChainAddress",),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
            ]),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    /// The parsed JSON ABI of the contract.
    pub static ASTRIAWITHDRAWER_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\xC0`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x06|8\x03\x80a\x06|\x839\x81\x01`@\x81\x90Ra\0/\x91a\0\xEFV[`\x12\x81c\xFF\xFF\xFF\xFF\x16\x11\x15a\0\xC6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`M`$\x82\x01R\x7FAstriaWithdrawer: base chain ass`D\x82\x01R\x7Fet precision must be less than o`d\x82\x01Rl\x0ED\x0C\xAE.\xAC-\x84\x0E\x8D\xE4\x06'`\x9B\x1B`\x84\x82\x01R`\xA4\x01`@Q\x80\x91\x03\x90\xFD[c\xFF\xFF\xFF\xFF\x81\x16`\x80Ra\0\xDB\x81`\x12a\x012V[a\0\xE6\x90`\na\x02<V[`\xA0RPa\x02NV[`\0` \x82\x84\x03\x12\x15a\x01\x01W`\0\x80\xFD[\x81Qc\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x01\x15W`\0\x80\xFD[\x93\x92PPPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[c\xFF\xFF\xFF\xFF\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15a\x01OWa\x01Oa\x01\x1CV[P\x92\x91PPV[`\x01\x81\x81[\x80\x85\x11\x15a\x01\x91W\x81`\0\x19\x04\x82\x11\x15a\x01wWa\x01wa\x01\x1CV[\x80\x85\x16\x15a\x01\x84W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x01[V[P\x92P\x92\x90PV[`\0\x82a\x01\xA8WP`\x01a\x026V[\x81a\x01\xB5WP`\0a\x026V[\x81`\x01\x81\x14a\x01\xCBW`\x02\x81\x14a\x01\xD5Wa\x01\xF1V[`\x01\x91PPa\x026V[`\xFF\x84\x11\x15a\x01\xE6Wa\x01\xE6a\x01\x1CV[PP`\x01\x82\x1Ba\x026V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x02\x14WP\x81\x81\na\x026V[a\x02\x1E\x83\x83a\x01VV[\x80`\0\x19\x04\x82\x11\x15a\x022Wa\x022a\x01\x1CV[\x02\x90P[\x92\x91PPV[`\0a\x01\x15c\xFF\xFF\xFF\xFF\x84\x16\x83a\x01\x99V[`\x80Q`\xA0Qa\x04\x04a\x02x`\09`\0\x81\x81`\xB6\x01Ra\x01M\x01R`\0`K\x01Ra\x04\x04`\0\xF3\xFE`\x80`@R`\x046\x10a\x004W`\x005`\xE0\x1C\x80c~\xB6\xDE\xC7\x14a\09W\x80c\x9A\x97z\xFE\x14a\0\x86W\x80c\xA9\x96\xE0 \x14a\0\x9BW[`\0\x80\xFD[4\x80\x15a\0EW`\0\x80\xFD[Pa\0m\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01`@Q\x80\x91\x03\x90\xF3[a\0\x99a\0\x946`\x04a\x01\xDEV[a\0\xAEV[\0[a\0\x99a\0\xA96`\x04a\x02WV[a\x01EV[4`\0a\0\xDB\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x02\xC3V[\x11a\x01\x01W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\0\xF8\x90a\x02\xE5V[`@Q\x80\x91\x03\x90\xFD[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x81R4\x903\x90\x7F\xAE\x8EffM\x10\x85DP\x9C\x9A[j\x9F3\xC3\xB5\xFE\xF3\xF8\x8E]?\xA6\x80pjo\xEB\x13`\xE3\x90` \x01`@Q\x80\x91\x03\x90\xA3PPV[4`\0a\x01r\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x02\xC3V[\x11a\x01\x8FW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\0\xF8\x90a\x02\xE5V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x01\xCF\x94\x93\x92\x91\x90a\x03\x9CV[`@Q\x80\x91\x03\x90\xA3PPPPPV[`\0` \x82\x84\x03\x12\x15a\x01\xF0W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x02\x07W`\0\x80\xFD[\x93\x92PPPV[`\0\x80\x83`\x1F\x84\x01\x12a\x02 W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x028W`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x02PW`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x02mW`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x02\x85W`\0\x80\xFD[a\x02\x91\x88\x83\x89\x01a\x02\x0EV[\x90\x96P\x94P` \x87\x015\x91P\x80\x82\x11\x15a\x02\xAAW`\0\x80\xFD[Pa\x02\xB7\x87\x82\x88\x01a\x02\x0EV[\x95\x98\x94\x97P\x95PPPPV[`\0\x82a\x02\xE0WcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x03\xB0`@\x83\x01\x86\x88a\x03sV[\x82\x81\x03` \x84\x01Ra\x03\xC3\x81\x85\x87a\x03sV[\x97\x96PPPPPPPV\xFE\xA2dipfsX\"\x12 \xBFH[\xDE\xF8\xBB^>\x0C\x97d\xAD2S-\x90\xB9x\xA3\xC3\xB0\xB2\xF5G\x8C\xE8\xE5\xDB>\x85\xD3\xA0dsolcC\0\x08\x15\x003";
    /// The bytecode of the contract.
    pub static ASTRIAWITHDRAWER_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\x004W`\x005`\xE0\x1C\x80c~\xB6\xDE\xC7\x14a\09W\x80c\x9A\x97z\xFE\x14a\0\x86W\x80c\xA9\x96\xE0 \x14a\0\x9BW[`\0\x80\xFD[4\x80\x15a\0EW`\0\x80\xFD[Pa\0m\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01`@Q\x80\x91\x03\x90\xF3[a\0\x99a\0\x946`\x04a\x01\xDEV[a\0\xAEV[\0[a\0\x99a\0\xA96`\x04a\x02WV[a\x01EV[4`\0a\0\xDB\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x02\xC3V[\x11a\x01\x01W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\0\xF8\x90a\x02\xE5V[`@Q\x80\x91\x03\x90\xFD[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x81R4\x903\x90\x7F\xAE\x8EffM\x10\x85DP\x9C\x9A[j\x9F3\xC3\xB5\xFE\xF3\xF8\x8E]?\xA6\x80pjo\xEB\x13`\xE3\x90` \x01`@Q\x80\x91\x03\x90\xA3PPV[4`\0a\x01r\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x02\xC3V[\x11a\x01\x8FW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\0\xF8\x90a\x02\xE5V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x01\xCF\x94\x93\x92\x91\x90a\x03\x9CV[`@Q\x80\x91\x03\x90\xA3PPPPPV[`\0` \x82\x84\x03\x12\x15a\x01\xF0W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x02\x07W`\0\x80\xFD[\x93\x92PPPV[`\0\x80\x83`\x1F\x84\x01\x12a\x02 W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x028W`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x02PW`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x02mW`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x02\x85W`\0\x80\xFD[a\x02\x91\x88\x83\x89\x01a\x02\x0EV[\x90\x96P\x94P` \x87\x015\x91P\x80\x82\x11\x15a\x02\xAAW`\0\x80\xFD[Pa\x02\xB7\x87\x82\x88\x01a\x02\x0EV[\x95\x98\x94\x97P\x95PPPPV[`\0\x82a\x02\xE0WcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x03\xB0`@\x83\x01\x86\x88a\x03sV[\x82\x81\x03` \x84\x01Ra\x03\xC3\x81\x85\x87a\x03sV[\x97\x96PPPPPPPV\xFE\xA2dipfsX\"\x12 \xBFH[\xDE\xF8\xBB^>\x0C\x97d\xAD2S-\x90\xB9x\xA3\xC3\xB0\xB2\xF5G\x8C\xE8\xE5\xDB>\x85\xD3\xA0dsolcC\0\x08\x15\x003";
    /// The deployed bytecode of the contract.
    pub static ASTRIAWITHDRAWER_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
    pub struct AstriaWithdrawer<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for AstriaWithdrawer<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for AstriaWithdrawer<M> {
        type Target = ::ethers::contract::Contract<M>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for AstriaWithdrawer<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for AstriaWithdrawer<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(AstriaWithdrawer))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> AstriaWithdrawer<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(
                address.into(),
                ASTRIAWITHDRAWER_ABI.clone(),
                client,
            ))
        }

        /// Constructs the general purpose `Deployer` instance based on the provided constructor
        /// arguments and sends it. Returns a new instance of a deployer that returns an
        /// instance of this contract after sending the transaction
        ///
        /// Notes:
        /// - If there are no constructor arguments, you should pass `()` as the argument.
        /// - The default poll duration is 7 seconds.
        /// - The default number of confirmations is 1 block.
        ///
        ///
        /// # Example
        ///
        /// Generate contract bindings with `abigen!` and deploy a new contract instance.
        ///
        /// *Note*: this requires a `bytecode` and `abi` object in the `greeter.json` artifact.
        ///
        /// ```ignore
        /// # async fn deploy<M: ethers::providers::Middleware>(client: ::std::sync::Arc<M>) {
        ///     abigen!(Greeter, "../greeter.json");
        ///
        ///    let greeter_contract = Greeter::deploy(client, "Hello world!".to_string()).unwrap().send().await.unwrap();
        ///    let msg = greeter_contract.greet().call().await.unwrap();
        /// # }
        /// ```
        pub fn deploy<T: ::ethers::core::abi::Tokenize>(
            client: ::std::sync::Arc<M>,
            constructor_args: T,
        ) -> ::core::result::Result<
            ::ethers::contract::builders::ContractDeployer<M, Self>,
            ::ethers::contract::ContractError<M>,
        > {
            let factory = ::ethers::contract::ContractFactory::new(
                ASTRIAWITHDRAWER_ABI.clone(),
                ASTRIAWITHDRAWER_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }

        /// Calls the contract's `BASE_CHAIN_ASSET_PRECISION` (0x7eb6dec7) function
        pub fn base_chain_asset_precision(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, u32> {
            self.0
                .method_hash([126, 182, 222, 199], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `withdrawToIbcChain` (0xa996e020) function
        pub fn withdraw_to_ibc_chain(
            &self,
            destination_chain_address: ::std::string::String,
            memo: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([169, 150, 224, 32], (destination_chain_address, memo))
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `withdrawToSequencer` (0x9a977afe) function
        pub fn withdraw_to_sequencer(
            &self,
            destination_chain_address: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([154, 151, 122, 254], destination_chain_address)
                .expect("method not found (this should never happen)")
        }

        /// Gets the contract's `Ics20Withdrawal` event
        pub fn ics_20_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, Ics20WithdrawalFilter>
        {
            self.0.event()
        }

        /// Gets the contract's `SequencerWithdrawal` event
        pub fn sequencer_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, SequencerWithdrawalFilter>
        {
            self.0.event()
        }

        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, AstriaWithdrawerEvents>
        {
            self.0
                .event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
        for AstriaWithdrawer<M>
    {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(
        name = "Ics20Withdrawal",
        abi = "Ics20Withdrawal(address,uint256,string,string)"
    )]
    pub struct Ics20WithdrawalFilter {
        #[ethevent(indexed)]
        pub sender: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
        pub memo: ::std::string::String,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(
        name = "SequencerWithdrawal",
        abi = "SequencerWithdrawal(address,uint256,address)"
    )]
    pub struct SequencerWithdrawalFilter {
        #[ethevent(indexed)]
        pub sender: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::ethers::core::types::Address,
    }
    /// Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaWithdrawerEvents {
        Ics20WithdrawalFilter(Ics20WithdrawalFilter),
        SequencerWithdrawalFilter(SequencerWithdrawalFilter),
    }
    impl ::ethers::contract::EthLogDecode for AstriaWithdrawerEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = Ics20WithdrawalFilter::decode_log(log) {
                return Ok(AstriaWithdrawerEvents::Ics20WithdrawalFilter(decoded));
            }
            if let Ok(decoded) = SequencerWithdrawalFilter::decode_log(log) {
                return Ok(AstriaWithdrawerEvents::SequencerWithdrawalFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for AstriaWithdrawerEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::Ics20WithdrawalFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::SequencerWithdrawalFilter(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<Ics20WithdrawalFilter> for AstriaWithdrawerEvents {
        fn from(value: Ics20WithdrawalFilter) -> Self {
            Self::Ics20WithdrawalFilter(value)
        }
    }
    impl ::core::convert::From<SequencerWithdrawalFilter> for AstriaWithdrawerEvents {
        fn from(value: SequencerWithdrawalFilter) -> Self {
            Self::SequencerWithdrawalFilter(value)
        }
    }
    /// Container type for all input parameters for the `BASE_CHAIN_ASSET_PRECISION` function with
    /// signature `BASE_CHAIN_ASSET_PRECISION()` and selector `0x7eb6dec7`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(
        name = "BASE_CHAIN_ASSET_PRECISION",
        abi = "BASE_CHAIN_ASSET_PRECISION()"
    )]
    pub struct BaseChainAssetPrecisionCall;
    /// Container type for all input parameters for the `withdrawToIbcChain` function with signature
    /// `withdrawToIbcChain(string,string)` and selector `0xa996e020`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "withdrawToIbcChain", abi = "withdrawToIbcChain(string,string)")]
    pub struct WithdrawToIbcChainCall {
        pub destination_chain_address: ::std::string::String,
        pub memo: ::std::string::String,
    }
    /// Container type for all input parameters for the `withdrawToSequencer` function with
    /// signature `withdrawToSequencer(address)` and selector `0x9a977afe`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "withdrawToSequencer", abi = "withdrawToSequencer(address)")]
    pub struct WithdrawToSequencerCall {
        pub destination_chain_address: ::ethers::core::types::Address,
    }
    /// Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaWithdrawerCalls {
        BaseChainAssetPrecision(BaseChainAssetPrecisionCall),
        WithdrawToIbcChain(WithdrawToIbcChainCall),
        WithdrawToSequencer(WithdrawToSequencerCall),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaWithdrawerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <BaseChainAssetPrecisionCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::BaseChainAssetPrecision(decoded));
            }
            if let Ok(decoded) =
                <WithdrawToIbcChainCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::WithdrawToIbcChain(decoded));
            }
            if let Ok(decoded) =
                <WithdrawToSequencerCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::WithdrawToSequencer(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for AstriaWithdrawerCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::BaseChainAssetPrecision(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::WithdrawToIbcChain(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::WithdrawToSequencer(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for AstriaWithdrawerCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::BaseChainAssetPrecision(element) => ::core::fmt::Display::fmt(element, f),
                Self::WithdrawToIbcChain(element) => ::core::fmt::Display::fmt(element, f),
                Self::WithdrawToSequencer(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<BaseChainAssetPrecisionCall> for AstriaWithdrawerCalls {
        fn from(value: BaseChainAssetPrecisionCall) -> Self {
            Self::BaseChainAssetPrecision(value)
        }
    }
    impl ::core::convert::From<WithdrawToIbcChainCall> for AstriaWithdrawerCalls {
        fn from(value: WithdrawToIbcChainCall) -> Self {
            Self::WithdrawToIbcChain(value)
        }
    }
    impl ::core::convert::From<WithdrawToSequencerCall> for AstriaWithdrawerCalls {
        fn from(value: WithdrawToSequencerCall) -> Self {
            Self::WithdrawToSequencer(value)
        }
    }
    /// Container type for all return fields from the `BASE_CHAIN_ASSET_PRECISION` function with
    /// signature `BASE_CHAIN_ASSET_PRECISION()` and selector `0x7eb6dec7`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct BaseChainAssetPrecisionReturn(pub u32);
}
