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

use crate::transaction::Transaction;

/// MempoolService handles one request: CheckTx.
/// It performs a stateless check of the given transaction,
/// returning an abci::response::CheckTx.
#[derive(Clone, Default)]
pub struct MempoolService {}

impl MempoolService {
    pub fn new() -> Self {
        Self {}
    }
}

impl Service<MempoolRequest> for MempoolService {
    type Error = BoxError;
    type Future = MempoolServiceFuture;
    type Response = MempoolResponse;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MempoolRequest) -> Self::Future {
        info!("got mempool request: {:?}", req);
        MempoolServiceFuture::new(req)
    }
}

pub struct MempoolServiceFuture {
    request: MempoolRequest,
}

impl MempoolServiceFuture {
    pub fn new(request: MempoolRequest) -> Self {
        Self {
            request,
        }
    }
}

impl Future for MempoolServiceFuture {
    type Output = Result<MempoolResponse, BoxError>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.request {
            MempoolRequest::CheckTx(req) => {
                // if the tx passes the check, status code 0 is returned.
                // TODO: status codes for various errors
                match Transaction::from_bytes(&req.tx) {
                    Ok(tx) => match tx.check_stateless() {
                        Ok(_) => Poll::Ready(Ok(MempoolResponse::CheckTx(response::CheckTx {
                            code: 0.into(),
                            ..Default::default()
                        }))),
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
