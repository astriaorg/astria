use std::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr,
};

pub const DEFAULT_API_LISTEN_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
pub const DEFAULT_SEQUENCER_URL: &str = "http://sequencer.astria.localdev.me";
pub const DEFAULT_SEQUENCER_ADDRESS: &str = "1c0c490f1b5528d8173c5de46d131160e4b2c0c3";
pub const DEFAULT_SEQUENCER_SECRET: &str =
    "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90";
pub const DEFAULT_CHAIN_ID: &str = "912559";
pub const DEFAULT_EXECUTION_WS_URL: &str = "ws://ws-executor.astria.localdev.me";
