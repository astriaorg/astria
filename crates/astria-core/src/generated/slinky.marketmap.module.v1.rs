/// Module is the config object of the builder module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Module {
    /// Authority defines the custom module authority. If not set, defaults to the
    /// governance module.
    #[prost(string, tag = "1")]
    pub authority: ::prost::alloc::string::String,
    /// HooksOrder specifies the order of marketmap hooks and should be a list
    /// of module names which provide a marketmap hooks instance. If no order is
    /// provided, then hooks will be applied in alphabetical order of module names.
    #[prost(string, repeated, tag = "2")]
    pub hooks_order: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
impl ::prost::Name for Module {
    const NAME: &'static str = "Module";
    const PACKAGE: &'static str = "slinky.marketmap.module.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.marketmap.module.v1.{}", Self::NAME)
    }
}
