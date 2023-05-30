use jsonrpsee::proc_macros::rpc;
use serde::Deserialize;

#[rpc(client)]
trait P2p {
    #[method(name = "p2p.Info")]
    async fn info(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.Peers")]
    async fn peers(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.PeerInfo")]
    async fn peer_info(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.Connect")]
    async fn connect(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.ClosePeer")]
    async fn close_peer(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.Connectedness")]
    async fn connectedness(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.NATStatus")]
    async fn nat_status(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.BlockPeer")]
    async fn block_peer(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.UnblockPeer")]
    async fn unblock_peer(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.ListBlockedPeers")]
    async fn list_blocked_peers(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.Protect")]
    async fn protect(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.Unprotect")]
    async fn unprotect(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.IsProtected")]
    async fn is_protected(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.BandwidthStats")]
    async fn bandwidth_stats(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.BandwidthForPeer")]
    async fn bandwidth_for_peer(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.BandwidthForProtocol")]
    async fn bandwidth_for_protocol(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.ResourceState")]
    async fn resource_state(&self) -> Result<serde_json::Value, Error>;

    #[method(name = "p2p.PubSubPeers")]
    async fn pub_sub_peers(&self) -> Result<serde_json::Value, Error>;
}

