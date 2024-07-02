/// Module is the config object of the builder module.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Module {
    /// Authority defines the custom module authority. If not set, defaults to the
    /// governance module.
    #[prost(string, tag = "1")]
    pub authority: ::prost::alloc::string::String,
}
impl ::prost::Name for Module {
    const NAME: &'static str = "Module";
    const PACKAGE: &'static str = "slinky.oracle.module.v1";
    fn full_name() -> ::prost::alloc::string::String {
        ::prost::alloc::format!("slinky.oracle.module.v1.{}", Self::NAME)
    }
}
