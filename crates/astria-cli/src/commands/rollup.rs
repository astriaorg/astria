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

const EVM_ROLLUP_CHART_URL: &str =
    "https://astriaorg.github.io/dev-cluster/astria-evm-rollup-0.3.0.tgz";

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

pub(crate) fn edit_config(args: &ConfigEditArgs) {
    println!("Edit Rollup Config {args:?}");
}

fn helm_from_env() -> PathBuf {
    let os_specific_hint = match OS {
        "macos" => "You could try running `brew install helm` or downloading a recent release from https://github.com/helm/helm/releases",
        "linux" => "You can download it from https://github.com/helm/helm/releases",
        _other =>  "Check if there is a precompiled version for your OS at https://github.com/helm/helm/releases"
    };
    let error_msg = "Could not find `helm` installation and this deployment cannot proceed without
    this knowledge. If `helm` is installed and this crate had trouble finding
    it, you can set the `HELM_PATH` environment variable with the specific path to your
    installed `helm` binary.";
    let msg = format!("{error_msg} {os_specific_hint}");

    env::var_os("HELM")
        .map(PathBuf::from)
        .or_else(|| which::which("helm").ok())
        .expect(&msg)
}

pub(crate) fn deploy_config(args: &ConfigDeployArgs) -> eyre::Result<()> {
    let helm = helm_from_env();

    let path = PathBuf::from(args.filename.clone());
    let file = File::open(path)?;
    let rollup: Rollup = serde_yaml::from_reader(file)?;

    let mut cmd = Command::new(helm.clone());

    // call `helm install` with appropriate args.
    // setting values via the generated config file
    // FIXME - is this the best place to set disable_finalization to true?
    //  bc we probably don't want it in the config file for them to change right now
    cmd.arg("install")
        .arg("--debug")
        .arg("--values")
        .arg(rollup.deployment_config.get_filename())
        .arg("--set")
        .arg("config.rollup.disableFinalization=true")
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
// TODO - implement separate commands to delete the config and to delete the deployment
pub(crate) fn delete_config(args: &ConfigDeleteArgs) -> eyre::Result<()> {
    let helm = helm_from_env();

    let path = PathBuf::from(args.filename.clone());
    let file = File::open(path)?;
    let rollup: Rollup = serde_yaml::from_reader(file)?;

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
            println!("Deleted deployment created from rollup config {}", args.filename);
        }
    };

    Ok(())
}
