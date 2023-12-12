# The Astria protobuf to Rust compiler tool

This small binary invokes the `buf` protobuf management cli and
`tonic-build` to compile the Astria protobuf specifications at
[`../proto/`](../proto/) and writes them to
[`../crates/astria-proto/src/proto/generated`](../crates/astria-proto/src/proto/generated).

See [`proto/README.md`](../proto/README.md) and
[`astria-proto/README.md`](../crates/astria-proto/README.md) for how to use this
tool.
