# Multi-node testnet

To run a multi-node local testnet, you will need to have cometbft installed.

## Install cometbft

Ensure `~/go` is in your `PATH`, or `GOPATH` is set to some other place in your
`PATH`.

```sh
git clone https://github.com/cometbft/cometbft
cd cometbft
git checkout origin/v0.38.6
export GOPATH=~/go
make install
```

## Build the sequencer application

In astria-sequencer/:

```sh
cargo build
```

## Initialize and run the testnet

This will create 3 cometbft validator configs and put the validator configs in
`./sequencer_testnet` by default. It will then start the respective sequencer
applications (one for each cometbft validator) and the cometbft validator nodes.

(This requires cometbft to be installed)

```sh
just run-testnet
```

To manually set the output directory or number of validators:

```sh
OUT_DIR=out_dir NUM_VALIDATORS=num_validators just run-testnet 
```

## Logs

For example, to see the cometbft logs for node0:

```sh
just cometbft-logs-testnet node0
```

If you do not provide a node name, by default `node0` is chosen

So the previous is equivalent to

```sh
just cometbft-logs-testnet
```

To see the sequencer app logs for node0:

```sh
just sequencer-logs-testnet node0
```

## Stopping the testnet

```sh
just stop-testnet
```

## Cleaning up the testnet

Simply stop the testnet and run

```sh
just clean-testnet sequencer_testnet
```

where `sequencer_testnet` is the `OUT_DIR` where the testnet files are.
