use std::{
    fs::File,
    path::Path,
};

use anyhow::{
    Context as _,
    Result,
};
use serde_json::{
    to_writer_pretty,
    Value,
};

use crate::config::Config;

pub struct GenesisParser;

impl GenesisParser {
    pub fn propagate_data(data: Config) -> Result<()> {
        println!("loading genesis json data for propigation:");
        println!("\tsource genesis file: {}", data.source_genesis_file);
        println!(
            "\tdestination genesis file: {}",
            data.destination_genesis_file
        );
        // load sequencer genesis data
        let source_genesis_file_path = File::open(data.source_genesis_file)
            .context("failed to open sequencer genesis file")?;
        let source_genesis_data: Value = serde_json::from_reader(&source_genesis_file_path)
            .context("failed deserializing sequencer genesis state from file")?;

        // load cometbft genesis data
        let destination_genesis_file_path = File::open(data.destination_genesis_file.as_str())
            .context("failed to open cometbft genesis file")?;
        let mut destination_genesis_data: Value =
            serde_json::from_reader(&destination_genesis_file_path)
                .context("failed deserializing cometbft genesis state from file")?;

        // merge sequencer genesis data into cometbft genesis data
        merge_json(&mut destination_genesis_data, &source_genesis_data);

        // write new state
        let dest_file = File::create(Path::new(data.destination_genesis_file.as_str()))
            .context("failed to open destination genesis json file")?;
        to_writer_pretty(dest_file, &destination_genesis_data)?;

        Ok(())
    }
}

/// Merges a source JSON Value into a destination JSON Value.
// inpiration for this function came from: https://stackoverflow.com/questions/47070876/how-can-i-merge-two-json-objects-with-rust
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
