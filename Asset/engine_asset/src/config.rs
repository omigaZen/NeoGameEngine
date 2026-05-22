use std::path::PathBuf;

use crate::{
    error::{AssetError, AssetResult},
    features::{asset_feature_status, AssetFeature, AssetFeatureStatus},
    hot_reload::HotReloadDependencyPolicy,
    id::AssetTypeId,
};

#[derive(Clone, Debug)]
pub struct AssetServerConfig {
    pub root: PathBuf,
    pub cooked_root: PathBuf,
    pub enable_hot_reload: bool,
    pub hot_reload_dependency_policy: HotReloadDependencyPolicy,
    pub enable_async_loading: bool,
    pub worker_threads: usize,
    pub max_io_jobs_per_frame: usize,
    pub max_cpu_jobs_per_frame: usize,
    pub max_gpu_uploads_per_frame: usize,
    pub gc: AssetGcConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetLoadingExecutionMode {
    Synchronous,
    WorkerAsync,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetLoadingFeatureDiagnostic {
    pub feature: AssetFeature,
    pub message: &'static str,
    pub error: Option<AssetError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetLoadingPolicyReport {
    pub async_loading_feature: AssetFeatureStatus,
    pub parallel_feature: AssetFeatureStatus,
    pub requested_async_loading: bool,
    pub requested_worker_threads: usize,
    pub effective_worker_threads: usize,
    pub max_io_jobs_per_frame: usize,
    pub max_cpu_jobs_per_frame: usize,
    pub effective_jobs_per_frame: usize,
    pub mode: AssetLoadingExecutionMode,
    pub diagnostics: Vec<AssetLoadingFeatureDiagnostic>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetAsyncWorkerPoolReport {
    pub enabled: bool,
    pub desired_workers: usize,
    pub active_workers: usize,
    pub in_flight_jobs: usize,
    pub dispatched_jobs: u64,
    pub completed_jobs: u64,
    pub worker_threads_started: u64,
    pub shutdowns: u64,
}

impl AssetLoadingPolicyReport {
    pub fn from_config(config: &AssetServerConfig, effective_jobs_per_frame: usize) -> Self {
        let async_loading_feature = asset_feature_status(AssetFeature::AsyncLoading);
        let parallel_feature = asset_feature_status(AssetFeature::Parallel);
        let mut diagnostics = Vec::new();

        if config.enable_async_loading && !async_loading_feature.enabled {
            diagnostics.push(AssetLoadingFeatureDiagnostic {
                feature: AssetFeature::AsyncLoading,
                message: "config requested async loading but the async_loading feature is disabled",
                error: Some(AssetError::Unsupported(
                    AssetFeature::AsyncLoading.unsupported_message(),
                )),
            });
        }

        if config.worker_threads > 1 && !parallel_feature.enabled {
            diagnostics.push(AssetLoadingFeatureDiagnostic {
                feature: AssetFeature::Parallel,
                message:
                    "config requested parallel worker threads but the parallel feature is disabled",
                error: Some(AssetError::Unsupported(
                    AssetFeature::Parallel.unsupported_message(),
                )),
            });
        }

        let mode = if config.enable_async_loading && async_loading_feature.enabled {
            AssetLoadingExecutionMode::WorkerAsync
        } else {
            AssetLoadingExecutionMode::Synchronous
        };
        let effective_worker_threads = if matches!(mode, AssetLoadingExecutionMode::WorkerAsync) {
            if config.worker_threads > 1 && parallel_feature.enabled {
                config.worker_threads
            } else {
                1
            }
        } else {
            0
        };

        Self {
            async_loading_feature,
            parallel_feature,
            requested_async_loading: config.enable_async_loading,
            requested_worker_threads: config.worker_threads,
            effective_worker_threads,
            max_io_jobs_per_frame: config.max_io_jobs_per_frame,
            max_cpu_jobs_per_frame: config.max_cpu_jobs_per_frame,
            effective_jobs_per_frame,
            mode,
            diagnostics,
        }
    }

    pub fn first_error(&self) -> Option<&AssetError> {
        self.diagnostics
            .iter()
            .find_map(|diagnostic| diagnostic.error.as_ref())
    }

    pub fn require_supported(&self) -> AssetResult<()> {
        if let Some(error) = self.first_error() {
            Err(error.clone())
        } else {
            Ok(())
        }
    }
}

impl Default for AssetServerConfig {
    fn default() -> Self {
        Self {
            root: "assets/source".into(),
            cooked_root: "assets/cooked".into(),
            enable_hot_reload: false,
            hot_reload_dependency_policy: HotReloadDependencyPolicy::Direct,
            enable_async_loading: false,
            worker_threads: 0,
            max_io_jobs_per_frame: 64,
            max_cpu_jobs_per_frame: 64,
            max_gpu_uploads_per_frame: 64,
            gc: AssetGcConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetGcConfig {
    pub enabled: bool,
    pub unload_after_unused_frames: u64,
    pub memory_budget_bytes: Option<u64>,
    pub type_memory_budgets: Vec<AssetTypeMemoryBudget>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetTypeMemoryBudget {
    pub asset_type: AssetTypeId,
    pub memory_budget_bytes: Option<u64>,
    pub cpu_budget_bytes: Option<u64>,
    pub gpu_budget_bytes: Option<u64>,
}

impl AssetTypeMemoryBudget {
    pub fn total(asset_type: AssetTypeId, bytes: u64) -> Self {
        Self {
            asset_type,
            memory_budget_bytes: Some(bytes),
            cpu_budget_bytes: None,
            gpu_budget_bytes: None,
        }
    }

    pub fn cpu(asset_type: AssetTypeId, bytes: u64) -> Self {
        Self {
            asset_type,
            memory_budget_bytes: None,
            cpu_budget_bytes: Some(bytes),
            gpu_budget_bytes: None,
        }
    }

    pub fn gpu(asset_type: AssetTypeId, bytes: u64) -> Self {
        Self {
            asset_type,
            memory_budget_bytes: None,
            cpu_budget_bytes: None,
            gpu_budget_bytes: Some(bytes),
        }
    }

    pub fn cpu_gpu(asset_type: AssetTypeId, cpu_bytes: u64, gpu_bytes: u64) -> Self {
        Self {
            asset_type,
            memory_budget_bytes: None,
            cpu_budget_bytes: Some(cpu_bytes),
            gpu_budget_bytes: Some(gpu_bytes),
        }
    }

    pub fn has_budget(&self) -> bool {
        self.memory_budget_bytes.is_some()
            || self.cpu_budget_bytes.is_some()
            || self.gpu_budget_bytes.is_some()
    }
}

impl Default for AssetGcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            unload_after_unused_frames: 300,
            memory_budget_bytes: None,
            type_memory_budgets: Vec::new(),
        }
    }
}
