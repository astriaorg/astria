//! Main node command
//!
//! Starts the client
use std::{
    net::{
        Ipv4Addr,
        SocketAddr,
        SocketAddrV4,
    },
    path::PathBuf,
    sync::Arc,
};

use astria_consensus::AstriaConsensus;
use clap::{
    crate_version,
    Parser,
};
use eyre::Context;
use fdlimit::raise_fd_limit;
use futures::pin_mut;
// use reth_basic_payload_builder::{
//     BasicPayloadJobGenerator,
//     BasicPayloadJobGeneratorConfig,
// };
use reth_blockchain_tree::{
    config::BlockchainTreeConfig,
    externals::TreeExternals,
    BlockchainTree,
    ShareableBlockchainTree,
};
use reth_config::Config;
// TODO: need to keep the db around for the blockchain tree so leaving these for now
use reth_db::{
    database::Database,
    mdbx::{
        Env,
        WriteMap,
    },
    tables,
    transaction::DbTx,
};
use reth_discv4::DEFAULT_DISCOVERY_PORT;
// reth_interfaces::consensus::ForkchoiceState, // TODO: we will need to add this back use this
// for setting genesis i think
use reth_interfaces::consensus::Consensus;
use reth_network::{
    error::NetworkError,
    NetworkConfig,
    NetworkHandle,
    NetworkManager,
};
// use reth_network_api::NetworkInfo;
// use reth_payload_builder::PayloadBuilderService;
use reth_primitives::ChainSpec;
use reth_primitives::{
    stage::StageId,
    Head,
};
use reth_provider::{
    providers::{
        get_stage_checkpoint,
        BlockchainProvider,
    },
    BlockProvider,
    CanonStateSubscriptions,
    HeaderProvider,
    ShareableDatabase,
};
use reth_revm::Factory;
// use reth_revm_inspectors::stack::Hook;
// use reth_rpc_engine_api::EngineApi;
use reth_staged_sync::utils::init::{
    init_db,
    init_genesis,
};
use reth_tasks::TaskExecutor;
use reth_transaction_pool::{
    EthTransactionValidator,
    TransactionPool,
};
use secp256k1::SecretKey;
// use tokio::sync::{
//     mpsc::unbounded_channel,
//     oneshot,
//     watch,
// };
use tracing::*;

use crate::{
    args::{
        get_secret_key,
        utils::{
            genesis_value_parser,
            parse_socket_address,
        },
        DebugArgs,
        NetworkArgs,
        RpcServerArgs,
    },
    dirs::{
        DataDirPath,
        MaybePlatformPath,
    },
    runner::CliContext,
};
pub mod astria_consensus;
// pub mod events;

/// Start the node
#[derive(Debug, Parser)]
pub struct Command {
    /// The path to the data dir for all reth files and subdirectories.
    ///
    /// Defaults to the OS-specific data directory:
    ///
    /// - Linux: `$XDG_DATA_HOME/reth/` or `$HOME/.local/share/reth/`
    /// - Windows: `{FOLDERID_RoamingAppData}/reth/`
    /// - macOS: `$HOME/Library/Application Support/reth/`
    #[arg(long, value_name = "DATA_DIR", verbatim_doc_comment, default_value_t)]
    datadir: MaybePlatformPath<DataDirPath>,

    /// The path to the configuration file to use.
    #[arg(long, value_name = "FILE", verbatim_doc_comment)]
    config: Option<PathBuf>,

