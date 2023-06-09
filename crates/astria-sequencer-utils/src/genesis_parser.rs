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
    /// Propagates json data from one json file to another.
    ///
    /// # Errors
    ///
    /// An `eyre::Result` is returned if either source genesis files cannot be opened,
    /// or if the destination genesis file cannot be saved.
    pub fn propagate_data(data: Config) -> Result<()> {
        println!("loading genesis json data for propigation:");
        println!("\tsource genesis file: {}", data.source_genesis_file);
        println!(
            "\tdestination genesis file: {}",
            data.destination_genesis_file
        );
        // load sequencer genesis data
        let source_genesis_file_path = File::open(data.source_genesis_file)
            .wrap_err("failed to open sequencer genesis file")?;
        let source_genesis_data: Value = serde_json::from_reader(&source_genesis_file_path)
            .wrap_err("failed deserializing sequencer genesis state from file")?;

        // load cometbft genesis data
        let destination_genesis_file_path = File::open(data.destination_genesis_file.as_str())
            .wrap_err("failed to open cometbft genesis file")?;
        let mut destination_genesis_data: Value =
            serde_json::from_reader(&destination_genesis_file_path)
                .wrap_err("failed deserializing cometbft genesis state from file")?;

        // merge sequencer genesis data into cometbft genesis data
        merge_json(&mut destination_genesis_data, &source_genesis_data);

        // write new state
        let dest_file = File::create(Path::new(data.destination_genesis_file.as_str()))
            .wrap_err("failed to open destination genesis json file")?;
        to_writer_pretty(dest_file, &destination_genesis_data)
            .wrap_err("failed to write to output json file")?;

        Ok(())
    }
}

/// Merges a source JSON Value into a destination JSON Value context:
// https://stackoverflow.com/questions/47070876/how-can-i-merge-two-json-objects-with-rust
fn merge_json(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                merge_json(a.entry(k).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    }
}

// This is your test module
#[cfg(test)]
mod tests {
    use serde_json::json;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
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

        merge_json(&mut a, &b);

        assert_eq!(a, output);
    }
}
