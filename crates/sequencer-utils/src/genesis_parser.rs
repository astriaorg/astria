use std::{
    fs::File,
    path::Path,
};

use eyre::{
    Result,
    WrapErr,
};
use serde_json::{
    to_writer_pretty,
    Value,
};

use crate::config::Config;

pub struct GenesisParser;

impl GenesisParser {
    /// Copies JSON application state from a file to a genesis JSON file,
    /// placing it at the key `app_state`.
    ///
    /// # Errors
    ///
    /// An `eyre::Result` is returned if either file cannot be opened,
    /// or if the destination genesis file cannot be saved.
    pub fn propagate_app_state(data: Config) -> Result<()> {
        println!("loading genesis app state for propagation:");
        println!(
            "\tsource genesis app state: {}",
            data.genesis_app_state_file
        );
        println!(
            "\tdestination genesis file: {}",
            data.destination_genesis_file
        );
        // load sequencer genesis data
        let source_genesis_file_path = File::open(data.genesis_app_state_file)
            .wrap_err("failed to open sequencer genesis file")?;
        let source_genesis_data: Value = serde_json::from_reader(&source_genesis_file_path)
            .wrap_err("failed deserializing sequencer genesis state from file")?;

        // load cometbft genesis data
        let destination_genesis_file_path = File::open(data.destination_genesis_file.as_str())
            .wrap_err("failed to open cometbft genesis file")?;
        let mut destination_genesis_data: Value =
            serde_json::from_reader(&destination_genesis_file_path)
                .wrap_err("failed deserializing cometbft genesis state from file")?;

        // insert sequencer app genesis data into cometbft genesis data
        insert_app_state_and_chain_id(
            &mut destination_genesis_data,
            &source_genesis_data,
            data.chain_id,
        );

        // write new state
        let dest_file = File::create(Path::new(data.destination_genesis_file.as_str()))
            .wrap_err("failed to open destination genesis json file")?;
        to_writer_pretty(dest_file, &destination_genesis_data)
            .wrap_err("failed to write to output json file")?;

        Ok(())
    }
}

fn insert_app_state_and_chain_id(dst: &mut Value, app_state: &Value, chain_id: String) {
    let Value::Object(dst) = dst else {
        panic!("dst is not an object");
    };
    dst.insert("app_state".to_string(), app_state.clone());
    dst.insert("chain_id".to_string(), chain_id.into());
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn merge_json_test() {
        let mut a = json!({
            "genesis_time": "2023-06-21T15:58:36.741257Z",
            "initial_height": "0",
            "consensus_params": {
                "validator": {
                    "pub_key_types": [
                      "ed25519"
                    ]
                  }
            }
        });

        let b = json!({
            "accounts": [
              {
                "address": "alice",
                "balance": 1000
              },
              {
                "address": "bob",
                "balance": 1000
              },
              {
                "address": "charlie",
                "balance": 1000
              }
            ],
        });

        let output = json!({
            "genesis_time": "2023-06-21T15:58:36.741257Z",
            "initial_height": "0",
            "consensus_params": {
                "validator": {
                    "pub_key_types": [
                      "ed25519"
                    ]
                  }
            },
            "app_state": {
                "accounts": [
                    {
                        "address": "alice",
                        "balance": 1000
                    },
                    {
                        "address": "bob",
                        "balance": 1000
                    },
                    {
                        "address": "charlie",
                        "balance": 1000
                    }
                ]
            },
            "chain_id": "test"
        });

        insert_app_state_and_chain_id(&mut a, &b, "test".to_string());
        assert_eq!(a, output);
    }
}
