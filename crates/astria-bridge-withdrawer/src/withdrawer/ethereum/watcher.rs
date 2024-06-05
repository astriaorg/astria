use std::sync::Arc;

use astria_core::primitive::v1::{
    asset,
    asset::Denom,
};
use astria_eyre::{
    eyre::{
        eyre,
        WrapErr as _,
    },
    Result,
};
use ethers::{
    contract::LogMeta,
    core::types::Block,
    providers::{
        Middleware,
        Provider,
        StreamExt as _,
        Ws,
    },
    utils::hex,
};
use tokio::{
    select,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    warn,
};

use crate::withdrawer::{
    batch::Batch,
    ethereum::{
        astria_withdrawer_interface::IAstriaWithdrawer,
        convert::{
            event_to_action,
            EventWithMetadata,
            WithdrawalEvent,
        },
    },
    state::State,
};

/// Watches for withdrawal events emitted by the `AstriaWithdrawer` contract.
pub(crate) struct Watcher {
    // contract: AstriaWithdrawer<Provider<Ws>>,
    contract_address: ethers::types::Address,
    ethereum_rpc_endpoint: String,
    batch_tx: mpsc::Sender<Batch>,
    fee_asset_id: asset::Id,
    rollup_asset_denom: Denom,
    state: Arc<State>,
    shutdown_token: CancellationToken,
}

impl Watcher {
    pub(crate) fn new(
        ethereum_contract_address: &str,
        ethereum_rpc_endpoint: &str,
        batch_tx: mpsc::Sender<Batch>,
        shutdown_token: &CancellationToken,
        state: Arc<State>,
        fee_asset_id: asset::Id,
        rollup_asset_denom: Denom,
    ) -> Result<Self> {
        let contract_address = address_from_string(ethereum_contract_address)
            .wrap_err("failed to parse ethereum contract address")?;

        if rollup_asset_denom.prefix().is_empty() {
            warn!(
                "rollup asset denomination is not prefixed; Ics20Withdrawal actions will not be \
                 submitted"
            );
        }

        Ok(Self {
            contract_address,
            ethereum_rpc_endpoint: ethereum_rpc_endpoint.to_string(),
            batch_tx,
            fee_asset_id,
            rollup_asset_denom,
            state,
            shutdown_token: shutdown_token.clone(),
        })
    }
}

impl Watcher {
    pub(crate) async fn run(self) -> Result<()> {
        let Watcher {
            contract_address,
            ethereum_rpc_endpoint,
            batch_tx,
            fee_asset_id,
            rollup_asset_denom,
            state,
            shutdown_token,
        } = self;

        let (event_tx, event_rx) = mpsc::channel(100);

        let provider = Arc::new(
            Provider::<Ws>::connect(ethereum_rpc_endpoint)
                .await
                .wrap_err("failed to connect to ethereum RPC endpoint")?,
        );
        let contract = IAstriaWithdrawer::new(contract_address, provider.clone());

        let base_chain_asset_precision = contract
            .base_chain_asset_precision()
            .call()
            .await
            .wrap_err("failed to get asset withdrawal decimals")?;
        let asset_withdrawal_divisor =
            10u128.pow(18u32.checked_sub(base_chain_asset_precision).expect(
                "base_chain_asset_precision must be <= 18, as the contract constructor enforces \
                 this",
            ));

        let batcher = Batcher::new(
            event_rx,
            provider,
            batch_tx,
            &shutdown_token,
            fee_asset_id,
            rollup_asset_denom,
            asset_withdrawal_divisor,
        );

        tokio::task::spawn(batcher.run());

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

        state.set_watcher_ready();

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
    contract: IAstriaWithdrawer<Provider<Ws>>,
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
        } else if item.is_err() {
            item.wrap_err("failed to read from event stream; event stream closed?")?;
        }
    }

    Ok(())
}

async fn watch_for_ics20_withdrawal_events(
    contract: IAstriaWithdrawer<Provider<Ws>>,
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
        } else if item.is_err() {
            item.wrap_err("failed to read from event stream; event stream closed?")?;
        }
    }

    Ok(())
}

struct Batcher {
    event_rx: mpsc::Receiver<(WithdrawalEvent, LogMeta)>,
    provider: Arc<Provider<Ws>>,
    batch_tx: mpsc::Sender<Batch>,
    shutdown_token: CancellationToken,
    fee_asset_id: asset::Id,
    rollup_asset_denom: Denom,
    asset_withdrawal_divisor: u128,
}

impl Batcher {
    pub(crate) fn new(
        event_rx: mpsc::Receiver<(WithdrawalEvent, LogMeta)>,
        provider: Arc<Provider<Ws>>,
        batch_tx: mpsc::Sender<Batch>,
        shutdown_token: &CancellationToken,
        fee_asset_id: asset::Id,
        rollup_asset_denom: Denom,
        asset_withdrawal_divisor: u128,
    ) -> Self {
        Self {
            event_rx,
            provider,
            batch_tx,
            shutdown_token: shutdown_token.clone(),
            fee_asset_id,
            rollup_asset_denom,
            asset_withdrawal_divisor,
        }
    }

