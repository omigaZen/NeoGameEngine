use std::fmt;

use crate::{
    id::{AssetId, AssetTypeId},
    path::AssetPath,
};

pub type AssetResult<T> = Result<T, AssetError>;
pub type AssetLoadError = AssetError;
pub type ImportError = AssetError;
pub type CookError = AssetError;

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum AssetError {
    #[error("asset not found: {id:?}")]
    AssetNotFound { id: AssetId },

    #[error("asset path not found: {path:?}")]
    PathNotFound { path: AssetPath },

    #[error("asset address not found: {address}")]
    AddressNotFound { address: String },

    #[error("loader not found for extension: {extension}")]
    LoaderNotFound { extension: String },

    #[error("loader not found for asset type: {asset_type:?}")]
    LoaderForTypeNotFound { asset_type: AssetTypeId },

    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("io error: {message}")]
    Io { message: String },

    #[error("decode error: {message}")]
    Decode { message: String },

    #[error("import error: {message}")]
    Import { message: String },

    #[error("cook error: {message}")]
    Cook { message: String },

    #[error("bundle error: {message}")]
    Bundle { message: String },

    #[error("gpu upload failed: {message}")]
    GpuUpload { message: String },

    #[error("dependency failed: asset {asset:?}, dependency {dependency:?}")]
    DependencyFailed { asset: AssetId, dependency: AssetId },

    #[error("cyclic dependency detected")]
    CyclicDependency,

    /// Reserved for future explicit insertion APIs that reject replacing live assets.
    #[error("asset is already loaded: {id:?}")]
    AlreadyLoaded { id: AssetId },

    #[error("asset is not loaded: {id:?}")]
    NotLoaded { id: AssetId },

    #[error("unsupported asset capability: {0}")]
    Unsupported(&'static str),
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum AssetIoError {
    #[error("{action} failed: file not found: {path}")]
    NotFound { path: String, action: AssetIoAction },

    #[error("{action} failed: permission denied: {path}, {message}")]
    PermissionDenied {
        path: String,
        action: AssetIoAction,
        message: String,
    },

    #[error("{action} failed: {path}, {message}")]
    ReadFailed {
        path: String,
        action: AssetIoAction,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetIoAction {
    Read,
    ReadRange,
    Metadata,
    List,
}

impl fmt::Display for AssetIoAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action = match self {
            AssetIoAction::Read => "read",
            AssetIoAction::ReadRange => "read_range",
            AssetIoAction::Metadata => "metadata",
            AssetIoAction::List => "list",
        };
        f.write_str(action)
    }
}

impl AssetIoError {
    pub fn action(&self) -> AssetIoAction {
        match self {
            Self::NotFound { action, .. }
            | Self::PermissionDenied { action, .. }
            | Self::ReadFailed { action, .. } => *action,
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::NotFound { path, .. }
            | Self::PermissionDenied { path, .. }
            | Self::ReadFailed { path, .. } => path,
        }
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            Self::NotFound { .. } => None,
            Self::PermissionDenied { message, .. } | Self::ReadFailed { message, .. } => {
                Some(message)
            }
        }
    }

    pub fn with_action(self, action: AssetIoAction) -> Self {
        match self {
            Self::NotFound { path, .. } => Self::NotFound { path, action },
            Self::PermissionDenied { path, message, .. } => Self::PermissionDenied {
                path,
                action,
                message,
            },
            Self::ReadFailed { path, message, .. } => Self::ReadFailed {
                path,
                action,
                message,
            },
        }
    }
}

impl From<AssetIoError> for AssetError {
    fn from(value: AssetIoError) -> Self {
        Self::Io {
            message: format!("asset io {} `{}`: {}", value.action(), value.path(), value),
        }
    }
}
