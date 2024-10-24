use std::{
    collections::HashMap,
    future::ready,
};

use astria_core::{
    generated::protocol::transaction::v1::TransactionBody as RawBody,
    primitive::v1::asset::{
        self,
        Denom,
    },
    protocol::{
        abci::AbciErrorCode,
        asset::v1::AllowedFeeAssetsResponse,
        transaction::v1::{
            action::{
                BridgeLock,
                BridgeSudoChange,
                BridgeUnlock,
                FeeAssetChange,
                FeeChange,
                IbcRelayerChange,
                IbcSudoChange,
                Ics20Withdrawal,
                InitBridgeAccount,
                RollupDataSubmission,
                SudoAddressChange,
                Transfer,
                ValidatorUpdate,
            },
            Action,
            TransactionBody,
        },
    },
    Protobuf,
};
use astria_eyre::eyre::{
    self,
    OptionExt as _,
    WrapErr as _,
};
use cnidarium::{
    StateRead,
    Storage,
};
use futures::{
    FutureExt as _,
    StreamExt as _,
};
use penumbra_ibc::IbcRelay;
use prost::{
    Message as _,
    Name as _,
};
use tendermint::abci::{
    request,
    response,
    Code,
};
use tokio::{
    sync::OnceCell,
    try_join,
};
use tracing::{
    instrument,
    warn,
};

use super::{
    access::FeeComponents,
    FeeHandler,
    StateReadExt as _,
};
use crate::{
    app::StateReadExt as _,
    assets::StateReadExt as _,
};

async fn find_trace_prefixed_or_return_ibc<S: StateRead>(
    state: S,
    asset: asset::IbcPrefixed,
) -> asset::Denom {
    state
        .map_ibc_to_trace_prefixed_asset(&asset)
        .await
        .wrap_err("failed to get ibc asset denom")
        .and_then(|maybe_asset| {
            maybe_asset.ok_or_eyre("ibc-prefixed asset did not have an entry in state")
        })
        .map_or_else(|_| asset.into(), Into::into)
}

#[instrument(skip_all)]
async fn get_allowed_fee_assets<S: StateRead>(state: &S) -> Vec<Denom> {
    let stream = state
        .allowed_fee_assets()
        .filter_map(|asset| {
            ready(
                asset
                    .inspect_err(|error| warn!(%error, "encountered issue reading allowed assets"))
                    .ok(),
            )
        })
        .map(|asset| find_trace_prefixed_or_return_ibc(state, asset))
        .buffered(16);
    stream.collect::<Vec<_>>().await
}

pub(crate) async fn allowed_fee_assets_request(
    storage: Storage,
    request: request::Query,
    _params: Vec<(String, String)>,
) -> response::Query {
    // get last snapshot
    let snapshot = storage.latest_snapshot();

    let height = async {
        snapshot
            .get_block_height()
            .await
            .wrap_err("failed getting block height")
    };
    let fee_assets = get_allowed_fee_assets(&snapshot).map(Ok);
    let (height, fee_assets) = match try_join!(height, fee_assets) {
        Ok(vals) => vals,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("{err:#}"),
                ..response::Query::default()
            };
        }
    };

    let payload = AllowedFeeAssetsResponse {
        height,
        fee_assets: fee_assets.into_iter().map(Into::into).collect(),
    }
    .into_raw()
    .encode_to_vec()
    .into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: tendermint::abci::Code::Ok,
        key: request.path.into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

