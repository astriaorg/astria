mod builder;
mod celestia_cost_params;
pub(crate) mod celestia_keys;
mod error;
#[cfg(test)]
mod tests;

use std::{
    convert::TryInto,
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};

use astria_core::generated::{
    celestia::v1::{
        query_client::QueryClient as BlobQueryClient,
        MsgPayForBlobs,
        Params as BlobParams,
        QueryParamsRequest as QueryBlobParamsRequest,
    },
    cosmos::{
        auth::v1beta1::{
            query_client::QueryClient as AuthQueryClient,
            BaseAccount,
            Params as AuthParams,
            QueryAccountRequest,
            QueryAccountResponse,
            QueryParamsRequest as QueryAuthParamsRequest,
        },
        base::{
            node::v1beta1::{
                service_client::ServiceClient as MinGasPriceClient,
                ConfigRequest as MinGasPriceRequest,
                ConfigResponse as MinGasPriceResponse,
            },
            v1beta1::Coin,
        },
        crypto::secp256k1,
        tx::v1beta1::{
            mode_info::{
                Single,
                Sum,
            },
            service_client::ServiceClient as TxClient,
            AuthInfo,
            BroadcastMode,
            BroadcastTxRequest,
            BroadcastTxResponse,
            Fee,
            GetTxRequest,
            GetTxResponse,
            ModeInfo,
            SignDoc,
            SignerInfo,
            Tx,
            TxBody,
        },
    },
    tendermint::types::{
        Blob as PbBlob,
        BlobTx,
    },
};
use astria_eyre::eyre::Report;
pub(super) use builder::{
    Builder as CelestiaClientBuilder,
    BuilderError,
};
use celestia_cost_params::CelestiaCostParams;
pub(crate) use celestia_keys::CelestiaKeys;
use celestia_types::Blob;
pub(super) use error::{
    GrpcResponseError,
    ProtobufDecodeError,
    TrySubmitError,
};
use prost::{
    bytes::Bytes,
    Message as _,
    Name as _,
};
use tokio::sync::watch;
use tonic::{
    transport::Channel,
    Response,
    Status,
};
use tracing::{
    debug,
    info,
    trace,
    warn,
};

// From https://github.com/celestiaorg/cosmos-sdk/blob/v1.18.3-sdk-v0.46.14/types/errors/errors.go#L75
const INSUFFICIENT_FEE_CODE: u32 = 13;

/// A client using the gRPC interface of a remote Celestia app to submit blob data to the Celestia
/// chain.
///
/// It is constructed using a [`CelestiaClientBuilder`].
#[derive(Debug, Clone)]
pub(super) struct CelestiaClient {
    /// The inner `tonic` gRPC channel shared by the various generated gRPC clients.
    grpc_channel: Channel,
    /// A gRPC client to broadcast and get transactions.
    tx_client: TxClient<Channel>,
    /// The crypto keys associated with our Celestia account.
    signing_keys: CelestiaKeys,
    /// The Bech32-encoded address of our Celestia account.
    address: Bech32Address,
    /// The Celestia network ID.
    chain_id: String,
}

