use std::{
    fs::File,
    io::Write,
};

use color_eyre::eyre;
use serde_yaml::to_string;

use crate::cli::{
    RollupConfigCreateArgs,
    RollupConfigDeleteArgs,
    RollupConfigDeployArgs,
    RollupConfigEditArgs,
};

/// Generate a yaml string from the arguments
fn generate_yaml(args: &RollupConfigCreateArgs) -> eyre::Result<String> {
    let yaml_str = to_string(args)?;
    Ok(yaml_str)
}

/// Create a new rollup config file
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the config file cannot be created
/// * If the arguments cannot be serialized to yaml
/// * If the yaml cannot be written to the file
pub(crate) fn create_config(args: &RollupConfigCreateArgs) -> eyre::Result<()> {
    // create file and prefix name with rollup name
    let name = &args.name;
    let path = format!("{name}-values-override.yaml");
    let mut output = File::create(path)?;

    // write args as yaml
    let yaml_str = generate_yaml(args)?;
    write!(output, "{yaml_str}")?;

    Ok(())
}
// pub(crate) fn edit_config(args: &RollupConfigEditArgs) -> eyre::Result<()> {
pub(crate) fn edit_config(args: &RollupConfigEditArgs) {
    println!("Edit Rollup Config {args:?}");
    // Ok(())
}
// pub(crate) fn deploy_config(args: &RollupConfigDeployArgs) -> eyre::Result<()> {
pub(crate) fn deploy_config(args: &RollupConfigDeployArgs) {
    println!("Deploy Rollup Config {args:?}");
    // Ok(())
}
// pub(crate) fn delete_config(args: &RollupConfigDeleteArgs) -> eyre::Result<()> {
pub(crate) fn delete_config(args: &RollupConfigDeleteArgs) {
    println!("Delete Rollup Config {args:?}");
    // Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::RollupConfigCreateArgs;

    #[test]
    fn test_generate_yaml() {
        let args = RollupConfigCreateArgs {
            name: "test".to_string(),
            genesis_alloc_address: "0x000000".to_string(),
            private_key: "0x000000".to_string(),
            evm_chain_id: "test".to_string(),
            evm_network_id: 12345,
            sequencer_private_key: "0x000000".to_string(),
        };
        let result = generate_yaml(&args);
        let yaml_str = result.unwrap();
        assert!(yaml_str.contains("test"));
        assert!(yaml_str.contains("0x000000"));
        assert!(yaml_str.contains("12345"));
    }
}
