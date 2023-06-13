/// A vendored copy of the unpublished [`tracing-tower`] crate:
///
/// Based on [`penumbra-tracing-tower`].
/// [`tracing-tower`]: https://github.com/tokio-rs/tracing/tree/e603c2a254d157a25a7a1fbfd4da46ad7e05f555/tracing-towerpub mod trace;
/// [`penumbra-tracing-tower`]: https://github.com/penumbra-zone/penumbra/tree/22cbaffe5843f3e1be86ac1a27591db01d0368b4/crates/util/tower-trace
pub mod trace;

mod request_ext;
use std::net::SocketAddr;

pub use request_ext::RequestExt;

// Extracted from tonic's remote_addr implementation; we'd like to instrument
// spans with the remote addr at the server level rather than at the individual
// request level, but the hook available to do that gives us an http::Request
// rather than a tonic::Request, so the tonic::Request::remote_addr method isn't
// available.
pub fn remote_addr(req: &http::Request<()>) -> Option<SocketAddr> {
    use tonic::transport::server::TcpConnectInfo;
    // NOTE: needs to also check TlsConnectInfo if we use TLS
    req.extensions()
        .get::<TcpConnectInfo>()
        .and_then(|i| i.remote_addr())
}
