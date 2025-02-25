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
                                    name: ::std::borrow::ToOwned::to_owned(
                                        "_destinationRollupBridgeAddress",
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
                            state_mutability: ::ethers::core::abi::ethabi::StateMutability::Payable,
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
    pub static ASTRIABRIDGEABLEERC20_ABI: ::ethers::contract::Lazy<
        ::ethers::core::abi::Abi,
    > = ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\xE0`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`@Qa\x1B\x178\x03\x80a\x1B\x17\x839\x81\x01`@\x81\x90Ra\0/\x91a\x02\xEBV[\x84\x843\x80a\0XW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\0a\x81a\x01\xADV[P`\na\0n\x83\x82a\x04\x7FV[P`\x0Ba\0{\x82\x82a\x04\x7FV[PPP`\0a\0\x8Ea\x01\xFD` \x1B` \x1CV[\x90P\x80`\xFF\x16\x89c\xFF\xFF\xFF\xFF\x16\x11\x15a\x015W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`^`$\x82\x01R\x7FAstriaBridgeableERC20: base chai`D\x82\x01R\x7Fn asset precision must be less t`d\x82\x01R\x7Fhan or equal to token decimals\0\0`\x84\x82\x01R`\xA4\x01a\0OV[c\xFF\xFF\xFF\xFF\x89\x16`\x80R`\x01a\x01K\x89\x82a\x04\x7FV[P`\x02a\x01X\x88\x82a\x04\x7FV[Pa\x01f\x89`\xFF\x83\x16a\x05SV[a\x01q\x90`\na\x06\\V[`\xC0RP`\x01`\x01`\xA0\x1B\x03\x98\x89\x16`\xA0R`\x03\x92\x90\x92U`\x04U`\x05\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x91\x90\x97\x16\x17\x90\x95UPa\x06u\x93PPPPV[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\x12\x90V[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x02\x19W`\0\x80\xFD[\x91\x90PV[\x80Qc\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x02\x19W`\0\x80\xFD[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0\x82`\x1F\x83\x01\x12a\x02YW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x02rWa\x02ra\x022V[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x02\xA0Wa\x02\xA0a\x022V[`@R\x81\x81R\x83\x82\x01` \x01\x85\x10\x15a\x02\xB8W`\0\x80\xFD[`\0[\x82\x81\x10\x15a\x02\xD7W` \x81\x86\x01\x81\x01Q\x83\x83\x01\x82\x01R\x01a\x02\xBBV[P`\0\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[`\0\x80`\0\x80`\0\x80`\0\x80`\0a\x01 \x8A\x8C\x03\x12\x15a\x03\nW`\0\x80\xFD[a\x03\x13\x8Aa\x02\x02V[\x98Pa\x03!` \x8B\x01a\x02\x1EV[`@\x8B\x01Q\x90\x98P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03=W`\0\x80\xFD[a\x03I\x8C\x82\x8D\x01a\x02HV[``\x8C\x01Q\x90\x98P\x90P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03gW`\0\x80\xFD[a\x03s\x8C\x82\x8D\x01a\x02HV[`\x80\x8C\x01Q\x90\x97P\x90P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03\x91W`\0\x80\xFD[a\x03\x9D\x8C\x82\x8D\x01a\x02HV[`\xA0\x8C\x01Q\x90\x96P\x90P`\x01`\x01`@\x1B\x03\x81\x11\x15a\x03\xBBW`\0\x80\xFD[a\x03\xC7\x8C\x82\x8D\x01a\x02HV[`\xC0\x8C\x01Q`\xE0\x8D\x01Q\x91\x96P\x94P\x92Pa\x03\xE7\x90Pa\x01\0\x8B\x01a\x02\x02V[\x90P\x92\x95\x98P\x92\x95\x98P\x92\x95\x98V[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x04\nW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x04*WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15a\x04zW\x80`\0R` `\0 `\x1F\x84\x01`\x05\x1C\x81\x01` \x85\x10\x15a\x04WWP\x80[`\x1F\x84\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15a\x04wW`\0\x81U`\x01\x01a\x04cV[PP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15a\x04\x98Wa\x04\x98a\x022V[a\x04\xAC\x81a\x04\xA6\x84Ta\x03\xF6V[\x84a\x040V[` `\x1F\x82\x11`\x01\x81\x14a\x04\xE0W`\0\x83\x15a\x04\xC8WP\x84\x82\x01Q[`\0\x19`\x03\x85\x90\x1B\x1C\x19\x16`\x01\x84\x90\x1B\x17\x84Ua\x04wV[`\0\x84\x81R` \x81 `\x1F\x19\x85\x16\x91[\x82\x81\x10\x15a\x05\x10W\x87\x85\x01Q\x82U` \x94\x85\x01\x94`\x01\x90\x92\x01\x91\x01a\x04\xF0V[P\x84\x82\x10\x15a\x05.W\x86\x84\x01Q`\0\x19`\x03\x87\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPP`\x01\x90\x81\x1B\x01\x90UPV[cNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[c\xFF\xFF\xFF\xFF\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\x05oWa\x05oa\x05=V[\x92\x91PPV[`\x01\x81[`\x01\x84\x11\x15a\x05\xB0W\x80\x85\x04\x81\x11\x15a\x05\x94Wa\x05\x94a\x05=V[`\x01\x84\x16\x15a\x05\xA2W\x90\x81\x02\x90[`\x01\x93\x90\x93\x1C\x92\x80\x02a\x05yV[\x93P\x93\x91PPV[`\0\x82a\x05\xC7WP`\x01a\x05oV[\x81a\x05\xD4WP`\0a\x05oV[\x81`\x01\x81\x14a\x05\xEAW`\x02\x81\x14a\x05\xF4Wa\x06\x10V[`\x01\x91PPa\x05oV[`\xFF\x84\x11\x15a\x06\x05Wa\x06\x05a\x05=V[PP`\x01\x82\x1Ba\x05oV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x063WP\x81\x81\na\x05oV[a\x06@`\0\x19\x84\x84a\x05uV[\x80`\0\x19\x04\x82\x11\x15a\x06TWa\x06Ta\x05=V[\x02\x93\x92PPPV[`\0a\x06nc\xFF\xFF\xFF\xFF\x84\x16\x83a\x05\xB8V[\x93\x92PPPV[`\x80Q`\xA0Q`\xC0Qa\x14^a\x06\xB9`\09`\0\x81\x81a\x07?\x01R\x81\x81a\x08\xF7\x01Ra\n\xF1\x01R`\0\x81\x81a\x05\x0B\x01Ra\x06>\x01R`\0a\x03\x1A\x01Ra\x14^`\0\xF3\xFE`\x80`@R`\x046\x10a\x01\xB7W`\x005`\xE0\x1C\x80c\xA7\xEA\xA79\x11a\0\xECW\x80c\xDB\x97\xDC\x98\x11a\0\x8AW\x80c\xEB\xD0\x90T\x11a\0dW\x80c\xEB\xD0\x90T\x14a\x04\xD9W\x80c\xEE\x9A1\xA2\x14a\x04\xF9W\x80c\xF2\xFD\xE3\x8B\x14a\x05-W\x80c\xFC\x88\xD3\x1B\x14a\x05MW`\0\x80\xFD[\x80c\xDB\x97\xDC\x98\x14a\x04^W\x80c\xDDb\xED>\x14a\x04sW\x80c\xE7K\x98\x1B\x14a\x04\xB9W`\0\x80\xFD[\x80c\xB6Gl~\x11a\0\xC6W\x80c\xB6Gl~\x14a\x04\x0EW\x80c\xCB\xEE\x8C\xFA\x14a\x04#W\x80c\xD2\x94\xF0\x93\x14a\x046W\x80c\xD3\x8F\xE9\xA7\x14a\x04KW`\0\x80\xFD[\x80c\xA7\xEA\xA79\x14a\x03\xB8W\x80c\xA9\x05\x9C\xBB\x14a\x03\xD8W\x80c\xAD\"\x82G\x14a\x03\xF8W`\0\x80\xFD[\x80coF8J\x11a\x01YW\x80c~\xB6\xDE\xC7\x11a\x013W\x80c~\xB6\xDE\xC7\x14a\x03\x08W\x80c\x88\x979y\x14a\x03QW\x80c\x8D\xA5\xCB[\x14a\x03qW\x80c\x95\xD8\x9BA\x14a\x03\xA3W`\0\x80\xFD[\x80coF8J\x14a\x02\xA7W\x80cp\xA0\x821\x14a\x02\xBDW\x80cqP\x18\xA6\x14a\x02\xF3W`\0\x80\xFD[\x80c#\xB8r\xDD\x11a\x01\x95W\x80c#\xB8r\xDD\x14a\x026W\x80c1<\xE5g\x14a\x02VW\x80c@\xC1\x0F\x19\x14a\x02rW\x80c_\xE5k\t\x14a\x02\x94W`\0\x80\xFD[\x80c\x06\xFD\xDE\x03\x14a\x01\xBCW\x80c\t^\xA7\xB3\x14a\x01\xE7W\x80c\x18\x16\r\xDD\x14a\x02\x17W[`\0\x80\xFD[4\x80\x15a\x01\xC8W`\0\x80\xFD[Pa\x01\xD1a\x05cV[`@Qa\x01\xDE\x91\x90a\x0F\xF0V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\xF3W`\0\x80\xFD[Pa\x02\x07a\x02\x026`\x04a\x10ZV[a\x05\xF5V[`@Q\x90\x15\x15\x81R` \x01a\x01\xDEV[4\x80\x15a\x02#W`\0\x80\xFD[P`\tT[`@Q\x90\x81R` \x01a\x01\xDEV[4\x80\x15a\x02BW`\0\x80\xFD[Pa\x02\x07a\x02Q6`\x04a\x10\x84V[a\x06\x0FV[4\x80\x15a\x02bW`\0\x80\xFD[P`@Q`\x12\x81R` \x01a\x01\xDEV[4\x80\x15a\x02~W`\0\x80\xFD[Pa\x02\x92a\x02\x8D6`\x04a\x10ZV[a\x063V[\0[a\x02\x92a\x02\xA26`\x04a\x11\nV[a\x07\x15V[4\x80\x15a\x02\xB3W`\0\x80\xFD[Pa\x02(`\x03T\x81V[4\x80\x15a\x02\xC9W`\0\x80\xFD[Pa\x02(a\x02\xD86`\x04a\x11\x89V[`\x01`\x01`\xA0\x1B\x03\x16`\0\x90\x81R`\x07` R`@\x90 T\x90V[4\x80\x15a\x02\xFFW`\0\x80\xFD[Pa\x02\x92a\x07\xF4V[4\x80\x15a\x03\x14W`\0\x80\xFD[Pa\x03<\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\xDEV[4\x80\x15a\x03]W`\0\x80\xFD[Pa\x02\x92a\x03l6`\x04a\x11\xABV[a\x08\x08V[4\x80\x15a\x03}W`\0\x80\xFD[P`\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\xDEV[4\x80\x15a\x03\xAFW`\0\x80\xFD[Pa\x01\xD1a\x08\x15V[4\x80\x15a\x03\xC4W`\0\x80\xFD[Pa\x02\x92a\x03\xD36`\x04a\x11\xABV[a\x08$V[4\x80\x15a\x03\xE4W`\0\x80\xFD[Pa\x02\x07a\x03\xF36`\x04a\x10ZV[a\x081V[4\x80\x15a\x04\x04W`\0\x80\xFD[Pa\x02(`\x06T\x81V[4\x80\x15a\x04\x1AW`\0\x80\xFD[Pa\x01\xD1a\x08?V[a\x02\x92a\x0416`\x04a\x11\nV[a\x08\xCDV[4\x80\x15a\x04BW`\0\x80\xFD[Pa\x02\x92a\t\x9BV[a\x02\x92a\x04Y6`\x04a\x11\xC4V[a\n\xC7V[4\x80\x15a\x04jW`\0\x80\xFD[Pa\x01\xD1a\x0B\xA0V[4\x80\x15a\x04\x7FW`\0\x80\xFD[Pa\x02(a\x04\x8E6`\x04a\x12\x10V[`\x01`\x01`\xA0\x1B\x03\x91\x82\x16`\0\x90\x81R`\x08` \x90\x81R`@\x80\x83 \x93\x90\x94\x16\x82R\x91\x90\x91R T\x90V[4\x80\x15a\x04\xC5W`\0\x80\xFD[Pa\x02\x92a\x04\xD46`\x04a\x11\x89V[a\x0B\xADV[4\x80\x15a\x04\xE5W`\0\x80\xFD[P`\x05Ta\x03\x8B\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x05\x05W`\0\x80\xFD[Pa\x03\x8B\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[4\x80\x15a\x059W`\0\x80\xFD[Pa\x02\x92a\x05H6`\x04a\x11\x89V[a\x0B\xD7V[4\x80\x15a\x05YW`\0\x80\xFD[Pa\x02(`\x04T\x81V[```\n\x80Ta\x05r\x90a\x12CV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05\x9E\x90a\x12CV[\x80\x15a\x05\xEBW\x80`\x1F\x10a\x05\xC0Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x05\xEBV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x05\xCEW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x90P\x90V[`\x003a\x06\x03\x81\x85\x85a\x0C\x15V[`\x01\x91PP[\x92\x91PPV[`\x003a\x06\x1D\x85\x82\x85a\x0C'V[a\x06(\x85\x85\x85a\x0C\xA5V[P`\x01\x94\x93PPPPV[3`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x06\xC4W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FAstriaBridgeableERC20: only brid`D\x82\x01Rj\x19\xD9H\x18\xD8[\x88\x1BZ[\x9D`\xAA\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[a\x06\xCE\x82\x82a\r\x04V[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Fg\x98\xA5`y:T\xC3\xBC\xFE\x86\xA9<\xDE\x1Es\x08}\x94L\x0E\xA2\x05D\x13}A!9h\x85\x82`@Qa\x07\t\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPV[\x84`\x04T\x804\x14a\x078W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12}V[`\0a\x07d\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x84a\x12\xCFV[\x11a\x07\x81W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12\xF1V[4`\x06`\0\x82\x82Ta\x07\x93\x91\x90a\x13\x90V[\x90\x91UPa\x07\xA3\x90P3\x88a\r>V[\x863`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x88\x88\x88\x88`@Qa\x07\xE3\x94\x93\x92\x91\x90a\x13\xDAV[`@Q\x80\x91\x03\x90\xA3PPPPPPPV[a\x07\xFCa\rtV[a\x08\x06`\0a\r\xA1V[V[a\x08\x10a\rtV[`\x04UV[```\x0B\x80Ta\x05r\x90a\x12CV[a\x08,a\rtV[`\x03UV[`\x003a\x06\x03\x81\x85\x85a\x0C\xA5V[`\x02\x80Ta\x08L\x90a\x12CV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x08x\x90a\x12CV[\x80\x15a\x08\xC5W\x80`\x1F\x10a\x08\x9AWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x08\xC5V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x08\xA8W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\x03T\x804\x14a\x08\xF0W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12}V[`\0a\t\x1C\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x84a\x12\xCFV[\x11a\t9W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12\xF1V[4`\x06`\0\x82\x82Ta\tK\x91\x90a\x13\x90V[\x90\x91UPa\t[\x90P3\x88a\r>V[\x863`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE2\xA9\xF2nJR8\xEC\tr\xCE>\xD7'1!.\xBC\x8D'i\xCF\x7F\x17\xD8\xBE\xD2\x8A\x0F\xC8i\xF5\x88\x88\x88\x88`@Qa\x07\xE3\x94\x93\x92\x91\x90a\x13\xDAV[`\x05T`\x01`\x01`\xA0\x1B\x03\x163\x14a\n\x07W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`)`$\x82\x01R\x7FAstriaBridgeableERC20: only fee `D\x82\x01Rh\x1C\x99X\xDA\\\x1AY[\x9D`\xBA\x1B`d\x82\x01R`\x84\x01a\x06\xBBV[`\x05T`\x06T`@Q`\0\x92`\x01`\x01`\xA0\x1B\x03\x16\x91\x90\x83\x81\x81\x81\x85\x87Z\xF1\x92PPP=\x80`\0\x81\x14a\nVW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=`\0` \x84\x01>a\n[V[``\x91P[PP\x90P\x80a\n\xBFW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`*`$\x82\x01R\x7FAstriaBridgeableERC20: fee trans`D\x82\x01Ri\x19\x99\\\x88\x19\x98Z[\x19Y`\xB2\x1B`d\x82\x01R`\x84\x01a\x06\xBBV[P`\0`\x06UV[\x82`\x03T\x804\x14a\n\xEAW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12}V[`\0a\x0B\x16\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x84a\x12\xCFV[\x11a\x0B3W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12\xF1V[4`\x06`\0\x82\x82Ta\x0BE\x91\x90a\x13\x90V[\x90\x91UPa\x0BU\x90P3\x86a\r>V[\x843`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x86\x86`@Qa\x0B\x91\x92\x91\x90a\x14\x0CV[`@Q\x80\x91\x03\x90\xA3PPPPPV[`\x01\x80Ta\x08L\x90a\x12CV[a\x0B\xB5a\rtV[`\x05\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[a\x0B\xDFa\rtV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0C\tW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\x0C\x12\x81a\r\xA1V[PV[a\x0C\"\x83\x83\x83`\x01a\r\xF1V[PPPV[`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\0\x90\x81R`\x08` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R T`\0\x19\x81\x14a\x0C\x9FW\x81\x81\x10\x15a\x0C\x90W`@Qc}\xC7\xA0\xD9`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x06\xBBV[a\x0C\x9F\x84\x84\x84\x84\x03`\0a\r\xF1V[PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x0C\xCFW`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x0C\xF9W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\x0C\"\x83\x83\x83a\x0E\xC6V[`\x01`\x01`\xA0\x1B\x03\x82\x16a\r.W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\r:`\0\x83\x83a\x0E\xC6V[PPV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\rhW`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\r:\x82`\0\x83a\x0E\xC6V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\x06W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x06\xBBV[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\x01`\x01`\xA0\x1B\x03\x84\x16a\x0E\x1BW`@Qc\xE6\x02\xDF\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x0EEW`@QcJ\x14\x06\xB1`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16`\0\x90\x81R`\x08` \x90\x81R`@\x80\x83 \x93\x87\x16\x83R\x92\x90R \x82\x90U\x80\x15a\x0C\x9FW\x82`\x01`\x01`\xA0\x1B\x03\x16\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\x8C[\xE1\xE5\xEB\xEC}[\xD1OqB}\x1E\x84\xF3\xDD\x03\x14\xC0\xF7\xB2)\x1E[ \n\xC8\xC7\xC3\xB9%\x84`@Qa\x0E\xB8\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x0E\xF1W\x80`\t`\0\x82\x82Ta\x0E\xE6\x91\x90a\x13\x90V[\x90\x91UPa\x0Fc\x90PV[`\x01`\x01`\xA0\x1B\x03\x83\x16`\0\x90\x81R`\x07` R`@\x90 T\x81\x81\x10\x15a\x0FDW`@Qc9\x144\xE3`\xE2\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x84\x16`\0\x90\x81R`\x07` R`@\x90 \x90\x82\x90\x03\x90U[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x0F\x7FW`\t\x80T\x82\x90\x03\x90Ua\x0F\x9EV[`\x01`\x01`\xA0\x1B\x03\x82\x16`\0\x90\x81R`\x07` R`@\x90 \x80T\x82\x01\x90U[\x81`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDD\xF2R\xAD\x1B\xE2\xC8\x9Bi\xC2\xB0h\xFC7\x8D\xAA\x95+\xA7\xF1c\xC4\xA1\x16(\xF5ZM\xF5#\xB3\xEF\x83`@Qa\x0F\xE3\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPV[` \x81R`\0\x82Q\x80` \x84\x01R`\0[\x81\x81\x10\x15a\x10\x1EW` \x81\x86\x01\x81\x01Q`@\x86\x84\x01\x01R\x01a\x10\x01V[P`\0`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x10UW`\0\x80\xFD[\x91\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\x10mW`\0\x80\xFD[a\x10v\x83a\x10>V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\x10\x99W`\0\x80\xFD[a\x10\xA2\x84a\x10>V[\x92Pa\x10\xB0` \x85\x01a\x10>V[\x92\x95\x92\x94PPP`@\x91\x90\x91\x015\x90V[`\0\x80\x83`\x1F\x84\x01\x12a\x10\xD3W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x10\xEBW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x11\x03W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`\0``\x86\x88\x03\x12\x15a\x11\"W`\0\x80\xFD[\x855\x94P` \x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x11@W`\0\x80\xFD[a\x11L\x88\x82\x89\x01a\x10\xC1V[\x90\x95P\x93PP`@\x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x11lW`\0\x80\xFD[a\x11x\x88\x82\x89\x01a\x10\xC1V[\x96\x99\x95\x98P\x93\x96P\x92\x94\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\x11\x9BW`\0\x80\xFD[a\x11\xA4\x82a\x10>V[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\x11\xBDW`\0\x80\xFD[P5\x91\x90PV[`\0\x80`\0`@\x84\x86\x03\x12\x15a\x11\xD9W`\0\x80\xFD[\x835\x92P` \x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x11\xF7W`\0\x80\xFD[a\x12\x03\x86\x82\x87\x01a\x10\xC1V[\x94\x97\x90\x96P\x93\x94PPPPV[`\0\x80`@\x83\x85\x03\x12\x15a\x12#W`\0\x80\xFD[a\x12,\x83a\x10>V[\x91Pa\x12:` \x84\x01a\x10>V[\x90P\x92P\x92\x90PV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x12WW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x12wWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x80\x82R`2\x90\x82\x01R\x7FAstriaBridgeableERC20: insuffici`@\x82\x01Rqent withdrawal fee`p\x1B``\x82\x01R`\x80\x01\x90V[`\0\x82a\x12\xECWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`s\x90\x82\x01R\x7FAstriaBridgeableERC20: insuffici`@\x82\x01R\x7Fent value, must be greater than ``\x82\x01R\x7F10 ** (TOKEN_DECIMALS - BASE_CHA`\x80\x82\x01RrIN_ASSET_PRECISION)`h\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x80\x82\x01\x80\x82\x11\x15a\x06\tWcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x13\xEE`@\x83\x01\x86\x88a\x13\xB1V[\x82\x81\x03` \x84\x01Ra\x14\x01\x81\x85\x87a\x13\xB1V[\x97\x96PPPPPPPV[` \x81R`\0a\x14 ` \x83\x01\x84\x86a\x13\xB1V[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 .^\xF6\x0C\x90\xC3\x9Br\xCC\xF5~\xBC\x92\x82\xD8\x8Eh\xF5.~\xD4\xC2\xA1\xE4\xE2\xF2\xFE\xEF@u\x97\x0EdsolcC\0\x08\x1A\x003";
    /// The bytecode of the contract.
    pub static ASTRIABRIDGEABLEERC20_BYTECODE: ::ethers::core::types::Bytes = ::ethers::core::types::Bytes::from_static(
        __BYTECODE,
    );
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R`\x046\x10a\x01\xB7W`\x005`\xE0\x1C\x80c\xA7\xEA\xA79\x11a\0\xECW\x80c\xDB\x97\xDC\x98\x11a\0\x8AW\x80c\xEB\xD0\x90T\x11a\0dW\x80c\xEB\xD0\x90T\x14a\x04\xD9W\x80c\xEE\x9A1\xA2\x14a\x04\xF9W\x80c\xF2\xFD\xE3\x8B\x14a\x05-W\x80c\xFC\x88\xD3\x1B\x14a\x05MW`\0\x80\xFD[\x80c\xDB\x97\xDC\x98\x14a\x04^W\x80c\xDDb\xED>\x14a\x04sW\x80c\xE7K\x98\x1B\x14a\x04\xB9W`\0\x80\xFD[\x80c\xB6Gl~\x11a\0\xC6W\x80c\xB6Gl~\x14a\x04\x0EW\x80c\xCB\xEE\x8C\xFA\x14a\x04#W\x80c\xD2\x94\xF0\x93\x14a\x046W\x80c\xD3\x8F\xE9\xA7\x14a\x04KW`\0\x80\xFD[\x80c\xA7\xEA\xA79\x14a\x03\xB8W\x80c\xA9\x05\x9C\xBB\x14a\x03\xD8W\x80c\xAD\"\x82G\x14a\x03\xF8W`\0\x80\xFD[\x80coF8J\x11a\x01YW\x80c~\xB6\xDE\xC7\x11a\x013W\x80c~\xB6\xDE\xC7\x14a\x03\x08W\x80c\x88\x979y\x14a\x03QW\x80c\x8D\xA5\xCB[\x14a\x03qW\x80c\x95\xD8\x9BA\x14a\x03\xA3W`\0\x80\xFD[\x80coF8J\x14a\x02\xA7W\x80cp\xA0\x821\x14a\x02\xBDW\x80cqP\x18\xA6\x14a\x02\xF3W`\0\x80\xFD[\x80c#\xB8r\xDD\x11a\x01\x95W\x80c#\xB8r\xDD\x14a\x026W\x80c1<\xE5g\x14a\x02VW\x80c@\xC1\x0F\x19\x14a\x02rW\x80c_\xE5k\t\x14a\x02\x94W`\0\x80\xFD[\x80c\x06\xFD\xDE\x03\x14a\x01\xBCW\x80c\t^\xA7\xB3\x14a\x01\xE7W\x80c\x18\x16\r\xDD\x14a\x02\x17W[`\0\x80\xFD[4\x80\x15a\x01\xC8W`\0\x80\xFD[Pa\x01\xD1a\x05cV[`@Qa\x01\xDE\x91\x90a\x0F\xF0V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\xF3W`\0\x80\xFD[Pa\x02\x07a\x02\x026`\x04a\x10ZV[a\x05\xF5V[`@Q\x90\x15\x15\x81R` \x01a\x01\xDEV[4\x80\x15a\x02#W`\0\x80\xFD[P`\tT[`@Q\x90\x81R` \x01a\x01\xDEV[4\x80\x15a\x02BW`\0\x80\xFD[Pa\x02\x07a\x02Q6`\x04a\x10\x84V[a\x06\x0FV[4\x80\x15a\x02bW`\0\x80\xFD[P`@Q`\x12\x81R` \x01a\x01\xDEV[4\x80\x15a\x02~W`\0\x80\xFD[Pa\x02\x92a\x02\x8D6`\x04a\x10ZV[a\x063V[\0[a\x02\x92a\x02\xA26`\x04a\x11\nV[a\x07\x15V[4\x80\x15a\x02\xB3W`\0\x80\xFD[Pa\x02(`\x03T\x81V[4\x80\x15a\x02\xC9W`\0\x80\xFD[Pa\x02(a\x02\xD86`\x04a\x11\x89V[`\x01`\x01`\xA0\x1B\x03\x16`\0\x90\x81R`\x07` R`@\x90 T\x90V[4\x80\x15a\x02\xFFW`\0\x80\xFD[Pa\x02\x92a\x07\xF4V[4\x80\x15a\x03\x14W`\0\x80\xFD[Pa\x03<\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\xDEV[4\x80\x15a\x03]W`\0\x80\xFD[Pa\x02\x92a\x03l6`\x04a\x11\xABV[a\x08\x08V[4\x80\x15a\x03}W`\0\x80\xFD[P`\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\xDEV[4\x80\x15a\x03\xAFW`\0\x80\xFD[Pa\x01\xD1a\x08\x15V[4\x80\x15a\x03\xC4W`\0\x80\xFD[Pa\x02\x92a\x03\xD36`\x04a\x11\xABV[a\x08$V[4\x80\x15a\x03\xE4W`\0\x80\xFD[Pa\x02\x07a\x03\xF36`\x04a\x10ZV[a\x081V[4\x80\x15a\x04\x04W`\0\x80\xFD[Pa\x02(`\x06T\x81V[4\x80\x15a\x04\x1AW`\0\x80\xFD[Pa\x01\xD1a\x08?V[a\x02\x92a\x0416`\x04a\x11\nV[a\x08\xCDV[4\x80\x15a\x04BW`\0\x80\xFD[Pa\x02\x92a\t\x9BV[a\x02\x92a\x04Y6`\x04a\x11\xC4V[a\n\xC7V[4\x80\x15a\x04jW`\0\x80\xFD[Pa\x01\xD1a\x0B\xA0V[4\x80\x15a\x04\x7FW`\0\x80\xFD[Pa\x02(a\x04\x8E6`\x04a\x12\x10V[`\x01`\x01`\xA0\x1B\x03\x91\x82\x16`\0\x90\x81R`\x08` \x90\x81R`@\x80\x83 \x93\x90\x94\x16\x82R\x91\x90\x91R T\x90V[4\x80\x15a\x04\xC5W`\0\x80\xFD[Pa\x02\x92a\x04\xD46`\x04a\x11\x89V[a\x0B\xADV[4\x80\x15a\x04\xE5W`\0\x80\xFD[P`\x05Ta\x03\x8B\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x05\x05W`\0\x80\xFD[Pa\x03\x8B\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[4\x80\x15a\x059W`\0\x80\xFD[Pa\x02\x92a\x05H6`\x04a\x11\x89V[a\x0B\xD7V[4\x80\x15a\x05YW`\0\x80\xFD[Pa\x02(`\x04T\x81V[```\n\x80Ta\x05r\x90a\x12CV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x05\x9E\x90a\x12CV[\x80\x15a\x05\xEBW\x80`\x1F\x10a\x05\xC0Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x05\xEBV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x05\xCEW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x90P\x90V[`\x003a\x06\x03\x81\x85\x85a\x0C\x15V[`\x01\x91PP[\x92\x91PPV[`\x003a\x06\x1D\x85\x82\x85a\x0C'V[a\x06(\x85\x85\x85a\x0C\xA5V[P`\x01\x94\x93PPPPV[3`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x06\xC4W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`+`$\x82\x01R\x7FAstriaBridgeableERC20: only brid`D\x82\x01Rj\x19\xD9H\x18\xD8[\x88\x1BZ[\x9D`\xAA\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[a\x06\xCE\x82\x82a\r\x04V[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Fg\x98\xA5`y:T\xC3\xBC\xFE\x86\xA9<\xDE\x1Es\x08}\x94L\x0E\xA2\x05D\x13}A!9h\x85\x82`@Qa\x07\t\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPV[\x84`\x04T\x804\x14a\x078W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12}V[`\0a\x07d\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x84a\x12\xCFV[\x11a\x07\x81W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12\xF1V[4`\x06`\0\x82\x82Ta\x07\x93\x91\x90a\x13\x90V[\x90\x91UPa\x07\xA3\x90P3\x88a\r>V[\x863`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x88\x88\x88\x88`@Qa\x07\xE3\x94\x93\x92\x91\x90a\x13\xDAV[`@Q\x80\x91\x03\x90\xA3PPPPPPPV[a\x07\xFCa\rtV[a\x08\x06`\0a\r\xA1V[V[a\x08\x10a\rtV[`\x04UV[```\x0B\x80Ta\x05r\x90a\x12CV[a\x08,a\rtV[`\x03UV[`\x003a\x06\x03\x81\x85\x85a\x0C\xA5V[`\x02\x80Ta\x08L\x90a\x12CV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x08x\x90a\x12CV[\x80\x15a\x08\xC5W\x80`\x1F\x10a\x08\x9AWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x08\xC5V[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x08\xA8W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x81V[4`\x03T\x804\x14a\x08\xF0W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12}V[`\0a\t\x1C\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x84a\x12\xCFV[\x11a\t9W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12\xF1V[4`\x06`\0\x82\x82Ta\tK\x91\x90a\x13\x90V[\x90\x91UPa\t[\x90P3\x88a\r>V[\x863`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE2\xA9\xF2nJR8\xEC\tr\xCE>\xD7'1!.\xBC\x8D'i\xCF\x7F\x17\xD8\xBE\xD2\x8A\x0F\xC8i\xF5\x88\x88\x88\x88`@Qa\x07\xE3\x94\x93\x92\x91\x90a\x13\xDAV[`\x05T`\x01`\x01`\xA0\x1B\x03\x163\x14a\n\x07W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`)`$\x82\x01R\x7FAstriaBridgeableERC20: only fee `D\x82\x01Rh\x1C\x99X\xDA\\\x1AY[\x9D`\xBA\x1B`d\x82\x01R`\x84\x01a\x06\xBBV[`\x05T`\x06T`@Q`\0\x92`\x01`\x01`\xA0\x1B\x03\x16\x91\x90\x83\x81\x81\x81\x85\x87Z\xF1\x92PPP=\x80`\0\x81\x14a\nVW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=`\0` \x84\x01>a\n[V[``\x91P[PP\x90P\x80a\n\xBFW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`*`$\x82\x01R\x7FAstriaBridgeableERC20: fee trans`D\x82\x01Ri\x19\x99\\\x88\x19\x98Z[\x19Y`\xB2\x1B`d\x82\x01R`\x84\x01a\x06\xBBV[P`\0`\x06UV[\x82`\x03T\x804\x14a\n\xEAW`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12}V[`\0a\x0B\x16\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x84a\x12\xCFV[\x11a\x0B3W`@QbF\x1B\xCD`\xE5\x1B\x81R`\x04\x01a\x06\xBB\x90a\x12\xF1V[4`\x06`\0\x82\x82Ta\x0BE\x91\x90a\x13\x90V[\x90\x91UPa\x0BU\x90P3\x86a\r>V[\x843`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0FIa\xCA\xB7S\x08\x04\x89\x84\x99\xAA\x89\xF5\xEC\x81\xD1\xA71\x02\xE2\xE4\xA1\xF3\x0F\x88\xE5\xAE5\x13\xBA*\x86\x86`@Qa\x0B\x91\x92\x91\x90a\x14\x0CV[`@Q\x80\x91\x03\x90\xA3PPPPPV[`\x01\x80Ta\x08L\x90a\x12CV[a\x0B\xB5a\rtV[`\x05\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[a\x0B\xDFa\rtV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0C\tW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\x0C\x12\x81a\r\xA1V[PV[a\x0C\"\x83\x83\x83`\x01a\r\xF1V[PPPV[`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\0\x90\x81R`\x08` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R T`\0\x19\x81\x14a\x0C\x9FW\x81\x81\x10\x15a\x0C\x90W`@Qc}\xC7\xA0\xD9`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x06\xBBV[a\x0C\x9F\x84\x84\x84\x84\x03`\0a\r\xF1V[PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x0C\xCFW`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x0C\xF9W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\x0C\"\x83\x83\x83a\x0E\xC6V[`\x01`\x01`\xA0\x1B\x03\x82\x16a\r.W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\r:`\0\x83\x83a\x0E\xC6V[PPV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\rhW`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[a\r:\x82`\0\x83a\x0E\xC6V[`\0T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\x06W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x06\xBBV[`\0\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[`\x01`\x01`\xA0\x1B\x03\x84\x16a\x0E\x1BW`@Qc\xE6\x02\xDF\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x0EEW`@QcJ\x14\x06\xB1`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16`\0\x90\x81R`\x08` \x90\x81R`@\x80\x83 \x93\x87\x16\x83R\x92\x90R \x82\x90U\x80\x15a\x0C\x9FW\x82`\x01`\x01`\xA0\x1B\x03\x16\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\x8C[\xE1\xE5\xEB\xEC}[\xD1OqB}\x1E\x84\xF3\xDD\x03\x14\xC0\xF7\xB2)\x1E[ \n\xC8\xC7\xC3\xB9%\x84`@Qa\x0E\xB8\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x0E\xF1W\x80`\t`\0\x82\x82Ta\x0E\xE6\x91\x90a\x13\x90V[\x90\x91UPa\x0Fc\x90PV[`\x01`\x01`\xA0\x1B\x03\x83\x16`\0\x90\x81R`\x07` R`@\x90 T\x81\x81\x10\x15a\x0FDW`@Qc9\x144\xE3`\xE2\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x06\xBBV[`\x01`\x01`\xA0\x1B\x03\x84\x16`\0\x90\x81R`\x07` R`@\x90 \x90\x82\x90\x03\x90U[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x0F\x7FW`\t\x80T\x82\x90\x03\x90Ua\x0F\x9EV[`\x01`\x01`\xA0\x1B\x03\x82\x16`\0\x90\x81R`\x07` R`@\x90 \x80T\x82\x01\x90U[\x81`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDD\xF2R\xAD\x1B\xE2\xC8\x9Bi\xC2\xB0h\xFC7\x8D\xAA\x95+\xA7\xF1c\xC4\xA1\x16(\xF5ZM\xF5#\xB3\xEF\x83`@Qa\x0F\xE3\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPV[` \x81R`\0\x82Q\x80` \x84\x01R`\0[\x81\x81\x10\x15a\x10\x1EW` \x81\x86\x01\x81\x01Q`@\x86\x84\x01\x01R\x01a\x10\x01V[P`\0`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x10UW`\0\x80\xFD[\x91\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\x10mW`\0\x80\xFD[a\x10v\x83a\x10>V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\x10\x99W`\0\x80\xFD[a\x10\xA2\x84a\x10>V[\x92Pa\x10\xB0` \x85\x01a\x10>V[\x92\x95\x92\x94PPP`@\x91\x90\x91\x015\x90V[`\0\x80\x83`\x1F\x84\x01\x12a\x10\xD3W`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x10\xEBW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\x11\x03W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`\0``\x86\x88\x03\x12\x15a\x11\"W`\0\x80\xFD[\x855\x94P` \x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x11@W`\0\x80\xFD[a\x11L\x88\x82\x89\x01a\x10\xC1V[\x90\x95P\x93PP`@\x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x11lW`\0\x80\xFD[a\x11x\x88\x82\x89\x01a\x10\xC1V[\x96\x99\x95\x98P\x93\x96P\x92\x94\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\x11\x9BW`\0\x80\xFD[a\x11\xA4\x82a\x10>V[\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\x11\xBDW`\0\x80\xFD[P5\x91\x90PV[`\0\x80`\0`@\x84\x86\x03\x12\x15a\x11\xD9W`\0\x80\xFD[\x835\x92P` \x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x11\xF7W`\0\x80\xFD[a\x12\x03\x86\x82\x87\x01a\x10\xC1V[\x94\x97\x90\x96P\x93\x94PPPPV[`\0\x80`@\x83\x85\x03\x12\x15a\x12#W`\0\x80\xFD[a\x12,\x83a\x10>V[\x91Pa\x12:` \x84\x01a\x10>V[\x90P\x92P\x92\x90PV[`\x01\x81\x81\x1C\x90\x82\x16\x80a\x12WW`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\x12wWcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[` \x80\x82R`2\x90\x82\x01R\x7FAstriaBridgeableERC20: insuffici`@\x82\x01Rqent withdrawal fee`p\x1B``\x82\x01R`\x80\x01\x90V[`\0\x82a\x12\xECWcNH{q`\xE0\x1B`\0R`\x12`\x04R`$`\0\xFD[P\x04\x90V[` \x80\x82R`s\x90\x82\x01R\x7FAstriaBridgeableERC20: insuffici`@\x82\x01R\x7Fent value, must be greater than ``\x82\x01R\x7F10 ** (TOKEN_DECIMALS - BASE_CHA`\x80\x82\x01RrIN_ASSET_PRECISION)`h\x1B`\xA0\x82\x01R`\xC0\x01\x90V[\x80\x82\x01\x80\x82\x11\x15a\x06\tWcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\x13\xEE`@\x83\x01\x86\x88a\x13\xB1V[\x82\x81\x03` \x84\x01Ra\x14\x01\x81\x85\x87a\x13\xB1V[\x97\x96PPPPPPPV[` \x81R`\0a\x14 ` \x83\x01\x84\x86a\x13\xB1V[\x94\x93PPPPV\xFE\xA2dipfsX\"\x12 .^\xF6\x0C\x90\xC3\x9Br\xCC\xF5~\xBC\x92\x82\xD8\x8Eh\xF5.~\xD4\xC2\xA1\xE4\xE2\xF2\xFE\xEF@u\x97\x0EdsolcC\0\x08\x1A\x003";
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
        ///Calls the contract's `claimFees` (0xd294f093) function
        pub fn claim_fees(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([210, 148, 240, 147], ())
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
        ///Calls the contract's `transferOwnership` (0xf2fde38b) function
        pub fn transfer_ownership(
            &self,
            new_owner: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([242, 253, 227, 139], new_owner)
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
        ///Calls the contract's `withdrawToRollup` (0xcbee8cfa) function
        pub fn withdraw_to_rollup(
            &self,
            amount: ::ethers::core::types::U256,
            destination_chain_address: ::std::string::String,
            destination_rollup_bridge_address: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [203, 238, 140, 250],
                    (
                        amount,
                        destination_chain_address,
                        destination_rollup_bridge_address,
                    ),
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
    pub enum AstriaBridgeableERC20Errors {
        ERC20InsufficientAllowance(ERC20InsufficientAllowance),
        ERC20InsufficientBalance(ERC20InsufficientBalance),
        ERC20InvalidApprover(ERC20InvalidApprover),
        ERC20InvalidReceiver(ERC20InvalidReceiver),
        ERC20InvalidSender(ERC20InvalidSender),
        ERC20InvalidSpender(ERC20InvalidSpender),
        OwnableInvalidOwner(OwnableInvalidOwner),
        OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
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
    impl ::core::convert::From<OwnableInvalidOwner> for AstriaBridgeableERC20Errors {
        fn from(value: OwnableInvalidOwner) -> Self {
            Self::OwnableInvalidOwner(value)
        }
    }
    impl ::core::convert::From<OwnableUnauthorizedAccount>
    for AstriaBridgeableERC20Errors {
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
        OwnershipTransferredFilter(OwnershipTransferredFilter),
        RollupWithdrawalFilter(RollupWithdrawalFilter),
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
            if let Ok(decoded) = OwnershipTransferredFilter::decode_log(log) {
                return Ok(
                    AstriaBridgeableERC20Events::OwnershipTransferredFilter(decoded),
                );
            }
            if let Ok(decoded) = RollupWithdrawalFilter::decode_log(log) {
                return Ok(AstriaBridgeableERC20Events::RollupWithdrawalFilter(decoded));
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
                Self::OwnershipTransferredFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::RollupWithdrawalFilter(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
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
    impl ::core::convert::From<OwnershipTransferredFilter>
    for AstriaBridgeableERC20Events {
        fn from(value: OwnershipTransferredFilter) -> Self {
            Self::OwnershipTransferredFilter(value)
        }
    }
    impl ::core::convert::From<RollupWithdrawalFilter> for AstriaBridgeableERC20Events {
        fn from(value: RollupWithdrawalFilter) -> Self {
            Self::RollupWithdrawalFilter(value)
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
    ///Container type for all input parameters for the `withdrawToRollup` function with signature `withdrawToRollup(uint256,string,string)` and selector `0xcbee8cfa`
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
        name = "withdrawToRollup",
        abi = "withdrawToRollup(uint256,string,string)"
    )]
    pub struct WithdrawToRollupCall {
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
        pub destination_rollup_bridge_address: ::std::string::String,
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
        AccumulatedFees(AccumulatedFeesCall),
        BaseChainAssetDenomination(BaseChainAssetDenominationCall),
        BaseChainAssetPrecision(BaseChainAssetPrecisionCall),
        BaseChainBridgeAddress(BaseChainBridgeAddressCall),
        Bridge(BridgeCall),
        FeeRecipient(FeeRecipientCall),
        IbcWithdrawalFee(IbcWithdrawalFeeCall),
        SequencerWithdrawalFee(SequencerWithdrawalFeeCall),
        Allowance(AllowanceCall),
        Approve(ApproveCall),
        BalanceOf(BalanceOfCall),
        ClaimFees(ClaimFeesCall),
        Decimals(DecimalsCall),
        Mint(MintCall),
        Name(NameCall),
        Owner(OwnerCall),
        RenounceOwnership(RenounceOwnershipCall),
        SetFeeRecipient(SetFeeRecipientCall),
        SetIbcWithdrawalFee(SetIbcWithdrawalFeeCall),
        SetSequencerWithdrawalFee(SetSequencerWithdrawalFeeCall),
        Symbol(SymbolCall),
        TotalSupply(TotalSupplyCall),
        Transfer(TransferCall),
        TransferFrom(TransferFromCall),
        TransferOwnership(TransferOwnershipCall),
        WithdrawToIbcChain(WithdrawToIbcChainCall),
        WithdrawToRollup(WithdrawToRollupCall),
        WithdrawToSequencer(WithdrawToSequencerCall),
    }
    impl ::ethers::core::abi::AbiDecode for AstriaBridgeableERC20Calls {
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
            if let Ok(decoded) = <BridgeCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::Bridge(decoded));
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
            if let Ok(decoded) = <ClaimFeesCall as ::ethers::core::abi::AbiDecode>::decode(
                data,
            ) {
                return Ok(Self::ClaimFees(decoded));
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
    impl ::ethers::core::abi::AbiEncode for AstriaBridgeableERC20Calls {
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
                Self::Bridge(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::FeeRecipient(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::IbcWithdrawalFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::SequencerWithdrawalFee(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Allowance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Approve(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::BalanceOf(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ClaimFees(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Decimals(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Mint(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Name(element) => ::ethers::core::abi::AbiEncode::encode(element),
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
    impl ::core::fmt::Display for AstriaBridgeableERC20Calls {
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
                Self::Bridge(element) => ::core::fmt::Display::fmt(element, f),
                Self::FeeRecipient(element) => ::core::fmt::Display::fmt(element, f),
                Self::IbcWithdrawalFee(element) => ::core::fmt::Display::fmt(element, f),
                Self::SequencerWithdrawalFee(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::Allowance(element) => ::core::fmt::Display::fmt(element, f),
                Self::Approve(element) => ::core::fmt::Display::fmt(element, f),
                Self::BalanceOf(element) => ::core::fmt::Display::fmt(element, f),
                Self::ClaimFees(element) => ::core::fmt::Display::fmt(element, f),
                Self::Decimals(element) => ::core::fmt::Display::fmt(element, f),
                Self::Mint(element) => ::core::fmt::Display::fmt(element, f),
                Self::Name(element) => ::core::fmt::Display::fmt(element, f),
                Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
                Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetFeeRecipient(element) => ::core::fmt::Display::fmt(element, f),
                Self::SetIbcWithdrawalFee(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::SetSequencerWithdrawalFee(element) => {
                    ::core::fmt::Display::fmt(element, f)
                }
                Self::Symbol(element) => ::core::fmt::Display::fmt(element, f),
                Self::TotalSupply(element) => ::core::fmt::Display::fmt(element, f),
                Self::Transfer(element) => ::core::fmt::Display::fmt(element, f),
                Self::TransferFrom(element) => ::core::fmt::Display::fmt(element, f),
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
    impl ::core::convert::From<AccumulatedFeesCall> for AstriaBridgeableERC20Calls {
        fn from(value: AccumulatedFeesCall) -> Self {
            Self::AccumulatedFees(value)
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
    impl ::core::convert::From<FeeRecipientCall> for AstriaBridgeableERC20Calls {
        fn from(value: FeeRecipientCall) -> Self {
            Self::FeeRecipient(value)
        }
    }
    impl ::core::convert::From<IbcWithdrawalFeeCall> for AstriaBridgeableERC20Calls {
        fn from(value: IbcWithdrawalFeeCall) -> Self {
            Self::IbcWithdrawalFee(value)
        }
    }
    impl ::core::convert::From<SequencerWithdrawalFeeCall>
    for AstriaBridgeableERC20Calls {
        fn from(value: SequencerWithdrawalFeeCall) -> Self {
            Self::SequencerWithdrawalFee(value)
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
    impl ::core::convert::From<ClaimFeesCall> for AstriaBridgeableERC20Calls {
        fn from(value: ClaimFeesCall) -> Self {
            Self::ClaimFees(value)
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
    impl ::core::convert::From<OwnerCall> for AstriaBridgeableERC20Calls {
        fn from(value: OwnerCall) -> Self {
            Self::Owner(value)
        }
    }
    impl ::core::convert::From<RenounceOwnershipCall> for AstriaBridgeableERC20Calls {
        fn from(value: RenounceOwnershipCall) -> Self {
            Self::RenounceOwnership(value)
        }
    }
    impl ::core::convert::From<SetFeeRecipientCall> for AstriaBridgeableERC20Calls {
        fn from(value: SetFeeRecipientCall) -> Self {
            Self::SetFeeRecipient(value)
        }
    }
    impl ::core::convert::From<SetIbcWithdrawalFeeCall> for AstriaBridgeableERC20Calls {
        fn from(value: SetIbcWithdrawalFeeCall) -> Self {
            Self::SetIbcWithdrawalFee(value)
        }
    }
    impl ::core::convert::From<SetSequencerWithdrawalFeeCall>
    for AstriaBridgeableERC20Calls {
        fn from(value: SetSequencerWithdrawalFeeCall) -> Self {
            Self::SetSequencerWithdrawalFee(value)
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
    impl ::core::convert::From<TransferOwnershipCall> for AstriaBridgeableERC20Calls {
        fn from(value: TransferOwnershipCall) -> Self {
            Self::TransferOwnership(value)
        }
    }
    impl ::core::convert::From<WithdrawToIbcChainCall> for AstriaBridgeableERC20Calls {
        fn from(value: WithdrawToIbcChainCall) -> Self {
            Self::WithdrawToIbcChain(value)
        }
    }
    impl ::core::convert::From<WithdrawToRollupCall> for AstriaBridgeableERC20Calls {
        fn from(value: WithdrawToRollupCall) -> Self {
            Self::WithdrawToRollup(value)
        }
    }
    impl ::core::convert::From<WithdrawToSequencerCall> for AstriaBridgeableERC20Calls {
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