    pub(crate) async fn run(mut self) -> Result<()> {
        let mut block_rx = self
            .provider
            .subscribe_blocks()
            .await
            .wrap_err("failed to subscribe to blocks")?;

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
                block = block_rx.next() => {
                    if let Some(Block { number, .. }) = block {
                        let Some(block_number) = number else {
                            // don't think this should happen
                            warn!("block number missing; skipping");
                            continue;
                        };

                        if block_number.as_u64() > curr_batch.rollup_height {
                            if !curr_batch.actions.is_empty() {
                                self.batch_tx
                                    .send(curr_batch)
                                    .await
                                    .wrap_err("failed to send batched events; receiver dropped?")?;
                            }

                            curr_batch = Batch {
                                actions: Vec::new(),
                                rollup_height: block_number.as_u64(),
                            };
                        }
                    } else {
                        error!("block stream closed; shutting down batcher");
                        break;
                    }
                }
                item = self.event_rx.recv() => {
                    if let Some((event, meta)) = item {
                        let event_with_metadata = EventWithMetadata {
                            event,
                            block_number: meta.block_number,
                            transaction_hash: meta.transaction_hash,
                        };
                        let action = event_to_action(event_with_metadata, self.fee_asset_id, self.rollup_asset_denom.clone(), self.asset_withdrawal_divisor)?;

                        if meta.block_number.as_u64() == curr_batch.rollup_height {
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
                                rollup_height: meta.block_number.as_u64(),
                            };
                        }
                    } else {
                        error!("event receiver dropped; shutting down batcher");
                        break;
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
    let address: [u8; 20] = bytes.try_into().map_err(|bytes: Vec<u8>| {
        eyre!(
            "invalid length for {} ethereum address, must be 20 bytes",
            bytes.len()
        )
    })?;
    Ok(address.into())
}

#[cfg(test)]
mod tests {
    use astria_core::protocol::transaction::v1alpha1::Action;
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
    use crate::withdrawer::ethereum::{
        astria_mintable_erc20::AstriaMintableERC20,
        astria_withdrawer::AstriaWithdrawer,
        astria_withdrawer_interface::{
            Ics20WithdrawalFilter,
            SequencerWithdrawalFilter,
        },
        convert::EventWithMetadata,
        test_utils::{
            ConfigureAstriaMintableERC20Deployer,
            ConfigureAstriaWithdrawerDeployer,
        },
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
    #[ignore = "requires foundry and solc to be installed"]
    async fn astria_withdrawer_invalid_value_fails() {
        let (contract_address, provider, wallet, _anvil) = ConfigureAstriaWithdrawerDeployer {
            base_chain_asset_precision: 15,
        }
        .deploy()
        .await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value: U256 = 999.into(); // 10^3 - 1
        let recipient = [0u8; 20].into();
        let tx = contract.withdraw_to_sequencer(recipient).value(value);
        tx.send()
            .await
            .expect_err("`withdraw` transaction should have failed due to value < 10^3");
    }

    #[tokio::test]
    #[ignore = "requires foundry and solc to be installed"]
    async fn watcher_can_watch_sequencer_withdrawals_astria_withdrawer() {
        let (contract_address, provider, wallet, anvil) =
            ConfigureAstriaWithdrawerDeployer::default().deploy().await;
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
        let denom: Denom = Denom::from_base_denom("nria");
        let expected_action =
            event_to_action(expected_event, denom.id(), denom.clone(), 1).unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
            Arc::new(State::new()),
            denom.id(),
            denom,
        )
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_sequencer_withdraw_transaction(&contract, value, recipient).await;

        let batch = event_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::BridgeUnlock(action) = &batch.actions[0] else {
            panic!(
                "expected action to be BridgeUnlock, got {:?}",
                batch.actions[0]
            );
        };
        assert_eq!(action, &expected_action);
    }

    async fn send_ics20_withdraw_transaction<M: Middleware>(
        contract: &AstriaWithdrawer<M>,
        value: U256,
        recipient: String,
    ) -> TransactionReceipt {
        let tx = contract
            .withdraw_to_ibc_chain(recipient, "nootwashere".to_string())
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
    #[ignore = "requires foundry and solc to be installed"]
    async fn watcher_can_watch_ics20_withdrawals_astria_withdrawer() {
        let (contract_address, provider, wallet, anvil) =
            ConfigureAstriaWithdrawerDeployer::default().deploy().await;
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
                memo: "nootwashere".to_string(),
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };
        let denom = Denom::from("transfer/channel-0/utia".to_string());
        let Action::Ics20Withdrawal(mut expected_action) =
            event_to_action(expected_event, denom.id(), denom.clone(), 1).unwrap()
        else {
            panic!("expected action to be Ics20Withdrawal");
        };
        expected_action.timeout_time = 0; // zero this for testing

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
            Arc::new(State::new()),
            denom.id(),
            denom,
        )
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_ics20_withdraw_transaction(&contract, value, recipient).await;

        let mut batch = event_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::Ics20Withdrawal(ref mut action) = batch.actions[0] else {
            panic!(
                "expected action to be Ics20Withdrawal, got {:?}",
                batch.actions[0]
            );
        };
        action.timeout_time = 0; // zero this for testing
        assert_eq!(action, &expected_action);
    }

    async fn mint_tokens<M: Middleware>(
        contract: &AstriaMintableERC20<M>,
        amount: U256,
        recipient: ethers::types::Address,
    ) -> TransactionReceipt {
        let mint_tx = contract.mint(recipient, amount);
        let receipt = mint_tx
            .send()
            .await
            .expect("failed to submit mint transaction")
            .await
            .expect("failed to await pending mint transaction")
            .expect("no mint receipt found");

        assert!(
            receipt.status == Some(ethers::types::U64::from(1)),
            "`mint` transaction failed: {receipt:?}",
        );

        receipt
    }

    async fn send_sequencer_withdraw_transaction_erc20<M: Middleware>(
        contract: &AstriaMintableERC20<M>,
        value: U256,
        recipient: ethers::types::Address,
    ) -> TransactionReceipt {
        let tx = contract.withdraw_to_sequencer(value, recipient);
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
    #[ignore = "requires foundry and solc to be installed"]
    async fn watcher_can_watch_sequencer_withdrawals_astria_mintable_erc20() {
        let (contract_address, provider, wallet, anvil) = ConfigureAstriaMintableERC20Deployer {
            base_chain_asset_precision: 18,
            ..Default::default()
        }
        .deploy()
        .await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaMintableERC20::new(contract_address, signer.clone());

        // mint some tokens to the wallet
        mint_tokens(&contract, 2_000_000_000.into(), wallet.address()).await;

        let value = 1_000_000_000.into();
        let recipient = [0u8; 20].into();
        let receipt = send_sequencer_withdraw_transaction_erc20(&contract, value, recipient).await;
        let expected_event = EventWithMetadata {
            event: WithdrawalEvent::Sequencer(SequencerWithdrawalFilter {
                sender: wallet.address(),
                destination_chain_address: recipient,
                amount: value,
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };
        let denom: Denom = Denom::from_base_denom("nria");
        let expected_action =
            event_to_action(expected_event, denom.id(), denom.clone(), 1).unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
            Arc::new(State::new()),
            denom.id(),
            denom,
        )
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_sequencer_withdraw_transaction_erc20(&contract, value, recipient).await;

        let batch = event_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::BridgeUnlock(action) = &batch.actions[0] else {
            panic!(
                "expected action to be BridgeUnlock, got {:?}",
                batch.actions[0]
            );
        };
        assert_eq!(action, &expected_action);
    }

    async fn send_ics20_withdraw_transaction_astria_mintable_erc20<M: Middleware>(
        contract: &AstriaMintableERC20<M>,
        value: U256,
        recipient: String,
    ) -> TransactionReceipt {
        let tx = contract.withdraw_to_ibc_chain(value, recipient, "nootwashere".to_string());
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
    #[ignore = "requires foundry and solc to be installed"]
    async fn watcher_can_watch_ics20_withdrawals_astria_mintable_erc20() {
        let (contract_address, provider, wallet, anvil) = ConfigureAstriaMintableERC20Deployer {
            base_chain_asset_precision: 18,
            ..Default::default()
        }
        .deploy()
        .await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaMintableERC20::new(contract_address, signer.clone());

        // mint some tokens to the wallet
        mint_tokens(&contract, 2_000_000_000.into(), wallet.address()).await;

        let value = 1_000_000_000.into();
        let recipient = "somebech32address".to_string();
        let receipt = send_ics20_withdraw_transaction_astria_mintable_erc20(
            &contract,
            value,
            recipient.clone(),
        )
        .await;
        let expected_event = EventWithMetadata {
            event: WithdrawalEvent::Ics20(Ics20WithdrawalFilter {
                sender: wallet.address(),
                destination_chain_address: recipient.clone(),
                amount: value,
                memo: "nootwashere".to_string(),
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };
        let denom = Denom::from("transfer/channel-0/utia".to_string());
        let Action::Ics20Withdrawal(mut expected_action) =
            event_to_action(expected_event, denom.id(), denom.clone(), 1).unwrap()
        else {
            panic!("expected action to be Ics20Withdrawal");
        };
        expected_action.timeout_time = 0; // zero this for testing

        let (event_tx, mut event_rx) = mpsc::channel(100);
        let watcher = Watcher::new(
            &hex::encode(contract_address),
            &anvil.ws_endpoint(),
            event_tx,
            &CancellationToken::new(),
            Arc::new(State::new()),
            denom.id(),
            denom,
        )
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_ics20_withdraw_transaction_astria_mintable_erc20(&contract, value, recipient).await;

        let mut batch = event_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::Ics20Withdrawal(ref mut action) = batch.actions[0] else {
            panic!(
                "expected action to be Ics20Withdrawal, got {:?}",
                batch.actions[0]
            );
        };
        action.timeout_time = 0; // zero this for testing
        assert_eq!(action, &expected_action);
    }
}
