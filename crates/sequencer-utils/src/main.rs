use astria_sequencer_utils::{
    config::Config,
    genesis_parser::GenesisParser,
};

fn main() {
    let config = Config::get();

    println!("running genesis parser");
    GenesisParser::propagate_app_state(config).expect("failed to propagate data");
    println!("genesis parsing complete");
}
