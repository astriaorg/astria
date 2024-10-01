use astria_core::{
    primitive::v1::Address,
    protocol::abci::AbciErrorCode,
};
use astria_eyre::eyre::WrapErr as _;
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