pub(crate) async fn transaction_fee_request(
    storage: Storage,
    request: request::Query,
    _params: Vec<(String, String)>,
) -> response::Query {
    use astria_core::protocol::fees::v1::TransactionFeeResponse;

    let tx = match preprocess_fees_request(&request) {
        Ok(tx) => tx,
        Err(err_rsp) => return err_rsp,
    };

    // use latest snapshot, as this is a query for a transaction fee
    let snapshot = storage.latest_snapshot();
    let height = match snapshot.get_block_height().await {
        Ok(height) => height,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed getting block height: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let fees_with_ibc_denoms = match get_fees_for_transaction(&tx, &snapshot).await {
        Ok(fees) => fees,
        Err(err) => {
            return response::Query {
                code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                info: AbciErrorCode::INTERNAL_ERROR.info(),
                log: format!("failed calculating fees for provided transaction: {err:#}"),
                ..response::Query::default()
            };
        }
    };

    let mut fees = Vec::with_capacity(fees_with_ibc_denoms.len());
    for (ibc_denom, value) in fees_with_ibc_denoms {
        let trace_denom = match snapshot.map_ibc_to_trace_prefixed_asset(&ibc_denom).await {
            Ok(Some(trace_denom)) => trace_denom,
            Ok(None) => {
                return response::Query {
                    code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                    info: AbciErrorCode::INTERNAL_ERROR.info(),
                    log: format!(
                        "failed mapping ibc denom to trace denom: {ibc_denom}; asset does not \
                         exist in state"
                    ),
                    ..response::Query::default()
                };
            }
            Err(err) => {
                return response::Query {
                    code: Code::Err(AbciErrorCode::INTERNAL_ERROR.value()),
                    info: AbciErrorCode::INTERNAL_ERROR.info(),
                    log: format!("failed mapping ibc denom to trace denom: {err:#}"),
                    ..response::Query::default()
                };
            }
        };
        fees.push((trace_denom.into(), value));
    }

    let resp = TransactionFeeResponse {
        height,
        fees,
    };

    let payload = resp.into_raw().encode_to_vec().into();

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: 0.into(),
        key: request.path.into_bytes().into(),
        value: payload,
        height,
        ..response::Query::default()
    }
}

#[instrument(skip_all)]
pub(crate) async fn get_fees_for_transaction<S: StateRead>(
    tx: &TransactionBody,
    state: &S,
) -> eyre::Result<HashMap<asset::IbcPrefixed, u128>> {
    let transfer_fees = OnceCell::new();
    let rollup_data_submission_fees = OnceCell::new();
    let ics20_withdrawal_fees = OnceCell::new();
    let init_bridge_account_fees = OnceCell::new();
    let bridge_lock_fees = OnceCell::new();
    let bridge_unlock_fees = OnceCell::new();
    let bridge_sudo_change_fees = OnceCell::new();
    let validator_update_fees = OnceCell::new();
    let sudo_address_change_fees = OnceCell::new();
    let ibc_sudo_change_fees = OnceCell::new();
    let ibc_relay_fees = OnceCell::new();
    let ibc_relayer_change_fees = OnceCell::new();
    let fee_asset_change_fees = OnceCell::new();
    let fee_change_fees = OnceCell::new();

    let mut fees_by_asset = HashMap::new();
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                let transfer_fees =
                    get_or_init_fees::<Transfer, _, _>(state, &transfer_fees, "Transfer").await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    transfer_fees,
                );
            }
            Action::RollupDataSubmission(act) => {
                let rollup_data_submission_fees = get_or_init_fees::<RollupDataSubmission, _, _>(
                    state,
                    &rollup_data_submission_fees,
                    "RollupDataSubmission",
                )
                .await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    rollup_data_submission_fees,
                );
            }
            Action::Ics20Withdrawal(act) => {
                let ics20_withdrawal_fees = get_or_init_fees::<Ics20Withdrawal, _, _>(
                    state,
                    &ics20_withdrawal_fees,
                    "Ics20Withdrawal",
                )
                .await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    ics20_withdrawal_fees,
                );
            }
            Action::InitBridgeAccount(act) => {
                let init_bridge_account_fees = get_or_init_fees::<InitBridgeAccount, _, _>(
                    state,
                    &init_bridge_account_fees,
                    "InitBridgeAccount",
                )
                .await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    init_bridge_account_fees,
                );
            }
            Action::BridgeLock(act) => {
                let bridge_lock_fees =
                    get_or_init_fees::<BridgeLock, _, _>(state, &bridge_lock_fees, "BridgeLock")
                        .await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    bridge_lock_fees,
                );
            }
            Action::BridgeUnlock(act) => {
                let bridge_unlock_fees = get_or_init_fees::<BridgeUnlock, _, _>(
                    state,
                    &bridge_unlock_fees,
                    "BridgeUnlock",
                )
                .await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    bridge_unlock_fees,
                );
            }
            Action::BridgeSudoChange(act) => {
                let bridge_sudo_change_fees = get_or_init_fees::<BridgeSudoChange, _, _>(
                    state,
                    &bridge_sudo_change_fees,
                    "BridgeSudoChange",
                )
                .await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    bridge_sudo_change_fees,
                );
            }
            Action::ValidatorUpdate(_) => {
                get_or_init_fees::<ValidatorUpdate, _, _>(
                    state,
                    &validator_update_fees,
                    "ValidatorUpdate",
                )
                .await?;
            }
            Action::SudoAddressChange(_) => {
                get_or_init_fees::<SudoAddressChange, _, _>(
                    state,
                    &sudo_address_change_fees,
                    "SudoAddressChange",
                )
                .await?;
            }
            Action::IbcSudoChange(_) => {
                get_or_init_fees::<IbcSudoChange, _, _>(
                    state,
                    &ibc_sudo_change_fees,
                    "IbcSudoChange",
                )
                .await?;
            }
            Action::Ibc(_) => {
                get_or_init_fees::<IbcRelay, _, _>(state, &ibc_relay_fees, "IbcRelay").await?;
            }
            Action::IbcRelayerChange(_) => {
                get_or_init_fees::<IbcRelayerChange, _, _>(
                    state,
                    &ibc_relayer_change_fees,
                    "IbcRelayerChange",
                )
                .await?;
            }
            Action::FeeAssetChange(_) => {
                get_or_init_fees::<FeeAssetChange, _, _>(
                    state,
                    &fee_asset_change_fees,
                    "FeeAssetChange",
                )
                .await?;
            }
            Action::FeeChange(_) => {
                get_or_init_fees::<FeeChange, _, _>(state, &fee_change_fees, "FeeChange").await?;
            }
        }
    }
    Ok(fees_by_asset)
}

