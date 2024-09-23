pub(crate) mod api {
    use astria_core::primitive::v1::RollupId;

    pub(crate) fn block_hash_by_height_key(height: u64) -> Vec<u8> {
        [b"blockhash/".as_slice(), &height.to_le_bytes()].concat()
    }

    pub(crate) fn sequencer_block_header_by_hash_key(hash: &[u8]) -> Vec<u8> {
        [b"blockheader/", hash].concat()
    }

    pub(crate) fn rollup_data_by_hash_and_rollup_id_key(
        hash: &[u8],
        rollup_id: &RollupId,
    ) -> Vec<u8> {
        [b"rollupdata/", hash, rollup_id.as_ref()].concat()
    }

    pub(crate) fn rollup_ids_by_hash_key(hash: &[u8]) -> Vec<u8> {
        [b"rollupids/", hash].concat()
    }

    pub(crate) fn rollup_transactions_proof_by_hash_key(hash: &[u8]) -> Vec<u8> {
        [b"rolluptxsproof/", hash].concat()
    }

    pub(crate) fn rollup_ids_proof_by_hash_key(hash: &[u8]) -> Vec<u8> {
        [b"rollupidsproof/", hash].concat()
    }

    #[cfg(test)]
    mod tests {
        use telemetry::display::base64;

        use super::*;

        #[test]
        fn keys_should_not_change() {
            insta::assert_snapshot!(base64(&block_hash_by_height_key(42)));

            let hash = [1; 32];
            insta::assert_snapshot!(base64(&sequencer_block_header_by_hash_key(&hash)));

            let rollup_id = RollupId::new([2; 32]);
            insta::assert_snapshot!(base64(&rollup_data_by_hash_and_rollup_id_key(
                &hash, &rollup_id
            )));

            insta::assert_snapshot!(base64(&rollup_ids_by_hash_key(&hash)));

            insta::assert_snapshot!(base64(&rollup_transactions_proof_by_hash_key(&hash)));

            insta::assert_snapshot!(base64(&rollup_ids_proof_by_hash_key(&hash)));
        }
    }
}

pub(crate) mod app {
    pub(crate) fn storage_version_by_height_key(height: u64) -> Vec<u8> {
        format!("storage_version/{height}").into_bytes()
    }
}

pub(crate) mod assets {
    use astria_core::primitive::v1::asset::IbcPrefixed;
    use astria_eyre::eyre::{
        ContextCompat,
        Result,
        WrapErr,
    };

    use crate::storage::verifiable_keys::Asset;

    pub(crate) const BLOCK_FEES_PREFIX: &[u8] = b"block_fees/";
    pub(crate) const FEE_ASSET_PREFIX: &[u8] = b"fee_asset/";

    pub(crate) fn fee_asset_key<TAsset: Into<IbcPrefixed>>(asset: TAsset) -> Vec<u8> {
        [FEE_ASSET_PREFIX, Asset::from(asset).to_string().as_bytes()].concat()
    }

    pub(crate) fn block_fees_key<TAsset: Into<IbcPrefixed>>(asset: TAsset) -> Vec<u8> {
        [BLOCK_FEES_PREFIX, Asset::from(asset).to_string().as_bytes()].concat()
    }

    pub(crate) fn extract_asset_from_fee_asset_key(key: &[u8]) -> Result<IbcPrefixed> {
        extract_asset_from_key(key, FEE_ASSET_PREFIX)
            .wrap_err("failed to extract asset from fee asset key")
    }

    pub(crate) fn extract_asset_from_block_fees_key(key: &[u8]) -> Result<IbcPrefixed> {
        extract_asset_from_key(key, BLOCK_FEES_PREFIX)
            .wrap_err("failed to extract asset from fee asset key")
    }

