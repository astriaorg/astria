use std::fmt::{
    self,
    Display,
    Formatter,
};

use super::{
    ChangeHash,
    ChangeName,
};
use crate::generated::sequencerblock::v1::get_upgrades_info_response as raw;

/// Brief details of a given upgrade change.
///
/// All upgrade changes provide these details at a minimum.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ChangeInfo {
    pub activation_height: u64,
    pub name: ChangeName,
    pub app_version: u64,
    pub hash: ChangeHash,
}

impl ChangeInfo {
    #[must_use]
    pub fn to_raw(&self) -> raw::ChangeInfo {
        raw::ChangeInfo {
            activation_height: self.activation_height,
            change_name: self.name.clone().into_string(),
            app_version: self.app_version,
            base64_hash: self.hash.to_string(),
        }
    }
}

impl Display for ChangeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "upgrade change `{}` with activation height {}, app version {}, change hash {}",
            self.name, self.activation_height, self.app_version, self.hash
        )
    }
}
