use std::{
    sync::atomic::{
        AtomicU16,
        Ordering,
    },
    time::Duration,
};

use askama::Template;
use once_cell::sync::Lazy;
use podman_api::{
    Id,
    Podman,
};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

static HOST_PORT: AtomicU16 = AtomicU16::new(1024);

static STOP_POD_TX: Lazy<UnboundedSender<String>> = Lazy::new(|| {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = std::thread::spawn(move || {
        let podman = init_environment();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        rt.block_on(async move {
            while let Some(pod_name) = rx.recv().await {
                let podman = podman.clone();
                // spawn "fire and forget" tasks so the force removes are sent
                // to podman immediately and without waiting for a server response.
                tokio::spawn(async move {
                    if let Err(e) = podman.pods().get(&pod_name).remove().await {
                        eprintln!("received error while removing pod `{pod_name}`: {e:?}");
                    }
                });
            }
        });
    });
    tx
});

#[derive(Template)]
#[template(path = "conductor_stack.yaml.jinja2")]
struct SequencerRelayerStack<'a> {
    pod_name: &'a str,
    celestia_home_volume: &'a str,
    metro_home_volume: &'a str,
    scripts_host_volume: &'a str,
    executor_home_volume: &'a str,
    relayer_home_volume: &'a str,
    executor_local_account: &'a str,
    celestia_app_host_port: u16,
    bridge_host_port: u16,
    sequencer_host_port: u16,
    sequencer_host_grpc_port: u16,
    executor_host_http_port: u16,
    executor_host_grpc_port: u16,
}

pub fn init_environment() -> Podman {
    #[cfg(target_os = "linux")]
    let podman_dir = {
        let uid = users::get_effective_uid();
        std::path::PathBuf::from(format!("/run/user/{uid}/podman"))
    };
    #[cfg(target_os = "macos")]
    let podman_dir = {
        let home_dir = home::home_dir().expect("there should always be a homedir on macos");
        home_dir.join(".local/share/containers/podman/machine/qemu")
    };
    if podman_dir.exists() {
        Podman::unix(podman_dir.join("podman.sock"))
    } else {
        panic!("podman socket not found at `{}`", podman_dir.display(),);
    }
}

pub struct StackInfo {
    pub pod_name: String,
    pub celestia_app_host_port: u16,
    pub bridge_host_port: u16,
    pub sequencer_host_port: u16,
    pub sequencer_host_grpc_port: u16,
    pub executor_host_http_port: u16,
    pub executor_host_grpc_port: u16,
    tx: UnboundedSender<String>,
}

impl StackInfo {
    pub fn make_bridge_endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.bridge_host_port,)
    }

    pub fn make_sequencer_api_endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.sequencer_host_port,)
    }

    // pub fn make_sequencer_grpc_endpoint(&self) -> String {
    //     format!("http://127.0.0.1:{}", self.sequencer_host_grpc_port,)
    // }

    // pub fn make_executor_endpoint(&self) -> String {
    //     format!("http://127.0.0.0:{}", self.executor_host_http_port,)
    // }

    // pub fn make_executor_grpc_endpoint(&self) -> String {
    //     format!("http://127.0.0.0:{}", self.executor_host_grpc_port,)
    // }
}

impl Drop for StackInfo {
    fn drop(&mut self) {
        if let Err(e) = self.tx.send(self.pod_name.clone()) {
            eprintln!(
                "failed sending pod `{name}` to cleanup task while dropping StackInfo: {e:?}",
                name = self.pod_name,
            )
        }
    }
}

pub async fn init_stack(podman: &Podman) -> StackInfo {
    let id = Uuid::new_v4().simple();
    let pod_name = format!("conductor_stack-{id}");
    let celestia_home_volume = format!("celestia-home-volume-{id}");
    let metro_home_volume = format!("metro-home-volume-{id}");
    let geth_home_volume = format!("geth-home-volume-{id}");
    let relayer_home_volume = format!("relayer-home-volume-{id}");
    let bridge_host_port = HOST_PORT.fetch_add(1, Ordering::Relaxed);
    let celestia_app_host_port = HOST_PORT.fetch_add(1, Ordering::Relaxed);
    let sequencer_host_port = HOST_PORT.fetch_add(1, Ordering::Relaxed);
    let sequencer_host_grpc_port = HOST_PORT.fetch_add(1, Ordering::Relaxed);
    let executor_host_http_port = HOST_PORT.fetch_add(1, Ordering::Relaxed);
    let executor_host_grpc_port = HOST_PORT.fetch_add(1, Ordering::Relaxed);

    let scripts_host_volume = format!("{}/container-scripts/", env!("CARGO_MANIFEST_DIR"));

    let stack = SequencerRelayerStack {
        pod_name: &pod_name,
        celestia_home_volume: &celestia_home_volume,
        metro_home_volume: &metro_home_volume,
        scripts_host_volume: &scripts_host_volume,
        executor_home_volume: &geth_home_volume,
        relayer_home_volume: &relayer_home_volume,
        // steezeburger's local account used for astria development
        executor_local_account: "0xb0E31D878F49Ec0403A25944d6B1aE1bf05D17E1",
        celestia_app_host_port,
        bridge_host_port,
        sequencer_host_port,
        sequencer_host_grpc_port,
        executor_host_http_port,
        executor_host_grpc_port,
    };

    let pod_kube_yaml = stack.render().unwrap();

    let stack_info = StackInfo {
        pod_name,
        celestia_app_host_port,
        bridge_host_port,
        sequencer_host_port,
        sequencer_host_grpc_port,
        executor_host_http_port,
        executor_host_grpc_port,
        tx: Lazy::force(&STOP_POD_TX).clone(),
    };

    if let Err(e) = podman
        .play_kubernetes_yaml(&Default::default(), pod_kube_yaml)
        .await
    {
        eprintln!("failed playing YAML failed on podman: {e:?}");
        panic!("{e:?}");
    }

    stack_info
}

pub async fn wait_until_ready(podman: &Podman, id: impl Into<Id>) {
    let pod = podman.pods().get(id);
    loop {
        let resp = pod.inspect().await.unwrap();
        if resp.state.as_deref() == Some("Running") {
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
