use thiserror::Error;

pub type Result<T, E = RvRsError> = std::result::Result<T, E>;

#[derive(Error, Debug, Clone)]
pub enum RvRsError {

}
