use color_eyre::eyre::Result;
use tokio;

use astria_conductor::run;

#[tokio::main]
async fn main() -> Result<()> {
    run().await?;
    Ok(())
}
