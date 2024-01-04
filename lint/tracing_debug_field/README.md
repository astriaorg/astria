# `tracing_debug_field`

## What it does

Warns about fields in `tracing::event!` macros that are debug formatted
using either the `?` sigil or the
[`tracing::field::debug`](https://docs.rs/tracing/0.1.40/tracing/field/fn.debug.html)
utility.

## Why is this bad?

Debug formatted fields are bad for observability as they are usually unparseable
and unreadable. They also surface language-specific implementation details that
are unnecessary and too verbose for operators.

## What to do instead

Depending on the complexity of what must be logged:

1. Use or implement `std::fmt::Display` for an object and the `%` sigil
   (this usually works for newtype wrappers of strings or string-like objects).
2. For binary types, implement `Display` with a base16 (hex) or base64 encoding.
3. Reduce the information that is emitted in to the necessary amount so not the
   entire kitchen-sink is logged. For example, emit the number of items
   contained in a container, or the range of a map's keys instead of the
   items/keys themselves.
4. If an entire object must be emitted, serialize it to a machine parseable
   format (for example using JSON).
5. Hash an object and emit the hash (this works very well for messages that will
   be otherwise recorded and can be retrieved outside the observability intake).

## Example

```rust
#[derive(Clone, Copy, Debug)]
struct Wrapped(&'static str);

fn main() {
    let val = Wrapped("wrapped");
    info!(field = ?val, "using the sigil");
}
```

Use instead:

```rust
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug)]
struct Wrapped(&'static str);

impl Display for Wrapped {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(self.0)
  }
}

fn main() {
    let val = Wrapped("wrapped");
    info!(field = %val, "using the sigil");
}
```
