use std::{
    cmp::{
        max,
        Ordering,
    },
    collections::HashMap,
    time::Duration,
};

use astria_core::{
    crypto::ADDRESS_LENGTH,
    primitive::v1::{
        Address,
        TransactionId,
    },
    protocol::{
        genesis::v1::GenesisAppState,
        transaction::v1::{
            action::RollupDataSubmission,
            Action,
        },
    },
    sequencerblock::v1::DataItem,
    upgrades::{
        test_utils::UpgradesBuilder,
        v1::{
            blackburn::Blackburn,
            Aspen,
            Change,
            Upgrades,
        },
    },
};
use bytes::Bytes;
use cnidarium::{
    Snapshot,
    StateDelta,
    Storage,
    TempStorage,
};
use ibc_types::{
    core::{
        client::ClientId,
        commitment::MerkleRoot,
    },
    lightclients::tendermint::{
        client_state::ClientState,
        ConsensusState,
    },
};
use penumbra_ibc::component::{
    ClientStateWriteExt as _,
    ConsensusStateWriteExt as _,
};
use sha2::Digest as _;
use telemetry::Metrics as _;
use tendermint::{
    abci,
    abci::types::CommitInfo,
    block::{
        Height,
        Round,
    },
    Time,
};

use super::{
    BridgeInitializer,
    ChainInitializer,
    CheckedTxBuilder,
    ALICE_ADDRESS_BYTES,
};
use crate::{
    accounts::{
        AddressBytes,
        StateReadExt as _,
    },
    app::{
        vote_extension::Handler as VeHandler,
        App,
        StateReadExt as _,
        StateWriteExt as _,
    },
    checked_actions::{
        CheckedAction,
        CheckedActionInitialCheckError,
    },
    fees::StateReadExt as _,
    ibc::host_interface::AstriaHost,
    mempool::Mempool,
    proposal::commitment::generate_rollup_datas_commitment,
    test_utils::{
        dummy_extended_commit_info,
        nria,
    },
    Metrics,
};

/// A fixture for initializing and updating global state, and providing helpers for use in tests
/// (e.g. creating checked transactions).
pub(crate) struct Fixture {
    pub(crate) app: App,
    pub(super) storage: Storage,
    pub(super) genesis_app_state: Option<GenesisAppState>,
}

impl Fixture {
    /// Returns a `Fixture` where `init_chain` has NOT been called.
    ///
    /// This is useful if you need to fine-tune chain initialization, which can be done as follows:
    /// ```ignore
    /// let mut fixture = Fixture::uninitialized(None).await;
    /// fixture.chain_initializer().with_xxx().with_yyy().init().await;
    /// ```
    ///
    /// If `upgrades` is `None`, then Aspen will be set to activate at height 1.
    pub(crate) async fn uninitialized(upgrades: Option<Upgrades>) -> Self {
        let storage = TempStorage::new().await.unwrap().clone();
        let snapshot = storage.latest_snapshot();
        let metrics = Box::leak(Box::new(Metrics::noop_metrics(&()).unwrap()));
        let mempool = Mempool::new(metrics, 100, 100);
        let upgrades_handler = upgrades
            .unwrap_or_else(|| {
                UpgradesBuilder::new()
                    .set_aspen(Some(1))
                    .set_blackburn(Some(3))
                    .build()
            })
            .into();
        let ve_handler = VeHandler::new(None);
        let app = App::new(snapshot, mempool, upgrades_handler, ve_handler, metrics)
            .await
            .unwrap();
        Self {
            storage,
            app,
            genesis_app_state: None,
        }
    }

    /// Returns a `Fixture` where default values have been used in a call to `init_chain`, and then
    /// `Self::run_until_blackburn_applied` has been executed.
    ///
    /// The Aspen and Blackburn upgrades will have been applied at block heights 1 and 3,
    /// respectively. Blocks 2, and 4 will also have been executed (both as empty blocks).
    ///
    /// For a list of the default values used at genesis, see the docs for [`ChainInitializer`].
    pub(crate) async fn default_initialized() -> Self {
        let mut fixture = Self::uninitialized(None).await;
        fixture.chain_initializer().init().await;
        let _ = fixture.run_until_blackburn_applied().await;
        fixture
    }

    /// Returns a `Fixture` where legacy default values have been used in a call to `init_chain`.
    ///
    /// This only exists to support snapshot tests in `app::test_breaking_changes` module.
    pub(crate) async fn legacy_initialized() -> Self {
        let mut fixture = Self::uninitialized(None).await;
        ChainInitializer::legacy(&mut fixture).init().await;
        fixture
    }

