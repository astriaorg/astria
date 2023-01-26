use flexi_logger::{Duplicate, FileSpec};

use crate::conf::Conf;
use crate::driver::DriverCommand;
use crate::error::*;

pub mod alert;
pub mod conf;
mod driver;
mod error;
mod executor;
mod reader;

#[tokio::main]
async fn main() -> Result<()> {
    // log to file and stderr
    flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory("/tmp/astria-rv-rs"))
        .duplicate_to_stderr(Duplicate::All)
        .start()
        .unwrap();

    let url = String::from("http://localhost:26659");
    let namespace_id: [u8; 8] = *b"DEADBEEF";
    let conf = Conf::new(url, namespace_id);

    let (driver_handle, alert_rx) = driver::spawn(conf)?;

    driver_handle.shutdown().await?;

    Ok(())
}
