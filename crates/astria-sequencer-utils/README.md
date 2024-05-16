# astria-sequencer-utils

## Requirements

- rust 1.68

## General

There are two functions provided by the tool, as described below.

### `copy-genesis-state`: JSON-encode Genesis State to a File

The subcommand takes all data in the input file and merges that data into the output file,
overwriting the original output file.

#### Usage

Running this subcommand requires three arguments:
- `--genesis-app-state-file`: the path to the input file
- `--output` (a.k.a. `--destination-genesis-file` for backwards compatibility): the path to the
output file
- `--chain-id`: the chain ID (a.k.a. network name) of the relevant network

#### Example

In `crates/astria-sequencer-utils`:

```sh
cargo run -- copy-genesis-state \
  --genesis-app-state-file=../astria-sequencer/test-genesis-app-state.json \
  --output=$HOME/.cometbft/config/genesis.json \
  --chain-id=astria
```

---

### `parse-blob`: Parse Encoded Blob Data

The subcommand takes in base-64-encoded blob data, such as can be found in
[the Celenium blob viewer](https://mocha.celenium.io/block/1726074?tab=transactions) and outputs it
to stdout in a human-readable format.

#### Usage

This subcommand has one required unnamed arg, one optional one and an optional flag:
- unnamed arg: this is interpreted as follows:
    - if the value is `-` (a single hyphen), the input is read from stdin
    - if the value is a path to a file, the file's contents are handled as the base-64 encoded data
    - otherwise the value is handled as the base-64 encoded data
- `--format`: can be `"display"` (the default) for human-readable output, or `"json"` for
  JSON-encoded output
- `--verbose`: if provided, the output contains the full contents of all the parseable data rather
  than summaries or counts

#### Example

In `crates/astria-sequencer-utils`:

```sh
# input from a file
cargo run -- parse-blob \
 tests/resources/parse_blob/batched_rollup_data/input.txt \
 --verbose

# input as base-64-encoded string
cargo run -- parse-blob \
 $(cat parse-blob tests/resources/parse_blob/batched_rollup_data/input.txt) \
 --format=json

# input via stdin
cargo run -- parse-blob <<< cat tests/resources/parse_blob/batched_rollup_data/input.txt
```
