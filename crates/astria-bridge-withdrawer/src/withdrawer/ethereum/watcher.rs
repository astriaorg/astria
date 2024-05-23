use std::sync::Arc;

use astria_core::{
    primitive::v1::asset,
    protocol::transaction::v1alpha1::Action,
};
use astria_eyre::{
    eyre::{
        self,
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
    batch::{
        event_to_action,
        Batch,
        EventWithMetadata,
    },
    state::State,
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
        batch_tx: mpsc::Sender<(Vec<Action>, u64)>,
        shutdown_token: &CancellationToken,
        state: Arc<State>,
        fee_asset_id: asset::Id,
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

        // TODO: verify fee_asset_id against sequencer
        let batcher = Batcher::new(event_rx, batch_tx, shutdown_token, fee_asset_id);
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
    pub(crate) async fn run(mut self) -> Result<()> {
        let batcher = self.batcher.take().expect("batcher must be present");
        tokio::task::spawn(batcher.run());

        self.state.set_watcher_ready();

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

struct Batcher {
    event_rx: mpsc::Receiver<(WithdrawalFilter, LogMeta)>,
    batch_tx: mpsc::Sender<Batch>,
    shutdown_token: CancellationToken,
    fee_asset_id: asset::Id,
}

impl Batcher {
    pub(crate) fn new(
        event_rx: mpsc::Receiver<(WithdrawalFilter, LogMeta)>,
        batch_tx: mpsc::Sender<Batch>,
        shutdown_token: &CancellationToken,
        fee_asset_id: asset::Id,
    ) -> Self {
        Self {
            event_rx,
            batch_tx,
            shutdown_token: shutdown_token.clone(),
            fee_asset_id,
        }
    }
}

impl Batcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let mut curr_batch = Batch {
            actions: Vec::new(),
            rollup_height: 0,
        };

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
                        let action = event_to_action(event_with_metadata, self.fee_asset_id)?;

                        if meta.block_number.into() == curr_batch.rollup_height {
                            // block number was the same; add event to current batch
                            curr_batch.actions.push(action);
                        } else {
                            // block number increased; send current batch and start a new one
                            if !curr_batch.actions.is_empty() {
                                self.batch_tx
                                    .send(curr_batch)
                                    .await
                                    .wrap_err("failed to send batched events; receiver dropped?")?;
                            }

                            curr_batch = Batch {
                                actions: vec![action],
                                rollup_height = meta.block_number.into()
                            };
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
    use crate::withdrawer::{
        batch::EventWithMetadata,
        ethereum::test_utils::deploy_astria_withdrawer,
    };

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
        let expected_action =
            event_to_action(expected_event, asset::Id::from_denom("nria")).unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!(
                "expected action to be BridgeUnlock, got {:?}",
                expected_action
            );
        };

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
            Arc::new(State::new()),
            asset::Id::from_denom("nria"),
        )
        .await
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_withdraw_transaction(&contract, value).await;

        let (events, _rollup_height) = event_rx.recv().await.unwrap();
        assert_eq!(events.len(), 1);
        let Action::BridgeUnlock(action) = &events[0] else {
            panic!("expected action to be BridgeUnlock, got {:?}", events[0]);
        };
        assert_eq!(action, &expected_action);
    }
}
