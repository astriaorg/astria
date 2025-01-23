/// A JSON-encoded form of this message is used as the upgrades file for the Sequencer.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Upgrades {
    #[prost(message, optional, tag = "1")]
    pub upgrade_1: ::core::option::Option<Upgrade1>,
}
impl ::prost::Name for Upgrades {
    const NAME: &'static str = "Upgrades";
    const PACKAGE: &'static str = "astria.upgrades.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.upgrades.v1.{}", Self::NAME)
    }
}
/// Info specific to a given upgrade.
///
/// All upgrades have this info at a minimum.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BaseUpgradeInfo {
    /// The upgrade should be applied during the lifecycle of the block at this height.
    #[prost(uint64, tag = "1")]
    pub activation_height: u64,
    /// The app version running after the upgrade is applied.
    #[prost(uint64, tag = "2")]
    pub app_version: u64,
}
impl ::prost::Name for BaseUpgradeInfo {
    const NAME: &'static str = "BaseUpgradeInfo";
    const PACKAGE: &'static str = "astria.upgrades.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.upgrades.v1.{}", Self::NAME)
    }
}
/// Upgrade 1 of the Sequencer network.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Upgrade1 {
    #[prost(message, optional, tag = "1")]
    pub base_info: ::core::option::Option<BaseUpgradeInfo>,
    #[prost(message, optional, tag = "2")]
    pub connect_oracle_change: ::core::option::Option<upgrade1::ConnectOracleChange>,
    #[prost(message, optional, tag = "3")]
    pub validator_update_action_change: ::core::option::Option<
        upgrade1::ValidatorUpdateActionChange,
    >,
}
/// Nested message and enum types in `Upgrade1`.
pub mod upgrade1 {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ConnectOracleChange {
        /// The Connect oracle genesis data.
        #[prost(message, optional, tag = "1")]
        pub genesis: ::core::option::Option<
            super::super::super::protocol::genesis::v1::ConnectGenesis,
        >,
    }
    impl ::prost::Name for ConnectOracleChange {
        const NAME: &'static str = "ConnectOracleChange";
        const PACKAGE: &'static str = "astria.upgrades.v1";
        fn full_name() -> ::prost::alloc::string::String {
            ::prost::alloc::format!("astria.upgrades.v1.Upgrade1.{}", Self::NAME)
        }
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ValidatorUpdateActionChange {}
    impl ::prost::Name for ValidatorUpdateActionChange {
        const NAME: &'static str = "ValidatorUpdateActionChange";
        const PACKAGE: &'static str = "astria.upgrades.v1";
        fn full_name() -> ::prost::alloc::string::String {
            ::prost::alloc::format!("astria.upgrades.v1.Upgrade1.{}", Self::NAME)
        }
    }
}
impl ::prost::Name for Upgrade1 {
    const NAME: &'static str = "Upgrade1";
    const PACKAGE: &'static str = "astria.upgrades.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("astria.upgrades.v1.{}", Self::NAME)
    }
}
