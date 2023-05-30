use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

use std::collections::HashMap;

#[rpc(client)]
trait Daser {
    #[method(name = "daser.SamplingStats")]
    async fn sampling_stats(&self) -> Result<SamplingStats, Error>;

    #[method(name = "daser.WaitCatchUp")]
    async fn wait_catch_up(&self) -> Result<(), Error>;
}

#[derive(Deserialize, Debug)]
pub struct SamplingStats {
    pub head_of_sampled_chain: u64,
    pub head_of_catchup: u64,
    pub network_head_height: u64,
    pub failed: HashMap<u64, i64>,
    pub workers: Vec<WorkerStats>,
    pub concurrency: i64,
    pub catch_up_done: bool,
    pub is_running: bool,
}

#[derive(Deserialize, Debug)]
pub struct WorkerStats {
    pub job_type: String,
    pub current: u64,
    pub from: u64,
    pub to: u64,
    #[serde(default)]
    pub error: Option<String>,
}