    /// The chain this node is running.
    ///
    /// Possible values are either a built-in chain or the path to a chain specification file.
    ///
    /// Built-in chains:
    /// - mainnet
    /// - goerli
    /// - sepolia
    #[arg(
    long,
    value_name = "CHAIN_OR_PATH",
    verbatim_doc_comment,
    default_value = "mainnet",
    value_parser = genesis_value_parser
    )]
    chain: Arc<ChainSpec>,

    /// Enable Prometheus metrics.
    ///
    /// The metrics will be served at the given interface and port.
    #[arg(long, value_name = "SOCKET", value_parser = parse_socket_address, help_heading = "Metrics")]
    metrics: Option<SocketAddr>,

    #[clap(flatten)]
    network: NetworkArgs,

    #[clap(flatten)]
    rpc: RpcServerArgs,

    #[clap(flatten)]
    debug: DebugArgs,

    /// Automatically mine blocks for new transactions
    #[arg(long)]
    auto_mine: bool,
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self, ctx: CliContext) -> eyre::Result<()> {
        info!(target: "symp::cli", "symphony {} starting", crate_version!());

        // Raise the fd limit of the process.
        // Does not do anything on windows.
        raise_fd_limit();

        // add network name to data dir
        let data_dir = self.datadir.unwrap_or_chain_default(self.chain.chain);
        let config_path = self.config.clone().unwrap_or(data_dir.config_path());

        // TODO: originally "let mut config" but don't need to mutate it
        // keeping this todo because we might need to mutate it
        let config: Config = self.load_config(config_path.clone())?;

        // always store reth.toml in the data dir, not the chain specific data dir
        info!(target: "symp::cli", path = ?config_path, "Configuration loaded");

        let db_path = data_dir.db_path();
        info!(target: "symp::cli", path = ?db_path, "Opening database");
        let db = Arc::new(init_db(&db_path)?);
        info!(target: "symp::cli", "Database opened");

        debug!(target: "symp::cli", chain=%self.chain.chain, genesis=?self.chain.genesis_hash(), "Initializing genesis");

        // TODO: currently unused, but we will need to use this to initialize the chain and
        // forkchoice
        let _genesis_hash = init_genesis(db.clone(), self.chain.clone())?;

        let consensus: Arc<dyn Consensus> = Arc::new(AstriaConsensus::new(self.chain.clone()));

        // configure blockchain tree
        let tree_externals = TreeExternals::new(
            db.clone(),
            Arc::clone(&consensus),
            Factory::new(self.chain.clone()),
            Arc::clone(&self.chain),
        );
        let tree_config = BlockchainTreeConfig::default();
        // The size of the broadcast is twice the maximum reorg depth, because at maximum reorg
        // depth at least N blocks must be sent at once.
        let (canon_state_notification_sender, _receiver) =
            tokio::sync::broadcast::channel(tree_config.max_reorg_depth() as usize * 2);
        let blockchain_tree = ShareableBlockchainTree::new(BlockchainTree::new(
            tree_externals,
            canon_state_notification_sender.clone(),
            tree_config,
        )?);

        // setup the blockchain provider
        let shareable_db = ShareableDatabase::new(Arc::clone(&db), Arc::clone(&self.chain));
        let blockchain_db = BlockchainProvider::new(shareable_db, blockchain_tree.clone())?;

        // TODO: create lobby and waiting tx pools to replace the tx pool below
        let transaction_pool = reth_transaction_pool::Pool::eth_pool(
            EthTransactionValidator::new(blockchain_db.clone(), Arc::clone(&self.chain)),
            Default::default(),
        );
        info!(target: "symp::cli", "Transaction pool initialized");

        // spawn txpool maintenance task
        {
            let pool = transaction_pool.clone();
            let chain_events = blockchain_db.canonical_state_stream();
            let client = blockchain_db.clone();
            ctx.task_executor.spawn_critical(
                "txpool maintenance task",
                Box::pin(async move {
                    reth_transaction_pool::maintain::maintain_transaction_pool(
                        client,
                        pool,
                        chain_events,
                    )
                    .await
                }),
            );
            debug!(target: "symp::cli", "Spawned txpool maintenance task");
        }
        info!(target: "symp::cli", "Connecting to P2P network");
        let network_secret_path = self
            .network
            .p2p_secret_key
            .clone()
            .unwrap_or_else(|| data_dir.p2p_secret_path());
        debug!(target: "symp::cli", ?network_secret_path, "Loading p2p key file");
        let secret_key = get_secret_key(&network_secret_path)?;
        let default_peers_path = data_dir.known_peers_path();
        let network_config = self.load_network_config(
            &config,
            Arc::clone(&db),
            ctx.task_executor.clone(),
            secret_key,
            default_peers_path.clone(),
        );
        let network = self
            .start_network(
                network_config,
                &ctx.task_executor,
                transaction_pool.clone(),
                default_peers_path,
            )
            .await?;
        // let network = NetworkHandle::

        // let (consensus_engine_tx, consensus_engine_rx) = unbounded_channel();

        // TODO: pretty sure we don't need to use channels to talk to consensus anymore
        // TODO: do need to use the genesis hash for initial forkchoice state
        // // Forward genesis as forkchoice state to the consensus engine.
        // // This will allow the downloader to start
        // if self.debug.continuous {
        //     info!(target: "reth::cli", "Continuous sync mode enabled");
        //     let (tip_tx, _tip_rx) = oneshot::channel();
        //     let state = ForkchoiceState {
        //         head_block_hash: genesis_hash,
        //         finalized_block_hash: genesis_hash,
        //         safe_block_hash: genesis_hash,
        //     };
        //     consensus_engine_tx.send(BeaconEngineMessage::ForkchoiceUpdated {
        //         state,
        //         payload_attrs: None,
        //         tx: tip_tx,
        //     })?;
        // }

        // TODO: pretty sure this can be removed, just going to directly call consensus
        // // Forward the `debug.tip` as forkchoice state to the consensus engine.
        // // This will initiate the sync up to the provided tip.
        // let _tip_rx = match self.debug.tip {
        //     Some(tip) => {
        //         let (tip_tx, tip_rx) = oneshot::channel();
        //         let state = ForkchoiceState {
        //             head_block_hash: tip,
        //             finalized_block_hash: tip,
        //             safe_block_hash: tip,
        //         };
        //         consensus_engine_tx.send(BeaconEngineMessage::ForkchoiceUpdated {
        //             state,
        //             payload_attrs: None,
        //             tx: tip_tx,
        //         })?;
        //         debug!(target: "reth::cli", %tip, "Tip manually set");
        //         Some(tip_rx)
        //     }
        //     None => None,
        // };

        // configure the payload builder
        // let payload_generator = BasicPayloadJobGenerator::new(
        //     blockchain_db.clone(),
        //     transaction_pool.clone(),
        //     ctx.task_executor.clone(),
        //     // TODO use extradata from args
        //     BasicPayloadJobGeneratorConfig::default(),
        //     Arc::clone(&self.chain),
        // );

        // TODO: we will need to write our own payload builder, this will take RTBs and turn them
        // into a payload for execution within revm
        // let (payload_service, payload_builder) = PayloadBuilderService::new(payload_generator);

        // debug!(target: "reth::cli", "Spawning payload builder service");
        // ctx.task_executor
        //     .spawn_critical("payload builder service", payload_service);

        // let pipeline_events = pipeline.events();

        // TODO: not sure if we need this, dig into BeaconConsensusEngine and figure out what it
        // does let (beacon_consensus_engine, beacon_engine_handle) =
        // BeaconConsensusEngine::with_channel(     Arc::clone(&db),
        //     client,
        //     pipeline,
        //     blockchain_db.clone(),
        //     Box::new(ctx.task_executor.clone()),
        //     self.debug.max_block,
        //     self.debug.continuous,
        //     payload_builder.clone(),
        //     consensus_engine_tx,
        //     consensus_engine_rx,
        // );
        // info!(target: "reth::cli", "Consensus engine initialized");

        // TODO: pretty sure we don't need this either, dig in here as well
        // let events = stream_select(
        //     stream_select(
        //         network.event_listener().map(Into::into),
        //         beacon_engine_handle.event_listener().map(Into::into),
        //     ),
        //     pipeline_events.map(Into::into),
        // );
        // let engine_api = EngineApi::new(
        //     blockchain_db.clone(),
        //     self.chain.clone(),
        //     beacon_engine_handle,
        //     payload_builder.into(),
        // );
        // info!(target: "reth::cli", "Engine API handler initialized");

        // extract the jwt secret from the args if possible
        // let default_jwt_path = data_dir.jwt_path();
        // let jwt_secret = self.rpc.jwt_secret(default_jwt_path)?;

        // Start RPC servers
        // let (_rpc_server, _auth_server) = self
        //     .rpc
        //     .start_servers(
        //         blockchain_db.clone(),
        //         transaction_pool.clone(),
        //         network.clone(),
        //         ctx.task_executor.clone(),
        //         blockchain_tree,
        //         engine_api,
        //         jwt_secret,
        //     )
        //     .await?;

        // just start the rpc server, we don't need the auth server
        let _rpc_server = self
            .rpc
            .start_rpc_server(
                blockchain_db.clone(),
                transaction_pool.clone(),
                network.clone(),
                ctx.task_executor.clone(),
                blockchain_tree,
            )
            .await?;

        // let _rpc_server = self
        //     .rpc
        //     .start_rpc_server(
        //         blockchain_db.clone(),
        //         transaction_pool.clone(),
        //         network.clone(),
        //         ctx.task_executor.clone(),
        //         blockchain_tree.clone(),
        //     )
        //     .await?;

        // TODO: don't need the consensus engine, swap this out with out own thing
        // // Run consensus engine to completion
        // let (tx, rx) = oneshot::channel();
        // info!(target: "reth::cli", "Starting consensus engine");
        // ctx.task_executor
        //     .spawn_critical("consensus engine", async move {
        //         let res = beacon_consensus_engine.await;
        //         let _ = tx.send(res);
        //     });

        // rx.await??;

        info!(target: "symp::cli", "Consensus engine has exited.");

        if self.debug.terminate {
            Ok(())
        } else {
            // The pipeline has finished downloading blocks up to `--debug.tip` or
            // `--debug.max-block`. Keep other node components alive for further usage.
            futures::future::pending().await
        }
    }

    fn load_network_config(
        &self,
        config: &Config,
        db: Arc<Env<WriteMap>>,
        executor: TaskExecutor,
        secret_key: SecretKey,
        default_peers_path: PathBuf,
    ) -> NetworkConfig<ShareableDatabase<Arc<Env<WriteMap>>>> {
        let head = self
            .lookup_head(Arc::clone(&db))
            .expect("the head block is missing");

        self.network
            .network_config(config, self.chain.clone(), secret_key, default_peers_path)
            .with_task_executor(Box::new(executor))
            .set_head(head)
            // .listener_addr(SocketAddr::V4(SocketAddrV4::new(
            //     Ipv4Addr::UNSPECIFIED,
            //     self.network.port.unwrap_or(DEFAULT_DISCOVERY_PORT),
            // )))
            // .discovery_addr(SocketAddr::V4(SocketAddrV4::new(
            //     Ipv4Addr::UNSPECIFIED,
            //     self.network
            //         .discovery
            //         .port
            //         .unwrap_or(DEFAULT_DISCOVERY_PORT),
            // )))
            .build(ShareableDatabase::new(db, self.chain.clone()))
    }

    fn lookup_head(
        &self,
        db: Arc<Env<WriteMap>>,
    ) -> Result<Head, reth_interfaces::db::DatabaseError> {
        db.view(|tx| {
            let head = get_stage_checkpoint(tx, StageId::Finish)?
                .unwrap_or_default()
                .block_number;
            let header = tx
                .get::<tables::Headers>(head)?
                .expect("the header for the latest block is missing, database is corrupt");
            let total_difficulty = tx.get::<tables::HeaderTD>(head)?.expect(
                "the total difficulty for the latest block is missing, database is corrupt",
            );
            let hash = tx
                .get::<tables::CanonicalHeaders>(head)?
                .expect("the hash for the latest block is missing, database is corrupt");
            Ok::<Head, reth_interfaces::db::DatabaseError>(Head {
                number: head,
                hash,
                difficulty: header.difficulty,
                total_difficulty: total_difficulty.into(),
                timestamp: header.timestamp,
            })
        })?
        .map_err(Into::into)
    }

    /// Loads the reth config with the given datadir root
    fn load_config(&self, config_path: PathBuf) -> eyre::Result<Config> {
        confy::load_path::<Config>(config_path.clone())
            .wrap_err_with(|| format!("Could not load config file {:?}", config_path))
    }

    /// Spawns the configured network and associated tasks and returns the [NetworkHandle] connected
    /// to that network.
    async fn start_network<C, Pool>(
        &self,
        config: NetworkConfig<C>,
        task_executor: &TaskExecutor,
        pool: Pool,
        default_peers_path: PathBuf,
    ) -> Result<NetworkHandle, NetworkError>
    where
        C: BlockProvider + HeaderProvider + Clone + Unpin + 'static,
        Pool: TransactionPool + Unpin + 'static,
    {
        let client = config.client.clone();
        let (handle, network, txpool, eth) = NetworkManager::builder(config)
            .await?
            .transactions(pool)
            .request_handler(client)
            .split_with_handle();

        let known_peers_file = self.network.persistent_peers_file(default_peers_path);
        task_executor.spawn_critical_with_signal("p2p network task", |shutdown| {
            run_network_until_shutdown(shutdown, network, known_peers_file)
        });

        task_executor.spawn_critical("p2p eth request handler", eth);
        task_executor.spawn_critical("p2p txpool request handler", txpool);

        Ok(handle)
    }
}

