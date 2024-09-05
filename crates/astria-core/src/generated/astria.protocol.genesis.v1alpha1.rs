#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenesisAppState {
    #[prost(string, tag = "1")]
    pub chain_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub address_prefixes: ::core::option::Option<AddressPrefixes>,
    #[prost(message, repeated, tag = "3")]
    pub accounts: ::prost::alloc::vec::Vec<Account>,
    #[prost(message, optional, tag = "4")]
    pub authority_sudo_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, optional, tag = "5")]
    pub ibc_sudo_address: ::core::option::Option<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(message, repeated, tag = "6")]
    pub ibc_relayer_addresses: ::prost::alloc::vec::Vec<
        super::super::super::primitive::v1::Address,
    >,
    #[prost(string, tag = "7")]
    pub native_asset_base_denomination: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "8")]
    pub ibc_parameters: ::core::option::Option<IbcParameters>,
    #[prost(string, repeated, tag = "9")]
    pub allowed_fee_assets: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "10")]
    pub fees: ::core::option::Option<Fees>,
}
impl ::prost::Name for GenesisAppState {
    const NAME: &'static str = "GenesisAppState";
    const PACKAGE: &'static str = "astria.protocol.genesis.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.genesis.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Account {
    #[prost(message, optional, tag = "1")]
    pub address: ::core::option::Option<super::super::super::primitive::v1::Address>,
    #[prost(message, optional, tag = "2")]
    pub balance: ::core::option::Option<super::super::super::primitive::v1::Uint128>,
}
impl ::prost::Name for Account {
    const NAME: &'static str = "Account";
    const PACKAGE: &'static str = "astria.protocol.genesis.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.genesis.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddressPrefixes {
    /// The base prefix used for most Astria Sequencer addresses.
    #[prost(string, tag = "1")]
    pub base: ::prost::alloc::string::String,
    /// The prefix used for sending ics20 transfers to IBC chains
    /// that enforce a bech32 format of the packet sender.
    #[prost(string, tag = "2")]
    pub ibc_compat: ::prost::alloc::string::String,
}
impl ::prost::Name for AddressPrefixes {
    const NAME: &'static str = "AddressPrefixes";
    const PACKAGE: &'static str = "astria.protocol.genesis.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.genesis.v1alpha1.{}", Self::NAME)
    }
}
/// IBC configuration data.
#[derive(Copy)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IbcParameters {
    /// Whether IBC (forming connections, processing IBC packets) is enabled.
    #[prost(bool, tag = "1")]
    pub ibc_enabled: bool,
    /// Whether inbound ICS-20 transfers are enabled
    #[prost(bool, tag = "2")]
    pub inbound_ics20_transfers_enabled: bool,
    /// Whether outbound ICS-20 transfers are enabled
    #[prost(bool, tag = "3")]
    pub outbound_ics20_transfers_enabled: bool,
}
impl ::prost::Name for IbcParameters {
    const NAME: &'static str = "IbcParameters";
    const PACKAGE: &'static str = "astria.protocol.genesis.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.genesis.v1alpha1.{}", Self::NAME)
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Fees {
    #[prost(message, optional, tag = "1")]
    pub transfer_base_fee: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(message, optional, tag = "2")]
    pub sequence_base_fee: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(message, optional, tag = "3")]
    pub sequence_byte_cost_multiplier: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(message, optional, tag = "4")]
    pub init_bridge_account_base_fee: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(message, optional, tag = "5")]
    pub bridge_lock_byte_cost_multiplier: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(message, optional, tag = "6")]
    pub bridge_sudo_change_fee: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
    #[prost(message, optional, tag = "7")]
    pub ics20_withdrawal_base_fee: ::core::option::Option<
        super::super::super::primitive::v1::Uint128,
    >,
}
impl ::prost::Name for Fees {
    const NAME: &'static str = "Fees";
    const PACKAGE: &'static str = "astria.protocol.genesis.v1alpha1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.protocol.genesis.v1alpha1.{}", Self::NAME)
    }
}
