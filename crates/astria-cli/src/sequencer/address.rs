use astria_core::primitive::v1::{
    Address,
    Bech32m,
    ADDRESS_LEN,
};
use color_eyre::eyre::{
    self,
    WrapErr as _,
};

#[derive(Debug, clap::Args)]
pub(super) struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    pub(super) fn run(self) -> eyre::Result<()> {
        let Command::Bech32m(bech32m) = self.command;
        bech32m.run()
    }
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Returns a bech32m sequencer address given a prefix and hex-encoded byte slice
    Bech32m(Bech32mArgs),
}

#[derive(Debug, clap::Args)]
struct Bech32mArgs {
    /// The hex formatted byte part of the bech32m address
    #[arg(long)]
    bytes: String,
    /// The human readable prefix (Hrp) of the bech32m adress
    #[arg(long, default_value = "astria")]
    prefix: String,
}

impl Bech32mArgs {
    fn run(self) -> eyre::Result<()> {
        use hex::FromHex as _;
        let bytes = <[u8; ADDRESS_LEN]>::from_hex(&self.bytes)
            .wrap_err("failed decoding provided hex bytes")?;
        let address = Address::<Bech32m>::builder()
            .array(bytes)
            .prefix(&self.prefix)
            .try_build()
            .wrap_err(
                "failed constructing a valid bech32m address from the provided hex bytes and \
                 prefix",
            )?;
        println!("{address}");
        Ok(())
    }
}
