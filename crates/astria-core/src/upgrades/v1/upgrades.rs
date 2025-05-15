use std::{
    collections::BTreeMap,
    path::{
        Path,
        PathBuf,
    },
};

use super::{
    aspen,
    Aspen,
    Upgrade,
    UpgradeName,
};
use crate::{
    generated::upgrades::v1::Upgrades as RawUpgrades,
    Protobuf,
};

/// The collection of all upgrades applied and scheduled to be applied to the network, ordered by
/// activation height lowest to highest.
#[derive(Clone, Debug, Default)]
pub struct Upgrades(Vec<Upgrade>);

impl Upgrades {
    /// Returns a new `Upgrades` by reading the file at `path` and decoding from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if reading, parsing or converting from raw (protobuf) upgrades fails.
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let contents = std::fs::read(path.as_ref())
            .map_err(|source| Error::read_file(source, path.as_ref().to_path_buf()))?;
        let raw_upgrades = serde_json::from_slice::<RawUpgrades>(&contents)
            .map_err(|source| Error::json_decode(source, path.as_ref().to_path_buf()))?;
        Self::try_from_raw(raw_upgrades, path.as_ref())
    }

    #[cfg(any(feature = "test-utils", test))]
    pub(crate) fn from_raw(raw: RawUpgrades) -> Self {
        Self::try_from_raw(raw, Path::new("dummy")).unwrap()
    }

    fn try_from_raw(raw: RawUpgrades, path: &Path) -> Result<Self, Error> {
        // Collect the upgrades into a BTreeMap, keyed by activation height.  This results in an
        // ordered collection (oldest to newest) and ensures we don't have any duplicate upgrades
        // in the file.
        let mut upgrades = BTreeMap::new();

        if let Some(upgrade) = raw
            .aspen
            .map(|raw_aspen| {
                Aspen::try_from_raw(raw_aspen)
                    .map(Upgrade::Aspen)
                    .map_err(|source| Error::convert_aspen(source, path.to_path_buf()))
            })
            .transpose()?
        {
            let upgrade_name = upgrade.name();
            if let Some(existing_upgrade) = upgrades.insert(upgrade.activation_height(), upgrade) {
                return Err(Error::duplicate_upgrade(
                    existing_upgrade.activation_height(),
                    existing_upgrade.name(),
                    upgrade_name,
                ));
            }
        }

        Ok(Self(upgrades.into_values().collect()))
    }

    /// Returns a verbose JSON-encoded string of `self`.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn to_json_pretty(&self) -> Result<String, Error> {
        let raw_upgrades = RawUpgrades {
            aspen: self.aspen().map(Aspen::to_raw),
        };
        serde_json::to_string_pretty(&raw_upgrades).map_err(Error::json_encode)
    }

    #[must_use]
    pub fn aspen(&self) -> Option<&Aspen> {
        #[expect(
            clippy::unnecessary_find_map,
            reason = "we'll want `find_map` once we have more than one `Upgrade` variant"
        )]
        self.0.iter().find_map(|upgrade| match upgrade {
            Upgrade::Aspen(aspen) => Some(aspen),
        })
    }

    /// Returns an iterator over the upgrades, sorted by activation height, lowest to highest.
    pub fn iter(&self) -> impl Iterator<Item = &'_ Upgrade> {
        self.0.iter()
    }

    /// Returns the upgrade with the given activation height, or `None` if no such upgrade exists.
    #[must_use]
    pub fn upgrade_activating_at_height(&self, height: u64) -> Option<&Upgrade> {
        for upgrade in &self.0 {
            let activation_height = upgrade.activation_height();
            if activation_height == height {
                return Some(upgrade);
            }
            if activation_height > height {
                break;
            }
        }
        None
    }
}

/// An error when constructing or JSON-encoding an [`Upgrades`].
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct Error(ErrorKind);

impl Error {
    fn read_file(source: std::io::Error, path: PathBuf) -> Self {
        Self(ErrorKind::ReadFile {
            source,
            path,
        })
    }

    fn json_decode(source: serde_json::Error, path: PathBuf) -> Self {
        Self(ErrorKind::JsonDecode {
            source,
            path,
        })
    }

    fn json_encode(source: serde_json::Error) -> Self {
        Self(ErrorKind::JsonEncode {
            source,
        })
    }

    fn duplicate_upgrade(
        activation_height: u64,
        first_upgrade: UpgradeName,
        second_upgrade: UpgradeName,
    ) -> Self {
        Self(ErrorKind::DuplicateUpgrade {
            activation_height,
            first_upgrade,
            second_upgrade,
        })
    }

    fn convert_aspen(source: aspen::Error, path: PathBuf) -> Self {
        Self(ErrorKind::ConvertAspen {
            source,
            path,
        })
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("failed to read file at `{}`", .path.display())]
    ReadFile {
        source: std::io::Error,
        path: PathBuf,
    },

    #[error("failed to json-decode file at `{}`", .path.display())]
    JsonDecode {
        source: serde_json::Error,
        path: PathBuf,
    },

    #[error("failed to json-encode upgrades")]
    JsonEncode { source: serde_json::Error },

    #[error(
        "upgrades `{first_upgrade}` and `{second_upgrade}` both have activation height \
         {activation_height}"
    )]
    DuplicateUpgrade {
        activation_height: u64,
        first_upgrade: UpgradeName,
        second_upgrade: UpgradeName,
    },

    #[error("error converting `aspen` in `{}`", .path.display())]
    ConvertAspen { source: aspen::Error, path: PathBuf },
}
