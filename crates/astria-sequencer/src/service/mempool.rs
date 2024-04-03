use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use astria_core::{
    generated::sequencer::v1 as raw,
    sequencer::v1::{
        AbciErrorCode,
        SignedTransaction,
    },
};
use cnidarium::Storage;
use futures::{
    Future,
    FutureExt,
};
use prost::Message as _;
use tendermint::v0_37::abci::{
    request,
    response,
    MempoolRequest,
    MempoolResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::Instrument;

use crate::{
    accounts::state_ext::StateReadExt,
    transaction,
};

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

/// Handles a [`request::CheckTx`] request.
///
/// Performs stateless checks (decoding and signature check),
/// as well as stateful checks (nonce and balance checks).
///
/// If the tx passes all checks, status code 0 is returned.
async fn handle_check_tx<S: StateReadExt + 'static>(
    req: request::CheckTx,
    state: S,
) -> response::CheckTx {
    let request::CheckTx {
        tx, ..
    } = req;
    if tx.len() > MAX_TX_SIZE {
        return response::CheckTx {
            code: AbciErrorCode::TRANSACTION_TOO_LARGE.into(),
            log: format!(
                "transaction size too large; allowed: {MAX_TX_SIZE} bytes, got {}",
                tx.len()
            ),
            info: AbciErrorCode::TRANSACTION_TOO_LARGE.to_string(),
            ..response::CheckTx::default()
        };
    }

    let raw_signed_tx = match raw::SignedTransaction::decode(tx) {
        Ok(tx) => tx,
        Err(e) => {
            return response::CheckTx {
                code: AbciErrorCode::INVALID_PARAMETER.into(),
                log: e.to_string(),
                info: "failed decoding bytes as a protobuf SignedTransaction".into(),
                ..response::CheckTx::default()
            };
        }
    };
    let signed_tx = match SignedTransaction::try_from_raw(raw_signed_tx) {
        Ok(tx) => tx,
        Err(e) => {
            return response::CheckTx {
                code: AbciErrorCode::INVALID_PARAMETER.into(),
                info: "the provided bytes was not a valid protobuf-encoded SignedTransaction, or \
                       the signature was invalid"
                    .into(),
                log: e.to_string(),
                ..response::CheckTx::default()
            };
        }
    };

    if let Err(e) = transaction::check_stateless(&signed_tx).await {
        return response::CheckTx {
            code: AbciErrorCode::INVALID_PARAMETER.into(),
            info: "transaction failed stateless check".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    if let Err(e) = transaction::check_nonce_mempool(&signed_tx, &state).await {
        return response::CheckTx {
            code: AbciErrorCode::INVALID_NONCE.into(),
            info: "failed verifying transaction nonce".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    if let Err(e) = transaction::check_balance_mempool(&signed_tx, &state).await {
        return response::CheckTx {
            code: AbciErrorCode::INSUFFICIENT_FUNDS.into(),
            info: "failed verifying account balance".into(),
            log: e.to_string(),
            ..response::CheckTx::default()
        };
    };

    response::CheckTx::default()
}
