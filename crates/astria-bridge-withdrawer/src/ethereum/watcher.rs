use std::sync::Arc;

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

use crate::ethereum::{
    astria_withdrawer::{
        astria_withdrawer::{
            Ics20WithdrawalFilter,
            SequencerWithdrawalFilter,
        },
        AstriaWithdrawer,
    },
    state::{
        State,
        StateSnapshot,
    },
};

/// Watches for withdrawal events emitted by the `AstriaWithdrawer` contract.
pub(crate) struct Watcher {
    contract: AstriaWithdrawer<Provider<Ws>>,
    event_tx: mpsc::Sender<(WithdrawalEvent, LogMeta)>,
    batcher: Batcher,
    state: Arc<State>,
    shutdown_token: CancellationToken,
}

impl Watcher {
    pub(crate) async fn new(
        ethereum_contract_address: &str,
        ethereum_rpc_endpoint: &str,
        event_with_metadata_tx: mpsc::Sender<Vec<EventWithMetadata>>,
        shutdown_token: &CancellationToken,
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
        let batcher = Batcher::new(event_rx, event_with_metadata_tx, shutdown_token);
        let state = Arc::new(State::new());
        Ok(Self {
            contract,
            event_tx,
            batcher,
            state,
            shutdown_token: shutdown_token.clone(),
        })
    }
}

impl Watcher {
    pub(crate) fn subscribe_to_state(&self) -> tokio::sync::watch::Receiver<StateSnapshot> {
        self.state.subscribe()
    }

    pub(crate) async fn run(self) -> Result<()> {
        let Watcher {
            contract,
            event_tx,
            batcher,
            state,
            shutdown_token,
        } = self;

        tokio::task::spawn(batcher.run());

        state.set_ready();

        // start from block 1 right now
        // TODO: determine the last block we've seen based on the sequencer data
        let sequencer_withdrawal_event_handler = tokio::task::spawn(
            watch_for_sequencer_withdrawal_events(contract.clone(), event_tx.clone(), 1),
        );
        let ics20_withdrawal_event_handler = tokio::task::spawn(watch_for_ics20_withdrawal_events(
            contract,
            event_tx.clone(),
            1,
        ));

        tokio::select! {
            res = sequencer_withdrawal_event_handler => {
                info!("sequencer withdrawal event handler exited");
                res.context("sequencer withdrawal event handler exited")?
            }
            res = ics20_withdrawal_event_handler => {
                info!("ics20 withdrawal event handler exited");
                res.context("ics20 withdrawal event handler exited")?
            }
            () = shutdown_token.cancelled() => {
                info!("watcher shutting down");
                Ok(())
            }
        }
    }
}

async fn watch_for_sequencer_withdrawal_events(
    contract: AstriaWithdrawer<Provider<Ws>>,
    event_tx: mpsc::Sender<(WithdrawalEvent, LogMeta)>,
    from_block: u64,
) -> Result<()> {
    let events = contract
        .sequencer_withdrawal_filter()
        .from_block(from_block)
        .address(contract.address().into());

    let mut stream = events.stream().await.unwrap().with_meta();

    while let Some(item) = stream.next().await {
        if let Ok((event, meta)) = item {
            event_tx
                .send((WithdrawalEvent::Sequencer(event), meta))
                .await
                .wrap_err("failed to send sequencer withdrawal event; receiver dropped?")?;
        } else if let Err(e) = item {
            return Err(e).wrap_err("failed to read from event stream; event stream closed?");
        }
    }

    Ok(())
}

