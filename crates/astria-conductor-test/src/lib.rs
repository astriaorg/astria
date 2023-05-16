use std::time::Duration;

use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::Namespace,
};
use kube::{
    api::{
        DeleteParams,
        DynamicObject,
        Patch,
        PatchParams,
        PostParams,
    },
    core::{
        GroupVersionKind,
        ObjectMeta,
    },
    discovery::{
        ApiCapabilities,
        ApiResource,
        Scope,
    },
    runtime::wait::{
        await_condition,
        Condition,
    },
    Api,
    Client,
    Discovery,
    ResourceExt,
};
use once_cell::sync::Lazy;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

const TEST_ENVIRONMENT_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/kubernetes/test-environment.yml"
));
const TEST_INGRESS_TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/kubernetes/ingress.yml.j2"
));

static STOP_POD_TX: Lazy<UnboundedSender<String>> = Lazy::new(|| {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async move {
            let client = Client::try_default()
                .await
                .expect("should be able to connect to kuberneter cluster; is it running?");
            while let Some(namespace) = rx.recv().await {
                // spawn "fire and forget" tasks so the force removes are sent
                // to kubernetes immediately and without waiting for a server response.
                let client = client.clone();
                tokio::spawn(async move { delete_test_environment(namespace, client).await });
            }
        });
    });
    tx
});

pub async fn init_test() -> TestEnvironment {
    TestEnvironment::init().await
}

pub struct TestEnvironment {
    pub host: String,
    pub namespace: String,
    pub tx: UnboundedSender<String>,
}

impl TestEnvironment {
    pub fn bridge_endpoint(&self) -> String {
        format!(
            "http://{namespace}.localdev.me/bridge",
            namespace = self.namespace
        )
    }

    pub fn sequencer_endpoint(&self) -> String {
        format!(
            "http://{namespace}.localdev.me/sequencer",
            namespace = self.namespace
        )
    }

    async fn init() -> Self {
        let namespace = Uuid::new_v4().simple().to_string();
        let client = Client::try_default()
            .await
            .expect("should be able to connect to kuberneter cluster; is it running?");
        let discovery = Discovery::new(client.clone())
            .run()
            .await
            .expect("should be able to run discovery against cluster");
        let documents = multidoc_deserialize(TEST_ENVIRONMENT_YAML).expect(
            "should have been able to deserialize valid kustomize generated yaml; rerun `just \
             kustomize`?",
        );

        // Create the unique namespace
        create_namespace(
            &namespace,
            client.clone(),
            &PostParams {
                dry_run: false,
                field_manager: Some("astria-conductor-test".to_string()),
            },
        )
        .await;

        // Apply the kustomize-generated kube yaml
        let ssapply = PatchParams::apply("astria-conductor-test").force();
        for doc in documents {
            apply_yaml_value(&namespace, client.clone(), doc, &ssapply, &discovery).await;
        }

        // Set up the ingress rule under the same namespace
        let ingress_yaml = populate_ingress_template(&namespace);
        apply_yaml_value(
            &namespace,
            client.clone(),
            ingress_yaml,
            &ssapply,
            &discovery,
        )
        .await;

        // Wait for the deployment to become available; this usually takes much longer than
        // setting up ingress rules or anything else.
        let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
        await_condition(
            deployment_api,
            "conductor-environment-deployment",
            is_deployment_available(),
        )
        .await
        .unwrap();

        // The deployment contains startupProbes to ensure that the deployment is
        // only available once its containers are available. However, nginx (the ingress
        // controller) has a small delay between the deployment becoming available and
        // being able to route requests to its services.
        tokio::join!(
            wait_until_bridge_is_available(&namespace),
            wait_until_sequencer_is_available(&namespace),
        );

        let host = format!("http://{namespace}.localdev.me");
        Self {
            host,
            namespace,
            tx: Lazy::force(&STOP_POD_TX).clone(),
        }
    }
}

fn is_deployment_available() -> impl Condition<Deployment> {
    move |obj: Option<&Deployment>| {
        if let Some(deployment) = &obj {
            if let Some(status) = &deployment.status {
                if let Some(conds) = &status.conditions {
                    if let Some(dcond) = conds.iter().find(|c| c.type_ == "Available") {
                        return dcond.status == "True";
                    }
                }
            }
        }
        false
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if let Err(e) = self.tx.send(self.namespace.clone()) {
            eprintln!(
                "failed sending kubernetes namespace `{namespace}` to cleanup task while dropping \
                 TestEnvironment: {e:?}",
                namespace = self.namespace,
            )
        }
    }
}

