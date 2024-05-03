# `astria-build-info`

Inject build information about services and binaries at compile time.

`astria-build-info` provides the `astria_build_info::emit` utility
and the `astria_build_info::get!` macro to inject build information
into binaries.

`emit` is used to emit environment variables in a binary's build script,
while `get!` picks up the environment variables and constructs a `BuildInfo`
at compile time. `get!` is a macro so that it runs in the compilation context
of the binary and not in the context of the source package.

## Features

`astria-build-info` provides two features (both disabled by default):

+ `build` to enable the `emit` utility.
+ `runtime` to enable `BuildInfo` and the `get!` macro.

## Usage

Set up a service's dependencies like so:

```toml
[dependencies]
astria-build-info = { path = "../astria-build-info", features = ["runtime"] }

[build-dependencies]
astria-build-info = { path = "../astria-build-info", features = ["build"] }
```

And then use `emit` in the binary's build.rs, specifying the git tag which is
used for the service. For example, if the service is tagged with
`shaving-cats-v0.1.2`, then provide `emit("shaving-cats-v")` (supplying
`"shaving-cats"` also works, but it is recommended to use `"shaving-cats-v"` in
case there is another service/tag named `"shaving-cats-and-dogs-v1.2.3"`).

```rust,ignore
fn main() -> Result<(), Box<dyn std::error::Error>> {
    astria_build_info::emit("<release-tag-of-service>")?;
    Ok(())
}
```

And pick up the emitted variables like so:

```rust,ignore
use astria_build_info::BuildInfo;
const BUILD_INFO: BuildInfo = astria_build_info::get!();
```
