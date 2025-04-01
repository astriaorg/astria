use std::collections::BTreeMap;

use frost_ed25519::keys::dkg::{
    part1,
    part2,
    part3,
};
use rand::rngs::OsRng;

fn generate_test_secret_key_package() {
    let participant_id_1 = 1.try_into().unwrap();
    let participant_id_2 = 2.try_into().unwrap();
    let mut rng = OsRng;

    let max_signers = 2;
    let min_signers = 2;

    let mut received_round_1_pkgs_participant_1 = BTreeMap::new();
    let mut received_round_1_pkgs_participant_2 = BTreeMap::new();
    let (round_1_secret_pkg_participant_1, _round_1_pkg_participant_1) =
        part1(participant_id_1, max_signers, min_signers, &mut rng).unwrap();
    received_round_1_pkgs_participant_2.insert(participant_id_1, _round_1_pkg_participant_1);

    let (round_1_secret_pkg_participant_2, round_1_pkg_participant_2) =
        part1(participant_id_2, max_signers, min_signers, &mut rng).unwrap();
    received_round_1_pkgs_participant_1.insert(participant_id_2, round_1_pkg_participant_2);

    let mut received_round_2_pkgs_participant_1 = BTreeMap::new();
    let mut received_round_2_pkgs_participant_2 = BTreeMap::new();
    let (round_2_secret_pkg_participant_1, round_2_pkgs_participant_1) = part2(
        round_1_secret_pkg_participant_1,
        &received_round_1_pkgs_participant_1,
    )
    .unwrap();
    for (_, pkg) in round_2_pkgs_participant_1 {
        received_round_2_pkgs_participant_2.insert(participant_id_1, pkg);
    }

    let (_round_2_secret_pkg_participant_2, round_2_pkgs_participant_2) = part2(
        round_1_secret_pkg_participant_2,
        &received_round_1_pkgs_participant_2,
    )
    .unwrap();
    for (_, pkg) in round_2_pkgs_participant_2 {
        received_round_2_pkgs_participant_1.insert(participant_id_2, pkg);
    }

    let (round_3_key_pkg_participant_1, _round_3_pub_pkg_participant_1) = part3(
        &round_2_secret_pkg_participant_1,
        &received_round_1_pkgs_participant_1,
        &received_round_2_pkgs_participant_1,
    )
    .unwrap();

    let key_pkg_json = serde_json::to_string(&round_3_key_pkg_participant_1).unwrap();
    std::fs::write("test_secret_key_package.json", key_pkg_json).unwrap();
}
