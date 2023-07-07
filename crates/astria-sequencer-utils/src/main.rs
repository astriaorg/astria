use astria_sequencer_utils::{
    config::Config,
    genesis_parser::GenesisParser,
};

fn main() {
    let config = Config::get();

    println!("running genesis parser");
    GenesisParser::propagate_data(config).expect("failed to propagate data");
    println!("genesis parsing complete");
}
