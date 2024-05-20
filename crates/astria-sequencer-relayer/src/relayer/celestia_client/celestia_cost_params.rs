/// Values set by the Celestia app which affect the blob submission fees.
///
/// These are all provided by the Celestia app via its gRPC interface.
#[derive(Copy, Clone, Debug, PartialEq, Default, serde::Serialize)]
pub(super) struct CelestiaCostParams {
    gas_per_blob_byte: u32,
    tx_size_cost_per_byte: u64,
    min_gas_price: f64,
}

impl CelestiaCostParams {
    pub(super) fn new(
        gas_per_blob_byte: u32,
        tx_size_cost_per_byte: u64,
        min_gas_price: f64,
    ) -> Self {
        Self {
            gas_per_blob_byte,
            tx_size_cost_per_byte,
            min_gas_price,
        }
    }

    pub(super) fn gas_per_blob_byte(&self) -> u32 {
        self.gas_per_blob_byte
    }

    pub(super) fn tx_size_cost_per_byte(&self) -> u64 {
        self.tx_size_cost_per_byte
    }

    pub(super) fn min_gas_price(&self) -> f64 {
        self.min_gas_price
    }
}
