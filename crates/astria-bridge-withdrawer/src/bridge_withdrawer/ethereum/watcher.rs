use std::{
    sync::Arc,
    time::Duration,
};

use astria_bridge_contracts::i_astria_withdrawer::{
    IAstriaWithdrawer,
    Ics20WithdrawalFilter,
    SequencerWithdrawalFilter,
};
use astria_core::{
    primitive::v1::{
        asset::{
            self,
            denom,
            Denom,
        },
        Address,
    },
    protocol::transaction::v1alpha1::Action,
};
use astria_eyre::{
    eyre::{
        self,
        bail,
        eyre,
        WrapErr as _,
    },
    Result,
};
use ethers::{
    contract::EthEvent as _,
    core::types::Block,
    providers::{
        Middleware,
        Provider,
        ProviderError,
        StreamExt as _,
        Ws,
    },
    types::{
        Filter,
        Log,
        H256,
    },
    utils::hex,
};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{
    debug,
    info,
    warn,
};

use crate::bridge_withdrawer::{
    batch::Batch,
    ethereum::convert::{
        event_to_action,
        EventWithMetadata,
        WithdrawalEvent,
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
    pub(crate) sequencer_address_prefix: String,
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
            sequencer_address_prefix,
        } = self;

        let contract_address = address_from_string(&ethereum_contract_address)
            .wrap_err("failed to parse ethereum contract address")?;

        if rollup_asset_denom
            .as_trace_prefixed()
            .map_or(false, denom::TracePrefixed::trace_is_empty)
        {
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
            sequencer_address_prefix,
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
    sequencer_address_prefix: String,
}

impl Watcher {
    pub(crate) async fn run(mut self) -> Result<()> {
        let (provider, contract, fee_asset, asset_withdrawal_divisor, next_rollup_block_height) =
            self.startup()
                .await
                .wrap_err("watcher failed to start up")?;

        let Self {
            contract_address: _contract_address,
            ethereum_rpc_endpoint: _ethereum_rps_endpoint,
            submitter_handle,
            rollup_asset_denom,
            bridge_address,
            state,
            shutdown_token,
            sequencer_address_prefix,
        } = self;

        let converter = EventToActionConvertConfig {
            fee_asset,
            rollup_asset_denom,
            bridge_address,
            asset_withdrawal_divisor,
            sequencer_address_prefix,
        };

        state.set_watcher_ready();

        tokio::select! {
            res = watch_for_blocks(
                provider,
                contract.address(),
                next_rollup_block_height,
                converter,
                submitter_handle,
                shutdown_token.clone(),
            ) => {
                info!("block handler exited");
                res.context("block handler exited")
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
        asset::Denom,
        u128,
        u64,
    )> {
        // wait for submitter to be ready
        let SequencerStartupInfo {
            fee_asset,
            next_batch_rollup_height,
        } = self
            .submitter_handle
            .recv_startup_info()
            .await
            .wrap_err("failed to get sequencer startup info")?;

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
            fee_asset,
            asset_withdrawal_divisor,
            next_batch_rollup_height,
        ))
    }
}

async fn sync_from_next_rollup_block_height(
    provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    converter: &EventToActionConvertConfig,
    submitter_handle: &submitter::Handle,
    next_rollup_block_height_to_check: u64,
    current_rollup_block_height: u64,
) -> Result<()> {
    if current_rollup_block_height < next_rollup_block_height_to_check {
        return Ok(());
    }

    for i in next_rollup_block_height_to_check..=current_rollup_block_height {
        let Some(block) = provider
            .get_block(i)
            .await
            .wrap_err("failed to get block")?
        else {
            bail!("block with number {i} missing");
        };

        get_and_send_events_at_block(
            provider.clone(),
            contract_address,
            block,
            converter,
            submitter_handle,
        )
        .await
        .wrap_err("failed to get and send events at block")?;
    }

    info!("synced from {next_rollup_block_height_to_check} to {current_rollup_block_height}");
    Ok(())
}

async fn watch_for_blocks(
    provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    next_rollup_block_height: u64,
    converter: EventToActionConvertConfig,
    submitter_handle: submitter::Handle,
    shutdown_token: CancellationToken,
) -> Result<()> {
    let mut block_rx = provider
        .subscribe_blocks()
        .await
        .wrap_err("failed to subscribe to blocks")?;

    // read latest block height from subscription;
    // use this value for syncing from the next height to submit to current.
    let Some(current_rollup_block) = block_rx.next().await else {
        bail!("failed to get current rollup block from subscription")
    };

    let Some(current_rollup_block_height) = current_rollup_block.number else {
        bail!("current rollup block missing block number")
    };

    // sync any blocks missing between `next_rollup_block_height` and the current latest
    // (inclusive).
    sync_from_next_rollup_block_height(
        provider.clone(),
        contract_address,
        &converter,
        &submitter_handle,
        next_rollup_block_height,
        current_rollup_block_height.as_u64(),
    )
    .await
    .wrap_err("failed to sync from next rollup block height")?;

    loop {
        select! {
            () = shutdown_token.cancelled() => {
                info!("block watcher shutting down");
                return Ok(());
            }
            block = block_rx.next() => {
                if let Some(block) = block {
                    get_and_send_events_at_block(
                        provider.clone(),
                        contract_address,
                        block,
                        &converter,
                        &submitter_handle,
                    )
                    .await
                    .wrap_err("failed to get and send events at block")?;
                } else {
                    bail!("block subscription ended")
                }
            }
        }
    }
}

async fn get_and_send_events_at_block(
    provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    block: Block<H256>,
    converter: &EventToActionConvertConfig,
    submitter_handle: &submitter::Handle,
) -> Result<()> {
    let Some(block_hash) = block.hash else {
        bail!("block hash missing; skipping")
    };

    let Some(block_number) = block.number else {
        bail!("block number missing; skipping")
    };

    let sequencer_withdrawal_events =
        get_sequencer_withdrawal_events(provider.clone(), contract_address, block_hash)
            .await
            .wrap_err("failed to get sequencer withdrawal events")?;
    let ics20_withdrawal_events =
        get_ics20_withdrawal_events(provider.clone(), contract_address, block_hash)
            .await
            .wrap_err("failed to get ics20 withdrawal events")?;
    let events = vec![sequencer_withdrawal_events, ics20_withdrawal_events]
        .into_iter()
        .flatten();
    let mut batch = Batch {
        actions: Vec::new(),
        rollup_height: block_number.as_u64(),
    };
    for (event, log) in events {
        let Some(transaction_hash) = log.transaction_hash else {
            warn!("transaction hash missing; skipping");
            continue;
        };

        let event_with_metadata = EventWithMetadata {
            event,
            block_number,
            transaction_hash,
        };
        let action = converter
            .convert(event_with_metadata)
            .wrap_err("failed to convert event to action")?;
        batch.actions.push(action);
    }

    if batch.actions.is_empty() {
        debug!("no actions to send at block {block_number}");
    } else {
        let actions_len = batch.actions.len();
        submitter_handle
            .send_batch(batch)
            .await
            .wrap_err("failed to send batched events; receiver dropped?")?;
        debug!(
            "sent batch with {} actions at block {block_number}",
            actions_len
        );
    }

    Ok(())
}

async fn get_sequencer_withdrawal_events(
    provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    block_hash: H256,
) -> Result<Vec<(WithdrawalEvent, Log)>> {
    let sequencer_withdrawal_event_sig = SequencerWithdrawalFilter::signature();
    let sequencer_withdrawal_filter = Filter::new()
        .at_block_hash(block_hash)
        .address(contract_address)
        .topic0(sequencer_withdrawal_event_sig);

    let logs = provider
        .get_logs(&sequencer_withdrawal_filter)
        .await
        .wrap_err("failed to get sequencer withdrawal events")?;

    let events = logs
        .into_iter()
        .map(|log| {
            let raw_log = ethers::abi::RawLog {
                topics: log.topics.clone(),
                data: log.data.to_vec(),
            };
            let event = SequencerWithdrawalFilter::decode_log(&raw_log)?;
            Ok((WithdrawalEvent::Sequencer(event), log))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(events)
}

async fn get_ics20_withdrawal_events(
    provider: Arc<Provider<Ws>>,
    contract_address: ethers::types::Address,
    block_hash: H256,
) -> Result<Vec<(WithdrawalEvent, Log)>> {
    let ics20_withdrawal_event_sig = Ics20WithdrawalFilter::signature();
    let ics20_withdrawal_filter = Filter::new()
        .at_block_hash(block_hash)
        .address(contract_address)
        .topic0(ics20_withdrawal_event_sig);

    let logs = provider
        .get_logs(&ics20_withdrawal_filter)
        .await
        .wrap_err("failed to get ics20 withdrawal events")?;

    let events = logs
        .into_iter()
        .map(|log| {
            let raw_log = ethers::abi::RawLog {
                topics: log.topics.clone(),
                data: log.data.to_vec(),
            };
            let event = Ics20WithdrawalFilter::decode_log(&raw_log)?;
            Ok((WithdrawalEvent::Ics20(event), log))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(events)
}

#[derive(Clone)]
struct EventToActionConvertConfig {
    fee_asset: Denom,
    rollup_asset_denom: Denom,
    bridge_address: Address,
    asset_withdrawal_divisor: u128,
    sequencer_address_prefix: String,
}

impl EventToActionConvertConfig {
    fn convert(&self, event: EventWithMetadata) -> Result<Action> {
        event_to_action(
            event,
            self.fee_asset.clone(),
            self.rollup_asset_denom.clone(),
            self.asset_withdrawal_divisor,
            self.bridge_address,
            &self.sequencer_address_prefix,
        )
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
    use astria_bridge_contracts::{
        astria_bridgeable_erc20::AstriaBridgeableERC20,
        astria_withdrawer::AstriaWithdrawer,
        i_astria_withdrawer::{
            Ics20WithdrawalFilter,
            SequencerWithdrawalFilter,
        },
    };
    use astria_core::{
        primitive::v1::{
            asset,
            Address,
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
    use tokio::sync::{
        mpsc,
        mpsc::error::TryRecvError::Empty,
        oneshot,
    };

    use super::*;
    use crate::bridge_withdrawer::ethereum::{
        convert::EventWithMetadata,
        test_utils::{
            ConfigureAstriaBridgeableERC20Deployer,
            ConfigureAstriaWithdrawerDeployer,
        },
    };

    fn default_native_asset() -> asset::Denom {
        "nria".parse().unwrap()
    }

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
        let recipient = crate::astria_address([1u8; 20]);
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
        let recipient = crate::astria_address([1u8; 20]);

        let bridge_address = crate::astria_address([1u8; 20]);
        let denom = default_native_asset();

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset: "nria".parse().unwrap(),
                next_batch_rollup_height: 1,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom.clone(),
            bridge_address,
            sequencer_address_prefix: crate::ASTRIA_ADDRESS_PREFIX.into(),
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

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
        let expected_action = event_to_action(
            expected_event,
            denom.clone(),
            denom,
            1,
            bridge_address,
            crate::ASTRIA_ADDRESS_PREFIX,
        )
        .unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::BridgeUnlock(action) = &batch.actions[0] else {
            panic!(
                "expected action to be BridgeUnlock, got {:?}",
                batch.actions[0]
            );
        };
        assert_eq!(action, &expected_action);
        assert_eq!(batch_rx.try_recv().unwrap_err(), Empty);
    }

    #[tokio::test]
    #[ignore = "requires foundry to be installed"]
    async fn watcher_can_watch_sequencer_withdrawals_astria_withdrawer_sync_from_next_rollup_height()
     {
        let (contract_address, provider, wallet, anvil) =
            ConfigureAstriaWithdrawerDeployer::default().deploy().await;
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        let contract = AstriaWithdrawer::new(contract_address, signer.clone());

        let value = 1_000_000_000.into();
        let recipient = crate::astria_address([1u8; 20]);
        let bridge_address = crate::astria_address([1u8; 20]);
        let denom = default_native_asset();

        // send tx before watcher starts
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
        let expected_action = event_to_action(
            expected_event,
            denom.clone(),
            denom.clone(),
            1,
            bridge_address,
            crate::ASTRIA_ADDRESS_PREFIX,
        )
        .unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset: denom.clone(),
                next_batch_rollup_height: 1,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom.clone(),
            bridge_address,
            sequencer_address_prefix: crate::ASTRIA_ADDRESS_PREFIX.into(),
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

        // send another tx to trigger a new block
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

        // should receive a second batch containing the second tx
        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
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
        let bridge_address = crate::astria_address([1u8; 20]);
        let denom = "transfer/channel-0/utia".parse::<Denom>().unwrap();

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset: denom.clone(),
                next_batch_rollup_height: 1,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom.clone(),
            bridge_address,
            sequencer_address_prefix: crate::ASTRIA_ADDRESS_PREFIX.into(),
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

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
        let Action::Ics20Withdrawal(mut expected_action) = event_to_action(
            expected_event,
            denom.clone(),
            denom.clone(),
            1,
            bridge_address,
            crate::ASTRIA_ADDRESS_PREFIX,
        )
        .unwrap() else {
            panic!("expected action to be Ics20Withdrawal");
        };
        expected_action.timeout_time = 0; // zero this for testing

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
        assert_eq!(batch_rx.try_recv().unwrap_err(), Empty);
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
        let recipient = crate::astria_address([1u8; 20]);
        let denom = default_native_asset();
        let bridge_address = crate::astria_address([1u8; 20]);

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset: "nria".parse().unwrap(),
                next_batch_rollup_height: 1,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom.clone(),
            bridge_address,
            sequencer_address_prefix: crate::ASTRIA_ADDRESS_PREFIX.into(),
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

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
        let expected_action = event_to_action(
            expected_event,
            denom.clone(),
            denom.clone(),
            1,
            bridge_address,
            crate::ASTRIA_ADDRESS_PREFIX,
        )
        .unwrap();
        let Action::BridgeUnlock(expected_action) = expected_action else {
            panic!("expected action to be BridgeUnlock, got {expected_action:?}");
        };

        let batch = batch_rx.recv().await.unwrap();
        assert_eq!(batch.actions.len(), 1);
        let Action::BridgeUnlock(action) = &batch.actions[0] else {
            panic!(
                "expected action to be BridgeUnlock, got {:?}",
                batch.actions[0]
            );
        };
        assert_eq!(action, &expected_action);
        assert_eq!(batch_rx.try_recv().unwrap_err(), Empty);
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
        let denom = "transfer/channel-0/utia".parse::<Denom>().unwrap();
        let bridge_address = crate::astria_address([1u8; 20]);

        let (batch_tx, mut batch_rx) = mpsc::channel(100);
        let (startup_tx, startup_rx) = oneshot::channel();
        let submitter_handle = submitter::Handle::new(startup_rx, batch_tx);
        startup_tx
            .send(SequencerStartupInfo {
                fee_asset: "transfer/channel-0/utia".parse().unwrap(),
                next_batch_rollup_height: 1,
            })
            .unwrap();

        let watcher = Builder {
            ethereum_contract_address: hex::encode(contract_address),
            ethereum_rpc_endpoint: anvil.ws_endpoint(),
            submitter_handle,
            shutdown_token: CancellationToken::new(),
            state: Arc::new(State::new()),
            rollup_asset_denom: denom.clone(),
            bridge_address,
            sequencer_address_prefix: crate::ASTRIA_ADDRESS_PREFIX.into(),
        }
        .build()
        .unwrap();

        tokio::task::spawn(watcher.run());

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
        let Action::Ics20Withdrawal(mut expected_action) = event_to_action(
            expected_event,
            denom.clone(),
            denom.clone(),
            1,
            bridge_address,
            crate::ASTRIA_ADDRESS_PREFIX,
        )
        .unwrap() else {
            panic!("expected action to be Ics20Withdrawal");
        };
        expected_action.timeout_time = 0; // zero this for testing

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
        assert_eq!(batch_rx.try_recv().unwrap_err(), Empty);
    }
}
