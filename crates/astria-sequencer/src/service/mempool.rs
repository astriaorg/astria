use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use futures::{
    Future,
    FutureExt,
};
use penumbra_tower_trace::RequestExt as _;
use tendermint::abci::{
    request,
    response,
    MempoolRequest,
    MempoolResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::Instrument;

use crate::transaction::Signed;

/// Mempool handles [`request::CheckTx`] abci requests.
//
/// It performs a stateless check of the given transaction,
/// returning a [`tendermint::abci::response::CheckTx`].
#[derive(Clone, Default)]
pub(crate) struct Mempool;

impl Service<MempoolRequest> for Mempool {
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<MempoolResponse, BoxError>> + Send + 'static>>;
    type Response = MempoolResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MempoolRequest) -> Self::Future {
        let span = req.create_span();
        let MempoolRequest::CheckTx(request::CheckTx {
            tx: tx_bytes, ..
        }) = req;
        async move {
            // if the tx passes the check, status code 0 is returned.
            // TODO: status codes for various errors
            // TODO: offload `check_stateless` using `deliver_tx_bytes` mechanism
            //       and a worker task similar to penumbra
            let tx = match Signed::try_from_slice(&tx_bytes) {
                Ok(tx) => tx,
                Err(e) => {
                    return Ok(MempoolResponse::CheckTx(response::CheckTx {
                        code: 1.into(),
                        log: format!("{e:#}"),
                        ..Default::default()
                    }));
                }
            };

            let rsp = match tx.check_stateless() {
                Ok(_) => response::CheckTx::default(),
                Err(e) => response::CheckTx {
                    code: 1.into(),
                    log: format!("{e:#}"),
                    ..Default::default()
                },
            };
            Ok(MempoolResponse::CheckTx(rsp))
        }
        .instrument(span)
        .boxed()
    }
}
