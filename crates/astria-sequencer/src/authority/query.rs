use astria_core::{
    primitive::v1::Address,
    protocol::abci::AbciErrorCode,
};
use astria_eyre::eyre::Context as _;
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

    match snapshot.get_validator_name(address.as_bytes()).await {
        Ok(Some(name)) => {
            return response::Query {
                code: Code::Ok,
                key: request.path.clone().into_bytes().into(),
                value: name.clone().into_bytes().into(),
                ..response::Query::default()
            };
        }
        Ok(None) => {}
        Err(err) => {
            return error_query_response(
                Some(err),
                AbciErrorCode::INTERNAL_ERROR,
                "failed to get validator names",
            );
        }
    };

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
        error_query_response(
            None,
            AbciErrorCode::VALUE_NOT_FOUND,
            "validator address not found in validator set",
        )
    } else {
        error_query_response(
            None,
            AbciErrorCode::VALUE_NOT_FOUND,
            "validator address exists but does not have a name",
        )
    }
}

fn preprocess_request(params: &[(String, String)]) -> Result<Address, response::Query> {
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
    let address = address
        .parse()
        .wrap_err("failed to parse argument as address")
        .map_err(|err| response::Query {
            code: Code::Err(AbciErrorCode::INVALID_PARAMETER.value()),
            info: AbciErrorCode::INVALID_PARAMETER.info(),
            log: format!("address could not be constructed from provided parameter: {err:#}"),
            ..response::Query::default()
        })?;
    Ok(address)
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

    use astria_core::protocol::{
        abci::AbciErrorCode,
        transaction::v1::action::{
            ValidatorUpdate,
            ValidatorUpdateV2,
        },
    };
    use cnidarium::StateDelta;
    use tendermint::abci::request;

    use crate::{
        authority::{
            query::validator_name_request,
            StateWriteExt,
            ValidatorSet,
        },
        benchmark_and_test_utils::{
            astria_address,
            verification_key,
        },
    };

    #[tokio::test]
    async fn validator_name_request_works_as_expected() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let verification_key = verification_key(1);
        let key_address_bytes = *verification_key.clone().address_bytes();
        let validator_name = "test".to_string();

        let update_with_name = ValidatorUpdateV2 {
            name: validator_name.clone(),
            power: 100,
            verification_key,
        };

        state
            .put_validator_name(&key_address_bytes, update_with_name.name)
            .unwrap();
        storage.commit(state).await.unwrap();

        let query = request::Query {
            data: vec![].into(),
            path: "path".to_string(),
            height: 0u32.into(),
            prove: false,
        };
        let params = vec![(
            "address".to_string(),
            astria_address(&key_address_bytes).to_string(),
        )];

        let rsp = validator_name_request(storage.clone(), query, params).await;
        assert!(rsp.code.is_ok(), "code: {:?}, log: {}", rsp.code, rsp.log);
        assert_eq!(rsp.key, "path".as_bytes());
        assert_eq!(rsp.value, validator_name);
    }

    #[tokio::test]
    async fn validator_name_request_fails_if_not_in_validator_set() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let verification_key = verification_key(1);

        let query = request::Query {
            data: vec![].into(),
            path: "path".to_string(),
            height: 0u32.into(),
            prove: false,
        };

        // Use a different address than the one submitted to the validator set
        let params = vec![(
            "address".to_string(),
            astria_address(&[0u8; 20]).to_string(),
        )];

        let inner_update = ValidatorUpdate {
            power: 100,
            verification_key,
        };

        let rsp = validator_name_request(storage.clone(), query.clone(), params.clone()).await;
        assert_eq!(
            rsp.code.value(),
            u32::from(AbciErrorCode::INTERNAL_ERROR.value()),
            "{}",
            rsp.log
        );
        let err_msg = "failed to get validator set: validator set not found";
        assert_eq!(rsp.log, err_msg);

        let inner_validator_map = BTreeMap::new();
        let mut validator_set = ValidatorSet::new(inner_validator_map);
        assert_eq!(validator_set.len(), 0);
        validator_set.insert(inner_update);
        state.put_validator_set(validator_set).unwrap();
        storage.commit(state).await.unwrap();

        let rsp = validator_name_request(storage.clone(), query, params).await;
        assert_eq!(
            rsp.code.value(),
            u32::from(AbciErrorCode::VALUE_NOT_FOUND.value()),
            "{}",
            rsp.log
        );
        let err_msg = "validator address not found in validator set";
        assert_eq!(rsp.log, err_msg);
    }

    #[tokio::test]
    async fn validator_name_request_fails_if_validator_has_no_name() {
        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = StateDelta::new(snapshot);

        let verification_key = verification_key(1);
        let key_address_bytes = *verification_key.clone().address_bytes();
        let inner_update = ValidatorUpdate {
            power: 100,
            verification_key,
        };

        let inner_validator_map = BTreeMap::new();
        let mut validator_set = ValidatorSet::new(inner_validator_map);
        assert_eq!(validator_set.len(), 0);
        validator_set.insert(inner_update);
        state.put_validator_set(validator_set).unwrap();
        storage.commit(state).await.unwrap();

        let query = request::Query {
            data: vec![].into(),
            path: "path".to_string(),
            height: 0u32.into(),
            prove: false,
        };

        let params = vec![(
            "address".to_string(),
            astria_address(&key_address_bytes).to_string(),
        )];
        let rsp = validator_name_request(storage.clone(), query, params).await;
        assert_eq!(
            rsp.code.value(),
            u32::from(AbciErrorCode::VALUE_NOT_FOUND.value()),
            "{}",
            rsp.log
        );
        let err_msg = "validator address exists but does not have a name";
        assert_eq!(rsp.log, err_msg);
    }
}
