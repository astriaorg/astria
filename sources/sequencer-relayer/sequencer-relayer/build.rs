use const_format::concatcp;
fn main() {
    const PROTO_ROOT: &str = concat!(env!("CARGO_WORKSPACE_DIR"), "/proto/");
    println!("cargo:rerun-if-changed={PROTO_ROOT}");
    prost_build::compile_protos(
        &[
            concatcp!(PROTO_ROOT, "msg.proto"),
            concatcp!(PROTO_ROOT, "tx.proto"),
        ],
        &[PROTO_ROOT],
    )
    .unwrap();
}
