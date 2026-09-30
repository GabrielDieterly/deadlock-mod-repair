#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("{0}")]
  InvalidInput(String),
  #[error("{0}")]
  ModInvalid(String),
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  Json(#[from] serde_json::Error),
  #[error(transparent)]
  Archive(#[from] source2_model::error::Source2Error),
  #[error(transparent)]
  Pack(#[from] vpkmanager::VpkManagerError),
}
