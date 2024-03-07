# Astria Core

This crate contains code to interact with the public API of Astria. In particularly
it contains definitions to convert Rust sources generated from the Astria protobuf
spececifications to idiomatic Rust types.

The Rust sources generated from protobuf specifications at
[`../proto/`](../proto) are commited to this crate under
[`./src/proto/generated/`](./src/proto/generated/).

This repo contains all the protobuf packages for Astria. All rust code generated
from the protobuf files in [`proto/`](`./proto/`) is committed to this repository
and no extra tools are needed to encode to/decode from protobuf.

## Modifying existing and adding new protobuf

CI verifies that the generated Rust code is in sync with the source protobuf
definitions in CI.

Add new or modify existing protobuf types in [`../proto`] and then regenerate
the Rust sources with Astria's protobuf compiler tool relative to the root of
the monorepo:

```sh
$ cargo run --manifest-path ../../tools/protobuf-compiler/Cargo.toml
# Will emit warnings or errors raised by buf
```

## Protos and Buf Build

[Buf Build](https://buf.build/) is a platform and registry for sharing Protocol
Buffers between team members. It also comes with a set of tools to generate gRPC
servers and clients in a range of languages.

[Astria's published protos](https://buf.build/astria/astria)

## Adding a package

* Create a new folder `proto/astria/<pkg-name>/<version>`.
* Write protos in this folder using the convention name
  `astria.<pkg-name>.<version>`.
* Update `src/proto/mod.rs` to include your new definitions in a module.
* Update `src/lib.rs` to reexport the modules at the crate root.

## Working with Buf locally

### First, install Buf CLI and authenticate yourself

* `$ brew install bufbuild/buf/buf` - using homebrew
  * [other ways to install](https://docs.buf.build/installation)
* `$ buf registry login` - [must first create an API
  token](https://docs.buf.build/tutorials/getting-started-with-bsr#create-an-api-token)

### Building and pushing after making changes in `proto`

* `$ buf build` - [builds the proto files into a single binary
  file](https://docs.buf.build/build/explanation#what-are-buf-images)
* `$ buf push` - pushes a module to the registry

### Generating clients and servers

* `$ buf generate` - generate clients and servers according to the configuration
  in `buf.gen.yaml`
