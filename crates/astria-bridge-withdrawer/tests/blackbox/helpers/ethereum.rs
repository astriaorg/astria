use std::{
    sync::Arc,
    time::Duration,
};

use astria_bridge_contracts::{
    astria_bridgeable_erc20::{
        AstriaBridgeableERC20,
        ASTRIABRIDGEABLEERC20_ABI,
        ASTRIABRIDGEABLEERC20_BYTECODE,
    },
    astria_withdrawer::{
        AstriaWithdrawer,
        ASTRIAWITHDRAWER_ABI,
        ASTRIAWITHDRAWER_BYTECODE,
    },
};
use astria_core::primitive::v1::Address as AstriaAddress;
use ethers::{
    abi::Tokenizable,
    core::utils::Anvil,
    prelude::*,
    signers::Signer,
    utils::AnvilInstance,
};
use tracing::debug;

use super::test_bridge_withdrawer::{
    astria_address,
    default_bridge_address,
    default_native_asset,
};

pub struct TestEthereum {
    contract_address: ethers::types::Address,
    provider: Arc<Provider<Ws>>,
    wallet: LocalWallet,
    anvil: AnvilInstance,
}

impl TestEthereum {
    pub fn wallet_addr(&self) -> ethers::types::Address {
        self.wallet.address()
    }

    pub fn contract_address(&self) -> String {
        hex::encode(self.contract_address)
    }

    pub fn ws_endpoint(&self) -> String {
        self.anvil.ws_endpoint()
    }

    pub async fn send_sequencer_withdraw_transaction(
        &self,
        value: U256,
        recipient: AstriaAddress,
    ) -> TransactionReceipt {
        let signer = Arc::new(SignerMiddleware::new(
            self.provider.clone(),
            self.wallet.clone(),
        ));
        let contract = AstriaWithdrawer::new(self.contract_address, signer.clone());
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

        debug!(transaction.hash = %receipt.transaction_hash, "sequencer withdraw tranasaction successfully submitted");

        assert!(
            receipt.status == Some(ethers::types::U64::from(1)),
            "`withdraw` transaction failed: {receipt:?}",
        );

        receipt
    }

    pub async fn send_ics20_withdraw_transaction(
        &self,
        value: U256,
        recipient: String,
    ) -> TransactionReceipt {
        let signer = Arc::new(SignerMiddleware::new(
            self.provider.clone(),
            self.wallet.clone(),
        ));
        let contract = AstriaWithdrawer::new(self.contract_address, signer.clone());
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

    pub async fn mint_tokens(
        &self,
        amount: U256,
        recipient: ethers::types::Address,
    ) -> TransactionReceipt {
        let signer = Arc::new(SignerMiddleware::new(
            self.provider.clone(),
            self.wallet.clone(),
        ));
        let contract = AstriaBridgeableERC20::new(self.contract_address, signer.clone());
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

    pub async fn send_sequencer_withdraw_transaction_erc20(
        &self,
        value: U256,
        recipient: AstriaAddress,
    ) -> TransactionReceipt {
        let signer = Arc::new(SignerMiddleware::new(
            self.provider.clone(),
            self.wallet.clone(),
        ));
        let contract = AstriaBridgeableERC20::new(self.contract_address, signer.clone());
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

    pub async fn send_ics20_withdraw_transaction_astria_bridgeable_erc20(
        &self,
        value: U256,
        recipient: String,
    ) -> TransactionReceipt {
        let signer = Arc::new(SignerMiddleware::new(
            self.provider.clone(),
            self.wallet.clone(),
        ));
        let contract = AstriaBridgeableERC20::new(self.contract_address, signer.clone());
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
}

pub enum TestEthereumConfig {
    AstriaWithdrawer(AstriaWithdrawerDeployerConfig),
    AstriaBridgeableERC20(AstriaBridgeableERC20DeployerConfig),
}

impl TestEthereumConfig {
    pub(crate) async fn spawn(self) -> TestEthereum {
        match self {
            Self::AstriaWithdrawer(config) => {
                let (contract_address, provider, wallet, anvil) = config.deploy().await;
                TestEthereum {
                    contract_address,
                    provider,
                    wallet,
                    anvil,
                }
            }
            Self::AstriaBridgeableERC20(config) => {
                let (contract_address, provider, wallet, anvil) = config.deploy().await;
                TestEthereum {
                    contract_address,
                    provider,
                    wallet,
                    anvil,
                }
            }
        }
    }
}

#[expect(
    clippy::struct_field_names,
    reason = "we want struct field names to be specific"
)]
pub struct AstriaWithdrawerDeployerConfig {
    pub base_chain_asset_precision: u32,
    pub base_chain_bridge_address: astria_core::primitive::v1::Address,
    pub base_chain_asset_denomination: String,
}

impl Default for AstriaWithdrawerDeployerConfig {
    fn default() -> Self {
        Self {
            base_chain_asset_precision: 18,
            base_chain_bridge_address: default_bridge_address(),
            base_chain_asset_denomination: default_native_asset().to_string(),
        }
    }
}

impl AstriaWithdrawerDeployerConfig {
    pub async fn deploy(self) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
        let Self {
            base_chain_asset_precision,
            base_chain_bridge_address,
            base_chain_asset_denomination,
        } = self;

        deploy_astria_withdrawer(
            base_chain_asset_precision.into(),
            base_chain_bridge_address,
            base_chain_asset_denomination,
        )
        .await
    }
}

/// Starts a local anvil instance and deploys the `AstriaWithdrawer` contract to it.
///
/// Returns the contract address, provider, wallet, and anvil instance.
///
/// # Panics
///
/// - if the provider fails to connect to the anvil instance
/// - if the contract fails to deploy
pub(crate) async fn deploy_astria_withdrawer(
    base_chain_asset_precision: U256,
    base_chain_bridge_address: astria_core::primitive::v1::Address,
    base_chain_asset_denomination: String,
) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
    // setup anvil and signing wallet
    let anvil = Anvil::new().block_time(1u64).spawn();
    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let provider = Arc::new(
        Provider::<Ws>::connect(anvil.ws_endpoint())
            .await
            .unwrap()
            .interval(Duration::from_millis(10u64)),
    );
    let signer = SignerMiddleware::new(
        provider.clone(),
        wallet.clone().with_chain_id(anvil.chain_id()),
    );

