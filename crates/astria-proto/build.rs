use std::{
    env,
    path::PathBuf,
    process::{
        exit,
        Command,
    },
};

fn buf_from_env() -> PathBuf {
    let os_specific_hint = if cfg!(target_os = "macos") {
        "You could try running `brew install buf` or downloading a recent release from https://github.com/bufbuild/buf/releases"
    } else if cfg!(target_os = "linux") {
        "If you're on Arch Linux you could try installing it from the AUR with `rua install buf` or another AUR helper, or download it from https://github.com/bufbuild/buf/releases"
    } else {
        "You can download it from https://github.com/bufbuild/buf/releases or from your package \
         manager."
    };
    let error_msg = "Could not find `buf` installation and this build crate cannot proceed without
    this knowledge. If `buf` is installed and this crate had trouble finding
    it, you can set the `BUF` environment variable with the specific path to your
    installed `buf` binary.";
    let msg = format!("{error_msg} {os_specific_hint}");

    env::var_os("BUF")
        .map(PathBuf::from)
        .or_else(|| which::which("buf").ok())
        .expect(&msg)
}

fn main() {
    println!("cargo:rerun-if-changed=proto/");

    let buf = buf_from_env();

    // Run the `buf generate` command to generate the Rust files
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR env var must be set by cargo");
    let mut cmd = Command::new(buf.clone());
    cmd.arg("generate")
        .arg("--output")
        .arg(out_dir)
        .arg("--template")
        .arg("buf.gen.yaml")
        .current_dir(env!("CARGO_MANIFEST_DIR"));

    match cmd.output() {
        Err(e) => {
            eprintln!("failed to invoke buf (path: {buf:?}): {e:?}");
            exit(e.raw_os_error().unwrap_or(-1));
        }
        Ok(output) if !output.status.success() => {
            eprintln!("buf failed: {}", String::from_utf8_lossy(&output.stderr));
            exit(output.status.code().unwrap_or(-1));
        }
        Ok(_) => {}
    };
}
