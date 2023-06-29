use astria_sequencer_utils::{
    config::{
        Command,
        Config,
    },
    genesis_parser::GenesisParser,
};

fn main() {
    let config = Config::get();

    match config.cmd {
        Command::GenesisParser(args) => {
            println!("running genesis parser");
            GenesisParser::propigate_data(args).expect("failed to propagate data");
            println!("genesis parsing complete");
        }
    }
}
