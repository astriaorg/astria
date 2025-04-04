use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use jsonrpsee::{
    server::Server,
    types::{
        ErrorCode,
        ErrorObject,
        ErrorObjectOwned,
    },
    RpcModule,
};
use payloads::{
    RawBundle,
    RawBundleToOrderError,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub(crate) mod payloads;

const ETH_SENDBUNDLE: &str = "eth_sendBundle";

pub(crate) struct Builder {
    pub(crate) cancellation_token: CancellationToken,
    pub(crate) endpoint: String,
    pub(crate) to_orderpool: crate::orderpool::Sender,
}

impl Builder {
    /// Spawns the JSONRPC server on the tokio runtime.
    ///
    /// Instantiation and running is done in one fuction to because
    /// jsonrpsee [`Server::builder]` is async.
    pub(crate) fn start(self) -> JoinHandle<eyre::Result<()>> {
        let Self {
            cancellation_token,
            endpoint,
            to_orderpool,
        } = self;

        tokio::spawn(async move {
            let server = Server::builder()
                .build(&endpoint)
                .await
                .wrap_err_with(|| format!("failed instantiating jsonrpc server `{endpoint}`"))?;

            let mut module = RpcModule::new(());
            {
                let to_orderpool = to_orderpool.clone();
                let cancellation_token = cancellation_token.child_token();
                module
                    .register_async_method(ETH_SENDBUNDLE, move |params, _, _| {
                        eth_send_bundle(to_orderpool.clone(), cancellation_token.clone(), params)
                    })
                    .wrap_err_with(|| {
                        format!("failed registering `{ETH_SENDBUNDLE}` RPC on server")
                    })?;
            }
            let handle = server.start(module);

            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    let _ = handle.stop();
                },
                _ = handle.clone().stopped() => {
                },
            };
            Ok(())
        })
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct EthSendBundleResponse;

#[derive(Debug, thiserror::Error)]
enum EthSendBundleError {
    #[error("failed parsing payload")]
    ParsePayload(#[source] ErrorObjectOwned),
    // TODO: The parse + "interpret" step should really just be combined into a single
    // "InvalidPayload".
    #[error("the payload could not be interpreted as an order")]
    InvalidPayload(#[source] RawBundleToOrderError),
    #[error("failed forwarding order to orderpool")]
    Orderpool(#[source] crate::orderpool::channel::SendError),
    #[error("request received a cancellation signal")]
    Cancelled,
}

impl From<EthSendBundleError> for ErrorObject<'static> {
    fn from(_value: EthSendBundleError) -> Self {
        ErrorObject::owned::<()>(
            ErrorCode::InternalError.code(),
            "<unimplemented error message>",
            None,
        )
    }
}

async fn eth_send_bundle(
    to_orderpool: crate::orderpool::Sender,
    cancellation_token: CancellationToken,
    params: jsonrpsee::types::Params<'static>,
) -> Result<EthSendBundleResponse, EthSendBundleError> {
    // let received_at = OffsetDateTime::now_utc();
    // let start = Instant::now();
    let raw_bundle: RawBundle = params.one().map_err(EthSendBundleError::ParsePayload)?;

    let order = raw_bundle
        .interpret_as_order()
        .map_err(EthSendBundleError::InvalidPayload)?;

    // TODO: turn this orderpool response into an object that we can reasonably return
    // to the client.
    let _rsp = cancellation_token
        .run_until_cancelled(to_orderpool.send(order))
        .await
        .ok_or(EthSendBundleError::Cancelled)?
        .map_err(EthSendBundleError::Orderpool)?;
    Ok(EthSendBundleResponse)
}
