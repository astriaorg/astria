use std::time::Duration;

use astria_eyre::eyre::{
    self,
    eyre,
    Report,
    WrapErr as _,
};
use tokio::{
    select,
    task::JoinHandle,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use tracing::{
    error,
    info,
    info_span,
    instrument,
    warn,
};

use crate::{
    executor,
    Config,
    Metrics,
};

/// Exit value of the inner conductor impl to signal to the outer task whether to restart or
/// shutdown
#[derive(Debug, PartialEq, Eq)]
pub(super) enum RestartOrShutdown {
    Restart,
    Shutdown,
}

impl std::fmt::Display for RestartOrShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            RestartOrShutdown::Restart => "restarting",
            RestartOrShutdown::Shutdown => "shutting down",
        };
        f.write_str(msg)
    }
}

/// The business logic of Conductur.
pub(super) struct Inner {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown_token: CancellationToken,

    config: Config,

    executor: Option<JoinHandle<eyre::Result<Option<crate::executor::State>>>>,
}

impl Inner {
    /// Create a new [`Inner`] from a [`Config`].
    ///
    /// # Errors
    /// Returns an error in the following cases if one of its constituent
    /// actors could not be spawned (executor, sequencer reader, or data availability reader).
    /// This usually happens if the actors failed to connect to their respective endpoints.
    pub(super) fn new(
        config: Config,
        metrics: &'static Metrics,
        shutdown_token: CancellationToken,
    ) -> eyre::Result<Self> {
        let executor = executor::Builder {
            config: config.clone(),
            shutdown: shutdown_token.clone(),
            metrics,
        }
        .build()
        .wrap_err("failed constructing executor")?;

        Ok(Self {
            shutdown_token,
            config,
            executor: Some(tokio::spawn(executor.run_until_stopped())),
        })
    }

    /// Runs [`Inner`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<RestartOrShutdown> {
        info_span!("Conductor::run_until_stopped").in_scope(|| info!("conductor is running"));

        let exit_status = select! {
            biased;

            () = self.shutdown_token.cancelled() => {
                Ok(None)
            },

            res = self.executor.as_mut().expect("task must always be set at this point") => {
                // XXX: must Option::take the JoinHandle to avoid polling it in the shutdown logic.
                self.executor.take();
                match res {
                    Ok(Ok(state)) => Ok(state),
                    Ok(Err(err)) => Err(err.wrap_err("executor exited with error")),
                    Err(err) => Err(Report::new(err).wrap_err("executor panicked")),
                }
            }
        };

        self.restart_or_shutdown(exit_status).await
    }

    /// Shuts down all tasks.
    ///
    /// Waits 25 seconds for all tasks to shut down before aborting them. 25 seconds
    /// because kubernetes issues SIGKILL 30 seconds after SIGTERM, giving 5 seconds
    /// to abort the remaining tasks.
    #[instrument(skip_all, err, ret(Display))]
    async fn restart_or_shutdown(
        mut self,
        exit_status: eyre::Result<Option<crate::executor::State>>,
    ) -> eyre::Result<RestartOrShutdown> {
        let restart_or_shutdown = 'decide_restart: {
            if self.shutdown_token.is_cancelled() {
                break 'decide_restart Ok(RestartOrShutdown::Shutdown);
            }

            match exit_status {
                Ok(None) => Err(eyre!(
                    "executor exited with a success value but without rollup status even though \
                     it was not explicitly cancelled; this shouldn't happen"
                )),
                Ok(Some(status)) => should_restart_or_shutdown(&self.config, &status),
                Err(error) => {
                    error!(%error, "executor failed; checking error chain if conductor should be restarted");
                    if should_restart_despite_error(&error) {
                        Ok(RestartOrShutdown::Restart)
                    } else {
                        Err(error)
                    }
                }
            }
        };
        self.shutdown_token.cancel();

        if let Some(mut executor) = self.executor.take() {
            let wait_until_timeout = Duration::from_secs(25);
            if timeout(wait_until_timeout, &mut executor).await.is_err() {
                warn!(
                    "waited `{}` for executor start to respond to shutdown signal; aborting",
                    humantime::format_duration(wait_until_timeout)
                );
                executor.abort();
            } else {
                info!("executor shut down regularly");
            }
        }

        restart_or_shutdown
    }
}

