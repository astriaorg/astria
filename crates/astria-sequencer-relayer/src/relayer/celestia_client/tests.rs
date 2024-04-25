use astria_core::generated::cosmos::base::abci::v1beta1::TxResponse;
use celestia_client::celestia_types::{
    blob::Commitment,
    nmt::Namespace,
};
use prost::bytes::Bytes;

use super::*;

#[test]
fn new_msg_pay_for_blobs_should_succeed() {
    let blobs: Vec<_> = (0..5)
        .map(|index| {
            Blob::new(
                Namespace::const_v0([index; 10]),
                vec![index; index as usize],
            )
            .unwrap()
        })
        .collect();
    let signer = Bech32Address("a".to_string());
    let msg = new_msg_pay_for_blobs(&blobs, signer.clone()).unwrap();
    assert_eq!(msg.signer, signer.0);

    let namespaces: Vec<_> = blobs
        .iter()
        .map(|blob| blob.namespace.as_bytes().to_vec())
        .collect();
    assert_eq!(msg.namespaces, namespaces);

    // allow: data length is small in this test case.
    #[allow(clippy::cast_possible_truncation)]
    let blob_sizes: Vec<_> = blobs.iter().map(|blob| blob.data.len() as u32).collect();
    assert_eq!(msg.blob_sizes, blob_sizes);

    let share_commitments: Vec<_> = blobs
        .iter()
        .map(|blob| blob.commitment.0.to_vec())
        .collect();
    assert_eq!(msg.share_commitments, share_commitments);

    let share_versions: Vec<_> = blobs
        .iter()
        .map(|blob| u32::from(blob.share_version))
        .collect();
    assert_eq!(msg.share_versions, share_versions);
}

#[test]
fn new_msg_pay_for_blobs_should_fail_for_large_blob() {
    let blob = Blob {
        namespace: Namespace::TRANSACTION,
        data: vec![0; u32::MAX as usize + 1],
        share_version: 0,
        commitment: Commitment([0; 32]),
    };
    let error = new_msg_pay_for_blobs(&[blob], Bech32Address("a".to_string())).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::BlobTooLarge { byte_count } if byte_count == u32::MAX as usize + 1)
    {
        panic!("expected `Error::BlobTooLarge` with byte_count == u32::MAX + 1, got {error:?}");
    }
}

#[test]
fn account_from_good_response_should_succeed() {
    let base_account = BaseAccount {
        address: "address".to_string(),
        pub_key: None,
        account_number: 1,
        sequence: 2,
    };
    let account_as_any = pbjson_types::Any {
        type_url: BaseAccount::type_url(),
        value: Bytes::from(base_account.encode_to_vec()),
    };
    let response = Response::new(QueryAccountResponse {
        account: Some(account_as_any),
    });

    let extracted_account = account_from_response(Ok(response)).unwrap();
    assert_eq!(base_account, extracted_account);
}

#[test]
fn account_from_bad_response_should_fail() {
    // Should return `FailedToGetAccountInfo` if outer response is an error.
    let error = account_from_response(Err(Status::internal(""))).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::FailedToGetAccountInfo(_)) {
        panic!("expected `Error::FailedToGetAccountInfo`, got {error:?}");
    }

    // Should return `EmptyAccountInfo` if the inner response's `account` is `None`.
    let response = Ok(Response::new(QueryAccountResponse {
        account: None,
    }));
    let error = account_from_response(response).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::EmptyAccountInfo) {
        panic!("expected `Error::EmptyAccountInfo`, got {error:?}");
    }

    // Should return `AccountInfoTypeMismatch` if the inner response's `account` has the wrong
    // type URL.
    let bad_url = "bad url";
    let bad_url_account = pbjson_types::Any {
        type_url: bad_url.to_string(),
        value: Bytes::new(),
    };
    let response = Ok(Response::new(QueryAccountResponse {
        account: Some(bad_url_account),
    }));
    let error = account_from_response(response).unwrap_err();
    match error {
        TrySubmitError::AccountInfoTypeMismatch {
            expected,
            received,
        } => {
            assert_eq!(expected, BaseAccount::type_url(),);
            assert_eq!(received, bad_url,);
        }
        _ => panic!("expected `AccountInfoTypeMismatch` error, but got {error:?}"),
    }

    // Should return `DecodeAccountInfo` if the inner response's `account` fails to decode.
    let bad_value_account = pbjson_types::Any {
        type_url: BaseAccount::type_url(),
        value: Bytes::from(vec![1]),
    };
    let response = Ok(Response::new(QueryAccountResponse {
        account: Some(bad_value_account),
    }));
    let error = account_from_response(response).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::DecodeAccountInfo(_)) {
        panic!("expected `Error::DecodeAccountInfo`, got {error:?}");
    }
}

