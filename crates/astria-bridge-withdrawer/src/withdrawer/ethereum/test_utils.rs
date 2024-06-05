use std::{
    path::Path,
    sync::Arc,
    time::Duration,
};

use ethers::{
    core::utils::Anvil,
    prelude::*,
    utils::AnvilInstance,
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
/// - if the contract cannot be found in the expected path
/// - if the contract cannot be compiled
/// - if the provider fails to connect to the anvil instance
/// - if the contract fails to deploy
pub(crate) async fn deploy_astria_withdrawer(
    base_chain_asset_precision: U256,
) -> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
    // compile contract for testing
    let source = Path::new(&env!("CARGO_MANIFEST_DIR")).join("ethereum/src/AstriaWithdrawer.sol");
    let input = CompilerInput::new(source.clone())
        .unwrap()
        .first()
        .unwrap()
        .clone();
    let compiled = Solc::default()
        .compile(&input)
        .expect("could not compile contract");
    assert!(compiled.errors.is_empty(), "errors: {:?}", compiled.errors);

    let (abi, bytecode, _) = compiled
        .find("AstriaWithdrawer")
        .expect("could not find contract")
        .into_parts_or_default();

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

    let factory = ContractFactory::new(abi, bytecode, signer.into());
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
