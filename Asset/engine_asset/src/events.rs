use crate::{
    error::AssetError,
    id::{AssetId, AssetTypeId},
    path::AssetPath,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetLoadState {
    Unloaded,
    Queued,
    LoadingBytes,
    DecodingCpu,
    WaitingForDependencies,
    LoadedCpu,
    UploadingGpu,
    Ready,
    Failed,
    Cancelled,
    Reloading,
    Unloading,
}

impl Default for AssetLoadState {
    fn default() -> Self {
        Self::Unloaded
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetEvent {
    LoadRequested {
        id: AssetId,
        path: Option<AssetPath>,
        asset_type: AssetTypeId,
    },
    LoadStarted {
        id: AssetId,
    },
    LoadedCpu {
        id: AssetId,
    },
    Ready {
        id: AssetId,
    },
    Failed {
        id: AssetId,
        error: AssetError,
    },
    Cancelled {
        id: AssetId,
    },
    ReloadStarted {
        id: AssetId,
    },
    Reloaded {
        id: AssetId,
    },
    Unloaded {
        id: AssetId,
    },
    DependencyReady {
        id: AssetId,
        dependency: AssetId,
    },
    DependencyFailed {
        id: AssetId,
        dependency: AssetId,
        error: AssetError,
    },
    GpuUploadQueued {
        id: AssetId,
    },
    GpuUploadFinished {
        id: AssetId,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetEventCursor {
    pub(crate) index: usize,
}