    /// Repeatedly executes `App::finalize_block` and `App::commit` until one block after the
    /// Blackburn upgrade has been applied (including Aspen upgrade as well).
    ///
    /// Returns the height of the next block to execute.
    ///
    /// Panics if the Aspen or Blackburn upgrades are not included in the app's upgrade handler (set
    /// by default to activate at blocks 1 and 3), or if either activation height is greater
    /// than 10.
    pub(crate) async fn run_until_blackburn_applied(&mut self) -> Height {
        let aspen = self
            .app
            .upgrades_handler()
            .upgrades()
            .aspen()
            .expect("upgrades should contain aspen upgrade")
            .clone();
        let blackburn = self
            .app
            .upgrades_handler()
            .upgrades()
            .blackburn()
            .expect("upgrades should contain blackburn upgrade")
            .clone();
        assert!(
            blackburn.activation_height() <= 10 && aspen.activation_height() <= 10,
            "activation heights must be <= 10; don't want to execute too many blocks for unit test"
        );

        let proposer_address: tendermint::account::Id =
            ALICE_ADDRESS_BYTES.to_vec().try_into().unwrap();
        let time = Time::from_unix_timestamp(1_744_036_762, 123_456_789).unwrap();

        let final_block_height = max(
            blackburn.activation_height().checked_add(1).unwrap(),
            aspen.activation_height().checked_add(1).unwrap(),
        );
        for height in 1..=final_block_height {
            let txs = aspen_and_blackburn_rollup_data_commitments(height, &aspen, &blackburn);
            let finalize_block = abci::request::FinalizeBlock {
                hash: tendermint::Hash::Sha256(sha2::Sha256::digest(height.to_le_bytes()).into()),
                height: Height::try_from(height).unwrap(),
                time: time.checked_add(Duration::from_secs(height)).unwrap(),
                next_validators_hash: tendermint::Hash::default(),
                proposer_address,
                txs,
                decided_last_commit: CommitInfo {
                    votes: vec![],
                    round: Round::default(),
                },
                misbehavior: vec![],
            };
            self.app
                .finalize_block(finalize_block, self.storage.clone())
                .await
                .unwrap();
            self.app.commit(self.storage.clone()).await.unwrap();
        }
        Height::try_from(
            final_block_height
                .checked_add(1)
                .expect("should increment final block height"),
        )
        .expect("should convert to height")
    }

    /// Consumes `self`, and returns the wrapped `App` and `Storage`.
    pub(crate) fn destructure(self) -> (App, Storage) {
        (self.app, self.storage)
    }

    /// Returns a reference to the state delta held by `App`.
    pub(crate) fn state(&self) -> &StateDelta<Snapshot> {
        self.app.state()
    }

    /// Returns a mutable reference to the state delta held by `App`.
    ///
    /// Note that changes made via this ref will not be persisted to the underlying storage by
    /// default. To persist changes, use e.g. [`App::new_state_delta`] and then
    /// [`App::apply_and_commit`].
    pub(crate) fn state_mut(&mut self) -> &mut StateDelta<Snapshot> {
        self.app.state_mut()
    }

    /// Returns a clone of the fixture's `Storage`.
    pub(crate) fn storage(&self) -> Storage {
        self.storage.clone()
    }

    /// Returns a reference to the applied genesis app state.
    pub(crate) fn genesis_app_state(&self) -> &GenesisAppState {
        self.genesis_app_state
            .as_ref()
            .expect("fixture should be initialized")
    }

    /// Returns a clone of the App's mempool.
    pub(crate) fn mempool(&self) -> Mempool {
        self.app.mempool()
    }

