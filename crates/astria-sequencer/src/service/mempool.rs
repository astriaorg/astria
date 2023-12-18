use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
};
use tendermint::v0_37::abci::{
    request,
    response,
    MempoolRequest,
    MempoolResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::Instrument;

use crate::accounts::state_ext::StateReadExt;

const MAX_TX_SIZE: usize = 256_000; // 256 KB

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::v0_37::abci::response::CheckTx`].
#[derive(Clone)]
pub(crate) struct Mempool {
    storage: Storage,
}

impl Mempool {
    pub(crate) fn new(storage: Storage) -> Self {
        Self {
            storage,
        }
    }
}

impl Service<MempoolRequest> for Mempool {
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<MempoolResponse, BoxError>> + Send + 'static>>;
    type Response = MempoolResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MempoolRequest) -> Self::Future {
        use penumbra_tower_trace::v037::RequestExt as _;
        let span = req.create_span();
        let storage = self.storage.clone();
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => {
                    MempoolResponse::CheckTx(handle_check_tx(req, storage.latest_snapshot()).await)
                }
            };
            Ok(rsp)
        }
        .instrument(span)
        .boxed()
    }
}

async fn handle_check_tx<S: StateReadExt + 'static>(
    req: request::CheckTx,
    state: S,
) -> response::CheckTx {
    use proto::{
        generated::sequencer::v1alpha1 as raw,
        native::sequencer::v1alpha1::SignedTransaction,
        Message as _,
    };
    use sequencer_types::abci_code::AbciCode;

    use crate::transaction;

    let request::CheckTx {
        tx, ..
    } = req;
    if tx.len() > MAX_TX_SIZE {
        return response::CheckTx {
            code: AbciCode::INVALID_SIZE.into(),
            log: format!(
                "transaction size too large; received: {}, allowed: {MAX_TX_SIZE}",
                tx.len()
            ),
            info: AbciCode::INVALID_SIZE.to_string(),
            ..response::CheckTx::default()
        };
    }

    let raw_signed_tx = match raw::SignedTransaction::decode(tx) {
        Ok(tx) => tx,
        Err(e) => {
            return response::CheckTx {
                code: AbciCode::INVALID_PARAMETER.into(),
                log: format!("{e:?}"),
                info: "failed decoding bytes as a protobuf SignedTransaction".into(),
                ..response::CheckTx::default()
            };
        }
    };
    let signed_tx = match SignedTransaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(e) => {
            return response::CheckTx {
                code: AbciCode::INVALID_PARAMETER.into(),
                info: "the provided bytes was not a valid protobuf-encoded SignedTransaction, or \
                       the signature was invalid"
                    .into(),
                log: format!("{e:?}"),
                ..response::CheckTx::default()
            };
        }
    };

    // if the tx passes the check, status code 0 is returned.
    // TODO(https://github.com/astriaorg/astria/issues/228): status codes for various errors
    // TODO(https://github.com/astriaorg/astria/issues/319): offload `check_stateless` using `deliver_tx_bytes` mechanism
    //       and a worker task similar to penumbra
    if let Err(e) = transaction::check_nonce_mempool(&signed_tx, &state).await {
        return response::CheckTx {
            code: AbciCode::INVALID_NONCE.into(),
            info: "failed verifying transaction nonce".into(),
            log: format!("{e:?}"),
            ..response::CheckTx::default()
        };
    };

    match transaction::check_stateless(&signed_tx).await {
        Ok(()) => response::CheckTx::default(),
        Err(e) => response::CheckTx {
            code: AbciCode::INVALID_PARAMETER.into(),
            info: "transaction failed stateless check".into(),
            log: format!("{e:?}"),
            ..response::CheckTx::default()
        },
    }
}
