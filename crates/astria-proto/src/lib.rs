pub mod execution {
    include!(concat!(env!("OUT_DIR"), "/astria.execution.v1.rs"));
}

pub mod sequencer_relayer {
    include!(concat!(env!("OUT_DIR"), "/astria.sequencer_relayer.v1.rs"));
}
