# Astria RPC

This repo contains all the protobuf definitions for the different Astria RPC services.

## Protos and Buf Build

[Buf Build](https://buf.build/) is a platform and registry for sharing Protocol Buffers between team members. It also comes with a set of tools to generate gRPC servers and clients in a range of languages.

[Astria's Buf Build organization](https://buf.build/astria)

## Adding an RPC service

* create new cargo lib in root of repo, e.g. `cargo new sandwich-apis --lib`
* add the new lib to `Cargo.toml` as a workspace member
* define protos in `sandwich-apis/proto/`
* create `buf.gen.yaml` and `buf.yaml` in `sandwich-apis/`.
* use `execution-apis/` as a reference for `buf.gen.yaml` and `buf.yaml`.
* create `build.rs`, `touch sandwich-apis/build.rs`
* copy over logic from `execution-apis/build.rs` for now, updating necessary paths
* update `sandwich-apis/src/lib.rs` to include the files generated from `build.rs`. see `execution-apis/src/lib.rs`
* add steps to `.github/workflows/pull-request.yml` and `.github/workflows/push.yml` to lint, build, and push the buf image from the new proto defs

## Working with Buf locally

### First, install Buf CLI and authenticate yourself:

* `$ brew install bufbuild/buf/buf` - using homebrew
    * [other ways to install](https://docs.buf.build/installation)
* `$ buf registry login` - [must first create an API token](https://docs.buf.build/tutorials/getting-started-with-bsr#create-an-api-token)

### Building and pushing after making changes in `proto`

* `$ cd astria-rpc` - must be in same directory as `buf.yaml`
* `$ buf build` - [builds the proto files into a single binary file](https://docs.buf.build/build/explanation#what-are-buf-images)
* `$ buf push` - pushes a module to the registry

### Generating clients and servers

* `$ cd astria-rpc` - must be in same directory as `buf.gen.yaml`
* `$ buf generate` - generate clients and servers according to the configuration in `buf.gen.yaml`

