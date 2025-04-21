use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    oracles::price_feed::{
        abci::v2::OracleVoteExtension,
        oracle::v2::QuotePrice,
        types::v2::{
            CurrencyPairId,
            Price,
        },
    },
    protocol::transaction::v1::{
        action::{
            Transfer,
            ValidatorName,
            ValidatorUpdate,
        },
        Transaction,
        TransactionBody,
    },
    sequencerblock::v1::block::ExpandedBlockData,
    upgrades::test_utils::UpgradesBuilder,
};
use astria_eyre::eyre::Result;
use bytes::Bytes;
use cnidarium::Storage;
use prost::Message as _;
use sha2::{
    Digest as _,
    Sha256,
};
use tendermint::{
    abci::{
        request::{
            FinalizeBlock,
            PrepareProposal,
            ProcessProposal,
            VerifyVoteExtension,
        },
        response,
        types::{
            BlockSignatureInfo,
            CommitInfo,
            ExtendedCommitInfo,
            ExtendedVoteInfo,
            Validator,
            VoteInfo,
        },
    },
    account,
    block,
    block::BlockIdFlag,
    Hash,
    Signature,
    Time,
};
use tendermint_proto::types::CanonicalVoteExtension;

use crate::{
    app::{
        benchmark_and_test_utils::{
            mock_balances,
            mock_tx_cost,
            proto_genesis_state,
            AppInitializer,
            BOB_ADDRESS,
        },
        test_utils::{
            get_alice_signing_key,
            get_judy_signing_key,
        },
        App,
        ShouldShutDown,
        StateReadExt as _,
    },
    authority::StateReadExt,
    benchmark_and_test_utils::{
        astria_address_from_hex_string,
        nria,
    },
    oracles::price_feed::oracle::state_ext::StateReadExt as _,
};

const PROPOSER_SEED: [u8; 32] = [1; 32];
const VALIDATOR_SEED: [u8; 32] = [2; 32];
const NON_VALIDATOR_SEED: [u8; 32] = [3; 32];
const NEW_CURRENCY_PAIR_0_PRICE: i128 = 1_000;
const NEW_CURRENCY_PAIR_1_PRICE: i128 = 1_001;

const TEST_VALIDATOR_UPDATE_SEED: [u8; 32] = [0; 32];
const TEST_VALIDATOR_UPDATE_NAME: &str = "test_validator";
const BLOCK_99_VALIDATOR_UPDATE_POWER: u32 = 10;
const BLOCK_100_VALIDATOR_UPDATE_POWER: u32 = 20;
const BLOCK_101_VALIDATOR_UPDATE_POWER: u32 = 0;

fn signed_transfer_tx(nonce: u32) -> Arc<Transaction> {
    let tx = TransactionBody::builder()
        .actions(vec![Transfer {
            to: astria_address_from_hex_string(BOB_ADDRESS),
            amount: 100_000,
            asset: nria().into(),
            fee_asset: nria().into(),
        }
        .into()])
        .chain_id("test")
        .nonce(nonce)
        .try_build()
        .unwrap();
    let alice = get_alice_signing_key();
    Arc::new(tx.sign(&alice))
}

fn signed_validator_update_tx(validator_update: ValidatorUpdate, nonce: u32) -> Arc<Transaction> {
    let tx = TransactionBody::builder()
        .actions(vec![validator_update.into()])
        .chain_id("test")
        .nonce(nonce)
        .try_build()
        .unwrap();
    let alice = get_judy_signing_key(); // Judy is the sudo address by default
    Arc::new(tx.sign(&alice))
}

struct PrepareProposalBuilder {
    local_last_commit: Option<ExtendedCommitInfo>,
    height: block::Height,
}

impl PrepareProposalBuilder {
    fn new() -> Self {
        Self {
            local_last_commit: None,
            height: block::Height::from(2_u8),
        }
    }

    fn with_extended_commit_info(mut self, value: ExtendedCommitInfo) -> Self {
        self.local_last_commit = Some(value);
        self
    }

    fn with_height<T: Into<block::Height>>(mut self, value: T) -> Self {
        self.height = value.into();
        self
    }

