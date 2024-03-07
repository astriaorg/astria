# The Astria protobuf to Rust compiler tool

This small binary invokes the `buf` protobuf management cli and
`tonic-build` to compile the Astria protobuf specifications at
[`../proto/`](../proto/) and writes them to
[`../crates/astria-core/src/generated`](../crates/astria-core/src/generated).

See [`proto/README.md`](../proto/README.md) and
[`astria-core/README.md`](../crates/astria-core/README.md) for how to use this
tool.