    pub(crate) fn extract_asset_from_key(key: &[u8], prefix: &[u8]) -> Result<IbcPrefixed> {
        let suffix = key.strip_prefix(prefix).wrap_err_with(|| {
            format!(
                "key {} did not have prefix {}",
                telemetry::display::hex(key),
                telemetry::display::hex(prefix)
            )
        })?;
        let asset = std::str::from_utf8(suffix)
            .wrap_err("key suffix was not utf8 encoded; this should not happen")?
            .parse::<Asset>()
            .wrap_err("failed to parse storage key suffix as address hunk")?
            .get();
        Ok(asset)
    }

    #[cfg(test)]
    mod tests {
        use astria_core::primitive::v1::asset::Denom;

        use super::*;

        #[test]
        fn keys_should_not_change() {
            let trace_prefixed = "a/denom/with/a/prefix".parse::<Denom>().unwrap();
            insta::assert_snapshot!(std::str::from_utf8(&fee_asset_key(&trace_prefixed)).unwrap());
            insta::assert_snapshot!(std::str::from_utf8(&block_fees_key(&trace_prefixed)).unwrap());
        }

        #[test]
        fn should_extract_asset_from_key() {
            let asset = IbcPrefixed::new([1; 32]);

            let key = fee_asset_key(asset);
            let recovered_asset = extract_asset_from_fee_asset_key(&key).unwrap();
            assert_eq!(asset, recovered_asset);

            let key = block_fees_key(asset);
            let recovered_asset = extract_asset_from_block_fees_key(&key).unwrap();
            assert_eq!(asset, recovered_asset);
        }
    }
}

pub(crate) mod authority {
    pub(crate) const VALIDATOR_UPDATES_KEY: &[u8] = b"valupdates";
}

pub(crate) mod bridge {
    use astria_core::primitive::v1::RollupId;

    use crate::{
        accounts::AddressBytes,
        storage::verifiable_keys::{
            bridge::BRIDGE_ACCOUNT_PREFIX,
            AddressPrefixer,
        },
    };

    pub(crate) const DEPOSIT_PREFIX: &[u8] = b"deposit/";
    pub(crate) const DEPOSIT_NONCE_PREFIX: &[u8] = b"depositnonce/";

    pub(crate) fn deposit_key_prefix(rollup_id: &RollupId) -> Vec<u8> {
        [DEPOSIT_PREFIX, rollup_id.as_ref()].concat()
    }

    pub(crate) fn deposit_key(rollup_id: &RollupId, nonce: u32) -> Vec<u8> {
        [DEPOSIT_PREFIX, rollup_id.as_ref(), &nonce.to_le_bytes()].concat()
    }

    pub(crate) fn deposit_nonce_key(rollup_id: &RollupId) -> Vec<u8> {
        [DEPOSIT_NONCE_PREFIX, hex::encode(rollup_id).as_bytes()].concat()
    }

    pub(crate) fn last_transaction_id_for_bridge_account_key<T: AddressBytes>(
        address: &T,
    ) -> Vec<u8> {
        format!(
            "{}/lasttx",
            AddressPrefixer::new(BRIDGE_ACCOUNT_PREFIX, address)
        )
        .into_bytes()
    }

    #[cfg(test)]
    mod tests {
        use telemetry::display::base64;

        use super::*;

        #[test]
        fn keys_should_not_change() {
            let rollup_id = RollupId::new([1; 32]);
            insta::assert_snapshot!(base64(&deposit_key_prefix(&rollup_id)));
            insta::assert_snapshot!(base64(&deposit_key(&rollup_id, 42)));
            insta::assert_snapshot!(std::str::from_utf8(&deposit_nonce_key(&rollup_id)).unwrap());
            insta::assert_snapshot!(
                std::str::from_utf8(&last_transaction_id_for_bridge_account_key(&[2; 20])).unwrap()
            );
        }

        #[test]
        fn deposit_prefix_should_be_prefix_of_deposit_key() {
            let rollup_id = RollupId::new([1; 32]);
            let prefix = deposit_key_prefix(&rollup_id);
            let key = deposit_key(&rollup_id, 99);
            assert!(key.strip_prefix(prefix.as_slice()).is_some());
        }
    }
}
