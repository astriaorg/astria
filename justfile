import 'charts/deploy.just'

# commands to simplify Kubetail usage
mod kubetail 'dev/kubetail.just'

mod? argo 'dev/argo.just'
mod? helm 'dev/helm.just'

default:
  @just --list

default_docker_tag := 'local'
default_repo_name := 'ghcr.io/astriaorg'

# Builds docker image for the crate. Defaults to 'local' tag.
# NOTE: `_crate_short_name` is invoked as dependency of this command so that failure to pass a valid
# binary will produce a meaningful error message.
docker-build crate tag=default_docker_tag repo_name=default_repo_name: (_crate_short_name crate "quiet")
  #!/usr/bin/env sh
  set -eu
  short_name=$(just _crate_short_name {{crate}})
  set -x
  docker buildx build --load --build-arg TARGETBINARY={{crate}} -f containerfiles/Dockerfile -t {{repo_name}}/$short_name:{{tag}} .

# Builds and loads docker image for the crate. Defaults to 'local' tag.
# NOTE: `_crate_short_name` is invoked as dependency of this command so that failure to pass a valid
# binary will produce a meaningful error message.
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
    astria-bridge-withdrawer) short_name=bridge-withdrawer ;;
    astria-cli) short_name=cli ;;
    astria-composer) short_name=composer ;;
    astria-conductor) short_name=conductor ;;
    astria-sequencer) short_name=sequencer ;;
    astria-sequencer-relayer) short_name=sequencer-relayer ;;
    *) echo "{{crate}} is not a supported binary" && exit 2
  esac
  [ -z {{quiet}} ] && echo $short_name || true

# Installs the astria rust cli from local codebase
install-cli:
  cargo install --path ./crates/astria-cli --locked

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
  cargo +nightly-2024-09-15 fmt --all

[no-exit-message]
_lint-rust:
  cargo +nightly-2024-09-15 fmt --all -- --check
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
