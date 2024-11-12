use assert_cmd::Command;
use predicates::prelude::*;

const INPUT_BYTES: &str = "1234567890abcdef1234567890abcdef12345678";

#[test]
fn input_without_options() {
    let assert = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .arg(INPUT_BYTES)
        .assert();
    assert.stdout(predicate::eq(
        "astria1zg69v7ys40x77y352eufp27daufrg4nc077y64\n",
    ));
}

#[test]
fn input_with_compat() {
    let assert = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .arg("--compat")
        .arg(INPUT_BYTES)
        .assert();
    assert.stdout(predicate::eq(
        "astria1zg69v7ys40x77y352eufp27daufrg4nc6zwglh\n",
    ));
}

#[test]
fn input_with_prefix() {
    let assert = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .args(["--prefix", "astriacompat"])
        .arg(INPUT_BYTES)
        .assert();
    assert.stdout(predicate::eq(
        "astriacompat1zg69v7ys40x77y352eufp27daufrg4ncd586wu\n",
    ));
}

#[test]
fn input_with_prefix_and_compat() {
    let assert = Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .arg("--compat")
        .args(["--prefix", "astriacompat"])
        .arg(INPUT_BYTES)
        .assert();
    assert.stdout(predicate::eq(
        "astriacompat1zg69v7ys40x77y352eufp27daufrg4nccghkt7\n",
    ));
}
