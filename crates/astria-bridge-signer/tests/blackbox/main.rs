use astria_core::{
    generated::astria::signer::v1::CommitmentWithIdentifier,
    protocol::transaction::v1::TransactionBody,
};
use ethers::utils::hex;
use frost_ed25519::Identifier;
use helpers::{
    make_bridge_unlock,
    make_ics20_withdrawal,
};
use tonic::Code;

use crate::helpers::test_bridge_signer::TestBridgeSigner;

mod helpers;

const SIGNER_ID: u16 = 1;
const NON_SIGNER_ID: u16 = 2;

#[tokio::test]
async fn get_verifying_share_works_as_expected() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let rsp = test_bridge_signer.get_verifying_share().await.unwrap();

    assert_eq!(rsp.verifying_share.len(), 32);

    // Hex-encoded verifying share defined in
    // `tests/blackbox/helpers/key-package/test_secret_key_package.json`
    assert_eq!(
        hex::encode(rsp.verifying_share).to_string(),
        "5e77c4f064bb625578007140748ed8e1747bdfec5d2c4f66cb579c27eeca1509"
    );
}

// NOTE: We cannot test the outputs of `execute_round_one` and `execute_round_two` because the round
// one relies on random number generation, and round two relies on the output of round one. Thie
// will need to be tested in an end-to-end test instead (https://github.com/astriaorg/astria/issues/2077).
#[tokio::test]
async fn executes_rounds_one_and_two_with_ics20_withdrawal() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let round_one_rsp = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp.request_identifier, 0);

    let signer_commitment_1 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment.clone(),
        participant_identifier: Identifier::try_from(SIGNER_ID).unwrap().serialize().into(),
    };
    // The second commitment's content doesn't matter, but it needs a different identifier to meet
    // the minimum signer count
    let signer_commitment_2 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment,
        participant_identifier: Identifier::try_from(NON_SIGNER_ID)
            .unwrap()
            .serialize()
            .into(),
    };
    let commitments = vec![signer_commitment_1, signer_commitment_2];
    let ics20_withdrawal = make_ics20_withdrawal();
    test_bridge_signer
        .mount_ics20_withdrawal_verification(&ics20_withdrawal)
        .await;
    let tx_body = TransactionBody::builder()
        .actions(vec![ics20_withdrawal.into()])
        .try_build()
        .unwrap();

    test_bridge_signer
        .execute_round_two(commitments, tx_body, round_one_rsp.request_identifier)
        .await
        .unwrap();
}

#[tokio::test]
async fn executes_rounds_one_and_two_with_bridge_unlock() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let round_one_rsp = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp.request_identifier, 0);

    let signer_commitment_1 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment.clone(),
        participant_identifier: Identifier::try_from(SIGNER_ID).unwrap().serialize().into(),
    };
    let signer_commitment_2 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment,
        participant_identifier: Identifier::try_from(NON_SIGNER_ID)
            .unwrap()
            .serialize()
            .into(),
    };
    let commitments = vec![signer_commitment_1, signer_commitment_2];
    let bridge_unlock = make_bridge_unlock();
    test_bridge_signer
        .mount_bridge_unlock_verification(&bridge_unlock)
        .await;
    let tx_body = TransactionBody::builder()
        .actions(vec![bridge_unlock.into()])
        .try_build()
        .unwrap();

    test_bridge_signer
        .execute_round_two(commitments, tx_body, round_one_rsp.request_identifier)
        .await
        .unwrap();
}

#[tokio::test]
async fn round_two_fails_if_not_enough_commitments() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let round_one_rsp = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp.request_identifier, 0);

    // Min signers is 2, only one commitment here to test failure
    let signer_commitment_1 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment.clone(),
        participant_identifier: Identifier::try_from(SIGNER_ID).unwrap().serialize().into(),
    };
    let commitments = vec![signer_commitment_1];
    let bridge_unlock = make_bridge_unlock();
    test_bridge_signer
        .mount_bridge_unlock_verification(&bridge_unlock)
        .await;
    let tx_body = TransactionBody::builder()
        .actions(vec![bridge_unlock.into()])
        .try_build()
        .unwrap();

    let err = test_bridge_signer
        .execute_round_two(commitments, tx_body, round_one_rsp.request_identifier)
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Internal);
    assert_eq!(
        err.message(),
        "failed to sign: Incorrect number of commitments."
    );
}