    let abi = ASTRIAWITHDRAWER_ABI.clone();
    let bytecode = ASTRIAWITHDRAWER_BYTECODE.to_vec();

    let args = vec![
        base_chain_asset_precision.into_token(),
        base_chain_bridge_address.to_string().into_token(),
        base_chain_asset_denomination.into_token(),
    ];

    let factory = ContractFactory::new(abi.clone(), bytecode.into(), signer.into());
    let contract = factory.deploy_tokens(args).unwrap().send().await.unwrap();
    let contract_address = contract.address();

    (
        contract_address,
        provider,
        wallet.with_chain_id(anvil.chain_id()),
        anvil,
    )
}

pub struct AstriaBridgeableERC20DeployerConfig {
    pub bridge_address: Address,
    pub base_chain_asset_precision: u32,
    pub base_chain_bridge_address: astria_core::primitive::v1::Address,
    pub base_chain_asset_denomination: String,
    pub name: String,
    pub symbol: String,
}

impl Default for AstriaBridgeableERC20DeployerConfig {
    fn default() -> Self {
        Self {
            bridge_address: Address::zero(),
            base_chain_asset_precision: 18,
            base_chain_bridge_address: astria_address([0u8; 20]),
            base_chain_asset_denomination: "testdenom".to_string(),
            name: "test-token".to_string(),
            symbol: "TT".to_string(),
        }
    }
}

impl AstriaBridgeableERC20DeployerConfig {
    pub(crate) async fn deploy(self) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
        let Self {
            bridge_address,
            base_chain_asset_precision,
            base_chain_bridge_address,
            base_chain_asset_denomination,
            name,
            symbol,
        } = self;

        deploy_astria_bridgeable_erc20(
            bridge_address,
            base_chain_asset_precision.into(),
            base_chain_bridge_address,
            base_chain_asset_denomination,
            name,
            symbol,
        )
        .await
    }
}

/// Starts a local anvil instance and deploys the `AstriaBridgeableERC20` contract to it.
///
/// Returns the contract address, provider, wallet, and anvil instance.
///
/// # Panics
///
/// - if the provider fails to connect to the anvil instance
/// - if the contract fails to deploy
pub(crate) async fn deploy_astria_bridgeable_erc20(
    mut bridge_address: Address,
    base_chain_asset_precision: ethers::abi::Uint,
    base_chain_bridge_address: astria_core::primitive::v1::Address,
    base_chain_asset_denomination: String,
    name: String,
    symbol: String,
) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
    // setup anvil and signing wallet
    let anvil = Anvil::new().spawn();
    let wallet: LocalWallet = anvil.keys()[0].clone().into();
    let provider = Arc::new(
        Provider::<Ws>::connect(anvil.ws_endpoint())
            .await
            .unwrap()
            .interval(Duration::from_millis(10u64)),
    );
    let signer = SignerMiddleware::new(
        provider.clone(),
        wallet.clone().with_chain_id(anvil.chain_id()),
    );

    let abi = ASTRIABRIDGEABLEERC20_ABI.clone();
    let bytecode = ASTRIABRIDGEABLEERC20_BYTECODE.to_vec();

    let factory = ContractFactory::new(abi.clone(), bytecode.into(), signer.into());

    if bridge_address == Address::zero() {
        bridge_address = wallet.address();
    }
    let args = vec![
        bridge_address.into_token(),
        base_chain_asset_precision.into_token(),
        base_chain_bridge_address.to_string().into_token(),
        base_chain_asset_denomination.into_token(),
        name.into_token(),
        symbol.into_token(),
    ];
    let contract = factory.deploy_tokens(args).unwrap().send().await.unwrap();
    let contract_address = contract.address();

    (
        contract_address,
        provider,
        wallet.with_chain_id(anvil.chain_id()),
        anvil,
    )
}

#[must_use]
pub fn default_rollup_address() -> ethers::types::Address {
    Address::random()
}