#[test]
fn min_gas_price_from_good_response_should_succeed() {
    let min_gas_price = 1234.56_f64;
    let response = Response::new(MinGasPriceResponse {
        minimum_gas_price: format!("{min_gas_price}utia"),
    });
    let extracted_price = min_gas_price_from_response(Ok(response)).unwrap();
    // allow: this floating point comparison should be ok due to the hard-coded values chosen.
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(min_gas_price, extracted_price);
    }
}

#[test]
fn min_gas_price_from_bad_response_should_fail() {
    // Should return `FailedToGetMinGasPrice` if outer response is an error.
    let error = min_gas_price_from_response(Err(Status::internal(""))).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::FailedToGetMinGasPrice(_)) {
        panic!("expected `Error::FailedToGetMinGasPrice`, got {error:?}");
    }

    // Should return `MinGasPriceBadSuffix` if the inner response's `minimum_gas_price` doesn't
    // have the suffix "utia".
    let bad_suffix = "9tia";
    let response = Ok(Response::new(MinGasPriceResponse {
        minimum_gas_price: bad_suffix.to_string(),
    }));
    let error = min_gas_price_from_response(response).unwrap_err();
    match error {
        TrySubmitError::MinGasPriceBadSuffix {
            min_gas_price,
            expected_suffix,
        } => {
            assert_eq!(min_gas_price, bad_suffix,);
            assert_eq!(expected_suffix, "utia",);
        }
        _ => panic!("expected `MinGasPriceBadSuffix` error, but got {error:?}"),
    }

    // Should return `FailedToParseMinGasPrice` if the inner response's `minimum_gas_price` doesn't
    // parse as a `f64` after stripping "utia" from the end.
    let bad_value = "9u";
    let response = Ok(Response::new(MinGasPriceResponse {
        minimum_gas_price: format!("{bad_value}utia"),
    }));
    let error = min_gas_price_from_response(response).unwrap_err();
    match error {
        TrySubmitError::FailedToParseMinGasPrice {
            min_gas_price, ..
        } => {
            assert_eq!(min_gas_price, bad_value,);
        }
        _ => panic!("expected `FailedToParseMinGasPrice` error, but got {error:?}"),
    }
}

#[derive(Default)]
struct TxResponseBuilder {
    height: i64,
    tx_hash: String,
    code: u32,
    codespace: String,
    raw_log: String,
}

impl TxResponseBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn with_height(mut self, height: i64) -> Self {
        self.height = height;
        self
    }

    fn with_tx_hash<T: AsRef<str>>(mut self, tx_hash: T) -> Self {
        self.tx_hash = tx_hash.as_ref().to_string();
        self
    }

    fn with_code(mut self, code: u32) -> Self {
        self.code = code;
        self
    }

    fn with_codespace<T: AsRef<str>>(mut self, codespace: T) -> Self {
        self.codespace = codespace.as_ref().to_string();
        self
    }

    fn with_raw_log<T: AsRef<str>>(mut self, raw_log: T) -> Self {
        self.raw_log = raw_log.as_ref().to_string();
        self
    }

    fn build(self) -> TxResponse {
        TxResponse {
            height: self.height,
            txhash: self.tx_hash,
            codespace: self.codespace,
            code: self.code,
            data: String::new(),
            raw_log: self.raw_log,
            logs: vec![],
            info: String::new(),
            gas_wanted: 0,
            gas_used: 0,
            tx: None,
            timestamp: String::new(),
            events: vec![],
        }
    }
}

#[test]
fn tx_hash_from_good_response_should_succeed() {
    let tx_hash = "abc";
    let tx_response = TxResponseBuilder::new().with_tx_hash(tx_hash).build();
    let response = Response::new(BroadcastTxResponse {
        tx_response: Some(tx_response),
    });

    let extracted_tx_hash = tx_hash_from_response(Ok(response)).unwrap();
    assert_eq!(tx_hash, extracted_tx_hash.0);
}

