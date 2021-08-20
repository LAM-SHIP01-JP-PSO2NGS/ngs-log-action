use thiserror::Error;

#[derive(Debug, Error)]
pub enum NgsLogActionError {
 #[error("error-code: {0}")]
 ErrorCode(u32),
}