#[tokio::test]
async fn round_two_fails_if_request_identifier_is_incorrect() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let round_one_rsp = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp.request_identifier, 0);

    let bridge_unlock = make_bridge_unlock();
    let tx_body = TransactionBody::builder()
        .actions(vec![bridge_unlock.into()])
        .try_build()
        .unwrap();

    let err = test_bridge_signer
        .execute_round_two(vec![], tx_body, 1) // incorrect request identifier
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "invalid request identifier");
}

#[tokio::test]
async fn round_two_fails_if_it_has_already_been_called() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let round_one_rsp = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp.request_identifier, 0);

    let signer_commitment_1 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment.clone(),
        participant_identifier: Identifier::try_from(SIGNER_ID).unwrap().serialize().into(),
    };
    let signer_commitment_2 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment,
        participant_identifier: Identifier::try_from(NON_SIGNER_ID)
            .unwrap()
            .serialize()
            .into(),
    };
    let commitments = vec![signer_commitment_1, signer_commitment_2];
    let bridge_unlock = make_bridge_unlock();
    test_bridge_signer
        .mount_bridge_unlock_verification(&bridge_unlock)
        .await;
    let tx_body = TransactionBody::builder()
        .actions(vec![bridge_unlock.into()])
        .try_build()
        .unwrap();

    test_bridge_signer
        .execute_round_two(
            commitments.clone(),
            tx_body.clone(),
            round_one_rsp.request_identifier,
        )
        .await
        .unwrap();

    // Second call should fail since request identifier and corresponding nonce should be removed
    // from state
    let err = test_bridge_signer
        .execute_round_two(commitments, tx_body, round_one_rsp.request_identifier)
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "invalid request identifier");
}

#[tokio::test]
async fn round_two_fails_if_signers_commitment_is_missing() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    let round_one_rsp = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp.request_identifier, 0);

    let signer_commitment_1 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment.clone(),
        participant_identifier: Identifier::try_from(NON_SIGNER_ID)
            .unwrap()
            .serialize()
            .into(),
    };
    let signer_commitment_2 = CommitmentWithIdentifier {
        commitment: round_one_rsp.commitment,
        participant_identifier: Identifier::try_from(NON_SIGNER_ID.saturating_add(1))
            .unwrap()
            .serialize()
            .into(),
    };
    let commitments = vec![signer_commitment_1, signer_commitment_2];
    let bridge_unlock = make_bridge_unlock();
    test_bridge_signer
        .mount_bridge_unlock_verification(&bridge_unlock)
        .await;
    let tx_body = TransactionBody::builder()
        .actions(vec![bridge_unlock.into()])
        .try_build()
        .unwrap();

    let err = test_bridge_signer
        .execute_round_two(commitments, tx_body, round_one_rsp.request_identifier)
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Internal);
    assert_eq!(
        err.message(),
        "failed to sign: The Signing Package must contain the participant's Commitment."
    );
}

#[tokio::test]
async fn round_two_fails_if_signers_commitment_is_incorrect() {
    let mut test_bridge_signer = TestBridgeSigner::spawn().await;

    // Here, two different commitments are generated. This way, an incorrect commitment (request id
    // 1) can be associated with the participant when executing round two for request id 0
    let round_one_rsp_1 = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp_1.request_identifier, 0);
    let round_one_rsp_2 = test_bridge_signer.execute_round_one().await.unwrap();
    assert_eq!(round_one_rsp_2.request_identifier, 1);

    let signer_commitment_1 = CommitmentWithIdentifier {
        // Note that we mount response 2's commitment here, instead of response 1's
        commitment: round_one_rsp_2.commitment.clone(),
        participant_identifier: Identifier::try_from(SIGNER_ID).unwrap().serialize().into(),
    };
    let signer_commitment_2 = CommitmentWithIdentifier {
        commitment: round_one_rsp_2.commitment,
        participant_identifier: Identifier::try_from(NON_SIGNER_ID)
            .unwrap()
            .serialize()
            .into(),
    };
    let commitments = vec![signer_commitment_1, signer_commitment_2];
    let bridge_unlock = make_bridge_unlock();
    test_bridge_signer
        .mount_bridge_unlock_verification(&bridge_unlock)
        .await;
    let tx_body = TransactionBody::builder()
        .actions(vec![bridge_unlock.into()])
        .try_build()
        .unwrap();

    let err = test_bridge_signer
        .execute_round_two(commitments, tx_body, round_one_rsp_1.request_identifier)
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Internal);
    assert_eq!(
        err.message(),
        "failed to sign: The participant's commitment is incorrect."
    );
}
