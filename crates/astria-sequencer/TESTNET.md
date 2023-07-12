# Multi-node testnet

To run a multi-node local testnet, you will need to have cometbft installed. 

#### Install cometbft
Ensure `~/go` is in your `PATH`, or `GOPATH` is set to some other place in your `PATH`.

```sh
git clone https://github.com/astriaorg/cometbft
cd cometbft
export GOPATH=~/go
make install
```

#### Build the sequencer application

In astria-sequencer/:
```sh
cargo build
```

#### Initialize and run the testnet

This will create 3 cometbft validator configs and put the validator configs in `./sequencer_testnet` by default.
It will then start the respective sequencer applications (one for each cometbft validator) and the cometbft validator nodes.

```sh
./scripts/testnet.sh
```

To manually set the output directory or number of validators:
```sh
OUT_DIR=out_dir NUM_VALIDATORS=num_validators ./scripts/testnet.sh
```

#### Logs

For example, to see the cometbft logs for node0:

```sh
tail -f sequencer_testnet/node0/cometbft.log 
```

To see the sequencer app logs for node0:

```sh
tail -f sequencer_testnet/node0/sequencer.log 
```

### Stopping the testnet

```sh
pkill cometbft && pkill astria-sequencer
```

### Cleaning up the testnet

Simply stop the testnet and `rm -r sequencer_testnet` or whichever directory the config files are in.
