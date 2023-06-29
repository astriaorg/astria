# astria-sequencer-utils

## Requirements

- rust 1.68

## Usage

Running requires two flags: `--source-genesis-file` and `--destination-genesis-file`.
These can be shortened to `-s` and `-d`.
The command takes all data in the source file and merges that data into the destination file, overwriting the original destination file.

### Build and start the application

In astria-sequencer-utils/:
```sh
cargo build
cargo run -- -s=<source json path> -d=<destination json path>
```
