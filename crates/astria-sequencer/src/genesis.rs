use std::{
    env,
    fs::File,
    path::Path,
};

use anyhow::Context;
use csv::Reader;
use serde::Deserialize;
use serde_json::{
    to_writer_pretty,
    Value,
};

use crate::accounts::types::{
    Address,
    Balance,
};

/// The genesis state for the application.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct GenesisState {
    pub(crate) accounts: Vec<Account>,
}

impl GenesisState {
    /// Load Account information from a CSV file
    pub(crate) fn from_csv<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut reader = Reader::from_path(path)?;
        let mut records: Vec<Account> = Vec::new();

        for result in reader.deserialize() {
            let record: Account = result?;
            records.push(record);
        }

        Ok(Self {
            accounts: records,
        })
    }

    /// Add the Account data from `GenesisState` to the `CometBFT` genesis.json file
    pub(crate) fn propagate_accounts_to(&self, path: String) -> anyhow::Result<()> {
        // build the absolute path to the json file you want to add the accounts to
        let mut home_path = env::var("HOME")?;
        home_path.push_str(&path);
        let abs_destination_json_file_path = Path::new(&home_path);
        let dest_file = File::open(abs_destination_json_file_path)
            .context("failed to open destination genesis json file")?;

        let mut dest_state: Value = serde_json::from_reader(&dest_file)
            .context("failed deserializing genesis state from file")?;

        // convert the accounts in GenesisState into a json Value
        let mut json_map: serde_json::Map<String, Value> = serde_json::Map::new();
        let mut accounts: Vec<Value> = Vec::new();
        for acct in &self.accounts {
            accounts.push(serde_json::json!({
                "address": acct.address,
                "balance": acct.balance,
            }));
        }
        json_map.insert("accounts".to_string(), Value::Array(accounts));

        // combine all json data into the same Value
        let o = Value::Object(json_map);
        merge_json_values(&mut dest_state, o);

        // write new state
        let dest_file = File::create(abs_destination_json_file_path)
            .context("failed to open destination genesis json file")?;
        to_writer_pretty(dest_file, &dest_state)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Account {
    pub(crate) address: Address,
    pub(crate) balance: Balance,
}

fn merge_json_values(a: &mut Value, b: Value) {
    if let Value::Object(a) = a {
        if let Value::Object(b) = b {
            for (k, v) in b {
                if v.is_null() {
                    a.remove(&k);
                } else {
                    merge_json_values(a.entry(k).or_insert(Value::Null), v);
                }
            }
            return;
        }
    }
    *a = b;
}