fn calculate_and_add_fees<T: FeeHandler, F: FeeComponents>(
    act: &T,
    fee_asset: asset::IbcPrefixed,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    fees: &F,
) {
    let base = fees.base();
    let multiplier = fees.multiplier();
    let total_fees = base.saturating_add(multiplier.saturating_mul(act.variable_component()));
    fees_by_asset
        .entry(fee_asset)
        .and_modify(|amt| *amt = amt.saturating_add(total_fees))
        .or_insert(total_fees);
}

fn preprocess_fees_request(request: &request::Query) -> Result<TransactionBody, response::Query> {
    let tx = match RawBody::decode(&*request.data) {
        Ok(tx) => tx,
        Err(err) => {
            return Err(response::Query {
                code: Code::Err(AbciErrorCode::BAD_REQUEST.value()),
                info: AbciErrorCode::BAD_REQUEST.info(),
                log: format!(
                    "failed to decode request data to a protobuf {}: {err:#}",
                    RawBody::full_name()
                ),
                ..response::Query::default()
            });
        }
    };

    let tx = match TransactionBody::try_from_raw(tx) {
        Ok(tx) => tx,
        Err(err) => {
            return Err(response::Query {
                code: Code::Err(AbciErrorCode::BAD_REQUEST.value()),
                info: AbciErrorCode::BAD_REQUEST.info(),
                log: format!(
                    "failed to convert raw proto unsigned transaction to native unsigned \
                     transaction: {err:#}"
                ),
                ..response::Query::default()
            });
        }
    };

    Ok(tx)
}

async fn get_or_init_fees<'a, Act, S, T>(
    state: &S,
    cell: &'a OnceCell<Option<T>>,
    action_name: &str,
) -> eyre::Result<&'a T>
where
    Act: FeeHandler<FeeComponents = T>,
    S: StateRead,
    T: FeeComponents,
{
    let fees = cell
        .get_or_try_init(|| async { Act::fee_components(state).await })
        .await
        .wrap_err(format!("failed to get fees for `{action_name}` action"))?
        .as_ref()
        .ok_or_eyre(format!(
            "fees not found for `{action_name}` action, hence it is disabled",
        ))?;
    Ok(fees)
}
