# rs-cnc

celestia-node REST client in Rust.

### Testing

TODO - use something like `dockertest`

- `$ docker-compose -f tests/docker/test-docker-compose.yml up bridge0`
- `$ cargo test -- --nocapture --color always`

### Debugging

- vscode
    - plugins needed:
        - https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer
        - https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb
    - launch.json already included in the repo
        - found in https://gist.github.com/xanathar/c7c83e6d53b72dd4464f695607012629
