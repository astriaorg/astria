use jsonrpsee::proc_macros::rpc;

#[rpc(client)]
trait Header {
    #[method(name = "header.GetByHash")]
    async fn get_by_hash(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "header.GetByHeight")]
    async fn get_by_height(&self, height: u64) -> Result<serde_json::Value, Error>;

    #[method(name = "header.GetVerifiedRangeByHeight")]
    async fn get_verified_range_by_height(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "header.LocalHead")]
    async fn local_head(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "header.LocalHead")]
    async fn network_head(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "header.Subscribe")]
    async fn subscribe(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "header.SyncState")]
    async fn sync_state(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "header.SyncWait")]
    async fn sync_wait(&self) -> Result<serde_json::Value, Error>;
}
