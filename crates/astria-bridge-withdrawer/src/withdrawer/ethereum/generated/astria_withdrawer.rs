pub use astria_withdrawer::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms,
    clippy::type_complexity,
    dead_code,
    non_camel_case_types,
)]
pub mod astria_withdrawer {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned(
                            "_baseChainAssetPrecision",
                        ),
                        kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("uint32"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned(
                            "_baseChainBridgeAddress",
                        ),
                        kind: ::ethers::core::abi::ethabi::ParamType::String,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("string"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned(
                            "_baseChainAssetDenomination",
                        ),
                        kind: ::ethers::core::abi::ethabi::ParamType::String,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("string"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("BASE_CHAIN_ASSET_DENOMINATION"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "BASE_CHAIN_ASSET_DENOMINATION",
                            ),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("BASE_CHAIN_ASSET_PRECISION"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "BASE_CHAIN_ASSET_PRECISION",
                            ),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint32"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("BASE_CHAIN_BRIDGE_ADDRESS"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "BASE_CHAIN_BRIDGE_ADDRESS",
                            ),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawToIbcChain"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("withdrawToIbcChain"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "destinationChainAddress",
                                    ),
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
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawToSequencer"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "withdrawToSequencer",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "destinationChainAddress",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
                        },
                    ],
                ),
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("Ics20Withdrawal"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("Ics20Withdrawal"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("sender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "destinationChainAddress",
                                    ),
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
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SequencerWithdrawal"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "SequencerWithdrawal",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("sender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "destinationChainAddress",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
            ]),
            errors: ::std::collections::BTreeMap::new(),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static ASTRIAWITHDRAWER_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\xC0`@R4\x80\x15b\0\0\x11W`\0\x80\xFD[P`@Qb\0\n\xED8\x03\x80b\0\n\xED\x839\x81\x01`@\x81\x90Rb\0\x004\x91b\0\x01\xE0V[`\x12\x83c\xFF\xFF\xFF\xFF\x16\x11\x15b\0\0\xCCW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`M`$\x82\x01R\x7FAstriaWithdrawer: base chain ass`D\x82\x01R\x7Fet precision must be less than o`d\x82\x01Rl\x0ED\x0C\xAE.\xAC-\x84\x0E\x8D\xE4\x06'`\x9B\x1B`\x84\x82\x01R`\xA4\x01`@Q\x80\x91\x03\x90\xFD[c\xFF\xFF\xFF\xFF\x83\x16`\x80R`\0b\0\0\xE4\x83\x82b\0\x02\xF6V[P`\x01b\0\0\xF3\x82\x82b\0\x02\xF6V[Pb\0\x01\x01\x83`\x12b\0\x03\xD8V[b\0\x01\x0E\x90`\nb\0\x04\xFEV[`\xA0RPb\0\x05\x19\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0\x82`\x1F\x83\x01\x12b\0\x01CW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x01`Wb\0\x01`b\0\x01\x1BV[`@Q`\x1F\x83\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01\x90\x82\x82\x11\x81\x83\x10\x17\x15b\0\x01\x8BWb\0\x01\x8Bb\0\x01\x1BV[\x81`@R\x83\x81R` \x92P\x86\x83\x85\x88\x01\x01\x11\x15b\0\x01\xA8W`\0\x80\xFD[`\0\x91P[\x83\x82\x10\x15b\0\x01\xCCW\x85\x82\x01\x83\x01Q\x81\x83\x01\x84\x01R\x90\x82\x01\x90b\0\x01\xADV[`\0\x93\x81\x01\x90\x92\x01\x92\x90\x92R\x94\x93PPPPV[`\0\x80`\0``\x84\x86\x03\x12\x15b\0\x01\xF6W`\0\x80\xFD[\x83Qc\xFF\xFF\xFF\xFF\x81\x16\x81\x14b\0\x02\x0BW`\0\x80\xFD[` \x85\x01Q\x90\x93P`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x02)W`\0\x80\xFD[b\0\x027\x87\x83\x88\x01b\0\x011V[\x93P`@\x86\x01Q\x91P\x80\x82\x11\x15b\0\x02NW`\0\x80\xFD[Pb\0\x02]\x86\x82\x87\x01b\0\x011V[\x91PP\x92P\x92P\x92V[`\x01\x81\x81\x1C\x90\x82\x16\x80b\0\x02|W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03b\0\x02\x9DWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15b\0\x02\xF1W`\0\x81\x81R` \x81 `\x1F\x85\x01`\x05\x1C\x81\x01` \x86\x10\x15b\0\x02\xCCWP\x80[`\x1F\x85\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15b\0\x02\xEDW\x82\x81U`\x01\x01b\0\x02\xD8V[PPP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15b\0\x03\x12Wb\0\x03\x12b\0\x01\x1BV[b\0\x03*\x81b\0\x03#\x84Tb\0\x02gV[\x84b\0\x02\xA3V[` \x80`\x1F\x83\x11`\x01\x81\x14b\0\x03bW`\0\x84\x15b\0\x03IWP\x85\x83\x01Q[`\0\x19`\x03\x86\x90\x1B\x1C\x19\x16`\x01\x85\x90\x1B\x17\x85Ub\0\x02\xEDV[`\0\x85\x81R` \x81 `\x1F\x19\x86\x16\x91[\x82\x81\x10\x15b\0\x03\x93W\x88\x86\x01Q\x82U\x94\x84\x01\x94`\x01\x90\x91\x01\x90\x84\x01b\0\x03rV[P\x85\x82\x10\x15b\0\x03\xB2W\x87\x85\x01Q`\0\x19`\x03\x88\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPPP`\x01\x90\x81\x1B\x01\x90UPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[c\xFF\xFF\xFF\xFF\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15b\0\x03\xF8Wb\0\x03\xF8b\0\x03\xC2V[P\x92\x91PPV[`\x01\x81\x81[\x80\x85\x11\x15b\0\x04@W\x81`\0\x19\x04\x82\x11\x15b\0\x04$Wb\0\x04$b\0\x03\xC2V[\x80\x85\x16\x15b\0\x042W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90b\0\x04\x04V[P\x92P\x92\x90PV[`\0\x82b\0\x04YWP`\x01b\0\x04\xF8V[\x81b\0\x04hWP`\0b\0\x04\xF8V[\x81`\x01\x81\x14b\0\x04\x81W`\x02\x81\x14b\0\x04\x8CWb\0\x04\xACV[`\x01\x91PPb\0\x04\xF8V[`\xFF\x84\x11\x15b\0\x04\xA0Wb\0\x04\xA0b\0\x03\xC2V[PP`\x01\x82\x1Bb\0\x04\xF8V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15b\0\x04\xD1WP\x81\x81\nb\0\x04\xF8V[b\0\x04\xDD\x83\x83b\0\x03\xFFV[\x80`\0\x19\x04\x82\x11\x15b\0\x04\xF4Wb\0\x04\xF4b\0\x03\xC2V[\x02\x90P[\x92\x91PPV[`\0b\0\x05\x12c\xFF\xFF\xFF\xFF\x84\x16\x83b\0\x04HV[\x93\x92PPPV[`\x80Q`\xA0Qa\x05\xA8b\0\x05E`\09`\0\x81\x81a\x01\x04\x01Ra\x024\x01R`\0`a\x01Ra\x05\xA8`\0\xF3\xFE`\x80`@R`\x046\x10a\0JW`\x005`\xE0\x1C\x80c~\xB6\xDE\xC7\x14a\0OW\x80c\xA9\x96\xE0 \x14a\0\x9DW\x80c\xB6Gl~\x14a\0\xB2W\x80c\xBA\xB9\x16\xD0\x14a\0\xD4W\x80c\xDB\x97\xDC\x98\x14a\0\xE7W[`\0\x80\xFD[4\x80\x15a\0[W`\0\x80\xFD[Pa\0\x83\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xB0a\0\xAB6`\x04a\x03\x15V[a\0\xFCV[\0[4\x80\x15a\0\xBEW`\0\x80\xFD[Pa\0\xC7a\x01\x9EV[`@Qa\0\x94\x91\x90a\x03\x81V[a\0\xB0a\0\xE26`\x04a\x03\xCFV[a\x02,V[4\x80\x15a\0\xF3W`\0\x80\xFD[Pa\0\xC7a\x02\xBFV[4`\0a\x01)\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\x11V[\x11a\x01OW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01F\x90a\x043V[`@Q\x80\x91\x03\x90\xFD[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x01\x8F\x94\x93\x92\x91\x90a\x04\xEAV[`@Q\x80\x91\x03\x90\xA3PPPPPV[`\x01\x80Ta\x01\xAB\x90a\x05\x1CV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x01\xD7\x90a\x05\x1CV[\x80\x15a\x02$W\x80`\x1F\x10a\x01\xF9Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x02$V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x02\x07W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\0a\x02Y\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\x11V[\x11a\x02vW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01F\x90a\x043V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x85\x85`@Qa\x02\xB2\x92\x91\x90a\x05VV[`@Q\x80\x91\x03\x90\xA3PPPV[`\0\x80Ta\x01\xAB\x90a\x05\x1CV[`\0\x80\x83`\x1F\x84\x01\x12a\x02\xDEW`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x02\xF6W`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x03\x0EW`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x03+W`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x03CW`\0\x80\xFD[a\x03O\x88\x83\x89\x01a\x02\xCCV[\x90\x96P\x94P` \x87\x015\x91P\x80\x82\x11\x15a\x03hW`\0\x80\xFD[Pa\x03u\x87\x82\x88\x01a\x02\xCCV[\x95\x98\x94\x97P\x95PPPPV[`\0` \x80\x83R\x83Q\x80\x82\x85\x01R`\0[\x81\x81\x10\x15a\x03\xAEW\x85\x81\x01\x83\x01Q\x85\x82\x01`@\x01R\x82\x01a\x03\x92V[P`\0`@\x82\x86\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x92PPP\x92\x91PPV[`\0\x80` \x83\x85\x03\x12\x15a\x03\xE2W`\0\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x03\xF9W`\0\x80\xFD[a\x04\x05\x85\x82\x86\x01a\x02\xCCV[\x90\x96\x90\x95P\x93PPPPV[`\0\x82a\x04.WcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x04\xFE`@\x83\x01\x86\x88a\x04\xC1V[\x82\x81\x03` \x84\x01Ra\x05\x11\x81\x85\x87a\x04\xC1V[\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x050W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x05PWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x81R`\0a\x05j` \x83\x01\x84\x86a\x04\xC1V[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 \t \xD0n0\xAB\xBF\xC4\xFB\xEC\xCAZ\x19\x17 \xDB\xA8\xDEE\x9E\x1B\xD5\x85O\xB2\xAAq\x80HU\x05\xC4dsolcC\0\x08\x15\x003";
    /// The bytecode of the contract.
    pub static ASTRIAWITHDRAWER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0JW`\x005`\xE0\x1C\x80c~\xB6\xDE\xC7\x14a\0OW\x80c\xA9\x96\xE0 \x14a\0\x9DW\x80c\xB6Gl~\x14a\0\xB2W\x80c\xBA\xB9\x16\xD0\x14a\0\xD4W\x80c\xDB\x97\xDC\x98\x14a\0\xE7W[`\0\x80\xFD[4\x80\x15a\0[W`\0\x80\xFD[Pa\0\x83\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xB0a\0\xAB6`\x04a\x03\x15V[a\0\xFCV[\0[4\x80\x15a\0\xBEW`\0\x80\xFD[Pa\0\xC7a\x01\x9EV[`@Qa\0\x94\x91\x90a\x03\x81V[a\0\xB0a\0\xE26`\x04a\x03\xCFV[a\x02,V[4\x80\x15a\0\xF3W`\0\x80\xFD[Pa\0\xC7a\x02\xBFV[4`\0a\x01)\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\x11V[\x11a\x01OW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01F\x90a\x043V[`@Q\x80\x91\x03\x90\xFD[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x01\x8F\x94\x93\x92\x91\x90a\x04\xEAV[`@Q\x80\x91\x03\x90\xA3PPPPPV[`\x01\x80Ta\x01\xAB\x90a\x05\x1CV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x01\xD7\x90a\x05\x1CV[\x80\x15a\x02$W\x80`\x1F\x10a\x01\xF9Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x02$V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x02\x07W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\0a\x02Y\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\x11V[\x11a\x02vW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01F\x90a\x043V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x85\x85`@Qa\x02\xB2\x92\x91\x90a\x05VV[`@Q\x80\x91\x03\x90\xA3PPPV[`\0\x80Ta\x01\xAB\x90a\x05\x1CV[`\0\x80\x83`\x1F\x84\x01\x12a\x02\xDEW`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x02\xF6W`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x03\x0EW`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x03+W`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x03CW`\0\x80\xFD[a\x03O\x88\x83\x89\x01a\x02\xCCV[\x90\x96P\x94P` \x87\x015\x91P\x80\x82\x11\x15a\x03hW`\0\x80\xFD[Pa\x03u\x87\x82\x88\x01a\x02\xCCV[\x95\x98\x94\x97P\x95PPPPV[`\0` \x80\x83R\x83Q\x80\x82\x85\x01R`\0[\x81\x81\x10\x15a\x03\xAEW\x85\x81\x01\x83\x01Q\x85\x82\x01`@\x01R\x82\x01a\x03\x92V[P`\0`@\x82\x86\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x92PPP\x92\x91PPV[`\0\x80` \x83\x85\x03\x12\x15a\x03\xE2W`\0\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x03\xF9W`\0\x80\xFD[a\x04\x05\x85\x82\x86\x01a\x02\xCCV[\x90\x96\x90\x95P\x93PPPPV[`\0\x82a\x04.WcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x04\xFE`@\x83\x01\x86\x88a\x04\xC1V[\x82\x81\x03` \x84\x01Ra\x05\x11\x81\x85\x87a\x04\xC1V[\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x050W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x05PWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x81R`\0a\x05j` \x83\x01\x84\x86a\x04\xC1V[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 \t \xD0n0\xAB\xBF\xC4\xFB\xEC\xCAZ\x19\x17 \xDB\xA8\xDEE\x9E\x1B\xD5\x85O\xB2\xAAq\x80HU\x05\xC4dsolcC\0\x08\x15\x003";
    /// The deployed bytecode of the contract.
    pub static ASTRIAWITHDRAWER_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __DEPLOYED_BYTECODE,
    );
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
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    ASTRIAWITHDRAWER_ABI.clone(),
                    client,
                ),
            )
        }
        /// Constructs the general purpose `Deployer` instance based on the provided constructor arguments and sends it.
        /// Returns a new instance of a deployer that returns an instance of this contract after sending the transaction
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
        ///Calls the contract's `BASE_CHAIN_ASSET_DENOMINATION` (0xb6476c7e) function
        pub fn base_chain_asset_denomination(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
            self.0
                .method_hash([182, 71, 108, 126], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `BASE_CHAIN_ASSET_PRECISION` (0x7eb6dec7) function
        pub fn base_chain_asset_precision(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, u32> {
            self.0
                .method_hash([126, 182, 222, 199], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `BASE_CHAIN_BRIDGE_ADDRESS` (0xdb97dc98) function
        pub fn base_chain_bridge_address(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
            self.0
                .method_hash([219, 151, 220, 152], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `withdrawToIbcChain` (0xa996e020) function
        pub fn withdraw_to_ibc_chain(
            &self,
            destination_chain_address: ::std::string::String,
            memo: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([169, 150, 224, 32], (destination_chain_address, memo))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `withdrawToSequencer` (0xbab916d0) function
        pub fn withdraw_to_sequencer(
            &self,
            destination_chain_address: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([186, 185, 22, 208], destination_chain_address)
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `Ics20Withdrawal` event
        pub fn ics_20_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            Ics20WithdrawalFilter,
        > {
            self.0.event()
        }
        ///Gets the contract's `SequencerWithdrawal` event
        pub fn sequencer_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            SequencerWithdrawalFilter,
        > {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            AstriaWithdrawerEvents,
        > {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for AstriaWithdrawer<M> {
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
        Hash
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
        Hash
    )]
    #[ethevent(
        name = "SequencerWithdrawal",
        abi = "SequencerWithdrawal(address,uint256,string)"
    )]
    pub struct SequencerWithdrawalFilter {
        #[ethevent(indexed)]
        pub sender: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
    }
    ///Container type for all of the contract's events
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
                Self::Ics20WithdrawalFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::SequencerWithdrawalFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
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
    ///Container type for all input parameters for the `BASE_CHAIN_ASSET_DENOMINATION` function with signature `BASE_CHAIN_ASSET_DENOMINATION()` and selector `0xb6476c7e`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(
        name = "BASE_CHAIN_ASSET_DENOMINATION",
        abi = "BASE_CHAIN_ASSET_DENOMINATION()"
    )]
    pub struct BaseChainAssetDenominationCall;
    ///Container type for all input parameters for the `BASE_CHAIN_ASSET_PRECISION` function with signature `BASE_CHAIN_ASSET_PRECISION()` and selector `0x7eb6dec7`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "BASE_CHAIN_ASSET_PRECISION", abi = "BASE_CHAIN_ASSET_PRECISION()")]
    pub struct BaseChainAssetPrecisionCall;
    ///Container type for all input parameters for the `BASE_CHAIN_BRIDGE_ADDRESS` function with signature `BASE_CHAIN_BRIDGE_ADDRESS()` and selector `0xdb97dc98`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "BASE_CHAIN_BRIDGE_ADDRESS", abi = "BASE_CHAIN_BRIDGE_ADDRESS()")]
    pub struct BaseChainBridgeAddressCall;
    ///Container type for all input parameters for the `withdrawToIbcChain` function with signature `withdrawToIbcChain(string,string)` and selector `0xa996e020`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "withdrawToIbcChain", abi = "withdrawToIbcChain(string,string)")]
    pub struct WithdrawToIbcChainCall {
        pub destination_chain_address: ::std::string::String,
        pub memo: ::std::string::String,
    }
    ///Container type for all input parameters for the `withdrawToSequencer` function with signature `withdrawToSequencer(string)` and selector `0xbab916d0`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[ethcall(name = "withdrawToSequencer", abi = "withdrawToSequencer(string)")]
    pub struct WithdrawToSequencerCall {
        pub destination_chain_address: ::std::string::String,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaWithdrawerCalls {
        BaseChainAssetDenomination(BaseChainAssetDenominationCall),
        BaseChainAssetPrecision(BaseChainAssetPrecisionCall),
        BaseChainBridgeAddress(BaseChainBridgeAddressCall),
        WithdrawToIbcChain(WithdrawToIbcChainCall),
        WithdrawToSequencer(WithdrawToSequencerCall),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaWithdrawerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <BaseChainAssetDenominationCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::BaseChainAssetDenomination(decoded));
            }
            if let Ok(decoded) = <BaseChainAssetPrecisionCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::BaseChainAssetPrecision(decoded));
            }
            if let Ok(decoded) = <BaseChainBridgeAddressCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::BaseChainBridgeAddress(decoded));
            }
            if let Ok(decoded) = <WithdrawToIbcChainCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::WithdrawToIbcChain(decoded));
            }
            if let Ok(decoded) = <WithdrawToSequencerCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::WithdrawToSequencer(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for AstriaWithdrawerCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::BaseChainAssetDenomination(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::BaseChainAssetPrecision(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::BaseChainBridgeAddress(element) => {
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
                Self::BaseChainAssetDenomination(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::BaseChainAssetPrecision(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::BaseChainBridgeAddress(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::WithdrawToIbcChain(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::WithdrawToSequencer(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
            }
        }
    }
    impl ::core::convert::From<BaseChainAssetDenominationCall>
    for AstriaWithdrawerCalls {
        fn from(value: BaseChainAssetDenominationCall) -> Self {
            Self::BaseChainAssetDenomination(value)
        }
    }
    impl ::core::convert::From<BaseChainAssetPrecisionCall> for AstriaWithdrawerCalls {
        fn from(value: BaseChainAssetPrecisionCall) -> Self {
            Self::BaseChainAssetPrecision(value)
        }
    }
    impl ::core::convert::From<BaseChainBridgeAddressCall> for AstriaWithdrawerCalls {
        fn from(value: BaseChainBridgeAddressCall) -> Self {
            Self::BaseChainBridgeAddress(value)
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
    ///Container type for all return fields from the `BASE_CHAIN_ASSET_DENOMINATION` function with signature `BASE_CHAIN_ASSET_DENOMINATION()` and selector `0xb6476c7e`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct BaseChainAssetDenominationReturn(pub ::std::string::String);
    ///Container type for all return fields from the `BASE_CHAIN_ASSET_PRECISION` function with signature `BASE_CHAIN_ASSET_PRECISION()` and selector `0x7eb6dec7`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct BaseChainAssetPrecisionReturn(pub u32);
    ///Container type for all return fields from the `BASE_CHAIN_BRIDGE_ADDRESS` function with signature `BASE_CHAIN_BRIDGE_ADDRESS()` and selector `0xdb97dc98`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    pub struct BaseChainBridgeAddressReturn(pub ::std::string::String);
}
