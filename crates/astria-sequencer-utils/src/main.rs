use astria_eyre::eyre::Result;
use astria_sequencer_utils::{
    activation_point_estimator,
    blob_parser,
    cli::{
        self,
        Command,
    },
    genesis_example,
    genesis_parser,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    astria_eyre::install()
        .expect("the astria eyre install hook must be called before eyre reports are constructed");
    match cli::get() {
        Command::CopyGenesisState(args) => genesis_parser::run(args),
        Command::GenerateGenesisState(args) => genesis_example::run(&args),
        Command::ParseBlob(args) => blob_parser::run(args),
        Command::EstimateActivationPoint(args) => activation_point_estimator::run(args).await,
    }
}