async fn watch_for_ics20_withdrawal_events(
    contract: AstriaWithdrawer<Provider<Ws>>,
    event_tx: mpsc::Sender<(WithdrawalEvent, LogMeta)>,
    from_block: u64,
) -> Result<()> {
    let events = contract
        .ics_20_withdrawal_filter()
        .from_block(from_block)
        .address(contract.address().into());

    let mut stream = events.stream().await.unwrap().with_meta();

    while let Some(item) = stream.next().await {
        if let Ok((event, meta)) = item {
            event_tx
                .send((WithdrawalEvent::Ics20(event), meta))
                .await
                .wrap_err("failed to send ics20 withdrawal event; receiver dropped?")?;
        } else if let Err(e) = item {
            return Err(e).wrap_err("failed to read from event stream; event stream closed?");
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WithdrawalEvent {
    Sequencer(SequencerWithdrawalFilter),
    Ics20(Ics20WithdrawalFilter),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EventWithMetadata {
    event: WithdrawalEvent,
    /// The block in which the log was emitted
    block_number: U64,
    /// The transaction hash in which the log was emitted
    transaction_hash: TxHash,
}

struct Batcher {
    event_rx: mpsc::Receiver<(WithdrawalEvent, LogMeta)>,
    event_with_metadata_tx: mpsc::Sender<Vec<EventWithMetadata>>,
    shutdown_token: CancellationToken,
}

impl Batcher {
    pub(crate) fn new(
        event_rx: mpsc::Receiver<(WithdrawalEvent, LogMeta)>,
        event_with_metadata_tx: mpsc::Sender<Vec<EventWithMetadata>>,
        shutdown_token: &CancellationToken,
    ) -> Self {
        Self {
            event_rx,
            event_with_metadata_tx,
            shutdown_token: shutdown_token.clone(),
        }
    }
}

impl Batcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let mut events = Vec::new();
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

                        if meta.block_number == last_block_number {
                            // block number was the same; add event to current batch
                            events.push(event_with_metadata);
                        } else {
                            // block number increased; send current batch and start a new one
                            if !events.is_empty() {
                                self.event_with_metadata_tx
                                    .send(events)
                                    .await
                                    .wrap_err("failed to send batched events; receiver dropped?")?;
                            }

                            events = vec![event_with_metadata];
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
    use crate::ethereum::test_utils::deploy_astria_withdrawer;

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

    async fn send_sequencer_withdraw_transaction<M: Middleware>(
        contract: &AstriaWithdrawer<M>,
        value: U256,
        recipient: ethers::types::Address,
    ) -> TransactionReceipt {
        let tx = contract.withdraw_to_sequencer(recipient).value(value);
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
    async fn watcher_can_watch_sequencer_withdrawals() {
        let (contract_address, provider, wallet, anvil) = deploy_astria_withdrawer().await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value = 1_000_000_000.into();
        let recipient = [0u8; 20].into();
        let receipt = send_sequencer_withdraw_transaction(&contract, value, recipient).await;
        let expected_event = EventWithMetadata {
            event: WithdrawalEvent::Sequencer(SequencerWithdrawalFilter {
                sender: wallet.address(),
                destination_chain_address: recipient,
                amount: value,
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
        )
        .await
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_sequencer_withdraw_transaction(&contract, value, recipient).await;

        let events = event_rx.recv().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], expected_event);
    }

    async fn send_ics20_withdraw_transaction<M: Middleware>(
        contract: &AstriaWithdrawer<M>,
        value: U256,
        recipient: String,
    ) -> TransactionReceipt {
        let tx = contract
            .withdraw_to_origin_chain(recipient, b"nootwashere".into())
            .value(value);
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
    async fn watcher_can_watch_ics20_withdrawals() {
        let (contract_address, provider, wallet, anvil) = deploy_astria_withdrawer().await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value = 1_000_000_000.into();
        let recipient = "somebech32address".to_string();
        let receipt = send_ics20_withdraw_transaction(&contract, value, recipient.clone()).await;
        let expected_event = EventWithMetadata {
            event: WithdrawalEvent::Ics20(Ics20WithdrawalFilter {
                sender: wallet.address(),
                destination_chain_address: recipient.clone(),
                amount: value,
                memo: b"nootwashere".into(),
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
        )
        .await
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_ics20_withdraw_transaction(&contract, value, recipient).await;

        let events = event_rx.recv().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], expected_event);
    }
}
