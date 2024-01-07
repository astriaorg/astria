# Astria code guidelines

## Clippy is pedantic

All clippy lints raised by `cargo clippy -- -W clippy::pedantic` must be
followed. Opt-outs using the `#[allow()]` attribute should be the exception
and must always be explained with a comment right next to the attribute.

Example:

```rust
// Allow: the function body asserts at compile time that the panic cannot happen.
#[allow(clippy::missing_panics_doc)]
pub fn from_verification_key(
    public_key: ed25519_consensus::VerificationKey,
) -> Self {
    // Ensure that `ADDRESS_LEN` is not changed to violate this assumption.
    // Allow: false-positive because clippy does not understand compile time constraints.
    #[allow(clippy::assertions_on_constants)]
    const _: () = assert!(ADDRESS_LEN <= 32);
    todo!()
  }
```

## No `unwrap`ping of `Option`s, `Result`s

The use of `Option::unwrap` or `Result::unwrap` is banned. In general, services
should not panic in the middle of operation and instead prefer to emit an error
and shutdown gracefully.

If an option or a result must be unwrapped, `Option::expect` or `Result::expect`
must be used together with a clear message why the author believes that this
should not lead to a panic in production.

Cases were unrwapping ("`expect`ing") is permitted are:
1. invariants being violated that should have been enforced at the type level.
2. very strong assumptions about fundamental functionality of the codebase.

An example for 1. could be a friends collection that tracks friends' locations
and their age and that can only be written to by updating both maps at the same
time:
```rust
struct Friends {
    location: HashMap<String, String>, 
    age: HashMap<String, u16>,
}

impl Friends {
    fn insert(&mut self, name: String, location: String, age: u16) {
        self.location(name.clone(), location);
        self.age(name.clone(), age);
    }

    fn get_location_and_age(&self, name: &str) -> Option<(String, u16)> {
        let location = self.location.get(name)?.clone();
        let age = self
          .age
          .get(name)
          .expect("a friend in the location map must also exist in the age map");
        Some((location, age))
    }
}
```

An example for 2 could be the assumption that CometBFT heights must always
be representable by an unsigned `u32`. While the
[`tendermint-rs`](docs.rs/tendermint) crate represents these internally as
unsigned `u64`, CometBFT opted to represet its heights as signed `int64`
which in practice must not be negative.

## Astria services use `tracing` and OpenTelemetry

All Astria services are instrumented using the
[`astria-telemetry`](crates/astria-telemetry) crate. Services only generate
trace data ("logs") via the macros and functions defined by the
[`tracing`] crate and write it to an OpenTelemetry endpoint via a
[`tracing-opentelemetry`] layer registered through [`tracing-subscriber`].

Services must not write directly to STDOUT or STDERR using `println!`,
`eprintln!`, a handle to stdout/stderr, or any other way.
The only exception from this rule is writing build and config information
before telemetry is set up.

[`tracing`]: docs.rs/tracing
[`tracing-subscriber`]: docs.rs/tracin-subscriber
[`tracing-opentelemtry`]: docs.rs/tracing-opentelemetry

## Astria services use `metrics`

TODO: Add a few words on metrics

## Library crates define typed errors

Library crates must use fully typed errors without exception.
Library crates must not use `eyre` or `anyhow`. Prefer using
[`thiserror`](docs.rs/thiserror) to reduce boiler plate when typing errors.

# Binary crates use dynamic errors via `astria-eyre`

The [`astria-eyre`](crates/astria-eyre) contains a global hook
that ensures that all eyre `Report` types are formatted with their full
cause-chain, and are compatible with machine-readable observability
systems. Every service must be initialized like shown below and
before any other `eyre::Report` types are constructed.

```rust
fn main() {
    astria_eyre::install().expect("the hook must be called first");
    todo();
}
```