    /// Returns a reference to the App's metrics.
    pub(crate) fn metrics(&self) -> &'static Metrics {
        self.app.metrics()
    }

    /// Consumes `self` and converts all changes in the `App`'s state delta to `Event`s.
    pub(crate) fn into_events(self) -> Vec<abci::Event> {
        self.app.into_events()
    }

    /// Returns a new `ChainInitializer` to customize and then run the chain's genesis.
    pub(crate) fn chain_initializer(&mut self) -> ChainInitializer<'_> {
        ChainInitializer::new(self)
    }

    /// Returns a new `ChainInitializer` where legacy default values are set.
    ///
    /// This only exists to support snapshot tests in `app::test_breaking_changes` module.
    pub(crate) fn legacy_chain_initializer(&mut self) -> ChainInitializer<'_> {
        ChainInitializer::legacy(self)
    }

    /// Returns a new `BridgeInitializer` to simplify initializing a new bridge account.
    pub(crate) fn bridge_initializer(&mut self, bridge_address: Address) -> BridgeInitializer<'_> {
        BridgeInitializer::new(self, bridge_address)
    }

    /// Returns a new `CheckedAction` derived from the provided `action`.
    ///
    /// Returns an error if construction fails the initial checks for the specific action.
    pub(crate) async fn new_checked_action<T: Into<Action>>(
        &self,
        action: T,
        tx_signer: [u8; ADDRESS_LENGTH],
    ) -> Result<CheckedAction, CheckedActionInitialCheckError> {
        match action.into() {
            Action::RollupDataSubmission(action) => {
                CheckedAction::new_rollup_data_submission(action)
            }
            Action::Transfer(action) => {
                CheckedAction::new_transfer(action, tx_signer, self.state()).await
            }
            Action::ValidatorUpdate(action) => {
                CheckedAction::new_validator_update(action, tx_signer, self.state()).await
            }
            Action::SudoAddressChange(action) => {
                CheckedAction::new_sudo_address_change(action, tx_signer, self.state()).await
            }
            Action::Ibc(action) => {
                CheckedAction::new_ibc_relay(action, tx_signer, self.state()).await
            }
            Action::IbcSudoChange(action) => {
                CheckedAction::new_ibc_sudo_change(action, tx_signer, self.state()).await
            }
            Action::Ics20Withdrawal(action) => {
                CheckedAction::new_ics20_withdrawal(action, tx_signer, self.state()).await
            }
            Action::IbcRelayerChange(action) => {
                CheckedAction::new_ibc_relayer_change(action, tx_signer, self.state()).await
            }
            Action::FeeAssetChange(action) => {
                CheckedAction::new_fee_asset_change(action, tx_signer, self.state()).await
            }
            Action::InitBridgeAccount(action) => {
                CheckedAction::new_init_bridge_account(action, tx_signer, self.state()).await
            }
            Action::BridgeLock(action) => {
                CheckedAction::new_bridge_lock(
                    action,
                    tx_signer,
                    TransactionId::new([10; 32]),
                    10,
                    self.state(),
                )
                .await
            }
            Action::BridgeUnlock(action) => {
                CheckedAction::new_bridge_unlock(action, tx_signer, self.state()).await
            }
            Action::BridgeSudoChange(action) => {
                CheckedAction::new_bridge_sudo_change(action, tx_signer, self.state()).await
            }
            Action::BridgeTransfer(action) => {
                CheckedAction::new_bridge_transfer(
                    action,
                    tx_signer,
                    TransactionId::new([11; 32]),
                    11,
                    self.state(),
                )
                .await
            }
            Action::FeeChange(action) => {
                CheckedAction::new_fee_change(action, tx_signer, self.state()).await
            }
            Action::RecoverIbcClient(action) => {
                CheckedAction::new_recover_ibc_client(action, tx_signer, self.state()).await
            }
            Action::CurrencyPairsChange(action) => {
                CheckedAction::new_currency_pairs_change(action, tx_signer, self.state()).await
            }
            Action::MarketsChange(action) => {
                CheckedAction::new_markets_change(action, tx_signer, self.state()).await
            }
        }
    }

    /// Returns a new `CheckedTxBuilder` to simplify constructing a new `CheckedTransaction`.
    pub(crate) fn checked_tx_builder(&self) -> CheckedTxBuilder<'_> {
        CheckedTxBuilder::new(self)
    }

    /// Returns the current block height as held in the `App`'s state delta.
    pub(crate) async fn block_height(&self) -> Height {
        let height = self.state().get_block_height().await.unwrap();
        Height::try_from(height).unwrap()
    }

    /// Returns the given account's balance of nria.
    pub(crate) async fn get_nria_balance<TAddress: AddressBytes>(
        &self,
        address: &TAddress,
    ) -> u128 {
        self.state()
            .get_account_balance(address, &nria())
            .await
            .unwrap()
    }

    /// Calculates the cost for a `RollupDataSubmission` based on the length of the `data` and the
    /// fees for this as held in the `App`'s state delta.
    pub(crate) async fn calculate_rollup_data_submission_cost(&self, data: &[u8]) -> u128 {
        let fees = self
            .state()
            .get_fees::<RollupDataSubmission>()
            .await
            .expect("should not error fetching rollup data submission fees")
            .expect("rollup data submission fees should be stored");
        fees.base()
            .checked_add(
                fees.multiplier()
                    .checked_mul(
                        data.len()
                            .try_into()
                            .expect("a usize should always convert to a u128"),
                    )
                    .expect("fee multiplication should not overflow"),
            )
            .expect("fee addition should not overflow")
    }

    /// Initializes a new active IBC client.
    pub(crate) async fn init_active_ibc_client(
        &mut self,
        client_id: &ClientId,
        client_state: ClientState,
    ) {
        self.init_ibc_client(client_id, client_state, true).await;
    }

    /// Initializes a new expired IBC client.
    pub(crate) async fn init_expired_ibc_client(
        &mut self,
        client_id: &ClientId,
        client_state: ClientState,
    ) {
        self.init_ibc_client(client_id, client_state, false).await;
    }

    async fn init_ibc_client(
        &mut self,
        client_id: &ClientId,
        client_state: ClientState,
        active: bool,
    ) {
        let height = client_state.latest_height;
        let trusting_period = client_state.trusting_period;
        self.state_mut().put_client(client_id, client_state);

        self.state_mut()
            .put_revision_number(height.revision_number)
            .unwrap();
        // Don't allow the stored block height to decrease.
        let current_stored_height = self.state().get_block_height().await.unwrap_or_default();
        self.state_mut()
            .put_block_height(std::cmp::max(height.revision_height, current_stored_height))
            .unwrap();

        let timestamp = Time::from_unix_timestamp(100, 2).unwrap();
        self.state_mut().put_block_timestamp(timestamp).unwrap();

        let consensus_state_timestamp = if active {
            // If we want the client to be active, just use the block timestamp for its consensus
            // state.
            timestamp
        } else {
            // If we want the client to be expired, make its consensus state timestamp earlier than
            // the block timestamp by more than the trusting period.
            timestamp
                .checked_sub(trusting_period)
                .and_then(|t| t.checked_sub(Duration::from_nanos(1)))
                .unwrap()
        };
        let consensus_state = ConsensusState::new(
            MerkleRoot {
                hash: vec![1; 32],
            },
            consensus_state_timestamp,
            tendermint::Hash::Sha256([2; 32]),
        );

        self.state_mut()
            .put_verified_consensus_state::<AstriaHost>(height, client_id.clone(), consensus_state)
            .await
            .unwrap();
    }
}

