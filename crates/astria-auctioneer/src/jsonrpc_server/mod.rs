use alloy_primitives::B256;
use astria_eyre::eyre::{
    self,
    WrapErr as _,
};
use jiff::Timestamp;
use jsonrpsee::{
    server::Server,
    types::{
        ErrorCode,
        ErrorObject,
        ErrorObjectOwned,
    },
    RpcModule,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::orderpool::{
    self,
    rpc::{
        RawBundle,
        RawBundleToOrderError,
    },
};

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
struct EthSendBundleResponse {
    #[serde(flatten)]
    kind: EthSendBundleResponseKind,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "action")]
enum EthSendBundleResponseKind {
    #[serde(rename = "order cancelled")]
    OrderCancelled {
        uuid: Uuid,
        cancellation_timestamp: Timestamp,
        order_timestamp: Timestamp,
        bundle_hash: B256,
    },
    #[serde(rename = "order not found")]
    OrderNotFound {
        uuid: Uuid,
        cancellation_timestamp: Timestamp,
    },
    #[serde(rename = "order not cancelled")]
    OrderNotCancelled {
        uuid: Uuid,
        cancellation_timestamp: Timestamp,
        order_timestamp: Timestamp,
        bundle_hash: B256,
    },
    #[serde(rename = "order placed")]
    OrderPlaced {
        uuid: Uuid,
        timestamp: Timestamp,
        bundle_hash: B256,
    },
    #[serde(rename = "order replaced")]
    OrderReplaced {
        uuid: Uuid,
        inserted_timestamp: Timestamp,
        inserted_bundle_hash: B256,
        removed_timestamp: Timestamp,
        removed_bundle_hash: B256,
    },
    #[serde(rename = "order not replaced")]
    OrderNotReplaced {
        uuid: Uuid,
        request_bundle_hash: B256,
        request_timestamp: Timestamp,
        stored_bundle_hash: B256,
        stored_timestamp: Timestamp,
    },
}

impl From<orderpool::Response> for EthSendBundleResponse {
    fn from(value: orderpool::Response) -> Self {
        match value {
            orderpool::Response::ForOrder(for_order) => for_order.into(),
            orderpool::Response::ForCancellation(for_cancellation) => for_cancellation.into(),
        }
    }
}

impl From<EthSendBundleResponseKind> for EthSendBundleResponse {
    fn from(kind: EthSendBundleResponseKind) -> Self {
        Self {
            kind,
        }
    }
}

impl From<orderpool::ForCancellation> for EthSendBundleResponse {
    fn from(value: orderpool::ForCancellation) -> Self {
        EthSendBundleResponseKind::from(value).into()
    }
}

impl From<orderpool::ForCancellation> for EthSendBundleResponseKind {
    fn from(value: orderpool::ForCancellation) -> Self {
        let orderpool::ForCancellation {
            uuid,
            timestamp,
            action,
        } = value;
        match action {
            orderpool::RemovedOrNotFound::Removed(bundle) => Self::OrderCancelled {
                uuid,
                cancellation_timestamp: timestamp,
                order_timestamp: *bundle.timestamp(),
                bundle_hash: *bundle.hash(),
            },

            orderpool::RemovedOrNotFound::NotFound(_) => Self::OrderNotFound {
                uuid,
                cancellation_timestamp: timestamp,
            },

            orderpool::RemovedOrNotFound::Aborted {
                in_storage_bundle, ..
            } => Self::OrderNotCancelled {
                uuid,
                cancellation_timestamp: timestamp,
                order_timestamp: *in_storage_bundle.timestamp(),
                bundle_hash: *in_storage_bundle.hash(),
            },
        }
    }
}

impl From<orderpool::ForOrder> for EthSendBundleResponse {
    fn from(value: orderpool::ForOrder) -> Self {
        EthSendBundleResponseKind::from(value).into()
    }
}

impl From<orderpool::ForOrder> for EthSendBundleResponseKind {
    fn from(value: orderpool::ForOrder) -> Self {
        let orderpool::ForOrder {
            uuid,
            action,
            ..
        } = value;
        match action {
            orderpool::InsertedOrReplaced::Inserted {
                timestamp,
                bundle_hash,
                ..
            } => Self::OrderPlaced {
                uuid,
                timestamp,
                bundle_hash,
            },
            orderpool::InsertedOrReplaced::Replaced {
                old,
                new,
            } => Self::OrderReplaced {
                uuid,
                inserted_timestamp: *new.timestamp(),
                inserted_bundle_hash: *new.hash(),
                removed_timestamp: *old.timestamp(),
                removed_bundle_hash: *old.hash(),
            },
            orderpool::InsertedOrReplaced::Aborted {
                requested,
                in_storage,
            } => Self::OrderNotReplaced {
                uuid,
                request_bundle_hash: *requested.hash(),
                request_timestamp: *requested.timestamp(),
                stored_bundle_hash: *in_storage.hash(),
                stored_timestamp: *in_storage.timestamp(),
            },
        }
    }
}

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
    let raw_bundle: RawBundle = params.one().map_err(EthSendBundleError::ParsePayload)?;

    let order = raw_bundle
        .interpret_as_order()
        .map_err(EthSendBundleError::InvalidPayload)?;

    let rsp = cancellation_token
        .run_until_cancelled(to_orderpool.send(order))
        .await
        .ok_or(EthSendBundleError::Cancelled)?
        .map_err(EthSendBundleError::Orderpool)?;

    Ok(rsp.into())
}
