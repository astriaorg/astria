# astria-sequencer

## Requirements

- rust 1.68
- go 1.18+

## Usage

#### Install tendermint
Ensure `~/go` is in your `PATH`, or `GOPATH` is set to some other place in your `PATH`.

```sh
git clone https://github.com/astriaorg/cometbft
cd cometbft
export GOPATH=~/go
make install
```

#### Optional: install abci-cli for a bit of CLI testing

In the cometbft/ dir:
```sh
make install_abci
```

#### Build and start the application

In astria-sequencer/:
```sh
cargo build
./target/debug/app
```

#### Query the app for info

```sh
$ abci-cli info
I[2023-05-16|16:53:56.786] service start                                module=abci-client msg="Starting socketClient service" impl=socketClient
-> code: OK
-> data: astria_sequencer
-> data.hex: 0x626173655F617070
```

#### Start the tendermint node
```sh
cometbft init
cometbft node
```