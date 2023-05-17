# abci-app-rs

## Requirements

- rust 1.68
- go 1.18+

## Usage

#### Install tendermint
```sh
git clone https://github.com/tendermint/tendermint.git
cd tendermint
git checkout release/v0.37.1
make install
```

#### Optional: install abci-cli for a bit of CLI testing

In the tendermint/ dir:
```sh
make install_abci
```

#### Build and start the application

In abci-app-rs/:
```sh
cargo build
./target/debug/app
```

#### Query the app for info

```sh
$ abci-cli info
I[2023-05-16|16:53:56.786] service start                                module=abci-client msg="Starting socketClient service" impl=socketClient
-> code: OK
-> data: base_app
-> data.hex: 0x626173655F617070
```

#### Start the tendermint node
```sh
tendermint node
```