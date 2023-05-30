use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

#[rpc(client)]
trait Node {
    #[method(name = "node.AuthNew")]
    async fn auth_new(&self) -> Result<serde_json::Value, Error>;
    
    #[method(name = "node.AuthVerify")]
    async fn auth_verify(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "node.Info")]
    async fn info(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "node.LogLevelSet")]
    async fn log_level_set(&self) -> Result<serde_json::Value, Error>;
}
