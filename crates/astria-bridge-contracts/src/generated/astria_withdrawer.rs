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
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned(
                            "_sequencerWithdrawalFee",
                        ),
                        kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("uint256"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_ibcWithdrawalFee"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("uint256"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_feeRecipient"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("ACCUMULATED_FEES"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("ACCUMULATED_FEES"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
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
                    ::std::borrow::ToOwned::to_owned("FEE_RECIPIENT"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("FEE_RECIPIENT"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("IBC_WITHDRAWAL_FEE"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("IBC_WITHDRAWAL_FEE"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("SEQUENCER_WITHDRAWAL_FEE"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "SEQUENCER_WITHDRAWAL_FEE",
                            ),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("claimFees"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("claimFees"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("owner"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("owner"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("renounceOwnership"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("renounceOwnership"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setFeeRecipient"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("setFeeRecipient"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_newFeeRecipient"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setIbcWithdrawalFee"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "setIbcWithdrawalFee",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_newFee"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("setSequencerWithdrawalFee"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "setSequencerWithdrawalFee",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_newFee"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("transferOwnership"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("transferOwnership"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("newOwner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
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
                    ::std::borrow::ToOwned::to_owned("OwnershipTransferred"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned(
                                "OwnershipTransferred",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("previousOwner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("newOwner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
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
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("OwnableInvalidOwner"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "OwnableInvalidOwner",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("owner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("OwnableUnauthorizedAccount"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "OwnableUnauthorizedAccount",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("account"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
                        },
                    ],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static ASTRIAWITHDRAWER_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\xC0`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x1208\x03\x80a\x120\x839\x81\x01`@\x81\x90Ra\0/\x91a\x02jV[3\x80a\0VW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\0_\x81a\x01aV[P`\x12\x86c\xFF\xFF\xFF\xFF\x16\x11\x15a\0\xF3W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`M`$\x82\x01R\x7FAstriaWithdrawer: base chain ass`D\x82\x01R\x7Fet precision must be less than o`d\x82\x01Rl\x0ED\x0C\xAE.\xAC-\x84\x0E\x8D\xE4\x06'`\x9B\x1B`\x84\x82\x01R`\xA4\x01a\0MV[c\xFF\xFF\xFF\xFF\x86\x16`\x80R`\x01a\x01\t\x86\x82a\x03\xAEV[P`\x02a\x01\x16\x85\x82a\x03\xAEV[Pa\x01\"\x86`\x12a\x04\x82V[a\x01-\x90`\na\x05\x8BV[`\xA0R`\x03\x92\x90\x92U`\x04U`\x05\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x90\x92\x16\x91\x90\x91\x17\x90UPa\x05\xA4\x91PPV[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0\x82`\x1F\x83\x01\x12a\x01\xD8W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x01\xF1Wa\x01\xF1a\x01\xB1V[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x02\x1FWa\x02\x1Fa\x01\xB1V[`@R\x81\x81R\x83\x82\x01` \x01\x85\x10\x15a\x027W`\0\x80\xFD[`\0[\x82\x81\x10\x15a\x02VW` \x81\x86\x01\x81\x01Q\x83\x83\x01\x82\x01R\x01a\x02:V[P`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[`\0\x80`\0\x80`\0\x80`\xC0\x87\x89\x03\x12\x15a\x02\x83W`\0\x80\xFD[\x86Qc\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x02\x97W`\0\x80\xFD[` \x88\x01Q\x90\x96P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02\xB3W`\0\x80\xFD[a\x02\xBF\x89\x82\x8A\x01a\x01\xC7V[`@\x89\x01Q\x90\x96P\x90P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02\xDDW`\0\x80\xFD[a\x02\xE9\x89\x82\x8A\x01a\x01\xC7V[``\x89\x01Q`\x80\x8A\x01Q`\xA0\x8B\x01Q\x92\x97P\x90\x95P\x93P\x90P`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x03\x17W`\0\x80\xFD[\x80\x91PP\x92\x95P\x92\x95P\x92\x95V[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x039W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x03YWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15a\x03\xA9W\x80`\0R` `\0 `\x1F\x84\x01`\x05\x1C\x81\x01` \x85\x10\x15a\x03\x86WP\x80[`\x1F\x84\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15a\x03\xA6W`\0\x81U`\x01\x01a\x03\x92V[PP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03\xC7Wa\x03\xC7a\x01\xB1V[a\x03\xDB\x81a\x03\xD5\x84Ta\x03%V[\x84a\x03_V[` `\x1F\x82\x11`\x01\x81\x14a\x04\x0FW`\0\x83\x15a\x03\xF7WP\x84\x82\x01Q[`\0\x19`\x03\x85\x90\x1B\x1C\x19\x16`\x01\x84\x90\x1B\x17\x84Ua\x03\xA6V[`\0\x84\x81R` \x81 `\x1F\x19\x85\x16\x91[\x82\x81\x10\x15a\x04?W\x87\x85\x01Q\x82U` \x94\x85\x01\x94`\x01\x90\x92\x01\x91\x01a\x04\x1FV[P\x84\x82\x10\x15a\x04]W\x86\x84\x01Q`\0\x19`\x03\x87\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPP`\x01\x90\x81\x1B\x01\x90UPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[c\xFF\xFF\xFF\xFF\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\x04\x9EWa\x04\x9Ea\x04lV[\x92\x91PPV[`\x01\x81[`\x01\x84\x11\x15a\x04\xDFW\x80\x85\x04\x81\x11\x15a\x04\xC3Wa\x04\xC3a\x04lV[`\x01\x84\x16\x15a\x04\xD1W\x90\x81\x02\x90[`\x01\x93\x90\x93\x1C\x92\x80\x02a\x04\xA8V[\x93P\x93\x91PPV[`\0\x82a\x04\xF6WP`\x01a\x04\x9EV[\x81a\x05\x03WP`\0a\x04\x9EV[\x81`\x01\x81\x14a\x05\x19W`\x02\x81\x14a\x05#Wa\x05?V[`\x01\x91PPa\x04\x9EV[`\xFF\x84\x11\x15a\x054Wa\x054a\x04lV[PP`\x01\x82\x1Ba\x04\x9EV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x05bWP\x81\x81\na\x04\x9EV[a\x05o`\0\x19\x84\x84a\x04\xA4V[\x80`\0\x19\x04\x82\x11\x15a\x05\x83Wa\x05\x83a\x04lV[\x02\x93\x92PPPV[`\0a\x05\x9Dc\xFF\xFF\xFF\xFF\x84\x16\x83a\x04\xE7V[\x93\x92PPPV[`\x80Q`\xA0Qa\x0CYa\x05\xD7`\09`\0\x81\x81a\x03?\x01R\x81\x81a\x04Y\x01Ra\x05\xC3\x01R`\0a\x01h\x01Ra\x0CY`\0\xF3\xFE`\x80`@R`\x046\x10a\0\xFEW`\x005`\xE0\x1C\x80c\xAD\"\x82G\x11a\0\x95W\x80c\xDB\x97\xDC\x98\x11a\0dW\x80c\xDB\x97\xDC\x98\x14a\x02\x84W\x80c\xE7K\x98\x1B\x14a\x02\x99W\x80c\xEB\xD0\x90T\x14a\x02\xB9W\x80c\xF2\xFD\xE3\x8B\x14a\x02\xD9W\x80c\xFC\x88\xD3\x1B\x14a\x02\xF9W`\0\x80\xFD[\x80c\xAD\"\x82G\x14a\x02$W\x80c\xB6Gl~\x14a\x02:W\x80c\xBA\xB9\x16\xD0\x14a\x02\\W\x80c\xD2\x94\xF0\x93\x14a\x02oW`\0\x80\xFD[\x80c\x88\x979y\x11a\0\xD1W\x80c\x88\x979y\x14a\x01\x9FW\x80c\x8D\xA5\xCB[\x14a\x01\xBFW\x80c\xA7\xEA\xA79\x14a\x01\xF1W\x80c\xA9\x96\xE0 \x14a\x02\x11W`\0\x80\xFD[\x80c\x05n\x0C\xEB\x14a\x01\x03W\x80coF8J\x14a\x01\x18W\x80cqP\x18\xA6\x14a\x01AW\x80c~\xB6\xDE\xC7\x14a\x01VW[`\0\x80\xFD[a\x01\x16a\x01\x116`\x04a\x08\xE9V[a\x03\x0FV[\0[4\x80\x15a\x01$W`\0\x80\xFD[Pa\x01.`\x03T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01MW`\0\x80\xFD[Pa\x01\x16a\x04\x04V[4\x80\x15a\x01bW`\0\x80\xFD[Pa\x01\x8A\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x018V[4\x80\x15a\x01\xABW`\0\x80\xFD[Pa\x01\x16a\x01\xBA6`\x04a\tZV[a\x04\x18V[4\x80\x15a\x01\xCBW`\0\x80\xFD[P`\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x018V[4\x80\x15a\x01\xFDW`\0\x80\xFD[Pa\x01\x16a\x02\x0C6`\x04a\tZV[a\x04%V[a\x01\x16a\x02\x1F6`\x04a\x08\xE9V[a\x042V[4\x80\x15a\x020W`\0\x80\xFD[Pa\x01.`\x06T\x81V[4\x80\x15a\x02FW`\0\x80\xFD[Pa\x02Oa\x05\x0EV[`@Qa\x018\x91\x90a\tsV[a\x01\x16a\x02j6`\x04a\t\xC1V[a\x05\x9CV[4\x80\x15a\x02{W`\0\x80\xFD[Pa\x01\x16a\x06\x82V[4\x80\x15a\x02\x90W`\0\x80\xFD[Pa\x02Oa\x07\xAEV[4\x80\x15a\x02\xA5W`\0\x80\xFD[Pa\x01\x16a\x02\xB46`\x04a\n\x03V[a\x07\xBBV[4\x80\x15a\x02\xC5W`\0\x80\xFD[P`\x05Ta\x01\xD9\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x02\xE5W`\0\x80\xFD[Pa\x01\x16a\x02\xF46`\x04a\n\x03V[a\x07\xE5V[4\x80\x15a\x03\x05W`\0\x80\xFD[Pa\x01.`\x04T\x81V[4`\x03T\x80\x82\x11a\x03;W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n3V[`@Q\x80\x91\x03\x90\xFD[`\0\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0a\x03h\x83\x85a\n\x96V[a\x03r\x91\x90a\n\xAFV[\x11a\x03\x8FW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n\xD1V[`\x03T`\x06`\0\x82\x82Ta\x03\xA3\x91\x90a\x0B_V[\x90\x91UPP`\x03Ta\x03\xB5\x904a\n\x96V[3`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE2\xA9\xF2nJR8\xEC\tr\xCE>\xD7'1!.\xBC\x8D'i\xCF\x7F\x17\xD8\xBE\xD2\x8A\x0F\xC8i\xF5\x88\x88\x88\x88`@Qa\x03\xF4\x94\x93\x92\x91\x90a\x0B\x9BV[`@Q\x80\x91\x03\x90\xA3PPPPPPV[a\x04\x0Ca\x08#V[a\x04\x16`\0a\x08PV[V[a\x04 a\x08#V[`\x04UV[a\x04-a\x08#V[`\x03UV[4`\x04T\x80\x82\x11a\x04UW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n3V[`\0\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0a\x04\x82\x83\x85a\n\x96V[a\x04\x8C\x91\x90a\n\xAFV[\x11a\x04\xA9W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n\xD1V[`\x04T`\x06`\0\x82\x82Ta\x04\xBD\x91\x90a\x0B_V[\x90\x91UPP`\x04Ta\x04\xCF\x904a\n\x96V[3`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x88\x88\x88\x88`@Qa\x03\xF4\x94\x93\x92\x91\x90a\x0B\x9BV[`\x02\x80Ta\x05\x1B\x90a\x0B\xCDV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05G\x90a\x0B\xCDV[\x80\x15a\x05\x94W\x80`\x1F\x10a\x05iWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x05\x94V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x05wW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\x03T\x80\x82\x11a\x05\xBFW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n3V[`\0\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0a\x05\xEC\x83\x85a\n\x96V[a\x05\xF6\x91\x90a\n\xAFV[\x11a\x06\x13W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n\xD1V[`\x03T`\x06`\0\x82\x82Ta\x06'\x91\x90a\x0B_V[\x90\x91UPP`\x03Ta\x069\x904a\n\x96V[3`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x86\x86`@Qa\x06t\x92\x91\x90a\x0C\x07V[`@Q\x80\x91\x03\x90\xA3PPPPV[`\x05T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x06\xEEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`)`$\x82\x01R\x7FAstriaBridgeableERC20: only fee `D\x82\x01Rh\x1C\x99X\xDA\\\x1AY[\x9D`\xBA\x1B`d\x82\x01R`\x84\x01a\x032V[`\x05T`\x06T`@Q`\0\x92`\x01`\x01`\xA0\x1B\x03\x16\x91\x90\x83\x81\x81\x81\x85\x87Z\xF1\x92PPP=\x80`\0\x81\x14a\x07=W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=`\0` \x84\x01>a\x07BV[``\x91P[PP\x90P\x80a\x07\xA6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`*`$\x82\x01R\x7FAstriaBridgeableERC20: fee trans`D\x82\x01Ri\x19\x99\\\x88\x19\x98Z[\x19Y`\xB2\x1B`d\x82\x01R`\x84\x01a\x032V[P`\0`\x06UV[`\x01\x80Ta\x05\x1B\x90a\x0B\xCDV[a\x07\xC3a\x08#V[`\x05\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[a\x07\xEDa\x08#V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x08\x17W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x032V[a\x08 \x81a\x08PV[PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x04\x16W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x032V[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\0\x80\x83`\x1F\x84\x01\x12a\x08\xB2W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x08\xCAW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x08\xE2W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x08\xFFW`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\t\x16W`\0\x80\xFD[a\t\"\x87\x82\x88\x01a\x08\xA0V[\x90\x95P\x93PP` \x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\tBW`\0\x80\xFD[a\tN\x87\x82\x88\x01a\x08\xA0V[\x95\x98\x94\x97P\x95PPPPV[`\0` \x82\x84\x03\x12\x15a\tlW`\0\x80\xFD[P5\x91\x90PV[` \x81R`\0\x82Q\x80` \x84\x01R`\0[\x81\x81\x10\x15a\t\xA1W` \x81\x86\x01\x81\x01Q`@\x86\x84\x01\x01R\x01a\t\x84V[P`\0`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[`\0\x80` \x83\x85\x03\x12\x15a\t\xD4W`\0\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\t\xEBW`\0\x80\xFD[a\t\xF7\x85\x82\x86\x01a\x08\xA0V[\x90\x96\x90\x95P\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\n\x15W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n,W`\0\x80\xFD[\x93\x92PPPV[` \x80\x82R`-\x90\x82\x01R\x7FAstriaWithdrawer: insufficient w`@\x82\x01Rlithdrawal fee`\x98\x1B``\x82\x01R`\x80\x01\x90V[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x81\x81\x03\x81\x81\x11\x15a\n\xA9Wa\n\xA9a\n\x80V[\x92\x91PPV[`\0\x82a\n\xCCWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x80\x82\x01\x80\x82\x11\x15a\n\xA9Wa\n\xA9a\n\x80V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x0B\xAF`@\x83\x01\x86\x88a\x0BrV[\x82\x81\x03` \x84\x01Ra\x0B\xC2\x81\x85\x87a\x0BrV[\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x0B\xE1W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x0C\x01WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x81R`\0a\x0C\x1B` \x83\x01\x84\x86a\x0BrV[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 \xEA\xC3\x07<\x1Ba\xF0Vx\x9F\xC4\xFAbU+.\xC8.\xDF\xC9\xF0\x03bk\x8C\xAEX\xE1\xD3\xF6\xE5?dsolcC\0\x08\x1A\x003";
    /// The bytecode of the contract.
    pub static ASTRIAWITHDRAWER_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\0\xFEW`\x005`\xE0\x1C\x80c\xAD\"\x82G\x11a\0\x95W\x80c\xDB\x97\xDC\x98\x11a\0dW\x80c\xDB\x97\xDC\x98\x14a\x02\x84W\x80c\xE7K\x98\x1B\x14a\x02\x99W\x80c\xEB\xD0\x90T\x14a\x02\xB9W\x80c\xF2\xFD\xE3\x8B\x14a\x02\xD9W\x80c\xFC\x88\xD3\x1B\x14a\x02\xF9W`\0\x80\xFD[\x80c\xAD\"\x82G\x14a\x02$W\x80c\xB6Gl~\x14a\x02:W\x80c\xBA\xB9\x16\xD0\x14a\x02\\W\x80c\xD2\x94\xF0\x93\x14a\x02oW`\0\x80\xFD[\x80c\x88\x979y\x11a\0\xD1W\x80c\x88\x979y\x14a\x01\x9FW\x80c\x8D\xA5\xCB[\x14a\x01\xBFW\x80c\xA7\xEA\xA79\x14a\x01\xF1W\x80c\xA9\x96\xE0 \x14a\x02\x11W`\0\x80\xFD[\x80c\x05n\x0C\xEB\x14a\x01\x03W\x80coF8J\x14a\x01\x18W\x80cqP\x18\xA6\x14a\x01AW\x80c~\xB6\xDE\xC7\x14a\x01VW[`\0\x80\xFD[a\x01\x16a\x01\x116`\x04a\x08\xE9V[a\x03\x0FV[\0[4\x80\x15a\x01$W`\0\x80\xFD[Pa\x01.`\x03T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01MW`\0\x80\xFD[Pa\x01\x16a\x04\x04V[4\x80\x15a\x01bW`\0\x80\xFD[Pa\x01\x8A\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x018V[4\x80\x15a\x01\xABW`\0\x80\xFD[Pa\x01\x16a\x01\xBA6`\x04a\tZV[a\x04\x18V[4\x80\x15a\x01\xCBW`\0\x80\xFD[P`\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x018V[4\x80\x15a\x01\xFDW`\0\x80\xFD[Pa\x01\x16a\x02\x0C6`\x04a\tZV[a\x04%V[a\x01\x16a\x02\x1F6`\x04a\x08\xE9V[a\x042V[4\x80\x15a\x020W`\0\x80\xFD[Pa\x01.`\x06T\x81V[4\x80\x15a\x02FW`\0\x80\xFD[Pa\x02Oa\x05\x0EV[`@Qa\x018\x91\x90a\tsV[a\x01\x16a\x02j6`\x04a\t\xC1V[a\x05\x9CV[4\x80\x15a\x02{W`\0\x80\xFD[Pa\x01\x16a\x06\x82V[4\x80\x15a\x02\x90W`\0\x80\xFD[Pa\x02Oa\x07\xAEV[4\x80\x15a\x02\xA5W`\0\x80\xFD[Pa\x01\x16a\x02\xB46`\x04a\n\x03V[a\x07\xBBV[4\x80\x15a\x02\xC5W`\0\x80\xFD[P`\x05Ta\x01\xD9\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x02\xE5W`\0\x80\xFD[Pa\x01\x16a\x02\xF46`\x04a\n\x03V[a\x07\xE5V[4\x80\x15a\x03\x05W`\0\x80\xFD[Pa\x01.`\x04T\x81V[4`\x03T\x80\x82\x11a\x03;W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n3V[`@Q\x80\x91\x03\x90\xFD[`\0\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0a\x03h\x83\x85a\n\x96V[a\x03r\x91\x90a\n\xAFV[\x11a\x03\x8FW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n\xD1V[`\x03T`\x06`\0\x82\x82Ta\x03\xA3\x91\x90a\x0B_V[\x90\x91UPP`\x03Ta\x03\xB5\x904a\n\x96V[3`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE2\xA9\xF2nJR8\xEC\tr\xCE>\xD7'1!.\xBC\x8D'i\xCF\x7F\x17\xD8\xBE\xD2\x8A\x0F\xC8i\xF5\x88\x88\x88\x88`@Qa\x03\xF4\x94\x93\x92\x91\x90a\x0B\x9BV[`@Q\x80\x91\x03\x90\xA3PPPPPPV[a\x04\x0Ca\x08#V[a\x04\x16`\0a\x08PV[V[a\x04 a\x08#V[`\x04UV[a\x04-a\x08#V[`\x03UV[4`\x04T\x80\x82\x11a\x04UW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n3V[`\0\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0a\x04\x82\x83\x85a\n\x96V[a\x04\x8C\x91\x90a\n\xAFV[\x11a\x04\xA9W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n\xD1V[`\x04T`\x06`\0\x82\x82Ta\x04\xBD\x91\x90a\x0B_V[\x90\x91UPP`\x04Ta\x04\xCF\x904a\n\x96V[3`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x88\x88\x88\x88`@Qa\x03\xF4\x94\x93\x92\x91\x90a\x0B\x9BV[`\x02\x80Ta\x05\x1B\x90a\x0B\xCDV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05G\x90a\x0B\xCDV[\x80\x15a\x05\x94W\x80`\x1F\x10a\x05iWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x05\x94V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x05wW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\x03T\x80\x82\x11a\x05\xBFW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n3V[`\0\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0a\x05\xEC\x83\x85a\n\x96V[a\x05\xF6\x91\x90a\n\xAFV[\x11a\x06\x13W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x032\x90a\n\xD1V[`\x03T`\x06`\0\x82\x82Ta\x06'\x91\x90a\x0B_V[\x90\x91UPP`\x03Ta\x069\x904a\n\x96V[3`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x86\x86`@Qa\x06t\x92\x91\x90a\x0C\x07V[`@Q\x80\x91\x03\x90\xA3PPPPV[`\x05T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x06\xEEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`)`$\x82\x01R\x7FAstriaBridgeableERC20: only fee `D\x82\x01Rh\x1C\x99X\xDA\\\x1AY[\x9D`\xBA\x1B`d\x82\x01R`\x84\x01a\x032V[`\x05T`\x06T`@Q`\0\x92`\x01`\x01`\xA0\x1B\x03\x16\x91\x90\x83\x81\x81\x81\x85\x87Z\xF1\x92PPP=\x80`\0\x81\x14a\x07=W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=`\0` \x84\x01>a\x07BV[``\x91P[PP\x90P\x80a\x07\xA6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`*`$\x82\x01R\x7FAstriaBridgeableERC20: fee trans`D\x82\x01Ri\x19\x99\\\x88\x19\x98Z[\x19Y`\xB2\x1B`d\x82\x01R`\x84\x01a\x032V[P`\0`\x06UV[`\x01\x80Ta\x05\x1B\x90a\x0B\xCDV[a\x07\xC3a\x08#V[`\x05\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[a\x07\xEDa\x08#V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x08\x17W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x032V[a\x08 \x81a\x08PV[PV[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x04\x16W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x032V[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\0\x80\x83`\x1F\x84\x01\x12a\x08\xB2W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x08\xCAW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x08\xE2W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`@\x85\x87\x03\x12\x15a\x08\xFFW`\0\x80\xFD[\x845g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\t\x16W`\0\x80\xFD[a\t\"\x87\x82\x88\x01a\x08\xA0V[\x90\x95P\x93PP` \x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\tBW`\0\x80\xFD[a\tN\x87\x82\x88\x01a\x08\xA0V[\x95\x98\x94\x97P\x95PPPPV[`\0` \x82\x84\x03\x12\x15a\tlW`\0\x80\xFD[P5\x91\x90PV[` \x81R`\0\x82Q\x80` \x84\x01R`\0[\x81\x81\x10\x15a\t\xA1W` \x81\x86\x01\x81\x01Q`@\x86\x84\x01\x01R\x01a\t\x84V[P`\0`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[`\0\x80` \x83\x85\x03\x12\x15a\t\xD4W`\0\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\t\xEBW`\0\x80\xFD[a\t\xF7\x85\x82\x86\x01a\x08\xA0V[\x90\x96\x90\x95P\x93PPPPV[`\0` \x82\x84\x03\x12\x15a\n\x15W`\0\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n,W`\0\x80\xFD[\x93\x92PPPV[` \x80\x82R`-\x90\x82\x01R\x7FAstriaWithdrawer: insufficient w`@\x82\x01Rlithdrawal fee`\x98\x1B``\x82\x01R`\x80\x01\x90V[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x81\x81\x03\x81\x81\x11\x15a\n\xA9Wa\n\xA9a\n\x80V[\x92\x91PPV[`\0\x82a\n\xCCWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`b\x90\x82\x01R\x7FAstriaWithdrawer: insufficient v`@\x82\x01R\x7Falue, must be greater than 10 **``\x82\x01R\x7F (18 - BASE_CHAIN_ASSET_PRECISIO`\x80\x82\x01RaN)`\xF0\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x80\x82\x01\x80\x82\x11\x15a\n\xA9Wa\n\xA9a\n\x80V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x0B\xAF`@\x83\x01\x86\x88a\x0BrV[\x82\x81\x03` \x84\x01Ra\x0B\xC2\x81\x85\x87a\x0BrV[\x97\x96PPPPPPPV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x0B\xE1W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x0C\x01WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x81R`\0a\x0C\x1B` \x83\x01\x84\x86a\x0BrV[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 \xEA\xC3\x07<\x1Ba\xF0Vx\x9F\xC4\xFAbU+.\xC8.\xDF\xC9\xF0\x03bk\x8C\xAEX\xE1\xD3\xF6\xE5?dsolcC\0\x08\x1A\x003";
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
        ///Calls the contract's `ACCUMULATED_FEES` (0xad228247) function
        pub fn accumulated_fees(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([173, 34, 130, 71], ())
                .expect("method not found (this should never happen)")
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
        ///Calls the contract's `FEE_RECIPIENT` (0xebd09054) function
        pub fn fee_recipient(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([235, 208, 144, 84], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `IBC_WITHDRAWAL_FEE` (0xfc88d31b) function
        pub fn ibc_withdrawal_fee(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([252, 136, 211, 27], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `SEQUENCER_WITHDRAWAL_FEE` (0x6f46384a) function
        pub fn sequencer_withdrawal_fee(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([111, 70, 56, 74], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `claimFees` (0xd294f093) function
        pub fn claim_fees(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([210, 148, 240, 147], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `owner` (0x8da5cb5b) function
        pub fn owner(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([141, 165, 203, 91], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `renounceOwnership` (0x715018a6) function
        pub fn renounce_ownership(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([113, 80, 24, 166], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setFeeRecipient` (0xe74b981b) function
        pub fn set_fee_recipient(
            &self,
            new_fee_recipient: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([231, 75, 152, 27], new_fee_recipient)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setIbcWithdrawalFee` (0x88973979) function
        pub fn set_ibc_withdrawal_fee(
            &self,
            new_fee: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([136, 151, 57, 121], new_fee)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `setSequencerWithdrawalFee` (0xa7eaa739) function
        pub fn set_sequencer_withdrawal_fee(
            &self,
            new_fee: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([167, 234, 167, 57], new_fee)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `transferOwnership` (0xf2fde38b) function
        pub fn transfer_ownership(
            &self,
            new_owner: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([242, 253, 227, 139], new_owner)
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
        ///Gets the contract's `OwnershipTransferred` event
        pub fn ownership_transferred_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            OwnershipTransferredFilter,
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
    ///Custom Error type `OwnableInvalidOwner` with signature `OwnableInvalidOwner(address)` and selector `0x1e4fbdf7`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(name = "OwnableInvalidOwner", abi = "OwnableInvalidOwner(address)")]
    pub struct OwnableInvalidOwner {
        pub owner: ::ethers::core::types::Address,
    }
    ///Custom Error type `OwnableUnauthorizedAccount` with signature `OwnableUnauthorizedAccount(address)` and selector `0x118cdaa7`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash
    )]
    #[etherror(
        name = "OwnableUnauthorizedAccount",
        abi = "OwnableUnauthorizedAccount(address)"
    )]
    pub struct OwnableUnauthorizedAccount {
        pub account: ::ethers::core::types::Address,
    }
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaWithdrawerErrors {
        OwnableInvalidOwner(OwnableInvalidOwner),
        OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaWithdrawerErrors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) = <OwnableInvalidOwner as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OwnableInvalidOwner(decoded));
            }
            if let Ok(decoded) = <OwnableUnauthorizedAccount as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::OwnableUnauthorizedAccount(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for AstriaWithdrawerErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::OwnableInvalidOwner(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OwnableUnauthorizedAccount(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for AstriaWithdrawerErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <OwnableInvalidOwner as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <OwnableUnauthorizedAccount as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for AstriaWithdrawerErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::OwnableInvalidOwner(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::OwnableUnauthorizedAccount(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for AstriaWithdrawerErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<OwnableInvalidOwner> for AstriaWithdrawerErrors {
        fn from(value: OwnableInvalidOwner) -> Self {
            Self::OwnableInvalidOwner(value)
        }
    }
    impl ::core::convert::From<OwnableUnauthorizedAccount> for AstriaWithdrawerErrors {
        fn from(value: OwnableUnauthorizedAccount) -> Self {
            Self::OwnableUnauthorizedAccount(value)
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
        name = "OwnershipTransferred",
        abi = "OwnershipTransferred(address,address)"
    )]
    pub struct OwnershipTransferredFilter {
        #[ethevent(indexed)]
        pub previous_owner: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub new_owner: ::ethers::core::types::Address,
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
        OwnershipTransferredFilter(OwnershipTransferredFilter),
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
            if let Ok(decoded) = OwnershipTransferredFilter::decode_log(log) {
                return Ok(AstriaWithdrawerEvents::OwnershipTransferredFilter(decoded));
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
                Self::OwnershipTransferredFilter(element) => {
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
    impl ::core::convert::From<OwnershipTransferredFilter> for AstriaWithdrawerEvents {
        fn from(value: OwnershipTransferredFilter) -> Self {
            Self::OwnershipTransferredFilter(value)
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
    ///Container type for all input parameters for the `ACCUMULATED_FEES` function with signature `ACCUMULATED_FEES()` and selector `0xad228247`
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
    #[ethcall(name = "ACCUMULATED_FEES", abi = "ACCUMULATED_FEES()")]
    pub struct AccumulatedFeesCall;
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
    ///Container type for all input parameters for the `FEE_RECIPIENT` function with signature `FEE_RECIPIENT()` and selector `0xebd09054`
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
    #[ethcall(name = "FEE_RECIPIENT", abi = "FEE_RECIPIENT()")]
    pub struct FeeRecipientCall;
    ///Container type for all input parameters for the `IBC_WITHDRAWAL_FEE` function with signature `IBC_WITHDRAWAL_FEE()` and selector `0xfc88d31b`
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
    #[ethcall(name = "IBC_WITHDRAWAL_FEE", abi = "IBC_WITHDRAWAL_FEE()")]
    pub struct IbcWithdrawalFeeCall;
    ///Container type for all input parameters for the `SEQUENCER_WITHDRAWAL_FEE` function with signature `SEQUENCER_WITHDRAWAL_FEE()` and selector `0x6f46384a`
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
    #[ethcall(name = "SEQUENCER_WITHDRAWAL_FEE", abi = "SEQUENCER_WITHDRAWAL_FEE()")]
    pub struct SequencerWithdrawalFeeCall;
    ///Container type for all input parameters for the `claimFees` function with signature `claimFees()` and selector `0xd294f093`
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
    #[ethcall(name = "claimFees", abi = "claimFees()")]
    pub struct ClaimFeesCall;
    ///Container type for all input parameters for the `owner` function with signature `owner()` and selector `0x8da5cb5b`
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
    #[ethcall(name = "owner", abi = "owner()")]
    pub struct OwnerCall;
    ///Container type for all input parameters for the `renounceOwnership` function with signature `renounceOwnership()` and selector `0x715018a6`
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
    #[ethcall(name = "renounceOwnership", abi = "renounceOwnership()")]
    pub struct RenounceOwnershipCall;
    ///Container type for all input parameters for the `setFeeRecipient` function with signature `setFeeRecipient(address)` and selector `0xe74b981b`
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
    #[ethcall(name = "setFeeRecipient", abi = "setFeeRecipient(address)")]
    pub struct SetFeeRecipientCall {
        pub new_fee_recipient: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `setIbcWithdrawalFee` function with signature `setIbcWithdrawalFee(uint256)` and selector `0x88973979`
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
    #[ethcall(name = "setIbcWithdrawalFee", abi = "setIbcWithdrawalFee(uint256)")]
    pub struct SetIbcWithdrawalFeeCall {
        pub new_fee: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `setSequencerWithdrawalFee` function with signature `setSequencerWithdrawalFee(uint256)` and selector `0xa7eaa739`
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
        name = "setSequencerWithdrawalFee",
        abi = "setSequencerWithdrawalFee(uint256)"
    )]
    pub struct SetSequencerWithdrawalFeeCall {
        pub new_fee: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `transferOwnership` function with signature `transferOwnership(address)` and selector `0xf2fde38b`
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
    #[ethcall(name = "transferOwnership", abi = "transferOwnership(address)")]
    pub struct TransferOwnershipCall {
        pub new_owner: ::ethers::core::types::Address,
    }
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
        AccumulatedFees(AccumulatedFeesCall),
        BaseChainAssetDenomination(BaseChainAssetDenominationCall),
        BaseChainAssetPrecision(BaseChainAssetPrecisionCall),
        BaseChainBridgeAddress(BaseChainBridgeAddressCall),
        FeeRecipient(FeeRecipientCall),
        IbcWithdrawalFee(IbcWithdrawalFeeCall),
        SequencerWithdrawalFee(SequencerWithdrawalFeeCall),
        ClaimFees(ClaimFeesCall),
        Owner(OwnerCall),
        RenounceOwnership(RenounceOwnershipCall),
        SetFeeRecipient(SetFeeRecipientCall),
        SetIbcWithdrawalFee(SetIbcWithdrawalFeeCall),
        SetSequencerWithdrawalFee(SetSequencerWithdrawalFeeCall),
        TransferOwnership(TransferOwnershipCall),
        WithdrawToIbcChain(WithdrawToIbcChainCall),
        WithdrawToRollup(WithdrawToRollupCall),
        WithdrawToSequencer(WithdrawToSequencerCall),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaWithdrawerCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <AccumulatedFeesCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::AccumulatedFees(decoded));
            }
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
            if let Ok(decoded) = <FeeRecipientCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::FeeRecipient(decoded));
            }
            if let Ok(decoded) = <IbcWithdrawalFeeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::IbcWithdrawalFee(decoded));
            }
            if let Ok(decoded) = <SequencerWithdrawalFeeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SequencerWithdrawalFee(decoded));
            }
            if let Ok(decoded) = <ClaimFeesCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ClaimFees(decoded));
            }
            if let Ok(decoded) = <OwnerCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Owner(decoded));
            }
            if let Ok(decoded) = <RenounceOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RenounceOwnership(decoded));
            }
            if let Ok(decoded) = <SetFeeRecipientCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetFeeRecipient(decoded));
            }
            if let Ok(decoded) = <SetIbcWithdrawalFeeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetIbcWithdrawalFee(decoded));
            }
            if let Ok(decoded) = <SetSequencerWithdrawalFeeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::SetSequencerWithdrawalFee(decoded));
            }
            if let Ok(decoded) = <TransferOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::TransferOwnership(decoded));
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
                Self::AccumulatedFees(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::BaseChainAssetDenomination(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::BaseChainAssetPrecision(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::BaseChainBridgeAddress(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::FeeRecipient(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::IbcWithdrawalFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SequencerWithdrawalFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ClaimFees(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Owner(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::RenounceOwnership(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetFeeRecipient(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetIbcWithdrawalFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SetSequencerWithdrawalFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::TransferOwnership(element) => {
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
                Self::AccumulatedFees(element) => ::core::fmt::Display::fmt(element, f),
                Self::BaseChainAssetDenomination(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::BaseChainAssetPrecision(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::BaseChainBridgeAddress(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::FeeRecipient(element) => ::core::fmt::Display::fmt(element, f),
                Self::IbcWithdrawalFee(element) => ::core::fmt::Display::fmt(element, f),
                Self::SequencerWithdrawalFee(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ClaimFees(element) => ::core::fmt::Display::fmt(element, f),
                Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
                Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetFeeRecipient(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetIbcWithdrawalFee(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::SetSequencerWithdrawalFee(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::TransferOwnership(element) => ::core::fmt::Display::fmt(element, f),
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
    impl ::core::convert::From<AccumulatedFeesCall> for AstriaWithdrawerCalls {
        fn from(value: AccumulatedFeesCall) -> Self {
            Self::AccumulatedFees(value)
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
    impl ::core::convert::From<FeeRecipientCall> for AstriaWithdrawerCalls {
        fn from(value: FeeRecipientCall) -> Self {
            Self::FeeRecipient(value)
        }
    }
    impl ::core::convert::From<IbcWithdrawalFeeCall> for AstriaWithdrawerCalls {
        fn from(value: IbcWithdrawalFeeCall) -> Self {
            Self::IbcWithdrawalFee(value)
        }
    }
    impl ::core::convert::From<SequencerWithdrawalFeeCall> for AstriaWithdrawerCalls {
        fn from(value: SequencerWithdrawalFeeCall) -> Self {
            Self::SequencerWithdrawalFee(value)
        }
    }
    impl ::core::convert::From<ClaimFeesCall> for AstriaWithdrawerCalls {
        fn from(value: ClaimFeesCall) -> Self {
            Self::ClaimFees(value)
        }
    }
    impl ::core::convert::From<OwnerCall> for AstriaWithdrawerCalls {
        fn from(value: OwnerCall) -> Self {
            Self::Owner(value)
        }
    }
    impl ::core::convert::From<RenounceOwnershipCall> for AstriaWithdrawerCalls {
        fn from(value: RenounceOwnershipCall) -> Self {
            Self::RenounceOwnership(value)
        }
    }
    impl ::core::convert::From<SetFeeRecipientCall> for AstriaWithdrawerCalls {
        fn from(value: SetFeeRecipientCall) -> Self {
            Self::SetFeeRecipient(value)
        }
    }
    impl ::core::convert::From<SetIbcWithdrawalFeeCall> for AstriaWithdrawerCalls {
        fn from(value: SetIbcWithdrawalFeeCall) -> Self {
            Self::SetIbcWithdrawalFee(value)
        }
    }
    impl ::core::convert::From<SetSequencerWithdrawalFeeCall> for AstriaWithdrawerCalls {
        fn from(value: SetSequencerWithdrawalFeeCall) -> Self {
            Self::SetSequencerWithdrawalFee(value)
        }
    }
    impl ::core::convert::From<TransferOwnershipCall> for AstriaWithdrawerCalls {
        fn from(value: TransferOwnershipCall) -> Self {
            Self::TransferOwnership(value)
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
    ///Container type for all return fields from the `ACCUMULATED_FEES` function with signature `ACCUMULATED_FEES()` and selector `0xad228247`
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
    pub struct AccumulatedFeesReturn(pub ::ethers::core::types::U256);
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
    ///Container type for all return fields from the `FEE_RECIPIENT` function with signature `FEE_RECIPIENT()` and selector `0xebd09054`
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
    pub struct FeeRecipientReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `IBC_WITHDRAWAL_FEE` function with signature `IBC_WITHDRAWAL_FEE()` and selector `0xfc88d31b`
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
    pub struct IbcWithdrawalFeeReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `SEQUENCER_WITHDRAWAL_FEE` function with signature `SEQUENCER_WITHDRAWAL_FEE()` and selector `0x6f46384a`
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
    pub struct SequencerWithdrawalFeeReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `owner` function with signature `owner()` and selector `0x8da5cb5b`
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
    pub struct OwnerReturn(pub ::ethers::core::types::Address);
}