#[instrument(skip_all)]
fn should_restart_despite_error(err: &eyre::Report) -> bool {
    let mut current = Some(err.as_ref() as &dyn std::error::Error);
    while let Some(err) = current {
        if let Some(status) = err.downcast_ref::<tonic::Status>() {
            if status.code() == tonic::Code::PermissionDenied {
                return true;
            }
        }
        current = err.source();
    }
    false
}

fn should_restart_or_shutdown(
    config: &Config,
    status: &crate::executor::State,
) -> eyre::Result<RestartOrShutdown> {
    let Some(rollup_stop_block_number) = status.rollup_stop_block_number() else {
        return Err(eyre!(
            "executor exited with a success value even though it was not configured to run with a \
             stop height and even though it received no shutdown signal; this should not happen"
        ));
    };

    match config.execution_commit_level {
        crate::config::CommitLevel::FirmOnly | crate::config::CommitLevel::SoftAndFirm => {
            if status.has_firm_number_reached_stop_height() {
                let restart_or_shutdown = if status.halt_at_stop_height() {
                    RestartOrShutdown::Shutdown
                } else {
                    RestartOrShutdown::Restart
                };
                Ok(restart_or_shutdown)
            } else {
                Err(eyre!(
                    "executor exited with a success value, but the stop height was not reached
                    (execution kind: `{}`, firm rollup block number: `{}`, mapped to sequencer \
                     height: `{}`, rollup start height: `{}`, sequencer start height: `{}`, \
                     sequencer stop height: `{}`)",
                    config.execution_commit_level,
                    status.firm_number(),
                    status.firm_block_number_as_sequencer_height(),
                    status.rollup_start_block_number(),
                    status.sequencer_start_height(),
                    rollup_stop_block_number,
                ))
            }
        }
        crate::config::CommitLevel::SoftOnly => {
            if status.has_soft_number_reached_stop_height() {
                let restart_or_shutdown = if status.halt_at_stop_height() {
                    RestartOrShutdown::Shutdown
                } else {
                    RestartOrShutdown::Restart
                };
                Ok(restart_or_shutdown)
            } else {
                Err(eyre!(
                    "executor exited with a success value, but the stop height was not reached
                    (execution kind: `{}`, soft rollup block number: `{}`, mapped to sequencer \
                     height: `{}`, rollup start height: `{}`, sequencer start height: `{}`, \
                     sequencer stop height: `{}`)",
                    config.execution_commit_level,
                    status.soft_number(),
                    status.soft_block_number_as_sequencer_height(),
                    status.rollup_start_block_number(),
                    status.sequencer_start_height(),
                    rollup_stop_block_number,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use astria_core::{
        generated::astria::execution::v1::{
            Block,
            CommitmentState,
            GenesisInfo,
        },
        primitive::v1::RollupId,
        Protobuf as _,
    };
    use astria_eyre::eyre::WrapErr as _;
    use pbjson_types::Timestamp;

    use super::{
        executor::State,
        RestartOrShutdown,
    };
    use crate::{
        config::CommitLevel,
        Config,
    };

    fn make_config() -> crate::Config {
        crate::Config {
            celestia_block_time_ms: 0,
            celestia_node_http_url: String::new(),
            no_celestia_auth: false,
            celestia_bearer_token: String::new(),
            sequencer_grpc_url: String::new(),
            sequencer_cometbft_url: String::new(),
            sequencer_block_time_ms: 0,
            sequencer_requests_per_second: 0,
            execution_rpc_url: String::new(),
            log: String::new(),
            execution_commit_level: CommitLevel::SoftAndFirm,
            force_stdout: false,
            no_otel: false,
            no_metrics: false,
            metrics_http_listener_addr: String::new(),
            pretty_print: false,
        }
    }

    fn make_commitment_state() -> CommitmentState {
        let firm = astria_core::generated::astria::execution::v1::Block {
            number: 1,
            hash: vec![42u8; 32].into(),
            parent_block_hash: vec![41u8; 32].into(),
            timestamp: Some(pbjson_types::Timestamp {
                seconds: 123_456,
                nanos: 789,
            }),
        };
        let soft = astria_core::generated::astria::execution::v1::Block {
            number: 2,
            hash: vec![43u8; 32].into(),
            parent_block_hash: vec![42u8; 32].into(),
            timestamp: Some(pbjson_types::Timestamp {
                seconds: 123_456,
                nanos: 789,
            }),
        };

        CommitmentState {
            soft: Some(soft),
            firm: Some(firm),
            base_celestia_height: 1,
        }
    }

    fn make_genesis_info() -> GenesisInfo {
        let rollup_id = RollupId::new([24; 32]);
        GenesisInfo {
            rollup_id: Some(rollup_id.to_raw()),
            sequencer_start_height: 10,
            celestia_block_variance: 0,
            rollup_start_block_number: 0,
            rollup_stop_block_number: 90,
            sequencer_chain_id: "test-sequencer-0".to_string(),
            celestia_chain_id: "test-celestia-0".to_string(),
            halt_at_stop_height: false,
        }
    }

    fn make_rollup_state(genesis_info: GenesisInfo, commitment_state: CommitmentState) -> State {
        let genesis_info =
            astria_core::execution::v1::GenesisInfo::try_from_raw(genesis_info).unwrap();
        let commitment_state =
            astria_core::execution::v1::CommitmentState::try_from_raw(commitment_state).unwrap();
        State::try_from_genesis_info_and_commitment_state(genesis_info, commitment_state).unwrap()
    }

    #[test]
    fn should_restart_despite_error() {
        let tonic_error: Result<&str, tonic::Status> =
            Err(tonic::Status::new(tonic::Code::PermissionDenied, "error"));
        let err = tonic_error.wrap_err("wrapper_1");
        let err = err.wrap_err("wrapper_2");
        let err = err.wrap_err("wrapper_3");
        assert!(super::should_restart_despite_error(&err.unwrap_err()));
    }

    #[track_caller]
    fn assert_restart_or_shutdown(
        config: &Config,
        state: &State,
        restart_or_shutdown: &RestartOrShutdown,
    ) {
        assert_eq!(
            &super::should_restart_or_shutdown(config, state).unwrap(),
            restart_or_shutdown,
        );
    }

    #[test]
    fn restart_or_shutdown_on_firm_height_reached() {
        assert_restart_or_shutdown(
            &Config {
                execution_commit_level: CommitLevel::SoftAndFirm,
                ..make_config()
            },
            &make_rollup_state(
                GenesisInfo {
                    sequencer_start_height: 10,
                    rollup_start_block_number: 10,
                    rollup_stop_block_number: 99,
                    halt_at_stop_height: false,
                    ..make_genesis_info()
                },
                CommitmentState {
                    firm: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    soft: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    ..make_commitment_state()
                },
            ),
            &RestartOrShutdown::Restart,
        );

        assert_restart_or_shutdown(
            &Config {
                execution_commit_level: CommitLevel::SoftAndFirm,
                ..make_config()
            },
            &make_rollup_state(
                GenesisInfo {
                    sequencer_start_height: 10,
                    rollup_start_block_number: 10,
                    rollup_stop_block_number: 99,
                    halt_at_stop_height: true,
                    ..make_genesis_info()
                },
                CommitmentState {
                    firm: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    soft: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    ..make_commitment_state()
                },
            ),
            &RestartOrShutdown::Shutdown,
        );
    }

    #[test]
    fn restart_or_shutdown_on_soft_height_reached() {
        assert_restart_or_shutdown(
            &Config {
                execution_commit_level: CommitLevel::SoftOnly,
                ..make_config()
            },
            &make_rollup_state(
                GenesisInfo {
                    sequencer_start_height: 10,
                    rollup_start_block_number: 10,
                    rollup_stop_block_number: 99,
                    halt_at_stop_height: false,
                    ..make_genesis_info()
                },
                CommitmentState {
                    firm: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    soft: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    ..make_commitment_state()
                },
            ),
            &RestartOrShutdown::Restart,
        );

        assert_restart_or_shutdown(
            &Config {
                execution_commit_level: CommitLevel::SoftOnly,
                ..make_config()
            },
            &make_rollup_state(
                GenesisInfo {
                    sequencer_start_height: 10,
                    rollup_start_block_number: 10,
                    rollup_stop_block_number: 99,
                    halt_at_stop_height: true,
                    ..make_genesis_info()
                },
                CommitmentState {
                    firm: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    soft: Some(Block {
                        number: 99,
                        hash: vec![0u8; 32].into(),
                        parent_block_hash: vec![].into(),
                        timestamp: Some(Timestamp::default()),
                    }),
                    ..make_commitment_state()
                },
            ),
            &RestartOrShutdown::Shutdown,
        );
    }
}
