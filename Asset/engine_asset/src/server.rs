use std::{
    any::Any,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};

#[cfg(feature = "async_loading")]
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    thread::{self, JoinHandle},
};

#[cfg(feature = "async_loading")]
use crate::config::AssetAsyncWorkerPoolReport;
use crate::{
    asset::{Asset, AssetDependencyReference},
    assets::{
        AnimationClip, AnimationLoader, AudioClip, AudioLoader, Font, FontLoader, Material,
        MaterialLoader, Mesh, MeshLoader, PhysicsMesh, PhysicsMeshLoader, Prefab, PrefabLoader,
        SceneAsset, SceneLoader, Shader, ShaderLoader, Skeleton, SkeletonLoader, Texture,
        TextureLoader,
    },
    bundle::{BundleId, MountedBundle},
    config::{AssetLoadingPolicyReport, AssetServerConfig, AssetTypeMemoryBudget},
    dependency::{DependencyGraph, DependencyGraphReport, DependencyScopeReport},
    error::{AssetError, AssetResult},
    events::{AssetEvent, AssetEventCursor, AssetLoadState},
    features::{require_asset_feature, AssetFeature},
    gpu_upload::{GpuUploadCommand, GpuUploadResult},
    handle::{Handle, HandleLifecycleTracker, HandleStrength, UntypedHandle},
    hot_reload::{
        HotReloadAsyncWatchReport, HotReloadChange, HotReloadDependencyPlan,
        HotReloadDependencyPolicy, HotReloadPolicyReport, HotReloadPollReport,
        HotReloadRollbackAssetReport, HotReloadRollbackPolicyReport, HotReloadRollbackRetention,
        HotReloadWatch, HotReloadWatchBackend,
    },
    id::{AssetId, AssetTypeId},
    io::{AssetIo, FileSystemAssetIo},
    loader::{
        AssetLoadGroup, AssetLoadGroupId, AssetLoader, AssetLoaderRegistry, LoadPriority,
        LoadProgress, LoadRequest, LoadScheduler, LoadedAsset, LoaderSettings,
    },
    metadata::AssetMetadata,
    path::AssetPath,
    ref_asset::AssetRef,
    registry::AssetRegistry,
    storage::{Assets, ErasedAssets},
    streaming::{StreamingRegion, StreamingRegionId},
};

#[cfg(feature = "bundle")]
use crate::bundle::{
    AssetPackageActivation, AssetPackageArtifactReport, AssetPackageArtifactStore,
    AssetPackageConflictReport, AssetPackageRegistry, AssetPackageUpdatePolicy,
    AssetPackageUpdateReport, BundleEntry, BundleManifest, BundleReader, MountedBundleRegistry,
};
#[cfg(feature = "hot_reload")]
use crate::hot_reload::{HotReloadWatchError, HotReloadWatchStatus};

#[derive(Default)]
struct WaitingAsset {
    asset_type: AssetTypeId,
    asset: Option<Box<dyn Any + Send + Sync>>,
    gpu_upload: Option<GpuUploadCommand>,
    reloaded: bool,
}

struct PendingGpuUpload {
    asset_type: AssetTypeId,
    asset: Box<dyn Any + Send + Sync>,
    reloaded: bool,
}

#[cfg(feature = "async_loading")]
struct AsyncLoadResult {
    request: LoadRequest,
    outcome: AsyncLoadOutcome,
}

#[cfg(feature = "async_loading")]
struct AsyncLoadJob {
    request: LoadRequest,
    path: AssetPath,
    io: Arc<dyn AssetIo>,
    loaders: AssetLoaderRegistry,
    registry: AssetRegistry,
}

#[cfg(feature = "async_loading")]
enum AsyncLoadOutcome {
    Loaded {
        path: AssetPath,
        loaded: LoadedAsset,
        dependencies: Vec<crate::loader::LoadDependency>,
        subresources: Vec<(AssetPath, AssetTypeId, AssetId)>,
    },
    Failed {
        state: AssetLoadState,
        error: AssetError,
    },
}

#[cfg(feature = "async_loading")]
struct AsyncWorkerPool {
    worker_count: usize,
    job_tx: Option<Sender<AsyncLoadJob>>,
    result_rx: Receiver<AsyncLoadResult>,
    workers: Vec<JoinHandle<()>>,
}

#[cfg(feature = "async_loading")]
impl AsyncWorkerPool {
    fn new(worker_count: usize) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<AsyncLoadJob>();
        let (result_tx, result_rx) = mpsc::channel::<AsyncLoadResult>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let mut workers = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let job_rx = Arc::clone(&job_rx);
            let result_tx = result_tx.clone();
            workers.push(thread::spawn(move || loop {
                let job = {
                    let receiver = job_rx
                        .lock()
                        .expect("async asset worker receiver mutex poisoned");
                    receiver.recv()
                };
                let Ok(job) = job else {
                    break;
                };
                let result = run_async_load_request(
                    job.request,
                    job.path,
                    job.io,
                    job.loaders,
                    job.registry,
                );
                if result_tx.send(result).is_err() {
                    break;
                }
            }));
        }
        Self {
            worker_count,
            job_tx: Some(job_tx),
            result_rx,
            workers,
        }
    }

    fn dispatch(&self, job: AsyncLoadJob) -> Result<(), AsyncLoadJob> {
        let Some(sender) = &self.job_tx else {
            return Err(job);
        };
        sender.send(job).map_err(|error| error.0)
    }

    fn try_recv(&self) -> Result<AsyncLoadResult, mpsc::TryRecvError> {
        self.result_rx.try_recv()
    }

    fn worker_count(&self) -> usize {
        self.worker_count
    }
}

