# Astria Address Tool

Construct Astria addresses given a 20 hex encoded bytes (usually obtained from
a private/public ed25519 keypair).

This tool is intended for developers and operators and usually not needed when
interacting with the Astria network.

## Building and usage

```console
# From inside the `astria-address` crate
$ cargo build --release
$ target/release/astria-address --help
Astria Address Tool

Utility to construct astria addresses given an address prefix
and 20 hex-encoded bytes.

USAGE:
  astria-address [OPTIONS] [INPUT]

FLAGS:
  -h, --help            Prints help information
  -c, --compat          Constructs a compat address (primarily IBC communication
                        with chains that only support bech32 non-m addresses)

OPTIONS:
  -p, --prefix STRING   Sets the prefix of the address (default: astria)

ARGS:
  <INPUT>               The 20 bytes in hex-format
```
