use std::{
    env::{
        self,
        consts::OS,
    },
    fs::File,
    io::Write,
    path::PathBuf,
    process::Command,
};

use color_eyre::{
    eyre,
    eyre::Context,
};

use crate::{
    cli::rollup::{
        ConfigCreateArgs,
        ConfigDeleteArgs,
        ConfigEditArgs,
        DeploymentCreateArgs,
        DeploymentDeleteArgs,
    },
    types::Rollup,
};

const EVM_ROLLUP_CHART_URL: &str =
    "https://astriaorg.github.io/dev-cluster/astria-evm-rollup-0.3.0.tgz";

///
fn helm_from_env() -> PathBuf {
    let os_specific_hint = match OS {
        "macos" => "You could try running `brew install helm` or downloading a recent release from https://github.com/helm/helm/releases",
        "linux" => "You can download it from https://github.com/helm/helm/releases",
        _other =>  "Check if there is a precompiled version for your OS at https://github.com/helm/helm/releases"
    };
    let error_msg = "Could not find `helm` installation and this deployment cannot proceed without
    this knowledge. If `helm` is installed and this crate had trouble finding
    it, you can set the `HELM` environment variable with the specific path to your
    installed `helm` binary.";
    let msg = format!("{error_msg} {os_specific_hint}");

    env::var_os("HELM")
        .map(PathBuf::from)
        .or_else(|| which::which("helm").ok())
        .expect(&msg)
}

/// Create a new rollup config file in the calling directory.
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
    // create rollup from args
    let rollup = Rollup::try_from(args)?;
    let filename = rollup.deployment_config.get_filename();

    // create file config file
    let mut output = File::create(&filename)?;

    // write args as yaml
    let yaml_str: String = rollup.try_into()?;
    write!(output, "{yaml_str}")?;

    println!("Created rollup config file {filename}");

    Ok(())
}

/// Deletes a config file
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the config file cannot be deleted
pub(crate) fn delete_config(args: &ConfigDeleteArgs) -> eyre::Result<()> {
    let path = PathBuf::from(args.config_path.clone());
    std::fs::remove_file(path).wrap_err("could not delete the config file")?;

    println!("Deleted rollup config file {}", args.config_path);

    Ok(())
}

pub(crate) fn edit_config(args: &ConfigEditArgs) {
    // TODO
    println!("Edit Rollup Config {args:?}");
}

/// Creates a deployment from a config file
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
///
/// * If the config file cannot be opened
/// * If the config file cannot be deserialized
/// * If the deployment cannot be created
/// * If the helm command fails
pub(crate) fn create_deployment(args: &DeploymentCreateArgs) -> eyre::Result<()> {
    let path = PathBuf::from(args.config_path.clone());
    let file = File::open(path)?;
    let rollup: Rollup = serde_yaml::from_reader(file)?;

    // call `helm install` with appropriate args.
    // setting values via the generated config file.
    let helm = helm_from_env();
    let mut cmd = Command::new(helm.clone());
    cmd.arg("install")
        .arg("--debug")
        .arg("--values")
        .arg(rollup.deployment_config.get_filename())
        .arg("--set")
        // FIXME - is this the best place to set disable_finalization to true?
        //  bc we probably don't want it in the config file for them to change right now
        .arg("config.rollup.disableFinalization=true")
        .arg("--set")
        .arg(format!(
            "config.faucet.privateKey={}",
            args.faucet_private_key.clone()
        ))
        .arg("--set")
        .arg(format!(
            "config.sequencer.privateKey={}",
            args.sequencer_private_key.clone()
        ))
        .arg(rollup.deployment_config.get_chart_release_name())
        .arg(EVM_ROLLUP_CHART_URL);

    match cmd.output() {
        Err(e) => {
            panic!("failed deploying config: failed to invoke helm (path: {helm:?}): {e:?}");
        }
        Ok(output) if !output.status.success() => {
            panic!(
                "failed deploying config: `helm` returned error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(_) => {}
    };

    Ok(())
}

/// Deletes a deployment
///
/// # Arguments
///
/// * `args` - The arguments passed to the command
///
/// # Errors
///
/// * If the config file cannot be opened
/// * If the config file cannot be deserialized
/// * If the deployment cannot be deleted
/// * If the helm command fails
pub(crate) fn delete_deployment(args: &DeploymentDeleteArgs) -> eyre::Result<()> {
    let path = PathBuf::from(args.config_path.clone());
    let file = File::open(path)?;
    let rollup: Rollup = serde_yaml::from_reader(file)?;

    // call `helm uninstall` with appropriate args
    let helm = helm_from_env();
    let mut cmd = Command::new(helm.clone());
    cmd.arg("uninstall")
        .arg(rollup.deployment_config.get_chart_release_name());

    match cmd.output() {
        Err(e) => {
            panic!("failed deleting config: failed to invoke helm (path: {helm:?}): {e:?}");
        }
        Ok(output) if !output.status.success() => {
            panic!(
                "failed deleting config: `helm` returned error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(_) => {
            println!(
                "Deleted deployment created from rollup config {}",
                args.config_path
            );
        }
    };

    Ok(())
}

/// Lists all deployments
///
/// # Errors
///
/// * If the helm command fails
pub(crate) fn list_deployments() -> eyre::Result<()> {
    // call `helm list` with appropriate args
    let helm = helm_from_env();
    let mut cmd = Command::new(helm.clone());
    // FIXME - right now it lists all helm releases, not just rollup release
    cmd.arg("list");

    match cmd.output() {
        Err(e) => {
            panic!("failed listing deployments: failed to invoke helm (path: {helm:?}): {e:?}");
        }
        Ok(output) if !output.status.success() => {
            panic!(
                "failed listing deployments: `helm` returned error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output) => {
            // print output
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    };

    Ok(())
}
