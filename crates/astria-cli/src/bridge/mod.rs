mod collect;
mod submit;

use std::{
    collections::BTreeMap,
    path::{
        Path,
        PathBuf,
    },
};

use astria_core::protocol::transaction::v1::Action;
use clap::Subcommand;
use color_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};
use tracing::instrument;

/// Interact with a Sequencer node
#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::CollectWithdrawals(args) => args.run().await,
            SubCommand::SubmitWithdrawals(args) => args.run().await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Collect withdrawals actions
    CollectWithdrawals(collect::Command),
    /// Submit collected withdrawal actions
    SubmitWithdrawals(submit::Command),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
struct ActionsByRollupHeight(BTreeMap<u64, Vec<Action>>);

impl ActionsByRollupHeight {
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    fn into_inner(self) -> BTreeMap<u64, Vec<Action>> {
        self.0
    }

    #[instrument(skip_all, err)]
    fn insert(&mut self, rollup_height: u64, actions: Vec<Action>) -> eyre::Result<()> {
        ensure!(
            self.0.insert(rollup_height, actions).is_none(),
            "already collected actions for block at rollup height `{rollup_height}`; no 2 blocks \
             with the same height should have been seen",
        );
        Ok(())
    }

    #[instrument(skip_all, fields(target = %output.path.display()), err)]
    fn write_to_output(self, output: Output) -> eyre::Result<()> {
        let writer = std::io::BufWriter::new(output.handle);
        serde_json::to_writer(writer, &self.0).wrap_err("failed writing actions to file")
    }
}

#[derive(Debug)]
struct Output {
    handle: std::fs::File,
    path: PathBuf,
}

#[instrument(skip(target), fields(target = %target.as_ref().display()), err)]
fn open_output<P: AsRef<Path>>(target: P, overwrite: bool) -> eyre::Result<Output> {
    let handle = if overwrite {
        let mut options = std::fs::File::options();
        options.write(true).create(true).truncate(true);
        options
    } else {
        let mut options = std::fs::File::options();
        options.write(true).create_new(true);
        options
    }
    .open(&target)
    .wrap_err("failed to open specified file for writing")?;
    Ok(Output {
        handle,
        path: target.as_ref().to_path_buf(),
    })
}
