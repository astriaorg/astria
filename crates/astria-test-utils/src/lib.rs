//! Utility functions that are mainly useful when setting up and running tests.

/// The path to the admin token in the celestia node container. Usually defined
/// in `/scripts/generate-token.sh`.
const ADMIN_TOKEN_PATH: &str = "/home/celestia/.admin_token";

/// The name of the container running the celestia node service.
/// Usually defined in `kubernetes/deployment.yml`.
const CELESTIA_NODE_CONTAINER_NAME: &str = "celestia-bridge";

/// Extract the bearer token from the celestia-node client using the
/// kubernetes REST API.
///
/// # Panics
/// This function is intended to be used within tests. As such it does not
/// provide error handling in favour of panics pointing to the exact line that
/// failed.
pub async fn extract_bearer_token_from_celestia_node(namespace: &str) -> String {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{
        api::{
            Api,
            AttachParams,
            ListParams,
            ResourceExt as _,
        },
        Client,
    };
    use tokio::io::AsyncReadExt as _;
    let client = Client::try_default()
        .await
        .expect("should be able to connect to a k8s cluster; is it running?");
    // the namespace is recorded in `k8s/deployment.yml`
    let pod_api: Api<Pod> = Api::namespaced(client.clone(), namespace);

    let pods_msg: &str = "should have been able to list pods in the \
                          `astria-celestia-jsonrpc-client-test` namespace; was the deployment in \
                          k8s/ applied?";
    let mut pods = pod_api
        .list(&ListParams::default())
        .await
        .expect(pods_msg)
        .into_iter();
    let our_pod = pods.next().expect(
        "should have been able to get deployed pod namespace if it exists. Check why the \
         `astria-celestia-jsonrpc-client-test` namespace is empty?",
    );
    let None = pods.next() else {
        panic!("namespace should not have contained more than one pod")
    };
    let mut cat_token = pod_api
        .exec(
            &our_pod.name_any(),
            vec!["cat", ADMIN_TOKEN_PATH],
            &AttachParams::default()
                .stderr(false)
                .container(CELESTIA_NODE_CONTAINER_NAME),
        )
        .await
        .expect(
            "should have been able to cat the auth token from celestia bridge; was it generated?",
        );
    let mut token_bytes = Vec::new();
    cat_token
        .stdout()
        .expect("stdout on attached process should exist by default if not disabled")
        .read_to_end(&mut token_bytes)
        .await
        .expect("should have been able to read token from pod");
    String::from_utf8(token_bytes).expect("token should have contained only utf8 bytes")
}
