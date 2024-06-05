use ethers::contract::Abigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    astria_build_info::emit("bridge-withdrawer-v")?;

    println!("cargo::rerun-if-changed=ethereum/src/AstriaWithdrawer.sol");
    println!("cargo::rerun-if-changed=ethereum/src/IAstriaWithdrawer.sol");
    println!("cargo::rerun-if-changed=ethereum/src/AstriaMintableERC20.sol");

    Abigen::new(
        "IAstriaWithdrawer",
        "./ethereum/out/IAstriaWithdrawer.sol/IAstriaWithdrawer.json",
    )?
    .generate()?
    .write_to_file("./src/withdrawer/ethereum/astria_withdrawer_interface.rs")?;
    Abigen::new(
        "AstriaWithdrawer",
        "./ethereum/out/AstriaWithdrawer.sol/AstriaWithdrawer.json",
    )?
    .generate()?
    .write_to_file("./src/withdrawer/ethereum/astria_withdrawer.rs")?;
    Abigen::new(
        "AstriaMintableERC20",
        "./ethereum/out/AstriaMintableERC20.sol/AstriaMintableERC20.json",
    )?
    .generate()?
    .write_to_file("./src/withdrawer/ethereum/astria_mintable_erc20.rs")?;

    Ok(())
}
