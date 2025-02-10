use std::sync::Arc;

use astria_core::{
    crypto::SigningKey,
    primitive::v1::RollupId,
    protocol::{
        genesis::v1::GenesisAppState,
        transaction::v1::{
            action::{
                group::Group,
                FeeAssetChange,
                InitBridgeAccount,
                RollupDataSubmission,
                SudoAddressChange,
                ValidatorUpdate,
            },
            Action,
            Transaction,
            TransactionBody,
        },
    },
};
use bytes::Bytes;

use crate::{
    app::{
        benchmark_and_test_utils::{
            denom_0,
            initialize_app_with_storage,
            JUDY_ADDRESS,
        },
        App,
    },
    benchmark_and_test_utils::astria_address_from_hex_string,
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

pub(crate) async fn initialize_app(
    genesis_state: Option<GenesisAppState>,
    genesis_validators: Vec<ValidatorUpdate>,
) -> App {
    let (app, _storage) = initialize_app_with_storage(genesis_state, genesis_validators).await;
    app
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