// TODO: this fn deals with some peers stuff and I'm pretty sure we don't need it
/// Drives the [NetworkManager] future until a [Shutdown](reth_tasks::shutdown::Shutdown) signal is
/// received. If configured, this writes known peers to `persistent_peers_file` afterwards.
async fn run_network_until_shutdown<C>(
    shutdown: reth_tasks::shutdown::Shutdown,
    network: NetworkManager<C>,
    persistent_peers_file: Option<PathBuf>,
) where
    C: BlockProvider + HeaderProvider + Clone + Unpin + 'static,
{
    pin_mut!(network, shutdown);

    tokio::select! {
        _ = &mut network => {},
        _ = shutdown => {},
    }

    if let Some(file_path) = persistent_peers_file {
        let known_peers = network.all_peers().collect::<Vec<_>>();
        if let Ok(known_peers) = serde_json::to_string_pretty(&known_peers) {
            trace!(target : "reth::cli", peers_file =?file_path, num_peers=%known_peers.len(), "Saving current peers");
            let parent_dir = file_path.parent().map(std::fs::create_dir_all).transpose();
            match parent_dir.and_then(|_| std::fs::write(&file_path, known_peers)) {
                Ok(_) => {
                    info!(target: "reth::cli", peers_file=?file_path, "Wrote network peers to file");
                }
                Err(err) => {
                    warn!(target: "reth::cli", ?err, peers_file=?file_path, "Failed to write network peers to file");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::IpAddr,
        path::Path,
    };

    use super::*;

    #[test]
    fn parse_help_node_command() {
        let err = Command::try_parse_from(["reth", "--help"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn parse_common_node_command_chain_args() {
        for chain in ["mainnet", "sepolia", "goerli"] {
            let args: Command = Command::parse_from(["reth", "--chain", chain]);
            assert_eq!(args.chain.chain, chain.parse().unwrap());
        }
    }

    #[test]
    fn parse_discovery_port() {
        let cmd = Command::try_parse_from(["reth", "--discovery.port", "300"]).unwrap();
        assert_eq!(cmd.network.discovery.port, Some(300));
    }

    #[test]
    fn parse_port() {
        let cmd =
            Command::try_parse_from(["reth", "--discovery.port", "300", "--port", "99"]).unwrap();
        assert_eq!(cmd.network.discovery.port, Some(300));
        assert_eq!(cmd.network.port, Some(99));
    }

    #[test]
    fn parse_config_path() {
        let cmd = Command::try_parse_from(["reth", "--config", "my/path/to/reth.toml"]).unwrap();
        // always store reth.toml in the data dir, not the chain specific data dir
        let data_dir = cmd.datadir.unwrap_or_chain_default(cmd.chain.chain);
        let config_path = cmd.config.unwrap_or(data_dir.config_path());
        assert_eq!(config_path, Path::new("my/path/to/reth.toml"));

        let cmd = Command::try_parse_from(["reth"]).unwrap();

        // always store reth.toml in the data dir, not the chain specific data dir
        let data_dir = cmd.datadir.unwrap_or_chain_default(cmd.chain.chain);
        let config_path = cmd.config.clone().unwrap_or(data_dir.config_path());
        assert!(
            config_path.ends_with("reth/mainnet/reth.toml"),
            "{:?}",
            cmd.config
        );
    }

    #[test]
    fn parse_db_path() {
        let cmd = Command::try_parse_from(["reth"]).unwrap();
        let data_dir = cmd.datadir.unwrap_or_chain_default(cmd.chain.chain);
        let db_path = data_dir.db_path();
        assert!(db_path.ends_with("reth/mainnet/db"), "{:?}", cmd.config);

        let cmd = Command::try_parse_from(["reth", "--datadir", "my/custom/path"]).unwrap();
        let data_dir = cmd.datadir.unwrap_or_chain_default(cmd.chain.chain);
        let db_path = data_dir.db_path();
        assert_eq!(db_path, Path::new("my/custom/path/db"));
    }
}
