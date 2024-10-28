# Astria-gRPC-Mock

A lightweight crate for mounting mock gRPC responses to a mock server, for use
in local testing. Heavily influenced by [Wiremock](https://docs.rs/wiremock/latest/wiremock/).

## Components

* [Matcher](https://github.com/astriaorg/astria/blob/main/crates/astria-grpc-mock/src/matcher.rs)
* [MockServer](https://github.com/astriaorg/astria/blob/main/crates/astria-grpc-mock/src/mock_server.rs)
* [Mock](https://github.com/astriaorg/astria/blob/main/crates/astria-grpc-mock/src/mock.rs)
* [Response](https://github.com/astriaorg/astria/blob/main/crates/astria-grpc-mock/src/response.rs)

## Usage

The gRPC mock crate works by providing the functionality to mount `Mock`s to `MockServer`s.
A `Mock`, among other fields, contains a `Matcher` and a `ResponseTemplate`. Upon
being mounted to the server, the `Mock` will check incoming requests to the server
for a match using the `Matcher`. If the criteria is met, it will respond with
`ResponseTemplate::respond()`.

To use the gRPC mock functionality, you first need to instantiate a `MockServer`
with `MockServer::new()`.

To create a `Mock`, a typical flow is the following:

```rust
Mock::for_rpc_given( "rpc_name", {Matcher} ).respond_with( {ResponseTemplate} );
```

Additionally, you can further customize a given `Mock` with the following methods:

* `up_to_n_times(n)` - Responds up to a given number of times (inclusive).
* `expect(range)` - Sets the range of times a `Mock` should expect to recieve a
matching request.
* `with_name(name)` - Sets the name of the `Mock`. Using this is best practice as
it can aid in determining failure points during testing.

There are two ways of mounting a `Mock` to a server:

1. `Mock::mount()` - This is the simplest way of mounting the mock, simply mounting
the mock to the server. The mock will be verified once the `MockServer` is dropped
or manually verified, panicking if it did not receive an expected number of requests
or if there were any bad responses from the `Mock`.
2. `Mock::mount_as_scoped()` - This method is best used if you want to evaluate
the outcome of a `Mock` before shutting down the server. It returns a `MockGuard`,
which can be eagerly evaluated by calling `MockGuard::wait_until_satisfied()`. Otherwise
it will be evaluated upon `Drop`ping.
