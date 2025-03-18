#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionFee {
    #[prost(string, tag = "1")]
    pub asset: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub fee: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for TransactionFee {
    const NAME: &'static str = "TransactionFee";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransferFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for TransferFeeComponents {
    const NAME: &'static str = "TransferFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RollupDataSubmissionFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for RollupDataSubmissionFeeComponents {
    const NAME: &'static str = "RollupDataSubmissionFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InitBridgeAccountFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for InitBridgeAccountFeeComponents {
    const NAME: &'static str = "InitBridgeAccountFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeLockFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for BridgeLockFeeComponents {
    const NAME: &'static str = "BridgeLockFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeUnlockFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for BridgeUnlockFeeComponents {
    const NAME: &'static str = "BridgeUnlockFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeSudoChangeFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for BridgeSudoChangeFeeComponents {
    const NAME: &'static str = "BridgeSudoChangeFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BridgeTransferFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for BridgeTransferFeeComponents {
    const NAME: &'static str = "BridgeTransferFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ics20WithdrawalFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for Ics20WithdrawalFeeComponents {
    const NAME: &'static str = "Ics20WithdrawalFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcRelayFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for IbcRelayFeeComponents {
    const NAME: &'static str = "IbcRelayFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorUpdateFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for ValidatorUpdateFeeComponents {
    const NAME: &'static str = "ValidatorUpdateFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeeAssetChangeFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for FeeAssetChangeFeeComponents {
    const NAME: &'static str = "FeeAssetChangeFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeeChangeFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for FeeChangeFeeComponents {
    const NAME: &'static str = "FeeChangeFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcRelayerChangeFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for IbcRelayerChangeFeeComponents {
    const NAME: &'static str = "IbcRelayerChangeFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SudoAddressChangeFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for SudoAddressChangeFeeComponents {
    const NAME: &'static str = "SudoAddressChangeFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcSudoChangeFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for IbcSudoChangeFeeComponents {
    const NAME: &'static str = "IbcSudoChangeFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RecoverIbcClientFeeComponents {
    #[prost(message, optional, tag = "1")]
    pub base: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
    #[prost(message, optional, tag = "2")]
    pub multiplier: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for RecoverIbcClientFeeComponents {
    const NAME: &'static str = "RecoverIbcClientFeeComponents";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
/// Response to a transaction fee ABCI query.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionFeeResponse {
    #[prost(uint64, tag = "2")]
    pub height: u64,
    #[prost(message, repeated, tag = "3")]
    pub fees: ::prost::alloc::vec::Vec<TransactionFee>,
}
impl ::prost::Name for TransactionFeeResponse {
    const NAME: &'static str = "TransactionFeeResponse";
    const PACKAGE: &'static str = "astria.protocol.fees.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.fees.v1.{}", Self::NAME)
    }
}