    fn build(self) -> PrepareProposal {
        PrepareProposal {
            max_tx_bytes: 22_019_254,
            txs: vec![],
            local_last_commit: self.local_last_commit,
            misbehavior: vec![],
            height: self.height,
            time: Time::from_unix_timestamp(1_736_692_723, 0).unwrap(),
            next_validators_hash: Hash::default(),
            proposer_address: account::Id::new(SigningKey::from(PROPOSER_SEED).address_bytes()),
        }
    }
}

fn create_block_hash(prepare_proposal_response: &response::PrepareProposal) -> Hash {
    let mut hasher = Sha256::new();
    for tx in &prepare_proposal_response.txs {
        hasher.update(tx.as_ref());
    }
    Hash::Sha256(hasher.finalize().into())
}

fn new_commit_info() -> CommitInfo {
    let votes = [PROPOSER_SEED, VALIDATOR_SEED]
        .into_iter()
        .map(|signing_key_seed| VoteInfo {
            validator: Validator {
                address: SigningKey::from(signing_key_seed).address_bytes(),
                power: 10_u8.into(),
            },
            sig_info: BlockSignatureInfo::Flag(BlockIdFlag::Commit),
        })
        .collect();
    CommitInfo {
        round: 0_u8.into(),
        votes,
    }
}

fn new_extended_commit_info(current_block_height: u32) -> ExtendedCommitInfo {
    let votes = [PROPOSER_SEED, VALIDATOR_SEED]
        .into_iter()
        .map(|signing_key_seed| {
            let signing_key = SigningKey::from(signing_key_seed);
            let vote_extension = new_vote_extension();
            // This is a vote extension from the previous block.
            let height = i64::from(current_block_height.saturating_sub(1));
            let canonical_vote_extension = CanonicalVoteExtension {
                extension: vote_extension.to_vec(),
                height,
                round: 0,
                chain_id: proto_genesis_state().chain_id.clone(),
            };
            let bytes_to_sign = canonical_vote_extension.encode_length_delimited_to_vec();
            let extension_signature = Some(
                Signature::try_from(signing_key.sign(&bytes_to_sign).to_bytes().as_ref()).unwrap(),
            );
            ExtendedVoteInfo {
                validator: Validator {
                    address: signing_key.address_bytes(),
                    power: 10_u8.into(),
                },
                sig_info: BlockSignatureInfo::Flag(BlockIdFlag::Commit),
                vote_extension,
                extension_signature,
            }
        })
        .collect();
    ExtendedCommitInfo {
        round: 0_u8.into(),
        votes,
    }
}

fn new_process_proposal(
    prepare_proposal_request: &PrepareProposal,
    prepare_proposal_response: &response::PrepareProposal,
) -> ProcessProposal {
    ProcessProposal {
        txs: prepare_proposal_response.txs.clone(),
        proposed_last_commit: Some(new_commit_info()),
        misbehavior: prepare_proposal_request.misbehavior.clone(),
        hash: create_block_hash(prepare_proposal_response),
        height: prepare_proposal_request.height,
        time: prepare_proposal_request.time,
        next_validators_hash: prepare_proposal_request.next_validators_hash,
        proposer_address: prepare_proposal_request.proposer_address,
    }
}

/// The app doesn't have a real oracle client for this test, and the `request::ExtendVote` arg is
/// ignored in `App::extend_vote` anyway, so we'll just create a fake `OracleVoteExtension` and
/// encode it as is done in `App::extend_vote`.
fn new_vote_extension() -> Bytes {
    let prices = [
        (
            CurrencyPairId::new(0),
            Price::new(NEW_CURRENCY_PAIR_0_PRICE),
        ),
        (
            CurrencyPairId::new(1),
            Price::new(NEW_CURRENCY_PAIR_1_PRICE),
        ),
    ]
    .into_iter()
    .collect();
    OracleVoteExtension {
        prices,
    }
    .into_raw()
    .encode_to_vec()
    .into()
}

fn new_verify_vote_extension(
    prepare_proposal_request: &PrepareProposal,
    prepare_proposal_response: &response::PrepareProposal,
    validator_address: [u8; 20],
) -> VerifyVoteExtension {
    VerifyVoteExtension {
        hash: create_block_hash(prepare_proposal_response),
        validator_address: account::Id::new(validator_address),
        height: prepare_proposal_request.height,
        vote_extension: new_vote_extension(),
    }
}

