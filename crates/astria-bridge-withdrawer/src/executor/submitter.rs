use std::{
    pin::Pin,
    task::Poll,
};

use astria_eyre::eyre;
use futures::{
    ready,
    Future,
};
use pin_project_lite::pin_project;
use sequencer_client::{
    tendermint_rpc::endpoint::broadcast::tx_commit,
    SignedTransaction,
};

use super::signer::SequencerSigner;

pin_project! {
    /// TODO
    pub(super) struct SubmitFut {
        tx: SignedTransaction,
        signer: SequencerSigner,
        cometbft_client: sequencer_client::HttpClient,
        nonce: u32,
        sequencer_chain_id: String,
        #[pin]
        state: SubmitState,
    }
}

pin_project! {
    #[project = SubmitStateProj]
    enum SubmitState {
        NotStarted,
        WaitingForTxCommit {
            #[pin]
            fut: Pin<Box<dyn Future<Output = eyre::Result<tx_commit::Response>> + Send>>
        },
        WaitingForNonce {
            #[pin]
            fut: Pin<Box<dyn Future<Output = eyre::Result<u32>> + Send>>
        }
    }
}

impl Future for SubmitFut {
    // TODO: output?
    type Output = eyre::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();
            let new_state = match this.state.project() {
                SubmitStateProj::NotStarted => {
                    // TODO: broadcast tx commit and change to waiting for txcommit
                    SubmitState::WaitingForTxCommit {
                        fut: todo!("broadcast tx commit"),
                    }
                }
                SubmitStateProj::WaitingForTxCommit {
                    fut,
                } => match ready!(fut.poll(cx)) {
                    Ok(rsp) => {
                        // handle 0 code
                        // handle non-zero code
                        // if invalid nonce, change to waiting for nonce
                        SubmitState::WaitingForNonce {
                            fut: todo!("nonce fetch"),
                        }
                    }
                    Err(e) => {
                        // exit with an error
                        return todo!("exit with an error");
                    }
                },
                SubmitStateProj::WaitingForNonce {
                    fut,
                } => match ready!(fut.poll(cx)) {
                    Ok(nonce) => {
                        // update tx nonce
                        // change to waiting for txcommit
                        SubmitState::WaitingForTxCommit {
                            fut: todo!("broadcast tx commit"),
                        }
                    }
                    Err(e) => {
                        // exit with an error
                        return todo!("exit with an error");
                    }
                },
            };
            self.as_mut().project().state.set(new_state);
        }
    }
}
