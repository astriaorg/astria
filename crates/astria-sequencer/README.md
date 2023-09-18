# astria-sequencer

## Requirements

- rust 1.68
- gcc-12, gcc-12-libs
- go 1.18+

## Building

Because `penumbra-storage:0.54.1` depends on `rocksdb:0.19.0` compilation on gcc
13 will not work.

On arch linux:

```sh
sudo pacman -S gcc12 gcc12-libs
CC=/usr/bin/gcc-12 CXX=/usr/bin/c++-12 cargo build
```

<https://github.com/rust-rocksdb/rust-rocksdb/issues/713>
<https://github.com/facebook/rocksdb/pull/11118>

## Usage

### Install cometbft

Ensure `~/go` is in your `PATH`, or `GOPATH` is set to some other place in your
`PATH`.

```sh
git clone https://github.com/astriaorg/cometbft
cd cometbft
export GOPATH=~/go
make install
```

### Optional: install abci-cli for a bit of CLI testing

In the cometbft/ dir:

```sh
make install_abci
```

### Build and start the application

In astria-sequencer-utils:
```sh
```

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
