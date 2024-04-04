use std::path::PathBuf;

fn main() {
    let root_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("astria-grpc-mock-test");
    let protos = &[root_dir.join("proto/health.proto")];
    let includes = &[root_dir.join("proto")];

    let out_dir = root_dir.join("src/generated");
    let file_descriptor_set_path = root_dir.join("src/generated/grpc_health_v1.bin");

    let mut prost_config = prost_build::Config::new();
    prost_config.enable_type_names();
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .use_arc_self(true)
        // override prost-types with pbjson-types
        .compile_well_known_types(true)
        .extern_path(".google.protobuf", "::pbjson_types")
        .file_descriptor_set_path(&file_descriptor_set_path)
        .out_dir(&out_dir)
        .compile_with_config(prost_config, protos, includes)
        .unwrap();

    let descriptor_set = std::fs::read(&file_descriptor_set_path).unwrap();

    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)
        .unwrap()
        .preserve_proto_field_names()
        .out_dir(&out_dir)
        // only add JSON to types required for the execution API for now
        .build(&["."])
        .unwrap();
}
