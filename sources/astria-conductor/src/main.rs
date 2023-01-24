use std::{thread, time};
use flexi_logger::{Duplicate, FileSpec};
use tokio::task;

mod driver;
mod error;

#[tokio::main]
async fn main() {
    // log to file and stderr
    flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory("/tmp/astria-rv-rs"))
        .duplicate_to_stderr(Duplicate::All)
        .start()
        .unwrap();

    // PoC usage of driver and driver_cmd_tx
    let (mut driver, driver_cmd_tx) = driver::Driver::new().unwrap();
    let _ = task::spawn(async move { driver.run().await });
    driver_cmd_tx.send(driver::DriverCommand::GetNewBlocks { last_block_height: 53 }).ok();
    // NOTE - fake some computation time so the previous command has time to run before shutdown
    thread::sleep(time::Duration::from_secs(1));
    driver_cmd_tx.send(driver::DriverCommand::Shutdown).ok();
}
