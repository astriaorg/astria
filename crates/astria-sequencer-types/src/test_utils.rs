use tendermint::block::Header;

#[allow(clippy::missing_panics_doc)]
#[must_use]
/// Returns a default tendermint block header for test purposes.
pub fn default_header() -> Header {
    use tendermint::{
        account,
        block::{
            header::Version,
            Height,
        },
        chain,
        hash::AppHash,
        Hash,
        Time,
    };

    Header {
        version: Version {
            block: 0,
            app: 0,
        },
        chain_id: chain::Id::try_from("test").unwrap(),
        height: Height::from(1u32),
        time: Time::now(),
        last_block_id: None,
        last_commit_hash: None,
        data_hash: None,
        validators_hash: Hash::Sha256([0; 32]),
        next_validators_hash: Hash::Sha256([0; 32]),
        consensus_hash: Hash::Sha256([0; 32]),
        app_hash: AppHash::try_from([0; 32].to_vec()).unwrap(),
        last_results_hash: None,
        evidence_hash: None,
        proposer_address: account::Id::try_from([0u8; 20].to_vec()).unwrap(),
    }
}