#[test]
fn tx_hash_from_bad_response_should_fail() {
    // Should return `FailedToBroadcastTx` if outer response is an error.
    let error = tx_hash_from_response(Err(Status::internal(""))).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::FailedToBroadcastTx(_)) {
        panic!("expected `Error::FailedToBroadcastTx`, got {error:?}");
    }

    // Should return `EmptyBroadcastTxResponse` if the inner response's `tx_response` is `None`.
    let response = Ok(Response::new(BroadcastTxResponse {
        tx_response: None,
    }));
    let error = tx_hash_from_response(response).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::EmptyBroadcastTxResponse) {
        panic!("expected `Error::EmptyBroadcastTxResponse`, got {error:?}");
    }

    // Should return `BroadcastTxResponseErrorCode` if the inner response's `tx_response.code` is
    // not 0.
    let tx_hash = "abc";
    let code = 9;
    let namespace = "def";
    let log = "ghi";
    let tx_response = TxResponseBuilder::new()
        .with_tx_hash(tx_hash)
        .with_code(code)
        .with_codespace(namespace)
        .with_raw_log(log)
        .build();
    let response = Ok(Response::new(BroadcastTxResponse {
        tx_response: Some(tx_response),
    }));
    let error = tx_hash_from_response(response).unwrap_err();
    match error {
        TrySubmitError::BroadcastTxResponseErrorCode {
            tx_hash: received_tx_hash,
            code: received_code,
            namespace: received_namespace,
            log: received_log,
        } => {
            assert_eq!(tx_hash, received_tx_hash,);
            assert_eq!(code, received_code,);
            assert_eq!(namespace, received_namespace,);
            assert_eq!(log, received_log,);
        }
        _ => panic!("expected `BroadcastTxResponseErrorCode` error, but got {error:?}"),
    }
}

#[test]
fn block_height_from_good_response_should_succeed() {
    let height = 9;
    let tx_response = TxResponseBuilder::new().with_height(height).build();
    let response = Response::new(GetTxResponse {
        tx: None,
        tx_response: Some(tx_response),
    });

    let extracted_height = block_height_from_response(Ok(response)).unwrap();
    assert_eq!(Some(u64::try_from(height).unwrap()), extracted_height);
}

#[test]
fn block_height_from_bad_response_should_fail() {
    // Should return `FailedToGetTx` if outer response is an error other than `NotFound`.
    let error = block_height_from_response(Err(Status::internal(""))).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::FailedToGetTx(_)) {
        panic!("expected `Error::FailedToGetTx`, got {error:?}");
    }

    // Should return `EmptyGetTxResponse` if the inner response's `tx_response` is `None`.
    let response = Ok(Response::new(GetTxResponse {
        tx: None,
        tx_response: None,
    }));
    let error = block_height_from_response(response).unwrap_err();
    // allow: `assert!(matches!(..))` provides poor feedback on failure.
    #[allow(clippy::manual_assert)]
    if !matches!(error, TrySubmitError::EmptyGetTxResponse) {
        panic!("expected `Error::EmptyGetTxResponse`, got {error:?}");
    }

    // Should return `GetTxResponseErrorCode` if the inner response's `tx_response.code` is not 0.
    let tx_hash = "abc";
    let code = 9;
    let namespace = "def";
    let log = "ghi";
    let tx_response = TxResponseBuilder::new()
        .with_tx_hash(tx_hash)
        .with_code(code)
        .with_codespace(namespace)
        .with_raw_log(log)
        .build();
    let response = Ok(Response::new(GetTxResponse {
        tx: None,
        tx_response: Some(tx_response),
    }));
    let error = block_height_from_response(response).unwrap_err();
    match error {
        TrySubmitError::GetTxResponseErrorCode {
            tx_hash: received_tx_hash,
            code: received_code,
            namespace: received_namespace,
            log: received_log,
        } => {
            assert_eq!(tx_hash, received_tx_hash,);
            assert_eq!(code, received_code,);
            assert_eq!(namespace, received_namespace,);
            assert_eq!(log, received_log,);
        }
        _ => panic!("expected `GetTxResponseErrorCode` error, but got {error:?}"),
    }
}

#[test]
fn block_height_from_response_with_negative_height_should_fail() {
    let height = -9;
    let tx_response = TxResponseBuilder::new().with_height(height).build();
    let response = Response::new(GetTxResponse {
        tx: None,
        tx_response: Some(tx_response),
    });

    let error = block_height_from_response(Ok(response)).unwrap_err();
    match error {
        TrySubmitError::GetTxResponseNegativeBlockHeight(received_height) => {
            assert_eq!(height, received_height);
        }
        _ => panic!("expected `GetTxResponseErrorCode` error, but got {error:?}"),
    }
}

