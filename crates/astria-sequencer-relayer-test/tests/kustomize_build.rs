use std::{
    env,
    fs,
    path::{
        Path,
        PathBuf,
    },
    process::{
        exit,
        Command,
    },
};

const KUSTOMIZE_DIR: &str = "kubernetes/";
const TEST_ENV_KUBE_YAML: &str = "kubernetes/test-environment.yml";

fn kubectl_from_env() -> PathBuf {
    let os_specific_hint = if cfg!(target_os = "macos") {
        "You could try running `brew install kubectl` or follow the official guide at https://kubernetes.io/docs/tasks/tools/install-kubectl-macos/"
    } else if cfg!(target_os = "linux") {
        "If you're on Arch Linux you could running `sudo pacman -S kubectl`. or follow the official guide at https://kubernetes.io/docs/tasks/tools/install-kubectl-linux/"
    } else {
        "You could try installing it by following the official guide at https://kubernetes.io/docs/tasks/tools/#kubectl"
    };
    let error_msg = "Could not find `kubectl` installation and this build crate cannot proceed \
                     without
    this knowledge. If `kubectl` is installed and this crate had trouble finding
    it, you can set the `KUBECTL` environment variable with the specific path to your
    installed `kubectl` binary.";
    let msg = format!("{error_msg} {os_specific_hint}");

    env::var_os("KUBECTL")
        .map(PathBuf::from)
        .or_else(|| which::which("kubectl").ok())
        .expect(&msg)
}

#[test]
fn run_kustomize() {
    let kubectl = kubectl_from_env();
    let out_file = tempfile::NamedTempFile::new()
        .expect("failed creating a temporary file to store the output of kubectl kustomize");
    let mut cmd = Command::new(kubectl.clone());
    cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("kustomize")
        .arg(KUSTOMIZE_DIR)
        .arg("-o")
        .arg(out_file.path());

    match cmd.output() {
        Err(e) => {
            eprintln!("failed to invoke kubectl (path: {kubectl:?}): {e:?}");
            exit(e.raw_os_error().unwrap_or(-1));
        }
        Ok(output) if !output.status.success() => {
            eprintln!(
                "kubectl kustomize failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            exit(output.status.code().unwrap_or(-1));
        }
        Ok(_) => {}
    }

    let before = read_content(TEST_ENV_KUBE_YAML);
    let after = read_content(out_file.path());

    ensure_files_are_same(before, after);
}

fn read_content(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).expect("cannot read from existing generated kube yml file")
}

fn ensure_files_are_same(before: String, after: String) {
    if before == after {
        return;
    }

    if env::var("CI").is_ok() {
        panic!(
            "generated kube yaml file has changed but it's a CI environment; please rerun this \
             test locally and commit the changes"
        );
    }

    fs::write(TEST_ENV_KUBE_YAML, after).expect(
        "cannot write generated kube yaml file to its target; if this is happening in a CI \
         environment rerun the test locally and commit the changes",
    );

    panic!("generated file has changed; commit the changed files and rerun the test");
}
