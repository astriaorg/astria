use std::{
    sync::Arc,
    time::Duration,
};

use ethers::{
    abi::Tokenizable,
    core::utils::Anvil,
    prelude::*,
    utils::AnvilInstance,
};

use crate::withdrawer::ethereum::{
    astria_bridgeable_erc20::{
        ASTRIABRIDGEABLEERC20_ABI,
        ASTRIABRIDGEABLEERC20_BYTECODE,
    },
    astria_withdrawer::{
        ASTRIAWITHDRAWER_ABI,
        ASTRIAWITHDRAWER_BYTECODE,
    },
};

#[derive(Default)]
pub(crate) struct ConfigureAstriaWithdrawerDeployer {
    pub(crate) base_chain_asset_precision: u32,
}

impl ConfigureAstriaWithdrawerDeployer {
    pub(crate) async fn deploy(
        &mut self,
    ) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
        if self.base_chain_asset_precision == 0 {
            self.base_chain_asset_precision = 18;
        }
        deploy_astria_withdrawer(self.base_chain_asset_precision.into()).await
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

    let abi = ASTRIAWITHDRAWER_ABI.clone();
    let bytecode = ASTRIAWITHDRAWER_BYTECODE.to_vec();

    // deploy contract with ASSET_WITHDRAWAL_DECIMALS as 0
    let factory = ContractFactory::new(abi.clone(), bytecode.into(), signer.into());
    let contract = factory
        .deploy(base_chain_asset_precision)
        .unwrap()
        .send()
        .await
        .unwrap();
    let contract_address = contract.address();

    (
        contract_address,
        provider,
        wallet.with_chain_id(anvil.chain_id()),
        anvil,
    )
}

#[derive(Default)]
pub(crate) struct ConfigureAstriaBridgeableERC20Deployer {
    pub(crate) bridge_address: Address,
    pub(crate) base_chain_asset_precision: u32,
    pub(crate) name: String,
    pub(crate) symbol: String,
}

impl ConfigureAstriaBridgeableERC20Deployer {
    pub(crate) async fn deploy(self) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
        let Self {
            bridge_address,
            base_chain_asset_precision,
            mut name,
            mut symbol,
        } = self;

        if name.is_empty() {
            name = "test-token".to_string();
        }

        if symbol.is_empty() {
            symbol = "TT".to_string();
        }

        deploy_astria_bridgeable_erc20(
            bridge_address,
            base_chain_asset_precision.into(),
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
