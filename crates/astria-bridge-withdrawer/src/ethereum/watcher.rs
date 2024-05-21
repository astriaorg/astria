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
use tokio::sync::mpsc;

use crate::ethereum::astria_withdrawer::{
    astria_withdrawer::WithdrawalFilter,
    AstriaWithdrawer,
};

/// Watches for withdrawal events emitted by the `AstriaWithdrawer` contract.
pub(crate) struct Watcher {
    contract: AstriaWithdrawer<Provider<Ws>>,
    event_tx: mpsc::Sender<(WithdrawalFilter, LogMeta)>,
    batcher: Option<Batcher>,
}

impl Watcher {
    pub(crate) async fn new(
        ethereum_contract_address: &str,
        ethereum_rpc_endpoint: &str,
        event_with_metadata_tx: mpsc::Sender<Vec<EventWithMetadata>>,
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
        let batcher = Batcher::new(event_rx, event_with_metadata_tx);
        Ok(Self {
            contract,
            event_tx,
            batcher: Some(batcher),
        })
    }
}

impl Watcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let batcher = self.batcher.take().expect("batcher must be present");
        tokio::task::spawn(batcher.run());

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

        while let Some(Ok((event, meta))) = stream.next().await {
            self.event_tx
                .send((event, meta))
                .await
                .wrap_err("failed to send withdrawal event; receiver dropped?")?;
        }

        Ok(())
    }
}

fn address_from_string(s: &str) -> Result<ethers::types::Address> {
    let bytes = hex::decode(s).wrap_err("failed to parse ethereum address as hex")?;
    let address: [u8; 20] = bytes
        .try_into()
        .map_err(|_| eyre!("invalid length for ethereum address, must be 20 bytes"))?;
    Ok(address.into())
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
    event_with_metadata_tx: mpsc::Sender<Vec<EventWithMetadata>>,
}

impl Batcher {
    pub(crate) fn new(
        event_rx: mpsc::Receiver<(WithdrawalFilter, LogMeta)>,
        event_with_metadata_tx: mpsc::Sender<Vec<EventWithMetadata>>,
    ) -> Self {
        Self {
            event_rx,
            event_with_metadata_tx,
        }
    }
}

impl Batcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let mut events = Vec::new();
        let mut last_block_number: U64 = 0.into();

        while let Some((event, meta)) = self.event_rx.recv().await {
            let event_with_metadata = EventWithMetadata {
                event,
                block_number: meta.block_number,
                transaction_hash: meta.transaction_hash,
            };

            if meta.block_number != last_block_number {
                // block number increased; send current batch and start a new one
                if !events.is_empty() {
                    self.event_with_metadata_tx
                        .send(events)
                        .await
                        .wrap_err("failed to send batched events; receiver dropped?")?;
                }

                events = vec![event_with_metadata];
                last_block_number = meta.block_number;
            } else {
                // block number was the same; add event to current batch
                events.push(event_with_metadata);
            }
        }

        Ok(())
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
    use crate::ethereum::test_utils::deploy_astria_withdrawer;

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
            "`withdraw` transaction failed: {:?}",
            receipt
        );

        receipt
    }

    #[tokio::test]
    async fn watcher_can_watch() {
        let (contract_address, provider, wallet, anvil) = deploy_astria_withdrawer().await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value = 1000000000.into();
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

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
        )
        .await
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_withdraw_transaction(&contract, value).await;

        let events = event_rx.recv().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], expected_event);
    }
}
