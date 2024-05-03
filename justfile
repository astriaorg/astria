import 'charts/deploy.just'

default:
  @just --list

default_docker_tag := 'local'

# Builds docker image for the crate. Defaults to 'local' tag.
docker-build crate tag=default_docker_tag:
  docker buildx build --load --build-arg TARGETBINARY={{crate}} -f containerfiles/Dockerfile -t {{crate}}:{{tag}} .

# Installs the astria rust cli from local codebase
install-cli:
  cargo install --path ./crates/astria-cli --locked

# Compiles the generated rust code from protos which are used in crates.
compile-protos:
  cargo run --manifest-path tools/protobuf-compiler/Cargo.toml

####################################################
## Scripts related to formatting code and linting ##
####################################################

default_lang := 'all'

# Can format 'rust', 'toml', 'proto', or 'all'. Defaults to all
fmt lang=default_lang:
  @just _fmt-{{lang}}

# Can lint 'rust', 'toml', 'proto', 'md' or 'all'. Defaults to all.
lint lang=default_lang:
  @just _lint-{{lang}}

_fmt-all:
  @just _fmt-rust
  @just _fmt-toml
  @just _fmt-proto

@_lint-all:
  -just _lint-rust
  -just _lint-toml
  -just _lint-proto
  -just _lint-md

[no-exit-message]
_fmt-rust:
  cargo +nightly-2024-02-07 fmt --all

[no-exit-message]
_lint-rust:
  cargo +nightly-2024-02-07 fmt --all -- --check
  cargo clippy -- --warn clippy::pedantic
  cargo dylint --all

[no-exit-message]
_fmt-toml:
  taplo format

[no-exit-message]
_lint-toml:
  taplo format --check

[no-exit-message]
_lint-md:
  markdownlint-cli2 "**/*.md" "#target" "#.github"

[no-exit-message]
_fmt-proto:
  buf format -w

[no-exit-message]
_lint-proto:
  buf lint
  buf format -d --exit-code
  buf breaking proto/primitives --against 'buf.build/astria/primitives'
  buf breaking proto/executionapis --against 'buf.build/astria/execution-apis'
  buf breaking proto/sequencerblockapis --against 'buf.build/astria/sequencerblock-apis'
  buf breaking proto/protocolapis --against 'buf.build/astria/protocol-apis'
  buf breaking proto/composerapis --against 'buf.build/astria/composer-apis'

deploy-load-test:
    wget https://github.com/astriaorg/goomy-blob/releases/download/v0.1/blob-spammer
    chmod +x blob-spammer

run-load-test:
    #!/usr/bin/env bash
    ETH_RPC_URL="http://executor.astria.localdev.me/"
    PRIVATE_KEY=8b3a7999072c9c9314c084044fe705db11714c6c4ed7cddb64da18ea270dd203
    MAX_WALLETS=10
    MAX_PENDING=5
    THROUGHPUT=5
    TOTAL_TXS_TO_SEND=10

    echo "Spamming EOA txs..."
    if ! ./blob-spammer --privkey $PRIVATE_KEY --rpchost $ETH_RPC_URL eoatx --max-wallets $MAX_WALLETS --throughput $THROUGHPUT --max-pending $MAX_PENDING --count $TOTAL_TXS_TO_SEND; then
        echo "Failed to spam EOA txs"
        exit 1
    else
        echo "Spammed EOA txs"
    fi
    sleep 1
    echo "Spamming ERC20 txs..."
    if ! ./blob-spammer --privkey $PRIVATE_KEY --rpchost $ETH_RPC_URL erctx --max-wallets $MAX_WALLETS --throughput $THROUGHPUT --max-pending $MAX_PENDING --count $TOTAL_TXS_TO_SEND; then
        echo "Failed to spam ERC20 txs"
        exit 1
    else
        echo "Spammed ERC20 txs"
    fi
    sleep 1
    echo "Spamming Large Txs txs..."
    if ! ./blob-spammer --privkey $PRIVATE_KEY --rpchost $ETH_RPC_URL largetx --max-wallets 1 --throughput 1 --max-pending 1 --count 5; then
        echo "Failed to spam Large Txs txs"
        exit 1
    else
        echo "Spammed Large Txs txs"
    fi
    echo "Done spamming Large Txs txs..."
    echo "Spamming Uniswap v2 swaps txs..."
    if ! ./blob-spammer --privkey $PRIVATE_KEY --rpchost $ETH_RPC_URL univ2tx --max-wallets $MAX_WALLETS --throughput $THROUGHPUT --max-pending $MAX_PENDING --count $TOTAL_TXS_TO_SEND; then
        echo "Failed to spam Uniswap v2 txs"
        exit 1
    else
        echo "Spammed Uniswap v2 txs"
    fi
    echo "Done spamming Uniswap v2 txs..."
