use std::time::Duration;

use astria_eyre::eyre::{
    self,
    bail,
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
    executor::{
        self,
    },
    state::State,
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

/// The business logic of Conductor.
pub(super) struct Inner {
    /// Token to signal to all tasks to shut down gracefully.
    shutdown_token: CancellationToken,

    /// Token to signal to the executor to shut down gracefully.
    executor_shutdown_token: CancellationToken,

    config: Config,

    executor: Option<JoinHandle<eyre::Result<Option<State>>>>,
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

        let executor_shutdown_token = shutdown_token.child_token();

        Ok(Self {
            shutdown_token,
            executor_shutdown_token,
            config,
            executor: Some(tokio::spawn(
                executor.run_until_stopped_or_stop_height_reached(),
            )),
        })
    }

    /// Runs [`Inner`] until it receives an exit signal.
    ///
    /// # Panics
    /// Panics if it could not install a signal handler.
    pub(super) async fn run_until_stopped(mut self) -> eyre::Result<RestartOrShutdown> {
        info_span!("Conductor::run_until_stopped").in_scope(|| info!("conductor is running"));

        let exit_reason = select! {
            biased;

            () = self.shutdown_token.cancelled() => {
                Ok(None)
            },

            res = self.executor.as_mut().expect("task must always be set at this point") => {
                // XXX: must Option::take the JoinHandle to avoid polling it in the shutdown logic.
                self.executor.take();
                match res {
                    Ok(Ok(state_opt)) => Ok(state_opt),
                    Ok(Err(err)) => Err(err.wrap_err("executor exited with error")),
                    Err(err) => Err(Report::new(err).wrap_err("executor panicked")),
                }
            }
        };

        self.restart_or_shutdown(exit_reason).await
    }

    /// Shuts down all tasks and returns a token indicating whether to restart or not.
    ///
    /// Waits 25 seconds for all tasks to shut down before aborting them. 25 seconds
    /// because kubernetes issues SIGKILL 30 seconds after SIGTERM, giving 5 seconds
    /// to abort the remaining tasks.
    #[instrument(skip_all, err, ret(Display))]
    async fn restart_or_shutdown(
        mut self,
        exit_result: eyre::Result<Option<State>>,
    ) -> eyre::Result<RestartOrShutdown> {
        self.executor_shutdown_token.cancel();
        let restart_or_shutdown = 'decide_restart: {
            if self.shutdown_token.is_cancelled() {
                break 'decide_restart Ok(RestartOrShutdown::Shutdown);
            }

            match exit_result {
                Ok(state_opt) => should_restart_or_shutdown(&self.config, state_opt.as_ref()),
                Err(error) => {
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
                    telemetry::display::format_duration(wait_until_timeout)
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
                warn!(%err, "executor failed; received `PermissionDenied` status, attempting new execution session");
                return true;
            }
        }
        current = err.source();
    }
    error!(%err, "executor failed; error does not warrant new execution session, shutting down");
    false
}

fn should_restart_or_shutdown(
    config: &Config,
    state_opt: Option<&State>,
) -> eyre::Result<RestartOrShutdown> {
    let Some(state) = state_opt else {
        return Ok(RestartOrShutdown::Shutdown);
    };

    let Some(rollup_stop_block_number) = state.rollup_end_block_number() else {
        bail!(
            "executor exited with a success value even though it was not configured to run with a \
             stop height and even though it received no shutdown signal; this should not happen"
        );
    };

    if config.execution_commit_level.is_with_firm() {
        if state.has_firm_number_reached_stop_height() {
            Ok(RestartOrShutdown::Restart)
        } else {
            Err(eyre!(
                "executor exited with a success value, but the stop height was not reached
                    (execution kind: `{}`, firm rollup block number: `{}`, mapped to sequencer \
                 height: `{}`, rollup start height: `{}`, sequencer start height: `{}`, sequencer \
                 stop height: `{}`)",
                config.execution_commit_level,
                state.firm_number(),
                state.firm_block_number_as_sequencer_height(),
                state.rollup_start_block_number(),
                state.sequencer_start_block_height(),
                rollup_stop_block_number,
            ))
        }
    } else if state.has_soft_number_reached_stop_height() {
        Ok(RestartOrShutdown::Restart)
    } else {
        Err(eyre!(
            "executor exited with a success value, but the stop height was not reached
                    (execution kind: `{}`, soft rollup block number: `{}`, mapped to sequencer \
             height: `{}`, rollup start height: `{}`, sequencer start height: `{}`, sequencer \
             stop height: `{}`)",
            config.execution_commit_level,
            state.soft_number(),
            state.soft_block_number_as_sequencer_height(),
            state.rollup_start_block_number(),
            state.sequencer_start_block_height(),
            rollup_stop_block_number,
        ))
    }
}

