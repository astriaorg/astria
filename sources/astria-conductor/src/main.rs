use clap::{Arg, Command};

use crate::conf::Conf;
use crate::error::*;

pub mod alert;
pub mod conf;
mod driver;
mod error;
mod executor;
mod logger;
mod reader;

#[tokio::main]
async fn main() -> Result<()> {
    // logs
    logger::initialize();

    // cli args
    let url_option = Arg::new("url")
        .short('u')
        .help("URL of the data layer server.")
        .required(true);
    let namespace_id_option = Arg::new("namespace_id")
        .short('n')
        .help("Namespace ID as a string; the hex encoding of a [u8; 8]")
        .required(true);
    let cli_app = Command::new("rv-rs")
        .version("0.1")
        .about("A cli to read and write blocks from and to different sources. Uses the Actor model.")
        .arg(url_option)
        .arg(namespace_id_option);

    let matches = cli_app.get_matches();
    let base_url = matches
        .get_one::<String>("url")
        .expect("url required");
    let namespace_id = matches
        .get_one::<String>("namespace_id")
        .expect("namespace id required");

    // configuration
    let conf = Conf::new(base_url.to_owned(), namespace_id.to_owned());
    log::info!("Using node at {}", conf.celestia_node_url);

    let (driver_handle, _alert_rx) = driver::spawn(conf).await?;
    driver_handle.shutdown().await?;

    Ok(())
}
