use super::InvalidNonce;

#[test]
fn invalid_nonce() {
    assert!(
        InvalidNonce {
            current: 24,
            in_transaction: 42
        }
        .is_ahead()
    );
    assert!(
        !InvalidNonce {
            current: 42,
            in_transaction: 24
        }
        .is_ahead()
    );
    assert!(
        !InvalidNonce {
            current: 42,
            in_transaction: 42
        }
        .is_ahead()
    );
}
