use std::process::Command;

fn main() {
    // Define the directory where your .proto files are stored
    let proto_dir = "proto";

    // Define the output directory for the generated files
    let output_dir = ".";

    // Create the output directory if it doesn't exist
    std::fs::create_dir_all(&output_dir).unwrap();

    // Run the `buf generate` command to generate the Rust files
    let status = Command::new("buf")
        .arg("generate")
        .arg(proto_dir)
        .arg("--output")
        .arg(output_dir)
        .status()
        .expect("Failed to run 'buf generate'. Is the buf cli installed?");

    if !status.success() {
        panic!("'buf generate' exited with an error: {:?}", status.code());
    }

    // Re-run the build script if any .proto files change
    println!("cargo:rerun-if-changed={}", proto_dir);
}
