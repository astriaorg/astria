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
    // TODO - move to own module
    let url_option = Arg::new("url")
        .short('u')
        .help("URL of the data layer.")
        .required(true);
    let cli_app = Command::new("rv-rs")
        .version("0.1")
        .about("A cli to read and write blocks from and to different sources.")
        .arg(url_option);
    let matches = cli_app.get_matches();

    // TODO - namespace id?
    let base_url= matches.get_one::<String>("url")
        .expect("url required");

    // logs
    logger::initialize();

    // configuration
    let namespace_id: [u8; 8] = *b"DEADBEEF";
    let conf = Conf::new(base_url.to_string(), namespace_id);

    log::info!("Using node at {}", conf.celestia_node_url);

    let (driver_handle, _alert_rx) = driver::spawn(conf).await?;

    driver_handle.shutdown().await?;

    Ok(())
}
