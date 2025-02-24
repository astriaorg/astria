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
                    ::std::borrow::ToOwned::to_owned("withdrawToRollup"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("withdrawToRollup"),
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
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "destinationRollupBridgeAddress",
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
                    ::std::borrow::ToOwned::to_owned("RollupWithdrawal"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("RollupWithdrawal"),
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
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "destinationRollupBridgeAddress",
                                    ),
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
    const __BYTECODE: &[u8] = b"`\xC0`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x0BP8\x03\x80a\x0BP\x839\x81\x01`@\x81\x90Ra\0/\x91a\x01\xC5V[`\x12\x83c\xFF\xFF\xFF\xFF\x16\x11\x15a\0\xC6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`M`$\x82\x01R\x7FAstriaWithdrawer: base chain ass`D\x82\x01R\x7Fet precision must be less than o`d\x82\x01Rl\x0ED\x0C\xAE.\xAC-\x84\x0E\x8D\xE4\x06'`\x9B\x1B`\x84\x82\x01R`\xA4\x01`@Q\x80\x91\x03\x90\xFD[c\xFF\xFF\xFF\xFF\x83\x16`\x80R`\0a\0\xDC\x83\x82a\x02\xD3V[P`\x01a\0\xE9\x82\x82a\x02\xD3V[Pa\0\xF5\x83`\x12a\x03\xA7V[a\x01\0\x90`\na\x04\xB0V[`\xA0RPa\x04\xC9\x91PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0\x82`\x1F\x83\x01\x12a\x013W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x01LWa\x01La\x01\x0CV[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x01zWa\x01za\x01\x0CV[`@R\x81\x81R\x83\x82\x01` \x01\x85\x10\x15a\x01\x92W`\0\x80\xFD[`\0[\x82\x81\x10\x15a\x01\xB1W` \x81\x86\x01\x81\x01Q\x83\x83\x01\x82\x01R\x01a\x01\x95V[P`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\x01\xDAW`\0\x80\xFD[\x83Qc\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x01\xEEW`\0\x80\xFD[` \x85\x01Q\x90\x93P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02\nW`\0\x80\xFD[a\x02\x16\x86\x82\x87\x01a\x01\"V[`@\x86\x01Q\x90\x93P\x90P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x024W`\0\x80\xFD[a\x02@\x86\x82\x87\x01a\x01\"V[\x91PP\x92P\x92P\x92V[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x02^W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x02~WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15a\x02\xCEW\x80`\0R` `\0 `\x1F\x84\x01`\x05\x1C\x81\x01` \x85\x10\x15a\x02\xABWP\x80[`\x1F\x84\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15a\x02\xCBW`\0\x81U`\x01\x01a\x02\xB7V[PP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02\xECWa\x02\xECa\x01\x0CV[a\x03\0\x81a\x02\xFA\x84Ta\x02JV[\x84a\x02\x84V[` `\x1F\x82\x11`\x01\x81\x14a\x034W`\0\x83\x15a\x03\x1CWP\x84\x82\x01Q[`\0\x19`\x03\x85\x90\x1B\x1C\x19\x16`\x01\x84\x90\x1B\x17\x84Ua\x02\xCBV[`\0\x84\x81R` \x81 `\x1F\x19\x85\x16\x91[\x82\x81\x10\x15a\x03dW\x87\x85\x01Q\x82U` \x94\x85\x01\x94`\x01\x90\x92\x01\x91\x01a\x03DV[P\x84\x82\x10\x15a\x03\x82W\x86\x84\x01Q`\0\x19`\x03\x87\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPP`\x01\x90\x81\x1B\x01\x90UPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[c\xFF\xFF\xFF\xFF\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\x03\xC3Wa\x03\xC3a\x03\x91V[\x92\x91PPV[`\x01\x81[`\x01\x84\x11\x15a\x04\x04W\x80\x85\x04\x81\x11\x15a\x03\xE8Wa\x03\xE8a\x03\x91V[`\x01\x84\x16\x15a\x03\xF6W\x90\x81\x02\x90[`\x01\x93\x90\x93\x1C\x92\x80\x02a\x03\xCDV[\x93P\x93\x91PPV[`\0\x82a\x04\x1BWP`\x01a\x03\xC3V[\x81a\x04(WP`\0a\x03\xC3V[\x81`\x01\x81\x14a\x04>W`\x02\x81\x14a\x04HWa\x04dV[`\x01\x91PPa\x03\xC3V[`\xFF\x84\x11\x15a\x04YWa\x04Ya\x03\x91V[PP`\x01\x82\x1Ba\x03\xC3V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x04\x87WP\x81\x81\na\x03\xC3V[a\x04\x94`\0\x19\x84\x84a\x03\xC9V[\x80`\0\x19\x04\x82\x11\x15a\x04\xA8Wa\x04\xA8a\x03\x91V[\x02\x93\x92PPPV[`\0a\x04\xC2c\xFF\xFF\xFF\xFF\x84\x16\x83a\x04\x0CV[\x93\x92PPPV[`\x80Q`\xA0Qa\x06Ua\x04\xFB`\09`\0\x81\x81a\x01\"\x01R\x81\x81a\x01\xC4\x01Ra\x02\xDC\x01R`\0`\x81\x01Ra\x06U`\0\xF3\xFE`\x80`@R`\x046\x10a\0UW`\x005`\xE0\x1C\x80c\x05n\x0C\xEB\x14a\0ZW\x80c~\xB6\xDE\xC7\x14a\0oW\x80c\xA9\x96\xE0 \x14a\0\xBDW\x80c\xB6Gl~\x14a\0\xD0W\x80c\xBA\xB9\x16\xD0\x14a\0\xF2W\x80c\xDB\x97\xDC\x98\x14a\x01\x05W[`\0\x80\xFD[a\0ma\0h6`\x04a\x03\xBDV[a\x01\x1AV[\0[4\x80\x15a\0{W`\0\x80\xFD[Pa\0\xA3\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0ma\0\xCB6`\x04a\x03\xBDV[a\x01\xBCV[4\x80\x15a\0\xDCW`\0\x80\xFD[Pa\0\xE5a\x02FV[`@Qa\0\xB4\x91\x90a\x04.V[a\0ma\x01\x006`\x04a\x04|V[a\x02\xD4V[4\x80\x15a\x01\x11W`\0\x80\xFD[Pa\0\xE5a\x03gV[4`\0a\x01G\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\xBEV[\x11a\x01mW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01d\x90a\x04\xE0V[`@Q\x80\x91\x03\x90\xFD[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE2\xA9\xF2nJR8\xEC\tr\xCE>\xD7'1!.\xBC\x8D'i\xCF\x7F\x17\xD8\xBE\xD2\x8A\x0F\xC8i\xF5\x87\x87\x87\x87`@Qa\x01\xAD\x94\x93\x92\x91\x90a\x05\x97V[`@Q\x80\x91\x03\x90\xA3PPPPPV[4`\0a\x01\xE9\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\xBEV[\x11a\x02\x06W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01d\x90a\x04\xE0V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x01\xAD\x94\x93\x92\x91\x90a\x05\x97V[`\x01\x80Ta\x02S\x90a\x05\xC9V[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\x7F\x90a\x05\xC9V[\x80\x15a\x02\xCCW\x80`\x1F\x10a\x02\xA1Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x02\xCCV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x02\xAFW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\0a\x03\x01\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\xBEV[\x11a\x03\x1EW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01d\x90a\x04\xE0V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x85\x85`@Qa\x03Z\x92\x91\x90a\x06\x03V[`@Q\x80\x91\x03\x90\xA3PPPV[`\0\x80Ta\x02S\x90a\x05\xC9V[`\0\x80\x83`\x1F\x84\x01\x12a\x03\x86W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x03\x9EW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x03\xB6W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x03\xD3W`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x03\xEAW`\0\x80\xFD[a\x03\xF6\x87\x82\x88\x01a\x03tV[\x90\x95P\x93PP` \x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x04\x16W`\0\x80\xFD[a\x04\"\x87\x82\x88\x01a\x03tV[\x95\x98\x94\x97P\x95PPPPV[` \x81R`\0\x82Q\x80` \x84\x01R`\0[\x81\x81\x10\x15a\x04\\W` \x81\x86\x01\x81\x01Q`@\x86\x84\x01\x01R\x01a\x04?V[P`\0`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[`\0\x80` \x83\x85\x03\x12\x15a\x04\x8FW`\0\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x04\xA6W`\0\x80\xFD[a\x04\xB2\x85\x82\x86\x01a\x03tV[\x90\x96\x90\x95P\x93PPPPV[`\0\x82a\x04\xDBWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x05\xAB`@\x83\x01\x86\x88a\x05nV[\x82\x81\x03` \x84\x01Ra\x05\xBE\x81\x85\x87a\x05nV[\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x05\xDDW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x05\xFDWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x81R`\0a\x06\x17` \x83\x01\x84\x86a\x05nV[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 \x86\x99i,\xA0\xE5\x13]\x8Bi\xEF\x038\x14\xBC4\x97\x9A\x1B\xD1A\x9D\x15\x07\x8E\xA0)\xFC\x9D\xAB\xD1\xFFdsolcC\0\x08\x1A\x003";
    /// The bytecode of the contract.
    pub static ASTRIAWITHDRAWER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0UW`\x005`\xE0\x1C\x80c\x05n\x0C\xEB\x14a\0ZW\x80c~\xB6\xDE\xC7\x14a\0oW\x80c\xA9\x96\xE0 \x14a\0\xBDW\x80c\xB6Gl~\x14a\0\xD0W\x80c\xBA\xB9\x16\xD0\x14a\0\xF2W\x80c\xDB\x97\xDC\x98\x14a\x01\x05W[`\0\x80\xFD[a\0ma\0h6`\x04a\x03\xBDV[a\x01\x1AV[\0[4\x80\x15a\0{W`\0\x80\xFD[Pa\0\xA3\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0ma\0\xCB6`\x04a\x03\xBDV[a\x01\xBCV[4\x80\x15a\0\xDCW`\0\x80\xFD[Pa\0\xE5a\x02FV[`@Qa\0\xB4\x91\x90a\x04.V[a\0ma\x01\x006`\x04a\x04|V[a\x02\xD4V[4\x80\x15a\x01\x11W`\0\x80\xFD[Pa\0\xE5a\x03gV[4`\0a\x01G\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\xBEV[\x11a\x01mW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01d\x90a\x04\xE0V[`@Q\x80\x91\x03\x90\xFD[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE2\xA9\xF2nJR8\xEC\tr\xCE>\xD7'1!.\xBC\x8D'i\xCF\x7F\x17\xD8\xBE\xD2\x8A\x0F\xC8i\xF5\x87\x87\x87\x87`@Qa\x01\xAD\x94\x93\x92\x91\x90a\x05\x97V[`@Q\x80\x91\x03\x90\xA3PPPPPV[4`\0a\x01\xE9\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\xBEV[\x11a\x02\x06W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01d\x90a\x04\xE0V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x01\xAD\x94\x93\x92\x91\x90a\x05\x97V[`\x01\x80Ta\x02S\x90a\x05\xC9V[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\x7F\x90a\x05\xC9V[\x80\x15a\x02\xCCW\x80`\x1F\x10a\x02\xA1Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x02\xCCV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x02\xAFW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\0a\x03\x01\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x04\xBEV[\x11a\x03\x1EW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x01d\x90a\x04\xE0V[43`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x85\x85`@Qa\x03Z\x92\x91\x90a\x06\x03V[`@Q\x80\x91\x03\x90\xA3PPPV[`\0\x80Ta\x02S\x90a\x05\xC9V[`\0\x80\x83`\x1F\x84\x01\x12a\x03\x86W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x03\x9EW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x03\xB6W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x03\xD3W`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x03\xEAW`\0\x80\xFD[a\x03\xF6\x87\x82\x88\x01a\x03tV[\x90\x95P\x93PP` \x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x04\x16W`\0\x80\xFD[a\x04\"\x87\x82\x88\x01a\x03tV[\x95\x98\x94\x97P\x95PPPPV[` \x81R`\0\x82Q\x80` \x84\x01R`\0[\x81\x81\x10\x15a\x04\\W` \x81\x86\x01\x81\x01Q`@\x86\x84\x01\x01R\x01a\x04?V[P`\0`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[`\0\x80` \x83\x85\x03\x12\x15a\x04\x8FW`\0\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x04\xA6W`\0\x80\xFD[a\x04\xB2\x85\x82\x86\x01a\x03tV[\x90\x96\x90\x95P\x93PPPPV[`\0\x82a\x04\xDBWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x05\xAB`@\x83\x01\x86\x88a\x05nV[\x82\x81\x03` \x84\x01Ra\x05\xBE\x81\x85\x87a\x05nV[\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x05\xDDW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x05\xFDWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x81R`\0a\x06\x17` \x83\x01\x84\x86a\x05nV[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 \x86\x99i,\xA0\xE5\x13]\x8Bi\xEF\x038\x14\xBC4\x97\x9A\x1B\xD1A\x9D\x15\x07\x8E\xA0)\xFC\x9D\xAB\xD1\xFFdsolcC\0\x08\x1A\x003";
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
        ///Calls the contract's `withdrawToRollup` (0x056e0ceb) function
        pub fn withdraw_to_rollup(
            &self,
            destination_chain_address: ::std::string::String,
            destination_rollup_bridge_address: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [5, 110, 12, 235],
                    (destination_chain_address, destination_rollup_bridge_address),
                )
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
        ///Gets the contract's `RollupWithdrawal` event
        pub fn rollup_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            RollupWithdrawalFilter,
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
        name = "RollupWithdrawal",
        abi = "RollupWithdrawal(address,uint256,string,string)"
    )]
    pub struct RollupWithdrawalFilter {
        #[ethevent(indexed)]
        pub sender: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
        pub destination_rollup_bridge_address: ::std::string::String,
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
        RollupWithdrawalFilter(RollupWithdrawalFilter),
        SequencerWithdrawalFilter(SequencerWithdrawalFilter),
    }
    impl ::ethers::contract::EthLogDecode for AstriaWithdrawerEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = Ics20WithdrawalFilter::decode_log(log) {
                return Ok(AstriaWithdrawerEvents::Ics20WithdrawalFilter(decoded));
            }
            if let Ok(decoded) = RollupWithdrawalFilter::decode_log(log) {
                return Ok(AstriaWithdrawerEvents::RollupWithdrawalFilter(decoded));
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
                Self::RollupWithdrawalFilter(element) => {
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
    impl ::core::convert::From<RollupWithdrawalFilter> for AstriaWithdrawerEvents {
        fn from(value: RollupWithdrawalFilter) -> Self {
            Self::RollupWithdrawalFilter(value)
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
    ///Container type for all input parameters for the `withdrawToRollup` function with signature `withdrawToRollup(string,string)` and selector `0x056e0ceb`
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
    #[ethcall(name = "withdrawToRollup", abi = "withdrawToRollup(string,string)")]
    pub struct WithdrawToRollupCall {
        pub destination_chain_address: ::std::string::String,
        pub destination_rollup_bridge_address: ::std::string::String,
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
        WithdrawToRollup(WithdrawToRollupCall),
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
            if let Ok(decoded) = <WithdrawToRollupCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::WithdrawToRollup(decoded));
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
                Self::WithdrawToRollup(element) => {
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
                Self::WithdrawToRollup(element) => ::core::fmt::Display::fmt(element, f),
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
    impl ::core::convert::From<WithdrawToRollupCall> for AstriaWithdrawerCalls {
        fn from(value: WithdrawToRollupCall) -> Self {
            Self::WithdrawToRollup(value)
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
