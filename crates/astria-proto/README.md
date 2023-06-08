# Astria Proto

This repo contains all the protobuf packages for Astria. 

## Protos and Buf Build

[Buf Build](https://buf.build/) is a platform and registry for sharing Protocol Buffers between team members. It also comes with a set of tools to generate gRPC servers and clients in a range of languages.

[Astria's published protos](https://buf.build/astria/astria)

## Adding a package

* Create a new folder under `proto/astria/{new_package}`
* write protos in new folder using package name `astria.{new_package}`
* update `src/lib.rs` to include your new defintions in a module. Module names should reflect proto package names.

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

