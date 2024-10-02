use astria_core::{
    crypto::SigningKey,
    primitive::v1::Address,
};
use astria_sequencer_client::{
    HttpClient,
    SequencerClientExt as _,
};
use clap::Subcommand;
use color_eyre::eyre::{
    self,
    WrapErr as _,
};
use rand::rngs::OsRng;

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            Command::Create(create) => create.run(),
            Command::Balance(balance) => balance.run().await,
            Command::Nonce(nonce) => nonce.run().await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generates a new ED25519 keypair.
    Create(CreateArgs),
    /// Queries the Sequencer for the balances of an account.
    Balance(BalanceArgs),
    /// Queries the Sequencer for the current nonce of an account.
    Nonce(NonceArgs),
}

#[derive(Debug, clap::Args)]
struct CreateArgs;

impl CreateArgs {
    #[expect(
        clippy::unused_self,
        clippy::unnecessary_wraps,
        reason = "for consistency with all the other commands"
    )]
    fn run(self) -> eyre::Result<()> {
        let signing_key = SigningKey::new(OsRng);
        let pretty_signing_key = hex::encode(signing_key.as_bytes());
        let pretty_verifying_key = hex::encode(signing_key.verification_key().as_bytes());
        let pretty_address = hex::encode(signing_key.address_bytes());
        println!("Create Sequencer Account");
        println!();
        // TODO: don't print private keys to CLI, prefer writing to file:
        // https://github.com/astriaorg/astria/issues/594
        println!("Private Key: {pretty_signing_key}");
        println!("Public Key:  {pretty_verifying_key}");
        println!("Address:     {pretty_address}");
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct BalanceArgs {
    #[command(flatten)]
    inner: ArgsInner,
}

impl BalanceArgs {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_latest_balance(args.address)
            .await
            .wrap_err("failed to get balance")?;

        println!("Balances for address: {}", args.address);
        for balance in res.balances {
            println!("    {} {}", balance.balance, balance.denom);
        }

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct NonceArgs {
    #[command(flatten)]
    inner: ArgsInner,
}

impl NonceArgs {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_latest_nonce(args.address)
            .await
            .wrap_err("failed to get nonce")?;

        println!("Nonce for address {}", args.address);
        println!("    {} at height {}", res.nonce, res.height);

        Ok(())
    }
}

#[derive(clap::Args, Debug)]
struct ArgsInner {
    /// The url of the Sequencer node
    #[arg(
        long,
        env = "SEQUENCER_URL",
        default_value = crate::DEFAULT_SEQUENCER_RPC
    )]
    sequencer_url: String,
    /// The address of the Sequencer account
    address: Address,
}
