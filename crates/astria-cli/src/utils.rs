use std::{
    fs::File,
    path::Path,
};

use astria_core::{
    crypto::SigningKey,
    primitive::v1::{
        Address,
        AddressError,
    },
};
use color_eyre::eyre::{
    self,
    ensure,
    WrapErr as _,
};

const ALLOWED_MODE: u32 = 0o600;

/// Creates a bech32m Astria address using `prefix` and `bytes`.
///
/// Uses [`crate::consts::ASTRIA_ADDRESS_PREFIX`] if `prefix` is not set.
pub(crate) fn make_address(prefix: &str, bytes: &[u8]) -> Result<Address, AddressError> {
    Address::builder().prefix(prefix).slice(bytes).try_build()
}

pub(crate) fn read_signing_key<P: AsRef<Path>>(path: P) -> eyre::Result<SigningKey> {
    fn read_inner(path: &Path) -> eyre::Result<SigningKey> {
        let file = File::open(path).wrap_err("failed to open file for reading")?;
        ensure_file_permissions_0o600(&file)
            .wrap_err("refusing to read signing key with unsafe permissions")?;
        let hex =
            std::fs::read_to_string(path).wrap_err("failed to read file contents into buffer")?;
        let bytes = hex::decode(hex.trim()).wrap_err("failed to decode file contents as hex")?;
        SigningKey::try_from(&*bytes).wrap_err("failed to construct signing key hex-decoded bytes")
    }

    read_inner(path.as_ref()).wrap_err_with(|| {
        format!(
            "failed reading signing key from path `{}`",
            path.as_ref().display()
        )
    })
}

pub(crate) fn create_file_with_permissions_0o600<P: AsRef<Path>>(path: P) -> eyre::Result<File> {
    fn create_inner(path: &Path) -> std::io::Result<File> {
        use std::os::unix::fs::OpenOptionsExt as _;
        File::options()
            .write(true)
            .create_new(true)
            .mode(ALLOWED_MODE)
            .open(path)
    }
    create_inner(path.as_ref())
        .wrap_err_with(|| format!("failed creating a file at `{}`", path.as_ref().display()))
}

fn ensure_file_permissions_0o600(file: &File) -> eyre::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = file
        .metadata()
        .wrap_err("failed to read file metadata")?
        .permissions()
        .mode()
        & 0o777; // mask the perm bits to ignore sticky bits, node/directory bits, etc
    ensure!(
        mode == ALLOWED_MODE,
        "only permissions `{ALLOWED_MODE:0o}` are permitted, but file had `{mode:0o}`",
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::ensure_file_permissions_0o600;

    #[test]
    fn file_with_0600_is_permitted() {
        let file = tempfile::tempfile().unwrap();
        let metadata = file.metadata().unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);

        ensure_file_permissions_0o600(&file)
            .expect("file with permissions 600 should be permitted");
    }
}
