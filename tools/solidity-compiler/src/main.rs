use ethers_contract_abigen::MultiAbigen;

const CRATE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/astria-bridge-contracts",
);
const SUBMODULE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/astria-bridge-contracts/astria-bridge-contracts",
);
const SUBMODULE_NAME: &str = "crates/astria-bridge-contracts/astria-bridge-contracts";

fn init_and_update(submodule_name: &str) -> Result<(), git2::Error> {
    println!("updating and initializing contracts submodule `{submodule_name}`");
    let repo = git2::Repository::open_from_env()?;
    let mut submodule = repo.find_submodule(submodule_name)?;
    submodule.update(true, None)?;
    Ok(())
}

fn generate_contract_abi(src: &str, dst: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "generating Rust bindings from solidity JSON ABI files\n\tsources: {src}\n\tdestination: \
         {dst}"
    );

    MultiAbigen::from_json_files(src)?
        .build()?
        .write_to_module(dst, false)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_and_update(SUBMODULE_NAME)?;

    generate_contract_abi(
        &format!("{SUBMODULE_DIR}/out"),
        &format!("{CRATE_DIR}/src/generated"),
    )?;

    Ok(())
}
