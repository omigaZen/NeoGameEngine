pub type PlatformResult<T> = Result<T, PlatformError>;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("window creation failed: {0}")]
    WindowCreationFailed(String),
    #[error("window not found")]
    WindowNotFound,
    #[error("invalid window handle")]
    InvalidWindowHandle,
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("file read failed: {0}")]
    FileReadFailed(String),
    #[error("file write failed: {0}")]
    FileWriteFailed(String),
    #[error("cursor operation failed: {0}")]
    CursorOperationFailed(String),
    #[error("backend error: {0}")]
    BackendError(String),
}