#[cfg(test)]
mod tests {
    use astria_core::generated::astria::execution::v2::{
        CommitmentState,
        ExecutedBlockMetadata,
        ExecutionSessionParameters,
    };
    use astria_eyre::eyre::WrapErr as _;
    use pbjson_types::Timestamp;

    use super::RestartOrShutdown;
    use crate::{
        config::CommitLevel,
        state::State,
        test_utils::{
            make_commitment_state,
            make_execution_session_parameters,
            make_rollup_state,
        },
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
        }
    }

    #[track_caller]
    fn should_restart_despite_error_test(code: tonic::Code) {
        let tonic_error: Result<&str, tonic::Status> = Err(tonic::Status::new(code, "error"));
        let err = tonic_error.wrap_err("wrapper_1");
        let err = err.wrap_err("wrapper_2");
        let err = err.wrap_err("wrapper_3");
        assert!(super::should_restart_despite_error(&err.unwrap_err()));
    }
    #[test]
    fn should_restart_despite_error() {
        should_restart_despite_error_test(tonic::Code::PermissionDenied);
    }

    #[track_caller]
    fn assert_restart_or_shutdown(
        config: &Config,
        state: &State,
        restart_or_shutdown: &RestartOrShutdown,
    ) {
        assert_eq!(
            &super::should_restart_or_shutdown(config, Some(state)).unwrap(),
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
                "test_execution_session".to_string(),
                ExecutionSessionParameters {
                    sequencer_start_block_height: 10,
                    rollup_start_block_number: 10,
                    rollup_end_block_number: 99,
                    ..make_execution_session_parameters()
                },
                CommitmentState {
                    firm_executed_block_metadata: Some(ExecutedBlockMetadata {
                        number: 99,
                        hash: hex::encode([0u8; 32]).to_string(),
                        parent_hash: String::new(),
                        timestamp: Some(Timestamp::default()),
                        sequencer_block_hash: String::new(),
                    }),
                    soft_executed_block_metadata: Some(ExecutedBlockMetadata {
                        number: 99,
                        hash: hex::encode([0u8; 32]).to_string(),
                        parent_hash: String::new(),
                        timestamp: Some(Timestamp::default()),
                        sequencer_block_hash: String::new(),
                    }),
                    ..make_commitment_state()
                },
            ),
            &RestartOrShutdown::Restart,
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
                "test_execution_session".to_string(),
                ExecutionSessionParameters {
                    sequencer_start_block_height: 10,
                    rollup_start_block_number: 10,
                    rollup_end_block_number: 99,
                    ..make_execution_session_parameters()
                },
                CommitmentState {
                    firm_executed_block_metadata: Some(ExecutedBlockMetadata {
                        number: 99,
                        hash: hex::encode([0u8; 32]).to_string(),
                        parent_hash: String::new(),
                        timestamp: Some(Timestamp::default()),
                        sequencer_block_hash: String::new(),
                    }),
                    soft_executed_block_metadata: Some(ExecutedBlockMetadata {
                        number: 99,
                        hash: hex::encode([0u8; 32]).to_string(),
                        parent_hash: String::new(),
                        timestamp: Some(Timestamp::default()),
                        sequencer_block_hash: String::new(),
                    }),
                    ..make_commitment_state()
                },
            ),
            &RestartOrShutdown::Restart,
        );
    }
}
