use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs::{
        read_dir,
        read_to_string,
        remove_file,
        write,
    },
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

use tempfile::tempdir;

const OUT_DIR: &str = "src/proto/generated";

const PROTO_DIR: &str = "proto/";
const INCLUDES: &[&str] = &[PROTO_DIR];

fn get_buf_from_env() -> PathBuf {
    let os_specific_hint = match env::consts::OS {
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
#[test]
fn build() {
    let before_build = build_content_map(OUT_DIR);

    let buf = get_buf_from_env();
    let mut cmd = Command::new(buf.clone());

    let buf_img = tempfile::NamedTempFile::new()
        .expect("should be able to create a temp file to hold the buf image file descriptor set");
    cmd.arg("build")
        .arg("--output")
        .arg(buf_img.path())
        .arg("--as-file-descriptor-set")
        .arg(".")
        .current_dir(env!("CARGO_MANIFEST_DIR"));

    match cmd.output() {
        Err(e) => {
            panic!(
                "failed creating file descriptor set from protobuf: failed to invoke buf (path: \
                 {buf:?}): {e:?}"
            );
        }
        Ok(output) if !output.status.success() => {
            panic!(
                "failed creating file descriptor set from protobuf: `buf` returned error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(_) => {}
    };

    let files = find_protos(PROTO_DIR);

    let out_dir =
        tempdir().expect("should be able to create a temp dir to store the generated files");

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .client_mod_attribute(".", "#[cfg(feature=\"client\")]")
        .server_mod_attribute(".", "#[cfg(feature=\"server\")]")
        .extern_path(".tendermint.abci", "::tendermint-proto::abci")
        .extern_path(".tendermint.crypto", "::tendermint-proto::crypto")
        .extern_path(".tendermint.version", "::tendermint-proto::version")
        .extern_path(".tendermint.types", "::tendermint-proto::types")
        .type_attribute(".astria.primitive.v1.Uint128", "#[derive(Copy)]")
        .out_dir(out_dir.path())
        .file_descriptor_set_path(buf_img.path())
        .skip_protoc_run()
        .compile(&files, INCLUDES)
        .expect("should be able to compile protobuf using tonic");

    let mut after_build = build_content_map(out_dir.path());
    clean_non_astria_code(&mut after_build);
    ensure_files_are_the_same(&before_build, after_build, OUT_DIR);
}

fn clean_non_astria_code(generated: &mut ContentMap) {
    let foreign_file_names: Vec<_> = generated
        .files
        .keys()
        .filter(|name| !name.starts_with("astria."))
        .cloned()
        .collect();
    for name in foreign_file_names {
        let _ = generated.codes.remove(&name);
        let file = generated
            .files
            .remove(&name)
            .expect("file should exist under the name");
        let _ = remove_file(file);
    }
}

fn find_protos<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    use walkdir::{
        DirEntry,
        WalkDir,
    };
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension() == Some(OsStr::new("proto")))
        .map(DirEntry::into_path)
        .collect()
}

fn ensure_files_are_the_same(before: &ContentMap, after: ContentMap, target_dir: &'static str) {
    if before.codes == after.codes {
        return;
    }

    assert!(
        env::var_os("CI").is_none(),
        "files compiled from protobuf have changed, but this is a CI environment. Rerun this test \
         locally and commit the changes."
    );

    for (name, content) in after.codes {
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

struct ContentMap {
    files: HashMap<String, PathBuf>,
    codes: HashMap<String, String>,
}

fn build_content_map(path: impl AsRef<Path>) -> ContentMap {
    let mut files = HashMap::new();
    let mut codes = HashMap::new();
    for entry in read_dir(path)
        .expect("should be able to read target folder for generated files")
        .flatten()
    {
        let path = entry.path();
        let name = path
            .file_name()
            .expect("generated file should have a file name")
            .to_string_lossy()
            .to_string();
        let contents = read_to_string(&path)
            .expect("should be able to read the contents of an existing generated file");
        files.insert(name.clone(), path);
        codes.insert(name.clone(), contents);
    }
    ContentMap {
        files,
        codes,
    }
}
