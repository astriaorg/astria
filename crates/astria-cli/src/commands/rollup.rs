use std::{
    fs::File,
    io::Write,
};

use color_eyre::eyre;

use crate::{
    cli::rollup::{
        ConfigCreateArgs,
        ConfigDeleteArgs,
        ConfigDeployArgs,
        ConfigEditArgs,
    },
    types::Rollup,
};

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
pub(crate) fn create_config(args: &ConfigCreateArgs) -> eyre::Result<()> {
    // create file and prefix name with rollup name
    let name = &args.name;
    let path = format!("{name}-values-override.yaml");
    let mut output = File::create(path)?;

    // write args as yaml
    let rollup = Rollup::from_cli_args(args)?;
    let yaml_str = rollup.to_yaml()?;
    write!(output, "{yaml_str}")?;

    Ok(())
}

pub(crate) fn edit_config(args: &ConfigEditArgs) {
    println!("Edit Rollup Config {args:?}");
}

pub(crate) fn deploy_config(args: &ConfigDeployArgs) {
    println!("Deploy Rollup Config {args:?}");
}

pub(crate) fn delete_config(args: &ConfigDeleteArgs) {
    println!("Delete Rollup Config {args:?}");
}
