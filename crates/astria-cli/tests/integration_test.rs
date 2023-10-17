use std::path::PathBuf;

use assert_cmd::Command;
use test_utils::with_temp_directory;

#[test]
fn test_envars_are_parsed_for_config_create() {
    with_temp_directory(|_dir| {
        let mut cmd = Command::cargo_bin("astria-cli").unwrap();

        cmd.arg("rollup")
            .arg("config")
            .arg("create")
            .env("ROLLUP_USE_TTY", "true")
            .env("ROLLUP_LOG_LEVEL", "debug")
            .env("ROLLUP_NAME", "testtest")
            .env("ROLLUP_CHAIN_ID", "test_chain_id")
            .env("ROLLUP_NETWORK_ID", "53")
            .env("ROLLUP_SKIP_EMPTY_BLOCKS", "true")
            .env(
                "ROLLUP_GENESIS_ACCOUNTS",
                "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:1000,\
                 aC21B97d35Bf75A7dAb16f35b111a50e78A72F30:1000",
            )
            .env("ROLLUP_SEQUENCER_INITIAL_BLOCK_HEIGHT", "10")
            .env("ROLLUP_SEQUENCER_WEBSOCKET", "ws://localhost:8080")
            .env("ROLLUP_SEQUENCER_RPC", "http://localhost:8081")
            .assert()
            .success();

        assert!(PathBuf::from("testtest-rollup-conf.yaml").exists());
    });
}

#[test]
fn test_error_when_incorrect_envar_values() {
    with_temp_directory(|_dir| {
        let mut cmd = Command::cargo_bin("astria-cli").unwrap();

        cmd.arg("rollup")
            .arg("config")
            .arg("create")
            .env("ROLLUP_USE_TTY", "not_a_bool")
            .env("ROLLUP_LOG_LEVEL", "debug")
            .env("ROLLUP_NAME", "testtest")
            .env("ROLLUP_CHAIN_ID", "test_chain_id")
            .env("ROLLUP_NETWORK_ID", "not_a_number")
            .env("ROLLUP_SKIP_EMPTY_BLOCKS", "true")
            .env(
                "ROLLUP_GENESIS_ACCOUNTS",
                "0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30:1000",
            )
            .env("ROLLUP_SEQUENCER_INITIAL_BLOCK_HEIGHT", "10")
            .env("ROLLUP_SEQUENCER_WEBSOCKET", "ws://localhost:8080")
            .env("ROLLUP_SEQUENCER_RPC", "http://localhost:8081")
            .assert()
            .failure();
    });
}
