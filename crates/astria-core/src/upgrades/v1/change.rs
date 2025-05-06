use sha2::{
    Digest as _,
    Sha256,
};

use super::{
    ChangeHash,
    ChangeInfo,
    ChangeName,
};

/// A trait defining functionality common to all individual changes in a given upgrade.
pub trait Change: DeterministicSerialize {
    fn name(&self) -> ChangeName;

    fn activation_height(&self) -> u64;

    fn app_version(&self) -> u64;

    fn calculate_hash(&self) -> ChangeHash {
        ChangeHash::new(Sha256::digest(self.to_vec()).into())
    }

    fn info(&self) -> ChangeInfo {
        ChangeInfo {
            name: self.name(),
            activation_height: self.activation_height(),
            app_version: self.app_version(),
            hash: self.calculate_hash(),
        }
    }
}

pub trait DeterministicSerialize {
    fn to_vec(&self) -> Vec<u8>;
}

impl<T: borsh::BorshSerialize> DeterministicSerialize for T {
    fn to_vec(&self) -> Vec<u8> {
        borsh::to_vec(self).expect("should borsh-encode")
    }
}
