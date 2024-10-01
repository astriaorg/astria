use astria_core::{
    primitive::v1::ADDRESS_LEN,
    protocol::abci::AbciErrorCode,
};
use cnidarium::Storage;
use tendermint::abci::{
    request,
    response,
    Code,
};

use crate::{
    accounts::AddressBytes,
    authority::state_ext::StateReadExt as _,
};

pub(crate) async fn validator_name_request(
    storage: Storage,
    request: request::Query,
    params: Vec<(String, String)>,
) -> response::Query {
    let address = match preprocess_request(&params) {
        Ok(address) => address,
        Err(err) => return err,
    };

    let snapshot = storage.latest_snapshot();

    let validator_set = match snapshot.get_validator_set().await {
        Ok(validator_set) => validator_set,
        Err(err) => {
            return error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get validator set",
            );
        }
    };

    if validator_set.get(address.address_bytes()).is_none() {
        return error_query_response(
            None,
            AbciErrorCode::VALUE_NOT_FOUND,
            "validator address not found in validator set",
        );
    }

    let validator_names = match snapshot.get_validator_names().await {
        Ok(names) => names,
        Err(err) => {
            return error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get validator names",
            );
        }
    };

    match validator_names.get(&hex::encode(address.address_bytes())) {
        Some(name) => response::Query {
            code: Code::Ok,
            key: request.path.clone().into_bytes().into(),
            value: name.clone().into_bytes().into(),
            ..response::Query::default()
        },
        None => error_query_response(
            None,
            AbciErrorCode::VALUE_NOT_FOUND,
            "validator address exists but does not have a name",
        ),
    }
}

fn preprocess_request(params: &[(String, String)]) -> Result<[u8; ADDRESS_LEN], response::Query> {
    let Some(address) = params
        .iter()
        .find_map(|(k, v)| (k == "address").then_some(v))
    else {
        return Err(error_query_response(
            None,
            AbciErrorCode::INVALID_PARAMETER,
            "path did not contain address parameter",
        ));
    };

    let address_bytes_vec = match hex::decode(address) {
        Ok(address_bytes) => address_bytes,
        Err(err) => {
            return Err(error_query_response(
                Some(err.into()),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to decode address from hex",
            ));
        }
    };

    let address_bytes: [u8; ADDRESS_LEN] = match address_bytes_vec.try_into() {
        Ok(address_bytes) => address_bytes,
        Err(_) => {
            return Err(error_query_response(
                None,
                AbciErrorCode::INVALID_PARAMETER,
                "address was not the correct length",
            ));
        }
    };

    Ok(address_bytes)
}

fn error_query_response(
    err: Option<astria_eyre::eyre::Error>,
    code: AbciErrorCode,
    msg: &str,
) -> response::Query {
    let log = match err {
        Some(err) => format!("{msg}: {err:#}"),
        None => msg.into(),
    };
    response::Query {
        code: Code::Err(code.value()),
        info: code.info(),
        log,
        ..response::Query::default()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        vec,
    };

    use astria_core::protocol::transaction::v1alpha1::action::{
        ValidatorUpdate,
        ValidatorUpdateWithName,
    };
    use cnidarium::StateDelta;
    use tendermint::abci::request;

    use crate::{
        authority::{
            query::validator_name_request,
            StateReadExt,
            StateWriteExt,
            ValidatorSet,
        },
        test_utils::verification_key,
    };

    #[tokio::test]
    async fn validator_name_request_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let verification_key = verification_key(1);
        let key_address = *verification_key.clone().address_bytes();
        let validator_name = "test".to_string();

        let inner_update = ValidatorUpdate {
            power: 100,
            verification_key,
        };
        let update_with_name = ValidatorUpdateWithName {
            validator_update: inner_update.clone(),
            name: validator_name.clone(),
        };

        let mut validator_names = state.get_validator_names().await.unwrap();
        assert_eq!(validator_names.len(), 0);

        let inner_validator_map = BTreeMap::new();
        let mut validator_set = ValidatorSet::new(inner_validator_map);
        assert_eq!(validator_set.len(), 0);

        validator_names.insert(hex::encode(key_address), update_with_name.name.clone());
        validator_set.push_update(inner_update);

        state.put_validator_names(validator_names).unwrap();
        state.put_validator_set(validator_set).unwrap();
        storage.commit(state).await.unwrap();

        let query = request::Query {
            data: vec![].into(),
            path: "path".to_string(),
            height: 0u32.into(),
            prove: false,
        };

        let params = vec![("address".to_string(), hex::encode(key_address))];

        let rsp = validator_name_request(storage.clone(), query, params).await;
        assert_eq!(rsp.code, 0.into(), "{}", rsp.log);
        assert_eq!(rsp.key, "path".as_bytes());
        assert_eq!(rsp.value, validator_name);
    }
}
