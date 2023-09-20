# astria-sequencer-utils

## Requirements

- rust 1.68

## Usage

Running requires two flags: `--genesis-app-state-file` and
`--destination-genesis-file`. The command takes all data in the source file and
merges that data into the destination file, overwriting the original destination
file.

### Build and start the application

In astria-sequencer-utils/:

```sh
cargo run -- --genesis-app-state-file=<source json path> \
  --destination-genesis-file=<destination json path>
```

For example:
```sh
cargo run -- --genesis-app-state-file=../astria-sequencer/test-genesis-app-state.json \
 --destination-genesis-file=$HOME/.cometbft/config/genesis.json
```
