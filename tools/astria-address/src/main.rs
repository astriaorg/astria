use astria_core_address as address;
use astria_core_address::{
    Address,
    ADDRESS_LENGTH,
};

const DEFAULT_PREFIX: &str = "astria";
const HELP: &str = const_format::formatcp!(
    r"Astria Address Tool

Utility to construct astria addresses given an address prefix
and {ADDRESS_LENGTH} hex-encoded bytes.

USAGE:
  astria-address [OPTIONS] [INPUT]

FLAGS:
  -h, --help            Prints help information
  -c, --compat          Constructs a compat address (primarily IBC communication
                        with chains that only support bech32 non-m addresses)

OPTIONS:
  -p, --prefix STRING   Sets the prefix of the address (default: {DEFAULT_PREFIX})

ARGS:
  <INPUT>               The {ADDRESS_LENGTH} bytes in hex-format
"
);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    args.run()?;
    Ok(())
}

#[derive(Debug)]
struct Args {
    compat: bool,
    prefix: Option<String>,
    input: [u8; ADDRESS_LENGTH],
}

impl Args {
    fn parse() -> Result<Self, pico_args::Error> {
        let mut pargs = pico_args::Arguments::from_env();

        // XXX: little hack to move the args out of pargs and back in:
        let no_args = {
            let raw_args = pargs.finish();
            let no_args = raw_args.is_empty();
            pargs = pico_args::Arguments::from_vec(raw_args);
            no_args
        };
        if pargs.contains(["-h", "--help"]) || no_args {
            print!("{}", HELP);
            std::process::exit(0);
        }

        let args = Self {
            compat: pargs.contains(["-c", "--compat"]),
            prefix: pargs.opt_value_from_str(["-p", "--prefix"])?,
            input: pargs.free_from_fn(|input| const_hex::decode_to_array(input))?,
        };

        // It's up to the caller what to do with the remaining arguments.
        let remaining = pargs.finish();
        if !remaining.is_empty() {
            return Err(pico_args::Error::ArgumentParsingFailed {
                cause: format!("unknown arguments: {remaining:?}"),
            });
        }

        Ok(args)
    }

    fn run(self) -> Result<(), address::Error> {
        use astria_core_address::{
            Bech32,
            Bech32m,
        };
        let prefix = self.prefix.as_deref().unwrap_or(DEFAULT_PREFIX);
        if self.compat {
            let address = Address::<Bech32>::builder()
                .array(self.input)
                .prefix(prefix)
                .try_build()?;
            println!("{address}");
        } else {
            let address = Address::<Bech32m>::builder()
                .array(self.input)
                .prefix(prefix)
                .try_build()?;
            println!("{address}");
        }
        Ok(())
    }
}
