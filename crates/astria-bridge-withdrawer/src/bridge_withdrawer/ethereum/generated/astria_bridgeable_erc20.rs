pub use astria_bridgeable_erc20::*;
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
pub mod astria_bridgeable_erc20 {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_bridge"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
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
                        name: ::std::borrow::ToOwned::to_owned("_name"),
                        kind: ::ethers::core::abi::ethabi::ParamType::String,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("string"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_symbol"),
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
                    ::std::borrow::ToOwned::to_owned("BRIDGE"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("BRIDGE"),
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
                    ::std::borrow::ToOwned::to_owned("allowance"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("allowance"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("owner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("spender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
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
                    ::std::borrow::ToOwned::to_owned("approve"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("approve"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("spender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("value"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bool"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("balanceOf"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("balanceOf"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("account"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                            ],
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
                    ::std::borrow::ToOwned::to_owned("decimals"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("decimals"),
                            inputs: ::std::vec![],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint8"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("mint"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("mint"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_amount"),
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
                    ::std::borrow::ToOwned::to_owned("name"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("name"),
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
                    ::std::borrow::ToOwned::to_owned("symbol"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("symbol"),
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
                    ::std::borrow::ToOwned::to_owned("totalSupply"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("totalSupply"),
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
                    ::std::borrow::ToOwned::to_owned("transfer"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("transfer"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("value"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bool"),
                                    ),
                                },
                            ],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("transferFrom"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned("transferFrom"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("value"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::string::String::new(),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("bool"),
                                    ),
                                },
                            ],
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
                                    name: ::std::borrow::ToOwned::to_owned("_amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "_destinationChainAddress",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_memo"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
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
                    ::std::borrow::ToOwned::to_owned("withdrawToSequencer"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Function {
                            name: ::std::borrow::ToOwned::to_owned(
                                "withdrawToSequencer",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("_amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "_destinationChainAddress",
                                    ),
                                    kind: ::ethers::core::abi::ethabi::ParamType::String,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("string"),
                                    ),
                                },
                            ],
                            outputs: ::std::vec![],
                            constant: ::core::option::Option::None,
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                        },
                    ],
                ),
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("Approval"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("Approval"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("owner"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("spender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("value"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    indexed: false,
                                },
                            ],
                            anonymous: false,
                        },
                    ],
                ),
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
                    ::std::borrow::ToOwned::to_owned("Mint"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("Mint"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("account"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("amount"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
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
                (
                    ::std::borrow::ToOwned::to_owned("Transfer"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::Event {
                            name: ::std::borrow::ToOwned::to_owned("Transfer"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("from"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("to"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    indexed: true,
                                },
                                ::ethers::core::abi::ethabi::EventParam {
                                    name: ::std::borrow::ToOwned::to_owned("value"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
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
                    ::std::borrow::ToOwned::to_owned("ERC20InsufficientAllowance"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ERC20InsufficientAllowance",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("spender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("allowance"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("needed"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InsufficientBalance"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ERC20InsufficientBalance",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("sender"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("address"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("balance"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("needed"),
                                    kind: ::ethers::core::abi::ethabi::ParamType::Uint(
                                        256usize,
                                    ),
                                    internal_type: ::core::option::Option::Some(
                                        ::std::borrow::ToOwned::to_owned("uint256"),
                                    ),
                                },
                            ],
                        },
                    ],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidApprover"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ERC20InvalidApprover",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("approver"),
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
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidReceiver"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ERC20InvalidReceiver",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("receiver"),
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
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidSender"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned("ERC20InvalidSender"),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("sender"),
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
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidSpender"),
                    ::std::vec![
                        ::ethers::core::abi::ethabi::AbiError {
                            name: ::std::borrow::ToOwned::to_owned(
                                "ERC20InvalidSpender",
                            ),
                            inputs: ::std::vec![
                                ::ethers::core::abi::ethabi::Param {
                                    name: ::std::borrow::ToOwned::to_owned("spender"),
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
    pub static ASTRIABRIDGEABLEERC20_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\xE0`@R4\x80\x15b\0\0\x11W`\0\x80\xFD[P`@Qb\0\x13\xF68\x03\x80b\0\x13\xF6\x839\x81\x01`@\x81\x90Rb\0\x004\x91b\0\x02rV[\x81\x81`\x05b\0\0D\x83\x82b\0\x03\xDFV[P`\x06b\0\0S\x82\x82b\0\x03\xDFV[PPP`\0b\0\0hb\0\x01v` \x1B` \x1CV[\x90P\x80`\xFF\x16\x86c\xFF\xFF\xFF\xFF\x16\x11\x15b\0\x01\x14W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`^`$\x82\x01R\x7FAstriaBridgeableERC20: base chai`D\x82\x01R\x7Fn asset precision must be less t`d\x82\x01R\x7Fhan or equal to token decimals\0\0`\x84\x82\x01R`\xA4\x01`@Q\x80\x91\x03\x90\xFD[c\xFF\xFF\xFF\xFF\x86\x16`\x80R`\0b\0\x01,\x86\x82b\0\x03\xDFV[P`\x01b\0\x01;\x85\x82b\0\x03\xDFV[Pb\0\x01K\x86`\xFF\x83\x16b\0\x04\xC1V[b\0\x01X\x90`\nb\0\x05\xE7V[`\xC0RPPP`\x01`\x01`\xA0\x1B\x03\x90\x93\x16`\xA0RPb\0\x06\x02\x91PPV[`\x12\x90V[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14b\0\x01\x93W`\0\x80\xFD[\x91\x90PV[\x80Qc\xFF\xFF\xFF\xFF\x81\x16\x81\x14b\0\x01\x93W`\0\x80\xFD[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0\x82`\x1F\x83\x01\x12b\0\x01\xD5W`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x01\xF2Wb\0\x01\xF2b\0\x01\xADV[`@Q`\x1F\x83\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01\x90\x82\x82\x11\x81\x83\x10\x17\x15b\0\x02\x1DWb\0\x02\x1Db\0\x01\xADV[\x81`@R\x83\x81R` \x92P\x86\x83\x85\x88\x01\x01\x11\x15b\0\x02:W`\0\x80\xFD[`\0\x91P[\x83\x82\x10\x15b\0\x02^W\x85\x82\x01\x83\x01Q\x81\x83\x01\x84\x01R\x90\x82\x01\x90b\0\x02?V[`\0\x93\x81\x01\x90\x92\x01\x92\x90\x92R\x94\x93PPPPV[`\0\x80`\0\x80`\0\x80`\xC0\x87\x89\x03\x12\x15b\0\x02\x8CW`\0\x80\xFD[b\0\x02\x97\x87b\0\x01{V[\x95Pb\0\x02\xA7` \x88\x01b\0\x01\x98V[`@\x88\x01Q\x90\x95P`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x02\xC5W`\0\x80\xFD[b\0\x02\xD3\x8A\x83\x8B\x01b\0\x01\xC3V[\x95P``\x89\x01Q\x91P\x80\x82\x11\x15b\0\x02\xEAW`\0\x80\xFD[b\0\x02\xF8\x8A\x83\x8B\x01b\0\x01\xC3V[\x94P`\x80\x89\x01Q\x91P\x80\x82\x11\x15b\0\x03\x0FW`\0\x80\xFD[b\0\x03\x1D\x8A\x83\x8B\x01b\0\x01\xC3V[\x93P`\xA0\x89\x01Q\x91P\x80\x82\x11\x15b\0\x034W`\0\x80\xFD[Pb\0\x03C\x89\x82\x8A\x01b\0\x01\xC3V[\x91PP\x92\x95P\x92\x95P\x92\x95V[`\x01\x81\x81\x1C\x90\x82\x16\x80b\0\x03eW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03b\0\x03\x86WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15b\0\x03\xDAW`\0\x81\x81R` \x81 `\x1F\x85\x01`\x05\x1C\x81\x01` \x86\x10\x15b\0\x03\xB5WP\x80[`\x1F\x85\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15b\0\x03\xD6W\x82\x81U`\x01\x01b\0\x03\xC1V[PPP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15b\0\x03\xFBWb\0\x03\xFBb\0\x01\xADV[b\0\x04\x13\x81b\0\x04\x0C\x84Tb\0\x03PV[\x84b\0\x03\x8CV[` \x80`\x1F\x83\x11`\x01\x81\x14b\0\x04KW`\0\x84\x15b\0\x042WP\x85\x83\x01Q[`\0\x19`\x03\x86\x90\x1B\x1C\x19\x16`\x01\x85\x90\x1B\x17\x85Ub\0\x03\xD6V[`\0\x85\x81R` \x81 `\x1F\x19\x86\x16\x91[\x82\x81\x10\x15b\0\x04|W\x88\x86\x01Q\x82U\x94\x84\x01\x94`\x01\x90\x91\x01\x90\x84\x01b\0\x04[V[P\x85\x82\x10\x15b\0\x04\x9BW\x87\x85\x01Q`\0\x19`\x03\x88\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPPP`\x01\x90\x81\x1B\x01\x90UPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[c\xFF\xFF\xFF\xFF\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15b\0\x04\xE1Wb\0\x04\xE1b\0\x04\xABV[P\x92\x91PPV[`\x01\x81\x81[\x80\x85\x11\x15b\0\x05)W\x81`\0\x19\x04\x82\x11\x15b\0\x05\rWb\0\x05\rb\0\x04\xABV[\x80\x85\x16\x15b\0\x05\x1BW\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90b\0\x04\xEDV[P\x92P\x92\x90PV[`\0\x82b\0\x05BWP`\x01b\0\x05\xE1V[\x81b\0\x05QWP`\0b\0\x05\xE1V[\x81`\x01\x81\x14b\0\x05jW`\x02\x81\x14b\0\x05uWb\0\x05\x95V[`\x01\x91PPb\0\x05\xE1V[`\xFF\x84\x11\x15b\0\x05\x89Wb\0\x05\x89b\0\x04\xABV[PP`\x01\x82\x1Bb\0\x05\xE1V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15b\0\x05\xBAWP\x81\x81\nb\0\x05\xE1V[b\0\x05\xC6\x83\x83b\0\x04\xE8V[\x80`\0\x19\x04\x82\x11\x15b\0\x05\xDDWb\0\x05\xDDb\0\x04\xABV[\x02\x90P[\x92\x91PPV[`\0b\0\x05\xFBc\xFF\xFF\xFF\xFF\x84\x16\x83b\0\x051V[\x93\x92PPPV[`\x80Q`\xA0Q`\xC0Qa\r\xB6b\0\x06@`\09`\0\x81\x81a\x04w\x01Ra\x05\xC6\x01R`\0\x81\x81a\x02\x83\x01Ra\x03\x98\x01R`\0a\x01\xD0\x01Ra\r\xB6`\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\x01\0W`\x005`\xE0\x1C\x80c~\xB6\xDE\xC7\x11a\0\x97W\x80c\xD3\x8F\xE9\xA7\x11a\0fW\x80c\xD3\x8F\xE9\xA7\x14a\x02*W\x80c\xDB\x97\xDC\x98\x14a\x02=W\x80c\xDDb\xED>\x14a\x02EW\x80c\xEE\x9A1\xA2\x14a\x02~W`\0\x80\xFD[\x80c~\xB6\xDE\xC7\x14a\x01\xCBW\x80c\x95\xD8\x9BA\x14a\x02\x07W\x80c\xA9\x05\x9C\xBB\x14a\x02\x0FW\x80c\xB6Gl~\x14a\x02\"W`\0\x80\xFD[\x80c1<\xE5g\x11a\0\xD3W\x80c1<\xE5g\x14a\x01kW\x80c@\xC1\x0F\x19\x14a\x01zW\x80c_\xE5k\t\x14a\x01\x8FW\x80cp\xA0\x821\x14a\x01\xA2W`\0\x80\xFD[\x80c\x06\xFD\xDE\x03\x14a\x01\x05W\x80c\t^\xA7\xB3\x14a\x01#W\x80c\x18\x16\r\xDD\x14a\x01FW\x80c#\xB8r\xDD\x14a\x01XW[`\0\x80\xFD[a\x01\ra\x02\xBDV[`@Qa\x01\x1A\x91\x90a\t\xB9V[`@Q\x80\x91\x03\x90\xF3[a\x016a\x0116`\x04a\n#V[a\x03OV[`@Q\x90\x15\x15\x81R` \x01a\x01\x1AV[`\x04T[`@Q\x90\x81R` \x01a\x01\x1AV[a\x016a\x01f6`\x04a\nMV[a\x03iV[`@Q`\x12\x81R` \x01a\x01\x1AV[a\x01\x8Da\x01\x886`\x04a\n#V[a\x03\x8DV[\0[a\x01\x8Da\x01\x9D6`\x04a\n\xD2V[a\x04oV[a\x01Ja\x01\xB06`\x04a\x0BLV[`\x01`\x01`\xA0\x1B\x03\x16`\0\x90\x81R`\x02` R`@\x90 T\x90V[a\x01\xF2\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\x1AV[a\x01\ra\x05\x13V[a\x016a\x02\x1D6`\x04a\n#V[a\x05\"V[a\x01\ra\x050V[a\x01\x8Da\x0286`\x04a\x0BnV[a\x05\xBEV[a\x01\ra\x06\\V[a\x01Ja\x02S6`\x04a\x0B\xBAV[`\x01`\x01`\xA0\x1B\x03\x91\x82\x16`\0\x90\x81R`\x03` \x90\x81R`@\x80\x83 \x93\x90\x94\x16\x82R\x91\x90\x91R T\x90V[a\x02\xA5\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x1AV[```\x05\x80Ta\x02\xCC\x90a\x0B\xEDV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\xF8\x90a\x0B\xEDV[\x80\x15a\x03EW\x80`\x1F\x10a\x03\x1AWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x03EV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x03(W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x90P\x90V[`\x003a\x03]\x81\x85\x85a\x06iV[`\x01\x91PP[\x92\x91PPV[`\x003a\x03w\x85\x82\x85a\x06{V[a\x03\x82\x85\x85\x85a\x06\xF9V[P`\x01\x94\x93PPPPV[3`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x04\x1EW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FAstriaBridgeableERC20: only brid`D\x82\x01Rj\x19\xD9H\x18\xD8[\x88\x1BZ[\x9D`\xAA\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[a\x04(\x82\x82a\x07XV[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Fg\x98\xA5`y:T\xC3\xBC\xFE\x86\xA9<\xDE\x1Es\x08}\x94L\x0E\xA2\x05D\x13}A!9h\x85\x82`@Qa\x04c\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPV[\x84`\0a\x04\x9C\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x0C'V[\x11a\x04\xB9W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x04\x15\x90a\x0CIV[a\x04\xC33\x87a\x07\x92V[\x853`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x05\x03\x94\x93\x92\x91\x90a\r\x11V[`@Q\x80\x91\x03\x90\xA3PPPPPPV[```\x06\x80Ta\x02\xCC\x90a\x0B\xEDV[`\x003a\x03]\x81\x85\x85a\x06\xF9V[`\x01\x80Ta\x05=\x90a\x0B\xEDV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05i\x90a\x0B\xEDV[\x80\x15a\x05\xB6W\x80`\x1F\x10a\x05\x8BWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x05\xB6V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x05\x99W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[\x82`\0a\x05\xEB\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x0C'V[\x11a\x06\x08W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x04\x15\x90a\x0CIV[a\x06\x123\x85a\x07\x92V[\x833`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x85\x85`@Qa\x06N\x92\x91\x90a\rCV[`@Q\x80\x91\x03\x90\xA3PPPPV[`\0\x80Ta\x05=\x90a\x0B\xEDV[a\x06v\x83\x83\x83`\x01a\x07\xC8V[PPPV[`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\0\x90\x81R`\x03` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R T`\0\x19\x81\x14a\x06\xF3W\x81\x81\x10\x15a\x06\xE4W`@Qc}\xC7\xA0\xD9`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x04\x15V[a\x06\xF3\x84\x84\x84\x84\x03`\0a\x07\xC8V[PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x07#W`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07MW`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[a\x06v\x83\x83\x83a\x08\x8FV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07\x82W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[a\x07\x8E`\0\x83\x83a\x08\x8FV[PPV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07\xBCW`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[a\x07\x8E\x82`\0\x83a\x08\x8FV[`\x01`\x01`\xA0\x1B\x03\x84\x16a\x07\xF2W`@Qc\xE6\x02\xDF\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x08\x1CW`@QcJ\x14\x06\xB1`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16`\0\x90\x81R`\x03` \x90\x81R`@\x80\x83 \x93\x87\x16\x83R\x92\x90R \x82\x90U\x80\x15a\x06\xF3W\x82`\x01`\x01`\xA0\x1B\x03\x16\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\x8C[\xE1\xE5\xEB\xEC}[\xD1OqB}\x1E\x84\xF3\xDD\x03\x14\xC0\xF7\xB2)\x1E[ \n\xC8\xC7\xC3\xB9%\x84`@Qa\x06N\x91\x81R` \x01\x90V[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x08\xBAW\x80`\x04`\0\x82\x82Ta\x08\xAF\x91\x90a\r_V[\x90\x91UPa\t,\x90PV[`\x01`\x01`\xA0\x1B\x03\x83\x16`\0\x90\x81R`\x02` R`@\x90 T\x81\x81\x10\x15a\t\rW`@Qc9\x144\xE3`\xE2\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x84\x16`\0\x90\x81R`\x02` R`@\x90 \x90\x82\x90\x03\x90U[`\x01`\x01`\xA0\x1B\x03\x82\x16a\tHW`\x04\x80T\x82\x90\x03\x90Ua\tgV[`\x01`\x01`\xA0\x1B\x03\x82\x16`\0\x90\x81R`\x02` R`@\x90 \x80T\x82\x01\x90U[\x81`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDD\xF2R\xAD\x1B\xE2\xC8\x9Bi\xC2\xB0h\xFC7\x8D\xAA\x95+\xA7\xF1c\xC4\xA1\x16(\xF5ZM\xF5#\xB3\xEF\x83`@Qa\t\xAC\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPV[`\0` \x80\x83R\x83Q\x80\x82\x85\x01R`\0[\x81\x81\x10\x15a\t\xE6W\x85\x81\x01\x83\x01Q\x85\x82\x01`@\x01R\x82\x01a\t\xCAV[P`\0`@\x82\x86\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x92PPP\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n\x1EW`\0\x80\xFD[\x91\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\n6W`\0\x80\xFD[a\n?\x83a\n\x07V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\nbW`\0\x80\xFD[a\nk\x84a\n\x07V[\x92Pa\ny` \x85\x01a\n\x07V[\x91P`@\x84\x015\x90P\x92P\x92P\x92V[`\0\x80\x83`\x1F\x84\x01\x12a\n\x9BW`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\n\xB3W`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\n\xCBW`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`\0``\x86\x88\x03\x12\x15a\n\xEAW`\0\x80\xFD[\x855\x94P` \x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x0B\tW`\0\x80\xFD[a\x0B\x15\x89\x83\x8A\x01a\n\x89V[\x90\x96P\x94P`@\x88\x015\x91P\x80\x82\x11\x15a\x0B.W`\0\x80\xFD[Pa\x0B;\x88\x82\x89\x01a\n\x89V[\x96\x99\x95\x98P\x93\x96P\x92\x94\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\x0B^W`\0\x80\xFD[a\x0Bg\x82a\n\x07V[\x93\x92PPPV[`\0\x80`\0`@\x84\x86\x03\x12\x15a\x0B\x83W`\0\x80\xFD[\x835\x92P` \x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x0B\xA1W`\0\x80\xFD[a\x0B\xAD\x86\x82\x87\x01a\n\x89V[\x94\x97\x90\x96P\x93\x94PPPPV[`\0\x80`@\x83\x85\x03\x12\x15a\x0B\xCDW`\0\x80\xFD[a\x0B\xD6\x83a\n\x07V[\x91Pa\x0B\xE4` \x84\x01a\n\x07V[\x90P\x92P\x92\x90PV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x0C\x01W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x0C!WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\0\x82a\x0CDWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`s\x90\x82\x01R\x7FAstriaBridgeableERC20: insuffici`@\x82\x01R\x7Fent value, must be greater than ``\x82\x01R\x7F10 ** (TOKEN_DECIMALS - BASE_CHA`\x80\x82\x01RrIN_ASSET_PRECISION)`h\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\r%`@\x83\x01\x86\x88a\x0C\xE8V[\x82\x81\x03` \x84\x01Ra\r8\x81\x85\x87a\x0C\xE8V[\x97\x96PPPPPPPV[` \x81R`\0a\rW` \x83\x01\x84\x86a\x0C\xE8V[\x94\x93PPPPV[\x80\x82\x01\x80\x82\x11\x15a\x03cWcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD\xFE\xA2dipfsX\"\x12 z\x0Bo\xC7\n\x8B$\t|\xDE4\x12\xC1\xAE\xBD[\x92[J\x8B\xAE\xDA\x81#An8^D\x1F\x80\x94dsolcC\0\x08\x15\x003";
    /// The bytecode of the contract.
    pub static ASTRIABRIDGEABLEERC20_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\x01\0W`\x005`\xE0\x1C\x80c~\xB6\xDE\xC7\x11a\0\x97W\x80c\xD3\x8F\xE9\xA7\x11a\0fW\x80c\xD3\x8F\xE9\xA7\x14a\x02*W\x80c\xDB\x97\xDC\x98\x14a\x02=W\x80c\xDDb\xED>\x14a\x02EW\x80c\xEE\x9A1\xA2\x14a\x02~W`\0\x80\xFD[\x80c~\xB6\xDE\xC7\x14a\x01\xCBW\x80c\x95\xD8\x9BA\x14a\x02\x07W\x80c\xA9\x05\x9C\xBB\x14a\x02\x0FW\x80c\xB6Gl~\x14a\x02\"W`\0\x80\xFD[\x80c1<\xE5g\x11a\0\xD3W\x80c1<\xE5g\x14a\x01kW\x80c@\xC1\x0F\x19\x14a\x01zW\x80c_\xE5k\t\x14a\x01\x8FW\x80cp\xA0\x821\x14a\x01\xA2W`\0\x80\xFD[\x80c\x06\xFD\xDE\x03\x14a\x01\x05W\x80c\t^\xA7\xB3\x14a\x01#W\x80c\x18\x16\r\xDD\x14a\x01FW\x80c#\xB8r\xDD\x14a\x01XW[`\0\x80\xFD[a\x01\ra\x02\xBDV[`@Qa\x01\x1A\x91\x90a\t\xB9V[`@Q\x80\x91\x03\x90\xF3[a\x016a\x0116`\x04a\n#V[a\x03OV[`@Q\x90\x15\x15\x81R` \x01a\x01\x1AV[`\x04T[`@Q\x90\x81R` \x01a\x01\x1AV[a\x016a\x01f6`\x04a\nMV[a\x03iV[`@Q`\x12\x81R` \x01a\x01\x1AV[a\x01\x8Da\x01\x886`\x04a\n#V[a\x03\x8DV[\0[a\x01\x8Da\x01\x9D6`\x04a\n\xD2V[a\x04oV[a\x01Ja\x01\xB06`\x04a\x0BLV[`\x01`\x01`\xA0\x1B\x03\x16`\0\x90\x81R`\x02` R`@\x90 T\x90V[a\x01\xF2\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\x1AV[a\x01\ra\x05\x13V[a\x016a\x02\x1D6`\x04a\n#V[a\x05\"V[a\x01\ra\x050V[a\x01\x8Da\x0286`\x04a\x0BnV[a\x05\xBEV[a\x01\ra\x06\\V[a\x01Ja\x02S6`\x04a\x0B\xBAV[`\x01`\x01`\xA0\x1B\x03\x91\x82\x16`\0\x90\x81R`\x03` \x90\x81R`@\x80\x83 \x93\x90\x94\x16\x82R\x91\x90\x91R T\x90V[a\x02\xA5\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x1AV[```\x05\x80Ta\x02\xCC\x90a\x0B\xEDV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\xF8\x90a\x0B\xEDV[\x80\x15a\x03EW\x80`\x1F\x10a\x03\x1AWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x03EV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x03(W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x90P\x90V[`\x003a\x03]\x81\x85\x85a\x06iV[`\x01\x91PP[\x92\x91PPV[`\x003a\x03w\x85\x82\x85a\x06{V[a\x03\x82\x85\x85\x85a\x06\xF9V[P`\x01\x94\x93PPPPV[3`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x04\x1EW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FAstriaBridgeableERC20: only brid`D\x82\x01Rj\x19\xD9H\x18\xD8[\x88\x1BZ[\x9D`\xAA\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[a\x04(\x82\x82a\x07XV[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Fg\x98\xA5`y:T\xC3\xBC\xFE\x86\xA9<\xDE\x1Es\x08}\x94L\x0E\xA2\x05D\x13}A!9h\x85\x82`@Qa\x04c\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPV[\x84`\0a\x04\x9C\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x0C'V[\x11a\x04\xB9W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x04\x15\x90a\x0CIV[a\x04\xC33\x87a\x07\x92V[\x853`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x87\x87\x87\x87`@Qa\x05\x03\x94\x93\x92\x91\x90a\r\x11V[`@Q\x80\x91\x03\x90\xA3PPPPPPV[```\x06\x80Ta\x02\xCC\x90a\x0B\xEDV[`\x003a\x03]\x81\x85\x85a\x06\xF9V[`\x01\x80Ta\x05=\x90a\x0B\xEDV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05i\x90a\x0B\xEDV[\x80\x15a\x05\xB6W\x80`\x1F\x10a\x05\x8BWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x05\xB6V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x05\x99W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[\x82`\0a\x05\xEB\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x83a\x0C'V[\x11a\x06\x08W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x04\x15\x90a\x0CIV[a\x06\x123\x85a\x07\x92V[\x833`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x85\x85`@Qa\x06N\x92\x91\x90a\rCV[`@Q\x80\x91\x03\x90\xA3PPPPV[`\0\x80Ta\x05=\x90a\x0B\xEDV[a\x06v\x83\x83\x83`\x01a\x07\xC8V[PPPV[`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\0\x90\x81R`\x03` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R T`\0\x19\x81\x14a\x06\xF3W\x81\x81\x10\x15a\x06\xE4W`@Qc}\xC7\xA0\xD9`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x04\x15V[a\x06\xF3\x84\x84\x84\x84\x03`\0a\x07\xC8V[PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x07#W`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07MW`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[a\x06v\x83\x83\x83a\x08\x8FV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07\x82W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[a\x07\x8E`\0\x83\x83a\x08\x8FV[PPV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07\xBCW`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[a\x07\x8E\x82`\0\x83a\x08\x8FV[`\x01`\x01`\xA0\x1B\x03\x84\x16a\x07\xF2W`@Qc\xE6\x02\xDF\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x08\x1CW`@QcJ\x14\x06\xB1`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16`\0\x90\x81R`\x03` \x90\x81R`@\x80\x83 \x93\x87\x16\x83R\x92\x90R \x82\x90U\x80\x15a\x06\xF3W\x82`\x01`\x01`\xA0\x1B\x03\x16\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\x8C[\xE1\xE5\xEB\xEC}[\xD1OqB}\x1E\x84\xF3\xDD\x03\x14\xC0\xF7\xB2)\x1E[ \n\xC8\xC7\xC3\xB9%\x84`@Qa\x06N\x91\x81R` \x01\x90V[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x08\xBAW\x80`\x04`\0\x82\x82Ta\x08\xAF\x91\x90a\r_V[\x90\x91UPa\t,\x90PV[`\x01`\x01`\xA0\x1B\x03\x83\x16`\0\x90\x81R`\x02` R`@\x90 T\x81\x81\x10\x15a\t\rW`@Qc9\x144\xE3`\xE2\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x04\x15V[`\x01`\x01`\xA0\x1B\x03\x84\x16`\0\x90\x81R`\x02` R`@\x90 \x90\x82\x90\x03\x90U[`\x01`\x01`\xA0\x1B\x03\x82\x16a\tHW`\x04\x80T\x82\x90\x03\x90Ua\tgV[`\x01`\x01`\xA0\x1B\x03\x82\x16`\0\x90\x81R`\x02` R`@\x90 \x80T\x82\x01\x90U[\x81`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDD\xF2R\xAD\x1B\xE2\xC8\x9Bi\xC2\xB0h\xFC7\x8D\xAA\x95+\xA7\xF1c\xC4\xA1\x16(\xF5ZM\xF5#\xB3\xEF\x83`@Qa\t\xAC\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPV[`\0` \x80\x83R\x83Q\x80\x82\x85\x01R`\0[\x81\x81\x10\x15a\t\xE6W\x85\x81\x01\x83\x01Q\x85\x82\x01`@\x01R\x82\x01a\t\xCAV[P`\0`@\x82\x86\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x92PPP\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\n\x1EW`\0\x80\xFD[\x91\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\n6W`\0\x80\xFD[a\n?\x83a\n\x07V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\nbW`\0\x80\xFD[a\nk\x84a\n\x07V[\x92Pa\ny` \x85\x01a\n\x07V[\x91P`@\x84\x015\x90P\x92P\x92P\x92V[`\0\x80\x83`\x1F\x84\x01\x12a\n\x9BW`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\n\xB3W`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\n\xCBW`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`\0``\x86\x88\x03\x12\x15a\n\xEAW`\0\x80\xFD[\x855\x94P` \x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x0B\tW`\0\x80\xFD[a\x0B\x15\x89\x83\x8A\x01a\n\x89V[\x90\x96P\x94P`@\x88\x015\x91P\x80\x82\x11\x15a\x0B.W`\0\x80\xFD[Pa\x0B;\x88\x82\x89\x01a\n\x89V[\x96\x99\x95\x98P\x93\x96P\x92\x94\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\x0B^W`\0\x80\xFD[a\x0Bg\x82a\n\x07V[\x93\x92PPPV[`\0\x80`\0`@\x84\x86\x03\x12\x15a\x0B\x83W`\0\x80\xFD[\x835\x92P` \x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x0B\xA1W`\0\x80\xFD[a\x0B\xAD\x86\x82\x87\x01a\n\x89V[\x94\x97\x90\x96P\x93\x94PPPPV[`\0\x80`@\x83\x85\x03\x12\x15a\x0B\xCDW`\0\x80\xFD[a\x0B\xD6\x83a\n\x07V[\x91Pa\x0B\xE4` \x84\x01a\n\x07V[\x90P\x92P\x92\x90PV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x0C\x01W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x0C!WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\0\x82a\x0CDWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`s\x90\x82\x01R\x7FAstriaBridgeableERC20: insuffici`@\x82\x01R\x7Fent value, must be greater than ``\x82\x01R\x7F10 ** (TOKEN_DECIMALS - BASE_CHA`\x80\x82\x01RrIN_ASSET_PRECISION)`h\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\r%`@\x83\x01\x86\x88a\x0C\xE8V[\x82\x81\x03` \x84\x01Ra\r8\x81\x85\x87a\x0C\xE8V[\x97\x96PPPPPPPV[` \x81R`\0a\rW` \x83\x01\x84\x86a\x0C\xE8V[\x94\x93PPPPV[\x80\x82\x01\x80\x82\x11\x15a\x03cWcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD\xFE\xA2dipfsX\"\x12 z\x0Bo\xC7\n\x8B$\t|\xDE4\x12\xC1\xAE\xBD[\x92[J\x8B\xAE\xDA\x81#An8^D\x1F\x80\x94dsolcC\0\x08\x15\x003";
    /// The deployed bytecode of the contract.
    pub static ASTRIABRIDGEABLEERC20_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __DEPLOYED_BYTECODE,
    );
    pub struct AstriaBridgeableERC20<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for AstriaBridgeableERC20<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for AstriaBridgeableERC20<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for AstriaBridgeableERC20<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for AstriaBridgeableERC20<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(AstriaBridgeableERC20))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> AstriaBridgeableERC20<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(
                ::ethers::contract::Contract::new(
                    address.into(),
                    ASTRIABRIDGEABLEERC20_ABI.clone(),
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
                ASTRIABRIDGEABLEERC20_ABI.clone(),
                ASTRIABRIDGEABLEERC20_BYTECODE.clone().into(),
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
        ///Calls the contract's `BRIDGE` (0xee9a31a2) function
        pub fn bridge(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<
            M,
            ::ethers::core::types::Address,
        > {
            self.0
                .method_hash([238, 154, 49, 162], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `allowance` (0xdd62ed3e) function
        pub fn allowance(
            &self,
            owner: ::ethers::core::types::Address,
            spender: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([221, 98, 237, 62], (owner, spender))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `approve` (0x095ea7b3) function
        pub fn approve(
            &self,
            spender: ::ethers::core::types::Address,
            value: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([9, 94, 167, 179], (spender, value))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `balanceOf` (0x70a08231) function
        pub fn balance_of(
            &self,
            account: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([112, 160, 130, 49], account)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `decimals` (0x313ce567) function
        pub fn decimals(&self) -> ::ethers::contract::builders::ContractCall<M, u8> {
            self.0
                .method_hash([49, 60, 229, 103], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `mint` (0x40c10f19) function
        pub fn mint(
            &self,
            to: ::ethers::core::types::Address,
            amount: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([64, 193, 15, 25], (to, amount))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `name` (0x06fdde03) function
        pub fn name(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
            self.0
                .method_hash([6, 253, 222, 3], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `symbol` (0x95d89b41) function
        pub fn symbol(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
            self.0
                .method_hash([149, 216, 155, 65], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `totalSupply` (0x18160ddd) function
        pub fn total_supply(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([24, 22, 13, 221], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `transfer` (0xa9059cbb) function
        pub fn transfer(
            &self,
            to: ::ethers::core::types::Address,
            value: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([169, 5, 156, 187], (to, value))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `transferFrom` (0x23b872dd) function
        pub fn transfer_from(
            &self,
            from: ::ethers::core::types::Address,
            to: ::ethers::core::types::Address,
            value: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([35, 184, 114, 221], (from, to, value))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `withdrawToIbcChain` (0x5fe56b09) function
        pub fn withdraw_to_ibc_chain(
            &self,
            amount: ::ethers::core::types::U256,
            destination_chain_address: ::std::string::String,
            memo: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [95, 229, 107, 9],
                    (amount, destination_chain_address, memo),
                )
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `withdrawToSequencer` (0xd38fe9a7) function
        pub fn withdraw_to_sequencer(
            &self,
            amount: ::ethers::core::types::U256,
            destination_chain_address: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([211, 143, 233, 167], (amount, destination_chain_address))
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `Approval` event
        pub fn approval_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            ApprovalFilter,
        > {
            self.0.event()
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
        ///Gets the contract's `Mint` event
        pub fn mint_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, MintFilter> {
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
        ///Gets the contract's `Transfer` event
        pub fn transfer_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            TransferFilter,
        > {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<
            ::std::sync::Arc<M>,
            M,
            AstriaBridgeableERC20Events,
        > {
            self.0.event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
    for AstriaBridgeableERC20<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `ERC20InsufficientAllowance` with signature `ERC20InsufficientAllowance(address,uint256,uint256)` and selector `0xfb8f41b2`
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
        name = "ERC20InsufficientAllowance",
        abi = "ERC20InsufficientAllowance(address,uint256,uint256)"
    )]
    pub struct ERC20InsufficientAllowance {
        pub spender: ::ethers::core::types::Address,
        pub allowance: ::ethers::core::types::U256,
        pub needed: ::ethers::core::types::U256,
    }
    ///Custom Error type `ERC20InsufficientBalance` with signature `ERC20InsufficientBalance(address,uint256,uint256)` and selector `0xe450d38c`
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
        name = "ERC20InsufficientBalance",
        abi = "ERC20InsufficientBalance(address,uint256,uint256)"
    )]
    pub struct ERC20InsufficientBalance {
        pub sender: ::ethers::core::types::Address,
        pub balance: ::ethers::core::types::U256,
        pub needed: ::ethers::core::types::U256,
    }
    ///Custom Error type `ERC20InvalidApprover` with signature `ERC20InvalidApprover(address)` and selector `0xe602df05`
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
    #[etherror(name = "ERC20InvalidApprover", abi = "ERC20InvalidApprover(address)")]
    pub struct ERC20InvalidApprover {
        pub approver: ::ethers::core::types::Address,
    }
    ///Custom Error type `ERC20InvalidReceiver` with signature `ERC20InvalidReceiver(address)` and selector `0xec442f05`
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
    #[etherror(name = "ERC20InvalidReceiver", abi = "ERC20InvalidReceiver(address)")]
    pub struct ERC20InvalidReceiver {
        pub receiver: ::ethers::core::types::Address,
    }
    ///Custom Error type `ERC20InvalidSender` with signature `ERC20InvalidSender(address)` and selector `0x96c6fd1e`
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
    #[etherror(name = "ERC20InvalidSender", abi = "ERC20InvalidSender(address)")]
    pub struct ERC20InvalidSender {
        pub sender: ::ethers::core::types::Address,
    }
    ///Custom Error type `ERC20InvalidSpender` with signature `ERC20InvalidSpender(address)` and selector `0x94280d62`
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
    #[etherror(name = "ERC20InvalidSpender", abi = "ERC20InvalidSpender(address)")]
    pub struct ERC20InvalidSpender {
        pub spender: ::ethers::core::types::Address,
    }
    ///Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaBridgeableERC20Errors {
        ERC20InsufficientAllowance(ERC20InsufficientAllowance),
        ERC20InsufficientBalance(ERC20InsufficientBalance),
        ERC20InvalidApprover(ERC20InvalidApprover),
        ERC20InvalidReceiver(ERC20InvalidReceiver),
        ERC20InvalidSender(ERC20InvalidSender),
        ERC20InvalidSpender(ERC20InvalidSpender),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaBridgeableERC20Errors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) = <ERC20InsufficientAllowance as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ERC20InsufficientAllowance(decoded));
            }
            if let Ok(decoded) = <ERC20InsufficientBalance as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ERC20InsufficientBalance(decoded));
            }
            if let Ok(decoded) = <ERC20InvalidApprover as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ERC20InvalidApprover(decoded));
            }
            if let Ok(decoded) = <ERC20InvalidReceiver as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ERC20InvalidReceiver(decoded));
            }
            if let Ok(decoded) = <ERC20InvalidSender as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ERC20InvalidSender(decoded));
            }
            if let Ok(decoded) = <ERC20InvalidSpender as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ERC20InvalidSpender(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for AstriaBridgeableERC20Errors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::ERC20InsufficientAllowance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ERC20InsufficientBalance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ERC20InvalidApprover(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ERC20InvalidReceiver(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ERC20InvalidSender(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ERC20InvalidSpender(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for AstriaBridgeableERC20Errors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <ERC20InsufficientAllowance as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ERC20InsufficientBalance as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ERC20InvalidApprover as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ERC20InvalidReceiver as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ERC20InvalidSender as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ERC20InvalidSpender as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for AstriaBridgeableERC20Errors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ERC20InsufficientAllowance(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ERC20InsufficientBalance(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ERC20InvalidApprover(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ERC20InvalidReceiver(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ERC20InvalidSender(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::ERC20InvalidSpender(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for AstriaBridgeableERC20Errors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<ERC20InsufficientAllowance>
    for AstriaBridgeableERC20Errors {
        fn from(value: ERC20InsufficientAllowance) -> Self {
            Self::ERC20InsufficientAllowance(value)
        }
    }
    impl ::core::convert::From<ERC20InsufficientBalance>
    for AstriaBridgeableERC20Errors {
        fn from(value: ERC20InsufficientBalance) -> Self {
            Self::ERC20InsufficientBalance(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidApprover> for AstriaBridgeableERC20Errors {
        fn from(value: ERC20InvalidApprover) -> Self {
            Self::ERC20InvalidApprover(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidReceiver> for AstriaBridgeableERC20Errors {
        fn from(value: ERC20InvalidReceiver) -> Self {
            Self::ERC20InvalidReceiver(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidSender> for AstriaBridgeableERC20Errors {
        fn from(value: ERC20InvalidSender) -> Self {
            Self::ERC20InvalidSender(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidSpender> for AstriaBridgeableERC20Errors {
        fn from(value: ERC20InvalidSpender) -> Self {
            Self::ERC20InvalidSpender(value)
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
    #[ethevent(name = "Approval", abi = "Approval(address,address,uint256)")]
    pub struct ApprovalFilter {
        #[ethevent(indexed)]
        pub owner: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub spender: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
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
    #[ethevent(name = "Mint", abi = "Mint(address,uint256)")]
    pub struct MintFilter {
        #[ethevent(indexed)]
        pub account: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
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
    #[ethevent(name = "Transfer", abi = "Transfer(address,address,uint256)")]
    pub struct TransferFilter {
        #[ethevent(indexed)]
        pub from: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub to: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    ///Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaBridgeableERC20Events {
        ApprovalFilter(ApprovalFilter),
        Ics20WithdrawalFilter(Ics20WithdrawalFilter),
        MintFilter(MintFilter),
        SequencerWithdrawalFilter(SequencerWithdrawalFilter),
        TransferFilter(TransferFilter),
    }
    impl ::ethers::contract::EthLogDecode for AstriaBridgeableERC20Events {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = ApprovalFilter::decode_log(log) {
                return Ok(AstriaBridgeableERC20Events::ApprovalFilter(decoded));
            }
            if let Ok(decoded) = Ics20WithdrawalFilter::decode_log(log) {
                return Ok(AstriaBridgeableERC20Events::Ics20WithdrawalFilter(decoded));
            }
            if let Ok(decoded) = MintFilter::decode_log(log) {
                return Ok(AstriaBridgeableERC20Events::MintFilter(decoded));
            }
            if let Ok(decoded) = SequencerWithdrawalFilter::decode_log(log) {
                return Ok(
                    AstriaBridgeableERC20Events::SequencerWithdrawalFilter(decoded),
                );
            }
            if let Ok(decoded) = TransferFilter::decode_log(log) {
                return Ok(AstriaBridgeableERC20Events::TransferFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for AstriaBridgeableERC20Events {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ApprovalFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::Ics20WithdrawalFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::MintFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::SequencerWithdrawalFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::TransferFilter(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<ApprovalFilter> for AstriaBridgeableERC20Events {
        fn from(value: ApprovalFilter) -> Self {
            Self::ApprovalFilter(value)
        }
    }
    impl ::core::convert::From<Ics20WithdrawalFilter> for AstriaBridgeableERC20Events {
        fn from(value: Ics20WithdrawalFilter) -> Self {
            Self::Ics20WithdrawalFilter(value)
        }
    }
    impl ::core::convert::From<MintFilter> for AstriaBridgeableERC20Events {
        fn from(value: MintFilter) -> Self {
            Self::MintFilter(value)
        }
    }
    impl ::core::convert::From<SequencerWithdrawalFilter>
    for AstriaBridgeableERC20Events {
        fn from(value: SequencerWithdrawalFilter) -> Self {
            Self::SequencerWithdrawalFilter(value)
        }
    }
    impl ::core::convert::From<TransferFilter> for AstriaBridgeableERC20Events {
        fn from(value: TransferFilter) -> Self {
            Self::TransferFilter(value)
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
    ///Container type for all input parameters for the `BRIDGE` function with signature `BRIDGE()` and selector `0xee9a31a2`
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
    #[ethcall(name = "BRIDGE", abi = "BRIDGE()")]
    pub struct BridgeCall;
    ///Container type for all input parameters for the `allowance` function with signature `allowance(address,address)` and selector `0xdd62ed3e`
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
    #[ethcall(name = "allowance", abi = "allowance(address,address)")]
    pub struct AllowanceCall {
        pub owner: ::ethers::core::types::Address,
        pub spender: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `approve` function with signature `approve(address,uint256)` and selector `0x095ea7b3`
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
    #[ethcall(name = "approve", abi = "approve(address,uint256)")]
    pub struct ApproveCall {
        pub spender: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `balanceOf` function with signature `balanceOf(address)` and selector `0x70a08231`
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
    #[ethcall(name = "balanceOf", abi = "balanceOf(address)")]
    pub struct BalanceOfCall {
        pub account: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `decimals` function with signature `decimals()` and selector `0x313ce567`
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
    #[ethcall(name = "decimals", abi = "decimals()")]
    pub struct DecimalsCall;
    ///Container type for all input parameters for the `mint` function with signature `mint(address,uint256)` and selector `0x40c10f19`
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
    #[ethcall(name = "mint", abi = "mint(address,uint256)")]
    pub struct MintCall {
        pub to: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `name` function with signature `name()` and selector `0x06fdde03`
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
    #[ethcall(name = "name", abi = "name()")]
    pub struct NameCall;
    ///Container type for all input parameters for the `symbol` function with signature `symbol()` and selector `0x95d89b41`
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
    #[ethcall(name = "symbol", abi = "symbol()")]
    pub struct SymbolCall;
    ///Container type for all input parameters for the `totalSupply` function with signature `totalSupply()` and selector `0x18160ddd`
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
    #[ethcall(name = "totalSupply", abi = "totalSupply()")]
    pub struct TotalSupplyCall;
    ///Container type for all input parameters for the `transfer` function with signature `transfer(address,uint256)` and selector `0xa9059cbb`
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
    #[ethcall(name = "transfer", abi = "transfer(address,uint256)")]
    pub struct TransferCall {
        pub to: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `transferFrom` function with signature `transferFrom(address,address,uint256)` and selector `0x23b872dd`
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
    #[ethcall(name = "transferFrom", abi = "transferFrom(address,address,uint256)")]
    pub struct TransferFromCall {
        pub from: ::ethers::core::types::Address,
        pub to: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `withdrawToIbcChain` function with signature `withdrawToIbcChain(uint256,string,string)` and selector `0x5fe56b09`
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
        name = "withdrawToIbcChain",
        abi = "withdrawToIbcChain(uint256,string,string)"
    )]
    pub struct WithdrawToIbcChainCall {
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
        pub memo: ::std::string::String,
    }
    ///Container type for all input parameters for the `withdrawToSequencer` function with signature `withdrawToSequencer(uint256,string)` and selector `0xd38fe9a7`
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
    #[ethcall(name = "withdrawToSequencer", abi = "withdrawToSequencer(uint256,string)")]
    pub struct WithdrawToSequencerCall {
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
    }
    ///Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaBridgeableERC20Calls {
        BaseChainAssetDenomination(BaseChainAssetDenominationCall),
        BaseChainAssetPrecision(BaseChainAssetPrecisionCall),
        BaseChainBridgeAddress(BaseChainBridgeAddressCall),
        Bridge(BridgeCall),
        Allowance(AllowanceCall),
        Approve(ApproveCall),
        BalanceOf(BalanceOfCall),
        Decimals(DecimalsCall),
        Mint(MintCall),
        Name(NameCall),
        Symbol(SymbolCall),
        TotalSupply(TotalSupplyCall),
        Transfer(TransferCall),
        TransferFrom(TransferFromCall),
        WithdrawToIbcChain(WithdrawToIbcChainCall),
        WithdrawToSequencer(WithdrawToSequencerCall),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaBridgeableERC20Calls {
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
            if let Ok(decoded) = <BridgeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Bridge(decoded));
            }
            if let Ok(decoded) = <AllowanceCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Allowance(decoded));
            }
            if let Ok(decoded) = <ApproveCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Approve(decoded));
            }
            if let Ok(decoded) = <BalanceOfCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::BalanceOf(decoded));
            }
            if let Ok(decoded) = <DecimalsCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Decimals(decoded));
            }
            if let Ok(decoded) = <MintCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Mint(decoded));
            }
            if let Ok(decoded) = <NameCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Name(decoded));
            }
            if let Ok(decoded) = <SymbolCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Symbol(decoded));
            }
            if let Ok(decoded) = <TotalSupplyCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::TotalSupply(decoded));
            }
            if let Ok(decoded) = <TransferCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Transfer(decoded));
            }
            if let Ok(decoded) = <TransferFromCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::TransferFrom(decoded));
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
    impl ::ethers::core::abi::AbiEncode for AstriaBridgeableERC20Calls {
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
                Self::Bridge(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Allowance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Approve(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::BalanceOf(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Decimals(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Mint(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Name(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Symbol(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::TotalSupply(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Transfer(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::TransferFrom(element) => {
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
    impl ::core::fmt::Display for AstriaBridgeableERC20Calls {
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
                Self::Bridge(element) => ::core::fmt::Display::fmt(element, f),
                Self::Allowance(element) => ::core::fmt::Display::fmt(element, f),
                Self::Approve(element) => ::core::fmt::Display::fmt(element, f),
                Self::BalanceOf(element) => ::core::fmt::Display::fmt(element, f),
                Self::Decimals(element) => ::core::fmt::Display::fmt(element, f),
                Self::Mint(element) => ::core::fmt::Display::fmt(element, f),
                Self::Name(element) => ::core::fmt::Display::fmt(element, f),
                Self::Symbol(element) => ::core::fmt::Display::fmt(element, f),
                Self::TotalSupply(element) => ::core::fmt::Display::fmt(element, f),
                Self::Transfer(element) => ::core::fmt::Display::fmt(element, f),
                Self::TransferFrom(element) => ::core::fmt::Display::fmt(element, f),
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
    for AstriaBridgeableERC20Calls {
        fn from(value: BaseChainAssetDenominationCall) -> Self {
            Self::BaseChainAssetDenomination(value)
        }
    }
    impl ::core::convert::From<BaseChainAssetPrecisionCall>
    for AstriaBridgeableERC20Calls {
        fn from(value: BaseChainAssetPrecisionCall) -> Self {
            Self::BaseChainAssetPrecision(value)
        }
    }
    impl ::core::convert::From<BaseChainBridgeAddressCall>
    for AstriaBridgeableERC20Calls {
        fn from(value: BaseChainBridgeAddressCall) -> Self {
            Self::BaseChainBridgeAddress(value)
        }
    }
    impl ::core::convert::From<BridgeCall> for AstriaBridgeableERC20Calls {
        fn from(value: BridgeCall) -> Self {
            Self::Bridge(value)
        }
    }
    impl ::core::convert::From<AllowanceCall> for AstriaBridgeableERC20Calls {
        fn from(value: AllowanceCall) -> Self {
            Self::Allowance(value)
        }
    }
    impl ::core::convert::From<ApproveCall> for AstriaBridgeableERC20Calls {
        fn from(value: ApproveCall) -> Self {
            Self::Approve(value)
        }
    }
    impl ::core::convert::From<BalanceOfCall> for AstriaBridgeableERC20Calls {
        fn from(value: BalanceOfCall) -> Self {
            Self::BalanceOf(value)
        }
    }
    impl ::core::convert::From<DecimalsCall> for AstriaBridgeableERC20Calls {
        fn from(value: DecimalsCall) -> Self {
            Self::Decimals(value)
        }
    }
    impl ::core::convert::From<MintCall> for AstriaBridgeableERC20Calls {
        fn from(value: MintCall) -> Self {
            Self::Mint(value)
        }
    }
    impl ::core::convert::From<NameCall> for AstriaBridgeableERC20Calls {
        fn from(value: NameCall) -> Self {
            Self::Name(value)
        }
    }
    impl ::core::convert::From<SymbolCall> for AstriaBridgeableERC20Calls {
        fn from(value: SymbolCall) -> Self {
            Self::Symbol(value)
        }
    }
    impl ::core::convert::From<TotalSupplyCall> for AstriaBridgeableERC20Calls {
        fn from(value: TotalSupplyCall) -> Self {
            Self::TotalSupply(value)
        }
    }
    impl ::core::convert::From<TransferCall> for AstriaBridgeableERC20Calls {
        fn from(value: TransferCall) -> Self {
            Self::Transfer(value)
        }
    }
    impl ::core::convert::From<TransferFromCall> for AstriaBridgeableERC20Calls {
        fn from(value: TransferFromCall) -> Self {
            Self::TransferFrom(value)
        }
    }
    impl ::core::convert::From<WithdrawToIbcChainCall> for AstriaBridgeableERC20Calls {
        fn from(value: WithdrawToIbcChainCall) -> Self {
            Self::WithdrawToIbcChain(value)
        }
    }
    impl ::core::convert::From<WithdrawToSequencerCall> for AstriaBridgeableERC20Calls {
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
    ///Container type for all return fields from the `BRIDGE` function with signature `BRIDGE()` and selector `0xee9a31a2`
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
    pub struct BridgeReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `allowance` function with signature `allowance(address,address)` and selector `0xdd62ed3e`
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
    pub struct AllowanceReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `approve` function with signature `approve(address,uint256)` and selector `0x095ea7b3`
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
    pub struct ApproveReturn(pub bool);
    ///Container type for all return fields from the `balanceOf` function with signature `balanceOf(address)` and selector `0x70a08231`
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
    pub struct BalanceOfReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `decimals` function with signature `decimals()` and selector `0x313ce567`
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
    pub struct DecimalsReturn(pub u8);
    ///Container type for all return fields from the `name` function with signature `name()` and selector `0x06fdde03`
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
    pub struct NameReturn(pub ::std::string::String);
    ///Container type for all return fields from the `symbol` function with signature `symbol()` and selector `0x95d89b41`
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
    pub struct SymbolReturn(pub ::std::string::String);
    ///Container type for all return fields from the `totalSupply` function with signature `totalSupply()` and selector `0x18160ddd`
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
    pub struct TotalSupplyReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `transfer` function with signature `transfer(address,uint256)` and selector `0xa9059cbb`
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
    pub struct TransferReturn(pub bool);
    ///Container type for all return fields from the `transferFrom` function with signature `transferFrom(address,address,uint256)` and selector `0x23b872dd`
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
    pub struct TransferFromReturn(pub bool);
}
