use crate::conf::Conf;
use crate::driver::DriverCommand;
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
    logger::initialize();

    let url = String::from("http://localhost:26659");
    let namespace_id: [u8; 8] = *b"DEADBEEF";
    let conf = Conf::new(url, namespace_id);

    let (driver_handle, alert_rx) = driver::spawn(conf).await?;

    driver_handle.shutdown().await?;

    Ok(())
}