impl CelestiaClient {
    /// Tries to submit the given blobs to the Celestia app.
    ///
    /// The `last_error_receiver` will provide the error from the previous attempt if this is not
    /// the first attempt for these blobs, or `None` if it is the first attempt.  The error can be
    /// used to obtain the appropriate fee in the case that the previous attempt failed due to a
    /// low fee.
    // Copied from https://github.com/celestiaorg/celestia-app/blob/v1.4.0/x/blob/payforblob.go
    pub(super) async fn try_submit(
        mut self,
        blobs: Arc<Vec<Blob>>,
        last_error_receiver: watch::Receiver<Option<TrySubmitError>>,
    ) -> Result<u64, TrySubmitError> {
        info!("fetching cost params and account info from celestia app");
        let (blob_params, auth_params, min_gas_price, base_account) = tokio::try_join!(
            self.fetch_blob_params(),
            self.fetch_auth_params(),
            self.fetch_min_gas_price(),
            self.fetch_account(),
        )?;

        let gas_per_blob_byte = blob_params.gas_per_blob_byte;
        let tx_size_cost_per_byte = auth_params.tx_size_cost_per_byte;
        info!(
            gas_per_blob_byte,
            tx_size_cost_per_byte,
            min_gas_price,
            account_number = base_account.account_number,
            sequence = base_account.sequence,
            "fetched cost params and account info from celestia app"
        );

        let msg_pay_for_blobs = new_msg_pay_for_blobs(blobs.as_slice(), self.address.clone())?;

        let cost_params =
            CelestiaCostParams::new(gas_per_blob_byte, tx_size_cost_per_byte, min_gas_price);
        let gas_limit = estimate_gas(&msg_pay_for_blobs.blob_sizes, cost_params);
        // Get the error from the last attempt to `try_submit`.
        let maybe_last_error = last_error_receiver.borrow().clone();
        let fee = calculate_fee(cost_params, gas_limit, maybe_last_error);

        let signed_tx = new_signed_tx(
            &msg_pay_for_blobs,
            &base_account,
            gas_limit,
            fee,
            self.chain_id.clone(),
            &self.signing_keys,
        );

        let blob_tx = new_blob_tx(&signed_tx, blobs.iter());

        info!(
            gas_limit = gas_limit.0,
            fee_utia = fee,
            "broadcasting blob transaction to celestia app"
        );
        let tx_hash = self.broadcast_tx(blob_tx).await?;
        info!(tx_hash = %tx_hash.0, "broadcast blob transaction succeeded");

        let height = self.confirm_submission(tx_hash).await;
        Ok(height)
    }

    async fn fetch_account(&self) -> Result<BaseAccount, TrySubmitError> {
        let mut auth_query_client = AuthQueryClient::new(self.grpc_channel.clone());
        let request = QueryAccountRequest {
            address: self.address.0.clone(),
        };
        let response = auth_query_client.account(request).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        account_from_response(response)
    }

    async fn fetch_blob_params(&self) -> Result<BlobParams, TrySubmitError> {
        let mut blob_query_client = BlobQueryClient::new(self.grpc_channel.clone());
        let response = blob_query_client.params(QueryBlobParamsRequest {}).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        response
            .map_err(|status| {
                TrySubmitError::FailedToGetBlobParams(GrpcResponseError::from(status))
            })?
            .into_inner()
            .params
            .ok_or_else(|| TrySubmitError::EmptyBlobParams)
    }

    async fn fetch_auth_params(&self) -> Result<AuthParams, TrySubmitError> {
        let mut auth_query_client = AuthQueryClient::new(self.grpc_channel.clone());
        let response = auth_query_client.params(QueryAuthParamsRequest {}).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        response
            .map_err(|status| {
                TrySubmitError::FailedToGetAuthParams(GrpcResponseError::from(status))
            })?
            .into_inner()
            .params
            .ok_or_else(|| TrySubmitError::EmptyAuthParams)
    }

    async fn fetch_min_gas_price(&self) -> Result<f64, TrySubmitError> {
        let mut min_gas_price_client = MinGasPriceClient::new(self.grpc_channel.clone());
        let response = min_gas_price_client.config(MinGasPriceRequest {}).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        min_gas_price_from_response(response)
    }

    /// Returns the tx hash if the tx is successfully placed into the node's mempool.
    ///
    /// Note, we use `BroadcastTxSync`, i.e. `BroadcastMode::Sync` as recommended by
    /// [`CometBFT`][cometbft].
    ///
    /// [cometbft]: https://github.com/cometbft/cometbft/blob/b139e139ad9ae6fccb9682aa5c2de4aa952fd055/rpc/openapi/openapi.yaml#L201-L204
    async fn broadcast_tx(&mut self, blob_tx: BlobTx) -> Result<TxHash, TrySubmitError> {
        let request = BroadcastTxRequest {
            tx_bytes: Bytes::from(blob_tx.encode_to_vec()),
            mode: i32::from(BroadcastMode::Sync),
        };
        let response = self.tx_client.broadcast_tx(request).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        tx_hash_from_response(response)
    }

    /// Returns `Some(height)` if the tx submission has completed, or `None` if it is still
    /// pending.
    async fn get_tx(&mut self, tx_hash: TxHash) -> Result<Option<u64>, TrySubmitError> {
        let request = GetTxRequest {
            hash: tx_hash.0.clone(),
        };
        let response = self.tx_client.get_tx(request).await;
        // trace-level logging, so using Debug format is ok.
        #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
        {
            trace!(?response);
        }
        block_height_from_response(response)
    }

