# System Tests

The files in this folder are system tests intended to be run locally and as part
of CI/CD.

## Prerequisites

[Python 3.8](https://www.python.org/downloads) or greater should be installed.

We recommend using [uv](https://github.com/astral-sh/uv) for package management,
which is a fast Python package installer and resolver written in Rust.

### Install uv

```shell
# Install uv using cargo
cargo install --git https://github.com/astral-sh/uv uv

# Or using the official installer
curl -LsSf https://astral.sh/uv/install.sh | sh

# Or on macOS with Homebrew
brew install uv
```

### Set up a virtual environment and install required packages

First, create a virtual environment using `uv`:

```shell
cd system-tests
uv venv
```

This creates a virtual environment in the `.venv` directory. Activate it with:

```shell
source .venv/bin/activate
```

Then install the required packages:

```shell
uv pip install -r requirements.txt
```

Alternatively, you can create a virtual environment and install packages in one command:

```shell
uv venv && uv pip install -r requirements.txt
```

Or install packages directly (not recommended):

```shell
uv pip install argparse requests google google-api-core grpcio python-on-whales
```

## Running the Upgrade Test

For now, it requires an environment to be set up via `just` commands prior to
execution.

To run the sequencer upgrade test where the binaries used to execute the upgrade
are as per the `latest` tag:

```shell
# If previously run, clean up
just clean helm

# Set up the test environment
just deploy cluster # only needs to be run first time
just deploy upgrade-test

# Run the test
just run upgrade-test
```

This invokes the `sequencer_upgrade_test.py` script with the args
`--image-tag latest` and `--upgrade-name aspen`.

To run the upgrade test using local builds:

```shell
# If previously run, clean up
just clean helm

# Set up the test environment
just deploy cluster # only needs to be run first time
cargo check
just docker-build-and-load astria-sequencer-relayer
just docker-build-and-load astria-sequencer
just docker-build-and-load astria-composer
just docker-build-and-load astria-conductor
just docker-build-and-load astria-cli
just docker-build-and-load astria-bridge-withdrawer
just deploy upgrade-test

# Run the test
just run upgrade-test local
```

This invokes the `sequencer_upgrade_test.py` script with the args
`--image-tag local` and `--upgrade-name aspen`.
