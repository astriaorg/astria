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
use tracing::info;

#[derive(Debug, clap::Args)]
pub(super) struct Command {
    #[command(subcommand)]
    command: SubCommand,
}

impl Command {
    pub(super) async fn run(self) -> eyre::Result<()> {
        match self.command {
            SubCommand::Create(create) => create.run(),
            SubCommand::Balance(balance) => balance.run().await,
            SubCommand::Nonce(nonce) => nonce.run().await,
        }
    }
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Generates a new ED25519 keypair.
    Create(Create),
    /// Queries the Sequencer for the balances of an account.
    Balance(Balance),
    /// Queries the Sequencer for the current nonce of an account.
    Nonce(Nonce),
}

#[derive(Debug, clap::Args)]
struct Create {
    /// The address prefix
    #[arg(long, default_value = "astria")]
    prefix: String,
}

impl Create {
    fn run(self) -> eyre::Result<()> {
        let signing_key = SigningKey::new(OsRng);
        let pretty_signing_key = hex::encode(signing_key.as_bytes());
        let pretty_verifying_key = hex::encode(signing_key.verification_key().as_bytes());
        let pretty_address = SigningKey::try_address(&signing_key, &self.prefix)?;
        // TODO: don't print private keys to CLI, prefer writing to file:
        // https://github.com/astriaorg/astria/issues/594
        info!(private_key = %pretty_signing_key, public_key = %pretty_verifying_key, address = %pretty_address, "sequencer account created");
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Balance {
    #[command(flatten)]
    inner: ArgsInner,
}

impl Balance {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_latest_balance(args.address)
            .await
            .wrap_err("failed to get balance")?;

        info!(
            address = %args.address,
            "Balances for address"
        );
        for balance in res.balances {
            info!(
                balance = %balance.balance,
                denom = %balance.denom,
                "Balance details"
            );
        }

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Nonce {
    #[command(flatten)]
    inner: ArgsInner,
}

impl Nonce {
    async fn run(self) -> eyre::Result<()> {
        let args = self.inner;
        let sequencer_client = HttpClient::new(args.sequencer_url.as_str())
            .wrap_err("failed constructing http sequencer client")?;

        let res = sequencer_client
            .get_latest_nonce(args.address)
            .await
            .wrap_err("failed to get nonce")?;

        info!(address = %args.address, nonce = res.nonce, "Nonce retrieved");

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
