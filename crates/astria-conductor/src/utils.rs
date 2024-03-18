// use celestia_client::celestia_types::Height as CelestiaHeight;
// use sequencer_client::tendermint::block::Height as SequencerHeight;

// /// A necessary evil because the celestia client code uses a forked tendermint-rs.
// pub(crate) trait IncrementableHeight {
//     fn increment(self) -> Self;
// }

// impl IncrementableHeight for CelestiaHeight {
//     fn increment(self) -> Self {
//         self.increment()
//     }
// }

// impl IncrementableHeight for SequencerHeight {
//     fn increment(self) -> Self {
//         self.increment()
//     }
// }
