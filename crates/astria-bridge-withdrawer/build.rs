use ethers::contract::Abigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    astria_build_info::emit("bridge-withdrawer-v")?;

    println!("cargo:rerun-if-changed=ethereum/src/AstriaWithdrawer.sol");
    println!("cargo:rerun-if-changed=ethereum/src/IAstriaWithdrawer.sol");
    println!("cargo:rerun-if-changed=ethereum/src/AstriaBridgeableERC20.sol");

    Abigen::new(
        "IAstriaWithdrawer",
        "./ethereum/out/IAstriaWithdrawer.sol/IAstriaWithdrawer.json",
    )?
    .generate()?
    .write_to_file("./src/withdrawer/ethereum/generated/astria_withdrawer_interface.rs")?;

    Abigen::new(
        "AstriaWithdrawer",
        "./ethereum/out/AstriaWithdrawer.sol/AstriaWithdrawer.json",
    )?
    .generate()?
    .write_to_file("./src/withdrawer/ethereum/generated/astria_withdrawer.rs")?;

    Abigen::new(
        "AstriaBridgeableERC20",
        "./ethereum/out/AstriaBridgeableERC20.sol/AstriaBridgeableERC20.json",
    )?
    .generate()?
    .write_to_file("./src/withdrawer/ethereum/generated/astria_bridgeable_erc20.rs")?;

    Ok(())
}