fn aspen_and_blackburn_rollup_data_commitments(
    current_block_height: u64,
    aspen: &Aspen,
    blackburn: &Blackburn,
) -> Vec<Bytes> {
    let extended_commit_info = match current_block_height.cmp(&aspen.activation_height()) {
        Ordering::Less => {
            // Prior to Aspen, and hence also pre-Blackburn, use legacy rollup data commitments.
            return generate_rollup_datas_commitment::<false>(&[], HashMap::new())
                .into_iter()
                .collect();
        }
        Ordering::Equal => {
            // At Aspen upgrade, and hence also pre-Blackburn, use new form of rollup data
            // commitments and append the Aspen upgrade change hashes.
            let aspen_upgrade_change_hashes = DataItem::UpgradeChangeHashes(
                aspen.changes().map(Change::calculate_hash).collect(),
            );
            return generate_rollup_datas_commitment::<true>(&[], HashMap::new())
                .into_iter()
                .chain(Some(aspen_upgrade_change_hashes.encode()))
                .collect();
        }
        Ordering::Greater => {
            // The first block after Aspen doesn't have extended commit info. All blocks after that
            // should have it.
            if current_block_height > aspen.activation_height().checked_add(1).unwrap() {
                Some(dummy_extended_commit_info().encode())
            } else {
                None
            }
        }
    };

    if current_block_height == blackburn.activation_height() {
        let blackburn_upgrade_change_hashes = DataItem::UpgradeChangeHashes(
            blackburn.changes().map(Change::calculate_hash).collect(),
        );
        return generate_rollup_datas_commitment::<true>(&[], HashMap::new())
            .into_iter()
            .chain(Some(blackburn_upgrade_change_hashes.encode()))
            .chain(extended_commit_info)
            .collect();
    }

    generate_rollup_datas_commitment::<true>(&[], HashMap::new())
        .into_iter()
        .chain(extended_commit_info)
        .collect()
}
