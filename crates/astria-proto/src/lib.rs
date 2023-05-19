pub mod execution {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/astria.execution.v1.rs"));
    }
}

pub mod sequencer {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/astria.sequencer.v1.rs"));
    }
}