#[test]
fn block_height_from_pending_response_should_return_none() {
    // Should return `None` if outer response is a `NotFound` error.
    let maybe_height = block_height_from_response(Err(Status::not_found(""))).unwrap();
    assert!(maybe_height.is_none());

    // Should return `None` if the height is 0.
    let tx_response = TxResponseBuilder::new().with_height(0).build();
    let response = Response::new(GetTxResponse {
        tx: None,
        tx_response: Some(tx_response),
    });

    let maybe_height = block_height_from_response(Ok(response)).unwrap();
    assert!(maybe_height.is_none());
}

#[test]
fn should_use_calculated_fee() {
    // If no last error provided, should use calculated fee.
    let cost_params = CelestiaCostParams::new(8, 10, 0.1);
    let fee = calculate_fee(cost_params, GasLimit(100), None);
    // 0.1 * 100
    let calculated_fee = 10;
    assert_eq!(fee, calculated_fee);

    // If last error wasn't `BroadcastTxResponseErrorCode`, should use calculated fee.
    let fee = calculate_fee(
        cost_params,
        GasLimit(100),
        Some(TrySubmitError::EmptyBroadcastTxResponse),
    );
    assert_eq!(fee, calculated_fee);

    // If last error was `BroadcastTxResponseErrorCode` but the code was not
    // `INSUFFICIENT_FEE_CODE`, should use calculated fee.
    let error = TrySubmitError::BroadcastTxResponseErrorCode {
        tx_hash: String::new(),
        code: INSUFFICIENT_FEE_CODE - 1,
        namespace: String::new(),
        log: String::new(),
    };
    let fee = calculate_fee(cost_params, GasLimit(100), Some(error));
    assert_eq!(fee, calculated_fee);

    // If last error was `BroadcastTxResponseErrorCode` and the code was `INSUFFICIENT_FEE_CODE`,
    // but the log couldn't be parsed, should use calculated fee.
    let error = TrySubmitError::BroadcastTxResponseErrorCode {
        tx_hash: String::new(),
        code: INSUFFICIENT_FEE_CODE,
        namespace: String::new(),
        log: String::new(),
    };
    let fee = calculate_fee(cost_params, GasLimit(100), Some(error));
    assert_eq!(fee, calculated_fee);
}

#[test]
fn should_use_fee_from_error_log() {
    // If last error was `BroadcastTxResponseErrorCode` and the code was `INSUFFICIENT_FEE_CODE`,
    // and the log could be parsed for the required fee, should use that.
    let cost_params = CelestiaCostParams::new(8, 10, 0.1);
    let required_fee = 99;
    let log =
        format!("insufficient fees; got: 1234utia required: {required_fee}utia: insufficient fee");
    let error = TrySubmitError::BroadcastTxResponseErrorCode {
        tx_hash: String::new(),
        code: INSUFFICIENT_FEE_CODE,
        namespace: String::new(),
        log,
    };
    let fee = calculate_fee(cost_params, GasLimit(100), Some(error));
    assert_eq!(fee, required_fee);
}

#[test]
fn extract_required_fee_from_log_should_succeed() {
    fn check(fee: u64) {
        let input =
            format!("insufficient fees; got: 1234utia required: {fee}utia: insufficient fee");
        let extracted = extract_required_fee_from_log(&input);
        assert_eq!(extracted, Some(fee));
    }

    check(0);
    check(1234);
    check(u64::MAX);
}

#[test]
fn extract_required_fee_from_log_should_fail() {
    // We need "utia: insufficient fee".
    let bad_suffix = "insufficient fees; got: 1utia required: 2tia: insufficient fee".to_string();
    assert!(extract_required_fee_from_log(&bad_suffix).is_none());

    // We need a space after "required:".
    let missing_space =
        "insufficient fees; got: 1utia required:2utia: insufficient fee".to_string();
    assert!(extract_required_fee_from_log(&missing_space).is_none());

    // We need the value between "required: " and "utia: ..." to parse as a `u64`.
    let bad_value = "insufficient fees; got: 1utia required: 2mutia: insufficient fee".to_string();
    assert!(extract_required_fee_from_log(&bad_value).is_none());
}