    /// Repeatedly sends `GetTx` until a successful response is received.  Returns the height of the
    /// Celestia block in which the blobs were submitted.
    async fn confirm_submission(&mut self, tx_hash: TxHash) -> u64 {
        // The min seconds to sleep after receiving a GetTx response and sending the next request.
        const MIN_POLL_INTERVAL_SECS: u64 = 1;
        // The max seconds to sleep after receiving a GetTx response and sending the next request.
        const MAX_POLL_INTERVAL_SECS: u64 = 12;
        // How long to wait after starting `confirm_submission` before starting to log errors.
        const START_LOGGING_DELAY: Duration = Duration::from_secs(12);
        // The minimum duration between logging errors.
        const LOG_ERROR_INTERVAL: Duration = Duration::from_secs(5);

        let start = Instant::now();
        let mut logged_at = start;

        let mut log_if_due = |maybe_error: Option<TrySubmitError>| {
            if start.elapsed() <= START_LOGGING_DELAY || logged_at.elapsed() <= LOG_ERROR_INTERVAL {
                return;
            }
            let reason = maybe_error.map_or(Report::msg("transaction still pending"), Report::new);
            warn!(
                %reason,
                tx_hash = tx_hash.0,
                elapsed_seconds = start.elapsed().as_secs_f32(),
                "waiting to confirm blob submission"
            );
            logged_at = Instant::now();
        };

        let mut sleep_secs = MIN_POLL_INTERVAL_SECS;
        loop {
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
            match self.get_tx(tx_hash.clone()).await {
                Ok(Some(height)) => return height,
                Ok(None) => {
                    sleep_secs = MIN_POLL_INTERVAL_SECS;
                    log_if_due(None);
                }
                Err(error) => {
                    sleep_secs =
                        std::cmp::min(sleep_secs.saturating_mul(2), MAX_POLL_INTERVAL_SECS);
                    log_if_due(Some(error));
                }
            }
        }
    }
}