#[cfg(feature = "async_loading")]
impl Drop for AsyncWorkerPool {
    fn drop(&mut self) {
        self.job_tx.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ReloadRollback {
    asset_type: AssetTypeId,
    previous_state: AssetLoadState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetMemoryStats {
    pub assets: usize,
    pub cpu_bytes: u64,
    pub gpu_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetMemoryInfo {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub state: AssetLoadState,
    pub cpu_bytes: u64,
    pub gpu_bytes: u64,
    pub last_used_frame: u64,
    pub strong_count: usize,
    pub weak_count: usize,
    pub dependency_ref_count: usize,
    pub resident: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetMemoryReport {
    pub total_cpu_bytes: u64,
    pub total_gpu_bytes: u64,
    pub asset_count: usize,
    pub assets: Vec<AssetMemoryInfo>,
    pub by_type: Vec<AssetTypeMemoryReport>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetTypeMemoryReport {
    pub asset_type: AssetTypeId,
    pub asset_count: usize,
    pub cpu_bytes: u64,
    pub gpu_bytes: u64,
    pub strong_count: usize,
    pub weak_count: usize,
    pub dependency_ref_count: usize,
    pub resident_assets: usize,
}

pub struct AssetServer {
    config: AssetServerConfig,
    registry: AssetRegistry,
    io: Arc<dyn AssetIo>,
    loaders: AssetLoaderRegistry,
    scheduler: LoadScheduler,
    storages: HashMap<AssetTypeId, Box<dyn ErasedAssets>>,
    type_names: HashMap<AssetTypeId, String>,
    events: Vec<AssetEvent>,
    dependencies: DependencyGraph,
    waiting_assets: HashMap<AssetId, WaitingAsset>,
    gpu_uploads: VecDeque<GpuUploadCommand>,
    pending_gpu: HashMap<AssetId, PendingGpuUpload>,
    #[cfg(feature = "async_loading")]
    async_worker_pool: Option<AsyncWorkerPool>,
    #[cfg(feature = "async_loading")]
    async_in_flight: HashMap<AssetId, LoadRequest>,
    #[cfg(feature = "async_loading")]
    cancelled_async_loads: HashSet<AssetId>,
    #[cfg(feature = "async_loading")]
    async_jobs_dispatched: u64,
    #[cfg(feature = "async_loading")]
    async_jobs_completed: u64,
    #[cfg(feature = "async_loading")]
    async_worker_threads_started: u64,
    #[cfg(feature = "async_loading")]
    async_worker_pool_shutdowns: u64,
    #[cfg(feature = "hot_reload")]
    hot_reload_queue: VecDeque<HotReloadChange>,
    #[cfg(feature = "hot_reload")]
    hot_reload_watches: HashMap<AssetPath, HotReloadWatch>,
    #[cfg(feature = "hot_reload")]
    hot_reload_async_watch_running: bool,
    #[cfg(feature = "hot_reload")]
    hot_reload_async_notifications: VecDeque<AssetPath>,
    #[cfg(feature = "hot_reload")]
    hot_reload_async_received_notifications: u64,
    #[cfg(feature = "hot_reload")]
    hot_reload_async_delivered_notifications: u64,
    #[cfg(feature = "hot_reload")]
    hot_reload_async_dropped_notifications: u64,
    #[cfg(feature = "hot_reload")]
    last_hot_reload_async_errors: Vec<HotReloadWatchError>,
    last_hot_reload_poll: HotReloadPollReport,
    reload_rollbacks: HashMap<AssetId, ReloadRollback>,
    hot_reload_rollback_overrides: HashMap<AssetTypeId, HotReloadRollbackRetention>,
    fallback_states: HashMap<AssetId, AssetLoadState>,
    fallback_errors: HashMap<AssetId, AssetError>,
    handle_lifecycle: Arc<HandleLifecycleTracker>,
    dependency_ref_counts: HashMap<AssetId, usize>,
    groups: HashMap<AssetLoadGroupId, Vec<AssetId>>,
    next_group_id: u64,
    #[cfg(feature = "bundle")]
    mounted_bundles: HashMap<BundleId, MountedBundle>,
    #[cfg(feature = "bundle")]
    asset_package_registry: AssetPackageRegistry,
    #[cfg(feature = "bundle")]
    next_bundle_id: u64,
    #[cfg(feature = "streaming")]
    streaming_regions: HashMap<StreamingRegionId, StreamingRegion>,
    #[cfg(feature = "streaming")]
    streaming_residency_counts: HashMap<AssetId, usize>,
    #[cfg(feature = "streaming")]
    next_streaming_region_id: u64,
    frame_index: u64,
}

impl AssetServer {
    pub fn new(config: AssetServerConfig) -> Self {
        let io: Arc<dyn AssetIo> = Arc::new(FileSystemAssetIo::new(config.root.clone()));
        let mut server = Self {
            config,
            registry: AssetRegistry::new(),
            io,
            loaders: AssetLoaderRegistry::new(),
            scheduler: LoadScheduler::new(),
            storages: HashMap::new(),
            type_names: HashMap::new(),
            events: Vec::new(),
            dependencies: DependencyGraph::new(),
            waiting_assets: HashMap::new(),
            gpu_uploads: VecDeque::new(),
            pending_gpu: HashMap::new(),
            #[cfg(feature = "async_loading")]
            async_worker_pool: None,
            #[cfg(feature = "async_loading")]
            async_in_flight: HashMap::new(),
            #[cfg(feature = "async_loading")]
            cancelled_async_loads: HashSet::new(),
            #[cfg(feature = "async_loading")]
            async_jobs_dispatched: 0,
            #[cfg(feature = "async_loading")]
            async_jobs_completed: 0,
            #[cfg(feature = "async_loading")]
            async_worker_threads_started: 0,
            #[cfg(feature = "async_loading")]
            async_worker_pool_shutdowns: 0,
            #[cfg(feature = "hot_reload")]
            hot_reload_queue: VecDeque::new(),
            #[cfg(feature = "hot_reload")]
            hot_reload_watches: HashMap::new(),
            #[cfg(feature = "hot_reload")]
            hot_reload_async_watch_running: false,
            #[cfg(feature = "hot_reload")]
            hot_reload_async_notifications: VecDeque::new(),
            #[cfg(feature = "hot_reload")]
            hot_reload_async_received_notifications: 0,
            #[cfg(feature = "hot_reload")]
            hot_reload_async_delivered_notifications: 0,
            #[cfg(feature = "hot_reload")]
            hot_reload_async_dropped_notifications: 0,
            #[cfg(feature = "hot_reload")]
            last_hot_reload_async_errors: Vec::new(),
            last_hot_reload_poll: HotReloadPollReport::default(),
            reload_rollbacks: HashMap::new(),
            hot_reload_rollback_overrides: HashMap::new(),
            fallback_states: HashMap::new(),
            fallback_errors: HashMap::new(),
            handle_lifecycle: Arc::new(HandleLifecycleTracker::default()),
            dependency_ref_counts: HashMap::new(),
            groups: HashMap::new(),
            next_group_id: 1,
            #[cfg(feature = "bundle")]
            mounted_bundles: HashMap::new(),
            #[cfg(feature = "bundle")]
            asset_package_registry: AssetPackageRegistry::default(),
            #[cfg(feature = "bundle")]
            next_bundle_id: 1,
            #[cfg(feature = "streaming")]
            streaming_regions: HashMap::new(),
            #[cfg(feature = "streaming")]
            streaming_residency_counts: HashMap::new(),
            #[cfg(feature = "streaming")]
            next_streaming_region_id: 1,
            frame_index: 0,
        };
        server.register_builtin_asset_types();
        server
    }

    pub fn config(&self) -> &AssetServerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AssetServerConfig {
        &mut self.config
    }

    pub fn loading_policy_report(&self) -> AssetLoadingPolicyReport {
        AssetLoadingPolicyReport::from_config(&self.config, self.loading_jobs_per_frame())
    }

    pub fn validate_loading_policy(&self) -> AssetResult<()> {
        self.loading_policy_report().require_supported()
    }

    #[cfg(feature = "async_loading")]
    pub fn async_worker_pool_report(&self) -> AssetAsyncWorkerPoolReport {
        AssetAsyncWorkerPoolReport {
            enabled: self.config.enable_async_loading,
            desired_workers: self.async_worker_limit(),
            active_workers: self
                .async_worker_pool
                .as_ref()
                .map(AsyncWorkerPool::worker_count)
                .unwrap_or(0),
            in_flight_jobs: self.async_in_flight.len(),
            dispatched_jobs: self.async_jobs_dispatched,
            completed_jobs: self.async_jobs_completed,
            worker_threads_started: self.async_worker_threads_started,
            shutdowns: self.async_worker_pool_shutdowns,
        }
    }

    #[cfg(feature = "async_loading")]
    pub fn shutdown_async_worker_pool(&mut self) -> AssetAsyncWorkerPoolReport {
        self.collect_async_load_results();
        if self.async_in_flight.is_empty() {
            self.shutdown_async_worker_pool_internal();
        }
        self.async_worker_pool_report()
    }

    pub fn set_async_loading_enabled(&mut self, enabled: bool) -> AssetResult<()> {
        if enabled {
            require_asset_feature(AssetFeature::AsyncLoading)?;
        }
        self.config.enable_async_loading = enabled;
        Ok(())
    }

    pub fn set_parallel_worker_threads(&mut self, worker_threads: usize) -> AssetResult<()> {
        if worker_threads > 1 {
            require_asset_feature(AssetFeature::Parallel)?;
        }
        self.config.worker_threads = worker_threads;
        Ok(())
    }

    pub fn registry(&self) -> &AssetRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut AssetRegistry {
        &mut self.registry
    }

    pub fn set_registry(&mut self, registry: AssetRegistry) {
        self.registry = registry;
    }

    pub fn set_io<I: AssetIo>(&mut self, io: I) {
        self.io = Arc::new(io);
    }

    pub fn register_builtin_asset_types(&mut self) {
        self.register_asset_type::<Texture>();
        self.register_asset_type::<Mesh>();
        self.register_asset_type::<Shader>();
        self.register_asset_type::<Material>();
        self.register_asset_type::<AudioClip>();
        self.register_asset_type::<AnimationClip>();
        self.register_asset_type::<Skeleton>();
        self.register_asset_type::<SceneAsset>();
        self.register_asset_type::<Prefab>();
        self.register_asset_type::<Font>();
        self.register_asset_type::<PhysicsMesh>();
    }

    pub fn register_builtin_loaders(&mut self) {
        self.register_loader(TextureLoader::new());
        self.register_loader(MeshLoader::new());
        self.register_loader(ShaderLoader::new());
        self.register_loader(MaterialLoader::new());
        self.register_loader(AudioLoader::new());
        self.register_loader(AnimationLoader::new());
        self.register_loader(SkeletonLoader::new());
        self.register_loader(SceneLoader::new());
        self.register_loader(PrefabLoader::new());
        self.register_loader(FontLoader::new());
        self.register_loader(PhysicsMeshLoader::new());
    }

    pub fn register_asset_type<T: Asset>(&mut self) {
        self.type_names.insert(T::TYPE_ID, T::TYPE_NAME.to_owned());
        self.storages
            .entry(T::TYPE_ID)
            .or_insert_with(|| Box::new(Assets::<T>::new()));
    }

    pub fn is_asset_type_registered<T: Asset>(&self) -> bool {
        self.storages.contains_key(&T::TYPE_ID)
    }

    pub fn register_loader<L: AssetLoader>(&mut self, loader: L) {
        self.ensure_builtin_storage_for_type(loader.asset_type());
        self.loaders.register(loader);
    }

    pub fn register_boxed_loader(&mut self, loader: Box<dyn AssetLoader>) {
        self.ensure_builtin_storage_for_type(loader.asset_type());
        self.loaders.register_boxed(loader);
    }

    pub fn loader_count(&self) -> usize {
        self.loaders.len()
    }

    pub fn load<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T> {
        self.load_with_priority(path, LoadPriority::Normal)
    }

    pub fn load_with_priority<T: Asset>(
        &mut self,
        path: impl Into<AssetPath>,
        priority: LoadPriority,
    ) -> Handle<T> {
        self.register_asset_type::<T>();
        let path = path.into();
        let id = self.registry.get_or_create(path.clone(), T::TYPE_ID);
        self.queue_request(LoadRequest {
            id,
            path: Some(path),
            asset_type: T::TYPE_ID,
            priority,
            recursive_dependencies: true,
            reload: false,
        });
        self.make_handle::<T>(id, HandleStrength::Strong)
    }

    pub fn load_by_id<T: Asset>(&mut self, id: AssetId) -> Handle<T> {
        self.load_by_id_with_priority::<T>(id, LoadPriority::Normal)
    }

    pub fn load_by_id_with_priority<T: Asset>(
        &mut self,
        id: AssetId,
        priority: LoadPriority,
    ) -> Handle<T> {
        self.register_asset_type::<T>();
        if let Some(metadata) = self.registry.get(id) {
            self.queue_request(LoadRequest {
                id,
                path: metadata.path.clone(),
                asset_type: T::TYPE_ID,
                priority,
                recursive_dependencies: true,
                reload: false,
            });
        } else {
            self.fail_asset(id, T::TYPE_ID, AssetError::AssetNotFound { id });
        }
        self.make_handle::<T>(id, HandleStrength::Strong)
    }

    pub fn load_untyped(&mut self, path: impl Into<AssetPath>) -> UntypedHandle {
        let path = path.into();
        let asset_type = path
            .extension()
            .and_then(|extension| self.loaders.asset_type_for_extension(extension))
            .unwrap_or(AssetTypeId::NIL);
        let id = self.registry.get_or_create(path.clone(), asset_type);
        if asset_type == AssetTypeId::NIL {
            self.fail_asset(
                id,
                asset_type,
                AssetError::LoaderNotFound {
                    extension: path.extension().unwrap_or("").to_owned(),
                },
            );
        } else {
            self.queue_request(LoadRequest {
                id,
                path: Some(path),
                asset_type,
                priority: LoadPriority::Normal,
                recursive_dependencies: true,
                reload: false,
            });
        }
        self.make_untyped_handle(id, asset_type, HandleStrength::Strong)
    }

    pub fn load_untyped_by_id(&mut self, id: AssetId, asset_type: AssetTypeId) -> UntypedHandle {
        if let Some(metadata) = self.registry.get(id) {
            self.queue_request(LoadRequest {
                id,
                path: metadata.path.clone(),
                asset_type,
                priority: LoadPriority::Normal,
                recursive_dependencies: true,
                reload: false,
            });
        } else {
            self.fail_asset(id, asset_type, AssetError::AssetNotFound { id });
        }
        self.make_untyped_handle(id, asset_type, HandleStrength::Strong)
    }

    pub fn preload<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T> {
        let handle = self.load_with_priority::<T>(path, LoadPriority::Background);
        handle.clone_weak()
    }

    pub fn preload_by_id<T: Asset>(&mut self, id: AssetId) -> Handle<T> {
        let handle = self.load_by_id_with_priority::<T>(id, LoadPriority::Background);
        handle.clone_weak()
    }

    pub fn load_ref<T: Asset>(&mut self, reference: &AssetRef<T>) -> Handle<T> {
        if self.registry.get(reference.id()).is_some() {
            self.load_by_id(reference.id())
        } else if let Some(path) = &reference.fallback_path {
            self.load(path.clone())
        } else {
            self.fail_asset(
                reference.id(),
                T::TYPE_ID,
                AssetError::AssetNotFound { id: reference.id() },
            );
            self.make_handle::<T>(reference.id(), HandleStrength::Strong)
        }
    }

    pub fn insert_loaded<T: Asset>(
        &mut self,
        path: impl Into<AssetPath>,
        asset: T,
    ) -> AssetResult<Handle<T>> {
        self.register_asset_type::<T>();
        let path = path.into();
        let id = self.registry.get_or_create(path.clone(), T::TYPE_ID);
        let mut metadata = self
            .registry
            .get(id)
            .cloned()
            .unwrap_or_else(|| AssetMetadata::runtime(id, path.clone(), T::TYPE_ID));
        if metadata.asset_type != AssetTypeId::NIL && metadata.asset_type != T::TYPE_ID {
            return Err(AssetError::TypeMismatch {
                expected: T::TYPE_NAME.to_owned(),
                actual: self.type_name(metadata.asset_type),
            });
        }
        metadata.asset_type = T::TYPE_ID;
        metadata.path = Some(path);
        self.insert_loaded_with_metadata(metadata, asset)
    }

    pub fn insert_loaded_by_id<T: Asset>(
        &mut self,
        id: AssetId,
        asset: T,
    ) -> AssetResult<Handle<T>> {
        self.register_asset_type::<T>();
        if let Some(metadata) = self.registry.get(id).cloned() {
            self.insert_loaded_with_metadata(metadata, asset)
        } else {
            self.insert_loaded_inner(id, None, asset)
        }
    }

    pub fn insert_loaded_with_metadata<T: Asset>(
        &mut self,
        mut metadata: AssetMetadata,
        asset: T,
    ) -> AssetResult<Handle<T>> {
        self.register_asset_type::<T>();
        if metadata.asset_type != AssetTypeId::NIL && metadata.asset_type != T::TYPE_ID {
            return Err(AssetError::TypeMismatch {
                expected: T::TYPE_NAME.to_owned(),
                actual: self.type_name(metadata.asset_type),
            });
        }
        metadata.asset_type = T::TYPE_ID;
        self.insert_loaded_inner(metadata.id, Some(metadata), asset)
    }

    pub fn load_group(&mut self, assets: &[AssetPath]) -> AssetLoadGroup {
        let handles = assets
            .iter()
            .cloned()
            .map(|path| self.load_untyped(path))
            .collect::<Vec<_>>();
        self.register_group(handles)
    }

    pub fn load_group_by_ids(&mut self, assets: &[(AssetId, AssetTypeId)]) -> AssetLoadGroup {
        let handles = assets
            .iter()
            .map(|(id, asset_type)| self.load_untyped_by_id(*id, *asset_type))
            .collect::<Vec<_>>();
        self.register_group(handles)
    }

    pub fn group_state(&self, group: &AssetLoadGroup) -> AssetLoadState {
        let mut saw_loading = false;
        let mut saw_cancelled = false;
        let mut saw_unloaded = false;
        for handle in &group.assets {
            match self.state_by_id(handle.id()) {
                AssetLoadState::Failed => return AssetLoadState::Failed,
                AssetLoadState::Cancelled => saw_cancelled = true,
                AssetLoadState::Unloaded => saw_unloaded = true,
                AssetLoadState::Ready => {}
                _ => saw_loading = true,
            }
        }
        if saw_cancelled {
            AssetLoadState::Cancelled
        } else if saw_unloaded {
            AssetLoadState::Unloaded
        } else if saw_loading {
            AssetLoadState::LoadingBytes
        } else {
            AssetLoadState::Ready
        }
    }

    pub fn group_progress(&self, group: &AssetLoadGroup) -> LoadProgress {
        let mut progress = LoadProgress {
            total_assets: group.assets.len(),
            ..LoadProgress::default()
        };
        for handle in &group.assets {
            match self.state_by_id(handle.id()) {
                AssetLoadState::Queued => progress.queued_assets += 1,
                AssetLoadState::Ready => {
                    progress.ready_assets += 1;
                    progress.bytes_loaded += self.memory_bytes_total_for_id(handle.id());
                }
                AssetLoadState::Failed => progress.failed_assets += 1,
                AssetLoadState::Cancelled => progress.cancelled_assets += 1,
                AssetLoadState::Unloaded => {}
                _ => progress.loading_assets += 1,
            }
            progress.bytes_total += self.memory_bytes_total_for_id(handle.id());
        }
        progress
    }

    pub fn release_group(&mut self, group: AssetLoadGroup) {
        self.groups.remove(&group.id);
        drop(group);
    }

    pub fn is_group_tracked(&self, id: AssetLoadGroupId) -> bool {
        self.groups.contains_key(&id)
    }

    pub fn cancel_load_by_id(&mut self, id: AssetId) -> bool {
        let Some(request) = self.scheduler.cancel(id) else {
            #[cfg(feature = "async_loading")]
            {
                if self.cancel_async_request(id) {
                    return true;
                }
            }
            return false;
        };
        self.cancel_queued_request(request);
        true
    }

    pub fn cancel_load_by_path(&mut self, path: impl Into<AssetPath>) -> bool {
        let path = path.into();
        let Some(id) = self.registry.id_from_path(&path) else {
            return false;
        };
        self.cancel_load_by_id(id)
    }

    pub fn cancel_load_group(&mut self, group: &AssetLoadGroup) -> usize {
        let mut cancelled = 0;
        for handle in &group.assets {
            if self.cancel_load_by_id(handle.id()) {
                cancelled += 1;
            }
        }
        self.groups.remove(&group.id);
        cancelled
    }

    #[cfg(feature = "bundle")]
    pub fn mount_bundle_bytes(&mut self, bytes: &[u8]) -> AssetResult<MountedBundle> {
        require_asset_feature(AssetFeature::Bundle)?;
        let reader = BundleReader::from_bytes(bytes)?;
        Ok(self.mount_bundle_manifest(reader.manifest().clone()))
    }

    #[cfg(not(feature = "bundle"))]
    pub fn mount_bundle_bytes(&mut self, bytes: &[u8]) -> AssetResult<MountedBundle> {
        let _ = bytes;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn mount_bundle_manifest(&mut self, manifest: BundleManifest) -> MountedBundle {
        let id = BundleId(self.next_bundle_id);
        self.next_bundle_id += 1;
        let mounted = MountedBundle {
            id,
            name: manifest.name.clone(),
            manifest,
        };
        self.mounted_bundles.insert(id, mounted.clone());
        mounted
    }

    #[cfg(feature = "bundle")]
    pub fn mounted_bundle(&self, id: BundleId) -> Option<&MountedBundle> {
        self.mounted_bundles.get(&id)
    }

    #[cfg(feature = "bundle")]
    pub fn mounted_bundles(&self) -> impl Iterator<Item = &MountedBundle> {
        self.mounted_bundles.values()
    }

    #[cfg(feature = "bundle")]
    pub fn mounted_bundle_registry(&self) -> MountedBundleRegistry {
        MountedBundleRegistry::from_mounted_bundles(self.mounted_bundles.values())
    }

    #[cfg(feature = "bundle")]
    pub fn restore_mounted_bundle_registry(
        &mut self,
        registry: MountedBundleRegistry,
    ) -> Vec<MountedBundle> {
        let bundles = registry.into_bundles();
        for bundle in &bundles {
            self.next_bundle_id = self.next_bundle_id.max(bundle.id.0.saturating_add(1));
            self.mounted_bundles.insert(bundle.id, bundle.clone());
        }
        bundles
    }

    #[cfg(feature = "bundle")]
    pub fn save_mounted_bundle_registry(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        require_asset_feature(AssetFeature::Bundle)?;
        self.mounted_bundle_registry().save_to_file(path)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn save_mounted_bundle_registry(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        let _ = path;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn load_mounted_bundle_registry(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<Vec<MountedBundle>> {
        require_asset_feature(AssetFeature::Bundle)?;
        let registry = MountedBundleRegistry::load_from_file(path)?;
        Ok(self.restore_mounted_bundle_registry(registry))
    }

    #[cfg(not(feature = "bundle"))]
    pub fn load_mounted_bundle_registry(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<Vec<MountedBundle>> {
        let _ = path;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn asset_package_registry(&self) -> &AssetPackageRegistry {
        &self.asset_package_registry
    }

    #[cfg(feature = "bundle")]
    pub fn asset_package_conflict_report(&self) -> AssetPackageConflictReport {
        self.asset_package_registry.conflict_report()
    }

    #[cfg(feature = "bundle")]
    pub fn preview_asset_package_update(
        &self,
        registry: &AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
    ) -> AssetResult<AssetPackageUpdateReport> {
        require_asset_feature(AssetFeature::Bundle)?;
        self.asset_package_registry.update_report(registry, policy)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn preview_asset_package_update(
        &self,
        registry: &crate::bundle::AssetPackageRegistry,
        policy: crate::bundle::AssetPackageUpdatePolicy,
    ) -> AssetResult<crate::bundle::AssetPackageUpdateReport> {
        let _ = (registry, policy);
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn activate_asset_package_registry(
        &mut self,
        registry: AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
    ) -> AssetResult<AssetPackageActivation> {
        require_asset_feature(AssetFeature::Bundle)?;
        let report = self
            .asset_package_registry
            .update_report(&registry, policy)?;
        report.require_compatible()?;
        let previous_registry = self.asset_package_registry.clone();
        let previous_mounted_bundles = self.mounted_bundles.clone();
        let previous_next_bundle_id = self.next_bundle_id;
        match self.restore_asset_package_registry(registry) {
            Ok(mounted_bundles) => Ok(AssetPackageActivation {
                report,
                mounted_bundles,
            }),
            Err(error) => {
                self.asset_package_registry = previous_registry;
                self.mounted_bundles = previous_mounted_bundles;
                self.next_bundle_id = previous_next_bundle_id;
                Err(error)
            }
        }
    }

    #[cfg(not(feature = "bundle"))]
    pub fn activate_asset_package_registry(
        &mut self,
        registry: crate::bundle::AssetPackageRegistry,
        policy: crate::bundle::AssetPackageUpdatePolicy,
    ) -> AssetResult<crate::bundle::AssetPackageActivation> {
        let _ = (registry, policy);
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn verify_asset_package_artifacts(
        &self,
        registry: &AssetPackageRegistry,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> AssetResult<AssetPackageArtifactReport> {
        require_asset_feature(AssetFeature::Bundle)?;
        AssetPackageArtifactStore::new(artifact_root.as_ref().to_path_buf())
            .verify_registry(registry)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn verify_asset_package_artifacts(
        &self,
        registry: &crate::bundle::AssetPackageRegistry,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> AssetResult<crate::bundle::AssetPackageArtifactReport> {
        let _ = (registry, artifact_root);
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn activate_asset_package_registry_from_artifacts(
        &mut self,
        registry: AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> AssetResult<AssetPackageActivation> {
        require_asset_feature(AssetFeature::Bundle)?;
        let store = AssetPackageArtifactStore::new(artifact_root.as_ref().to_path_buf());
        store.verify_registry(&registry)?.require_available()?;
        self.activate_asset_package_registry(registry, policy)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn activate_asset_package_registry_from_artifacts(
        &mut self,
        registry: crate::bundle::AssetPackageRegistry,
        policy: crate::bundle::AssetPackageUpdatePolicy,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> AssetResult<crate::bundle::AssetPackageActivation> {
        let _ = (registry, policy, artifact_root);
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn restore_asset_package_registry(
        &mut self,
        registry: AssetPackageRegistry,
    ) -> AssetResult<Vec<MountedBundle>> {
        require_asset_feature(AssetFeature::Bundle)?;
        registry.validate()?;
        let mounted = registry
            .enabled_packages()
            .map(|package| MountedBundle {
                id: package.bundle_id,
                name: package.manifest.name.clone(),
                manifest: package.manifest.clone(),
            })
            .collect::<Vec<_>>();
        for package in self.asset_package_registry.packages() {
            self.mounted_bundles.remove(&package.bundle_id);
        }
        for bundle in &mounted {
            self.next_bundle_id = self.next_bundle_id.max(bundle.id.0.saturating_add(1));
            self.mounted_bundles.insert(bundle.id, bundle.clone());
        }
        self.asset_package_registry = registry;
        Ok(mounted)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn restore_asset_package_registry(
        &mut self,
        registry: crate::bundle::AssetPackageRegistry,
    ) -> AssetResult<Vec<MountedBundle>> {
        let _ = registry;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn save_asset_package_registry(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        require_asset_feature(AssetFeature::Bundle)?;
        self.asset_package_registry.save_to_file(path)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn save_asset_package_registry(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        let _ = path;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn load_asset_package_registry(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<Vec<MountedBundle>> {
        require_asset_feature(AssetFeature::Bundle)?;
        let registry = AssetPackageRegistry::load_from_file(path)?;
        self.restore_asset_package_registry(registry)
    }

    #[cfg(not(feature = "bundle"))]
    pub fn load_asset_package_registry(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<Vec<MountedBundle>> {
        let _ = path;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn unmount_bundle(&mut self, id: BundleId) -> AssetResult<MountedBundle> {
        self.mounted_bundles
            .remove(&id)
            .ok_or_else(|| AssetError::Bundle {
                message: format!("bundle is not mounted: {id:?}"),
            })
    }

    #[cfg(not(feature = "bundle"))]
    pub fn unmount_bundle(&mut self, id: BundleId) -> AssetResult<MountedBundle> {
        let _ = id;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    #[cfg(feature = "bundle")]
    pub fn preload_bundle(&mut self, bundle: &MountedBundle) -> AssetLoadGroup {
        let handles = bundle
            .manifest
            .entries
            .iter()
            .filter_map(|entry| {
                self.register_bundle_entry_metadata(entry)?;
                Some(self.load_untyped_by_id(entry.id, entry.asset_type))
            })
            .collect::<Vec<_>>();
        self.register_group(handles)
    }

    #[cfg(feature = "streaming")]
    pub fn register_streaming_region(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        assets: Vec<UntypedHandle>,
    ) -> StreamingRegionId {
        let id = StreamingRegionId(self.next_streaming_region_id);
        self.next_streaming_region_id += 1;
        self.streaming_regions.insert(
            id,
            StreamingRegion {
                id,
                name: name.into(),
                priority,
                assets,
                resident: false,
            },
        );
        id
    }

    #[cfg(feature = "streaming")]
    pub fn register_streaming_region_paths(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        paths: &[AssetPath],
    ) -> AssetResult<StreamingRegionId> {
        require_asset_feature(AssetFeature::Streaming)?;
        let mut handles: Vec<UntypedHandle> = Vec::with_capacity(paths.len());
        for path in paths {
            let extension = path.extension().unwrap_or("");
            let asset_type = self
                .loaders
                .asset_type_for_extension(extension)
                .ok_or_else(|| AssetError::LoaderNotFound {
                    extension: extension.to_owned(),
                })?;
            let id = self.registry.get_or_create(path.clone(), asset_type);
            if handles.iter().any(|handle| handle.id() == id) {
                continue;
            }
            handles.push(self.make_untyped_handle(id, asset_type, HandleStrength::Weak));
        }
        Ok(self.register_streaming_region(name, priority, handles))
    }

    #[cfg(feature = "streaming")]
    pub fn add_asset_to_streaming_region(
        &mut self,
        id: StreamingRegionId,
        path: &AssetPath,
    ) -> AssetResult<bool> {
        let extension = path.extension().unwrap_or("");
        let asset_type = self
            .loaders
            .asset_type_for_extension(extension)
            .ok_or_else(|| AssetError::LoaderNotFound {
                extension: extension.to_owned(),
            })?;
        let asset_id = self.registry.get_or_create(path.clone(), asset_type);
        let handle = self.make_untyped_handle(asset_id, asset_type, HandleStrength::Weak);

        let needs_residency = {
            let region = self
                .streaming_regions
                .get_mut(&id)
                .ok_or_else(|| streaming_region_not_found(id))?;
            if region.assets.iter().any(|handle| handle.id() == asset_id) {
                return Ok(false);
            }
            region.assets.push(handle);
            region.resident
        };
        if needs_residency {
            self.retain_streaming_residency([asset_id]);
        }
        Ok(true)
    }

    #[cfg(not(feature = "streaming"))]
    pub fn register_streaming_region_paths(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        paths: &[AssetPath],
    ) -> AssetResult<StreamingRegionId> {
        let _ = (name.into(), priority, paths);
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(all(feature = "streaming", feature = "bundle"))]
    pub fn register_streaming_region_bundle(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        bundle: BundleId,
    ) -> AssetResult<StreamingRegionId> {
        require_asset_feature(AssetFeature::Streaming)?;
        require_asset_feature(AssetFeature::Bundle)?;
        let entries = self.bundle_entries(bundle)?;
        self.register_streaming_region_bundle_entries(name, priority, entries)
    }

    #[cfg(any(not(feature = "streaming"), not(feature = "bundle")))]
    pub fn register_streaming_region_bundle(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        bundle: BundleId,
    ) -> AssetResult<StreamingRegionId> {
        let _ = (name.into(), priority, bundle);
        require_asset_feature(AssetFeature::Streaming)?;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("streaming or bundle feature is disabled")
    }

    #[cfg(all(feature = "streaming", feature = "bundle"))]
    pub fn register_streaming_region_bundle_subset(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        bundle: BundleId,
        assets: &[AssetId],
    ) -> AssetResult<StreamingRegionId> {
        require_asset_feature(AssetFeature::Streaming)?;
        require_asset_feature(AssetFeature::Bundle)?;
        let entries = self.bundle_entries(bundle)?;
        let mut subset = Vec::with_capacity(assets.len());
        for id in assets {
            if subset.iter().any(|entry: &BundleEntry| entry.id == *id) {
                continue;
            }
            let entry = entries
                .iter()
                .find(|entry| entry.id == *id)
                .cloned()
                .ok_or(AssetError::AssetNotFound { id: *id })?;
            subset.push(entry);
        }
        self.register_streaming_region_bundle_entries(name, priority, subset)
    }

    #[cfg(any(not(feature = "streaming"), not(feature = "bundle")))]
    pub fn register_streaming_region_bundle_subset(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        bundle: BundleId,
        assets: &[AssetId],
    ) -> AssetResult<StreamingRegionId> {
        let _ = (name.into(), priority, bundle, assets);
        require_asset_feature(AssetFeature::Streaming)?;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("streaming or bundle feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn streaming_region(&self, id: StreamingRegionId) -> Option<&StreamingRegion> {
        self.streaming_regions.get(&id)
    }

    #[cfg(feature = "streaming")]
    pub fn remove_streaming_region(
        &mut self,
        id: StreamingRegionId,
    ) -> AssetResult<StreamingRegion> {
        let region = self
            .streaming_regions
            .remove(&id)
            .ok_or_else(|| streaming_region_not_found(id))?;
        if region.resident {
            self.release_streaming_residency(region.assets.iter().map(|handle| handle.id()));
        }
        Ok(region)
    }

    #[cfg(feature = "streaming")]
    pub fn remove_streaming_region_asset(
        &mut self,
        region_id: StreamingRegionId,
        asset_id: AssetId,
    ) -> AssetResult<bool> {
        let mut removed = false;
        let mut should_release = false;
        {
            let region = self
                .streaming_regions
                .get_mut(&region_id)
                .ok_or_else(|| streaming_region_not_found(region_id))?;
            let position = region
                .assets
                .iter()
                .position(|handle| handle.id() == asset_id);
            if let Some(position) = position {
                region.assets.swap_remove(position);
                removed = true;
                should_release = region.resident;
            }
        }
        if removed && should_release {
            self.release_streaming_residency([asset_id]);
        }
        Ok(removed)
    }

    #[cfg(not(feature = "streaming"))]
    pub fn remove_streaming_region(
        &mut self,
        id: StreamingRegionId,
    ) -> AssetResult<StreamingRegion> {
        let _ = id;
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn set_streaming_region_resident(
        &mut self,
        id: StreamingRegionId,
        resident: bool,
    ) -> AssetResult<()> {
        let (was_resident, asset_ids) = {
            let region = self
                .streaming_regions
                .get_mut(&id)
                .ok_or_else(|| streaming_region_not_found(id))?;
            let was_resident = region.resident;
            if was_resident == resident {
                return Ok(());
            }
            region.resident = resident;
            (
                was_resident,
                region
                    .assets
                    .iter()
                    .map(|handle| handle.id())
                    .collect::<Vec<_>>(),
            )
        };
        if resident && !was_resident {
            self.retain_streaming_residency(asset_ids);
        } else if was_resident && !resident {
            self.release_streaming_residency(asset_ids);
        }
        Ok(())
    }

    #[cfg(not(feature = "streaming"))]
    pub fn set_streaming_region_resident(
        &mut self,
        id: StreamingRegionId,
        resident: bool,
    ) -> AssetResult<()> {
        let _ = (id, resident);
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn set_streaming_region_priority(
        &mut self,
        id: StreamingRegionId,
        priority: LoadPriority,
    ) -> AssetResult<LoadPriority> {
        let asset_ids = {
            let region = self
                .streaming_regions
                .get(&id)
                .ok_or_else(|| streaming_region_not_found(id))?;
            region
                .assets
                .iter()
                .map(|handle| handle.id())
                .collect::<Vec<_>>()
        };
        let region = self
            .streaming_regions
            .get_mut(&id)
            .ok_or_else(|| streaming_region_not_found(id))?;
        let previous = region.priority;
        region.priority = priority;

        for asset_id in &asset_ids {
            self.scheduler.set_priority(*asset_id, priority);
            #[cfg(feature = "async_loading")]
            if let Some(request) = self.async_in_flight.get_mut(asset_id) {
                request.priority = priority;
            }
        }

        Ok(previous)
    }

    #[cfg(not(feature = "streaming"))]
    pub fn set_streaming_region_priority(
        &mut self,
        id: StreamingRegionId,
        priority: LoadPriority,
    ) -> AssetResult<LoadPriority> {
        let _ = (id, priority);
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn preload_streaming_region(
        &mut self,
        id: StreamingRegionId,
    ) -> AssetResult<AssetLoadGroup> {
        let (priority, handles) = {
            let region = self
                .streaming_regions
                .get(&id)
                .ok_or_else(|| streaming_region_not_found(id))?;
            (region.priority, region.assets.clone())
        };
        for handle in &handles {
            self.queue_request(LoadRequest {
                id: handle.id(),
                path: self.registry.path_from_id(handle.id()).cloned(),
                asset_type: handle.asset_type(),
                priority,
                recursive_dependencies: true,
                reload: false,
            });
        }
        Ok(self.register_group(handles))
    }

    #[cfg(not(feature = "streaming"))]
    pub fn preload_streaming_region(
        &mut self,
        id: StreamingRegionId,
    ) -> AssetResult<AssetLoadGroup> {
        let _ = id;
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn unload_streaming_region(&mut self, id: StreamingRegionId) -> AssetResult<usize> {
        let (resident, asset_ids) = {
            let region = self
                .streaming_regions
                .get(&id)
                .ok_or_else(|| streaming_region_not_found(id))?;
            (
                region.resident,
                region
                    .assets
                    .iter()
                    .map(|handle| handle.id())
                    .collect::<Vec<_>>(),
            )
        };
        if resident {
            return Ok(0);
        }
        let mut unloaded = 0;
        for asset_id in asset_ids {
            if self.is_asset_resident(asset_id) {
                continue;
            }
            match self.unload_by_id(asset_id) {
                Ok(()) => unloaded += 1,
                Err(AssetError::NotLoaded { .. } | AssetError::AssetNotFound { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(unloaded)
    }

    #[cfg(not(feature = "streaming"))]
    pub fn unload_streaming_region(&mut self, id: StreamingRegionId) -> AssetResult<usize> {
        let _ = id;
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn streaming_region_progress(&self, id: StreamingRegionId) -> AssetResult<LoadProgress> {
        let region = self
            .streaming_regions
            .get(&id)
            .ok_or_else(|| streaming_region_not_found(id))?;
        let group = AssetLoadGroup {
            id: AssetLoadGroupId(0),
            assets: region.assets.clone(),
        };
        Ok(self.group_progress(&group))
    }

    #[cfg(not(feature = "streaming"))]
    pub fn streaming_region_progress(&self, id: StreamingRegionId) -> AssetResult<LoadProgress> {
        let _ = id;
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    #[cfg(feature = "streaming")]
    pub fn streaming_region_state(&self, id: StreamingRegionId) -> AssetResult<AssetLoadState> {
        let region = self
            .streaming_regions
            .get(&id)
            .ok_or_else(|| streaming_region_not_found(id))?;
        let group = AssetLoadGroup {
            id: AssetLoadGroupId(0),
            assets: region.assets.clone(),
        };
        Ok(self.group_state(&group))
    }

    #[cfg(not(feature = "streaming"))]
    pub fn streaming_region_state(&self, id: StreamingRegionId) -> AssetResult<AssetLoadState> {
        let _ = id;
        require_asset_feature(AssetFeature::Streaming)?;
        unreachable!("streaming feature is disabled")
    }

    pub fn get<T: Asset>(&self, handle: &Handle<T>) -> Option<&T> {
        self.storage::<T>()?.get(handle)
    }

    pub fn get_by_id<T: Asset>(&self, id: AssetId) -> Option<&T> {
        self.storage::<T>()?.get_by_id(id)
    }

    pub fn get_mut<T: Asset>(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.storage_mut::<T>()?.get_mut(handle)
    }

    pub fn get_mut_by_id<T: Asset>(&mut self, id: AssetId) -> Option<&mut T> {
        self.storage_mut::<T>()?.get_mut_by_id(id)
    }

    pub fn storage<T: Asset>(&self) -> Option<&Assets<T>> {
        self.storages
            .get(&T::TYPE_ID)?
            .as_any()
            .downcast_ref::<Assets<T>>()
    }

    pub fn storage_mut<T: Asset>(&mut self) -> Option<&mut Assets<T>> {
        self.storages
            .get_mut(&T::TYPE_ID)?
            .as_any_mut()
            .downcast_mut::<Assets<T>>()
    }

    pub fn state<T: Asset>(&self, handle: &Handle<T>) -> AssetLoadState {
        self.storage::<T>()
            .map(|storage| storage.state(handle.id()))
            .unwrap_or(AssetLoadState::Unloaded)
    }

    pub fn state_by_id(&self, id: AssetId) -> AssetLoadState {
        if let Some(metadata) = self.registry.get(id) {
            if let Some(storage) = self.storages.get(&metadata.asset_type) {
                return storage.state(id);
            }
        }
        self.storages
            .values()
            .find_map(|storage| {
                let state = storage.state(id);
                (state != AssetLoadState::Unloaded).then_some(state)
            })
            .or_else(|| self.fallback_states.get(&id).copied())
            .unwrap_or(AssetLoadState::Unloaded)
    }

    pub fn error_by_id(&self, id: AssetId) -> Option<&AssetError> {
        if let Some(metadata) = self.registry.get(id) {
            if let Some(error) = self
                .storages
                .get(&metadata.asset_type)
                .and_then(|storage| storage.error(id))
            {
                return Some(error);
            }
        }
        self.storages
            .values()
            .find_map(|storage| storage.error(id))
            .or_else(|| self.fallback_errors.get(&id))
    }

    pub fn is_ready<T: Asset>(&self, handle: &Handle<T>) -> bool {
        self.state(handle) == AssetLoadState::Ready
    }

    pub fn is_ready_by_id(&self, id: AssetId) -> bool {
        self.state_by_id(id) == AssetLoadState::Ready
    }

    pub fn is_ready_with_dependencies<T: Asset>(&self, handle: &Handle<T>) -> bool {
        self.is_ready(handle)
            && self
                .dependencies
                .transitive_dependencies(handle.id())
                .into_iter()
                .all(|dependency| self.is_ready_by_id(dependency))
    }

    pub fn update(&mut self, frame_index: u64) {
        self.frame_index = frame_index;
        self.update_loading();
        self.update_hot_reload();
        self.update_gc(frame_index);
    }

    pub fn update_loading(&mut self) {
        #[cfg(feature = "async_loading")]
        {
            self.collect_async_load_results();
            self.maintain_async_worker_pool();
            if self.config.enable_async_loading {
                let jobs = self.async_dispatch_jobs_available();
                for _ in 0..jobs {
                    let Some(request) = self.scheduler.pop_next() else {
                        break;
                    };
                    self.dispatch_async_request(request);
                }
                self.resolve_waiting_assets();
                self.refresh_handle_counts();
                return;
            }
        }

        let jobs = self.loading_jobs_per_frame();
        for _ in 0..jobs {
            let Some(request) = self.scheduler.pop_next() else {
                break;
            };
            self.process_request(request);
        }
        self.resolve_waiting_assets();
        self.refresh_handle_counts();
    }

    #[cfg(feature = "hot_reload")]
    pub fn update_hot_reload(&mut self) {
        if crate::features::asset_feature_enabled(AssetFeature::HotReload)
            && !self.hot_reload_watches.is_empty()
        {
            let _ = self.poll_hot_reload_watches();
        }
        let changes = self.hot_reload_queue.drain(..).collect::<Vec<_>>();
        for change in changes {
            let reload_result = if let Some(id) = change.id {
                self.reload_by_id(id)
            } else {
                self.reload_by_path(&change.path)
            };
            if let Ok(()) = reload_result {
                let Some(id) = change
                    .id
                    .or_else(|| self.registry.id_from_path(&change.path))
                else {
                    continue;
                };
                let Ok(plan) = self
                    .hot_reload_dependency_plan_by_id(id, self.config.hot_reload_dependency_policy)
                else {
                    continue;
                };
                for dependent in plan.dependents {
                    let _ = self.reload_by_id(dependent);
                }
            }
        }
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn update_hot_reload(&mut self) {}

    #[cfg(feature = "hot_reload")]
    pub fn hot_reload_dependency_plan_by_id(
        &self,
        id: AssetId,
        policy: HotReloadDependencyPolicy,
    ) -> AssetResult<HotReloadDependencyPlan> {
        require_asset_feature(AssetFeature::HotReload)?;
        let metadata = self
            .registry
            .get(id)
            .cloned()
            .ok_or(AssetError::AssetNotFound { id })?;
        let dependents = match policy {
            HotReloadDependencyPolicy::Direct => self.dependencies.direct_dependents(id),
            HotReloadDependencyPolicy::Transitive => self.dependencies.transitive_dependents(id),
        };
        Ok(HotReloadDependencyPlan {
            changed: id,
            changed_path: metadata.path,
            policy,
            dependents,
        })
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn hot_reload_dependency_plan_by_id(
        &self,
        id: AssetId,
        policy: HotReloadDependencyPolicy,
    ) -> AssetResult<HotReloadDependencyPlan> {
        let _ = (id, policy);
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn hot_reload_dependency_plan_by_path(
        &self,
        path: &AssetPath,
        policy: HotReloadDependencyPolicy,
    ) -> AssetResult<HotReloadDependencyPlan> {
        require_asset_feature(AssetFeature::HotReload)?;
        let id = self
            .registry
            .id_from_path(path)
            .ok_or_else(|| AssetError::PathNotFound { path: path.clone() })?;
        self.hot_reload_dependency_plan_by_id(id, policy)
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn hot_reload_dependency_plan_by_path(
        &self,
        path: &AssetPath,
        policy: HotReloadDependencyPolicy,
    ) -> AssetResult<HotReloadDependencyPlan> {
        let _ = (path, policy);
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    pub fn hot_reload_rollback_policy_for_type(
        &self,
        asset_type: AssetTypeId,
    ) -> HotReloadRollbackPolicyReport {
        let override_retention = self.hot_reload_rollback_overrides.get(&asset_type).copied();
        let retention = override_retention.unwrap_or_else(|| {
            if !self.storages.contains_key(&asset_type) {
                HotReloadRollbackRetention::None
            } else if gpu_rollback_asset_type(asset_type) {
                HotReloadRollbackRetention::CpuAndGpu
            } else {
                HotReloadRollbackRetention::Cpu
            }
        });
        HotReloadRollbackPolicyReport {
            asset_type,
            type_name: self.type_name(asset_type),
            retention,
            requires_previous_ready_state: retention != HotReloadRollbackRetention::None,
            overridden: override_retention.is_some(),
        }
    }

    pub fn hot_reload_rollback_policies(&self) -> Vec<HotReloadRollbackPolicyReport> {
        let mut asset_types = self.type_names.keys().copied().collect::<Vec<_>>();
        for asset_type in self.storages.keys().copied() {
            if !asset_types.contains(&asset_type) {
                asset_types.push(asset_type);
            }
        }
        asset_types.sort();
        asset_types
            .into_iter()
            .map(|asset_type| self.hot_reload_rollback_policy_for_type(asset_type))
            .collect()
    }

    pub fn hot_reload_rollback_report_by_id(
        &self,
        id: AssetId,
    ) -> AssetResult<HotReloadRollbackAssetReport> {
        let metadata = self.registry.get(id);
        let asset_type = metadata
            .map(|metadata| metadata.asset_type)
            .or_else(|| self.asset_type_for_id(id))
            .ok_or(AssetError::AssetNotFound { id })?;
        let current_state = self.state_for_type(id, asset_type);
        let policy = self.hot_reload_rollback_policy_for_type(asset_type);
        Ok(HotReloadRollbackAssetReport {
            id,
            path: metadata.and_then(|metadata| metadata.path.clone()),
            current_state,
            can_rollback_now: current_state == AssetLoadState::Ready
                && policy.can_retain_previous_ready_state(),
            policy,
        })
    }

    pub fn set_hot_reload_rollback_override(
        &mut self,
        asset_type: AssetTypeId,
        retention: HotReloadRollbackRetention,
    ) {
        self.hot_reload_rollback_overrides
            .insert(asset_type, retention);
    }

    pub fn clear_hot_reload_rollback_override(
        &mut self,
        asset_type: AssetTypeId,
    ) -> Option<HotReloadRollbackRetention> {
        self.hot_reload_rollback_overrides.remove(&asset_type)
    }

    #[cfg(feature = "hot_reload")]
    pub fn hot_reload_policy_report(&self) -> HotReloadPolicyReport {
        let mut watches = self
            .hot_reload_watches
            .values()
            .cloned()
            .collect::<Vec<_>>();
        watches.sort_by_key(|watch| watch.path.display_string());
        let watch_backend = if watches
            .iter()
            .any(|watch| matches!(watch.backend, HotReloadWatchBackend::AsyncNotification))
            && !watches
                .iter()
                .any(|watch| matches!(watch.backend, HotReloadWatchBackend::PollingMetadata))
        {
            HotReloadWatchBackend::AsyncNotification
        } else {
            HotReloadWatchBackend::PollingMetadata
        };
        let watch_statuses = watches
            .iter()
            .map(|watch| HotReloadWatchStatus {
                path: watch.path.clone(),
                backend: watch.backend,
                queued: self
                    .hot_reload_queue
                    .iter()
                    .any(|change| change.path == watch.path),
                last_metadata: watch.last_metadata.clone(),
                last_error: self
                    .last_hot_reload_poll
                    .errors
                    .iter()
                    .find(|error| error.path == watch.path)
                    .map(|error| error.error.clone()),
            })
            .collect();
        HotReloadPolicyReport {
            dependency_policy: self.config.hot_reload_dependency_policy,
            rollback_policies: self.hot_reload_rollback_policies(),
            watch_backend,
            async_watch: self.hot_reload_async_watch_report(),
            watches,
            watch_statuses,
            queued_changes: self.hot_reload_queue.iter().cloned().collect(),
            last_poll: self.last_hot_reload_poll.clone(),
        }
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn hot_reload_policy_report(&self) -> HotReloadPolicyReport {
        HotReloadPolicyReport {
            dependency_policy: self.config.hot_reload_dependency_policy,
            rollback_policies: self.hot_reload_rollback_policies(),
            watch_backend: HotReloadWatchBackend::PollingMetadata,
            async_watch: HotReloadAsyncWatchReport::default(),
            watches: Vec::new(),
            watch_statuses: Vec::new(),
            queued_changes: Vec::new(),
            last_poll: self.last_hot_reload_poll.clone(),
        }
    }

    #[cfg(feature = "hot_reload")]
    pub fn watch_hot_reload_path(&mut self, path: impl Into<AssetPath>) -> AssetResult<()> {
        self.watch_hot_reload_path_with_backend(path, HotReloadWatchBackend::PollingMetadata)
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn watch_hot_reload_path(&mut self, path: impl Into<AssetPath>) -> AssetResult<()> {
        let _ = path.into();
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn watch_hot_reload_path_with_backend(
        &mut self,
        path: impl Into<AssetPath>,
        backend: HotReloadWatchBackend,
    ) -> AssetResult<()> {
        require_asset_feature(AssetFeature::HotReload)?;
        let path = path.into();
        let metadata = self.io.metadata(path.path()).map_err(AssetError::from)?;
        self.hot_reload_watches.insert(
            path.clone(),
            HotReloadWatch {
                path,
                backend,
                last_metadata: metadata,
            },
        );
        Ok(())
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn watch_hot_reload_path_with_backend(
        &mut self,
        path: impl Into<AssetPath>,
        backend: HotReloadWatchBackend,
    ) -> AssetResult<()> {
        let _ = path.into();
        let _ = backend;
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn start_hot_reload_async_watch_backend(
        &mut self,
    ) -> AssetResult<HotReloadAsyncWatchReport> {
        require_asset_feature(AssetFeature::HotReload)?;
        self.hot_reload_async_watch_running = true;
        Ok(self.hot_reload_async_watch_report())
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn start_hot_reload_async_watch_backend(
        &mut self,
    ) -> AssetResult<HotReloadAsyncWatchReport> {
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn stop_hot_reload_async_watch_backend(
        &mut self,
    ) -> AssetResult<HotReloadAsyncWatchReport> {
        require_asset_feature(AssetFeature::HotReload)?;
        self.hot_reload_async_watch_running = false;
        self.hot_reload_async_dropped_notifications +=
            self.hot_reload_async_notifications.len() as u64;
        self.hot_reload_async_notifications.clear();
        Ok(self.hot_reload_async_watch_report())
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn stop_hot_reload_async_watch_backend(
        &mut self,
    ) -> AssetResult<HotReloadAsyncWatchReport> {
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn notify_hot_reload_async_watch_change(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> AssetResult<bool> {
        require_asset_feature(AssetFeature::HotReload)?;
        let path = path.into();
        if !self.hot_reload_async_watch_running {
            self.hot_reload_async_dropped_notifications += 1;
            return Ok(false);
        }
        let watched_by_async = self
            .hot_reload_watches
            .get(&path)
            .is_some_and(|watch| matches!(watch.backend, HotReloadWatchBackend::AsyncNotification));
        if !watched_by_async {
            self.hot_reload_async_dropped_notifications += 1;
            return Ok(false);
        }
        self.hot_reload_async_received_notifications += 1;
        self.hot_reload_async_notifications.push_back(path);
        Ok(true)
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn notify_hot_reload_async_watch_change(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> AssetResult<bool> {
        let _ = path.into();
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn hot_reload_async_watch_report(&self) -> HotReloadAsyncWatchReport {
        HotReloadAsyncWatchReport {
            lifecycle: if self.hot_reload_async_watch_running {
                crate::hot_reload::HotReloadAsyncWatchLifecycle::Running
            } else {
                crate::hot_reload::HotReloadAsyncWatchLifecycle::Stopped
            },
            watched_paths: self
                .hot_reload_watches
                .values()
                .filter(|watch| matches!(watch.backend, HotReloadWatchBackend::AsyncNotification))
                .count(),
            pending_notifications: self.hot_reload_async_notifications.len(),
            received_notifications: self.hot_reload_async_received_notifications,
            delivered_notifications: self.hot_reload_async_delivered_notifications,
            dropped_notifications: self.hot_reload_async_dropped_notifications,
            errors: self.last_hot_reload_async_errors.clone(),
        }
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn hot_reload_async_watch_report(&self) -> HotReloadAsyncWatchReport {
        HotReloadAsyncWatchReport::default()
    }

    #[cfg(feature = "hot_reload")]
    pub fn unwatch_hot_reload_path(&mut self, path: &AssetPath) -> bool {
        self.hot_reload_watches.remove(path).is_some()
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn unwatch_hot_reload_path(&mut self, path: &AssetPath) -> bool {
        let _ = path;
        false
    }

    #[cfg(feature = "hot_reload")]
    pub fn hot_reload_watch(&self, path: &AssetPath) -> Option<&HotReloadWatch> {
        self.hot_reload_watches.get(path)
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn hot_reload_watch(&self, path: &AssetPath) -> Option<&HotReloadWatch> {
        let _ = path;
        None
    }

    #[cfg(feature = "hot_reload")]
    pub fn hot_reload_watches(&self) -> impl Iterator<Item = &HotReloadWatch> {
        self.hot_reload_watches.values()
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn hot_reload_watches(&self) -> impl Iterator<Item = &HotReloadWatch> {
        std::iter::empty()
    }

    pub fn last_hot_reload_poll_report(&self) -> &HotReloadPollReport {
        &self.last_hot_reload_poll
    }

    #[cfg(feature = "hot_reload")]
    pub fn poll_hot_reload_watches(&mut self) -> AssetResult<HotReloadPollReport> {
        require_asset_feature(AssetFeature::HotReload)?;
        let paths = self
            .hot_reload_watches
            .iter()
            .filter_map(|(path, watch)| {
                matches!(watch.backend, HotReloadWatchBackend::PollingMetadata)
                    .then(|| path.clone())
            })
            .collect::<Vec<_>>();
        let mut report = HotReloadPollReport {
            watched_paths: self.hot_reload_watches.len(),
            ..Default::default()
        };
        let mut batch_paths = HashSet::new();

        for path in paths {
            let metadata = match self.io.metadata(path.path()) {
                Ok(metadata) => metadata,
                Err(error) => {
                    report.errors.push(HotReloadWatchError { path, error });
                    continue;
                }
            };
            let changed = {
                let Some(watch) = self.hot_reload_watches.get_mut(&path) else {
                    continue;
                };
                if watch.last_metadata == metadata {
                    report.unchanged_paths += 1;
                    false
                } else {
                    watch.last_metadata = metadata;
                    true
                }
            };
            if !changed {
                continue;
            }
            let change = HotReloadChange {
                id: self.registry.id_from_path(&path),
                path: path.clone(),
            };
            if self.queue_hot_reload_change_debounced(change.clone(), &mut batch_paths) {
                report.changed.push(change);
            } else {
                report.debounced_changes += 1;
            }
        }

        self.last_hot_reload_async_errors.clear();
        while let Some(path) = self.hot_reload_async_notifications.pop_front() {
            report.async_notifications += 1;
            let Some(watch) = self.hot_reload_watches.get_mut(&path) else {
                report.dropped_notifications += 1;
                self.hot_reload_async_dropped_notifications += 1;
                continue;
            };
            if !matches!(watch.backend, HotReloadWatchBackend::AsyncNotification) {
                report.dropped_notifications += 1;
                self.hot_reload_async_dropped_notifications += 1;
                continue;
            }
            match self.io.metadata(path.path()) {
                Ok(metadata) => {
                    watch.last_metadata = metadata;
                    let change = HotReloadChange {
                        id: self.registry.id_from_path(&path),
                        path: path.clone(),
                    };
                    if self.queue_hot_reload_change_debounced(change.clone(), &mut batch_paths) {
                        report.changed.push(change);
                        self.hot_reload_async_delivered_notifications += 1;
                    } else {
                        report.debounced_changes += 1;
                    }
                }
                Err(error) => {
                    let watch_error = HotReloadWatchError { path, error };
                    report.errors.push(watch_error.clone());
                    self.last_hot_reload_async_errors.push(watch_error);
                }
            }
        }

        self.last_hot_reload_poll = report.clone();
        Ok(report)
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn poll_hot_reload_watches(&mut self) -> AssetResult<HotReloadPollReport> {
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    pub fn queue_hot_reload_path(&mut self, path: impl Into<AssetPath>) {
        let _ = self.try_queue_hot_reload_path(path);
    }

    #[cfg(feature = "hot_reload")]
    pub fn try_queue_hot_reload_path(&mut self, path: impl Into<AssetPath>) -> AssetResult<()> {
        require_asset_feature(AssetFeature::HotReload)?;
        self.hot_reload_queue.push_back(HotReloadChange {
            id: None,
            path: path.into(),
        });
        Ok(())
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn try_queue_hot_reload_path(&mut self, path: impl Into<AssetPath>) -> AssetResult<()> {
        let _ = path.into();
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn queue_hot_reload_id(&mut self, id: AssetId) -> AssetResult<()> {
        require_asset_feature(AssetFeature::HotReload)?;
        let path = self
            .registry
            .path_from_id(id)
            .cloned()
            .ok_or(AssetError::AssetNotFound { id })?;
        self.hot_reload_queue
            .push_back(HotReloadChange { id: Some(id), path });
        Ok(())
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn queue_hot_reload_id(&mut self, id: AssetId) -> AssetResult<()> {
        let _ = id;
        require_asset_feature(AssetFeature::HotReload)?;
        unreachable!("hot_reload feature is disabled")
    }

    #[cfg(feature = "hot_reload")]
    pub fn queue_hot_reload_change(&mut self, change: HotReloadChange) {
        if crate::features::asset_feature_enabled(AssetFeature::HotReload) {
            self.hot_reload_queue.push_back(change);
        }
    }

    #[cfg(not(feature = "hot_reload"))]
    pub fn queue_hot_reload_change(&mut self, change: HotReloadChange) {
        let _ = change;
    }

    #[cfg(feature = "hot_reload")]
    fn queue_hot_reload_change_debounced(
        &mut self,
        change: HotReloadChange,
        batch_paths: &mut HashSet<AssetPath>,
    ) -> bool {
        if !batch_paths.insert(change.path.clone()) {
            return false;
        }
        if self
            .hot_reload_queue
            .iter()
            .any(|queued| queued.path == change.path)
        {
            return false;
        }
        self.hot_reload_queue.push_back(change);
        true
    }

    pub fn update_gc(&mut self, frame_index: u64) {
        self.frame_index = frame_index;
        if !self.config.gc.enabled {
            return;
        }
        self.refresh_handle_counts();
        let mut unload = Vec::new();
        for storage in self.storages.values() {
            for id in storage.ids() {
                let state = storage.state(id);
                if !matches!(state, AssetLoadState::Ready | AssetLoadState::Failed) {
                    continue;
                }
                let strong = self.live_handle_counts(id).0;
                let dependency = self.dependency_ref_counts.get(&id).copied().unwrap_or(0);
                let unused_frames = frame_index.saturating_sub(storage.last_used_frame(id));
                if strong == 0
                    && dependency == 0
                    && !storage.is_resident(id)
                    && unused_frames >= self.config.gc.unload_after_unused_frames
                {
                    unload.push(id);
                }
            }
        }
        for id in unload {
            let _ = self.unload_by_id(id);
        }
        self.evict_to_memory_budget();
        self.evict_to_type_memory_budgets();
    }

    fn evict_to_memory_budget(&mut self) {
        let Some(budget) = self.config.gc.memory_budget_bytes else {
            return;
        };
        let mut total = self.memory_stats().cpu_bytes + self.memory_stats().gpu_bytes;
        if total <= budget {
            return;
        }
        let mut candidates = Vec::new();
        for storage in self.storages.values() {
            for id in storage.ids() {
                let state = storage.state(id);
                if !matches!(state, AssetLoadState::Ready | AssetLoadState::Failed) {
                    continue;
                }
                let strong = self.live_handle_counts(id).0;
                let dependency = self.dependency_ref_counts.get(&id).copied().unwrap_or(0);
                if strong != 0 || dependency != 0 || storage.is_resident(id) {
                    continue;
                }
                let (cpu, gpu) = storage.cpu_gpu_bytes(id);
                let bytes = cpu.saturating_add(gpu);
                if bytes == 0 {
                    continue;
                }
                candidates.push((storage.last_used_frame(id), id, bytes));
            }
        }
        candidates.sort_by_key(|(last_used, _, _)| *last_used);
        for (_, id, bytes) in candidates {
            if total <= budget {
                break;
            }
            if self.unload_by_id(id).is_ok() {
                total = total.saturating_sub(bytes);
            }
        }
    }

    fn evict_to_type_memory_budgets(&mut self) {
        let budgets = self.config.gc.type_memory_budgets.clone();
        for budget in budgets
            .into_iter()
            .filter(AssetTypeMemoryBudget::has_budget)
        {
            self.evict_asset_type_to_budget(budget);
        }
    }

    fn evict_asset_type_to_budget(&mut self, budget: AssetTypeMemoryBudget) {
        let Some(storage) = self.storages.get(&budget.asset_type) else {
            return;
        };
        let mut cpu_total: u64 = 0;
        let mut gpu_total: u64 = 0;
        let mut candidates = Vec::new();
        for id in storage.ids() {
            let state = storage.state(id);
            if !matches!(state, AssetLoadState::Ready | AssetLoadState::Failed) {
                continue;
            }
            let (cpu, gpu) = storage.cpu_gpu_bytes(id);
            cpu_total = cpu_total.saturating_add(cpu);
            gpu_total = gpu_total.saturating_add(gpu);
            let strong = self.live_handle_counts(id).0;
            let dependency = self.dependency_ref_counts.get(&id).copied().unwrap_or(0);
            if strong != 0 || dependency != 0 || storage.is_resident(id) {
                continue;
            }
            if cpu.saturating_add(gpu) == 0 {
                continue;
            }
            candidates.push((storage.last_used_frame(id), id, cpu, gpu));
        }
        if type_memory_budget_satisfied(cpu_total, gpu_total, &budget) {
            return;
        }
        candidates.sort_by_key(|(last_used, _, _, _)| *last_used);
        for (_, id, cpu, gpu) in candidates {
            if type_memory_budget_satisfied(cpu_total, gpu_total, &budget) {
                break;
            }
            if self.unload_by_id(id).is_ok() {
                cpu_total = cpu_total.saturating_sub(cpu);
                gpu_total = gpu_total.saturating_sub(gpu);
            }
        }
    }

    pub fn reload<T: Asset>(&mut self, handle: &Handle<T>) -> AssetResult<()> {
        self.reload_by_id(handle.id())
    }

    pub fn reload_by_id(&mut self, id: AssetId) -> AssetResult<()> {
        let metadata = self
            .registry
            .get(id)
            .cloned()
            .ok_or(AssetError::AssetNotFound { id })?;
        let previous_state = self.state_for_type(id, metadata.asset_type);
        if previous_state == AssetLoadState::Ready {
            self.reload_rollbacks.insert(
                id,
                ReloadRollback {
                    asset_type: metadata.asset_type,
                    previous_state,
                },
            );
        }
        self.set_state_for_type(id, metadata.asset_type, AssetLoadState::Reloading);
        self.events.push(AssetEvent::ReloadStarted { id });
        self.queue_request(LoadRequest {
            id,
            path: metadata.path,
            asset_type: metadata.asset_type,
            priority: LoadPriority::Immediate,
            recursive_dependencies: true,
            reload: true,
        });
        Ok(())
    }

    pub fn reload_by_path(&mut self, path: &AssetPath) -> AssetResult<()> {
        let id = self
            .registry
            .id_from_path(path)
            .ok_or_else(|| AssetError::PathNotFound { path: path.clone() })?;
        self.reload_by_id(id)
    }

    pub fn unload<T: Asset>(&mut self, handle: Handle<T>) {
        let _ = self.unload_by_id(handle.id());
    }

    pub fn unload_by_id(&mut self, id: AssetId) -> AssetResult<()> {
        let asset_type = self
            .registry
            .get(id)
            .map(|metadata| metadata.asset_type)
            .or_else(|| self.asset_type_for_id(id))
            .ok_or(AssetError::AssetNotFound { id })?;
        let Some(storage) = self.storages.get_mut(&asset_type) else {
            return Err(AssetError::AssetNotFound { id });
        };
        if !storage.ids().contains(&id) {
            return Err(AssetError::NotLoaded { id });
        }
        storage.set_state(id, AssetLoadState::Unloading);
        if storage.remove(id) {
            self.dependencies.set_dependencies(id, Vec::new());
            if let Some(metadata) = self.registry.get_mut(id) {
                metadata.dependencies.clear();
            }
            self.refresh_dependency_counts();
            self.events.push(AssetEvent::Unloaded { id });
            Ok(())
        } else {
            Err(AssetError::NotLoaded { id })
        }
    }

    pub fn unload_unused(&mut self) {
        self.update_gc(self.frame_index);
    }

    pub fn collect_unused(&mut self) {
        self.unload_unused();
    }

    pub fn collect_until_budget(&mut self) {
        self.refresh_handle_counts();
        self.evict_to_memory_budget();
        self.evict_to_type_memory_budgets();
    }

    pub fn set_asset_resident(&mut self, id: AssetId, resident: bool) {
        if let Some(asset_type) = self.asset_type_for_id(id) {
            if let Some(storage) = self.storages.get_mut(&asset_type) {
                storage.set_resident(id, resident);
            }
        }
    }

    pub fn is_asset_resident(&self, id: AssetId) -> bool {
        self.asset_type_for_id(id)
            .and_then(|asset_type| self.storages.get(&asset_type))
            .map(|storage| storage.is_resident(id))
            .unwrap_or(false)
    }

    pub fn metadata(&self, id: AssetId) -> Option<&AssetMetadata> {
        self.registry.get(id)
    }

    pub fn metadata_by_path(&self, path: &AssetPath) -> Option<&AssetMetadata> {
        self.registry.metadata_by_path(path)
    }

    pub fn id_from_path(&self, path: &AssetPath) -> Option<AssetId> {
        self.registry.id_from_path(path)
    }

    pub fn path_from_id(&self, id: AssetId) -> Option<&AssetPath> {
        self.registry.path_from_id(id)
    }

    pub fn events(&self) -> &[AssetEvent] {
        &self.events
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = AssetEvent> + '_ {
        self.events.drain(..)
    }

    pub fn events_since(&self, cursor: &mut AssetEventCursor) -> &[AssetEvent] {
        let start = cursor.index.min(self.events.len());
        cursor.index = self.events.len();
        &self.events[start..]
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn dependency_graph(&self) -> &DependencyGraph {
        &self.dependencies
    }

    pub fn dependency_report(&self) -> DependencyGraphReport {
        self.dependencies.report()
    }

    pub fn scoped_dependency_report(&self, root: AssetId) -> AssetResult<DependencyScopeReport> {
        self.dependencies.scoped_report(root)
    }

    pub fn dependency_report_text(&self) -> String {
        self.dependency_report().to_text()
    }

    pub fn dependency_report_dot(&self) -> String {
        self.dependency_report().to_dot()
    }

    pub fn dependency_report_json(&self) -> String {
        self.dependency_report().to_json()
    }

    pub fn dependency_report_html(&self) -> String {
        let report = self.dependency_report();
        let labels = self.dependency_report_labels(&report);
        report.to_html_with_labels(labels)
    }

    pub fn scoped_dependency_report_text(&self, root: AssetId) -> AssetResult<String> {
        Ok(self.scoped_dependency_report(root)?.to_text())
    }

    pub fn scoped_dependency_report_dot(&self, root: AssetId) -> AssetResult<String> {
        Ok(self.scoped_dependency_report(root)?.to_dot())
    }

    pub fn scoped_dependency_report_json(&self, root: AssetId) -> AssetResult<String> {
        Ok(self.scoped_dependency_report(root)?.to_json())
    }

    pub fn scoped_dependency_report_html(&self, root: AssetId) -> AssetResult<String> {
        let report = self.scoped_dependency_report(root)?;
        let labels = self.dependency_report_labels(&report.graph);
        Ok(report.to_html_with_labels(labels))
    }

    pub fn save_dependency_report_text(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        self.dependency_report().save_text(path)
    }

    pub fn save_dependency_report_dot(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()> {
        self.dependency_report().save_dot(path)
    }

    pub fn save_dependency_report_json(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        self.dependency_report().save_json(path)
    }

    pub fn save_dependency_report_html(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        let report = self.dependency_report();
        let labels = self.dependency_report_labels(&report);
        let path = path.as_ref();
        std::fs::write(path, report.to_html_with_labels(labels)).map_err(|error| AssetError::Io {
            message: format!(
                "failed to write dependency report `{}`: {error}",
                path.display()
            ),
        })
    }

    pub fn save_scoped_dependency_report_text(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        self.scoped_dependency_report(root)?.save_text(path)
    }

    pub fn save_scoped_dependency_report_dot(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        self.scoped_dependency_report(root)?.save_dot(path)
    }

    pub fn save_scoped_dependency_report_json(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        self.scoped_dependency_report(root)?.save_json(path)
    }

    pub fn save_scoped_dependency_report_html(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()> {
        let report = self.scoped_dependency_report(root)?;
        let labels = self.dependency_report_labels(&report.graph);
        let path = path.as_ref();
        std::fs::write(path, report.to_html_with_labels(labels)).map_err(|error| AssetError::Io {
            message: format!(
                "failed to write dependency report `{}`: {error}",
                path.display()
            ),
        })
    }

    fn dependency_report_labels(&self, report: &DependencyGraphReport) -> Vec<(AssetId, String)> {
        report
            .assets
            .iter()
            .filter_map(|asset| {
                self.metadata(*asset)
                    .map(|metadata| (*asset, dependency_report_metadata_label(metadata)))
            })
            .collect()
    }

    pub fn drain_gpu_uploads(&mut self) -> impl Iterator<Item = GpuUploadCommand> + '_ {
        let max = self.config.max_gpu_uploads_per_frame.max(1);
        let mut uploads = Vec::new();
        for _ in 0..max {
            let Some(upload) = self.gpu_uploads.pop_front() else {
                break;
            };
            uploads.push(upload);
        }
        uploads.into_iter()
    }

    pub fn finish_gpu_uploads(&mut self, results: impl IntoIterator<Item = GpuUploadResult>) {
        for result in results {
            let Some(pending) = self.pending_gpu.remove(&result.id) else {
                continue;
            };
            match result.result {
                Ok(gpu) => {
                    if let Some(storage) = self.storages.get_mut(&pending.asset_type) {
                        if let Err(error) =
                            storage.insert_boxed(result.id, pending.asset, AssetLoadState::Ready)
                        {
                            self.fail_asset(result.id, pending.asset_type, error);
                            continue;
                        }
                        let _ = storage.apply_gpu_upload(result.id, gpu);
                    }
                    self.reload_rollbacks.remove(&result.id);
                    if pending.reloaded {
                        self.events.push(AssetEvent::Reloaded { id: result.id });
                    }
                    self.events
                        .push(AssetEvent::GpuUploadFinished { id: result.id });
                    self.events.push(AssetEvent::Ready { id: result.id });
                }
                Err(message) => self.fail_asset(
                    result.id,
                    pending.asset_type,
                    AssetError::GpuUpload { message },
                ),
            }
        }
        self.resolve_waiting_assets();
    }

    pub fn memory_stats(&self) -> AssetMemoryStats {
        let report = self.memory_report();
        AssetMemoryStats {
            assets: report.asset_count,
            cpu_bytes: report.total_cpu_bytes,
            gpu_bytes: report.total_gpu_bytes,
        }
    }

    pub fn memory_info(&self, id: AssetId) -> Option<AssetMemoryInfo> {
        let asset_type = self.asset_type_for_id(id)?;
        let storage = self.storages.get(&asset_type)?;
        storage
            .ids()
            .contains(&id)
            .then(|| self.memory_info_for_storage(id, asset_type, storage.as_ref()))
    }

    pub fn memory_report(&self) -> AssetMemoryReport {
        let mut assets = Vec::new();
        let mut by_type = BTreeMap::<AssetTypeId, AssetTypeMemoryReport>::new();
        let mut total_cpu_bytes = 0;
        let mut total_gpu_bytes = 0;
        for (asset_type, storage) in &self.storages {
            for id in storage.ids() {
                let info = self.memory_info_for_storage(id, *asset_type, storage.as_ref());
                total_cpu_bytes += info.cpu_bytes;
                total_gpu_bytes += info.gpu_bytes;
                let type_report =
                    by_type
                        .entry(*asset_type)
                        .or_insert_with(|| AssetTypeMemoryReport {
                            asset_type: *asset_type,
                            ..AssetTypeMemoryReport::default()
                        });
                type_report.asset_count += 1;
                type_report.cpu_bytes += info.cpu_bytes;
                type_report.gpu_bytes += info.gpu_bytes;
                type_report.strong_count += info.strong_count;
                type_report.weak_count += info.weak_count;
                type_report.dependency_ref_count += info.dependency_ref_count;
                if info.resident {
                    type_report.resident_assets += 1;
                }
                assets.push(info);
            }
        }
        assets.sort_by_key(|info| (info.asset_type, info.id));
        AssetMemoryReport {
            total_cpu_bytes,
            total_gpu_bytes,
            asset_count: assets.len(),
            assets,
            by_type: by_type.into_values().collect(),
        }
    }

    fn memory_info_for_storage(
        &self,
        id: AssetId,
        asset_type: AssetTypeId,
        storage: &dyn ErasedAssets,
    ) -> AssetMemoryInfo {
        let (cpu_bytes, gpu_bytes) = storage.cpu_gpu_bytes(id);
        let (strong_count, weak_count) = self.live_handle_counts(id);
        AssetMemoryInfo {
            id,
            asset_type,
            state: storage.state(id),
            cpu_bytes,
            gpu_bytes,
            last_used_frame: storage.last_used_frame(id),
            strong_count,
            weak_count,
            dependency_ref_count: self.dependency_ref_counts.get(&id).copied().unwrap_or(0),
            resident: storage.is_resident(id),
        }
    }

    fn process_request(&mut self, request: LoadRequest) {
        let Some(path) = request
            .path
            .clone()
            .or_else(|| self.registry.path_from_id(request.id).cloned())
        else {
            self.fail_asset(
                request.id,
                request.asset_type,
                AssetError::AssetNotFound { id: request.id },
            );
            return;
        };
        self.set_state_for_type(request.id, request.asset_type, AssetLoadState::LoadingBytes);
        self.events.push(AssetEvent::LoadStarted { id: request.id });
        let bytes = match self.io.read(path.path()) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.fail_asset(request.id, request.asset_type, error.into());
                return;
            }
        };
        self.set_state_for_type(request.id, request.asset_type, AssetLoadState::DecodingCpu);
        let loader = match self
            .loaders
            .loader_for_path_and_type(Some(&path), request.asset_type)
        {
            Ok(loader) => loader,
            Err(error) => {
                self.fail_asset(request.id, request.asset_type, error);
                return;
            }
        };
        let mut context =
            crate::loader::LoadContext::new(request.id, path.clone(), &mut self.registry);
        let loaded = match loader.load(&mut context, &bytes, &LoaderSettings::default()) {
            Ok(loaded) => loaded,
            Err(error) => {
                self.fail_asset(request.id, request.asset_type, error);
                return;
            }
        };
        let (dependencies, _subresources) = context.finish();
        self.store_loaded_asset(request, path, loaded, dependencies);
    }

    #[cfg(feature = "async_loading")]
    fn dispatch_async_request(&mut self, request: LoadRequest) {
        let Some(path) = request
            .path
            .clone()
            .or_else(|| self.registry.path_from_id(request.id).cloned())
        else {
            self.fail_asset(
                request.id,
                request.asset_type,
                AssetError::AssetNotFound { id: request.id },
            );
            return;
        };

        self.set_state_for_type(request.id, request.asset_type, AssetLoadState::LoadingBytes);
        self.events.push(AssetEvent::LoadStarted { id: request.id });
        let request_id = request.id;
        let request_asset_type = request.asset_type;
        self.async_in_flight.insert(request.id, request.clone());

        let io = Arc::clone(&self.io);
        let loaders = self.loaders.clone();
        let registry = self.registry.clone();
        let job = AsyncLoadJob {
            request,
            path,
            io,
            loaders,
            registry,
        };
        self.ensure_async_worker_pool();
        if let Some(pool) = self.async_worker_pool.as_ref() {
            if pool.dispatch(job).is_ok() {
                self.async_jobs_dispatched += 1;
                return;
            }
        }
        self.async_in_flight.remove(&request_id);
        self.fail_asset(
            request_id,
            request_asset_type,
            AssetError::Io {
                message: "async worker pool is unavailable".to_owned(),
            },
        );
    }

    #[cfg(feature = "async_loading")]
    fn collect_async_load_results(&mut self) {
        loop {
            let result = match self
                .async_worker_pool
                .as_ref()
                .map(AsyncWorkerPool::try_recv)
            {
                Some(Ok(result)) => result,
                _ => break,
            };
            self.async_jobs_completed += 1;
            let request = result.request.clone();
            self.async_in_flight.remove(&request.id);
            if self.cancelled_async_loads.remove(&request.id) {
                continue;
            }

            match result.outcome {
                AsyncLoadOutcome::Loaded {
                    path,
                    loaded,
                    dependencies,
                    subresources,
                } => {
                    self.merge_async_registry_entries(&dependencies, &subresources);
                    self.set_state_for_type(
                        request.id,
                        request.asset_type,
                        AssetLoadState::DecodingCpu,
                    );
                    self.store_loaded_asset(request, path, loaded, dependencies);
                }
                AsyncLoadOutcome::Failed { state, error } => {
                    self.set_state_for_type(request.id, request.asset_type, state);
                    self.fail_asset(request.id, request.asset_type, error);
                }
            }
        }
    }

    #[cfg(feature = "async_loading")]
    fn ensure_async_worker_pool(&mut self) {
        let desired_workers = self.async_worker_limit();
        if desired_workers == 0 {
            return;
        }
        if self
            .async_worker_pool
            .as_ref()
            .is_some_and(|pool| pool.worker_count() == desired_workers)
        {
            return;
        }
        if self.async_worker_pool.is_some() && !self.async_in_flight.is_empty() {
            return;
        }
        self.shutdown_async_worker_pool_internal();
        self.async_worker_pool = Some(AsyncWorkerPool::new(desired_workers));
        self.async_worker_threads_started += desired_workers as u64;
    }

    #[cfg(feature = "async_loading")]
    fn maintain_async_worker_pool(&mut self) {
        if self.config.enable_async_loading {
            self.ensure_async_worker_pool();
        } else if self.async_in_flight.is_empty() {
            self.shutdown_async_worker_pool_internal();
        }
    }

    #[cfg(feature = "async_loading")]
    fn shutdown_async_worker_pool_internal(&mut self) {
        if self.async_worker_pool.take().is_some() {
            self.async_worker_pool_shutdowns += 1;
        }
    }

    #[cfg(feature = "async_loading")]
    fn merge_async_registry_entries(
        &mut self,
        dependencies: &[crate::loader::LoadDependency],
        subresources: &[(AssetPath, AssetTypeId, AssetId)],
    ) {
        for dependency in dependencies {
            self.registry.insert(AssetMetadata::runtime(
                dependency.id,
                dependency.path.clone(),
                dependency.asset_type,
            ));
        }
        for (path, asset_type, id) in subresources {
            self.registry
                .insert(AssetMetadata::runtime(*id, path.clone(), *asset_type));
        }
    }

    #[cfg(feature = "async_loading")]
    fn cancel_async_request(&mut self, id: AssetId) -> bool {
        let Some(request) = self.async_in_flight.get(&id).cloned() else {
            return false;
        };
        self.cancelled_async_loads.insert(id);
        self.cancel_queued_request(request);
        true
    }

    #[cfg(feature = "async_loading")]
    fn async_dispatch_jobs_available(&self) -> usize {
        let worker_limit = self.async_worker_limit();
        worker_limit
            .saturating_sub(self.async_in_flight.len())
            .min(self.loading_jobs_per_frame())
    }

    #[cfg(feature = "async_loading")]
    fn async_worker_limit(&self) -> usize {
        if !self.config.enable_async_loading {
            return 0;
        }
        if cfg!(feature = "parallel") && self.config.worker_threads > 1 {
            self.config.worker_threads
        } else {
            1
        }
    }

    fn store_loaded_asset(
        &mut self,
        request: LoadRequest,
        path: AssetPath,
        loaded: LoadedAsset,
        dependencies: Vec<crate::loader::LoadDependency>,
    ) {
        let LoadedAsset {
            asset_type,
            asset,
            gpu_upload,
            asset_dependencies,
        } = loaded;
        if request.asset_type != AssetTypeId::NIL && request.asset_type != asset_type {
            self.fail_asset(
                request.id,
                request.asset_type,
                AssetError::TypeMismatch {
                    expected: self.type_name(request.asset_type),
                    actual: self.type_name(asset_type),
                },
            );
            return;
        }
        let metadata_dependency_ids = self
            .registry
            .get(request.id)
            .map(|metadata| metadata.dependencies.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|dependency| self.metadata_dependency_has_source_or_state(*dependency))
            .collect::<Vec<_>>();
        let mut dependency_ids = Vec::new();
        for dependency in &metadata_dependency_ids {
            if !dependency_ids.contains(dependency) {
                dependency_ids.push(*dependency);
            }
        }
        for dependency in &dependencies {
            if !dependency_ids.contains(&dependency.id) {
                dependency_ids.push(dependency.id);
            }
        }
        for dependency in &asset_dependencies {
            self.register_asset_dependency_fallback(dependency);
            if !dependency_ids.contains(&dependency.id()) {
                dependency_ids.push(dependency.id());
            }
        }
        let missing_asset_dependencies = asset_dependencies
            .iter()
            .filter(|dependency| !self.asset_dependency_has_source_or_state(dependency))
            .map(|dependency| (dependency.id(), dependency.asset_type()))
            .collect::<Vec<_>>();
        self.ensure_builtin_storage_for_type(asset_type);
        let Some(storage) = self.storages.get_mut(&asset_type) else {
            self.fail_asset(
                request.id,
                asset_type,
                AssetError::LoaderForTypeNotFound { asset_type },
            );
            return;
        };
        let state = if dependency_ids.is_empty() {
            AssetLoadState::LoadedCpu
        } else {
            AssetLoadState::WaitingForDependencies
        };
        storage.set_state(request.id, state);
        if let Some(metadata) = self.registry.get_mut(request.id) {
            metadata.asset_type = asset_type;
            metadata.path = Some(path);
            metadata.dependencies = dependency_ids.clone();
        }
        if let Some(metadata) = self.registry.get(request.id).cloned() {
            storage.set_metadata(request.id, metadata);
        }
        self.dependencies
            .set_dependencies(request.id, dependency_ids);
        if self.dependencies.has_cycle_from(request.id) {
            self.fail_asset(request.id, asset_type, AssetError::CyclicDependency);
            return;
        }
        self.refresh_dependency_counts();
        self.events.push(AssetEvent::LoadedCpu { id: request.id });
        for (dependency, dependency_type) in missing_asset_dependencies {
            self.fail_asset(
                dependency,
                dependency_type,
                AssetError::AssetNotFound { id: dependency },
            );
        }
        for dependency in metadata_dependency_ids {
            if !request.recursive_dependencies || self.is_ready_by_id(dependency) {
                continue;
            }
            let Some(metadata) = self.registry.get(dependency).cloned() else {
                continue;
            };
            let Some(path) = metadata.path.clone() else {
                continue;
            };
            self.queue_request(LoadRequest {
                id: dependency,
                path: Some(path),
                asset_type: metadata.asset_type,
                priority: request.priority,
                recursive_dependencies: true,
                reload: false,
            });
        }
        for dependency in dependencies {
            if request.recursive_dependencies {
                self.queue_request(LoadRequest {
                    id: dependency.id,
                    path: Some(dependency.path),
                    asset_type: dependency.asset_type,
                    priority: request.priority,
                    recursive_dependencies: true,
                    reload: false,
                });
            }
        }
        for dependency in asset_dependencies {
            if !request.recursive_dependencies {
                continue;
            }
            let Some(metadata) = self.registry.get(dependency.id()).cloned() else {
                continue;
            };
            let Some(path) = metadata
                .path
                .clone()
                .or_else(|| dependency.fallback_path.clone())
            else {
                continue;
            };
            self.queue_request(LoadRequest {
                id: dependency.id(),
                path: Some(path),
                asset_type: if metadata.asset_type == AssetTypeId::NIL {
                    dependency.asset_type()
                } else {
                    metadata.asset_type
                },
                priority: request.priority,
                recursive_dependencies: true,
                reload: false,
            });
        }
        if self.dependencies_ready(request.id) {
            self.finalize_cpu_loaded(request.id, asset_type, asset, gpu_upload, request.reload);
        } else {
            self.waiting_assets.insert(
                request.id,
                WaitingAsset {
                    asset_type,
                    asset: Some(asset),
                    gpu_upload,
                    reloaded: request.reload,
                },
            );
        }
    }

    fn asset_dependency_has_source_or_state(&self, dependency: &AssetDependencyReference) -> bool {
        if !matches!(
            self.state_by_id(dependency.id()),
            AssetLoadState::Unloaded | AssetLoadState::Cancelled
        ) {
            return true;
        }
        self.registry
            .get(dependency.id())
            .and_then(|metadata| metadata.path.as_ref())
            .is_some()
            || dependency.fallback_path().is_some()
    }

    fn metadata_dependency_has_source_or_state(&self, dependency: AssetId) -> bool {
        if !matches!(
            self.state_by_id(dependency),
            AssetLoadState::Unloaded | AssetLoadState::Cancelled
        ) {
            return true;
        }
        self.registry
            .get(dependency)
            .and_then(|metadata| metadata.path.as_ref())
            .is_some()
    }

    fn register_asset_dependency_fallback(&mut self, dependency: &AssetDependencyReference) {
        let Some(path) = dependency.fallback_path().cloned() else {
            return;
        };
        if let Some(metadata) = self.registry.get_mut(dependency.id()) {
            if metadata.asset_type == AssetTypeId::NIL {
                metadata.asset_type = dependency.asset_type();
            }
            if metadata.path.is_none() {
                metadata.path = Some(path);
            }
            return;
        }
        self.registry.insert(AssetMetadata::runtime(
            dependency.id(),
            path,
            dependency.asset_type(),
        ));
    }

    fn insert_loaded_inner<T: Asset>(
        &mut self,
        id: AssetId,
        metadata: Option<AssetMetadata>,
        asset: T,
    ) -> AssetResult<Handle<T>> {
        if self.is_live_for_insert(id) {
            return Err(AssetError::AlreadyLoaded { id });
        }
        if let Some(metadata) = &metadata {
            self.registry.insert(metadata.clone());
        }
        self.waiting_assets.remove(&id);
        self.pending_gpu.remove(&id);
        self.reload_rollbacks.remove(&id);
        let dependencies = metadata
            .as_ref()
            .map(|metadata| metadata.dependencies.clone())
            .unwrap_or_default();
        let storage =
            self.storages
                .get_mut(&T::TYPE_ID)
                .ok_or(AssetError::LoaderForTypeNotFound {
                    asset_type: T::TYPE_ID,
                })?;
        storage.insert_boxed(id, Box::new(asset), AssetLoadState::Ready)?;
        if let Some(metadata) = self.registry.get(id).cloned() {
            storage.set_metadata(id, metadata);
        }
        self.dependencies.set_dependencies(id, dependencies);
        self.refresh_dependency_counts();
        self.events.push(AssetEvent::LoadedCpu { id });
        self.events.push(AssetEvent::Ready { id });
        Ok(self.make_handle::<T>(id, HandleStrength::Strong))
    }

    fn is_live_for_insert(&self, id: AssetId) -> bool {
        if self
            .storages
            .values()
            .any(|storage| is_live_insert_state(storage.state(id)))
        {
            return true;
        }
        self.waiting_assets.contains_key(&id) || self.pending_gpu.contains_key(&id)
    }

    fn finalize_cpu_loaded(
        &mut self,
        id: AssetId,
        asset_type: AssetTypeId,
        asset: Box<dyn Any + Send + Sync>,
        upload: Option<GpuUploadCommand>,
        reloaded: bool,
    ) {
        if let Some(upload) = upload {
            self.set_state_for_type(id, asset_type, AssetLoadState::UploadingGpu);
            self.pending_gpu.insert(
                id,
                PendingGpuUpload {
                    asset_type,
                    asset,
                    reloaded,
                },
            );
            self.gpu_uploads.push_back(upload);
            self.events.push(AssetEvent::GpuUploadQueued { id });
        } else {
            if let Some(storage) = self.storages.get_mut(&asset_type) {
                if let Err(error) = storage.insert_boxed(id, asset, AssetLoadState::Ready) {
                    self.fail_asset(id, asset_type, error);
                    return;
                }
            }
            self.reload_rollbacks.remove(&id);
            if reloaded {
                self.events.push(AssetEvent::Reloaded { id });
            }
            self.events.push(AssetEvent::Ready { id });
        }
    }

    fn resolve_waiting_assets(&mut self) {
        let ids = self.waiting_assets.keys().copied().collect::<Vec<_>>();
        for id in ids {
            if let Some((dependency, error)) = self.failed_dependency(id) {
                let asset_type = self
                    .waiting_assets
                    .remove(&id)
                    .map(|waiting| waiting.asset_type)
                    .unwrap_or_else(|| self.asset_type_for_id(id).unwrap_or(AssetTypeId::NIL));
                self.events.push(AssetEvent::DependencyFailed {
                    id,
                    dependency,
                    error: error.clone(),
                });
                self.fail_asset(
                    id,
                    asset_type,
                    AssetError::DependencyFailed {
                        asset: id,
                        dependency,
                    },
                );
                continue;
            }
            if self.dependencies_ready(id) {
                if let Some(waiting) = self.waiting_assets.remove(&id) {
                    let Some(asset) = waiting.asset else {
                        continue;
                    };
                    for dependency in self.dependencies.direct_dependencies(id) {
                        self.events.push(AssetEvent::DependencyReady {
                            id,
                            dependency: *dependency,
                        });
                    }
                    self.finalize_cpu_loaded(
                        id,
                        waiting.asset_type,
                        asset,
                        waiting.gpu_upload,
                        waiting.reloaded,
                    );
                }
            }
        }
    }

    fn dependencies_ready(&self, id: AssetId) -> bool {
        self.dependencies
            .direct_dependencies(id)
            .iter()
            .all(|dependency| self.is_ready_by_id(*dependency))
    }

    fn failed_dependency(&self, id: AssetId) -> Option<(AssetId, AssetError)> {
        self.dependencies
            .direct_dependencies(id)
            .iter()
            .find_map(|dependency| {
                (self.state_by_id(*dependency) == AssetLoadState::Failed).then(|| {
                    (
                        *dependency,
                        self.error_by_id(*dependency)
                            .cloned()
                            .unwrap_or(AssetError::AssetNotFound { id: *dependency }),
                    )
                })
            })
    }

    fn queue_request(&mut self, request: LoadRequest) {
        self.ensure_builtin_storage_for_type(request.asset_type);
        let current_state = self.state_for_type(request.id, request.asset_type);
        if !request.reload
            && matches!(
                current_state,
                AssetLoadState::Queued
                    | AssetLoadState::LoadingBytes
                    | AssetLoadState::DecodingCpu
                    | AssetLoadState::WaitingForDependencies
                    | AssetLoadState::LoadedCpu
                    | AssetLoadState::UploadingGpu
                    | AssetLoadState::Ready
            )
        {
            return;
        }
        let queued_state = if request.reload {
            AssetLoadState::Reloading
        } else {
            AssetLoadState::Queued
        };
        self.set_state_for_type(request.id, request.asset_type, queued_state);
        if let Some(metadata) = self.registry.get(request.id).cloned() {
            if let Some(storage) = self.storages.get_mut(&request.asset_type) {
                storage.set_metadata(request.id, metadata);
            }
        }
        self.events.push(AssetEvent::LoadRequested {
            id: request.id,
            path: request.path.clone(),
            asset_type: request.asset_type,
        });
        self.scheduler.enqueue(request);
    }

    fn cancel_queued_request(&mut self, request: LoadRequest) {
        self.set_state_for_type(request.id, request.asset_type, AssetLoadState::Cancelled);
        self.waiting_assets.remove(&request.id);
        self.pending_gpu.remove(&request.id);
        self.reload_rollbacks.remove(&request.id);
        self.events.push(AssetEvent::Cancelled { id: request.id });
    }

    fn fail_asset(&mut self, id: AssetId, asset_type: AssetTypeId, error: AssetError) {
        self.ensure_builtin_storage_for_type(asset_type);
        let rollback = self.reload_rollbacks.remove(&id);
        let final_state = rollback
            .filter(|rollback| {
                let policy = self.hot_reload_rollback_policy_for_type(asset_type);
                rollback.asset_type == asset_type
                    && rollback.previous_state == AssetLoadState::Ready
                    && policy.can_retain_previous_ready_state()
            })
            .map(|rollback| rollback.previous_state);
        if let Some(storage) = self.storages.get_mut(&asset_type) {
            if let Some(state) = final_state {
                storage.set_error_with_state(id, error.clone(), state);
            } else {
                storage.set_error(id, error.clone());
            }
        } else {
            self.fallback_states
                .insert(id, final_state.unwrap_or(AssetLoadState::Failed));
            self.fallback_errors.insert(id, error.clone());
        }
        self.waiting_assets.remove(&id);
        self.pending_gpu.remove(&id);
        self.events.push(AssetEvent::Failed { id, error });
    }

    fn set_state_for_type(&mut self, id: AssetId, asset_type: AssetTypeId, state: AssetLoadState) {
        self.ensure_builtin_storage_for_type(asset_type);
        if let Some(storage) = self.storages.get_mut(&asset_type) {
            storage.set_state(id, state);
        } else {
            self.fallback_states.insert(id, state);
        }
    }

    fn state_for_type(&self, id: AssetId, asset_type: AssetTypeId) -> AssetLoadState {
        self.storages
            .get(&asset_type)
            .map(|storage| storage.state(id))
            .unwrap_or(AssetLoadState::Unloaded)
    }

    fn make_handle<T: Asset>(&mut self, id: AssetId, strength: HandleStrength) -> Handle<T> {
        let handle = Handle::new_tracked(id, strength, self.handle_lifecycle.clone());
        self.refresh_counts_for_id(id);
        handle
    }

    fn make_untyped_handle(
        &mut self,
        id: AssetId,
        asset_type: AssetTypeId,
        strength: HandleStrength,
    ) -> UntypedHandle {
        let handle =
            UntypedHandle::new_tracked(id, asset_type, strength, self.handle_lifecycle.clone());
        self.refresh_counts_for_id(id);
        handle
    }

    fn refresh_handle_counts(&mut self) {
        let mut ids = self.handle_lifecycle.tracked_ids();
        let mut seen = ids.iter().copied().collect::<HashSet<_>>();
        for storage in self.storages.values() {
            for id in storage.ids() {
                if seen.insert(id) {
                    ids.push(id);
                }
            }
        }
        for id in ids {
            self.refresh_counts_for_id(id);
        }
    }

    fn refresh_counts_for_id(&mut self, id: AssetId) {
        let (strong, weak) = self.live_handle_counts(id);
        let dependency = self.dependency_ref_counts.get(&id).copied().unwrap_or(0);
        if let Some(asset_type) = self.asset_type_for_id(id) {
            if let Some(storage) = self.storages.get_mut(&asset_type) {
                if !storage.ids().contains(&id) {
                    return;
                }
                storage.set_counts(id, strong, weak, dependency);
                if strong > 0 {
                    storage.mark_used(id, self.frame_index);
                }
            }
        }
    }

    fn live_handle_counts(&self, id: AssetId) -> (usize, usize) {
        let counts = self.handle_lifecycle.counts(id);
        (counts.strong, counts.weak)
    }

    fn refresh_dependency_counts(&mut self) {
        let mut dependency_ids = self
            .dependency_ref_counts
            .keys()
            .copied()
            .collect::<Vec<_>>();
        self.dependency_ref_counts.clear();
        let asset_ids = self
            .registry
            .values()
            .map(|metadata| metadata.id)
            .collect::<Vec<_>>();
        for id in asset_ids {
            if !matches!(
                self.state_by_id(id),
                AssetLoadState::Queued
                    | AssetLoadState::LoadingBytes
                    | AssetLoadState::DecodingCpu
                    | AssetLoadState::WaitingForDependencies
                    | AssetLoadState::LoadedCpu
                    | AssetLoadState::UploadingGpu
                    | AssetLoadState::Ready
                    | AssetLoadState::Reloading
            ) {
                continue;
            }
            for dependency in self.dependencies.direct_dependencies(id) {
                *self.dependency_ref_counts.entry(*dependency).or_default() += 1;
            }
        }
        for id in self.dependency_ref_counts.keys().copied() {
            if !dependency_ids.contains(&id) {
                dependency_ids.push(id);
            }
        }
        for id in dependency_ids {
            self.refresh_counts_for_id(id);
        }
        self.refresh_handle_counts();
    }

    fn register_group(&mut self, handles: Vec<UntypedHandle>) -> AssetLoadGroup {
        let id = AssetLoadGroupId(self.next_group_id);
        self.next_group_id += 1;
        self.groups
            .insert(id, handles.iter().map(|handle| handle.id()).collect());
        AssetLoadGroup {
            id,
            assets: handles,
        }
    }

    fn asset_type_for_id(&self, id: AssetId) -> Option<AssetTypeId> {
        self.registry
            .get(id)
            .map(|metadata| metadata.asset_type)
            .or_else(|| {
                self.storages.iter().find_map(|(asset_type, storage)| {
                    (storage.state(id) != AssetLoadState::Unloaded).then_some(*asset_type)
                })
            })
    }

    fn memory_bytes_for_id(&self, id: AssetId) -> (u64, u64) {
        self.asset_type_for_id(id)
            .and_then(|asset_type| self.storages.get(&asset_type))
            .map(|storage| storage.cpu_gpu_bytes(id))
            .unwrap_or((0, 0))
    }

    fn memory_bytes_total_for_id(&self, id: AssetId) -> u64 {
        let (cpu, gpu) = self.memory_bytes_for_id(id);
        cpu + gpu
    }

    fn loading_jobs_per_frame(&self) -> usize {
        self.config
            .max_io_jobs_per_frame
            .max(1)
            .min(self.config.max_cpu_jobs_per_frame.max(1))
    }

    #[cfg(all(feature = "bundle", feature = "streaming"))]
    fn bundle_entries(&self, id: BundleId) -> AssetResult<Vec<BundleEntry>> {
        self.mounted_bundle(id)
            .map(|bundle| bundle.manifest.entries.clone())
            .ok_or_else(|| AssetError::Bundle {
                message: format!("bundle is not mounted: {id:?}"),
            })
    }

    #[cfg(feature = "bundle")]
    fn register_bundle_entry_metadata(&mut self, entry: &BundleEntry) -> Option<()> {
        let path = entry.path.clone()?;
        let mut metadata = AssetMetadata::runtime(entry.id, path, entry.asset_type);
        metadata.cooked_hash = Some(entry.content_hash);
        metadata.dependencies = entry.dependencies.clone();
        self.registry.insert(metadata);
        Some(())
    }

    #[cfg(all(feature = "streaming", feature = "bundle"))]
    fn register_streaming_region_bundle_entries(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        entries: Vec<BundleEntry>,
    ) -> AssetResult<StreamingRegionId> {
        let mut handles = Vec::new();
        for entry in entries {
            if self.register_bundle_entry_metadata(&entry).is_some() {
                handles.push(self.make_untyped_handle(
                    entry.id,
                    entry.asset_type,
                    HandleStrength::Weak,
                ));
            }
        }
        Ok(self.register_streaming_region(name, priority, handles))
    }

    #[cfg(feature = "streaming")]
    fn retain_streaming_residency(&mut self, asset_ids: impl IntoIterator<Item = AssetId>) {
        for asset_id in asset_ids {
            *self.streaming_residency_counts.entry(asset_id).or_default() += 1;
            self.set_asset_resident(asset_id, true);
        }
    }

    #[cfg(feature = "streaming")]
    fn release_streaming_residency(&mut self, asset_ids: impl IntoIterator<Item = AssetId>) {
        for asset_id in asset_ids {
            let Some(count) = self.streaming_residency_counts.get_mut(&asset_id) else {
                self.set_asset_resident(asset_id, false);
                continue;
            };
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.streaming_residency_counts.remove(&asset_id);
                self.set_asset_resident(asset_id, false);
            }
        }
    }

    fn type_name(&self, asset_type: AssetTypeId) -> String {
        self.type_names
            .get(&asset_type)
            .cloned()
            .unwrap_or_else(|| format!("{asset_type:?}"))
    }

    fn ensure_builtin_storage_for_type(&mut self, asset_type: AssetTypeId) {
        if self.storages.contains_key(&asset_type) {
            return;
        }
        if asset_type == Texture::TYPE_ID {
            self.register_asset_type::<Texture>();
        } else if asset_type == Mesh::TYPE_ID {
            self.register_asset_type::<Mesh>();
        } else if asset_type == Shader::TYPE_ID {
            self.register_asset_type::<Shader>();
        } else if asset_type == Material::TYPE_ID {
            self.register_asset_type::<Material>();
        } else if asset_type == AudioClip::TYPE_ID {
            self.register_asset_type::<AudioClip>();
        } else if asset_type == AnimationClip::TYPE_ID {
            self.register_asset_type::<AnimationClip>();
        } else if asset_type == Skeleton::TYPE_ID {
            self.register_asset_type::<Skeleton>();
        } else if asset_type == SceneAsset::TYPE_ID {
            self.register_asset_type::<SceneAsset>();
        } else if asset_type == Prefab::TYPE_ID {
            self.register_asset_type::<Prefab>();
        } else if asset_type == Font::TYPE_ID {
            self.register_asset_type::<Font>();
        } else if asset_type == PhysicsMesh::TYPE_ID {
            self.register_asset_type::<PhysicsMesh>();
        }
    }
}

#[cfg(feature = "async_loading")]
fn run_async_load_request(
    request: LoadRequest,
    path: AssetPath,
    io: Arc<dyn AssetIo>,
    loaders: AssetLoaderRegistry,
    mut registry: AssetRegistry,
) -> AsyncLoadResult {
    let bytes = match io.read(path.path()) {
        Ok(bytes) => bytes,
        Err(error) => {
            return AsyncLoadResult {
                request,
                outcome: AsyncLoadOutcome::Failed {
                    state: AssetLoadState::LoadingBytes,
                    error: error.into(),
                },
            }
        }
    };

    let loader = match loaders.loader_for_path_and_type(Some(&path), request.asset_type) {
        Ok(loader) => loader,
        Err(error) => {
            return AsyncLoadResult {
                request,
                outcome: AsyncLoadOutcome::Failed {
                    state: AssetLoadState::DecodingCpu,
                    error,
                },
            }
        }
    };

    let mut context = crate::loader::LoadContext::new(request.id, path.clone(), &mut registry);
    let loaded = match loader.load(&mut context, &bytes, &LoaderSettings::default()) {
        Ok(loaded) => loaded,
        Err(error) => {
            return AsyncLoadResult {
                request,
                outcome: AsyncLoadOutcome::Failed {
                    state: AssetLoadState::DecodingCpu,
                    error,
                },
            }
        }
    };
    let (dependencies, subresources) = context.finish();
    AsyncLoadResult {
        request,
        outcome: AsyncLoadOutcome::Loaded {
            path,
            loaded,
            dependencies,
            subresources,
        },
    }
}

#[cfg(feature = "streaming")]
fn streaming_region_not_found(id: StreamingRegionId) -> AssetError {
    AssetError::AddressNotFound {
        address: format!("streaming region {:?}", id),
    }
}

fn is_live_insert_state(state: AssetLoadState) -> bool {
    matches!(
        state,
        AssetLoadState::Queued
            | AssetLoadState::LoadingBytes
            | AssetLoadState::DecodingCpu
            | AssetLoadState::WaitingForDependencies
            | AssetLoadState::LoadedCpu
            | AssetLoadState::UploadingGpu
            | AssetLoadState::Ready
            | AssetLoadState::Reloading
            | AssetLoadState::Unloading
    )
}

fn dependency_report_metadata_label(metadata: &AssetMetadata) -> String {
    let path = metadata
        .path
        .as_ref()
        .map(AssetPath::display_string)
        .unwrap_or_else(|| "unmapped".to_owned());
    format!("{path} | type {}", metadata.asset_type.raw())
}

fn type_memory_budget_satisfied(
    cpu_bytes: u64,
    gpu_bytes: u64,
    budget: &AssetTypeMemoryBudget,
) -> bool {
    budget
        .memory_budget_bytes
        .map_or(true, |limit| cpu_bytes.saturating_add(gpu_bytes) <= limit)
        && budget
            .cpu_budget_bytes
            .map_or(true, |limit| cpu_bytes <= limit)
        && budget
            .gpu_budget_bytes
            .map_or(true, |limit| gpu_bytes <= limit)
}

fn gpu_rollback_asset_type(asset_type: AssetTypeId) -> bool {
    matches!(
        asset_type,
        Texture::TYPE_ID | Mesh::TYPE_ID | Shader::TYPE_ID | Material::TYPE_ID
    )
}
