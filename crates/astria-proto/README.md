# Astria Proto

This repo contains all the protobuf packages for Astria. All rust code
generated from the protobuf files in [`proto/`](`./proto/`) is commited
to this repository and no extra tools are needed to encode to/decode from
protobuf.

Only when changing protobuf definitions (which is done by running running
an integration test) is extra tooling required. See below.

## Modifying existing and adding new protobuf

CI verifies that the generated Rust code is in sync with the source protobuf
definitions in CI. The test invoked by `cargo test -p astria-proto build` calls
the `buf` CLI tool and verifies that the Rust code before and after the change is the same.
If not, the test fails and leaves the repository in a dirty state. Commit
the generated Rust code and then rerun the test:
```sh
$ cargo test -p astria-proto build
test build ... FAILED

failures:

---- build stdout ----
thread 'build' panicked at 'the generated files have changed; please commit the changes', crates/astria-proto/tests/proto_build.rs:126:5
$ git commit -am "<message about protobuf changes>"
$ cargo test -p astria-proto build
test build ... ok
```

## Protos and Buf Build

[Buf Build](https://buf.build/) is a platform and registry for sharing Protocol Buffers between team members. It also comes with a set of tools to generate gRPC servers and clients in a range of languages.

[Astria's published protos](https://buf.build/astria/astria)

## Adding a package

* Create a new folder `proto/astria/<pkg-name>/<version>`;
* write protos in this folder using the convention name `astria.<pkg-name>.<version>`;
* update `src/proto/mod.rs` to include your new defintions in a module.
  Follow the conventions of the other submodules;
* update `src/lib.rs` to reexport the modules at the crate root.

## Working with Buf locally

### First, install Buf CLI and authenticate yourself:

* `$ brew install bufbuild/buf/buf` - using homebrew
    * [other ways to install](https://docs.buf.build/installation)
* `$ buf registry login` - [must first create an API token](https://docs.buf.build/tutorials/getting-started-with-bsr#create-an-api-token)

### Building and pushing after making changes in `proto`

* `$ buf build` - [builds the proto files into a single binary file](https://docs.buf.build/build/explanation#what-are-buf-images)
* `$ buf push` - pushes a module to the registry

### Generating clients and servers

* `$ buf generate` - generate clients and servers according to the configuration in `buf.gen.yaml`
