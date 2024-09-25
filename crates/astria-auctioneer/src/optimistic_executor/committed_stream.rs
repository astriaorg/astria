struct Client {}

struct CommittedBlockStream {
    client: Client,
}

impl CommittedBlockStream {
    pub(super) fn new(_sequencer_grpc_endpoint: String) -> Self {
        Self {
            client: Client {},
        }
    }
}
