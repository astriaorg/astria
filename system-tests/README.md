# System Tests

The files in this folder are system tests intended to be run locally and as part
of CI/CD.

## Prerequisites

[Python 3.8](https://www.python.org/downloads) or greater should be installed.

The following packages are required:

```shell
pip3 install argparse requests google google-api-core grpcio python-on-whales
```

## Running the Upgrade Test

For now, it requires an environment to be set up via `just` commands prior to
execution.

To run the sequencer upgrade test where the binaries used to execute the upgrade
are as per the `latest` tag:

```shell
# If previously run, clean up
just clean
sudo rm -r /tmp/astria

# Set up the test environment
just deploy cluster
just deploy upgrade-test

# Run the test
just run-upgrade-test
```

This invokes the `sequencer_upgrade_test.py` script with the args
`--image-tag latest` and `--upgrade-name upgrade1`.

To run the upgrade test using local builds:

```shell
# If previously run, clean up
just clean
sudo rm -r /tmp/astria

# Set up the test environment
just deploy cluster
cargo check
just docker-build-and-load astria-sequencer-relayer
just docker-build-and-load astria-sequencer
just docker-build-and-load astria-composer
just docker-build-and-load astria-conductor
just docker-build-and-load astria-cli
just docker-build-and-load astria-bridge-withdrawer
just deploy upgrade-test

# Run the test
just run-upgrade-test local
```

This invokes the `sequencer_upgrade_test.py` script with the args
`--image-tag local` and `--upgrade-name upgrade1`.
