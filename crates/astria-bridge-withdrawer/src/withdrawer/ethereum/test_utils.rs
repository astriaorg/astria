use std::{
    sync::Arc,
    time::Duration,
};

use ethers::{
    core::utils::Anvil,
    prelude::*,
    utils::AnvilInstance,
};

use crate::withdrawer::ethereum::astria_withdrawer::{
    ASTRIAWITHDRAWER_ABI,
    ASTRIAWITHDRAWER_BYTECODE,
};

/// Starts a local anvil instance and deploys the `AstriaWithdrawer` contract to it.
///
/// Returns the contract address, provider, wallet, and anvil instance.
///
/// # Panics
///
/// - if the contract cannot be found in the expected path
/// - if the contract cannot be compiled
/// - if the provider fails to connect to the anvil instance
/// - if the contract fails to deploy
pub(crate) async fn deploy_astria_withdrawer()
-> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
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
    let contract = factory.deploy(U256::from(0)).unwrap().send().await.unwrap();
    let contract_address = contract.address();

    (
        contract_address,
        provider,
        wallet.with_chain_id(anvil.chain_id()),
        anvil,
    )
}
