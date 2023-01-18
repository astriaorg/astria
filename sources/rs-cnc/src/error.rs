use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CelestiaNodeClientError {
    #[error("Reqwest: {0}")]
    HttpClient(String),
}
