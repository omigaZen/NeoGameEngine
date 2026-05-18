pub type GraphicsResult<T> = Result<T, GraphicsError>;

#[derive(Debug, thiserror::Error)]
pub enum GraphicsError {
    #[error("graphics adapter not found")]
    AdapterNotFound,
    #[error("graphics device creation failed: {0}")]
    DeviceCreationFailed(String),
    #[error("surface creation failed: {0}")]
    SurfaceCreationFailed(String),
    #[error("surface configuration failed: {0}")]
    SurfaceConfigurationFailed(String),
    #[error("surface is out of memory")]
    SurfaceOutOfMemory,
    #[error("surface acquire timed out")]
    SurfaceTimeout,
    #[error("invalid graphics resource: {0}")]
    InvalidResource(String),
    #[error("backend error: {0}")]
    Backend(String),
}
