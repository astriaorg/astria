# `alias_trait_imports`

### What it does
Checks if a trait is imported without an alias, but is not explicitly named in the code.

### Why is this bad?
Importing a trait without aliasing can lead to namespace pollution.

### Example
```rust
// `Write` trait is imported but not aliased
use std::fmt::Write;

let mut out_string = String::new();
writeln!(&mut out_string, "Hello, world!");
```
Use instead:
```rust
use std::fmt::Write as _;

let mut out_string = String::new();
writeln!(&mut out_string, "Hello, world!");
```
