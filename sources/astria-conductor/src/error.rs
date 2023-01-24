use thiserror::Error;

pub type Result<T, E = RvrsError> = std::result::Result<T, E>;

#[derive(Error, Debug, Clone)]
pub enum RvrsError {

}
