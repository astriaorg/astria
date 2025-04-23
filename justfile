####################################################
## NOTE: Minimum supported just version is 1.37.0 ##
####################################################

import 'charts/just/mod.just'

# Kubetail usage. 'just --list kubetail' for more...
mod kubetail 'dev/just/kubetail.just'
# ArgoCD usage. 'just --list argo' for more...
mod argo 'dev/just/argo.just'
# Helm usage. 'just --list helm' for more...
mod helm 'dev/just/helm.just'

_default:
  @just --list

default_docker_tag := 'local'
default_repo_name := 'ghcr.io/astriaorg'


# Docker Build
###############
[doc("
Builds docker image for the crate. Defaults to 'local' tag.
NOTE: `_crate_short_name` is invoked as dependency of this command so that failure to pass a valid
binary will produce a meaningful error message.
Usage:
  just docker-build [crate] <tag> <repo_name> (defaults: 'local', 'ghcr.io/astriaorg')
Example:
  just docker-build astria-sequencer
")]
docker-build crate tag=default_docker_tag repo_name=default_repo_name: (_crate_short_name crate "quiet")
  #!/usr/bin/env sh
  set -eu
  short_name=$(just _crate_short_name {{crate}})
  set -x
  docker buildx build --load --build-arg TARGETBINARY={{crate}} -f containerfiles/Dockerfile -t {{repo_name}}/$short_name:{{tag}} .


# Docker Build and Load
########################
[doc("
Builds and loads docker image for the crate. Defaults to 'local' tag.
NOTE: `_crate_short_name` is invoked as dependency of this command so that failure to pass a valid
binary will produce a meaningful error message.
Usage:
  just docker-build-and-load [crate] <tag> <repo_name> (defaults: 'local', 'ghcr.io/astriaorg')
Example:
  just docker-build-and-load astria-sequencer
")]
docker-build-and-load crate tag=default_docker_tag repo_name=default_repo_name: (_crate_short_name crate "quiet")
  #!/usr/bin/env sh
  set -eu
  short_name=$(just _crate_short_name {{crate}})
  set -x
  just docker-build {{crate}} {{tag}} {{repo_name}}
  just load-image $short_name {{tag}} {{repo_name}}


# Maps a crate name to the shortened name used in the docker tag.
# If `quiet` is an empty string the shortened name will be echoed. If `quiet` is a non-empty string,
# the only output will be in the case of an error, where the input `crate` is not a valid one.
_crate_short_name crate quiet="":
  #!/usr/bin/env sh
  set -eu
  case {{crate}} in
    astria-auctioneer) short_name=auctioneer ;;
    astria-bridge-withdrawer) short_name=evm-bridge-withdrawer ;;
    astria-cli) short_name=astria-cli ;;
    astria-composer) short_name=composer ;;
    astria-conductor) short_name=conductor ;;
    astria-sequencer) short_name=sequencer ;;
    astria-sequencer-relayer) short_name=sequencer-relayer ;;
    *) echo "{{crate}} is not a supported binary" && exit 2
  esac
  [ -z {{quiet}} ] && echo $short_name || true


# Install CLI
##############
[doc("
Installs the Astria Rust CLI from local codebase.
")]
install-cli:
  cargo install --path ./crates/astria-cli --locked


# Compile Protos
#################
[doc("
Generates rust code from protos into 'crates/astria-core/generated' to be used
throughout the codebase.
")]
compile-protos:
  cargo run --manifest-path tools/protobuf-compiler/Cargo.toml


# Compile Solidity Contracts
#############################
[doc("
Compiles Solidity contracts for bridging.
")]
compile-solidity-contracts:
  cargo run --manifest-path tools/solidity-compiler/Cargo.toml


####################################################
## Scripts related to formatting code and linting ##
####################################################

default_lang := 'all'


# Format
#########
[doc("
Can format 'rust', 'toml', 'proto', or 'all'. Defaults to all.
")]
fmt lang=default_lang:
  @just _fmt-{{lang}}


# Lint
#######
[doc("
Can lint 'rust', 'toml', 'proto', 'md' or 'all'. Defaults to all.
Sub-lints for rust include: 'rust-fmt', 'rust-clippy', 'rust-clippy-custom', 'rust-clippy-tools', 'rust-dylint'
")]
lint lang=default_lang:
  @just _lint-{{lang}}


#####################
## Private Recipes ##
#####################

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
  cargo +nightly-2024-10-03 fmt --all

[no-exit-message]
_lint-rust:
  just _lint-rust-fmt
  just _lint-rust-clippy
  just _lint-rust-clippy-custom
  just _lint-rust-clippy-tools
  just _lint-rust-dylint

[no-exit-message]
_lint-rust-fmt:
  cargo +nightly-2024-10-03 fmt --all -- --check

[no-exit-message]
_lint-rust-clippy:
  cargo clippy --version
  cargo clippy --all-targets --all-features \
          -- --warn clippy::pedantic --warn clippy::arithmetic-side-effects \
          --warn clippy::allow_attributes --warn clippy::allow_attributes_without_reason \
          --deny warnings

[no-exit-message]
_lint-rust-clippy-custom:
  cargo +nightly-2024-10-03 clippy --all-targets --all-features \
          -p tracing_debug_field \
          -- --warn clippy::pedantic --deny warnings

[no-exit-message]
_lint-rust-clippy-tools:
  cargo clippy --manifest-path tools/protobuf-compiler/Cargo.toml \
          --all-targets --all-features \
          -- --warn clippy::pedantic --deny warnings

[no-exit-message]
_lint-rust-dylint:
  cargo dylint --all --workspace

[no-exit-message]
_fmt-toml:
  taplo format

[no-exit-message]
_lint-toml:
  taplo format --check

[no-exit-message]
_lint-md:
  markdownlint-cli2

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
  buf breaking proto/upgrades --against 'buf.build/astria/upgrades'
  buf breaking proto/mempoolapis --against 'buf.build/astria/mempool-apis'
