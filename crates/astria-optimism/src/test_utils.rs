use std::{
    path::Path,
    sync::Arc,
    time::Duration,
};

use ethers::{
    core::utils::Anvil,
    prelude::*,
    solc::Solc,
    utils::AnvilInstance,
};

#[allow(dead_code)]
pub(crate) async fn deploy_mock_optimism_portal()
-> (Address, Arc<Provider<Ws>>, LocalWallet, AnvilInstance) {
    // compile contract for testing
    let source = Path::new(&env!("CARGO_MANIFEST_DIR")).join("contracts/MockOptimismPortal.sol");
    let input = CompilerInput::new(source.clone())
        .unwrap()
        .first()
        .unwrap()
        .clone()
        .evm_version(EvmVersion::Homestead); // TODO: idk why the default version doesn't work
    let compiled = Solc::default()
        .compile(&input)
        .expect("could not compile contract");
    assert!(compiled.errors.is_empty(), "errors: {:?}", compiled.errors);

    let (abi, bytecode, _) = compiled
        .find("MockOptimismPortal")
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

    // deploy contract
    let factory = ContractFactory::new(abi, bytecode, signer.into());
    let contract = factory.deploy(()).unwrap().send().await.unwrap();
    let contract_address = contract.address();

    (
        contract_address,
        provider,
        wallet.with_chain_id(anvil.chain_id()),
        anvil,
    )
}
