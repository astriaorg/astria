use std::{
    collections::HashMap,
    future::ready,
};

use astria_core::{
    generated::astria::protocol::transaction::v1::TransactionBody as RawBody,
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
                BridgeTransfer,
                BridgeUnlock,
                CurrencyPairsChange,
                FeeAssetChange,
                FeeChange,
                IbcRelayerChange,
                IbcSudoChange,
                Ics20Withdrawal,
                InitBridgeAccount,
                MarketsChange,
                RecoverIbcClient,
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
    OptionExt as _,
    Report,
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
    join,
    sync::OnceCell,
    try_join,
};
use tracing::{
    instrument,
    warn,
    Level,
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

#[instrument(skip_all, fields(%asset))]
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

#[instrument(skip_all)]
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

pub(crate) async fn components(
    storage: Storage,
    request: request::Query,
    _params: Vec<(String, String)>,
) -> response::Query {
    let snapshot = storage.latest_snapshot();

    let height = async {
        snapshot
            .get_block_height()
            .await
            .wrap_err("failed getting block height")
    };
    let fee_components = get_all_fee_components(&snapshot).map(Ok);
    let (height, fee_components) = match try_join!(height, fee_components) {
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

    let height = tendermint::block::Height::try_from(height).expect("height must fit into an i64");
    response::Query {
        code: tendermint::abci::Code::Ok,
        key: request.path.into_bytes().into(),
        value: serde_json::to_vec(&fee_components)
            .expect("object does not contain keys that don't map to json keys")
            .into(),
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

#[instrument(skip_all, err(level = Level::DEBUG))]
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
    let bridge_transfer_fees: OnceCell<Option<FeeComponents<BridgeTransfer>>> = OnceCell::new();
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
    let recover_ibc_client_fees: OnceCell<Option<FeeComponents<RecoverIbcClient>>> =
        OnceCell::new();
    let currency_pairs_change_fees: OnceCell<Option<FeeComponents<CurrencyPairsChange>>> =
        OnceCell::new();
    let markets_change_fees: OnceCell<Option<FeeComponents<MarketsChange>>> = OnceCell::new();

    let mut fees_by_asset = HashMap::new();
    for action in tx.actions() {
        match action {
            Action::Transfer(act) => {
                let fees = get_or_init_fees(state, &transfer_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::RollupDataSubmission(act) => {
                let fees = get_or_init_fees(state, &rollup_data_submission_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::Ics20Withdrawal(act) => {
                let fees = get_or_init_fees(state, &ics20_withdrawal_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::InitBridgeAccount(act) => {
                let fees = get_or_init_fees(state, &init_bridge_account_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::BridgeLock(act) => {
                let fees = get_or_init_fees(state, &bridge_lock_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::BridgeUnlock(act) => {
                let fees = get_or_init_fees(state, &bridge_unlock_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::BridgeTransfer(act) => {
                let fees = get_or_init_fees(state, &bridge_transfer_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::BridgeSudoChange(act) => {
                let fees = get_or_init_fees(state, &bridge_sudo_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::ValidatorUpdate(act) => {
                let fees = get_or_init_fees(state, &validator_update_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::SudoAddressChange(act) => {
                let fees = get_or_init_fees(state, &sudo_address_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::IbcSudoChange(act) => {
                let fees = get_or_init_fees(state, &ibc_sudo_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::Ibc(act) => {
                let fees = get_or_init_fees(state, &ibc_relay_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::IbcRelayerChange(act) => {
                let fees = get_or_init_fees(state, &ibc_relayer_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::FeeAssetChange(act) => {
                let fees = get_or_init_fees(state, &fee_asset_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::FeeChange(act) => {
                let fees = get_or_init_fees(state, &fee_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::RecoverIbcClient(act) => {
                let fees = get_or_init_fees(state, &recover_ibc_client_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::CurrencyPairsChange(act) => {
                let fees = get_or_init_fees(state, &currency_pairs_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
            Action::MarketsChange(act) => {
                let fees = get_or_init_fees(state, &markets_change_fees).await?;
                calculate_and_add_fees(act, &mut fees_by_asset, fees);
            }
        }
    }
    Ok(fees_by_asset)
}

fn calculate_and_add_fees<F: FeeHandler>(
    action: &F,
    fees_by_asset: &mut HashMap<asset::IbcPrefixed, u128>,
    fees: &FeeComponents<F>,
) {
    let Some(fee_asset) = action.fee_asset().map(Denom::to_ibc_prefixed) else {
        // If there's no fee asset, we don't charge fees.
        return;
    };
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
    FeeComponents<F>: TryFrom<StoredValue<'a>, Error = Report>,
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

#[derive(serde::Serialize)]
struct AllFeeComponents {
    transfer: FetchResult,
    rollup_data_submission: FetchResult,
    ics20_withdrawal: FetchResult,
    init_bridge_account: FetchResult,
    bridge_lock: FetchResult,
    bridge_unlock: FetchResult,
    bridge_transfer: FetchResult,
    bridge_sudo_change: FetchResult,
    ibc_relay: FetchResult,
    validator_update: FetchResult,
    fee_asset_change: FetchResult,
    fee_change: FetchResult,
    ibc_relayer_change: FetchResult,
    sudo_address_change: FetchResult,
    ibc_sudo_change: FetchResult,
    recover_ibc_client: FetchResult,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum FetchResult {
    Err(String),
    Missing(&'static str),
    Component(FeeComponent),
}

impl<T> From<eyre::Result<Option<FeeComponents<T>>>> for FetchResult {
    fn from(value: eyre::Result<Option<FeeComponents<T>>>) -> Self {
        match value {
            Ok(Some(val)) => Self::Component(FeeComponent {
                base: val.base(),
                multiplier: val.multiplier(),
            }),
            Ok(None) => Self::Missing("not set"),
            Err(err) => Self::Err(err.to_string()),
        }
    }
}

async fn get_all_fee_components<S: StateRead>(state: &S) -> AllFeeComponents {
    let (
        transfer,
        rollup_data_submission,
        ics20_withdrawal,
        init_bridge_account,
        bridge_lock,
        bridge_unlock,
        bridge_transfer,
        bridge_sudo_change,
        validator_update,
        sudo_address_change,
        ibc_sudo_change,
        ibc_relay,
        ibc_relayer_change,
        fee_asset_change,
        fee_change,
        recover_ibc_client,
    ) = join!(
        state.get_fees::<Transfer>().map(FetchResult::from),
        state
            .get_fees::<RollupDataSubmission>()
            .map(FetchResult::from),
        state.get_fees::<Ics20Withdrawal>().map(FetchResult::from),
        state.get_fees::<InitBridgeAccount>().map(FetchResult::from),
        state.get_fees::<BridgeLock>().map(FetchResult::from),
        state.get_fees::<BridgeUnlock>().map(FetchResult::from),
        state.get_fees::<BridgeTransfer>().map(FetchResult::from),
        state.get_fees::<BridgeSudoChange>().map(FetchResult::from),
        state.get_fees::<ValidatorUpdate>().map(FetchResult::from),
        state.get_fees::<SudoAddressChange>().map(FetchResult::from),
        state.get_fees::<IbcSudoChange>().map(FetchResult::from),
        state.get_fees::<IbcRelay>().map(FetchResult::from),
        state.get_fees::<IbcRelayerChange>().map(FetchResult::from),
        state.get_fees::<FeeAssetChange>().map(FetchResult::from),
        state.get_fees::<FeeChange>().map(FetchResult::from),
        state.get_fees::<RecoverIbcClient>().map(FetchResult::from),
    );
    AllFeeComponents {
        transfer,
        rollup_data_submission,
        ics20_withdrawal,
        init_bridge_account,
        bridge_lock,
        bridge_unlock,
        bridge_transfer,
        bridge_sudo_change,
        ibc_relay,
        validator_update,
        fee_asset_change,
        fee_change,
        ibc_relayer_change,
        sudo_address_change,
        ibc_sudo_change,
        recover_ibc_client,
    }
}

#[derive(serde::Serialize)]
struct FeeComponent {
    base: u128,
    multiplier: u128,
}

#[cfg(test)]
mod test {
    use astria_core::{
        generated::astria::protocol::fees::v1::TransactionFeeResponse as RawTransactionFeeResponse,
        primitive::v1::RollupId,
        protocol::fees::v1::TransactionFeeResponse,
    };

    use super::*;
    use crate::{
        app::StateWriteExt as _,
        assets::StateWriteExt as _,
        benchmark_and_test_utils::astria_address,
        fees::StateWriteExt as _,
    };

    #[tokio::test]
    async fn transaction_fee_request_ok() {
        let asset_a: Denom = "test-asset-a".parse().unwrap();
        let asset_b: Denom = "test-asset-b".parse().unwrap();
        let action_a = Transfer {
            to: astria_address([1u8; 20].as_slice()),
            amount: 100,
            asset: asset_a.clone(),
            fee_asset: asset_a.clone(),
        };
        let action_b = RollupDataSubmission {
            data: vec![1, 2, 3].into(),
            rollup_id: RollupId::from_unhashed_bytes(b"rollupid"),
            fee_asset: asset_b.clone(),
        };
        let chain_id = "test-1";

        let body = TransactionBody::builder()
            .actions(vec![action_a.clone().into(), action_b.clone().into()])
            .chain_id(chain_id)
            .nonce(0)
            .try_build()
            .unwrap();

        let storage = cnidarium::TempStorage::new().await.unwrap();
        let snapshot = storage.latest_snapshot();
        let mut state = cnidarium::StateDelta::new(snapshot);

        let transfer_base_fee = 10;
        let transfer_multiplier = 1;
        let rollup_data_submission_base_fee = 20;
        let rollup_data_submission_multiplier = 2;
        state
            .put_fees::<Transfer>(FeeComponents::new(transfer_base_fee, transfer_multiplier))
            .unwrap();
        state
            .put_fees::<RollupDataSubmission>(FeeComponents::new(
                rollup_data_submission_base_fee,
                rollup_data_submission_multiplier,
            ))
            .unwrap();
        state.put_block_height(1).unwrap();
        state
            .put_ibc_asset(asset_a.clone().unwrap_trace_prefixed())
            .unwrap();
        state
            .put_ibc_asset(asset_b.clone().unwrap_trace_prefixed())
            .unwrap();
        storage.commit(state).await.unwrap();

        let query = request::Query {
            data: body.into_raw().encode_to_vec().into(),
            path: "path".to_string(),
            height: 0u32.into(),
            prove: false,
        };

        let resp = transaction_fee_request(storage.clone(), query, vec![]).await;
        assert_eq!(resp.code, 0.into(), "{}", resp.log);

        let proto = RawTransactionFeeResponse::decode(&*resp.value).unwrap();
        let mut resp = TransactionFeeResponse::try_from_raw(proto).unwrap();
        let mut expected = TransactionFeeResponse {
            height: 1,
            fees: vec![
                (
                    asset_a,
                    transfer_base_fee + transfer_multiplier * action_a.variable_component(),
                ),
                (
                    asset_b,
                    rollup_data_submission_base_fee
                        + rollup_data_submission_multiplier * action_b.variable_component(),
                ),
            ],
        };
        resp.fees
            .sort_by(|a: &(Denom, u128), b: &(Denom, u128)| a.0.cmp(&b.0));
        expected
            .fees
            .sort_by(|a: &(Denom, u128), b: &(Denom, u128)| a.0.cmp(&b.0));

        assert_eq!(resp, expected);
    }
}
