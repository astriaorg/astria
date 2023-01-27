/// The global configuration for the driver and its components.
#[allow(dead_code)] // TODO - remove after developing
pub struct Conf {
    /// URL of the Celestia Node
    pub celestia_node_url: String,

    /// Namespace that we want to work in
    pub namespace_id: [u8; 8],
}

impl Conf {
    pub fn new(celestia_node_url: String, namespace_id: [u8; 8]) -> Self {
        Self {
            namespace_id,
            celestia_node_url
        }
    }
}
