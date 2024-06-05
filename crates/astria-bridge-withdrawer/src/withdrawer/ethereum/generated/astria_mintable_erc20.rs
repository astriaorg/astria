#![allow(warnings)]
#![allow(clippy::all)]
pub use astria_mintable_erc20::*;
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
pub mod astria_mintable_erc20 {
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
                        name: ::std::borrow::ToOwned::to_owned("_assetWithdrawalDecimals",),
                        kind: ::ethers::core::abi::ethabi::ParamType::Uint(32usize),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("uint32"),
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
                    ::std::borrow::ToOwned::to_owned("ASSET_WITHDRAWAL_DECIMALS"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("ASSET_WITHDRAWAL_DECIMALS",),
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
                    ::std::borrow::ToOwned::to_owned("BRIDGE"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("BRIDGE"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("allowance"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
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
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("approve"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("bool"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("balanceOf"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("balanceOf"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("account"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("decimals"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("decimals"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint8"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("mint"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("name"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("name"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::String,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("string"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("symbol"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("symbol"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::String,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("string"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("totalSupply"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("totalSupply"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("transfer"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("bool"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("transferFrom"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("bool"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawToIbcChain"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("withdrawToIbcChain"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("_amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("_destinationChainAddress",),
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
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("withdrawToSequencer"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("withdrawToSequencer",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("_amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("_destinationChainAddress",),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("address"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("Approval"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
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
                    ::std::borrow::ToOwned::to_owned("Mint"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("Mint"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("account"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: true,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
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
                (
                    ::std::borrow::ToOwned::to_owned("Transfer"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InsufficientAllowance"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ERC20InsufficientAllowance",),
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("needed"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InsufficientBalance"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ERC20InsufficientBalance",),
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
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("needed"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidApprover"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ERC20InvalidApprover",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("approver"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidReceiver"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ERC20InvalidReceiver",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("receiver"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidSender"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ERC20InvalidSender"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("sender"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ERC20InvalidSpender"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ERC20InvalidSpender",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("spender"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                    },],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    /// The parsed JSON ABI of the contract.
    pub static ASTRIAMINTABLEERC20_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\xC0`@R4\x80\x15b\0\0\x11W`\0\x80\xFD[P`@Qb\0\x0E\xCA8\x03\x80b\0\x0E\xCA\x839\x81\x01`@\x81\x90Rb\0\x004\x91b\0\x01:V[\x81\x81`\x03b\0\0D\x83\x82b\0\x02qV[P`\x04b\0\0S\x82\x82b\0\x02qV[PPP`\x01`\x01`\xA0\x1B\x03\x90\x93\x16`\xA0RPc\xFF\xFF\xFF\xFF\x16`\x80RPb\0\x03=V[cNH{q`\xE0\x1B`\0R`A`\x04R`$`\0\xFD[`\0\x82`\x1F\x83\x01\x12b\0\0\x9DW`\0\x80\xFD[\x81Q`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\0\xBAWb\0\0\xBAb\0\0uV[`@Q`\x1F\x83\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01\x90\x82\x82\x11\x81\x83\x10\x17\x15b\0\0\xE5Wb\0\0\xE5b\0\0uV[\x81`@R\x83\x81R` \x92P\x86\x83\x85\x88\x01\x01\x11\x15b\0\x01\x02W`\0\x80\xFD[`\0\x91P[\x83\x82\x10\x15b\0\x01&W\x85\x82\x01\x83\x01Q\x81\x83\x01\x84\x01R\x90\x82\x01\x90b\0\x01\x07V[`\0\x93\x81\x01\x90\x92\x01\x92\x90\x92R\x94\x93PPPPV[`\0\x80`\0\x80`\x80\x85\x87\x03\x12\x15b\0\x01QW`\0\x80\xFD[\x84Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14b\0\x01iW`\0\x80\xFD[` \x86\x01Q\x90\x94Pc\xFF\xFF\xFF\xFF\x81\x16\x81\x14b\0\x01\x84W`\0\x80\xFD[`@\x86\x01Q\x90\x93P`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x01\xA2W`\0\x80\xFD[b\0\x01\xB0\x88\x83\x89\x01b\0\0\x8BV[\x93P``\x87\x01Q\x91P\x80\x82\x11\x15b\0\x01\xC7W`\0\x80\xFD[Pb\0\x01\xD6\x87\x82\x88\x01b\0\0\x8BV[\x91PP\x92\x95\x91\x94P\x92PV[`\x01\x81\x81\x1C\x90\x82\x16\x80b\0\x01\xF7W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03b\0\x02\x18WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[`\x1F\x82\x11\x15b\0\x02lW`\0\x81\x81R` \x81 `\x1F\x85\x01`\x05\x1C\x81\x01` \x86\x10\x15b\0\x02GWP\x80[`\x1F\x85\x01`\x05\x1C\x82\x01\x91P[\x81\x81\x10\x15b\0\x02hW\x82\x81U`\x01\x01b\0\x02SV[PPP[PPPV[\x81Q`\x01`\x01`@\x1B\x03\x81\x11\x15b\0\x02\x8DWb\0\x02\x8Db\0\0uV[b\0\x02\xA5\x81b\0\x02\x9E\x84Tb\0\x01\xE2V[\x84b\0\x02\x1EV[` \x80`\x1F\x83\x11`\x01\x81\x14b\0\x02\xDDW`\0\x84\x15b\0\x02\xC4WP\x85\x83\x01Q[`\0\x19`\x03\x86\x90\x1B\x1C\x19\x16`\x01\x85\x90\x1B\x17\x85Ub\0\x02hV[`\0\x85\x81R` \x81 `\x1F\x19\x86\x16\x91[\x82\x81\x10\x15b\0\x03\x0EW\x88\x86\x01Q\x82U\x94\x84\x01\x94`\x01\x90\x91\x01\x90\x84\x01b\0\x02\xEDV[P\x85\x82\x10\x15b\0\x03-W\x87\x85\x01Q`\0\x19`\x03\x88\x90\x1B`\xF8\x16\x1C\x19\x16\x81U[PPPPP`\x01\x90\x81\x1B\x01\x90UPV[`\x80Q`\xA0Qa\x0B`b\0\x03j`\09`\0\x81\x81a\x02]\x01Ra\x03r\x01R`\0a\x01\xCD\x01Ra\x0B``\0\xF3\xFE`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\xEAW`\x005`\xE0\x1C\x80cp\xA0\x821\x11a\0\x8CW\x80c\x95\xD8\x9BA\x11a\0fW\x80c\x95\xD8\x9BA\x14a\x02\x04W\x80c\xA9\x05\x9C\xBB\x14a\x02\x0CW\x80c\xDDb\xED>\x14a\x02\x1FW\x80c\xEE\x9A1\xA2\x14a\x02XW`\0\x80\xFD[\x80cp\xA0\x821\x14a\x01\x8CW\x80cu~\x98t\x14a\x01\xB5W\x80c\x8F-\x8C\xB8\x14a\x01\xC8W`\0\x80\xFD[\x80c#\xB8r\xDD\x11a\0\xC8W\x80c#\xB8r\xDD\x14a\x01BW\x80c1<\xE5g\x14a\x01UW\x80c@\xC1\x0F\x19\x14a\x01dW\x80c_\xE5k\t\x14a\x01yW`\0\x80\xFD[\x80c\x06\xFD\xDE\x03\x14a\0\xEFW\x80c\t^\xA7\xB3\x14a\x01\rW\x80c\x18\x16\r\xDD\x14a\x010W[`\0\x80\xFD[a\0\xF7a\x02\x97V[`@Qa\x01\x04\x91\x90a\x08iV[`@Q\x80\x91\x03\x90\xF3[a\x01 a\x01\x1B6`\x04a\x08\xD3V[a\x03)V[`@Q\x90\x15\x15\x81R` \x01a\x01\x04V[`\x02T[`@Q\x90\x81R` \x01a\x01\x04V[a\x01 a\x01P6`\x04a\x08\xFDV[a\x03CV[`@Q`\x12\x81R` \x01a\x01\x04V[a\x01wa\x01r6`\x04a\x08\xD3V[a\x03gV[\0[a\x01wa\x01\x876`\x04a\t\x82V[a\x04GV[a\x014a\x01\x9A6`\x04a\t\xFCV[`\x01`\x01`\xA0\x1B\x03\x16`\0\x90\x81R` \x81\x90R`@\x90 T\x90V[a\x01wa\x01\xC36`\x04a\n\x1EV[a\x04\xA0V[a\x01\xEF\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\x04V[a\0\xF7a\x04\xEEV[a\x01 a\x02\x1A6`\x04a\x08\xD3V[a\x04\xFDV[a\x014a\x02-6`\x04a\nJV[`\x01`\x01`\xA0\x1B\x03\x91\x82\x16`\0\x90\x81R`\x01` \x90\x81R`@\x80\x83 \x93\x90\x94\x16\x82R\x91\x90\x91R T\x90V[a\x02\x7F\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x04V[```\x03\x80Ta\x02\xA6\x90a\ntV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\xD2\x90a\ntV[\x80\x15a\x03\x1FW\x80`\x1F\x10a\x02\xF4Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x03\x1FV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x03\x02W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x90P\x90V[`\x003a\x037\x81\x85\x85a\x05\x0BV[`\x01\x91PP[\x92\x91PPV[`\x003a\x03Q\x85\x82\x85a\x05\x1DV[a\x03\\\x85\x85\x85a\x05\x9BV[P`\x01\x94\x93PPPPV[3`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x03\xF6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`)`$\x82\x01R\x7FAstriaMintableERC20: only bridge`D\x82\x01Rh\x08\x18\xD8[\x88\x1BZ[\x9D`\xBA\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[a\x04\0\x82\x82a\x05\xFAV[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Fg\x98\xA5`y:T\xC3\xBC\xFE\x86\xA9<\xDE\x1Es\x08}\x94L\x0E\xA2\x05D\x13}A!9h\x85\x82`@Qa\x04;\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPV[a\x04Q3\x86a\x064V[\x843`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x86\x86\x86\x86`@Qa\x04\x91\x94\x93\x92\x91\x90a\n\xD7V[`@Q\x80\x91\x03\x90\xA3PPPPPV[a\x04\xAA3\x83a\x064V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x82\x903\x90\x7F\xAE\x8EffM\x10\x85DP\x9C\x9A[j\x9F3\xC3\xB5\xFE\xF3\xF8\x8E]?\xA6\x80pjo\xEB\x13`\xE3\x90` \x01`@Q\x80\x91\x03\x90\xA3PPV[```\x04\x80Ta\x02\xA6\x90a\ntV[`\x003a\x037\x81\x85\x85a\x05\x9BV[a\x05\x18\x83\x83\x83`\x01a\x06jV[PPPV[`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\0\x90\x81R`\x01` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R T`\0\x19\x81\x14a\x05\x95W\x81\x81\x10\x15a\x05\x86W`@Qc}\xC7\xA0\xD9`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x03\xEDV[a\x05\x95\x84\x84\x84\x84\x03`\0a\x06jV[PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x05\xC5W`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x05\xEFW`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[a\x05\x18\x83\x83\x83a\x07?V[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x06$W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[a\x060`\0\x83\x83a\x07?V[PPV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x06^W`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[a\x060\x82`\0\x83a\x07?V[`\x01`\x01`\xA0\x1B\x03\x84\x16a\x06\x94W`@Qc\xE6\x02\xDF\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x06\xBEW`@QcJ\x14\x06\xB1`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16`\0\x90\x81R`\x01` \x90\x81R`@\x80\x83 \x93\x87\x16\x83R\x92\x90R \x82\x90U\x80\x15a\x05\x95W\x82`\x01`\x01`\xA0\x1B\x03\x16\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\x8C[\xE1\xE5\xEB\xEC}[\xD1OqB}\x1E\x84\xF3\xDD\x03\x14\xC0\xF7\xB2)\x1E[ \n\xC8\xC7\xC3\xB9%\x84`@Qa\x071\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x07jW\x80`\x02`\0\x82\x82Ta\x07_\x91\x90a\x0B\tV[\x90\x91UPa\x07\xDC\x90PV[`\x01`\x01`\xA0\x1B\x03\x83\x16`\0\x90\x81R` \x81\x90R`@\x90 T\x81\x81\x10\x15a\x07\xBDW`@Qc9\x144\xE3`\xE2\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x84\x16`\0\x90\x81R` \x81\x90R`@\x90 \x90\x82\x90\x03\x90U[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07\xF8W`\x02\x80T\x82\x90\x03\x90Ua\x08\x17V[`\x01`\x01`\xA0\x1B\x03\x82\x16`\0\x90\x81R` \x81\x90R`@\x90 \x80T\x82\x01\x90U[\x81`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDD\xF2R\xAD\x1B\xE2\xC8\x9Bi\xC2\xB0h\xFC7\x8D\xAA\x95+\xA7\xF1c\xC4\xA1\x16(\xF5ZM\xF5#\xB3\xEF\x83`@Qa\x08\\\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPV[`\0` \x80\x83R\x83Q\x80\x82\x85\x01R`\0[\x81\x81\x10\x15a\x08\x96W\x85\x81\x01\x83\x01Q\x85\x82\x01`@\x01R\x82\x01a\x08zV[P`\0`@\x82\x86\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x92PPP\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xCEW`\0\x80\xFD[\x91\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\x08\xE6W`\0\x80\xFD[a\x08\xEF\x83a\x08\xB7V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\t\x12W`\0\x80\xFD[a\t\x1B\x84a\x08\xB7V[\x92Pa\t)` \x85\x01a\x08\xB7V[\x91P`@\x84\x015\x90P\x92P\x92P\x92V[`\0\x80\x83`\x1F\x84\x01\x12a\tKW`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\tcW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\t{W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`\0``\x86\x88\x03\x12\x15a\t\x9AW`\0\x80\xFD[\x855\x94P` \x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\t\xB9W`\0\x80\xFD[a\t\xC5\x89\x83\x8A\x01a\t9V[\x90\x96P\x94P`@\x88\x015\x91P\x80\x82\x11\x15a\t\xDEW`\0\x80\xFD[Pa\t\xEB\x88\x82\x89\x01a\t9V[\x96\x99\x95\x98P\x93\x96P\x92\x94\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\n\x0EW`\0\x80\xFD[a\n\x17\x82a\x08\xB7V[\x93\x92PPPV[`\0\x80`@\x83\x85\x03\x12\x15a\n1W`\0\x80\xFD[\x825\x91Pa\nA` \x84\x01a\x08\xB7V[\x90P\x92P\x92\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\n]W`\0\x80\xFD[a\nf\x83a\x08\xB7V[\x91Pa\nA` \x84\x01a\x08\xB7V[`\x01\x81\x81\x1C\x90\x82\x16\x80a\n\x88W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\n\xA8WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\n\xEB`@\x83\x01\x86\x88a\n\xAEV[\x82\x81\x03` \x84\x01Ra\n\xFE\x81\x85\x87a\n\xAEV[\x97\x96PPPPPPPV[\x80\x82\x01\x80\x82\x11\x15a\x03=WcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD\xFE\xA2dipfsX\"\x12 \xC8\xBB\x8E\x84\xF3\x1D,\xD3\x84-h\x05E\x8F\xAFa\x94\x88\xF9,\xC2\xDA\xC5\xA0\xB2\x9B\xC0dlQ\xFC?dsolcC\0\x08\x15\x003";
    /// The bytecode of the contract.
    pub static ASTRIAMINTABLEERC20_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x10W`\0\x80\xFD[P`\x046\x10a\0\xEAW`\x005`\xE0\x1C\x80cp\xA0\x821\x11a\0\x8CW\x80c\x95\xD8\x9BA\x11a\0fW\x80c\x95\xD8\x9BA\x14a\x02\x04W\x80c\xA9\x05\x9C\xBB\x14a\x02\x0CW\x80c\xDDb\xED>\x14a\x02\x1FW\x80c\xEE\x9A1\xA2\x14a\x02XW`\0\x80\xFD[\x80cp\xA0\x821\x14a\x01\x8CW\x80cu~\x98t\x14a\x01\xB5W\x80c\x8F-\x8C\xB8\x14a\x01\xC8W`\0\x80\xFD[\x80c#\xB8r\xDD\x11a\0\xC8W\x80c#\xB8r\xDD\x14a\x01BW\x80c1<\xE5g\x14a\x01UW\x80c@\xC1\x0F\x19\x14a\x01dW\x80c_\xE5k\t\x14a\x01yW`\0\x80\xFD[\x80c\x06\xFD\xDE\x03\x14a\0\xEFW\x80c\t^\xA7\xB3\x14a\x01\rW\x80c\x18\x16\r\xDD\x14a\x010W[`\0\x80\xFD[a\0\xF7a\x02\x97V[`@Qa\x01\x04\x91\x90a\x08iV[`@Q\x80\x91\x03\x90\xF3[a\x01 a\x01\x1B6`\x04a\x08\xD3V[a\x03)V[`@Q\x90\x15\x15\x81R` \x01a\x01\x04V[`\x02T[`@Q\x90\x81R` \x01a\x01\x04V[a\x01 a\x01P6`\x04a\x08\xFDV[a\x03CV[`@Q`\x12\x81R` \x01a\x01\x04V[a\x01wa\x01r6`\x04a\x08\xD3V[a\x03gV[\0[a\x01wa\x01\x876`\x04a\t\x82V[a\x04GV[a\x014a\x01\x9A6`\x04a\t\xFCV[`\x01`\x01`\xA0\x1B\x03\x16`\0\x90\x81R` \x81\x90R`@\x90 T\x90V[a\x01wa\x01\xC36`\x04a\n\x1EV[a\x04\xA0V[a\x01\xEF\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\x04V[a\0\xF7a\x04\xEEV[a\x01 a\x02\x1A6`\x04a\x08\xD3V[a\x04\xFDV[a\x014a\x02-6`\x04a\nJV[`\x01`\x01`\xA0\x1B\x03\x91\x82\x16`\0\x90\x81R`\x01` \x90\x81R`@\x80\x83 \x93\x90\x94\x16\x82R\x91\x90\x91R T\x90V[a\x02\x7F\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x04V[```\x03\x80Ta\x02\xA6\x90a\ntV[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x02\xD2\x90a\ntV[\x80\x15a\x03\x1FW\x80`\x1F\x10a\x02\xF4Wa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x03\x1FV[\x82\x01\x91\x90`\0R` `\0 \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x03\x02W\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x90P\x90V[`\x003a\x037\x81\x85\x85a\x05\x0BV[`\x01\x91PP[\x92\x91PPV[`\x003a\x03Q\x85\x82\x85a\x05\x1DV[a\x03\\\x85\x85\x85a\x05\x9BV[P`\x01\x94\x93PPPPV[3`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x03\xF6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`)`$\x82\x01R\x7FAstriaMintableERC20: only bridge`D\x82\x01Rh\x08\x18\xD8[\x88\x1BZ[\x9D`\xBA\x1B`d\x82\x01R`\x84\x01[`@Q\x80\x91\x03\x90\xFD[a\x04\0\x82\x82a\x05\xFAV[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Fg\x98\xA5`y:T\xC3\xBC\xFE\x86\xA9<\xDE\x1Es\x08}\x94L\x0E\xA2\x05D\x13}A!9h\x85\x82`@Qa\x04;\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPV[a\x04Q3\x86a\x064V[\x843`\x01`\x01`\xA0\x1B\x03\x16\x7F\x0Cd\xE2\x9ART\xA7\x1C\x7FNR\xB3\xD2\xD264\x8C\x80\xE0\n\0\xBA.\x19a\x96+\xD2\x82|\x03\xFB\x86\x86\x86\x86`@Qa\x04\x91\x94\x93\x92\x91\x90a\n\xD7V[`@Q\x80\x91\x03\x90\xA3PPPPPV[a\x04\xAA3\x83a\x064V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x82\x903\x90\x7F\xAE\x8EffM\x10\x85DP\x9C\x9A[j\x9F3\xC3\xB5\xFE\xF3\xF8\x8E]?\xA6\x80pjo\xEB\x13`\xE3\x90` \x01`@Q\x80\x91\x03\x90\xA3PPV[```\x04\x80Ta\x02\xA6\x90a\ntV[`\x003a\x037\x81\x85\x85a\x05\x9BV[a\x05\x18\x83\x83\x83`\x01a\x06jV[PPPV[`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\0\x90\x81R`\x01` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R T`\0\x19\x81\x14a\x05\x95W\x81\x81\x10\x15a\x05\x86W`@Qc}\xC7\xA0\xD9`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x03\xEDV[a\x05\x95\x84\x84\x84\x84\x03`\0a\x06jV[PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x05\xC5W`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x05\xEFW`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[a\x05\x18\x83\x83\x83a\x07?V[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x06$W`@Qc\xECD/\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[a\x060`\0\x83\x83a\x07?V[PPV[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x06^W`@QcKc~\x8F`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[a\x060\x82`\0\x83a\x07?V[`\x01`\x01`\xA0\x1B\x03\x84\x16a\x06\x94W`@Qc\xE6\x02\xDF\x05`\xE0\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x06\xBEW`@QcJ\x14\x06\xB1`\xE1\x1B\x81R`\0`\x04\x82\x01R`$\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16`\0\x90\x81R`\x01` \x90\x81R`@\x80\x83 \x93\x87\x16\x83R\x92\x90R \x82\x90U\x80\x15a\x05\x95W\x82`\x01`\x01`\xA0\x1B\x03\x16\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\x8C[\xE1\xE5\xEB\xEC}[\xD1OqB}\x1E\x84\xF3\xDD\x03\x14\xC0\xF7\xB2)\x1E[ \n\xC8\xC7\xC3\xB9%\x84`@Qa\x071\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x07jW\x80`\x02`\0\x82\x82Ta\x07_\x91\x90a\x0B\tV[\x90\x91UPa\x07\xDC\x90PV[`\x01`\x01`\xA0\x1B\x03\x83\x16`\0\x90\x81R` \x81\x90R`@\x90 T\x81\x81\x10\x15a\x07\xBDW`@Qc9\x144\xE3`\xE2\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x81\x01\x83\x90R`d\x01a\x03\xEDV[`\x01`\x01`\xA0\x1B\x03\x84\x16`\0\x90\x81R` \x81\x90R`@\x90 \x90\x82\x90\x03\x90U[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x07\xF8W`\x02\x80T\x82\x90\x03\x90Ua\x08\x17V[`\x01`\x01`\xA0\x1B\x03\x82\x16`\0\x90\x81R` \x81\x90R`@\x90 \x80T\x82\x01\x90U[\x81`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDD\xF2R\xAD\x1B\xE2\xC8\x9Bi\xC2\xB0h\xFC7\x8D\xAA\x95+\xA7\xF1c\xC4\xA1\x16(\xF5ZM\xF5#\xB3\xEF\x83`@Qa\x08\\\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPV[`\0` \x80\x83R\x83Q\x80\x82\x85\x01R`\0[\x81\x81\x10\x15a\x08\x96W\x85\x81\x01\x83\x01Q\x85\x82\x01`@\x01R\x82\x01a\x08zV[P`\0`@\x82\x86\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x92PPP\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x08\xCEW`\0\x80\xFD[\x91\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\x08\xE6W`\0\x80\xFD[a\x08\xEF\x83a\x08\xB7V[\x94` \x93\x90\x93\x015\x93PPPV[`\0\x80`\0``\x84\x86\x03\x12\x15a\t\x12W`\0\x80\xFD[a\t\x1B\x84a\x08\xB7V[\x92Pa\t)` \x85\x01a\x08\xB7V[\x91P`@\x84\x015\x90P\x92P\x92P\x92V[`\0\x80\x83`\x1F\x84\x01\x12a\tKW`\0\x80\xFD[P\x815g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\tcW`\0\x80\xFD[` \x83\x01\x91P\x83` \x82\x85\x01\x01\x11\x15a\t{W`\0\x80\xFD[\x92P\x92\x90PV[`\0\x80`\0\x80`\0``\x86\x88\x03\x12\x15a\t\x9AW`\0\x80\xFD[\x855\x94P` \x86\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\t\xB9W`\0\x80\xFD[a\t\xC5\x89\x83\x8A\x01a\t9V[\x90\x96P\x94P`@\x88\x015\x91P\x80\x82\x11\x15a\t\xDEW`\0\x80\xFD[Pa\t\xEB\x88\x82\x89\x01a\t9V[\x96\x99\x95\x98P\x93\x96P\x92\x94\x93\x92PPPV[`\0` \x82\x84\x03\x12\x15a\n\x0EW`\0\x80\xFD[a\n\x17\x82a\x08\xB7V[\x93\x92PPPV[`\0\x80`@\x83\x85\x03\x12\x15a\n1W`\0\x80\xFD[\x825\x91Pa\nA` \x84\x01a\x08\xB7V[\x90P\x92P\x92\x90PV[`\0\x80`@\x83\x85\x03\x12\x15a\n]W`\0\x80\xFD[a\nf\x83a\x08\xB7V[\x91Pa\nA` \x84\x01a\x08\xB7V[`\x01\x81\x81\x1C\x90\x82\x16\x80a\n\x88W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a\n\xA8WcNH{q`\xE0\x1B`\0R`\"`\x04R`$`\0\xFD[P\x91\x90PV[\x81\x83R\x81\x81` \x85\x017P`\0\x82\x82\x01` \x90\x81\x01\x91\x90\x91R`\x1F\x90\x91\x01`\x1F\x19\x16\x90\x91\x01\x01\x90V[`@\x81R`\0a\n\xEB`@\x83\x01\x86\x88a\n\xAEV[\x82\x81\x03` \x84\x01Ra\n\xFE\x81\x85\x87a\n\xAEV[\x97\x96PPPPPPPV[\x80\x82\x01\x80\x82\x11\x15a\x03=WcNH{q`\xE0\x1B`\0R`\x11`\x04R`$`\0\xFD\xFE\xA2dipfsX\"\x12 \xC8\xBB\x8E\x84\xF3\x1D,\xD3\x84-h\x05E\x8F\xAFa\x94\x88\xF9,\xC2\xDA\xC5\xA0\xB2\x9B\xC0dlQ\xFC?dsolcC\0\x08\x15\x003";
    /// The deployed bytecode of the contract.
    pub static ASTRIAMINTABLEERC20_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
    pub struct AstriaMintableERC20<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for AstriaMintableERC20<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for AstriaMintableERC20<M> {
        type Target = ::ethers::contract::Contract<M>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for AstriaMintableERC20<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for AstriaMintableERC20<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(AstriaMintableERC20))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> AstriaMintableERC20<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(
                address.into(),
                ASTRIAMINTABLEERC20_ABI.clone(),
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
                ASTRIAMINTABLEERC20_ABI.clone(),
                ASTRIAMINTABLEERC20_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }

        /// Calls the contract's `ASSET_WITHDRAWAL_DECIMALS` (0x8f2d8cb8) function
        pub fn asset_withdrawal_decimals(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, u32> {
            self.0
                .method_hash([143, 45, 140, 184], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `BRIDGE` (0xee9a31a2) function
        pub fn bridge(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
            self.0
                .method_hash([238, 154, 49, 162], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `allowance` (0xdd62ed3e) function
        pub fn allowance(
            &self,
            owner: ::ethers::core::types::Address,
            spender: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([221, 98, 237, 62], (owner, spender))
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `approve` (0x095ea7b3) function
        pub fn approve(
            &self,
            spender: ::ethers::core::types::Address,
            value: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([9, 94, 167, 179], (spender, value))
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `balanceOf` (0x70a08231) function
        pub fn balance_of(
            &self,
            account: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([112, 160, 130, 49], account)
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `decimals` (0x313ce567) function
        pub fn decimals(&self) -> ::ethers::contract::builders::ContractCall<M, u8> {
            self.0
                .method_hash([49, 60, 229, 103], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `mint` (0x40c10f19) function
        pub fn mint(
            &self,
            to: ::ethers::core::types::Address,
            amount: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([64, 193, 15, 25], (to, amount))
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `name` (0x06fdde03) function
        pub fn name(&self) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
            self.0
                .method_hash([6, 253, 222, 3], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `symbol` (0x95d89b41) function
        pub fn symbol(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::std::string::String> {
            self.0
                .method_hash([149, 216, 155, 65], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `totalSupply` (0x18160ddd) function
        pub fn total_supply(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([24, 22, 13, 221], ())
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `transfer` (0xa9059cbb) function
        pub fn transfer(
            &self,
            to: ::ethers::core::types::Address,
            value: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([169, 5, 156, 187], (to, value))
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `transferFrom` (0x23b872dd) function
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

        /// Calls the contract's `withdrawToIbcChain` (0x5fe56b09) function
        pub fn withdraw_to_ibc_chain(
            &self,
            amount: ::ethers::core::types::U256,
            destination_chain_address: ::std::string::String,
            memo: ::std::string::String,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([95, 229, 107, 9], (amount, destination_chain_address, memo))
                .expect("method not found (this should never happen)")
        }

        /// Calls the contract's `withdrawToSequencer` (0x757e9874) function
        pub fn withdraw_to_sequencer(
            &self,
            amount: ::ethers::core::types::U256,
            destination_chain_address: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([117, 126, 152, 116], (amount, destination_chain_address))
                .expect("method not found (this should never happen)")
        }

        /// Gets the contract's `Approval` event
        pub fn approval_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ApprovalFilter> {
            self.0.event()
        }

        /// Gets the contract's `Ics20Withdrawal` event
        pub fn ics_20_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, Ics20WithdrawalFilter>
        {
            self.0.event()
        }

        /// Gets the contract's `Mint` event
        pub fn mint_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, MintFilter> {
            self.0.event()
        }

        /// Gets the contract's `SequencerWithdrawal` event
        pub fn sequencer_withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, SequencerWithdrawalFilter>
        {
            self.0.event()
        }

        /// Gets the contract's `Transfer` event
        pub fn transfer_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, TransferFilter> {
            self.0.event()
        }

        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, AstriaMintableERC20Events>
        {
            self.0
                .event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
        for AstriaMintableERC20<M>
    {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    /// Custom Error type `ERC20InsufficientAllowance` with signature
    /// `ERC20InsufficientAllowance(address,uint256,uint256)` and selector `0xfb8f41b2`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
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
    /// Custom Error type `ERC20InsufficientBalance` with signature
    /// `ERC20InsufficientBalance(address,uint256,uint256)` and selector `0xe450d38c`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
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
    /// Custom Error type `ERC20InvalidApprover` with signature `ERC20InvalidApprover(address)` and
    /// selector `0xe602df05`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "ERC20InvalidApprover", abi = "ERC20InvalidApprover(address)")]
    pub struct ERC20InvalidApprover {
        pub approver: ::ethers::core::types::Address,
    }
    /// Custom Error type `ERC20InvalidReceiver` with signature `ERC20InvalidReceiver(address)` and
    /// selector `0xec442f05`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "ERC20InvalidReceiver", abi = "ERC20InvalidReceiver(address)")]
    pub struct ERC20InvalidReceiver {
        pub receiver: ::ethers::core::types::Address,
    }
    /// Custom Error type `ERC20InvalidSender` with signature `ERC20InvalidSender(address)` and
    /// selector `0x96c6fd1e`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "ERC20InvalidSender", abi = "ERC20InvalidSender(address)")]
    pub struct ERC20InvalidSender {
        pub sender: ::ethers::core::types::Address,
    }
    /// Custom Error type `ERC20InvalidSpender` with signature `ERC20InvalidSpender(address)` and
    /// selector `0x94280d62`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "ERC20InvalidSpender", abi = "ERC20InvalidSpender(address)")]
    pub struct ERC20InvalidSpender {
        pub spender: ::ethers::core::types::Address,
    }
    /// Container type for all of the contract's custom errors
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaMintableERC20Errors {
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
    impl ::ethers::core::abi::AbiDecode for AstriaMintableERC20Errors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) =
                <ERC20InsufficientAllowance as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ERC20InsufficientAllowance(decoded));
            }
            if let Ok(decoded) =
                <ERC20InsufficientBalance as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ERC20InsufficientBalance(decoded));
            }
            if let Ok(decoded) =
                <ERC20InvalidApprover as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ERC20InvalidApprover(decoded));
            }
            if let Ok(decoded) =
                <ERC20InvalidReceiver as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ERC20InvalidReceiver(decoded));
            }
            if let Ok(decoded) =
                <ERC20InvalidSender as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ERC20InvalidSender(decoded));
            }
            if let Ok(decoded) =
                <ERC20InvalidSpender as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ERC20InvalidSpender(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for AstriaMintableERC20Errors {
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
    impl ::ethers::contract::ContractRevert for AstriaMintableERC20Errors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <ERC20InsufficientAllowance as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ERC20InsufficientBalance as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ERC20InvalidApprover as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ERC20InvalidReceiver as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ERC20InvalidSender as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ERC20InvalidSpender as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for AstriaMintableERC20Errors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ERC20InsufficientAllowance(element) => ::core::fmt::Display::fmt(element, f),
                Self::ERC20InsufficientBalance(element) => ::core::fmt::Display::fmt(element, f),
                Self::ERC20InvalidApprover(element) => ::core::fmt::Display::fmt(element, f),
                Self::ERC20InvalidReceiver(element) => ::core::fmt::Display::fmt(element, f),
                Self::ERC20InvalidSender(element) => ::core::fmt::Display::fmt(element, f),
                Self::ERC20InvalidSpender(element) => ::core::fmt::Display::fmt(element, f),
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for AstriaMintableERC20Errors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<ERC20InsufficientAllowance> for AstriaMintableERC20Errors {
        fn from(value: ERC20InsufficientAllowance) -> Self {
            Self::ERC20InsufficientAllowance(value)
        }
    }
    impl ::core::convert::From<ERC20InsufficientBalance> for AstriaMintableERC20Errors {
        fn from(value: ERC20InsufficientBalance) -> Self {
            Self::ERC20InsufficientBalance(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidApprover> for AstriaMintableERC20Errors {
        fn from(value: ERC20InvalidApprover) -> Self {
            Self::ERC20InvalidApprover(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidReceiver> for AstriaMintableERC20Errors {
        fn from(value: ERC20InvalidReceiver) -> Self {
            Self::ERC20InvalidReceiver(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidSender> for AstriaMintableERC20Errors {
        fn from(value: ERC20InvalidSender) -> Self {
            Self::ERC20InvalidSender(value)
        }
    }
    impl ::core::convert::From<ERC20InvalidSpender> for AstriaMintableERC20Errors {
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
        Hash,
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
    #[ethevent(name = "Transfer", abi = "Transfer(address,address,uint256)")]
    pub struct TransferFilter {
        #[ethevent(indexed)]
        pub from: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub to: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    /// Container type for all of the contract's events
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaMintableERC20Events {
        ApprovalFilter(ApprovalFilter),
        Ics20WithdrawalFilter(Ics20WithdrawalFilter),
        MintFilter(MintFilter),
        SequencerWithdrawalFilter(SequencerWithdrawalFilter),
        TransferFilter(TransferFilter),
    }
    impl ::ethers::contract::EthLogDecode for AstriaMintableERC20Events {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = ApprovalFilter::decode_log(log) {
                return Ok(AstriaMintableERC20Events::ApprovalFilter(decoded));
            }
            if let Ok(decoded) = Ics20WithdrawalFilter::decode_log(log) {
                return Ok(AstriaMintableERC20Events::Ics20WithdrawalFilter(decoded));
            }
            if let Ok(decoded) = MintFilter::decode_log(log) {
                return Ok(AstriaMintableERC20Events::MintFilter(decoded));
            }
            if let Ok(decoded) = SequencerWithdrawalFilter::decode_log(log) {
                return Ok(AstriaMintableERC20Events::SequencerWithdrawalFilter(
                    decoded,
                ));
            }
            if let Ok(decoded) = TransferFilter::decode_log(log) {
                return Ok(AstriaMintableERC20Events::TransferFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for AstriaMintableERC20Events {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ApprovalFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::Ics20WithdrawalFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::MintFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::SequencerWithdrawalFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::TransferFilter(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<ApprovalFilter> for AstriaMintableERC20Events {
        fn from(value: ApprovalFilter) -> Self {
            Self::ApprovalFilter(value)
        }
    }
    impl ::core::convert::From<Ics20WithdrawalFilter> for AstriaMintableERC20Events {
        fn from(value: Ics20WithdrawalFilter) -> Self {
            Self::Ics20WithdrawalFilter(value)
        }
    }
    impl ::core::convert::From<MintFilter> for AstriaMintableERC20Events {
        fn from(value: MintFilter) -> Self {
            Self::MintFilter(value)
        }
    }
    impl ::core::convert::From<SequencerWithdrawalFilter> for AstriaMintableERC20Events {
        fn from(value: SequencerWithdrawalFilter) -> Self {
            Self::SequencerWithdrawalFilter(value)
        }
    }
    impl ::core::convert::From<TransferFilter> for AstriaMintableERC20Events {
        fn from(value: TransferFilter) -> Self {
            Self::TransferFilter(value)
        }
    }
    /// Container type for all input parameters for the `ASSET_WITHDRAWAL_DECIMALS` function with
    /// signature `ASSET_WITHDRAWAL_DECIMALS()` and selector `0x8f2d8cb8`
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
        name = "ASSET_WITHDRAWAL_DECIMALS",
        abi = "ASSET_WITHDRAWAL_DECIMALS()"
    )]
    pub struct AssetWithdrawalDecimalsCall;
    /// Container type for all input parameters for the `BRIDGE` function with signature `BRIDGE()`
    /// and selector `0xee9a31a2`
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
    #[ethcall(name = "BRIDGE", abi = "BRIDGE()")]
    pub struct BridgeCall;
    /// Container type for all input parameters for the `allowance` function with signature
    /// `allowance(address,address)` and selector `0xdd62ed3e`
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
    #[ethcall(name = "allowance", abi = "allowance(address,address)")]
    pub struct AllowanceCall {
        pub owner: ::ethers::core::types::Address,
        pub spender: ::ethers::core::types::Address,
    }
    /// Container type for all input parameters for the `approve` function with signature
    /// `approve(address,uint256)` and selector `0x095ea7b3`
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
    #[ethcall(name = "approve", abi = "approve(address,uint256)")]
    pub struct ApproveCall {
        pub spender: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    /// Container type for all input parameters for the `balanceOf` function with signature
    /// `balanceOf(address)` and selector `0x70a08231`
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
    #[ethcall(name = "balanceOf", abi = "balanceOf(address)")]
    pub struct BalanceOfCall {
        pub account: ::ethers::core::types::Address,
    }
    /// Container type for all input parameters for the `decimals` function with signature
    /// `decimals()` and selector `0x313ce567`
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
    #[ethcall(name = "decimals", abi = "decimals()")]
    pub struct DecimalsCall;
    /// Container type for all input parameters for the `mint` function with signature
    /// `mint(address,uint256)` and selector `0x40c10f19`
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
    #[ethcall(name = "mint", abi = "mint(address,uint256)")]
    pub struct MintCall {
        pub to: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    /// Container type for all input parameters for the `name` function with signature `name()` and
    /// selector `0x06fdde03`
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
    #[ethcall(name = "name", abi = "name()")]
    pub struct NameCall;
    /// Container type for all input parameters for the `symbol` function with signature `symbol()`
    /// and selector `0x95d89b41`
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
    #[ethcall(name = "symbol", abi = "symbol()")]
    pub struct SymbolCall;
    /// Container type for all input parameters for the `totalSupply` function with signature
    /// `totalSupply()` and selector `0x18160ddd`
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
    #[ethcall(name = "totalSupply", abi = "totalSupply()")]
    pub struct TotalSupplyCall;
    /// Container type for all input parameters for the `transfer` function with signature
    /// `transfer(address,uint256)` and selector `0xa9059cbb`
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
    #[ethcall(name = "transfer", abi = "transfer(address,uint256)")]
    pub struct TransferCall {
        pub to: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    /// Container type for all input parameters for the `transferFrom` function with signature
    /// `transferFrom(address,address,uint256)` and selector `0x23b872dd`
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
    #[ethcall(name = "transferFrom", abi = "transferFrom(address,address,uint256)")]
    pub struct TransferFromCall {
        pub from: ::ethers::core::types::Address,
        pub to: ::ethers::core::types::Address,
        pub value: ::ethers::core::types::U256,
    }
    /// Container type for all input parameters for the `withdrawToIbcChain` function with signature
    /// `withdrawToIbcChain(uint256,string,string)` and selector `0x5fe56b09`
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
        name = "withdrawToIbcChain",
        abi = "withdrawToIbcChain(uint256,string,string)"
    )]
    pub struct WithdrawToIbcChainCall {
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::std::string::String,
        pub memo: ::std::string::String,
    }
    /// Container type for all input parameters for the `withdrawToSequencer` function with
    /// signature `withdrawToSequencer(uint256,address)` and selector `0x757e9874`
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
        name = "withdrawToSequencer",
        abi = "withdrawToSequencer(uint256,address)"
    )]
    pub struct WithdrawToSequencerCall {
        pub amount: ::ethers::core::types::U256,
        pub destination_chain_address: ::ethers::core::types::Address,
    }
    /// Container type for all of the contract's call
    #[derive(Clone, ::ethers::contract::EthAbiType, Debug, PartialEq, Eq, Hash)]
    pub enum AstriaMintableERC20Calls {
        AssetWithdrawalDecimals(AssetWithdrawalDecimalsCall),
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
    impl ::ethers::core::abi::AbiDecode for AstriaMintableERC20Calls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <AssetWithdrawalDecimalsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::AssetWithdrawalDecimals(decoded));
            }
            if let Ok(decoded) = <BridgeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Bridge(decoded));
            }
            if let Ok(decoded) = <AllowanceCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Allowance(decoded));
            }
            if let Ok(decoded) = <ApproveCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Approve(decoded));
            }
            if let Ok(decoded) = <BalanceOfCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::BalanceOf(decoded));
            }
            if let Ok(decoded) = <DecimalsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Decimals(decoded));
            }
            if let Ok(decoded) = <MintCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Mint(decoded));
            }
            if let Ok(decoded) = <NameCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Name(decoded));
            }
            if let Ok(decoded) = <SymbolCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Symbol(decoded));
            }
            if let Ok(decoded) = <TotalSupplyCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::TotalSupply(decoded));
            }
            if let Ok(decoded) = <TransferCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Transfer(decoded));
            }
            if let Ok(decoded) = <TransferFromCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::TransferFrom(decoded));
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
    impl ::ethers::core::abi::AbiEncode for AstriaMintableERC20Calls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::AssetWithdrawalDecimals(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Bridge(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Allowance(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Approve(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::BalanceOf(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Decimals(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Mint(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Name(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Symbol(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::TotalSupply(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Transfer(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::TransferFrom(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::WithdrawToIbcChain(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::WithdrawToSequencer(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
            }
        }
    }
    impl ::core::fmt::Display for AstriaMintableERC20Calls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::AssetWithdrawalDecimals(element) => ::core::fmt::Display::fmt(element, f),
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
                Self::WithdrawToIbcChain(element) => ::core::fmt::Display::fmt(element, f),
                Self::WithdrawToSequencer(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<AssetWithdrawalDecimalsCall> for AstriaMintableERC20Calls {
        fn from(value: AssetWithdrawalDecimalsCall) -> Self {
            Self::AssetWithdrawalDecimals(value)
        }
    }
    impl ::core::convert::From<BridgeCall> for AstriaMintableERC20Calls {
        fn from(value: BridgeCall) -> Self {
            Self::Bridge(value)
        }
    }
    impl ::core::convert::From<AllowanceCall> for AstriaMintableERC20Calls {
        fn from(value: AllowanceCall) -> Self {
            Self::Allowance(value)
        }
    }
    impl ::core::convert::From<ApproveCall> for AstriaMintableERC20Calls {
        fn from(value: ApproveCall) -> Self {
            Self::Approve(value)
        }
    }
    impl ::core::convert::From<BalanceOfCall> for AstriaMintableERC20Calls {
        fn from(value: BalanceOfCall) -> Self {
            Self::BalanceOf(value)
        }
    }
    impl ::core::convert::From<DecimalsCall> for AstriaMintableERC20Calls {
        fn from(value: DecimalsCall) -> Self {
            Self::Decimals(value)
        }
    }
    impl ::core::convert::From<MintCall> for AstriaMintableERC20Calls {
        fn from(value: MintCall) -> Self {
            Self::Mint(value)
        }
    }
    impl ::core::convert::From<NameCall> for AstriaMintableERC20Calls {
        fn from(value: NameCall) -> Self {
            Self::Name(value)
        }
    }
    impl ::core::convert::From<SymbolCall> for AstriaMintableERC20Calls {
        fn from(value: SymbolCall) -> Self {
            Self::Symbol(value)
        }
    }
    impl ::core::convert::From<TotalSupplyCall> for AstriaMintableERC20Calls {
        fn from(value: TotalSupplyCall) -> Self {
            Self::TotalSupply(value)
        }
    }
    impl ::core::convert::From<TransferCall> for AstriaMintableERC20Calls {
        fn from(value: TransferCall) -> Self {
            Self::Transfer(value)
        }
    }
    impl ::core::convert::From<TransferFromCall> for AstriaMintableERC20Calls {
        fn from(value: TransferFromCall) -> Self {
            Self::TransferFrom(value)
        }
    }
    impl ::core::convert::From<WithdrawToIbcChainCall> for AstriaMintableERC20Calls {
        fn from(value: WithdrawToIbcChainCall) -> Self {
            Self::WithdrawToIbcChain(value)
        }
    }
    impl ::core::convert::From<WithdrawToSequencerCall> for AstriaMintableERC20Calls {
        fn from(value: WithdrawToSequencerCall) -> Self {
            Self::WithdrawToSequencer(value)
        }
    }
    /// Container type for all return fields from the `ASSET_WITHDRAWAL_DECIMALS` function with
    /// signature `ASSET_WITHDRAWAL_DECIMALS()` and selector `0x8f2d8cb8`
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
    pub struct AssetWithdrawalDecimalsReturn(pub u32);
    /// Container type for all return fields from the `BRIDGE` function with signature `BRIDGE()`
    /// and selector `0xee9a31a2`
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
    pub struct BridgeReturn(pub ::ethers::core::types::Address);
    /// Container type for all return fields from the `allowance` function with signature
    /// `allowance(address,address)` and selector `0xdd62ed3e`
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
    pub struct AllowanceReturn(pub ::ethers::core::types::U256);
    /// Container type for all return fields from the `approve` function with signature
    /// `approve(address,uint256)` and selector `0x095ea7b3`
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
    pub struct ApproveReturn(pub bool);
    /// Container type for all return fields from the `balanceOf` function with signature
    /// `balanceOf(address)` and selector `0x70a08231`
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
    pub struct BalanceOfReturn(pub ::ethers::core::types::U256);
    /// Container type for all return fields from the `decimals` function with signature
    /// `decimals()` and selector `0x313ce567`
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
    pub struct DecimalsReturn(pub u8);
    /// Container type for all return fields from the `name` function with signature `name()` and
    /// selector `0x06fdde03`
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
    pub struct NameReturn(pub ::std::string::String);
    /// Container type for all return fields from the `symbol` function with signature `symbol()`
    /// and selector `0x95d89b41`
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
    pub struct SymbolReturn(pub ::std::string::String);
    /// Container type for all return fields from the `totalSupply` function with signature
    /// `totalSupply()` and selector `0x18160ddd`
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
    pub struct TotalSupplyReturn(pub ::ethers::core::types::U256);
    /// Container type for all return fields from the `transfer` function with signature
    /// `transfer(address,uint256)` and selector `0xa9059cbb`
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
    pub struct TransferReturn(pub bool);
    /// Container type for all return fields from the `transferFrom` function with signature
    /// `transferFrom(address,address,uint256)` and selector `0x23b872dd`
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
    pub struct TransferFromReturn(pub bool);
}
