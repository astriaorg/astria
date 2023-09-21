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
use tendermint::abci::{
    request,
    response,
    MempoolRequest,
    MempoolResponse,
};
use tower::Service;
use tower_abci::BoxError;
use tracing::Instrument;

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
        use penumbra_tower_trace::RequestExt as _;
        let span = req.create_span();
        async move {
            let rsp = match req {
                MempoolRequest::CheckTx(req) => MempoolResponse::CheckTx(handle_check_tx(req)),
            };
            Ok(rsp)
        }
        .instrument(span)
        .boxed()
    }
}

fn handle_check_tx(req: request::CheckTx) -> response::CheckTx {
    use proto::{
        generated::sequencer::v1alpha1 as raw,
        native::sequencer::v1alpha1::SignedTransaction,
        Message as _,
    };
    use sequencer_types::abci_codes::AbciCode;

    use crate::transaction;

    let request::CheckTx {
        tx, ..
    } = req;
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
    let signed_tx = SignedTransaction::try_from_raw(raw_signed_tx).unwrap();
    // if the tx passes the check, status code 0 is returned.
    // TODO(https://github.com/astriaorg/astria/issues/228): status codes for various errors
    // TODO(https://github.com/astriaorg/astria/issues/319): offload `check_stateless` using `deliver_tx_bytes` mechanism
    //       and a worker task similar to penumbra
    match transaction::check_stateless(&signed_tx) {
        Ok(_) => response::CheckTx::default(),
        Err(e) => response::CheckTx {
            code: AbciCode::INVALID_PARAMETER.into(),
            info: "failed verifying decoded protobuf SignedTransaction".into(),
            log: format!("{e:?}"),
            ..response::CheckTx::default()
        },
    }
}
