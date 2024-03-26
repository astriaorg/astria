use std::{
    collections::{
        HashMap,
        HashSet,
    },
    env,
    ffi::OsStr,
    fs::{
        read_dir,
        read_to_string,
        remove_file,
    },
    io::Write as _,
    path::{
        Path,
        PathBuf,
    },
    process::Command,
};

const OUT_DIR: &str = "../../crates/astria-core/src/generated";
const SRC_DIR: &str = "../../proto";

const INCLUDES: &[&str] = &[SRC_DIR];

fn main() {
    let buf = get_buf_from_env();
    let mut cmd = Command::new(buf.clone());

    let buf_img = tempfile::NamedTempFile::new()
        .expect("should be able to create a temp file to hold the buf image file descriptor set");

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = crate_dir.join(SRC_DIR);
    let out_dir = crate_dir.join(OUT_DIR);

    cmd.arg("build")
        .arg("--output")
        .arg(buf_img.path())
        .arg("--as-file-descriptor-set");

    let buf_output = match cmd.output() {
        Err(e) => {
            panic!(
                "failed creating file descriptor set from protobuf: failed to invoke buf (path: \
                 {buf:?}): {e:?}"
            );
        }
        Ok(output) => output,
    };

    emit_buf_stdout(&buf_output.stdout).expect("able to write to stdout");
    emit_buf_stderr(&buf_output.stderr).expect("able to write to stderr");

    assert!(
        buf_output.status.success(),
        "failed creating file descriptor set from protobuf: `buf` returned non-zero exit code"
    );

    let files = find_protos(src_dir);

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .emit_rerun_if_changed(false)
        .bytes([".astria.execution.v1alpha2"])
        .client_mod_attribute(".", "#[cfg(feature=\"client\")]")
        .server_mod_attribute(".", "#[cfg(feature=\"server\")]")
        .extern_path(
            ".astria_vendored.tendermint.abci",
            "::tendermint-proto::abci",
        )
        .extern_path(
            ".astria_vendored.tendermint.crypto",
            "::tendermint-proto::crypto",
        )
        .extern_path(
            ".astria_vendored.tendermint.version",
            "::tendermint-proto::version",
        )
        .extern_path(
            ".astria_vendored.tendermint.types",
            "::tendermint-proto::types",
        )
        .extern_path(".astria_vendored.penumbra", "::penumbra-proto")
        .type_attribute(".astria.primitive.v1.Uint128", "#[derive(Copy)]")
        .use_arc_self(true)
        // override prost-types with pbjson-types
        .compile_well_known_types(true)
        .extern_path(".google.protobuf", "::pbjson_types")
        .file_descriptor_set_path(buf_img.path())
        .skip_protoc_run()
        .out_dir(&out_dir)
        .compile(&files, INCLUDES)
        .expect("should be able to compile protobuf using tonic");

    let descriptor_set = std::fs::read(buf_img.path())
        .expect("the buf image/descriptor set must exist and be readable at this point");

    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)
        .unwrap()
        .preserve_proto_field_names()
        .out_dir(&out_dir)
        // only add JSON to types required for the execution API for now
        .build(&[
            ".astria.execution.v1alpha2",
            ".astria.sequencer.v1.Deposit",
            ".astria.sequencer.v1.RollupData",
            ".astria.primitive.v1.Uint128",
        ])
        .unwrap();

    let mut after_build = build_content_map(&out_dir);
    clean_non_astria_code(&mut after_build);
}

fn emit_buf_stdout(buf: &[u8]) -> std::io::Result<()> {
    if !buf.is_empty() {
        std::io::stdout().lock().write_all(buf)?;
        println!();
    }
    Ok(())
}

fn emit_buf_stderr(buf: &[u8]) -> std::io::Result<()> {
    if !buf.is_empty() {
        std::io::stderr().lock().write_all(buf)?;
        eprintln!();
    }
    Ok(())
}

fn clean_non_astria_code(generated: &mut ContentMap) {
    let mut foreign_file_names: HashSet<_> = generated
        .files
        .keys()
        .filter(|name| !name.starts_with("astria."))
        .cloned()
        .collect();
    // also mask mod.rs because we need are defining it
    foreign_file_names.remove("mod.rs");
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
