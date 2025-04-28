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
    Ok,
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
    /// Optional mnemonic to use for key generation.
    #[arg(long = "from-mnemonic", default_value = None)]
    mnemonic: Option<String>,
    /// Mnemonic length.
    #[arg(long, default_value = "24")]
    mnemonic_length: u8,
}

impl Create {
    fn run(self) -> eyre::Result<()> {
        let bip39_mnemonic = match self.mnemonic {
            Some(mnemonic) => {
                bip39::Mnemonic::validate(&mnemonic, bip39::Language::English)
                    .wrap_err("phrase verification failed")?;
                bip39::Mnemonic::from_phrase(&mnemonic, bip39::Language::English)
                    .wrap_err("failed to create mnemonic from phrase")?
            }
            None => {
                let mnemonic_type = match self.mnemonic_length {
                    12 => bip39::MnemonicType::Words12,
                    15 => bip39::MnemonicType::Words15,
                    18 => bip39::MnemonicType::Words18,
                    21 => bip39::MnemonicType::Words21,
                    24 => bip39::MnemonicType::Words24,
                    _ => return Err(eyre::eyre!("Invalid mnemonic length")),
                };
                bip39::Mnemonic::new(mnemonic_type, bip39::Language::English)
            }
        };

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
            .try_build()?;

        println!("Create Sequencer Account");
        println!();
        // TODO: don't print private keys to CLI, prefer writing to file:
        // https://github.com/astriaorg/astria/issues/594
        println!("Mnemonic:    {bip39_mnemonic}");
        println!("Private Key: {pretty_signing_key}");
        println!("Public Key:  {pretty_verifying_key}");
        println!("Address:     {pretty_address}");
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