fn new_finalize_block(
    prepare_proposal_request: &PrepareProposal,
    prepare_proposal_response: &response::PrepareProposal,
) -> FinalizeBlock {
    FinalizeBlock {
        txs: prepare_proposal_response.txs.clone(),
        decided_last_commit: new_commit_info(),
        misbehavior: prepare_proposal_request.misbehavior.clone(),
        hash: create_block_hash(prepare_proposal_response),
        height: prepare_proposal_request.height,
        time: prepare_proposal_request.time,
        next_validators_hash: prepare_proposal_request.next_validators_hash,
        proposer_address: prepare_proposal_request.proposer_address,
    }
}

async fn latest_currency_pair_price(storage: Storage, index: usize) -> QuotePrice {
    let markets = UpgradesBuilder::new()
        .build()
        .aspen()
        .unwrap()
        .price_feed_change()
        .market_map_genesis()
        .market_map
        .markets
        .clone();
    assert!(index < markets.len());
    let currency_pair = markets
        .values()
        .skip(index)
        .map(|market| market.ticker.currency_pair.clone())
        .next()
        .unwrap();
    storage
        .latest_snapshot()
        .get_currency_pair_state(&currency_pair)
        .await
        .expect("should get currency pair state")
        .expect("currency pair state should be Some")
        .price
        .expect("price should be Some")
}

struct Node {
    app: App,
    storage: Storage,
    signing_key: SigningKey,
}

impl Node {
    async fn new(signing_key_seed: [u8; 32]) -> Self {
        let initial_validator_set = vec![
            ValidatorUpdate {
                power: 10,
                verification_key: SigningKey::from(PROPOSER_SEED).verification_key(),
                name: "test".parse().unwrap(),
            },
            ValidatorUpdate {
                power: 10,
                verification_key: SigningKey::from(VALIDATOR_SEED).verification_key(),
                name: "test".parse().unwrap(),
            },
        ];

        let (app, storage) = AppInitializer::new()
            .with_genesis_validators(initial_validator_set)
            .with_upgrades(UpgradesBuilder::new().set_aspen(Some(100)).build())
            .init()
            .await;
        Self {
            app,
            storage,
            signing_key: SigningKey::from(signing_key_seed),
        }
    }

    async fn prepare_proposal(
        &mut self,
        prepare_proposal: &PrepareProposal,
    ) -> Result<response::PrepareProposal> {
        self.app
            .prepare_proposal(prepare_proposal.clone(), self.storage.clone())
            .await
    }

    async fn process_proposal(&mut self, process_proposal: &ProcessProposal) -> Result<()> {
        self.app
            .process_proposal(process_proposal.clone(), self.storage.clone())
            .await
    }

    async fn verify_vote_extension(
        &mut self,
        verify_vote_extension: VerifyVoteExtension,
    ) -> Result<response::VerifyVoteExtension> {
        self.app.verify_vote_extension(verify_vote_extension).await
    }

    async fn finalize_block(
        &mut self,
        finalize_block: &FinalizeBlock,
    ) -> Result<response::FinalizeBlock> {
        self.app
            .finalize_block(finalize_block.clone(), self.storage.clone())
            .await
    }

    async fn commit(&mut self) -> Result<ShouldShutDown> {
        self.app.commit(self.storage.clone()).await
    }
}

/// Runs the abci calls for the execution of blocks at heights 99 through 102, where `Aspen` is
/// scheduled to activate at height 100.
///
/// There are three `App` instances, representing three separate nodes on the network.  The first
/// will be the proposer throughout.  The second is a validator, not used as a proposer.  The third
/// is a non-validating node, e.g. a validator that is syncing, or a "full node".
#[tokio::test]
async fn should_upgrade() {
    // Initialize `App`s with vote extensions disabled and with `Aspen` scheduled.
    let proposer = &mut Node::new(PROPOSER_SEED).await;
    let validator = &mut Node::new(VALIDATOR_SEED).await;
    let non_validator = &mut Node::new(NON_VALIDATOR_SEED).await;

    execute_block_99(proposer, validator, non_validator).await;
    execute_block_100(proposer, validator, non_validator).await;
    execute_block_101(proposer, validator, non_validator).await;
    execute_block_102(proposer, validator, non_validator).await;
}

