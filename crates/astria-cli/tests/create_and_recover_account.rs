use std::{
    fs,
    path::Path,
};

use assert_cmd::Command;
use astria_eyre::{
    eyre::WrapErr,
    Result,
};
use predicates::prelude::predicate;

struct Resources {
    input: String,
    expected_verbose_display: String,
}

impl Resources {
    /// Reads the contents of the files in the `tests/resources/parse_blob/<test_case>` folder to
    /// the respective fields of `Self`.
    fn new(test_case: &str) -> Result<Self> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("resources")
            .join(test_case);
        let read_file = |filename: &str| -> Result<String> {
            let path = dir.join(filename);
            fs::read_to_string(&path).wrap_err(format!("failed to read {}", path.display()))
        };
        Ok(Resources {
            input: read_file("input.txt")?,
            expected_verbose_display: read_file("expected_verbose_output.txt")?,
        })
    }

    #[track_caller]
    fn check_reconstruct_account(self) -> Result<()> {
        let mut cmd = new_account_command()?;
        cmd.arg("recover")
            .arg("--mnemonic")
            .arg(&self.input)
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_verbose_display.clone()));

        Ok(())
    }
}

#[track_caller]
fn new_account_command() -> Result<Command> {
    // astria-cli sequencer account create command
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.arg("sequencer").arg("account");
    Ok(cmd)
}

#[test]
fn should_reconstruct_account() -> Result<(), Box<dyn std::error::Error>> {
    let resources: Resources = Resources::new("recover_account")?;
    resources.check_reconstruct_account()?;
    Ok(())
}

#[test]
fn should_create_account() -> Result<()> {
    let mut cmd = new_account_command()?;
    cmd.arg("create")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mnemonic:"))
        .stdout(predicate::str::contains("Private Key:"))
        .stdout(predicate::str::contains("Address:"))
        .stdout(predicate::str::contains("Public Key:"));

    Ok(())
}
