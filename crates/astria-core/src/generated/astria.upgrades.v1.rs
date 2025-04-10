// This file is @generated by prost-build.
/// A JSON-encoded form of this message is used as the upgrades file for the Sequencer.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Upgrades {
    #[prost(message, optional, tag = "1")]
    pub aspen: ::core::option::Option<Aspen>,
}
impl ::prost::Name for Upgrades {
    const NAME: &'static str = "Upgrades";
    const PACKAGE: &'static str = "astria.upgrades.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "astria.upgrades.v1.Upgrades".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/astria.upgrades.v1.Upgrades".into()
    }
}
/// Info specific to a given upgrade.
///
/// All upgrades have this info at a minimum.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
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
        "astria.upgrades.v1.BaseUpgradeInfo".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/astria.upgrades.v1.BaseUpgradeInfo".into()
    }
}
/// Aspen upgrade of the Sequencer network.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Aspen {
    #[prost(message, optional, tag = "1")]
    pub base_info: ::core::option::Option<BaseUpgradeInfo>,
    #[prost(message, optional, tag = "2")]
    pub price_feed_change: ::core::option::Option<aspen::PriceFeedChange>,
    #[prost(message, optional, tag = "3")]
    pub validator_update_action_change: ::core::option::Option<
        aspen::ValidatorUpdateActionChange,
    >,
    #[prost(message, optional, tag = "4")]
    pub ibc_acknowledgement_failure_change: ::core::option::Option<
        aspen::IbcAcknowledgementFailureChange,
    >,
}
/// Nested message and enum types in `Aspen`.
pub mod aspen {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PriceFeedChange {
        /// The price feed genesis data.
        #[prost(message, optional, tag = "1")]
        pub genesis: ::core::option::Option<
            super::super::super::protocol::genesis::v1::PriceFeedGenesis,
        >,
    }
    impl ::prost::Name for PriceFeedChange {
        const NAME: &'static str = "PriceFeedChange";
        const PACKAGE: &'static str = "astria.upgrades.v1";
        fn full_name() -> ::prost::alloc::string::String {
            "astria.upgrades.v1.Aspen.PriceFeedChange".into()
        }
        fn type_url() -> ::prost::alloc::string::String {
            "/astria.upgrades.v1.Aspen.PriceFeedChange".into()
        }
    }
    #[derive(Clone, Copy, PartialEq, ::prost::Message)]
    pub struct ValidatorUpdateActionChange {}
    impl ::prost::Name for ValidatorUpdateActionChange {
        const NAME: &'static str = "ValidatorUpdateActionChange";
        const PACKAGE: &'static str = "astria.upgrades.v1";
        fn full_name() -> ::prost::alloc::string::String {
            "astria.upgrades.v1.Aspen.ValidatorUpdateActionChange".into()
        }
        fn type_url() -> ::prost::alloc::string::String {
            "/astria.upgrades.v1.Aspen.ValidatorUpdateActionChange".into()
        }
    }
    #[derive(Clone, Copy, PartialEq, ::prost::Message)]
    pub struct IbcAcknowledgementFailureChange {}
    impl ::prost::Name for IbcAcknowledgementFailureChange {
        const NAME: &'static str = "IbcAcknowledgementFailureChange";
        const PACKAGE: &'static str = "astria.upgrades.v1";
        fn full_name() -> ::prost::alloc::string::String {
            "astria.upgrades.v1.Aspen.IbcAcknowledgementFailureChange".into()
        }
        fn type_url() -> ::prost::alloc::string::String {
            "/astria.upgrades.v1.Aspen.IbcAcknowledgementFailureChange".into()
        }
    }
}
impl ::prost::Name for Aspen {
    const NAME: &'static str = "Aspen";
    const PACKAGE: &'static str = "astria.upgrades.v1";
    fn full_name() -> ::prost::alloc::string::String {
        "astria.upgrades.v1.Aspen".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/astria.upgrades.v1.Aspen".into()
    }
}