fn new_msg_pay_for_blobs(
    blobs: &[Blob],
    signer: Bech32Address,
) -> Result<MsgPayForBlobs, TrySubmitError> {
    // Gather the required fields of the blobs into separate collections, one collection per
    // field.
    let mut blob_sizes = Vec::with_capacity(blobs.len());
    let mut namespaces = Vec::with_capacity(blobs.len());
    let mut share_commitments = Vec::with_capacity(blobs.len());
    let mut share_versions = Vec::with_capacity(blobs.len());
    for blob in blobs {
        blob_sizes.push(blob.data.len());
        namespaces.push(Bytes::from(blob.namespace.as_bytes().to_vec()));
        share_commitments.push(Bytes::from(blob.commitment.0.to_vec()));
        share_versions.push(u32::from(blob.share_version));
    }

    // The `MsgPayForBlobs` struct requires the blob lengths as `u32`s, so fail in the unlikely
    // event that a blob is too large.
    let blob_sizes = blob_sizes
        .into_iter()
        .map(|blob_size| {
            u32::try_from(blob_size).map_err(|_| TrySubmitError::BlobTooLarge {
                byte_count: blob_size,
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(MsgPayForBlobs {
        signer: signer.0,
        namespaces,
        blob_sizes,
        share_commitments,
        share_versions,
    })
}

/// Extracts a `BaseAccount` from the given response.
fn account_from_response(
    response: Result<Response<QueryAccountResponse>, Status>,
) -> Result<BaseAccount, TrySubmitError> {
    let account_info = response.map_err(|status| {
        TrySubmitError::FailedToGetAccountInfo(GrpcResponseError::from(status))
    })?;

    let account_as_any = account_info
        .into_inner()
        .account
        .ok_or_else(|| TrySubmitError::EmptyAccountInfo)?;
    let expected_type_url = BaseAccount::type_url();

    if expected_type_url == account_as_any.type_url {
        return BaseAccount::decode(&*account_as_any.value)
            .map_err(|error| TrySubmitError::DecodeAccountInfo(ProtobufDecodeError::from(error)));
    }

    Err(TrySubmitError::AccountInfoTypeMismatch {
        expected: expected_type_url,
        received: account_as_any.type_url,
    })
}

/// Extracts the minimum gas price from the given response.
fn min_gas_price_from_response(
    response: Result<Response<MinGasPriceResponse>, Status>,
) -> Result<f64, TrySubmitError> {
    const UNITS_SUFFIX: &str = "utia";
    let min_gas_price_with_suffix = response
        .map_err(|status| TrySubmitError::FailedToGetMinGasPrice(GrpcResponseError::from(status)))?
        .into_inner()
        .minimum_gas_price;
    let min_gas_price_str = min_gas_price_with_suffix
        .strip_suffix(UNITS_SUFFIX)
        .ok_or_else(|| TrySubmitError::MinGasPriceBadSuffix {
            min_gas_price: min_gas_price_with_suffix.clone(),
            expected_suffix: UNITS_SUFFIX,
        })?;
    min_gas_price_str
        .parse::<f64>()
        .map_err(|source| TrySubmitError::FailedToParseMinGasPrice {
            min_gas_price: min_gas_price_str.to_string(),
            source,
        })
}

/// Extracts the tx hash from the given response.
fn tx_hash_from_response(
    response: Result<Response<BroadcastTxResponse>, Status>,
) -> Result<TxHash, TrySubmitError> {
    let tx_response = response
        .map_err(|status| TrySubmitError::FailedToBroadcastTx(GrpcResponseError::from(status)))?
        .into_inner()
        .tx_response
        .ok_or_else(|| TrySubmitError::EmptyBroadcastTxResponse)?;
    if tx_response.code != 0 {
        let error = TrySubmitError::BroadcastTxResponseErrorCode {
            tx_hash: tx_response.txhash,
            code: tx_response.code,
            namespace: tx_response.codespace,
            log: tx_response.raw_log,
        };
        return Err(error);
    }
    Ok(TxHash(tx_response.txhash))
}

/// Extracts the block height from the given response if available, or `None` if the transaction is
/// not available yet.
fn block_height_from_response(
    response: Result<Response<GetTxResponse>, Status>,
) -> Result<Option<u64>, TrySubmitError> {
    let ok_response = match response {
        Ok(resp) => resp,
        Err(status) => {
            // trace-level logging, so using Debug format is ok.
            #[cfg_attr(dylint_lib = "tracing_debug_field", allow(tracing_debug_field))]
            {
                trace!(?status);
            }
            if status.code() == tonic::Code::NotFound {
                debug!(msg = status.message(), "transaction still pending");
                return Ok(None);
            }
            return Err(TrySubmitError::FailedToGetTx(GrpcResponseError::from(
                status,
            )));
        }
    };
    let tx_response = ok_response
        .into_inner()
        .tx_response
        .ok_or_else(|| TrySubmitError::EmptyGetTxResponse)?;
    if tx_response.code != 0 {
        let error = TrySubmitError::GetTxResponseErrorCode {
            tx_hash: tx_response.txhash,
            code: tx_response.code,
            namespace: tx_response.codespace,
            log: tx_response.raw_log,
        };
        return Err(error);
    }
    if tx_response.height == 0 {
        debug!(tx_hash = %tx_response.txhash, "transaction still pending");
        return Ok(None);
    }

    let height = u64::try_from(tx_response.height)
        .map_err(|_| TrySubmitError::GetTxResponseNegativeBlockHeight(tx_response.height))?;

    debug!(tx_hash = %tx_response.txhash, height, "transaction succeeded");
    Ok(Some(height))
}

// Copied from https://github.com/celestiaorg/celestia-app/blob/v1.4.0/x/blob/types/payforblob.go#L174
//
// `blob_sizes` is the collection of sizes in bytes of all the blobs' `data` fields.
fn estimate_gas(blob_sizes: &[u32], cost_params: CelestiaCostParams) -> GasLimit {
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/pkg/appconsts/global_consts.go#L28
    const SHARE_SIZE: u64 = 512;
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/pkg/appconsts/global_consts.go#L55
    const CONTINUATION_COMPACT_SHARE_CONTENT_SIZE: u32 = 482;
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/pkg/appconsts/global_consts.go#L59
    const FIRST_SPARSE_SHARE_CONTENT_SIZE: u32 = 478;
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/x/blob/types/payforblob.go#L40
    const PFB_GAS_FIXED_COST: u64 = 75_000;
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/x/blob/types/payforblob.go#L44
    const BYTES_PER_BLOB_INFO: u64 = 70;

    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/pkg/shares/share_sequence.go#L126
    //
    // `blob_len` is the size in bytes of one blob's `data` field.
    fn sparse_shares_needed(blob_len: u32) -> u64 {
        if blob_len == 0 {
            return 0;
        }

        if blob_len < FIRST_SPARSE_SHARE_CONTENT_SIZE {
            return 1;
        }

        // Use `u64` here to avoid overflow while adding below.
        let mut bytes_available = u64::from(FIRST_SPARSE_SHARE_CONTENT_SIZE);
        let mut shares_needed = 1_u64;
        while bytes_available < u64::from(blob_len) {
            bytes_available = bytes_available
                .checked_add(u64::from(CONTINUATION_COMPACT_SHARE_CONTENT_SIZE))
                .expect(
                    "this can't overflow, as on each iteration `bytes_available < u32::MAX`, and \
                     we're adding at most `u32::MAX` to it",
                );
            shares_needed = shares_needed.checked_add(1).expect(
                "this can't overflow, as the loop cannot execute for `u64::MAX` iterations",
            );
        }
        shares_needed
    }

    let total_shares_used: u64 = blob_sizes.iter().copied().map(sparse_shares_needed).sum();
    let blob_count = blob_sizes.len().try_into().unwrap_or(u64::MAX);

    let shares_gas = total_shares_used
        .saturating_mul(SHARE_SIZE)
        .saturating_mul(u64::from(cost_params.gas_per_blob_byte()));
    let blob_info_gas = cost_params
        .tx_size_cost_per_byte()
        .saturating_mul(BYTES_PER_BLOB_INFO)
        .saturating_mul(blob_count);

    GasLimit(
        shares_gas
            .saturating_add(blob_info_gas)
            .saturating_add(PFB_GAS_FIXED_COST),
    )
}

/// Returns the fee for the signed tx.
///
/// This is calculated as `min gas price * gas limit`, but if a required fee can be extracted from
/// `maybe_last_error`, it will be returned rather than a calculated value.
fn calculate_fee(
    cost_params: CelestiaCostParams,
    gas_limit: GasLimit,
    maybe_last_error: Option<TrySubmitError>,
) -> u64 {
    // Try to extract the required fee from the last error.
    let maybe_required_fee = match maybe_last_error {
        Some(TrySubmitError::BroadcastTxResponseErrorCode {
            code,
            log,
            ..
        }) if code == INSUFFICIENT_FEE_CODE => extract_required_fee_from_log(&log),
        _ => None,
    };

    // Calculate the fee from the provided values.
    // From https://github.com/celestiaorg/celestia-node/blob/v0.12.4/state/core_access.go#L225
    //
    // allow: the gas limit should never be negative, and truncation/precision is not a problem
    // as this is a best-effort calculation.  If the result is incorrect, the retry will use
    // the fee provided in the failure response.
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    let calculated_fee = (cost_params.min_gas_price() * gas_limit.0 as f64).ceil() as u64;

    // If we have extracted the required fee from the last error, use that.  Otherwise use the
    // calculated one.
    match maybe_required_fee {
        Some(required_fee) => {
            // If the calculated fee is still lower than the required fee or is significantly
            // higher (> 1.2 times), log an error as the calculation function probably needs fixed.
            if calculated_fee < required_fee {
                warn!(
                    calculated_fee,
                    required_fee,
                    "fee calculation yielded a low value: investigate calculation function"
                );
            }
            if calculated_fee > required_fee.saturating_mul(6).saturating_div(5) {
                warn!(
                    calculated_fee,
                    required_fee,
                    "fee calculation yielded a high value: investigate calculation function"
                );
            }
            required_fee
        }
        None => calculated_fee,
    }
}

/// `log`'s value for this case currently looks like:
/// "insufficient fees; got: 1234utia required: 7980utia: insufficient fee"
/// We'll make a best-effort attempt to parse, but this is just a failsafe to check the
/// new calculated fee using updated Celestia costs is sufficient, so if parsing fails
/// we'll just log the error and otherwise ignore.
fn extract_required_fee_from_log(celestia_broadcast_tx_error_log: &str) -> Option<u64> {
    const SUFFIX: &str = "utia: insufficient fee";
    // Should be left with e.g. "insufficient fees; got: 1234utia required: 7980".
    let Some(log_without_suffix) = celestia_broadcast_tx_error_log.strip_suffix(SUFFIX) else {
        warn!(
            celestia_broadcast_tx_error_log,
            "insufficient gas error doesn't end with '{SUFFIX}'"
        );
        return None;
    };
    // Should be left with e.g. "7980".
    let Some(required) = log_without_suffix.rsplit(' ').next() else {
        warn!(
            celestia_broadcast_tx_error_log,
            "insufficient gas error doesn't have a space before the required amount"
        );
        return None;
    };
    match required.parse::<u64>() {
        Ok(required_fee) => {
            info!(
                required_fee,
                "extracted required fee from broadcast transaction response raw log"
            );
            Some(required_fee)
        }
        Err(error) => {
            warn!(
                celestia_broadcast_tx_error_log, %error,
                "insufficient gas error required amount cannot be parsed as u64"
            );
            None
        }
    }
}

fn new_signed_tx(
    msg_pay_for_blobs: &MsgPayForBlobs,
    base_account: &BaseAccount,
    gas_limit: GasLimit,
    fee: u64,
    chain_id: String,
    signing_keys: &CelestiaKeys,
) -> Tx {
    // From https://github.com/celestiaorg/celestia-app/blob/v1.4.0/pkg/appconsts/global_consts.go#L76
    const FEE_DENOM: &str = "utia";
    // From https://github.com/celestiaorg/cosmos-sdk/blob/v1.18.3-sdk-v0.46.14/proto/cosmos/tx/signing/v1beta1/signing.proto#L24
    const SIGNING_MODE_INFO: Option<ModeInfo> = Some(ModeInfo {
        sum: Some(Sum::Single(Single {
            mode: 1,
        })),
    });

    let fee_coin = Coin {
        denom: FEE_DENOM.to_string(),
        amount: fee.to_string(),
    };
    let fee = Fee {
        amount: vec![fee_coin],
        gas_limit: gas_limit.0,
        ..Fee::default()
    };

    let public_key = secp256k1::PubKey {
        key: Bytes::from(
            signing_keys
                .verification_key
                .to_encoded_point(true)
                .as_bytes()
                .to_vec(),
        ),
    };
    let public_key_as_any = pbjson_types::Any {
        type_url: secp256k1::PubKey::type_url(),
        value: public_key.encode_to_vec().into(),
    };
    let auth_info = AuthInfo {
        signer_infos: vec![SignerInfo {
            public_key: Some(public_key_as_any),
            mode_info: SIGNING_MODE_INFO,
            sequence: base_account.sequence,
        }],
        fee: Some(fee),
        tip: None,
    };

    let msg = pbjson_types::Any {
        type_url: MsgPayForBlobs::type_url(),
        value: msg_pay_for_blobs.encode_to_vec().into(),
    };
    let tx_body = TxBody {
        messages: vec![msg],
        ..TxBody::default()
    };

    let bytes_to_sign = SignDoc {
        body_bytes: Bytes::from(tx_body.encode_to_vec()),
        auth_info_bytes: Bytes::from(auth_info.encode_to_vec()),
        chain_id,
        account_number: base_account.account_number,
    }
    .encode_to_vec();
    let signature = signing_keys.sign(&bytes_to_sign);
    Tx {
        body: Some(tx_body),
        auth_info: Some(auth_info),
        signatures: vec![Bytes::from(signature.to_bytes().to_vec())],
    }
}

fn new_blob_tx<'a>(signed_tx: &Tx, blobs: impl Iterator<Item = &'a Blob>) -> BlobTx {
    // From https://github.com/celestiaorg/celestia-core/blob/v1.29.0-tm-v0.34.29/pkg/consts/consts.go#L19
    const BLOB_TX_TYPE_ID: &str = "BLOB";

    let blobs = blobs
        .map(|blob| PbBlob {
            namespace_id: Bytes::from(blob.namespace.id().to_vec()),
            namespace_version: u32::from(blob.namespace.version()),
            data: Bytes::from(blob.data.clone()),
            share_version: u32::from(blob.share_version),
        })
        .collect();
    BlobTx {
        tx: Bytes::from(signed_tx.encode_to_vec()),
        blobs,
        type_id: BLOB_TX_TYPE_ID.to_string(),
    }
}

/// A Bech32-encoded account ID.
#[derive(Clone, Debug)]
struct Bech32Address(String);

#[derive(Copy, Clone, Debug)]
struct GasLimit(u64);

/// A hex-encoded transaction hash.
#[derive(Clone, Debug)]
struct TxHash(String);