/// Last block before upgrade - nothing special happens here.
async fn execute_block_99(proposer: &mut Node, validator: &mut Node, non_validator: &mut Node) {
    // Add transfer action
    proposer
        .app
        .mempool
        .insert(
            signed_transfer_tx(0),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    // Add validator update action
    let block_99_validator_update = ValidatorUpdate {
        power: BLOCK_99_VALIDATOR_UPDATE_POWER,
        verification_key: SigningKey::from(TEST_VALIDATOR_UPDATE_SEED).verification_key(),
        name: TEST_VALIDATOR_UPDATE_NAME.parse().unwrap(),
    };
    proposer
        .app
        .mempool
        .insert(
            signed_validator_update_tx(block_99_validator_update.clone(), 0),
            0,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    // Execute `PrepareProposal` for block 99 on the proposer.
    let prepare_proposal = PrepareProposalBuilder::new().with_height(99_u8).build();
    let prepare_proposal_response = proposer.prepare_proposal(&prepare_proposal).await.unwrap();
    // Check the response's `txs` are in the legacy form, i.e. not encoded `DataItem`s, and that the
    // tx inserted to the mempool has been added to the block.
    let expanded_block_data =
        ExpandedBlockData::new_from_untyped_data(&prepare_proposal_response.txs).unwrap();
    assert_eq!(2, expanded_block_data.user_submitted_transactions.len());

    // Execute `ProcessProposal` for block 99 on the proposer and on the non-proposing validator.
    let process_proposal = new_process_proposal(&prepare_proposal, &prepare_proposal_response);
    proposer.process_proposal(&process_proposal).await.unwrap();
    validator.process_proposal(&process_proposal).await.unwrap();

    // Execute `FinalizeBlock` for block 99 on all three nodes.
    let finalize_block = new_finalize_block(&prepare_proposal, &prepare_proposal_response);
    let finalize_block_response = proposer.finalize_block(&finalize_block).await.unwrap();
    assert_eq!(
        finalize_block_response,
        validator.finalize_block(&finalize_block).await.unwrap()
    );
    assert_eq!(
        finalize_block_response,
        non_validator.finalize_block(&finalize_block).await.unwrap()
    );
    // There should be four tx results: the two commitments and the two transactions.
    assert_eq!(4, finalize_block_response.tx_results.len());
    assert!(finalize_block_response.consensus_param_updates.is_none());

    // Execute `Commit` for block 99 on all three nodes.
    let _ = proposer.commit().await.unwrap();
    let _ = validator.commit().await.unwrap();
    let _ = non_validator.commit().await.unwrap();
    assert_eq!(proposer.app.app_hash, validator.app.app_hash);
    assert_eq!(proposer.app.app_hash, non_validator.app.app_hash);
    // There should be no currency pairs stored, and consensus params should not be in storage.
    let snapshot_99 = proposer.storage.latest_snapshot();
    assert_eq!(0, snapshot_99.get_num_currency_pairs().await.unwrap());
    assert!(snapshot_99
        .get_consensus_params()
        .await
        .expect("should get consensus params")
        .is_none());
    // Check that validator has been correctly added, and that no name has been stored pre-upgrade
    let validator_set = snapshot_99
        .pre_aspen_get_validator_set()
        .await
        .expect("should get validator set");
    assert_eq!(
        3,
        validator_set.len(),
        "should be 3 validators in validator set"
    );
    let existing_validator_update = validator_set
        .get(block_99_validator_update.verification_key.address_bytes())
        .expect("test validator should be in state")
        .to_owned();
    assert_eq!(
        (
            existing_validator_update.verification_key,
            existing_validator_update.power
        ),
        (
            block_99_validator_update.verification_key,
            block_99_validator_update.power
        ),
    );
    assert_eq!(existing_validator_update.name, ValidatorName::empty(),);
}

/// Upgrade should execute as part of this block, and the `vote_extensions_enable_height` should get
/// set to 101.
async fn execute_block_100(proposer: &mut Node, validator: &mut Node, non_validator: &mut Node) {
    // Add transfer action
    proposer
        .app
        .mempool
        .insert(
            signed_transfer_tx(1),
            1,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    // Add validator update action
    let block_100_validator_update = ValidatorUpdate {
        power: BLOCK_100_VALIDATOR_UPDATE_POWER,
        verification_key: SigningKey::from(TEST_VALIDATOR_UPDATE_SEED).verification_key(),
        name: TEST_VALIDATOR_UPDATE_NAME.parse().unwrap(),
    };
    proposer
        .app
        .mempool
        .insert(
            signed_validator_update_tx(block_100_validator_update.clone(), 1),
            1,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    // Execute `PrepareProposal` for block 100 on the proposer.
    let prepare_proposal = PrepareProposalBuilder::new().with_height(100_u8).build();
    let prepare_proposal_response = proposer.prepare_proposal(&prepare_proposal).await.unwrap();
    // Check the response's `txs` are in the new form, i.e. encoded `DataItem`s, that the upgrade
    // change hashes are included in them, and that the tx inserted to the mempool is also included.
    // Extended commit info will not be produced yet.
    let expanded_block_data =
        ExpandedBlockData::new_from_typed_data(&prepare_proposal_response.txs, false).unwrap();
    assert!(!expanded_block_data.upgrade_change_hashes.is_empty());
    assert_eq!(2, expanded_block_data.user_submitted_transactions.len());

    // Execute `ProcessProposal` for block 100 on the proposer and on the non-proposing validator.
    let process_proposal = new_process_proposal(&prepare_proposal, &prepare_proposal_response);
    proposer.process_proposal(&process_proposal).await.unwrap();
    validator.process_proposal(&process_proposal).await.unwrap();

    // Execute `FinalizeBlock` for block 100 on all three nodes.
    let finalize_block = new_finalize_block(&prepare_proposal, &prepare_proposal_response);
    let finalize_block_response = proposer.finalize_block(&finalize_block).await.unwrap();
    assert_eq!(
        finalize_block_response,
        validator.finalize_block(&finalize_block).await.unwrap()
    );
    assert_eq!(
        finalize_block_response,
        non_validator.finalize_block(&finalize_block).await.unwrap()
    );
    // There should be five tx results: the two commitments, the upgrade change hashes and the two
    // transactions.
    assert_eq!(5, finalize_block_response.tx_results.len());
    // The consensus params should be `Some`, with `vote_extensions_enable_height` set to 101.
    assert_eq!(
        Some(block::Height::from(101_u8)),
        finalize_block_response
            .consensus_param_updates
            .unwrap()
            .abci
            .vote_extensions_enable_height
    );

    // Execute `Commit` for block 100 on all three nodes.
    let _ = proposer.commit().await.unwrap();
    let _ = validator.commit().await.unwrap();
    let _ = non_validator.commit().await.unwrap();
    assert_eq!(proposer.app.app_hash, validator.app.app_hash);
    assert_eq!(proposer.app.app_hash, non_validator.app.app_hash);
    // There should be two currency pairs now stored, and `vote_extensions_enable_height` in storage
    // should be 101.
    let snapshot_100 = proposer.storage.latest_snapshot();
    assert_eq!(2, snapshot_100.get_num_currency_pairs().await.unwrap());
    assert_eq!(
        Some(block::Height::from(101_u8)),
        snapshot_100
            .get_consensus_params()
            .await
            .expect("should get consensus params")
            .expect("consensus params should be Some")
            .abci
            .vote_extensions_enable_height
    );

    // Check that validator set is no longer in use
    let _ = snapshot_100
        .pre_aspen_get_validator_set()
        .await
        .expect_err("validator set should no longer exist in state");

    // Check that validator has been correctly added, and that name has been stored post-upgrade
    let validator_in_state = snapshot_100
        .get_validator(block_100_validator_update.verification_key.address_bytes())
        .await
        .unwrap()
        .expect("test validator should be in state");
    assert_eq!(block_100_validator_update, validator_in_state);
    assert_eq!(snapshot_100.get_validator_count().await.unwrap(), 3);
}

/// This will be the first block where `ExtendVote` and `VerifyVoteExtension` will be called. No
/// vote extension will be available in `PrepareProposal` until the next block.
async fn execute_block_101(proposer: &mut Node, validator: &mut Node, non_validator: &mut Node) {
    // Add transfer action
    proposer
        .app
        .mempool
        .insert(
            signed_transfer_tx(2),
            2,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    // Add validator update action
    let block_101_validator_update = ValidatorUpdate {
        power: BLOCK_101_VALIDATOR_UPDATE_POWER,
        verification_key: SigningKey::from(TEST_VALIDATOR_UPDATE_SEED).verification_key(),
        name: TEST_VALIDATOR_UPDATE_NAME.parse().unwrap(),
    };
    proposer
        .app
        .mempool
        .insert(
            signed_validator_update_tx(block_101_validator_update.clone(), 2),
            2,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();

    // Execute `PrepareProposal` for block 101 on the proposer.
    let prepare_proposal = PrepareProposalBuilder::new().with_height(101_u8).build();
    let prepare_proposal_response = proposer.prepare_proposal(&prepare_proposal).await.unwrap();
    // Check the response's `txs` are in the new form, i.e. encoded `DataItem`s, that no extended
    // commit info is provided, and that the tx inserted to the mempool is also included.
    let expanded_block_data =
        ExpandedBlockData::new_from_typed_data(&prepare_proposal_response.txs, false).unwrap();
    assert!(expanded_block_data.upgrade_change_hashes.is_empty());
    assert_eq!(2, expanded_block_data.user_submitted_transactions.len());

    // Execute `ProcessProposal` for block 101 on the proposer and on the non-proposing validator.
    let process_proposal = new_process_proposal(&prepare_proposal, &prepare_proposal_response);
    proposer.process_proposal(&process_proposal).await.unwrap();
    validator.process_proposal(&process_proposal).await.unwrap();

    // `ExtendVote` for block 101 would be called at this stage on the proposer and on the
    // non-proposing validator.  We just use a fake vote extension since we don't have an oracle
    // client in the test.
    //
    // Execute `VerifyVoteExtension` for block 101 on the proposer using the non-proposing
    // validator's address and vice-versa.
    let verify_request = new_verify_vote_extension(
        &prepare_proposal,
        &prepare_proposal_response,
        validator.signing_key.address_bytes(),
    );
    let verify_response = proposer
        .verify_vote_extension(verify_request)
        .await
        .unwrap();
    assert_eq!(response::VerifyVoteExtension::Accept, verify_response);
    let verify_request = new_verify_vote_extension(
        &prepare_proposal,
        &prepare_proposal_response,
        proposer.signing_key.address_bytes(),
    );
    let verify_response = validator
        .verify_vote_extension(verify_request)
        .await
        .unwrap();
    assert_eq!(response::VerifyVoteExtension::Accept, verify_response);

    // Execute `FinalizeBlock` for block 101 on all three nodes.
    let finalize_block = new_finalize_block(&prepare_proposal, &prepare_proposal_response);
    let finalize_block_response = proposer.finalize_block(&finalize_block).await.unwrap();
    assert_eq!(
        finalize_block_response,
        validator.finalize_block(&finalize_block).await.unwrap()
    );
    assert_eq!(
        finalize_block_response,
        non_validator.finalize_block(&finalize_block).await.unwrap()
    );
    // There should be three tx results: the two commitments and the two transactions.
    assert_eq!(4, finalize_block_response.tx_results.len());
    // The consensus params should be `None`.
    assert!(finalize_block_response.consensus_param_updates.is_none());

    // Execute `Commit` for block 101 on all three nodes.
    let _ = proposer.commit().await.unwrap();
    let _ = validator.commit().await.unwrap();
    let _ = non_validator.commit().await.unwrap();
    assert_eq!(proposer.app.app_hash, validator.app.app_hash);
    assert_eq!(proposer.app.app_hash, non_validator.app.app_hash);

    // Check that validator has been correctly removed
    let snapshot_101 = proposer.storage.latest_snapshot();
    assert!(
        snapshot_101
            .get_validator(block_101_validator_update.verification_key.address_bytes())
            .await
            .unwrap()
            .is_none(),
        "test validator should be removed from state"
    );
    assert_eq!(snapshot_101.get_validator_count().await.unwrap(), 2);
}

/// This will be the first block where the previous block's vote extensions will be available in
/// `PrepareProposal`.
async fn execute_block_102(proposer: &mut Node, validator: &mut Node, non_validator: &mut Node) {
    // Fetch the two currency pairs' stored prices before executing this block.
    let currency_pair_0_price = latest_currency_pair_price(proposer.storage.clone(), 0).await;
    let currency_pair_1_price = latest_currency_pair_price(proposer.storage.clone(), 1).await;
    assert_ne!(NEW_CURRENCY_PAIR_0_PRICE, currency_pair_0_price.price.get());
    assert_ne!(NEW_CURRENCY_PAIR_1_PRICE, currency_pair_1_price.price.get());
    assert_eq!(0, currency_pair_0_price.block_height);
    assert_eq!(0, currency_pair_1_price.block_height);

    // Execute `PrepareProposal` for block 102 on the proposer.
    proposer
        .app
        .mempool
        .insert(
            signed_transfer_tx(3),
            3,
            mock_balances(0, 0),
            mock_tx_cost(0, 0, 0),
        )
        .await
        .unwrap();
    let prepare_proposal = PrepareProposalBuilder::new()
        .with_height(102_u8)
        .with_extended_commit_info(new_extended_commit_info(102))
        .build();
    let prepare_proposal_response = proposer.prepare_proposal(&prepare_proposal).await.unwrap();
    // Check the response's `txs` are in the new form, i.e. encoded `DataItem`s, that extended
    // commit info is provided, and that the tx inserted to the mempool is also included.
    let expanded_block_data =
        ExpandedBlockData::new_from_typed_data(&prepare_proposal_response.txs, true).unwrap();
    assert!(expanded_block_data.upgrade_change_hashes.is_empty());
    assert!(expanded_block_data
        .extended_commit_info_with_proof
        .is_some());
    assert_eq!(1, expanded_block_data.user_submitted_transactions.len());

    // Execute `ProcessProposal` for block 102 on the proposer and on the non-proposing validator.
    let process_proposal = new_process_proposal(&prepare_proposal, &prepare_proposal_response);
    proposer.process_proposal(&process_proposal).await.unwrap();
    validator.process_proposal(&process_proposal).await.unwrap();

    // `ExtendVote` and `VerifyVoteExtension` for block 102 would be called at this stage on the
    // proposer and on the non-proposing validator, but there's no need to do that here as this is
    // the last block of the test.
    //
    // Execute `FinalizeBlock` for block 102 on all three nodes.
    let finalize_block = new_finalize_block(&prepare_proposal, &prepare_proposal_response);
    let finalize_block_response = proposer.finalize_block(&finalize_block).await.unwrap();
    assert_eq!(
        finalize_block_response,
        validator.finalize_block(&finalize_block).await.unwrap()
    );
    assert_eq!(
        finalize_block_response,
        non_validator.finalize_block(&finalize_block).await.unwrap()
    );
    // There should be four tx results: the two commitments, the extended commit info and the
    // rollup tx.
    assert_eq!(4, finalize_block_response.tx_results.len());
    // The consensus params should be `None`.
    assert!(finalize_block_response.consensus_param_updates.is_none());

    // Execute `Commit` for block 102 on all three nodes.
    let _ = proposer.commit().await.unwrap();
    let _ = validator.commit().await.unwrap();
    let _ = non_validator.commit().await.unwrap();
    assert_eq!(proposer.app.app_hash, validator.app.app_hash);
    assert_eq!(proposer.app.app_hash, non_validator.app.app_hash);

    // Ensure the currency pair prices have been updated.
    let currency_pair_0_price = latest_currency_pair_price(proposer.storage.clone(), 0).await;
    let currency_pair_1_price = latest_currency_pair_price(proposer.storage.clone(), 1).await;
    assert_eq!(NEW_CURRENCY_PAIR_0_PRICE, currency_pair_0_price.price.get());
    assert_eq!(NEW_CURRENCY_PAIR_1_PRICE, currency_pair_1_price.price.get());
    assert_eq!(102, currency_pair_0_price.block_height);
    assert_eq!(102, currency_pair_1_price.block_height);
}
