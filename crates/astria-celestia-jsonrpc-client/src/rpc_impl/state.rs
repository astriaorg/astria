/// The Celestia JSON RPC state API.
///
/// This currently only provides a wrapper for the `state.SubmitPayForBlob` RPC method.
/// It is not completely clear what value `fee` should take. According to the
/// [go submitter interface] this is a cosmos-sdk/math.Int which wraps big.Int.
///
/// This implementation follows cosmrs's choice to use `u128` to
/// represent amounts. See their [`cosmrs::Amount`] type, and the discussion in [cosmos-rust
/// PR#235].
///
/// [go submitter interface]: https://github.com/celestiaorg/celestia-node/blob/beaf6dbdc73fd43b73a98578330a7a5ad422c3c8/blob/service.go#L31
/// [`cosmrs::Amount`]: https://github.com/cosmos/cosmos-rust/blob/aef5c708e6dddeec4ad1ba2672c7874a40b9bfc1/cosmrs/src/base.rs#L10
/// [cosmos-rust PR#235]: https://github.com/cosmos/cosmos-rust/pull/235
use jsonrpsee::proc_macros::rpc;
// This only needs to be explicitly imported when activaing the server feature
// due to a quirk in the jsonrpsee proc macro.
#[cfg(feature = "server")]
use jsonrpsee::types::ErrorObjectOwned;
use serde::Serialize;

use crate::rpc_impl::blob::Blob;

/// Newtype wrapper around a `u128` to serialize it as a string.
///
/// This is necessary because the `state.SubmitPayForBlob` endpoint requires
/// a String object and is not able to directly unmarshal a json number to a
/// `math.Int`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "server", derive(serde::Deserialize))]
pub struct Fee(#[serde(with = "u128_string")] u128);

mod u128_string {
    use serde::{
        Serialize,
        Serializer,
    };

    #[cfg(feature = "server")]
    pub fn deserialize<'de, D>(deser: D) -> Result<u128, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::Deserialize as _;
        let s = String::deserialize(deser)?;
        s.parse::<u128>().map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(val: &u128, ser: S) -> Result<S::Ok, S::Error> {
        let val = val.to_string();
        val.serialize(ser)
    }
}

impl Fee {
    /// Construct a new `Fee` from a `u128`.
    #[must_use]
    pub fn from_u128(val: u128) -> Self {
        Self(val)
    }
}

#[cfg_attr(not(feature = "server"), rpc(client))]
#[cfg_attr(feature = "server", rpc(client, server))]
pub trait State {
    #[method(name = "state.SubmitPayForBlob")]
    async fn submit_pay_for_blob(
        &self,
        fee: Fee,
        gas_limit: u64,
        blobs: Vec<Blob>,
    ) -> Result<Box<serde_json::value::RawValue>, ErrorObjectOwned>;
}
