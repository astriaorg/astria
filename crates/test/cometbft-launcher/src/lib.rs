//! Launch cometbft as a subprocess for running tests.
//!
//! # Examples
//! ```no_run
//! # tokio_test::block_on(async {
//! use cometbft_launcher::CometBft;
//! let cometbft = CometBft::builder()
//!     .proxy_app("tcp://127.0.0.1:26657")
//!     .launch()
//!     .await
//!     .unwrap();
//! # })

use std::{
    ffi::{
        OsStr,
        OsString,
    },
    net::SocketAddr,
    path::Path,
    process::Stdio,
    time::Duration,
};

use anyhow::{
    ensure,
    Context as _,
};
use tempfile::TempDir;
use tokio::process::{
    Child,
    Command,
};

/// `CometBft` running in a subprocess.
pub struct CometBft {
    /// The temporary directory within cometbft created its config files.
    pub home: TempDir,
    /// A handle to the subprocess running cometbft.
    pub process: Child,
    /// The local TCP socket address on which cometbft is serving RPCs.
    pub rpc_listen_addr: SocketAddr,
}

impl CometBft {
    /// Configure a cometbft instance.
    #[must_use]
    pub fn builder() -> CometBftBuilder {
        CometBftBuilder::new()
    }
}

/// A builder struct to configure how cometbft is launched.
#[derive(Default)]
pub struct CometBftBuilder {
    proxy_app: Option<OsString>,
}

impl CometBftBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Will pass `proxy_app` to `cometbft start --proxy_app`.
    ///
    /// Default is to pass `cometbft start --proxy_app=noop`. `proxy_app` will
    /// be passed as-is without extra verification.
    #[must_use]
    pub fn proxy_app(self, proxy_app: impl AsRef<OsStr>) -> Self {
        Self {
            proxy_app: Some(proxy_app.as_ref().to_os_string()),
        }
    }

    /// Launches cometbft in a subprocess consuming the builder.
    ///
    /// # Errors
    /// This function will retun errors under the following conditions:
    /// - the `cometbft` exectuble could not be found in `PATH` or the `COMETBFT` env var;
    /// - creating a tempdir to hold the cometbft config failed;
    /// - running `cometbft init` failed;
    /// - setting the rpc and p2p listen addresses failed overwriting cometbft toml;
    /// - running `cometbft start` failed;
    /// - finding the socket over which cometbft serves its RPCs failed.
    pub async fn launch(self) -> anyhow::Result<CometBft> {
        let Self {
            proxy_app,
        } = self;
        let executable = find_cometbft_executable().await.context(
            "could not find cometbft executable; set the env var COMETBFT=path/to/cometbft or add \
             it to PATH; see this url for how to install: https://github.com/cometbft/cometbft/blob/6a7dfb40d0302de5cdbca51cc4daede74b6d44c2/docs/guides/install.md"
        )?;

        let home = tempfile::tempdir()
            .context("failed creating a tempdir to store cometbft config files in")?;

        init_cometbft(&executable, home.path().as_os_str())
            .await
            .context("failed to initialize cometbft")?;

        set_listen_addresses(home.path())
            .await
            .context("failed setting listen addresses")?;

        let proxy_app = proxy_app.as_deref().unwrap_or("noop".as_ref());
        let process = start_cometbft(&executable, home.path().as_os_str(), proxy_app)
            .context("failed starting cometbft")?;

        let rpc_listen_addr = find_rpc_listen_address(&process)
            .await
            .context("failed finding ports cometbft is listening on")?;
        Ok(CometBft {
            home,
            process,
            rpc_listen_addr,
        })
    }
}

/// Returns the cometbft rpc listenaddress by sending a HTTP GET using the /status endpoint.
async fn find_rpc_listen_address(process: &Child) -> anyhow::Result<SocketAddr> {
    let pid = process
        .id()
        .context(
            "cometbft process did not have a PID; this can only happen if it already completed",
        )?
        .try_into()
        .expect("failed converting a u32 PID to i32; this should never fail in practice");

    let rpc_listen_addr = tryhard::retry_fn(move || async move {
        let addrs = tokio::task::spawn_blocking(move || enumerate_local_tcp_socket_addrs(pid))
            .await
            .context("task panicked looking for ports cometbft is listening on")?
            .context("failed looking for local ports")?;
        let client = reqwest::Client::new();
        for addr in addrs {
            let Ok(response) = client
                .get(format!("http://{addr}/status"))
                .timeout(Duration::from_millis(100))
                .send()
                .await
            else {
                continue;
            };
            if response.status().is_success() {
                return Ok(addr);
            }
        }
        Err(anyhow::anyhow!(
            "no cometbft tcp socket found that responds to /status"
        ))
    })
    .retries(9)
    .fixed_backoff(Duration::from_secs(1))
    .await
    .context("finding local tcp addresses failed after 10 attempts")?;

    Ok(rpc_listen_addr)
}

