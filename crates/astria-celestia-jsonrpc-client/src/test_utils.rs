//! Utilities that are shared between all tests.
use astria_test_utils::extract_bearer_token_from_celestia_node;

/// The namespace of the kubernetes deployment used to run the jsonrpc tests.
/// recorded in [../k8s/deployment.yml].
const TEST_K8S_NAMESPACE: &str = "astria-celestia-jsonrpc-client-test";

/// Generate a client with the celestia JSON RPC endpoint and bearer token set.
///
/// [`TEST_ENDPOINT`] is the default endpoint against which RPCs are called, and
/// [`TEST_K8S_NAMESPACE`] is the default kubernetes namespace within which the celestia
/// node is deployed and from where the bearer token is extracted.
///
/// Namespace and json rpc endpoint can be configured with the environment variables
/// `NAMESPACE` and `ENDPOINT` (useful for debugging).
pub(crate) async fn make_client() -> crate::Client {
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| TEST_K8S_NAMESPACE.to_string());
    let endpoint = std::env::var("ENDPOINT")
        .unwrap_or_else(|_| format!("http://{namespace}.localdev.me:80/jsonrpc/"));
    let token = extract_bearer_token_from_celestia_node(&namespace).await;
    crate::Client::builder()
        .bearer_token(&token)
        .endpoint(&endpoint)
        .build()
        .unwrap()
}
