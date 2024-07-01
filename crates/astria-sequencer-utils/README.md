# astria-sequencer-utils

## Requirements

- rust 1.68

## General

There are three functions provided by the tool, further described below:

1. `generate-genesis-state`
1. `copy-genesis-state`
1. `parse-blob`

### `generate-genesis-state`: create an example sequencer genesis state

This subcommand creates an example genesis state file that can be
fed into the `copy-genesis-state` command. It should be editted to
have the desired options.

#### Usage for `generate-genesis-state`

1. no arguments: it will write the example genesis to stdout.
1. `--output <PATH>`: write the example genesis to `<PATH>`.
1. `-f`: override `<PATH>` in the previous argument if another file was present.

#### Example for `generate-genesis-state`

```sh
cargo run -- generate-genesis-state -o genesis-state.json
```

### `copy-genesis-state`: JSON-encode Genesis State to a File

The subcommand takes all data in the input file and merges that data into the
output file, overwriting the original output file.

#### Usage for `copy-genesis-state`

Running this subcommand requires three arguments:

1. `--genesis-app-state-file`: the path to the input file
1. `--output` (a.k.a. `--destination-genesis-file` for backwards compatibility):
the path to the output file
1. `--chain-id`: the chain ID (a.k.a. network name) of the relevant network

#### Example for `copy-genesis-state`

In `crates/astria-sequencer-utils`:

```sh
# genesis-state.json would be a file created by the generate-genesis-state
# subcommand and then manually edited
cargo run -- copy-genesis-state \
  --genesis-app-state-file=genesis-state.json \
  --output=$HOME/.cometbft/config/genesis.json \
  --chain-id=astria
```

---

### `parse-blob`: Parse Encoded Blob Data

The subcommand takes in base-64-encoded blob data, such as can be found in
[the Celenium blob viewer](https://mocha.celenium.io/block/1726074?tab=transactions)
and outputs it to stdout in a human-readable format.

#### Usage for `parse-blob`

This subcommand has one required unnamed arg, and two optional ones:

1. unnamed arg: this is interpreted as follows:
    1. if the value is `-` (a single hyphen), the input is read from stdin
    1. if the value is a path to a file, the file's contents are handled as the
base-64-encoded data
    1. otherwise the value is handled as the base-64 encoded data
1. `--format`: can be `"display"` (the default) for human-readable output, or
`"json"` for JSON-encoded output
1. `--verbose`: if provided, the output contains the full contents of all the
parseable data rather than summaries or counts

#### Example for `parse-blob`

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
