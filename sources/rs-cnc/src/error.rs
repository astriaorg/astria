use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CelestiaNodeClientError {
    /// An error from our Reqwest http client
    #[error("Reqwest: {0}")]
    HttpClient(String),
}
