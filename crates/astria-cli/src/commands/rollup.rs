use std::{
    env::{
        self,
        consts::OS,
    },
    fs::File,
    io::{
        Read,
        Write,
    },
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

fn update_yaml_value(
    value: &mut serde_yaml::Value,
    key: &str,
    new_value: &str,
) -> eyre::Result<()> {
    let mut target = value;

    let keys: Vec<&str> = key.split('.').collect();

    for &key in keys.iter().take(keys.len() - 1) {
        target = target
            .get_mut(key)
            .ok_or_else(|| eyre::eyre!("Invalid key path: {}", key))?;
    }

    let last_key = keys
        .last()
        .ok_or_else(|| eyre::eyre!("Key path is empty"))?;

    if let Some(v) = target.get_mut(*last_key) {
        *v = serde_yaml::Value::String(new_value.to_string());
    } else {
        return Err(eyre::eyre!("Invalid last key: {}", last_key));
    }
    Ok(())
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

pub(crate) fn edit_config(args: &ConfigEditArgs) -> eyre::Result<()> {
    // get file contents
    let path = PathBuf::from(&args.config_path);
    let mut file = File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut yaml_value: serde_yaml::Value = serde_yaml::from_str(&contents)?;
    update_yaml_value(&mut yaml_value, &args.key, &args.value)?;

    // Write the updated YAML back to the file
    let updated_yaml = serde_yaml::to_string(&yaml_value)?;
    let mut file = File::create(&path)?;
    file.write_all(updated_yaml.as_bytes())?;

    Ok(())
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

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use once_cell::sync::Lazy;
    use tempfile::{
        self,
        TempDir,
    };

    use super::*;

    static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    /// Run a closure with a temporary directory as the current directory.
    /// This is useful for tests that want to change the current directory.
    /// `set_current_env` is not thread safe, so it will cause flaky tests if not behind a mutex.
    fn with_temp_directory<F>(closure: F)
    where
        F: FnOnce(&TempDir),
    {
        // Lock the mutex
        let _guard = CURRENT_DIR_LOCK.lock().unwrap();

        // Store the original current directory
        let original_dir = env::current_dir().unwrap();

        // Create a new temporary directory
        let temp_dir = TempDir::new().unwrap();

        // Change to the temporary directory
        env::set_current_dir(&temp_dir).unwrap();

        // Run the closure, passing it a reference to the temp directory
        closure(&temp_dir);

        // Restore the original current directory
        env::set_current_dir(original_dir).unwrap();

        temp_dir.close().unwrap();
    }

    fn get_config_create_args() -> ConfigCreateArgs {
        ConfigCreateArgs {
            use_tty: false,
            name: "test".to_string(),
            chain_id: None,
            network_id: 0,
            skip_empty_blocks: false,
            genesis_accounts: vec![],
            sequencer_initial_block_height: None,
            sequencer_websocket: "".to_string(),
            sequencer_rpc: "".to_string(),
            log_level: "".to_string(),
            celestia_full_node_url: "".to_string(),
        }
    }

    #[test]
    fn test_create_config_file() {
        with_temp_directory(|_dir| {
            let args = get_config_create_args();
            create_config(&args).unwrap();

            let file_path = PathBuf::from("test-rollup-conf.yaml");
            assert!(file_path.exists());
        });
    }

    #[test]
    fn test_delete_config_file() {
        with_temp_directory(|_dir| {
            let file_path = PathBuf::from("test-rollup-conf.yaml");
            File::create(&file_path).unwrap();

            let args = ConfigDeleteArgs {
                config_path: file_path.to_str().unwrap().to_string(),
            };
            delete_config(&args).unwrap();
            assert!(!file_path.exists());
        });
    }

    #[test]
    fn test_edit_config_file() {
        with_temp_directory(|_dir| {
            let args = get_config_create_args();
            create_config(&args).unwrap();

            let file_path = PathBuf::from("test-rollup-conf.yaml");
            let args = ConfigEditArgs {
                config_path: file_path.to_str().unwrap().to_string(),
                key: "config.rollup.name".to_string(),
                value: "bugbug".to_string(),
            };
            edit_config(&args).unwrap();

            let file = File::open(&file_path).unwrap();
            let rollup: Rollup = serde_yaml::from_reader(file).unwrap();
            assert_eq!(rollup.deployment_config.get_rollup_name(), "bugbug");
        });
    }

    #[test]
    fn test_edit_config_file_errors_for_wrong_key() {
        with_temp_directory(|_dir| {
            let args = get_config_create_args();
            create_config(&args).unwrap();

            let file_path = PathBuf::from("test-rollup-conf.yaml");
            let args = ConfigEditArgs {
                config_path: file_path.to_str().unwrap().to_string(),
                key: "config.blahblah".to_string(),
                value: "bugbug".to_string(),
            };
            assert!(edit_config(&args).is_err());
        });
    }
}
