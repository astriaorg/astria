# `astria-eyre`

Provides a custom [`eyre::EyreHandler`] type for use with [`eyre`] that formats
an [`eyre::Report`] as a JSON object with incrementing (but string-serialized)
integer keys:

```console
{"0": "high level context", "1": "intermediate", "2": "lowest level error"}
````

`astria-eyre`'s intended use is to make it easier to emit errors and their full
cause-chain in [`tracing`] events and spans while avoiding that errors are
accidentally written using their debug representation or in a human readable
format (making the emitted errors no longer machine parseable).

See the Usage section below on how to use this in tracing context.

[`eyre::EyreHandler`]: https://docs.rs/eyre/*/eyre/trait.EyreHandler.html
[`eyre`]: https://docs.rs/eyre
[`eyre::Report`]: https://docs.rs/eyre/*/eyre/struct.Report.html
[`tracing`]: https://docs.rs/tracing
[`tracing::instrument`]: https://docs.rs/tracing/*/tracing/attr.instrument.html

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
astria-eyre = { path = "../astria-eyre" }
```

Install the error hook before constructing any `eyre::Report` error types.

## Example

### Simple example

```rust,should_panic
use astria_eyre::eyre::{eyre, Report, WrapErr};

fn main() -> Result<(), Report> {
    astria_eyre::install()?;
    let e: Report = eyre!("an error occured");
    Err(e).wrap_err("usage example failed as expected")
}
```

### Usage with tracing events

Errors can be emitted as part of tracing events by simply adding a
display-formatted field. Because the `astria-eyre` hook ensures that the full
cause-chain is written it is not necessary to cast the error to a trait
object to trigger trigger formatting via
[`tracing::field::Value for &dyn std::error::Error`].

```rust
use astria_eyre::eyre::{eyre, Report};

astria_eyre::install();
let the_error: Report = eyre!("an error").wrap_err("some context");
tracing::warn!(%the_error, "an important task failed while serving a request");
```

### Usage in instrumented functions

For async functions that return an error it is often desirable to automatically
generate an event within the context of their span. This can be done using
[`instrument(err)`](https://docs.rs/tracing/*/tracing/attr.instrument.html).
However, apart from specifying `err(Display)` or `err(Debug)`
[`tracing::instrument`] does not give control on how these errors are
formatted. This means the error is either missing its cause-chain, or the event
is no longer machine-parseable.

`astria-eyre` ensures that emitted erors contain their full JSON-formatted
cause-chain.

Note that the example does not install a [`tracing-subscriber`].

[`tracing-subscriber`]: https://docs.rs/tracing-subscriber

```rust,should_panic
use astria_eyre::eyre::{bail, Result};

#[tracing::instrument(err)]
async fn get(endpoint: &str) -> Result<()> {
    // This will automatically generated an error-level event.
    // If the `astria-eyre` hook is installed before any eyre Reports are
    // constructed then the emitted error will be JSON-formatted.
    bail!("the request failed");
    Ok(())
}

fn main() -> Result<()> {
    astria_eyre::install()?;
    tokio_test::block_on(get("myhost.com"))?;
    Ok(())
}
```
