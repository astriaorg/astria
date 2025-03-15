use std::{
    fs,
    io,
    path::{
        Path,
    },
};

use astria_eyre::eyre::{
    Result,
    WrapErr,
};
use base64::{
    prelude::BASE64_STANDARD,
    Engine,
};
use astria_core::{
    generated::astria::protocol::transaction::v1::Transaction as RawTransaction,
    protocol::transaction::v1::Transaction,
};
use astria_core::primitive::v1::{Address, Bech32m};
use astria_core::Protobuf;


#[derive(clap::Args, Debug)]
pub struct Args {
    /// Base64-encoded transaction data, or a file containing this, or stdin if `-`
    #[arg(value_name = "TX|PATH")]
    input: String,
}

/// Copies JSON application state from a file to a genesis JSON file,
/// placing it at the key `app_state`.
///
/// # Errors
///
/// An `eyre::Result` is returned if either file cannot be opened,
/// or if the destination genesis file cannot be saved.
pub fn run(
    Args {
        input,
    }: Args,
) -> Result<()> {
    let raw_transaction = parse(&input)?;
    println!("Transaction:");
    println!(
        "{}",
        serde_json::to_string_pretty(&raw_transaction).wrap_err("failed to json-encode")?
    );
    println!();
    println!("Transaction Body:");
    let transaction = Transaction::try_from_raw(raw_transaction).wrap_err("failed to convert to transaction")?;
    let address: Address<Bech32m> = Address::builder().prefix("astria").slice(transaction.address_bytes()).try_build()?;
    println!("Address: {}", address);
    println!("{}", serde_json::to_string_pretty(&transaction.unsigned_transaction().to_raw()).wrap_err("failed to json-encode")?);

    Ok(())
}

fn parse(input: &str) -> Result<RawTransaction> {
    use prost::Message;
    let data = get_base64_data(input)?;
    let transaction = RawTransaction::decode(&*data).wrap_err("failed to decode transaction")?;
    Ok(transaction)
}

fn get_base64_data(input: &str) -> Result<Vec<u8>> {
    if input == "-" {
        let encoded = io::read_to_string(io::stdin().lock()).wrap_err("failed to read stdin")?;
        return BASE64_STANDARD
            .decode(encoded.trim())
            .wrap_err("failed to decode stdin data as base64");
    }

    if Path::new(input).is_file() {
        let encoded =
            fs::read_to_string(input).wrap_err_with(|| format!("failed to read file `{input}`"))?;
        return BASE64_STANDARD
            .decode(encoded.trim())
            .wrap_err_with(|| format!("failed to decode contents of `{input}` as base64"));
    }

    BASE64_STANDARD
        .decode(input.trim())
        .wrap_err("failed to decode provided blob data as base64")
}
