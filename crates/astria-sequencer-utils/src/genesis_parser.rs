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
use tracing::{
    info,
    instrument,
};

use crate::config::GenesisParserArgs;

pub struct GenesisParser;

impl GenesisParser {
    #[instrument(skip_all)]
    pub async fn propigate_data(data: GenesisParserArgs) -> Result<()> {
        info!(
            source_genesis_file = data.source_genesis_file.as_str(),
            destination_genesis_file = data.destination_genesis_file.as_str(),
            "loading genesis json data for propigation"
        );
        // load sequencer genesis data
        let source_genesis_file_path = File::open(data.source_genesis_file)
            .context("failed to open sequencer genesis file")?;
        let source_genesis_data: Value = serde_json::from_reader(&source_genesis_file_path)
            .context("failed deserializing sequencer genesis state from file")?;

        // load cometbft genesis data
        let destination_genesis_file_path = File::open(data.destination_genesis_file.clone())
            .context("failed to open cometbft genesis file")?;
        let mut destination_genesis_data: Value =
            serde_json::from_reader(&destination_genesis_file_path)
                .context("failed deserializing cometbft genesis state from file")?;

        // merge sequencer genesis data into cometbft genesis data
        merge_values(&mut destination_genesis_data, &source_genesis_data);

        // write new state
        let dest_file = File::create(Path::new(data.destination_genesis_file.as_str()))
            .context("failed to open destination genesis json file")?;
        to_writer_pretty(dest_file, &destination_genesis_data)?;

        Ok(())
    }
}

/// Merges a source JSON Value into a destination JSON Value.
fn merge_values(a: &mut Value, b: &Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), Value::Object(b)) => {
            for (k, v) in b {
                merge_values(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}
