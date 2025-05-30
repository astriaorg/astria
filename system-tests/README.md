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

## Running Tests

For now, it requires an environment to be set up via `just` commands prior to
execution.

To run a system test:

```shell
# If previously run, clean up
just clean helm

# Set up the test environment
just deploy cluster # only needs to be run first time
just deploy [TEST_NAME]

# Current Python tests:
#   - evm-restart-test
#   - multiple-relayer-test
#   - smoke-test
#   - upgrade-test

# Run the desired test
just run upgrade-test <IMAGE_TAG> # e.g. 'latest', 'local', 'pr-2000'
just run smoke-test <FLAGS> # e.g. --image-tag sequencer=latest
just run evm-restart-test <FLAGS>
just run multiple-relayer-test <FLAGS>
```

To run the upgrade test using local builds:

```shell
# If previously run, clean up
just clean helm

# Set up the test environment
just deploy cluster # only needs to be run first time
cargo check
just docker-build-and-load-all
just deploy [TEST_NAME]

# Run the desired test
just run upgrade-test local
just run smoke-test -i sequencer=local -i sequencer-relayer=local -i bridge-withdrawer=local\
    -i composer=local -i conductor=local -i cli=local
just run evm-restart-test -i sequencer=local -i sequencer-relayer=local -i bridge-withdrawer=local\
    -i composer=local -i conductor=local -i cli=local
```
