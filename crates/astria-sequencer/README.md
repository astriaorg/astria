# Astria-Sequencer

## Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient project
specific commands.

- rust 1.68
- gcc-12, gcc-12-libs
- go 1.18+

### Penumbra

Because `penumbra-storage:0.54.1` depends on `rocksdb:0.19.0` compilation on gcc
13 will not work.

On arch linux:

```bash
just build-penumbra
```

<https://github.com/rust-rocksdb/rust-rocksdb/issues/713>
<https://github.com/facebook/rocksdb/pull/11118>


## Running the Sequencer

### Configuration

Composer is configured via environment variables. An example configuration can
be seen in `local.env.example`.

To copy a configuration to your `.env` file run:

```sh

# Can specify an environment
just copy-env <ENVIRONMENT>

# By default will copy `local.env.example`
just copy-env
```

### Install cometbft

Ensure `~/go` is in your `PATH`, or `GOPATH` is set to some other place in your
`PATH`.

```bash
just install-cometbft
```

### Optional: install abci-cli for a bit of CLI testing

In the cometbft/ dir:

```bash
make install_abci
```

### Start the application

In astria-sequencer/:

```sh
cargo build
../../target/debug/astria-sequencer --db-filepath=/tmp/astria_db
```


### Query the app for info

```sh
$ abci-cli info
I[2023-05-16|16:53:56.786] service start    module=abci-client
    msg="Starting socketClient service"
    impl=socketClient
-> code: OK
-> data: astria_sequencer
-> data.hex: 0x626173655F617070
```

### Start the cometbft node

```sh
# initialize the node
cometbft init

# inside astria-sequencer, update the genesis file to include genesis application state
../../target/debug/astria-sequencer-utils --genesis-app-state-file=test-genesis-app-state.json  --destination-genesis-file=$HOME/.cometbft/config/genesis.json

# set the block time to 15s
sed -i'.bak' 's/timeout_commit = "1s"/timeout_commit = "15s"/g' ~/.cometbft/config/config.toml

# start the node
cometbft start
```

You should see blocks being produced.

You can also use `just` to run the above commands:

```bash
just run-cometbft
```

## Testnet

## Initialize and run the testnet

This installs cometbft.

This will create 3 cometbft validator configs and put the validator configs in
`./sequencer_testnet` by default. It will then start the respective sequencer
applications (one for each cometbft validator) and the cometbft validator nodes.

```bash
just run-testnet
```

you can also manually set the number of validators or the output dir

```bash
OUT_DIR=out_dir NUM_VALIDATORS=num_validators just run-testnet 
```

## Logs

For example, to see the cometbft logs for node0:

```sh
just cometbft-logs-testnet node0
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

Call it on the directory that the config files are in. By default looks at `sequencer_testnet`

Automatically stops the testnet when this is called.

```bash
just clean-testnet
```