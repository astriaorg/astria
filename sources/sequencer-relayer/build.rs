fn main() {
    println!("cargo:rerun-if-changed=proto/");
    prost_build::compile_protos(&["proto/msg.proto", "proto/tx.proto"], &["proto/"]).unwrap();
}
