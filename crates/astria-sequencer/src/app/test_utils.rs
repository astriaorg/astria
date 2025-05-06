use std::{
    cmp::Ordering,
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::RollupId,
    protocol::transaction::v1::{
        action::{
            group::Group,
            FeeAssetChange,
            InitBridgeAccount,
            RollupDataSubmission,
            SudoAddressChange,
        },
        Action,
        Transaction,
        TransactionBody,
    },
    sequencerblock::v1::{
        block::Deposit,
        DataItem,
    },
    upgrades::v1::Change,
    Protobuf,
};
use bytes::Bytes;
use cnidarium::Storage;
use sha2::Digest as _;
use tendermint::{
    abci,
    abci::types::CommitInfo,
    block::{
        Height,
        Round,
    },
    Hash,
    Time,
};

use crate::{
    app::{
        benchmark_and_test_utils::{
            denom_0,
            JUDY_ADDRESS,
        },
        App,
    },
    benchmark_and_test_utils::astria_address_from_hex_string,
    proposal::commitment::generate_rollup_datas_commitment,
};

pub(crate) fn get_alice_signing_key() -> SigningKey {
    // this secret key corresponds to ALICE_ADDRESS
    let alice_secret_bytes: [u8; 32] =
        hex::decode("2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(alice_secret_bytes)
}

pub(crate) fn get_bob_signing_key() -> SigningKey {
    // this secret key corresponds to BOB_ADDRESS
    let bob_secret_bytes: [u8; 32] =
        hex::decode("b70fd3b99cab2d98dbd73602deb026b9cdc9bb7b85d35f0bbb81b17c78923dd0")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(bob_secret_bytes)
}

pub(crate) fn get_carol_signing_key() -> SigningKey {
    // this secret key corresponds to CAROL_ADDRESS
    let carol_secret_bytes: [u8; 32] =
        hex::decode("0e951afdcbefc420fe6f71b82b0c28c11eb6ee5d95be0886ce9dbf6fa512debc")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(carol_secret_bytes)
}

pub(crate) fn get_judy_signing_key() -> SigningKey {
    // this secret key corresponds to JUDY_ADDRESS
    let judy_secret_bytes: [u8; 32] =
        hex::decode("3b2a05a2168952a102dcc07f39b9e385a45b9c2a9b6e3d06acf46fb39fd14019")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(judy_secret_bytes)
}

pub(crate) fn get_bridge_signing_key() -> SigningKey {
    let bridge_secret_bytes: [u8; 32] =
        hex::decode("db4982e01f3eba9e74ac35422fcd49aa2b47c3c535345c7e7da5220fe3a0ce79")
            .unwrap()
            .try_into()
            .unwrap();
    SigningKey::from(bridge_secret_bytes)
}

pub(crate) struct MockTxBuilder {
    nonce: u32,
    signer: SigningKey,
    chain_id: String,
    group: Group,
}

impl MockTxBuilder {
    pub(crate) fn new() -> Self {
        Self {
            chain_id: "test".to_string(),
            nonce: 0,
            signer: get_alice_signing_key(),
            group: Group::BundleableGeneral,
        }
    }

    pub(crate) fn nonce(self, nonce: u32) -> Self {
        Self {
            nonce,
            ..self
        }
    }

    pub(crate) fn signer(self, signer: SigningKey) -> Self {
        Self {
            signer,
            ..self
        }
    }

    pub(crate) fn chain_id(self, chain_id: &str) -> Self {
        Self {
            chain_id: chain_id.to_string(),
            ..self
        }
    }

    pub(crate) fn group(self, group: Group) -> Self {
        Self {
            group,
            ..self
        }
    }

    pub(crate) fn build(self) -> Arc<Transaction> {
        let action: Action = match self.group {
            Group::BundleableGeneral => RollupDataSubmission {
                rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
                data: Bytes::from_static(&[0x99]),
                fee_asset: denom_0(),
            }
            .into(),
            Group::UnbundleableGeneral => InitBridgeAccount {
                rollup_id: RollupId::from_unhashed_bytes("rollup-id"),
                asset: denom_0(),
                fee_asset: denom_0(),
                sudo_address: None,
                withdrawer_address: None,
            }
            .into(),
            Group::BundleableSudo => FeeAssetChange::Addition(denom_0()).into(),
            Group::UnbundleableSudo => SudoAddressChange {
                new_address: astria_address_from_hex_string(JUDY_ADDRESS),
            }
            .into(),
        };

        assert!(
            action.group() == self.group,
            "action group mismatch: wanted {:?}, got {:?}",
            self.group,
            action.group()
        );

        let tx = TransactionBody::builder()
            .actions(vec![action])
            .chain_id(self.chain_id)
            .nonce(self.nonce)
            .try_build()
            .unwrap();

        Arc::new(tx.sign(&self.signer))
    }
}

pub(crate) fn transactions_with_extended_commit_info_and_commitments(
    block_height: tendermint::block::Height,
    txs: &[Arc<Transaction>],
    deposits: Option<HashMap<RollupId, Vec<Deposit>>>,
) -> Vec<Bytes> {
    use astria_core::protocol::price_feed::v1::ExtendedCommitInfoWithCurrencyPairMapping;
    use prost::Message as _;

    use crate::proposal::commitment::generate_rollup_datas_commitment;

    // If vote extensions are enabled at block height 1 (the minimum possible), then the first
    // block to include extended commit info is at height 2.
    assert!(
        block_height > tendermint::block::Height::from(1_u8),
        "extended commit info can only be applied to block height 2 or greater"
    );

    let extended_commit_info = ExtendedCommitInfoWithCurrencyPairMapping::empty(0u16.into());
    let encoded_extended_commit_info =
        DataItem::ExtendedCommitInfo(extended_commit_info.into_raw().encode_to_vec().into())
            .encode();
    let commitments = generate_rollup_datas_commitment::<true>(txs, deposits.unwrap_or_default());
    let txs_with_commit_info: Vec<Bytes> = commitments
        .into_iter()
        .chain(std::iter::once(encoded_extended_commit_info))
        .chain(txs.iter().map(|tx| tx.to_raw().encode_to_vec().into()))
        .collect();
    txs_with_commit_info
}

pub(crate) fn default_consensus_params() -> tendermint::consensus::Params {
    tendermint::consensus::Params {
        block: tendermint::block::Size {
            max_bytes: 22_020_096,
            max_gas: -1,
            time_iota_ms: 1000,
        },
        evidence: tendermint::evidence::Params {
            max_age_num_blocks: 100_000,
            max_age_duration: tendermint::evidence::Duration(std::time::Duration::from_secs(
                172_800_000_000_000,
            )),
            max_bytes: 1_048_576,
        },
        validator: tendermint::consensus::params::ValidatorParams {
            pub_key_types: vec![tendermint::public_key::Algorithm::Ed25519],
        },
        version: Some(tendermint::consensus::params::VersionParams {
            app: 0,
        }),
        abci: tendermint::consensus::params::AbciParams {
            vote_extensions_enable_height: Some(tendermint::block::Height::from(1_u8)),
        },
    }
}

/// Repeatedly executes `App::finalize_block` and `App::commit` until one block after the Aspen
/// upgrade has been applied.
///
/// Returns the height of the next block to execute.
///
/// Panics if the Aspen upgrade is not included in the app's upgrade handler (is set by default to
/// activate at block 1 via `AppInitializer`), or if its activation height is greater than 10.
pub(crate) async fn run_until_aspen_applied(app: &mut App, storage: Storage) -> Height {
    let aspen = app
        .upgrades_handler
        .upgrades()
        .aspen()
        .expect("upgrades should contain aspen upgrade")
        .clone();
    assert!(
        aspen.activation_height() <= 10,
        "activation height must be <= 10; don't want to execute too many blocks for unit test"
    );

    let proposer_address: tendermint::account::Id = [99u8; 20].to_vec().try_into().unwrap();
    let time = Time::from_unix_timestamp(1_744_036_762, 123_456_789).unwrap();

    let final_block_height = aspen.activation_height().checked_add(1).unwrap();
    for height in 1..=final_block_height {
        let txs = match height.cmp(&aspen.activation_height()) {
            Ordering::Less => {
                // Use the legacy form of rollup data commitments.
                generate_rollup_datas_commitment::<false>(&[], HashMap::new())
                    .into_iter()
                    .collect()
            }
            Ordering::Equal => {
                // Use the new (`DataItem`) form of rollup data commitments, and append the upgrade
                // change hashes.
                let upgrade_change_hashes = DataItem::UpgradeChangeHashes(
                    aspen.changes().map(Change::calculate_hash).collect(),
                );
                generate_rollup_datas_commitment::<true>(&[], HashMap::new())
                    .into_iter()
                    .chain(Some(upgrade_change_hashes.encode()))
                    .collect()
            }
            Ordering::Greater => {
                // Use the new (`DataItem`) form of rollup data commitments. Note the first block
                // after Aspen doesn't have extended commit info. All blocks after
                // that should have it.
                generate_rollup_datas_commitment::<true>(&[], HashMap::new())
                    .into_iter()
                    .collect()
            }
        };
        let finalize_block = abci::request::FinalizeBlock {
            hash: Hash::Sha256(sha2::Sha256::digest(height.to_le_bytes()).into()),
            height: Height::try_from(height).unwrap(),
            time: time.checked_add(Duration::from_secs(height)).unwrap(),
            next_validators_hash: Hash::default(),
            proposer_address,
            txs,
            decided_last_commit: CommitInfo {
                votes: vec![],
                round: Round::default(),
            },
            misbehavior: vec![],
        };
        app.finalize_block(finalize_block, storage.clone())
            .await
            .unwrap();
        app.commit(storage.clone()).await.unwrap();
    }
    Height::try_from(
        final_block_height
            .checked_add(1)
            .expect("should increment final block height"),
    )
    .expect("should convert to height")
}