fn multidoc_deserialize(data: &str) -> eyre::Result<Vec<serde_yaml::Value>> {
    use serde::Deserialize;
    let mut docs = vec![];
    for de in serde_yaml::Deserializer::from_str(data) {
        docs.push(serde_yaml::Value::deserialize(de)?);
    }
    Ok(docs)
}

fn dynamic_api(
    ar: ApiResource,
    caps: ApiCapabilities,
    client: Client,
    namespace: &str,
) -> Api<DynamicObject> {
    if caps.scope == Scope::Cluster {
        Api::all_with(client, &ar)
    } else {
        Api::namespaced_with(client, namespace, &ar)
    }
}

async fn apply_yaml_value(
    namespace: &str,
    client: Client,
    document: serde_yaml::Value,
    ssapply: &PatchParams,
    discovery: &Discovery,
) {
    let obj: DynamicObject = serde_yaml::from_value(document).expect(
        "should have been able to read valid kustomize generated doc into dynamic object; rerun \
         `just kustomize`?",
    );
    let gvk = if let Some(tm) = &obj.types {
        GroupVersionKind::try_from(tm)
            .expect("failed reading group version kind from dynamic object types")
    } else {
        panic!("cannot apply object without valid TypeMeta: {obj:?}");
    };
    let name = obj.name_any();
    let Some((ar, caps)) = discovery.resolve_gvk(&gvk) else {
        panic!("cannot apply document for unknown group version kind: {gvk:?}");
    };
    let api = dynamic_api(ar, caps, client, namespace);
    let data: serde_json::Value = serde_json::to_value(&obj)
        .expect("should have been able to turn DynamicObject serde_json Value");
    let _r = api
        .patch(&name, ssapply, &Patch::Apply(data))
        .await
        .expect("should have been able to apply patch");
}

async fn delete_test_environment(namespace: String, client: Client) {
    delete_namespace(
        &namespace,
        client.clone(),
        &DeleteParams {
            grace_period_seconds: Some(0),
            ..DeleteParams::default()
        },
    )
    .await;
}

async fn create_namespace(namespace: &str, client: Client, params: &PostParams) {
    let api: Api<Namespace> = Api::all(client);
    api.create(
        params,
        &Namespace {
            metadata: ObjectMeta {
                name: Some(namespace.to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .await
    .expect("should have been able to create the unique namespace; does it exist?");
}

async fn delete_namespace(namespace: &str, client: Client, params: &DeleteParams) {
    let api: Api<Namespace> = Api::all(client);
    api.delete(namespace, params)
        .await
        .expect("should have been able to delete the unique namespace; does it exist?");
}

fn populate_ingress_template(namespace: &str) -> serde_yaml::Value {
    let mut jinja_env = minijinja::Environment::new();
    jinja_env
        .add_template("ingress.yml", TEST_INGRESS_TEMPLATE)
        .expect("compile-time loaded ingress should be valid jinja");
    let ingress_template = jinja_env
        .get_template("ingress.yml")
        .expect("ingress.yml was just loaded, it should exist");
    serde_yaml::from_str(
        &ingress_template
            .render(minijinja::context!(namespace => namespace))
            .expect("should be able to render the ingress jinja template"),
    )
    .expect("should be able to parse rendered ingress yaml as serde_yaml Value")
}

async fn wait_until_bridge_is_available(namespace: &str) {
    let client = reqwest::Client::builder()
        .build()
        .expect("building a basic reqwest client should never fail");
    let url = reqwest::Url::parse(&format!("http://{namespace}.localdev.me/bridge/header/1"))
        .expect("bridge endpoint should be a valid url");
    loop {
        if client
            .get(url.clone())
            .send()
            .await
            .expect("sending a get request should not fail")
            .error_for_status()
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn wait_until_sequencer_is_available(namespace: &str) {
    let client = reqwest::Client::builder()
        .build()
        .expect("building a basic reqwest client should never fail");
    let url = reqwest::Url::parse(&format!(
        "http://{namespace}.localdev.me/sequencer/cosmos/base/tendermint/v1beta1/blocks/latest"
    ))
    .expect("sequencer endpoint should be a valid url");
    loop {
        if client
            .get(url.clone())
            .send()
            .await
            .expect("sending a get request should not fail")
            .error_for_status()
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
