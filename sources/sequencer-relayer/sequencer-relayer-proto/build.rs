fn main() {
    const PROTO_ROOT: &str = "./proto/";
    println!("cargo:rerun-if-changed={PROTO_ROOT}");
    prost_build::compile_protos(&["proto/msg.proto", "proto/tx.proto"], &[PROTO_ROOT]).unwrap();
}
