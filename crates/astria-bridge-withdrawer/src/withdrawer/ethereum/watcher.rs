use std::{
    sync::Arc,
    time::Duration,
};

use astria_core::primitive::v1::{
    asset,
    asset::Denom,
    Address,
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
    core::types::Block,
    providers::{
        Middleware,
        Provider,
        ProviderError,
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
    submitter,
    SequencerStartupInfo,
};

pub(crate) struct Builder {
    pub(crate) ethereum_contract_address: String,
    pub(crate) ethereum_rpc_endpoint: String,
    pub(crate) submitter_handle: submitter::Handle,
    pub(crate) shutdown_token: CancellationToken,
    pub(crate) state: Arc<State>,
    pub(crate) rollup_asset_denom: Denom,
    pub(crate) bridge_address: Address,
}

impl Builder {
    pub(crate) fn build(self) -> Result<Watcher> {
        let Builder {
            ethereum_contract_address,
            ethereum_rpc_endpoint,
            submitter_handle,
            shutdown_token,
            state,
            rollup_asset_denom,
            bridge_address,
        } = self;

        let contract_address = address_from_string(&ethereum_contract_address)
            .wrap_err("failed to parse ethereum contract address")?;

        if !rollup_asset_denom.is_prefixed() {
            warn!(
                "rollup asset denomination is not prefixed; Ics20Withdrawal actions will not be \
                 submitted"
            );
        }

        Ok(Watcher {
            contract_address,
            ethereum_rpc_endpoint: ethereum_rpc_endpoint.to_string(),
            submitter_handle,
            rollup_asset_denom,
            bridge_address,
            state,
            shutdown_token: shutdown_token.clone(),
        })
    }
}

/// Watches for withdrawal events emitted by the `AstriaWithdrawer` contract.
pub(crate) struct Watcher {
    contract_address: ethers::types::Address,
    ethereum_rpc_endpoint: String,
    submitter_handle: submitter::Handle,
    rollup_asset_denom: Denom,
    bridge_address: Address,
    state: Arc<State>,
    shutdown_token: CancellationToken,
}

impl Watcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let (provider, contract, fee_asset_id, asset_withdrawal_divisor, next_rollup_block_height) =
            self.startup().await?;

        let Self {
            contract_address: _contract_address,
            ethereum_rpc_endpoint: _ethereum_rps_endpoint,
            submitter_handle,
            rollup_asset_denom,
            bridge_address,
            state,
            shutdown_token,
        } = self;

        let (event_tx, event_rx) = mpsc::channel(100);

        let batcher = Batcher {
            event_rx,
            provider,
            submitter_handle,
            shutdown_token: shutdown_token.clone(),
            fee_asset_id,
            rollup_asset_denom,
            bridge_address,
            asset_withdrawal_divisor,
        };

        tokio::task::spawn(batcher.run());

        let sequencer_withdrawal_event_handler =
            tokio::task::spawn(watch_for_sequencer_withdrawal_events(
                contract.clone(),
                event_tx.clone(),
                next_rollup_block_height,
            ));
        let ics20_withdrawal_event_handler = tokio::task::spawn(watch_for_ics20_withdrawal_events(
            contract,
            event_tx.clone(),
            next_rollup_block_height,
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

    /// Gets the startup data from the submitter and connects to the Ethereum node.
    ///
    /// Returns the contract handle, the asset ID of the fee asset, the divisor for the asset
    /// withdrawal amount, and the rollup block height to watch from.
    ///
    /// # Errors
    /// - If the fee asset ID provided in the config is not a valid fee asset on the sequencer.
    /// - If the Ethereum node cannot be connected to after several retries.
    /// - If the asset withdrawal decimals cannot be fetched.
    async fn startup(
        &mut self,
    ) -> eyre::Result<(
        Arc<Provider<Ws>>,
        IAstriaWithdrawer<Provider<Ws>>,
        asset::Id,
        u128,
        u64,
    )> {
        // wait for submitter to be ready
        let SequencerStartupInfo {
            fee_asset_id,
            next_batch_rollup_height,
        } = self.submitter_handle.recv_startup_info().await?;

        // connect to eth node
        let retry_config = tryhard::RetryFutureConfig::new(1024)
            .exponential_backoff(Duration::from_millis(500))
            .max_delay(Duration::from_secs(60))
            .on_retry(
                |attempt, next_delay: Option<Duration>, error: &ProviderError| {
                    let wait_duration = next_delay
                        .map(humantime::format_duration)
                        .map(tracing::field::display);
                    warn!(
                        attempt,
                        wait_duration,
                        error = error as &dyn std::error::Error,
                        "attempt to connect to rollup node failed; retrying after backoff",
                    );
                    futures::future::ready(())
                },
            );

        let provider = tryhard::retry_fn(|| {
            let url = self.ethereum_rpc_endpoint.clone();
            async move {
                let websocket_client = Ws::connect_with_reconnects(url, 0).await?;
                Ok(Provider::new(websocket_client))
            }
        })
        .with_config(retry_config)
        .await
        .wrap_err("failed connecting to rollup after several retries; giving up")?;
        let provider = Arc::new(provider);

        // get contract handle
        let contract = IAstriaWithdrawer::new(self.contract_address, provider.clone());

        // get asset withdrawal decimals
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

        self.state.set_watcher_ready();

        Ok((
            provider.clone(),
            contract,
            fee_asset_id,
            asset_withdrawal_divisor,
            next_batch_rollup_height,
        ))
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

    let mut stream = events
        .stream()
        .await
        .wrap_err("failed to subscribe to sequencer withdrawal events")?
        .with_meta();

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

    let mut stream = events
        .stream()
        .await
        .wrap_err("failed to subscribe to ics20 withdrawal events")?
        .with_meta();

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
    submitter_handle: submitter::Handle,
    shutdown_token: CancellationToken,
    fee_asset_id: asset::Id,
    rollup_asset_denom: Denom,
    bridge_address: Address,
    asset_withdrawal_divisor: u128,
}

impl Batcher {
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
                                self.submitter_handle.send_batch(curr_batch)
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
                        let action = event_to_action(event_with_metadata, self.fee_asset_id, self.rollup_asset_denom.clone(), self.asset_withdrawal_divisor, self.bridge_address)?;

                        if meta.block_number.as_u64() == curr_batch.rollup_height {
                            // block number was the same; add event to current batch
                            curr_batch.actions.push(action);
                        } else {
                            // block number increased; send current batch and start a new one
                            if !curr_batch.actions.is_empty() {
                                self.submitter_handle.send_batch(curr_batch)
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
    use asset::default_native_asset;
    use astria_core::{
        primitive::v1::{
            Address,
            ASTRIA_ADDRESS_PREFIX,
        },
        protocol::transaction::v1alpha1::Action,
    };
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
    use tokio::sync::oneshot;

    use super::*;
    use crate::withdrawer::ethereum::{
        astria_bridgeable_erc20::AstriaBridgeableERC20,
        astria_withdrawer::AstriaWithdrawer,
        astria_withdrawer_interface::{
            Ics20WithdrawalFilter,
            SequencerWithdrawalFilter,
        },
        convert::EventWithMetadata,
        test_utils::{
            ConfigureAstriaBridgeableERC20Deployer,
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
        recipient: Address,
    ) -> TransactionReceipt {
        let tx = contract
            .withdraw_to_sequencer(recipient.to_string())
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
    #[ignore = "requires foundry to be installed"]
    async fn astria_withdrawer_invalid_value_fails() {
        let (contract_address, provider, wallet, _anvil) = ConfigureAstriaWithdrawerDeployer {
            base_chain_asset_precision: 15,
            ..Default::default()
        }
        .deploy()
        .await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value: U256 = 999.into(); // 10^3 - 1
        let recipient = Address::builder()
            .array([1u8; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let tx = contract
            .withdraw_to_sequencer(recipient.to_string())
            .value(value);
        tx.send()
            .await
            .expect_err("`withdraw` transaction should have failed due to value < 10^3");
    }

    #[tokio::test]
    #[ignore = "requires foundry to be installed"]
    async fn watcher_can_watch_sequencer_withdrawals_astria_withdrawer() {
        let (contract_address, provider, wallet, anvil) =
            ConfigureAstriaWithdrawerDeployer::default().deploy().await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value = 1_000_000_000.into();
        let recipient = Address::builder()
            .array([1u8; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let receipt = send_sequencer_withdraw_transaction(&contract, value, recipient).await;
        let expected_event = EventWithMetadata {
            event: WithdrawalEvent::Sequencer(SequencerWithdrawalFilter {
                sender: wallet.address(),
                destination_chain_address: recipient.to_string(),
                amount: value,
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };
        let bridge_address = crate::astria_address([1u8; 20]);
        let denom = default_native_asset();
        let expected_action =
            event_to_action(expected_event, denom.id(), denom.clone(), 1, bridge_address).unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset_id: denom.id(),
                next_batch_rollup_height: 0,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom,
            bridge_address,
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_sequencer_withdraw_transaction(&contract, value, recipient).await;

        let batch = batch_rx.recv().await.unwrap();
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
    #[ignore = "requires foundry to be installed"]
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
        let bridge_address = crate::astria_address([1u8; 20]);
        let denom = "transfer/channel-0/utia".parse::<Denom>().unwrap();
        let Action::Ics20Withdrawal(mut expected_action) =
            event_to_action(expected_event, denom.id(), denom.clone(), 1, bridge_address).unwrap()
        else {
            panic!("expected action to be Ics20Withdrawal");
        };
        expected_action.timeout_time = 0; // zero this for testing

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset_id: denom.id(),
                next_batch_rollup_height: 0,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom,
            bridge_address,
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_ics20_withdraw_transaction(&contract, value, recipient).await;

        let mut batch = batch_rx.recv().await.unwrap();
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
        contract: &AstriaBridgeableERC20<M>,
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
        contract: &AstriaBridgeableERC20<M>,
        value: U256,
        recipient: Address,
    ) -> TransactionReceipt {
        let tx = contract.withdraw_to_sequencer(value, recipient.to_string());
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
    #[ignore = "requires foundry to be installed"]
    async fn watcher_can_watch_sequencer_withdrawals_astria_bridgeable_erc20() {
        let (contract_address, provider, wallet, anvil) = ConfigureAstriaBridgeableERC20Deployer {
            base_chain_asset_precision: 18,
            ..Default::default()
        }
        .deploy()
        .await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaBridgeableERC20::new(contract_address, signer.clone());

        // mint some tokens to the wallet
        mint_tokens(&contract, 2_000_000_000.into(), wallet.address()).await;

        let value = 1_000_000_000.into();
        let recipient = Address::builder()
            .array([1u8; 20])
            .prefix(ASTRIA_ADDRESS_PREFIX)
            .try_build()
            .unwrap();
        let receipt = send_sequencer_withdraw_transaction_erc20(&contract, value, recipient).await;
        let expected_event = EventWithMetadata {
            event: WithdrawalEvent::Sequencer(SequencerWithdrawalFilter {
                sender: wallet.address(),
                destination_chain_address: recipient.to_string(),
                amount: value,
            }),
            block_number: receipt.block_number.unwrap(),
            transaction_hash: receipt.transaction_hash,
        };
        let denom = default_native_asset();
        let bridge_address = crate::astria_address([1u8; 20]);
        let expected_action =
            event_to_action(expected_event, denom.id(), denom.clone(), 1, bridge_address).unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset_id: denom.id(),
                next_batch_rollup_height: 0,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom,
            bridge_address,
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_sequencer_withdraw_transaction_erc20(&contract, value, recipient).await;

        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::BridgeUnlock(action) = &batch.actions[0] else {
            panic!(
                "expected action to be BridgeUnlock, got {:?}",
                batch.actions[0]
            );
        };
        assert_eq!(action, &expected_action);
    }

    async fn send_ics20_withdraw_transaction_astria_bridgeable_erc20<M: Middleware>(
        contract: &AstriaBridgeableERC20<M>,
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
    #[ignore = "requires foundry to be installed"]
    async fn watcher_can_watch_ics20_withdrawals_astria_bridgeable_erc20() {
        let (contract_address, provider, wallet, anvil) = ConfigureAstriaBridgeableERC20Deployer {
            base_chain_asset_precision: 18,
            ..Default::default()
        }
        .deploy()
        .await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaBridgeableERC20::new(contract_address, signer.clone());

        // mint some tokens to the wallet
        mint_tokens(&contract, 2_000_000_000.into(), wallet.address()).await;

        let value = 1_000_000_000.into();
        let recipient = "somebech32address".to_string();
        let receipt = send_ics20_withdraw_transaction_astria_bridgeable_erc20(
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
        let denom = "transfer/channel-0/utia".parse::<Denom>().unwrap();
        let bridge_address = crate::astria_address([1u8; 20]);
        let Action::Ics20Withdrawal(mut expected_action) =
            event_to_action(expected_event, denom.id(), denom.clone(), 1, bridge_address).unwrap()
        else {
            panic!("expected action to be Ics20Withdrawal");
        };
        expected_action.timeout_time = 0; // zero this for testing

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset_id: asset::Id::from_denom("transfer/channel-0/utia"),
                next_batch_rollup_height: 0,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom,
            bridge_address,
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

        // make another tx to trigger anvil to make another block
        send_ics20_withdraw_transaction_astria_bridgeable_erc20(&contract, value, recipient).await;

        let mut batch = batch_rx.recv().await.unwrap();
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
