use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ClientError {
    #[error("Reqwest: {0}")]
    Http(String),
}
