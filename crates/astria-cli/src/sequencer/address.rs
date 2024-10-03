use astria_core::primitive::v1::{
    Address,
    ADDRESS_LEN,
};
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
    pub(super) fn run(self) -> eyre::Result<()> {
        let SubCommand::Bech32m(bech32m) = self.command;
        bech32m.run()
    }
}

#[derive(Debug, clap::Subcommand)]
enum SubCommand {
    /// Returns a bech32m sequencer address given a prefix and hex-encoded byte slice
    Bech32m(Bech32m),
}

#[derive(Debug, clap::Args)]
struct Bech32m {
    /// The hex formatted byte part of the bech32m address
    #[arg(long)]
    bytes: String,
    /// The human readable prefix (Hrp) of the bech32m adress
    #[arg(long, default_value = "astria")]
    prefix: String,
}

impl Bech32m {
    fn run(self) -> eyre::Result<()> {
        use hex::FromHex as _;
        let bytes = <[u8; ADDRESS_LEN]>::from_hex(&self.bytes)
            .wrap_err("failed decoding provided hex bytes")?;
        let address = Address::<astria_core::primitive::v1::Bech32m>::builder()
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
