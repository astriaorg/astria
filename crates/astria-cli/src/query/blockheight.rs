use astria_cli::config::{
    SequencerNetworksConfig,
    DEFAULT_SEQUENCER_URL,
};
use astria_sequencer_client::{
    Client,
    HttpClient,
};
use clap::{
    builder::Str,
    Arg,
    ArgAction,
    ArgMatches,
    Command,
};
use color_eyre::{
    eyre,
    eyre::Context,
};
use home::home_dir;

pub(crate) fn command() -> Command {
    let mut path = home_dir().expect("Could not determine the home directory.");
    path.push(".astria");
    path.push("sequencer-networks-config.toml");

    Command::new("blockheight")
        .about("Get the current blockheight from the sequencer")
        .arg(
            // flag input
            Arg::new("sequencer-url")
                .long("sequencer-url")
                .help("URL of the sequencer")
                .action(ArgAction::Set)
                .default_value(DEFAULT_SEQUENCER_URL)
                .env("SEQUENCER_URL"),
        )
        .arg(
            // count bool flag
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help("Print debug information verbosely"),
        )
        .arg(
            // flag input
            Arg::new("network")
                .long("network")
                .action(ArgAction::Set)
                .help("Select a network config preset"),
        )
        .arg(
            // flag input
            Arg::new("config")
                .long("config")
                .action(ArgAction::Set)
                .help("Specify a network config file")
                .default_value(Str::from(path.display().to_string())),
        )
}

pub(crate) async fn run(matches: &ArgMatches) -> eyre::Result<()> {
    // load and parse the config file
    let config: SequencerNetworksConfig = {
        let config_path = matches.get_one::<String>("config");
        if let Some(path) = config_path {
            SequencerNetworksConfig::load(path).expect("Could not load config file")
        } else {
            let mut path = home_dir().expect("Could not determine the home directory.");
            path.push(".astria");
            path.push("sequencer-networks-config.toml");
            SequencerNetworksConfig::load(path).expect("Could not load config file")
        }
    };

    // get verbosity cound (currently unused)
    let verbose = matches.get_count("verbose");
    println!("verbose count: {:?}", verbose);

    // get the chosen network config
    let network = matches.get_one::<String>("network");
    println!("network: {:?}", network);

    // get the correct sequencer_url based on all inputs
    let sequenecer_url = if let Some(chosen_network) = network {
        let net_config = config
            .get_network(chosen_network)
            .expect("network not found");
        net_config.sequencer_url.clone()
    } else {
        let seq_url = matches.get_one::<String>("sequencer-url");
        seq_url.unwrap().clone()
    };

    // submit the query to the sequencer
    let sequencer_client = HttpClient::new(sequenecer_url.as_str())
        .wrap_err("failed constructing http sequencer client")?;

    let res = sequencer_client
        .latest_block()
        .await
        .wrap_err("failed to get cometbft block")?;

    println!("Block Height:");
    println!("    {}", res.block.header.height);

    Ok(())
}
