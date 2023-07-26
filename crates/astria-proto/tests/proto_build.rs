use std::{
    collections::HashMap,
    env::{
        self,
        consts::OS,
    },
    fs::{
        read_dir,
        read_to_string,
        write,
    },
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

use tempfile::tempdir;

const OUT_DIR: &str = "src/proto/tonic";

fn buf_from_env() -> PathBuf {
    let os_specific_hint = match OS {
        "macos" => "You could try running `brew install buf` or downloading a recent release from https://github.com/bufbuild/buf/releases",
        "linux" => "You can download it from https://github.com/bufbuild/buf/releases; if you are on Arch Linux, install it from the AUR with `rua install buf` or another helper",
        _other =>  "Check if there is a precompiled version for your OS at https://github.com/bufbuild/buf/releases"
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

fn build_content_map(path: impl AsRef<Path>) -> HashMap<String, String> {
    read_dir(path)
        .expect("should be able to read target folder for generated files")
        .flatten()
        .map(|entry| {
            let path = entry.path();
            let name = path
                .file_name()
                .expect("every generated file should have a file name")
                .to_string_lossy()
                .to_string();
            let contents = read_to_string(path)
                .expect("should be able to read the contents of an existing generated file");
            (name, contents)
        })
        .collect()
}

#[test]
fn build() {
    let before_build = build_content_map(OUT_DIR);
    let buf = buf_from_env();

    let out_dir =
        tempdir().expect("should be able to create a temp dir to store the generated files");
    let out_dir_str = out_dir
        .path()
        .to_str()
        .expect(
            "temp out dir should always be generated with valid utf8 encoded alphanumeric bytes",
        )
        .to_string();

    // Run the `buf generate` command to generate the Rust files
    let mut cmd = Command::new(buf.clone());
    cmd.arg("generate")
        .arg("--output")
        .arg(out_dir_str)
        .arg("--template")
        .arg("buf.gen.yaml")
        .current_dir(env!("CARGO_MANIFEST_DIR"));

    match cmd.output() {
        Err(e) => {
            panic!("failed compiling protobuf: failed to invoke buf (path: {buf:?}): {e:?}");
        }
        Ok(output) if !output.status.success() => {
            panic!(
                "failed compiling protobuf: `buf` returned error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(_) => {}
    };

    let after_build = build_content_map(out_dir.path().join(OUT_DIR));
    ensure_files_are_the_same(&before_build, after_build, OUT_DIR);
}

fn ensure_files_are_the_same(
    before: &HashMap<String, String>,
    after: HashMap<String, String>,
    target_dir: &'static str,
) {
    if before == &after {
        return;
    }

    assert!(
        env::var_os("CI").is_none(),
        "files compiled from protobuf have changed, but this is a CI environment. Rerun this test \
         locally and commit the changes."
    );

    for (name, content) in after {
        let dst = Path::new(target_dir).join(name);
        if let Err(e) = write(&dst, content) {
            panic!(
                "failed to write code generated from protobuf to `{}`; if this is a CI \
                 environment, rerun the test locally and commit the changes. Error: {e:?}",
                dst.display(),
            );
        }
    }

    panic!("the generated files have changed; please commit the changes");
}
