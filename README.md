# Astria

Astria replaces centralized sequencers, allowing many rollups to share a single
decentralized network of sequencers that’s simple and permissionless to join.
This shared sequencer network provides out-of-the-box censorship resistance,
fast block confirmations, and atomic cross-rollup composability – all while
retaining each rollup’s sovereignty.

This repository contains the custom Astria components that make up the Astria
network. Other components of the Astria network can be found in the
[astriaorg](https://github.com/astriaorg) organization.

To run locally, we utilize a dev-cluster which can be found at
[astriaorg/dev-cluster](https://github.com/astriaorg/dev-cluster).

To learn more about Astria, please visit [astria.org](https://astria.org).

## Components

* [conductor](https://github.com/astriaorg/astria/tree/main/crates/astria-conductor):
  conducts blocks from the data availability layer to the execution layer.
* [proto](https://github.com/astriaorg/astria/tree/main/crates/astria-proto):
  relevant protobufs for Astria types.
* [sequencer](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer):
  ABCI application that defines the sequencer state transition logic.
* [sequencer-relayer](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer-relayer):
  relays blocks from the sequencer chain to the data availability layer.

## Build

To build the relevant Astria binaries, you only need
[Rust](https://www.rust-lang.org/tools/install) installed.

Then:

```sh
git clone https://github.com/astriaorg/astria.git
cd astria
cargo build --release
```

### Docker build

To build a docker image locally you will first need docker installed. With
docker installed you can use the following just command:

```sh
# Full command:
just docker-build <CRATE> <TAG=local>
#
# Replace CRATE with what the target binary is ie `astria-sequencer`
# TAG defaults to `local` but can be changed.

# this command will build a local image tagged as 'astria-sequencer:local' 
just docker-build astria-sequencer

# this command will build a local image tagged as 'astria-sequencer:debug' 
just docker-build astria-sequencer debug
```

## Running locally

The entire stack consists of many different binaries. It's recommended to use the
setup located in
[astriaorg/dev-cluster](https://github.com/astriaorg/dev-cluster).

## Testing

To run unit tests:

```sh
cargo test
```

Note that the `astria-proto` generates its code by running tests (and verifying
that nothing changed). In order for its tests to run you also need
[Buf](https://buf.build/docs/installation/) installed.

## Formatting

This project uses [rustfmt](https://github.com/rust-lang/rustfmt) to format rust
sources, [taplo](https://github.com/tamasfe/taplo) to format toml files, and
[markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2) for
markdown.

### Rust

```sh
# Install rustfmt
rustup +nightly-2024-02-07 component add rustfmt
# Run rustfmt
just fmt rust
```

### Toml

```sh
# Install for macOS
brew install taplo
# Install for Arch Linux
sudo pacman -S taplo

# Run
just fmt toml
```

### Markdown

```sh
# Install for macOS w/ homebrew
brew install markdownlint-cli2
# Install for Arch Linux
sudo pacman -S markdownlint-cli2
# Install with NPM
npm install markdownlint-cli2 --global

# Run
just lint md

# Run with docker
docker run -v $PWD:/workdir davidanson/markdownlint-cli2:v0.8.1 "**/*.md" "#.github"
```

## Contributing

Pull requests should be created against the `main` branch. In general, we follow
the "fork-and-pull" Git workflow.

1. Fork the repo on GitHub
2. Clone the project to your own machine
3. Commit changes to your own branch
4. Push your work back up to your fork
5. Submit a Pull request so that we can review your changes

NOTE: Be sure to merge the latest from upstream before making a pull request!

## Issues

If you encounter any issues while using this project or have any questions,
please open an issue in this repository
[here](https://github.com/astriaorg/astria/issues).
