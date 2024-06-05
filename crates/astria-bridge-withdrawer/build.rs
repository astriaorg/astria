use std::fs;

use ethers::contract::Abigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    astria_build_info::emit("bridge-withdrawer-v")?;

    println!("cargo:rerun-if-changed=ethereum/src/AstriaWithdrawer.sol");
    println!("cargo:rerun-if-changed=ethereum/src/IAstriaWithdrawer.sol");
    println!("cargo:rerun-if-changed=ethereum/src/AstriaMintableERC20.sol");

    let abi = Abigen::new(
        "IAstriaWithdrawer",
        "./ethereum/out/IAstriaWithdrawer.sol/IAstriaWithdrawer.json",
    )?
    .generate()?;
    fs::write(
        "./src/withdrawer/ethereum/generated/astria_withdrawer_interface.rs",
        format!("#![allow(clippy::all)]\n{abi}"),
    )?;

    let abi = Abigen::new(
        "AstriaWithdrawer",
        "./ethereum/out/AstriaWithdrawer.sol/AstriaWithdrawer.json",
    )?
    .generate()?;
    fs::write(
        "./src/withdrawer/ethereum/generated/astria_withdrawer.rs",
        format!("#![allow(clippy::all)]\n{abi}"),
    )?;
    let abi = Abigen::new(
        "AstriaMintableERC20",
        "./ethereum/out/AstriaMintableERC20.sol/AstriaMintableERC20.json",
    )?
    .generate()?;
    fs::write(
        "./src/withdrawer/ethereum/generated/astria_mintable_erc20.rs",
        format!("#![allow(clippy::all)]\n{abi}"),
    )?;

    Ok(())
}
