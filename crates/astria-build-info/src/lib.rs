#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "runtime")]
pub use runtime::BuildInfo;

#[cfg(feature = "runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
mod runtime {
    /// Constructs a [`BuildInfo`] at compile time.
    #[macro_export]
    macro_rules! get {
        () => {
            BuildInfo {
                build_timestamp: env!("VERGEN_BUILD_TIMESTAMP"),
                cargo_opt_level: env!("VERGEN_CARGO_OPT_LEVEL"),
                cargo_pkg_name: env!("CARGO_PKG_NAME"),
                cargo_target_triple: env!("VERGEN_CARGO_TARGET_TRIPLE"),
                git_branch: env!("VERGEN_GIT_BRANCH"),
                git_commit_date: env!("VERGEN_GIT_COMMIT_DATE"),
                git_describe: env!("VERGEN_GIT_DESCRIBE"),
                git_sha: env!("VERGEN_GIT_SHA"),
                rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
                rustc_commit_hash: env!("VERGEN_RUSTC_COMMIT_HASH"),
                rustc_host_triple: env!("VERGEN_RUSTC_HOST_TRIPLE"),
            }
        };
    }

    /// The build info of a package constructed at compile time.
    ///
    /// This intended to be constructed at compile time using the
    /// [`get`] macro:
    ///
    /// ```no_run
    /// # use astria_build_info::BuildInfo;
    /// const BUILD_INFO: BuildInfo = astria_build_info::get!();
    /// ```
    #[derive(Debug, serde::Serialize)]
    pub struct BuildInfo {
        pub build_timestamp: &'static str,
        pub cargo_opt_level: &'static str,
        pub cargo_pkg_name: &'static str,
        pub cargo_target_triple: &'static str,
        pub git_branch: &'static str,
        pub git_commit_date: &'static str,
        pub git_describe: &'static str,
        pub git_sha: &'static str,
        pub rustc_channel: &'static str,
        pub rustc_commit_hash: &'static str,
        pub rustc_host_triple: &'static str,
    }
}

#[cfg(feature = "build")]
#[cfg_attr(docsrs, doc(cfg(feature = "build")))]
/// Emits build infos as environment variables.
///
/// The `prefix` argument is used akin to manually calling
/// `git describe --tags --match="<prefix>*". It assumes that releases of
/// services (and binaries) are tagged using a format like `<prefix>0.1.2`.
///
/// Note that if two services share a common prefix like `sequencer` and
/// `sequencer-relayer`, `<prefix>` should be supplied as `"sequencer-v"` and
/// `"sequencer-relayer-v"` for tags like `sequencer-v1.2.3` or
/// `sequencer-relayer-v1.2.3`. This is to avoid matching on the wrong prefix
pub fn emit(prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let git_describe_prefix = Box::leak(format!("{prefix}*").into_boxed_str());
    vergen::EmitBuilder::builder()
        .build_timestamp()
        .cargo_opt_level()
        .cargo_target_triple()
        .git_branch()
        .git_commit_date()
        .git_describe(true, true, Some(&*git_describe_prefix))
        .git_sha(false)
        .rustc_channel()
        .rustc_commit_hash()
        .rustc_host_triple()
        .rustc_semver()
        .emit()?;
    Ok(())
}
