use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
};

use assert_cmd::Command;
use astria_eyre::{
    eyre::WrapErr,
    Result,
};
use predicates::prelude::*;

struct Resources {
    input_path: PathBuf,
    input: String,
    expected_brief_display: String,
    expected_brief_json: String,
    expected_verbose_display: String,
    expected_verbose_json: String,
}

impl Resources {
    /// Reads the contents of the files in the `tests/resources/parse_blob/<test_case>` folder to
    /// the respective fields of `Self`.
    fn new(test_case: &str) -> Result<Self> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("resources")
            .join("parse_blob")
            .join(test_case);
        let read_file = |filename: &str| -> Result<String> {
            let path = dir.join(filename);
            fs::read_to_string(&path).wrap_err(format!("failed to read {}", path.display()))
        };
        Ok(Resources {
            input_path: dir.join("input.txt"),
            input: read_file("input.txt")?,
            expected_brief_display: read_file("expected_brief_output.txt")?,
            expected_brief_json: read_file("expected_brief_output.json")?,
            expected_verbose_display: read_file("expected_verbose_output.txt")?,
            expected_verbose_json: read_file("expected_verbose_output.json")?,
        })
    }

    #[track_caller]
    fn check_parse_blob(self) -> Result<()> {
        // No verbose flag, default format ("display"), input via unnamed arg.
        let mut cmd = new_command()?;
        cmd.arg(&self.input)
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_brief_display.clone()));

        // No verbose flag, JSON format, input via unnamed arg.
        let mut cmd = new_command()?;
        cmd.arg(&self.input)
            .arg("-fjson")
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_brief_json));

        // With verbose flag, default format ("display"), input via unnamed arg.
        let mut cmd = new_command()?;
        cmd.arg(&self.input)
            .arg("-v")
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_verbose_display));

        // With verbose flag, JSON format, input via unnamed arg.
        let mut cmd = new_command()?;
        cmd.arg(&self.input)
            .arg("--verbose")
            .arg("--format")
            .arg("json")
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_verbose_json));

        // No verbose flag, default format ("display"), input from file.
        let mut cmd = new_command()?;
        cmd.arg(self.input_path)
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_brief_display.clone()));

        // No verbose flag, default format ("display"), input via `-` (stdin).
        let mut cmd = new_command()?;
        cmd.arg("-")
            .write_stdin(self.input)
            .assert()
            .success()
            .stdout(predicate::eq(self.expected_brief_display));

        Ok(())
    }
}

fn new_command() -> Result<Command> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    // Disable colored output to make the snapshots more legible.
    cmd.arg("parse-blob").env("NO_COLOR", "1");
    Ok(cmd)
}

#[test]
fn should_parse_batched_metadata() -> Result<()> {
    Resources::new("batched_metadata")?.check_parse_blob()
}

#[test]
fn should_parse_batched_rollup_data() -> Result<()> {
    Resources::new("batched_rollup_data")?.check_parse_blob()
}

#[test]
fn should_parse_unbatched_metadata() -> Result<()> {
    Resources::new("unbatched_metadata")?.check_parse_blob()
}

#[test]
fn should_parse_unbatched_rollup_data() -> Result<()> {
    Resources::new("unbatched_rollup_data")?.check_parse_blob()
}
