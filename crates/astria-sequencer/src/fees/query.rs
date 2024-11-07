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
        fees::v1::FeeComponents,
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
    Protobuf as _,
};
use astria_eyre::eyre::{
    self,
    eyre,
    ErrReport,
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

use crate::{
    app::StateReadExt as _,
    assets::StateReadExt as _,
    fees::{
        FeeHandler,
        StateReadExt as _,
    },
    storage::StoredValue,
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
    let transfer_fees: OnceCell<Option<FeeComponents<Transfer>>> = OnceCell::new();
    let rollup_data_submission_fees: OnceCell<Option<FeeComponents<RollupDataSubmission>>> =
        OnceCell::new();
    let ics20_withdrawal_fees: OnceCell<Option<FeeComponents<Ics20Withdrawal>>> = OnceCell::new();
    let init_bridge_account_fees: OnceCell<Option<FeeComponents<InitBridgeAccount>>> =
        OnceCell::new();
    let bridge_lock_fees: OnceCell<Option<FeeComponents<BridgeLock>>> = OnceCell::new();
    let bridge_unlock_fees: OnceCell<Option<FeeComponents<BridgeUnlock>>> = OnceCell::new();
    let bridge_sudo_change_fees: OnceCell<Option<FeeComponents<BridgeSudoChange>>> =
        OnceCell::new();
    let validator_update_fees: OnceCell<Option<FeeComponents<ValidatorUpdate>>> = OnceCell::new();
    let sudo_address_change_fees: OnceCell<Option<FeeComponents<SudoAddressChange>>> =
        OnceCell::new();
    let ibc_sudo_change_fees: OnceCell<Option<FeeComponents<IbcSudoChange>>> = OnceCell::new();
    let ibc_relay_fees: OnceCell<Option<FeeComponents<IbcRelay>>> = OnceCell::new();
    let ibc_relayer_change_fees: OnceCell<Option<FeeComponents<IbcRelayerChange>>> =
        OnceCell::new();
    let fee_asset_change_fees: OnceCell<Option<FeeComponents<FeeAssetChange>>> = OnceCell::new();
    let fee_change_fees: OnceCell<Option<FeeComponents<FeeChange>>> = OnceCell::new();

    let mut fees_by_asset = HashMap::new();
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                let transfer_fees = get_or_init_fees(state, &transfer_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    transfer_fees,
                );
            }
            Action::RollupDataSubmission(act) => {
                let rollup_data_submission_fees =
                    get_or_init_fees(state, &rollup_data_submission_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    rollup_data_submission_fees,
                );
            }
            Action::Ics20Withdrawal(act) => {
                let ics20_withdrawal_fees = get_or_init_fees(state, &ics20_withdrawal_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    ics20_withdrawal_fees,
                );
            }
            Action::InitBridgeAccount(act) => {
                let init_bridge_account_fees =
                    get_or_init_fees(state, &init_bridge_account_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    init_bridge_account_fees,
                );
            }
            Action::BridgeLock(act) => {
                let bridge_lock_fees = get_or_init_fees(state, &bridge_lock_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    bridge_lock_fees,
                );
            }
            Action::BridgeUnlock(act) => {
                let bridge_unlock_fees = get_or_init_fees(state, &bridge_unlock_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    bridge_unlock_fees,
                );
            }
            Action::BridgeSudoChange(act) => {
                let bridge_sudo_change_fees =
                    get_or_init_fees(state, &bridge_sudo_change_fees).await?;
                calculate_and_add_fees(
                    act,
                    act.fee_asset.to_ibc_prefixed(),
                    &mut fees_by_asset,
                    bridge_sudo_change_fees,
                );
            }
            Action::ValidatorUpdate(_) => {
                get_or_init_fees(state, &validator_update_fees).await?;
            }
            Action::SudoAddressChange(_) => {
                get_or_init_fees(state, &sudo_address_change_fees).await?;
            }
            Action::IbcSudoChange(_) => {
                get_or_init_fees(state, &ibc_sudo_change_fees).await?;
            }
            Action::Ibc(_) => {
                get_or_init_fees(state, &ibc_relay_fees).await?;
            }
            Action::IbcRelayerChange(_) => {
                get_or_init_fees(state, &ibc_relayer_change_fees).await?;
            }
            Action::FeeAssetChange(_) => {
                get_or_init_fees(state, &fee_asset_change_fees).await?;
            }
            Action::FeeChange(_) => {
                get_or_init_fees(state, &fee_change_fees).await?;
            }
        }
    }
    Ok(fees_by_asset)
}

fn calculate_and_add_fees<F: FeeHandler>(
    action: &F,
    fee_asset: asset::IbcPrefixed,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    fees: &FeeComponents<F>,
) {
    let base = fees.base();
    let multiplier = fees.multiplier();
    let total_fees = base.saturating_add(multiplier.saturating_mul(action.variable_component()));
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

async fn get_or_init_fees<'a, F, S>(
    state: &S,
    fee_components: &'a OnceCell<Option<FeeComponents<F>>>,
) -> eyre::Result<&'a FeeComponents<F>>
where
    F: FeeHandler,
    FeeComponents<F>: TryFrom<StoredValue<'a>, Error = ErrReport>,
    S: StateRead,
{
    let fees = fee_components
        .get_or_try_init(|| async { state.get_fees::<F>().await })
        .await
        .wrap_err_with(|| format!("failed to get fees for `{}` action", F::snake_case_name()))?
        .as_ref()
        .ok_or_else(|| {
            eyre!(
                "fees not found for `{}` action, hence it is disabled",
                F::snake_case_name()
            )
        })?;
    Ok(fees)
}
