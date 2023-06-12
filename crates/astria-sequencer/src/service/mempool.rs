use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use futures::Future;
use tendermint::abci::{
    response,
    MempoolRequest,
    MempoolResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::info;

use crate::transaction::{
    ActionHandler as _,
    Transaction,
};

/// Mempool handles one request: CheckTx.
/// It performs a stateless check of the given transaction,
/// returning an abci::response::CheckTx.
#[derive(Clone, Default)]
pub struct Mempool;

impl Service<MempoolRequest> for Mempool {
    type Error = BoxError;
    type Future = MempoolFuture;
    type Response = MempoolResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MempoolRequest) -> Self::Future {
        info!("got mempool request: {:?}", req);
        MempoolFuture::new(req)
    }
}

pub struct MempoolFuture {
    request: MempoolRequest,
}

impl MempoolFuture {
    pub fn new(request: MempoolRequest) -> Self {
        Self {
            request,
        }
    }
}

impl Future for MempoolFuture {
    type Output = Result<MempoolResponse, BoxError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.request {
            MempoolRequest::CheckTx(req) => {
                // if the tx passes the check, status code 0 is returned.
                // TODO: status codes for various errors
                match Transaction::from_bytes(&req.tx) {
                    Ok(tx) => match tx.check_stateless() {
                        Ok(_) => {
                            Poll::Ready(Ok(MempoolResponse::CheckTx(response::CheckTx::default())))
                        }
                        Err(e) => Poll::Ready(Ok(MempoolResponse::CheckTx(response::CheckTx {
                            code: 1.into(),
                            log: format!("{:?}", e),
                            ..Default::default()
                        }))),
                    },
                    Err(e) => Poll::Ready(Ok(MempoolResponse::CheckTx(response::CheckTx {
                        code: 1.into(),
                        log: format!("{:?}", e),
                        ..Default::default()
                    }))),
                }
            }
        }
    }
}
