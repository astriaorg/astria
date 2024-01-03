default:
  @just --list

default_docker_tag := 'local'

# Builds docker image for the crate. Defaults to 'local' tag.
docker-build crate tag=default_docker_tag:
  docker buildx build --load --build-arg TARGETBINARY={{crate}} -f containerfiles/Dockerfile -t {{crate}}:{{tag}} .

install-cli:
  cargo install --path ./crates/astria-cli --locked

# Compiles the generated rust code from protos which are used in crates.
compile-protos:
  cargo run --manifest-path tools/protobuf-compiler/Cargo.toml

## Scripts related to formatting code
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
  cargo +nightly-2023-08-18 fmt --all 

[no-exit-message]
_lint-rust:
  cargo +nightly-2023-08-18 fmt --all -- --check
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
  buf format proto -w

[no-exit-message]
_lint-proto:
  buf lint proto
  buf format proto -d --exit-code
  buf breaking proto --against 'buf.build/astria/astria'