/// Returns all TCP socket addresses of the process identified by `pid`.
#[cfg(target_os = "macos")]
fn enumerate_local_tcp_socket_addrs(pid: i32) -> anyhow::Result<Vec<SocketAddr>> {
    use std::net::{
        IpAddr,
        Ipv4Addr,
        Ipv6Addr,
    };

    use anyhow::Error;
    use libc::{
        AF_INET,
        AF_INET6,
    };
    use libproc::libproc::{
        bsd_info::BSDInfo,
        file_info::{
            pidfdinfo,
            ListFDs,
            ProcFDType,
        },
        net_info::{
            SocketFDInfo,
            SocketInfoKind,
        },
        proc_pid::{
            listpidinfo,
            pidinfo,
        },
    };

    let info = pidinfo::<BSDInfo>(pid, 0)
        .map_err(Error::msg)
        .context("failed getting process info")?;
    let mut local_addresses = vec![];
    for fd in listpidinfo::<ListFDs>(pid, info.pbi_nfiles as usize)
        .map_err(Error::msg)
        .context("failed listing file descriptors associated with pid")?
    {
        if let ProcFDType::Socket = fd.proc_fdtype.into() {
            let socket = pidfdinfo::<SocketFDInfo>(pid, fd.proc_fd)
                .map_err(Error::msg)
                .context("failed getting info on socket fd")?;
            if let SocketInfoKind::Tcp = socket.psi.soi_kind.into() {
                // unsafe ok because union is discriminated by soi_kind
                let info = unsafe { socket.psi.soi_proto.pri_tcp };
                let port = u16::from_be(info.tcpsi_ini.insi_lport.try_into().expect(
                    "converting an i32 local port to u16 failed; this should never happen in \
                     practice",
                ));
                // For reference on accessing union fields:
                // https://github.com/apple-oss-distributions/lsof/blob/a26b67d2f0c6600d269f0b33233a2cb4b877b279/lsof/dialects/darwin/libproc/dsock.c#L144
                let ip_addr: IpAddr = match socket.psi.soi_family {
                    AF_INET => {
                        // unsafe access ok because the union in insi_laddr is discriminated
                        // by soi_family
                        let wire_addr = unsafe { info.tcpsi_ini.insi_laddr.ina_46.i46a_addr4 };
                        // the stored laddr is big endian, but the From<u32> for Ipv4Addr
                        // impl assumes host endianness.
                        // u32::to_le_bytes fixes this with a hack putting the bytes in the
                        // "correct" order. Note that this only makes sense
                        // on little endian machines.
                        let addr: Ipv4Addr = wire_addr.s_addr.to_le_bytes().into();
                        addr.into()
                    }
                    AF_INET6 => {
                        let wire_addr = unsafe { info.tcpsi_ini.insi_laddr.ina_6 };
                        let addr: Ipv6Addr = wire_addr.s6_addr.into();
                        addr.into()
                    }
                    _ => continue,
                };
                let socket_addr = (ip_addr, port).into();
                local_addresses.push(socket_addr);
            }
        }
    }
    Ok(local_addresses)
}

/// Returns all TCP socket addresses of the process identified by `pid`.
#[cfg(target_os = "linux")]
fn enumerate_local_tcp_socket_addrs(pid: i32) -> anyhow::Result<Vec<SocketAddr>> {
    use procfs::process::Process;
    let process = Process::new(pid).context("failed getting process")?;
    let local_ipv4_addresses = process
        .tcp()
        .context("failed reading v4 tcp addresses of process")?;
    let local_ipv6_addresses = process
        .tcp6()
        .context("failed reading v6 tcp addresses of process")?;
    Ok(local_ipv4_addresses
        .into_iter()
        .chain(local_ipv6_addresses)
        .map(|entry| entry.local_address)
        .collect())
}

/// Sets cometbft config RPC laddr and P2P laddr to 127.0.0.1:0.
async fn set_listen_addresses(dir: &Path) -> anyhow::Result<()> {
    use toml_edit::{
        Document,
        Item,
        Value,
    };
    let cfg_path = dir.join("config/config.toml");
    let toml = tokio::fs::read_to_string(&cfg_path)
        .await
        .context("failed reading config file to buffer")?;
    let mut doc = toml
        .parse::<Document>()
        .context("could not parse config toml")?;
    let rpc_laddr = doc
        .get_mut("rpc")
        .context("config did not contain rpc table")?
        .get_mut("laddr")
        .context("config did not contain laddr field in rpc table")?;
    *rpc_laddr = Item::Value(Value::from("tcp://127.0.0.1:0"));
    let p2p_laddr = doc
        .get_mut("p2p")
        .context("config did not contain p2p table")?
        .get_mut("laddr")
        .context("config did not contain laddr field in p2p table")?;
    *p2p_laddr = Item::Value(Value::from("tcp://127.0.0.1:0"));
    tokio::fs::write(&cfg_path, doc.to_string())
        .await
        .context("failed writing updated cfg toml to disk")?;
    Ok(())
}

/// Runs `cometbft init` in the provided home directory.
async fn init_cometbft(executable: &OsStr, home: &OsStr) -> anyhow::Result<()> {
    let init_out = Command::new(executable)
        .arg("init")
        .arg("--home")
        .arg(home)
        .output()
        .await
        .context("failed to execute `cometbft init`")?;

    ensure!(
        init_out.status.success(),
        "cometbft exited with non-zero code: {}\nstdout: {}\nstderr: {}",
        init_out.status,
        String::from_utf8_lossy(&init_out.stdout),
        String::from_utf8_lossy(&init_out.stderr),
    );

    Ok(())
}

/// Runs `cometbft start` in the provided home directory and using `proxy_app`.
fn start_cometbft(executable: &OsStr, home: &OsStr, proxy_app: &OsStr) -> anyhow::Result<Child> {
    Command::new(executable)
        .arg("start")
        .arg("--home")
        .arg(home)
        .arg("--proxy_app")
        .arg(proxy_app)
        .kill_on_drop(true)
        .stdout(Stdio::null())
        .spawn()
        .context("failed executing `cometbft start`")
}

async fn find_cometbft_executable() -> anyhow::Result<OsString> {
    if let Some(exec) = std::env::var_os("COMETBFT") {
        return Ok(exec);
    }
    tokio::task::spawn_blocking(|| which::which("cometbft"))
        .await
        .context("thread panicked looking for `cometbft` in PATH")?
        .context("could not finding cometbft executable in PATH")
        .map(Into::into)
}
