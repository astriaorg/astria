use std::path::Path;

use ethers::contract::Abigen;

fn emit_rerun_if_changed(file: &str) {
    assert!(
        Path::new(file).is_file(),
        "rerun-if-changed file does not exist at `{file}`"
    );
    println!("cargo:rerun-if-changed={file}");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    astria_build_info::emit("bridge-withdrawer-v")?;

    emit_rerun_if_changed("astria-bridge-contracts/src/AstriaWithdrawer.sol");
    emit_rerun_if_changed("astria-bridge-contracts/src/IAstriaWithdrawer.sol");
    emit_rerun_if_changed("astria-bridge-contracts/src/AstriaBridgeableERC20.sol");

    Abigen::new(
        "IAstriaWithdrawer",
        "astria-bridge-contracts/out/IAstriaWithdrawer.sol/IAstriaWithdrawer.json",
    )?
    .generate()?
    .write_to_file("src/bridge_withdrawer/ethereum/generated/astria_withdrawer_interface.rs")?;

    Abigen::new(
        "AstriaWithdrawer",
        "astria-bridge-contracts/out/AstriaWithdrawer.sol/AstriaWithdrawer.json",
    )?
    .generate()?
    .write_to_file("src/bridge_withdrawer/ethereum/generated/astria_withdrawer.rs")?;

    Abigen::new(
        "AstriaBridgeableERC20",
        "astria-bridge-contracts/out/AstriaBridgeableERC20.sol/AstriaBridgeableERC20.json",
    )?
    .generate()?
    .write_to_file("src/bridge_withdrawer/ethereum/generated/astria_bridgeable_erc20.rs")?;

    Ok(())
}
