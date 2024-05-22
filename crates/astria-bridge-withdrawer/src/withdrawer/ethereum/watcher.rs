use std::sync::Arc;

use astria_core::protocol::transaction::v1alpha1::Action;
use astria_eyre::{
    eyre::{
        eyre,
        WrapErr as _,
    },
    Result,
};
use ethers::{
    contract::LogMeta,
    providers::{
        Provider,
        StreamExt as _,
        Ws,
    },
    types::{
        TxHash,
        U64,
    },
    utils::hex,
};
use tokio::{
    select,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use tracing::info;

use super::astria_withdrawer::{
    astria_withdrawer::WithdrawalFilter,
    AstriaWithdrawer,
};
use crate::withdrawer::{
    state::State,
    StateSnapshot,
};

/// Watches for withdrawal events emitted by the `AstriaWithdrawer` contract.
pub(crate) struct Watcher {
    contract: AstriaWithdrawer<Provider<Ws>>,
    event_tx: mpsc::Sender<(WithdrawalFilter, LogMeta)>,
    batcher: Option<Batcher>,
    state: Arc<State>,
    shutdown_token: CancellationToken,
}

impl Watcher {
    pub(crate) async fn new(
        ethereum_contract_address: &str,
        ethereum_rpc_endpoint: &str,
        batch_tx: mpsc::Sender<Vec<Action>>,
        shutdown_token: &CancellationToken,
        state: Arc<State>,
    ) -> Result<Self> {
        let provider = Arc::new(
            Provider::<Ws>::connect(ethereum_rpc_endpoint)
                .await
                .wrap_err("failed to connect to ethereum RPC endpoint")?,
        );
        let contract_address = address_from_string(ethereum_contract_address)
            .wrap_err("failed to parse ethereum contract address")?;
        let contract = AstriaWithdrawer::new(contract_address, provider);

        let (event_tx, event_rx) = mpsc::channel(100);
        let batcher = Batcher::new(event_rx, batch_tx, shutdown_token);
        Ok(Self {
            contract,
            event_tx,
            batcher: Some(batcher),
            state,
            shutdown_token: shutdown_token.clone(),
        })
    }
}

impl Watcher {
    pub(crate) fn subscribe_to_state(&self) -> tokio::sync::watch::Receiver<StateSnapshot> {
        self.state.subscribe()
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let batcher = self.batcher.take().expect("batcher must be present");
        tokio::task::spawn(batcher.run());

        self.state.set_ready();

        // start from block 1 right now
        // TODO: determine the last block we've seen based on the sequencer data
        self.watch_for_withdrawal_events(1).await?;
        Ok(())
    }

    async fn watch_for_withdrawal_events(&self, from_block: u64) -> Result<()> {
        let events = self
            .contract
            .withdrawal_filter()
            .from_block(from_block)
            .address(self.contract.address().into());

        let mut stream = events.stream().await.unwrap().with_meta();

        loop {
            select! {
                () = self.shutdown_token.cancelled() => {
                    info!("watcher shutting down");
                    break;
                }
                item = stream.next() => {
                    if let Some(Ok((event, meta))) = item {
                        self.event_tx
                            .send((event, meta))
                            .await
                            .wrap_err("failed to send withdrawal event; receiver dropped?")?;
                    } else if let Some(Err(e)) = item {
                        return Err(e).wrap_err("failed to read from event stream; event stream closed?");
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EventWithMetadata {
    event: WithdrawalFilter,
    /// The block in which the log was emitted
    block_number: U64,
    /// The transaction hash in which the log was emitted
    transaction_hash: TxHash,
}

struct Batcher {
    event_rx: mpsc::Receiver<(WithdrawalFilter, LogMeta)>,
    batch_tx: mpsc::Sender<Vec<Action>>,
    shutdown_token: CancellationToken,
}

impl Batcher {
    pub(crate) fn new(
        event_rx: mpsc::Receiver<(WithdrawalFilter, LogMeta)>,
        batch_tx: mpsc::Sender<Vec<Action>>,
        shutdown_token: &CancellationToken,
    ) -> Self {
        Self {
            event_rx,
            batch_tx,
            shutdown_token: shutdown_token.clone(),
        }
    }
}

impl Batcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let mut actions = Vec::new();
        let mut last_block_number: U64 = 0.into();

        loop {
            select! {
                () = self.shutdown_token.cancelled() => {
                    info!("batcher shutting down");
                    break;
                }
                item = self.event_rx.recv() => {
                    if let Some((event, meta)) = item {
                        let event_with_metadata = EventWithMetadata {
                            event,
                            block_number: meta.block_number,
                            transaction_hash: meta.transaction_hash,
                        };
                        let action = Action::from(event_with_metadata);

                        if meta.block_number == last_block_number {
                            // block number was the same; add event to current batch
                            actions.push(action);
                        } else {
                            // block number increased; send current batch and start a new one
                            if !actions.is_empty() {
                                self.batch_tx
                                    .send(actions)
                                    .await
                                    .wrap_err("failed to send batched events; receiver dropped?")?;
                            }

                            actions = vec![action];
                            last_block_number = meta.block_number;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// converts an ethereum address string to an `ethers::types::Address`.
// the input string may be prefixed with "0x" or not.
fn address_from_string(s: &str) -> Result<ethers::types::Address> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).wrap_err("failed to parse ethereum address as hex")?;
    let address: [u8; 20] = bytes
        .try_into()
        .map_err(|_| eyre!("invalid length for ethereum address, must be 20 bytes"))?;
    Ok(address.into())
}

impl From<EventWithMetadata> for Action {
    fn from(event_with_metadata: EventWithMetadata) -> Self {
        todo!("implement action conversion logic");
    }
}

#[cfg(test)]
mod tests {
    use ethers::{
        prelude::SignerMiddleware,
        providers::Middleware,
        signers::Signer as _,
        types::{
            TransactionReceipt,
            U256,
        },
        utils::hex,
    };

    use super::*;
    use crate::withdrawer::ethereum::test_utils::deploy_astria_withdrawer;

    #[test]
    fn address_from_string_prefix() {
        let address = address_from_string("0x1234567890123456789012345678901234567890").unwrap();
        let bytes: [u8; 20] = hex::decode("1234567890123456789012345678901234567890")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(address, ethers::types::Address::from(bytes));
    }

    #[test]
    fn address_from_string_no_prefix() {
        let address = address_from_string("1234567890123456789012345678901234567890").unwrap();
        let bytes: [u8; 20] = hex::decode("1234567890123456789012345678901234567890")
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(address, ethers::types::Address::from(bytes));
    }

    async fn send_withdraw_transaction<M: Middleware>(
        contract: &AstriaWithdrawer<M>,
        value: U256,
    ) -> TransactionReceipt {
        let tx = contract.withdraw(b"nootwashere".into()).value(value);
        let receipt = tx
            .send()
            .await
            .expect("failed to submit transaction")
            .await
            .expect("failed to await pending transaction")
            .expect("no receipt found");

        assert!(
            receipt.status == Some(ethers::types::U64::from(1)),
            "`withdraw` transaction failed: {receipt:?}",
        );

        receipt
    }

    #[tokio::test]
    async fn watcher_can_watch() {
        let (contract_address, provider, wallet, anvil) = deploy_astria_withdrawer().await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value = 1_000_000_000.into();
        let receipt = send_withdraw_transaction(&contract, value).await;
        let expected_event = EventWithMetadata {
            event: WithdrawalFilter {
                sender: wallet.address(),
                amount: value,
                memo: b"nootwashere".into(),
            },
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };
        let expected_action = Action::from(expected_event);

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
            Arc::new(State::new()),
        )
        .await
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_withdraw_transaction(&contract, value).await;

        let events = event_rx.recv().await.unwrap();
        assert_eq!(events.len(), 1);
        // assert_eq!(events[0], expected_action);
    }
}