**NOTE**: `crates/astria-sequencer` is exempt from this rule as it relies
heavily on [`penumbra`](https://github.com/penumbra-zone/penumbra), which
uses [`docs.rs/anyhow`].

## All errors must provide context, ball-of-mud enums are banned

**TODO**: Give a better example that shows how two IO errors shouldn't
be bunched together.

All errors must provide context on what and why they happened. For example,
report that opening file containing logs failed, not that filesystem IO
returned an errop. This means that service code using `eyre` or `anyhow` must
use the `eyre::WrapErr::wrap_err` or the `anyhow::Context::context` adaptors and
add meaningful information to understand what went wrong.
For libraries this means that so called "ball-of-mud" enums that only exist to
make the question-mark operator line up are forbidden.

### Example for services using `eyre`

```rust
use eyre::WrapErr;

// Prefer
fn good_read_logs(p: impl AsRef<Path>) -> eyre::Result<()> {
    let f = std::fs::File::open(p)
      .wrap_err("failed to open file at provided path")?;
    let cfg: Config = serde_json::from_reader(f)
      .wrap_err("failed to read config")?;
    Ok(())
}

// Avoid
fn bad_read_logs(p: impl AsRef<Path>) -> eyre::Result<()> {
    let f = std::fs::File::open(p)?;
    let cfg: Config = serde_json::from_reader(f)?;
    Ok(())
}
```

### Example for libraries using `thiserror`

```rust
#[derive(Debug, thiserror::Error)]
enum BadError {
  #[error("io failed")]
  Io(#[from] std::io::Error),
  #[error("json failed")]
  Json(#[from] serde_json::se::Error),
}

#[derive(Debug, thiserror::Error)]
enum GoodError {
  #[error("failed opening log at {path}")]
  OpenLog(#[source] std::io::Error, path: String),
  #[error("failed parsing line {line} of log file as JSON")]
  ParseLine { source: serde_json::se::Error, line: usize },
}
```

## Errors are formatted using the `astria_eyre` hook or a `std::error::Error` trait object

To get the full cause-chain of an error without using its debug formatting,
either use an `eyre::Report` with the eyre formatting hook installed via
`astria_eyre::install()`. Or alternatively cast an error to a
`dyn std::error::Error` trait object.

The reason for the latter is that `tracing` implements
[`tracing::Value for dyn std::error::Error`], but not does not provide a
blanket impl for all `T: std::error::Error` (likely because Rust does not
support specialization).

[`tracing::Value for dyn std::error::Error`]: https://docs.rs/tracing/*/tracing/trait.Value.html#impl-Value-for-dyn+Error

```rust
# cast to the trait object if you have Result<T, E> where E: std::error::Error
warn!(
  error = &your_err as &dyn std::error::Error,
  "something went wrong",
);
```

**NOTE**: `astria-sequencer` uses `anyhow` and hence cannot make use of the
`astria-eyre` formatting hook. It there has to use its
`AsRef<dyn std::error::Error>` implementation:

```rust
warn!(
  error = AsRef::<dyn std::Error::Error>::as_ref(&the_anyhow_error),
  "something went wrong",
);
```

## The tracing `instrument` attribute must `skip_all`

Async functions instrumented using the
[`tracing::instrument`](https://docs.rs/tracing/*/tracing/attr.instrument.html)
attribute must specify `skip_all`. There are
three reasons for this:
1. expliclitly specifying which fields are injected into the span makes it
   easier to understand for non-rust practitioners.
2. populating spans with too much data leads to noisy logs and is in general bad
   practice.
3. the span constructed via the proc macro by default emits all fields using
   their `std::fmt::Debug` implementation, which is forbidden.

There is only one exception: if all fields of a function are reported, and if
all fields are primitive types (or more specifically, types which implemented
[`tracing::field::Value`]).

[`tracing::field::Value`]: https://docs.rs/tracing/*/tracing/trait.Value.html

Example:

```rust
use tracing::instrument;

#[instrument(skip_all)]
async fn the_method(&self, foo: Foo, bar: Bar) {
  todo!()
}
```

## Spans must contain minimal informaition, not inject noise

Keep the amount of information in span fields minimal, because
every field of that span will be attached to all events emitted under it (this
also includes all events under its child spans!). If observing the contents
of a bigger type is desired (like a config or the body of an HTTP request),
serialize it as JSON and emit it as a field.

```rust
// Prefer
#[instrument(skip_all)]
async fn good_service(config: Config) {
  info!(
    config = %display::json(&config),
    "creating the service",
  );
}

// Avoid
#[instrument(skip_all, fields(config = %display::json(&config)))]
async fn bad_service(config: Config) {
  todo!()
}
```

## Add a field to a span to avoid repetition

If you repeatedly find yourself adding the same field with the same
name and information to all events in a function, consider instrumenting
the function and adding the field to its span.

Example:

```rust
// Prefer this: add the repeated fields to the span
#[instrument]
async fn fetch(endpoint: &str, height: u64) {
    info!("fetching the thing");
    match client.get(format!("{endpoint}/{height}")).await {
        Ok(the_thing) => info!("got the thing"),
        Err(error) => error!(%error, "didn't get the thing"),
    }
}

// Avoid this: a function with the same field in every event
async fn fetch(endpoint: &str, height: u64) {
    info!(endpoint, height, "fetching the thing");
    match client.get(format!("{endpoint}/{height}")).await {
        Ok(the_thing) => info!(endpoint, height, "got the thing"),
        Err(error) => error!(%error, endpoint, height, "didn't get the thing"),
    }
}
```

## No debug: fields in tracing events and spans must be display-formatted

Tracing events containing debug-formatted fields are forbidden.
This is usually done using the `?` sigil or the [`tracing::fmt::debug`]
utility in explicitly constructed events or spans via macros
like [`tracing::info!`] or [`tracing::warn_span!`].

In spans constructed via the [`tracing::instrument`] attribute
this either happens implicitly when forgetting to skip fields, or explicitly
by using `?` or [`tracing::fmt::debug`] in the `fields` directive.

[`tracing::fmt::debug`]: https://docs.rs/tracing/*/tracing/field/fn.debug.html
[`tracing::info!`]: https://docs.rs/tracing/*/tracing/macro.info.html
[`tracing::warn_span!`]: https://docs.rs/tracing/latest/*/macro.warn_span.html
[`tracing::instrument`]: https://docs.rs/tracing/*/tracing/attr.instrument.html
```rust
// XXX: Prefer this:
#[instrument(skip_all, fields(%foo, %bar)]
fn the_method(foo: Foo, bar: Bar, baz: Baz) {
    info!(%baz, "an event");
}

// XXX: Avoid this:
#[instrument(skip(bar, baz), fields(?bar)]
fn the_method(foo: Foo, bar: Bar, baz: Baz) {
    info!(?baz, "an event");
}
```
