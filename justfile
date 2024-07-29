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

# version is a celestia-app release. Example: v1.9.0
# operatingSystem is Linux or Darwin
# machineHardwareName is arm64 or x86_64
celestia_default_appd_dst := "."
get-celestia-appd version operatingSystem machineHardwareName dst=celestia_default_appd_dst:
  #!/usr/bin/env bash
  src="celestia-app_{{operatingSystem}}_{{machineHardwareName}}.tar.gz"
  curl -LOsS -q --output-dir "{{dst}}" \
    https://github.com/celestiaorg/celestia-app/releases/download/{{version}}/"$src"
  curl -LOsS -q --output-dir "{{dst}}" \
    https://github.com/celestiaorg/celestia-app/releases/download/{{version}}/checksums.txt
  cd "{{dst}}"
  cd -
  tar --directory "{{dst}}" -xvzf "{{dst}}"/"$src" celestia-appd

# Compiles the generated rust code from protos which are used in crates.
compile-protos:
  cargo run --manifest-path tools/protobuf-compiler/Cargo.toml

# Compiles the generated rust code from protos which are used in crates.
compile-solidity-contracts:
  cargo run --manifest-path tools/solidity-compiler/Cargo.toml

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
  markdownlint-cli2 "**/*.md" "#target" "#.github" "#**/*/astria-bridge-contracts/lib/**/*.md"

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
