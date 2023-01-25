/// The global configuration for the driver and its components.
pub struct Conf {
    /// URL of the Celestia Node
    celestia_node_url: String,

    /// Namespace that we want to work in
    namespace_id: [u8; 8],
}

impl Conf {
    pub fn new(celestia_node_url: String, namespace_id: [u8; 8]) -> Self {
        Self {
            namespace_id,
            celestia_node_url
        }
    }
}
