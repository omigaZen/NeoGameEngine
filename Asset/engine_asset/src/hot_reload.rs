use crate::{
    error::AssetIoError,
    events::AssetLoadState,
    id::{AssetId, AssetTypeId},
    io::AssetIoMetadata,
    path::AssetPath,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotReloadDependencyPolicy {
    Direct,
    Transitive,
}

impl Default for HotReloadDependencyPolicy {
    fn default() -> Self {
        Self::Direct
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadChange {
    pub id: Option<AssetId>,
    pub path: AssetPath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadDependencyPlan {
    pub changed: AssetId,
    pub changed_path: Option<AssetPath>,
    pub policy: HotReloadDependencyPolicy,
    pub dependents: Vec<AssetId>,
}

impl HotReloadDependencyPlan {
    pub fn has_dependents(&self) -> bool {
        !self.dependents.is_empty()
    }

    pub fn reload_order(&self) -> Vec<AssetId> {
        let mut order = Vec::with_capacity(self.dependents.len() + 1);
        order.push(self.changed);
        order.extend(self.dependents.iter().copied());
        order
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotReloadRollbackRetention {
    None,
    Cpu,
    CpuAndGpu,
}

impl HotReloadRollbackRetention {
    pub fn retains_cpu(self) -> bool {
        matches!(self, Self::Cpu | Self::CpuAndGpu)
    }

    pub fn retains_gpu(self) -> bool {
        matches!(self, Self::CpuAndGpu)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadRollbackPolicyReport {
    pub asset_type: AssetTypeId,
    pub type_name: String,
    pub retention: HotReloadRollbackRetention,
    pub requires_previous_ready_state: bool,
    pub overridden: bool,
}

impl HotReloadRollbackPolicyReport {
    pub fn can_retain_previous_ready_state(&self) -> bool {
        self.retention.retains_cpu()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadRollbackAssetReport {
    pub id: AssetId,
    pub path: Option<AssetPath>,
    pub current_state: AssetLoadState,
    pub policy: HotReloadRollbackPolicyReport,
    pub can_rollback_now: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadPolicyReport {
    pub dependency_policy: HotReloadDependencyPolicy,
    pub rollback_policies: Vec<HotReloadRollbackPolicyReport>,
    pub watch_backend: HotReloadWatchBackend,
    pub async_watch: HotReloadAsyncWatchReport,
    pub watches: Vec<HotReloadWatch>,
    pub watch_statuses: Vec<HotReloadWatchStatus>,
    pub queued_changes: Vec<HotReloadChange>,
    pub last_poll: HotReloadPollReport,
}

impl HotReloadPolicyReport {
    pub fn watched_paths(&self) -> usize {
        self.watches.len()
    }

    pub fn queued_changes(&self) -> usize {
        self.queued_changes.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotReloadWatchBackend {
    PollingMetadata,
    AsyncNotification,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HotReloadAsyncWatchLifecycle {
    #[default]
    Stopped,
    Running,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HotReloadAsyncWatchReport {
    pub lifecycle: HotReloadAsyncWatchLifecycle,
    pub watched_paths: usize,
    pub pending_notifications: usize,
    pub received_notifications: u64,
    pub delivered_notifications: u64,
    pub dropped_notifications: u64,
    pub errors: Vec<HotReloadWatchError>,
}

impl HotReloadAsyncWatchReport {
    pub fn is_running(&self) -> bool {
        self.lifecycle == HotReloadAsyncWatchLifecycle::Running
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadWatchStatus {
    pub path: AssetPath,
    pub backend: HotReloadWatchBackend,
    pub queued: bool,
    pub last_metadata: AssetIoMetadata,
    pub last_error: Option<AssetIoError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadWatch {
    pub path: AssetPath,
    pub backend: HotReloadWatchBackend,
    pub last_metadata: AssetIoMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotReloadWatchError {
    pub path: AssetPath,
    pub error: AssetIoError,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HotReloadPollReport {
    pub watched_paths: usize,
    pub unchanged_paths: usize,
    pub changed: Vec<HotReloadChange>,
    pub debounced_changes: usize,
    pub errors: Vec<HotReloadWatchError>,
    pub async_notifications: usize,
    pub dropped_notifications: usize,
}
