use astria_core::{
    primitive::v1::{
        asset,
        RollupId,
        ROLLUP_ID_LEN,
    },
    protocol::transaction::v1alpha1::action::RollupDataSubmission,
    Protobuf as _,
};

fn encoded_len(action: &RollupDataSubmission) -> usize {
    use prost::Message as _;
    action.to_raw().encoded_len()
}

pub(crate) fn sequence_action_with_n_bytes(n: usize) -> RollupDataSubmission {
    RollupDataSubmission {
        rollup_id: RollupId::new([0; ROLLUP_ID_LEN]),
        data: vec![0; n].into(),
        fee_asset: "nria"
            .parse::<asset::Denom>()
            .unwrap()
            .to_ibc_prefixed()
            .into(),
    }
}

pub(crate) fn empty_sequence_action() -> RollupDataSubmission {
    sequence_action_with_n_bytes(0)
}

pub(crate) fn sequence_action_of_max_size(max: usize) -> RollupDataSubmission {
    // an action where the data part is exactly max bytes long
    let big_action = sequence_action_with_n_bytes(max);
    // the number of bytes past max
    let excess = encoded_len(&big_action).saturating_sub(max);
    // an action with just so many bytes that the encoded len is =< max
    // note that this does not guarantee == max since the len part of
    // len-delimited fields is var-int encoded.
    sequence_action_with_n_bytes(max.saturating_sub(excess))
}
