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
            SubCommand::Recover(recover) => recover.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    /// Generates a new Ed25519 keypair.
    Create(Create),
    /// Recovers an Ed25519 keypair from a mnemonic.
    Recover(Recover),
    /// Queries the Sequencer for the balances of an account.
    Balance(Balance),
    /// Queries the Sequencer for the current nonce of an account.
    Nonce(Nonce),
}

#[derive(Debug, clap::Args)]
struct Recover {
    /// The address prefix
    #[arg(long, default_value = "astria")]
    prefix: String,
    /// The recovery mnemonic phrase, must be wrapped in quotes.
    #[arg(long)]
    mnemonic: String,
}

impl Recover {
    fn run(self) -> eyre::Result<()> {
        let bip39_mnemonic = bip39::Mnemonic::from_phrase(&self.mnemonic, bip39::Language::English)
            .wrap_err("failed to recover mnemonic from phrase")?;
        let seed = bip39::Seed::new(&bip39_mnemonic, "");
        let seed_bytes: [u8; 32] = seed.as_bytes()[0..32]
            .try_into()
            .wrap_err("failed to convert seed to 32 bytes")?;

        let signing_key = SigningKey::from(seed_bytes);

        let pretty_signing_key = hex::encode(signing_key.as_bytes());
        let pretty_verifying_key = hex::encode(signing_key.verification_key().as_bytes());

        let pretty_address: Address = Address::builder()
            .array(signing_key.address_bytes())
            .prefix(&self.prefix)
            .try_build()
            .wrap_err("failed constructing a valid from address from the provided prefix")?;

        // TODO: don't print private keys to CLI, prefer writing to file:
        // https://github.com/astriaorg/astria/issues/594
        println!(
            "Recover Sequencer Account\n\nPrivate Key: {pretty_signing_key}\nPublic Key:  \
             {pretty_verifying_key}\nAddress:     {pretty_address}"
        );

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
struct Create {
    /// The address prefix
    #[arg(long, default_value = "astria")]
    prefix: String,
}

impl Create {
    fn run(self) -> eyre::Result<()> {
        let bip39_mnemonic =
            bip39::Mnemonic::new(bip39::MnemonicType::Words12, bip39::Language::English);

        let seed = bip39::Seed::new(&bip39_mnemonic, "");
        let seed_bytes: [u8; 32] = seed.as_bytes()[0..32]
            .try_into()
            .wrap_err("failed to convert seed to 32 bytes")?;

        let signing_key = SigningKey::from(seed_bytes);

        let pretty_signing_key = hex::encode(signing_key.as_bytes());
        let pretty_verifying_key = hex::encode(signing_key.verification_key().as_bytes());

        let pretty_address: Address = Address::builder()
            .array(signing_key.address_bytes())
            .prefix(&self.prefix)
            .try_build()
            .wrap_err("failed constructing a valid from address from the provided prefix")?;

        // TODO: don't print private keys to CLI, prefer writing to file:
        // https://github.com/astriaorg/astria/issues/594
        println!(
            "Create Sequencer Account\n\nMnemonic:    {bip39_mnemonic}\nPrivate Key: \
             {pretty_signing_key}\nPublic Key:  {pretty_verifying_key}\nAddress:     \
             {pretty_address}"
        );

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

        println!("Balances for address: {}", args.address);
        for balance in res.balances {
            println!("    {} {}", balance.balance, balance.denom);
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

        println!("Nonce for address {}", args.address);
        println!("    {} at height {}", res.nonce, res.height);

        Ok(())
    }
}

#[derive(clap::Args, Debug)]
struct ArgsInner {
    /// The url of the Sequencer node
    #[arg(long, env = "SEQUENCER_URL")]
    sequencer_url: String,
    /// The address of the Sequencer account
    address: Address,
}
