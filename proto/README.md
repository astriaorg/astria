# The Astria protobuf specifications

This directory holds the Protobuf specifications that are used
by all Astria services. See the [`astria-proto`](../crates/astria-proto) crate
for how to use them.

## Protos and Buf Build

[Buf Build](https://buf.build/) is a platform and registry for sharing Protocol
Buffers between team members. It also comes with a set of tools to generate gRPC
servers and clients in a range of languages.

[Astria's published protos](https://buf.build/astria/astria)

## Modifying existing and adding new protobuf types

CI verifies that the generated Rust code is in sync with the source protobuf
definitions in CI.

Add new or modify existing protobuf types in [`../proto`] and then regenerate
the Rust sources with Astria's protobuf compiler tool from the root of the monorepo:

```sh
$ cargo run --manifest-path tools/protobuf-compiler/Cargo.toml
# Will emit warnings or errors raised by buf
```

There are also just commands which can be run from anywhere in repo:

```sh
# Compiles protos as above
$ just compile-protos

# Will apply formatting to proto files
$ just fmt proto

# checks for breaking changes, buf lint errors, and logs any formatting changes
$ just lint proto
```

When creating a new package, follow the following convention:

* Create a new folder `proto/<pkg-name>/astria/<pkg-name>/<version>`.
* Create a new `buf.yaml` file at `proto/<pkg-name>/buf.yaml`
* Add the new package to the dependencies of workspace at `../buf.work.yaml`
* Write protos in this folder using the convention name
  `astria.<pkg-name>.<version>`.

## Working with Buf locally

### First, install Buf CLI and authenticate yourself

* `$ brew install bufbuild/buf/buf` - using homebrew
  * [other ways to install](https://docs.buf.build/installation)
* `$ buf registry login` - [must first create an API
  token](https://docs.buf.build/tutorials/getting-started-with-bsr#create-an-api-token)
