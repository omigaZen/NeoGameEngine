# `engine_asset` API 文档

版本：`v0.1-draft`
目标语言：Rust
适用范围：自研游戏引擎的资源管理系统，包括编辑器导入管线、运行时异步加载、资源依赖、Bundle、热重载、GPU 上传、卸载与 ECS 集成。

---

## 0. 设计目标

`engine_asset` 的目标不是做一个“路径读取工具”，而是做一套完整的资源生命周期系统。

它负责把源文件转换为可追踪、可依赖、可异步加载、可热重载、可打包、可卸载的运行时资源。

### 核心能力

```text
稳定 AssetId
路径与子资源标签解析
类型安全 Handle<T>
强 / 弱资源引用
类型化资源存储 Assets<T>
异步加载队列
资源加载状态追踪
资源依赖图
资源热重载
资源卸载与内存预算
GPU 上传队列
Asset Importer 编辑器导入管线
Cooker 运行时资源烘焙
Bundle / Pak 资源包
Streaming Region 流式加载
ECS 组件与系统集成
错误追踪与诊断
```

### 设计原则

```text
1. 资源身份使用 AssetId，不直接依赖路径。
2. 游戏逻辑持有 Handle<T>，不直接持有资源本体。
3. Runtime Loader 和 Editor Importer 分离。
4. CPU 加载与 GPU 上传分离。
5. AssetServer 是运行时入口，AssetDatabase 是编辑器入口。
6. 依赖加载由系统自动处理。
7. 热重载不替换 Handle，只替换 Handle 指向的资源内容。
8. 资源卸载由强引用、依赖引用和内存预算共同决定。
```

---

## 1. 总体架构

```text
Editor / Build Time

Source Assets
  │
  ▼
AssetDatabase
  │
  ├── ImporterRegistry
  ├── AssetImporter
  ├── AssetMetadata
  ├── DependencyGraph
  └── Cooker
        │
        ▼
Cooked Assets / Bundles


Runtime

AssetServer
  │
  ├── AssetRegistry
  ├── AssetIo
  ├── AssetLoaderRegistry
  ├── LoadScheduler
  ├── DependencyGraph
  ├── Assets<T>
  ├── GpuUploadQueue
  ├── HotReloadSystem
  ├── AssetGarbageCollector
  └── BundleRegistry
```

### 推荐目录

```text
assets/
  source/
    textures/
    models/
    audio/
    shaders/
    materials/
    scenes/
    prefabs/

  imported/
    *.meta
    *.asset

  cooked/
    *.texture
    *.mesh
    *.material
    *.shader
    *.audio
    *.scene

  bundles/
    base.bundle
    level_01.bundle
    characters.bundle
```

---

## 2. Crate 模块结构

```rust
pub mod prelude;

pub mod id;
pub mod path;
pub mod asset;
pub mod handle;
pub mod ref_asset;
pub mod storage;
pub mod server;
pub mod config;
pub mod registry;
pub mod metadata;
pub mod dependency;
pub mod events;
pub mod error;

pub mod io;
pub mod loader;
pub mod importer;
pub mod cooker;
pub mod bundle;
pub mod hot_reload;
pub mod gpu_upload;
pub mod gc;
pub mod streaming;
pub mod ecs;

pub mod assets;
```

其中 `assets` 用于放置内建资源类型：

```rust
pub mod assets {
    pub mod texture;
    pub mod mesh;
    pub mod material;
    pub mod shader;
    pub mod audio;
    pub mod animation;
    pub mod skeleton;
    pub mod scene;
    pub mod prefab;
    pub mod font;
    pub mod physics_mesh;
}
```

### Prelude

```rust
pub mod prelude {
    pub use crate::id::*;
    pub use crate::path::*;
    pub use crate::asset::*;
    pub use crate::handle::*;
    pub use crate::ref_asset::*;
    pub use crate::storage::*;
    pub use crate::server::*;
    pub use crate::config::*;
    pub use crate::metadata::*;
    pub use crate::events::*;
    pub use crate::error::*;

    pub use crate::assets::texture::*;
    pub use crate::assets::mesh::*;
    pub use crate::assets::material::*;
    pub use crate::assets::shader::*;
    pub use crate::assets::audio::*;
    pub use crate::assets::animation::*;
    pub use crate::assets::scene::*;
    pub use crate::assets::prefab::*;
}
```

---

## 3. 基础类型

## 3.1 `AssetId`

资源的稳定身份。路径可以变，`AssetId` 不应该变。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId(pub uuid::Uuid);

impl AssetId {
    pub const NIL: AssetId;

    pub fn new() -> Self;
    pub fn from_uuid(uuid: uuid::Uuid) -> Self;
    pub fn uuid(self) -> uuid::Uuid;
    pub fn is_nil(self) -> bool;
}
```

### 使用建议

```text
场景、Prefab、材质、动画等持久化文件中，优先保存 AssetId。
路径只作为 fallback，用于修复丢失引用。
```

---

## 3.2 `AssetTypeId`

运行时资源类型 ID。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetTypeId(pub uuid::Uuid);

impl AssetTypeId {
    pub fn of<T: Asset>() -> Self;
}
```

资源类型也可以使用静态名称：

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetTypeName(pub String);
```

---

## 3.3 `AssetPath`

资源路径。支持子资源标签。

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetPath {
    pub path: String,
    pub label: Option<String>,
}
```

示例：

```text
textures/hero_albedo.png
models/hero.glb#Mesh0
models/hero.glb#Animation/Run
materials/hero.material
scenes/forest.scene
```

API：

```rust
impl AssetPath {
    pub fn new(path: impl Into<String>) -> Self;
    pub fn with_label(path: impl Into<String>, label: impl Into<String>) -> Self;

    pub fn parse(value: &str) -> Self;

    pub fn path(&self) -> &str;
    pub fn label(&self) -> Option<&str>;
    pub fn extension(&self) -> Option<&str>;

    pub fn without_label(&self) -> Self;
    pub fn display_string(&self) -> String;
}
```

---

## 3.4 `AssetKey`

完整资源键，用于运行时定位资源。

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetKey {
    pub id: AssetId,
    pub path: Option<AssetPath>,
    pub asset_type: AssetTypeId,
}
```

---

## 3.5 Hash 类型

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContentHash(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VersionHash(pub u64);
```

用途：

```text
source_hash   ：源文件内容 hash
settings_hash ：导入设置 hash
cooked_hash   ：烘焙后内容 hash
version_hash  ：资源格式版本 hash
```

---

## 4. `Asset` Trait

所有运行时资源都实现 `Asset`。

```rust
pub trait Asset: Send + Sync + 'static {
    const TYPE_NAME: &'static str;
    const TYPE_ID: AssetTypeId;
}
```

可选扩展：

```rust
pub trait AssetMemoryUsage {
    fn cpu_bytes(&self) -> u64;
    fn gpu_bytes(&self) -> u64;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetDependencyReference {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub fallback_path: Option<AssetPath>,
}

impl AssetDependencyReference {
    pub fn new(id: AssetId, asset_type: AssetTypeId) -> Self;
    pub fn with_fallback_path(
        id: AssetId,
        asset_type: AssetTypeId,
        fallback_path: AssetPath,
    ) -> Self;
    pub fn from_handle(handle: UntypedHandle) -> Self;
    pub fn id(&self) -> AssetId;
    pub fn asset_type(&self) -> AssetTypeId;
    pub fn fallback_path(&self) -> Option<&AssetPath>;
    pub fn to_untyped_handle(&self) -> UntypedHandle;
}

pub trait AssetDependencies {
    fn visit_dependencies(&self, visitor: &mut dyn FnMut(AssetDependencyReference));
}
```

示例：

```rust
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_count: u32,
    pub gpu: Option<GpuTextureHandle>,
}

impl Asset for Texture {
    const TYPE_NAME: &'static str = "Texture";
    const TYPE_ID: AssetTypeId = AssetTypeId(uuid::uuid!("00000000-0000-0000-0000-000000000001"));
}
```

---

## 5. Handle 系统

## 5.1 Handle 强度

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleStrength {
    /// 强引用。资源不会被自动 GC 卸载。
    Strong,

    /// 弱引用。不阻止资源卸载。
    Weak,
}
```

---

## 5.2 `Handle<T>`

类型安全资源引用。

```rust
#[derive(Debug)]
pub struct Handle<T: Asset> {
    // opaque Arc-backed handle identity
}
```

API：

```rust
impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self;
}

impl<T: Asset> Handle<T> {
    pub fn strong(id: AssetId) -> Self;
    pub fn weak(id: AssetId) -> Self;

    pub fn id(&self) -> AssetId;
    pub fn asset_type(&self) -> AssetTypeId;
    pub fn strength(&self) -> HandleStrength;

    pub fn is_strong(&self) -> bool;
    pub fn is_weak(&self) -> bool;

    pub fn clone_weak(&self) -> Self;
    pub fn clone_strong(&self) -> Self;

    pub fn untyped(&self) -> UntypedHandle;
}
```

### Drop 语义

```rust
impl<T: Asset> Drop for Handle<T> {
    fn drop(&mut self) {
        // Server-created handles release their lifecycle count automatically.
    }
}
```

当前实现使用 `Arc<HandleInner>` 保存 id、asset type、strength，以及可选的
`AssetServer` lifecycle tracker。通过 `AssetServer::load*` / `preload*` / `load_untyped*`
创建的 handle 会自动登记引用计数；从这些 handle 派生的 `clone`、`clone_weak`、
`clone_strong`、`untyped` 和 `UntypedHandle::typed` 也会登记。`Drop` 会释放对应
strong/weak 计数，不需要外部显式 release。直接调用 `Handle::strong` /
`Handle::weak` / `UntypedHandle::new` 创建的 standalone identity handle 不绑定 server
lifecycle tracker，因此不会影响某个 `AssetServer` 的 GC 计数。

```text
Server-created strong Handle live  -> strong_count > 0, prevents unused GC
Server-created weak Handle live    -> weak_count visible, does not prevent unused GC
Derived clone/conversion live      -> counted until Drop
Standalone identity Handle live    -> not counted by AssetServer
```

---

## 5.3 `UntypedHandle`

非类型化资源引用，用于依赖图、序列化、编辑器、Bundle manifest。

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UntypedHandle {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub strength: HandleStrength,
}

impl UntypedHandle {
    pub fn typed<T: Asset>(&self) -> Option<Handle<T>>;
    pub fn clone_weak(&self) -> Self;
    pub fn clone_strong(&self) -> Self;
}
```

---

## 5.4 `AssetRef<T>`

用于序列化场景、Prefab、材质等持久化资源引用。

```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AssetRef<T: Asset> {
    pub id: AssetId,
    pub fallback_path: Option<AssetPath>,

    #[serde(skip)]
    marker: std::marker::PhantomData<T>,
}

impl<T: Asset> AssetRef<T> {
    pub fn new(id: AssetId) -> Self;
    pub fn with_fallback(id: AssetId, path: AssetPath) -> Self;

    pub fn load(&self, assets: &mut AssetServer) -> Handle<T>;
    pub fn id(&self) -> AssetId;
    pub fn dependency(&self) -> AssetDependencyReference;
    pub fn dependency_handle(&self) -> UntypedHandle;
    pub fn visit_dependency(&self, visitor: &mut dyn FnMut(AssetDependencyReference));
}
```

### 推荐序列化形式

```ron
AssetRef(
    id: "2df0f450-28c7-47ad-90a8-4c15d709f9aa",
    fallback_path: Some("models/tree.glb#Mesh0"),
)
```

---

## 6. 资源加载状态

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetLoadState {
    /// 还没有加载。
    Unloaded,

    /// 已进入加载队列。
    Queued,

    /// 正在读取字节。
    LoadingBytes,

    /// 正在 CPU 解码 / 反序列化。
    DecodingCpu,

    /// 正在等待依赖资源。
    WaitingForDependencies,

    /// CPU 资源已经构建完成。
    LoadedCpu,

    /// 正在提交 GPU 上传。
    UploadingGpu,

    /// 资源可用。
    Ready,

    /// 加载失败。
    Failed,

    /// 已在解码前取消。
    Cancelled,

    /// 正在热重载。
    Reloading,

    /// 正在卸载。
    Unloading,
}
```

---

## 7. 资源事件

```rust
#[derive(Clone, Debug)]
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
```

### 事件游标

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct AssetEventCursor {
    index: usize,
}

impl AssetServer {
    pub fn events(&self) -> &[AssetEvent];
    pub fn drain_events(&mut self) -> impl Iterator<Item = AssetEvent> + '_;
    pub fn events_since(&self, cursor: &mut AssetEventCursor) -> &[AssetEvent];
    pub fn clear_events(&mut self);
}
```

---

## 8. 类型化资源存储 `Assets<T>`

## 8.1 `AssetEntry<T>`

```rust
pub struct AssetEntry<T: Asset> {
    pub id: AssetId,
    pub asset: Option<T>,
    pub state: AssetLoadState,
    pub metadata: Option<AssetMetadata>,
    pub strong_count: usize,
    pub weak_count: usize,
    pub dependency_ref_count: usize,
    pub last_used_frame: u64,
    pub resident: bool,
    pub error: Option<AssetError>,
}
```

---

## 8.2 `Assets<T>` API

```rust
pub struct Assets<T: Asset> {
    // private
}

impl<T: Asset> Assets<T> {
    pub fn new() -> Self;

    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;

    pub fn contains(&self, id: AssetId) -> bool;
    pub fn entry(&self, id: AssetId) -> Option<&AssetEntry<T>>;
    pub fn entry_mut(&mut self, id: AssetId) -> Option<&mut AssetEntry<T>>;
    pub fn ensure_entry(&mut self, id: AssetId) -> &mut AssetEntry<T>;

    pub fn get(&self, handle: &Handle<T>) -> Option<&T>;
    pub fn get_by_id(&self, id: AssetId) -> Option<&T>;
    pub fn get_cpu_by_id(&self, id: AssetId) -> Option<&T>;

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T>;
    pub fn get_mut_by_id(&mut self, id: AssetId) -> Option<&mut T>;

    pub fn insert(&mut self, id: AssetId, asset: T) -> Option<T>;
    pub fn insert_with_state(
        &mut self,
        id: AssetId,
        asset: T,
        state: AssetLoadState,
    ) -> Option<T>;
    pub fn remove(&mut self, id: AssetId) -> Option<T>;

    pub fn state(&self, id: AssetId) -> AssetLoadState;
    pub fn set_state(&mut self, id: AssetId, state: AssetLoadState);

    pub fn error(&self, id: AssetId) -> Option<&AssetError>;

    pub fn iter(&self) -> impl Iterator<Item = (AssetId, &T)>;
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId, &mut T)>;

    pub fn mark_used(&mut self, id: AssetId, frame: u64);
}
```

---

## 9. `AssetServer`

`AssetServer` 是运行时资源系统主入口。

## 9.1 配置

```rust
#[derive(Clone, Debug)]
pub struct AssetServerConfig {
    pub root: std::path::PathBuf,
    pub cooked_root: std::path::PathBuf,
    pub enable_hot_reload: bool,
    pub hot_reload_dependency_policy: HotReloadDependencyPolicy,
    pub enable_async_loading: bool,
    pub worker_threads: usize,
    pub max_io_jobs_per_frame: usize,
    pub max_cpu_jobs_per_frame: usize,
    pub max_gpu_uploads_per_frame: usize,
    pub gc: AssetGcConfig,
}

impl Default for AssetServerConfig {
    fn default() -> Self;
}
```

加载执行策略可用下面的报告接口观察。当前实现的 `async_loading` 路径会把 IO 读取和
CPU decode 调度到可复用的长生命周期 worker pool，并在后续 `update_loading` 中收集结果、合并依赖/子资源元数据、
推进等待依赖与 GPU 上传交接。每帧 dispatch 仍受 IO/CPU 预算中更严格的值约束；未启用
`parallel` 时一次最多保持 1 个 async worker in-flight，启用 `parallel` 且配置
`worker_threads > 1` 时会按该 worker 数并发调度。worker pool 会按需懒创建、跨多次
`update_loading` 复用，并可通过 report/shutdown API 观察生命周期。

```rust
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
```

`effective_jobs_per_frame` 使用 `max_io_jobs_per_frame` 和 `max_cpu_jobs_per_frame` 中更严格的值。
当配置请求未启用的 `async_loading` 或 `parallel` 能力时，report 会包含
`AssetError::Unsupported`，`validate_loading_policy` 会返回同一个错误。
`AssetAsyncWorkerPoolReport` 只在启用 `async_loading` feature 时可用，用于观察期望 worker 数、
实际存活 worker 数、in-flight job、累计 dispatch/complete 数、累计启动线程数和显式 shutdown 次数。

---

## 9.2 创建与注册

```rust
pub struct AssetServer {
    // private
}

impl AssetServer {
    pub fn new(config: AssetServerConfig) -> Self;

    pub fn config(&self) -> &AssetServerConfig;
    pub fn config_mut(&mut self) -> &mut AssetServerConfig;

    pub fn loading_policy_report(&self) -> AssetLoadingPolicyReport;
    pub fn validate_loading_policy(&self) -> AssetResult<()>;

    #[cfg(feature = "async_loading")]
    pub fn async_worker_pool_report(&self) -> AssetAsyncWorkerPoolReport;
    #[cfg(feature = "async_loading")]
    pub fn shutdown_async_worker_pool(&mut self) -> AssetAsyncWorkerPoolReport;

    pub fn set_async_loading_enabled(&mut self, enabled: bool) -> AssetResult<()>;
    pub fn set_parallel_worker_threads(&mut self, worker_threads: usize) -> AssetResult<()>;

    pub fn register_builtin_asset_types(&mut self);
    pub fn register_builtin_loaders(&mut self);

    pub fn register_asset_type<T: Asset>(&mut self);
    pub fn is_asset_type_registered<T: Asset>(&self) -> bool;

    pub fn register_loader<L: AssetLoader>(&mut self, loader: L);
    pub fn register_boxed_loader(&mut self, loader: Box<dyn AssetLoader>);

    pub fn set_io<I: AssetIo>(&mut self, io: I);
    pub fn set_registry(&mut self, registry: AssetRegistry);
}
```

---

## 9.3 加载 API

```rust
impl AssetServer {
    pub fn load<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T>;

    pub fn load_with_priority<T: Asset>(
        &mut self,
        path: impl Into<AssetPath>,
        priority: LoadPriority,
    ) -> Handle<T>;

    pub fn load_by_id<T: Asset>(&mut self, id: AssetId) -> Handle<T>;

    pub fn load_by_id_with_priority<T: Asset>(
        &mut self,
        id: AssetId,
        priority: LoadPriority,
    ) -> Handle<T>;

    pub fn load_untyped(&mut self, path: impl Into<AssetPath>) -> UntypedHandle;

    pub fn load_untyped_by_id(
        &mut self,
        id: AssetId,
        asset_type: AssetTypeId,
    ) -> UntypedHandle;

    pub fn preload<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T>;

    pub fn preload_by_id<T: Asset>(&mut self, id: AssetId) -> Handle<T>;

    pub fn insert_loaded<T: Asset>(
        &mut self,
        path: impl Into<AssetPath>,
        asset: T,
    ) -> Result<Handle<T>, AssetError>;

    pub fn insert_loaded_by_id<T: Asset>(
        &mut self,
        id: AssetId,
        asset: T,
    ) -> Result<Handle<T>, AssetError>;

    pub fn insert_loaded_with_metadata<T: Asset>(
        &mut self,
        metadata: AssetMetadata,
        asset: T,
    ) -> Result<Handle<T>, AssetError>;
}
```

`insert_loaded*` 是显式注入 already-decoded/ready 资源的路径，用于测试、工具或
运行时生成资源。它会写入/更新 registry metadata、typed storage、dependency graph、
memory info，并发出 `LoadedCpu` 和 `Ready` 事件；如果同一 `AssetId` 当前处于
`Queued`、`LoadingBytes`、`DecodingCpu`、`WaitingForDependencies`、`LoadedCpu`、
`UploadingGpu`、`Ready`、`Reloading` 或 `Unloading`，会返回
`AssetError::AlreadyLoaded { id }`，避免静默覆盖 live asset。`Failed`、`Cancelled`
或已 `Unloaded` 的 id 可以重新插入。

---

## 9.4 批量加载

```rust
pub struct AssetLoadGroup {
    pub id: AssetLoadGroupId,
    pub assets: Vec<UntypedHandle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AssetLoadGroupId(pub u64);
```

```rust
impl AssetServer {
    pub fn load_group(&mut self, assets: &[AssetPath]) -> AssetLoadGroup;

    pub fn load_group_by_ids(
        &mut self,
        assets: &[(AssetId, AssetTypeId)],
    ) -> AssetLoadGroup;

    pub fn group_state(&self, group: &AssetLoadGroup) -> AssetLoadState;

    pub fn group_progress(&self, group: &AssetLoadGroup) -> LoadProgress;

    pub fn is_group_tracked(&self, id: AssetLoadGroupId) -> bool;

    pub fn cancel_load_by_id(&mut self, id: AssetId) -> bool;

    pub fn cancel_load_by_path(&mut self, path: impl Into<AssetPath>) -> bool;

    pub fn cancel_load_group(&mut self, group: &AssetLoadGroup) -> usize;

    pub fn release_group(&mut self, group: AssetLoadGroup);
}
```

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct LoadProgress {
    pub total_assets: usize,
    pub queued_assets: usize,
    pub loading_assets: usize,
    pub ready_assets: usize,
    pub failed_assets: usize,
    pub cancelled_assets: usize,
    pub bytes_total: u64,
    pub bytes_loaded: u64,
}
```

`release_group` 只释放跟踪关系，不会取消已排队的加载；需要取消时使用
`cancel_load_group`。`bytes_total` 和 `bytes_loaded` 当前按已常驻资源的
CPU+GPU 内存字节统计，尚未加载的资源不贡献字节数。

---

## 9.5 访问 API

```rust
impl AssetServer {
    pub fn get<T: Asset>(&self, handle: &Handle<T>) -> Option<&T>;
    pub fn get_by_id<T: Asset>(&self, id: AssetId) -> Option<&T>;

    pub fn get_mut<T: Asset>(&mut self, handle: &Handle<T>) -> Option<&mut T>;
    pub fn get_mut_by_id<T: Asset>(&mut self, id: AssetId) -> Option<&mut T>;

    pub fn storage<T: Asset>(&self) -> Option<&Assets<T>>;
    pub fn storage_mut<T: Asset>(&mut self) -> Option<&mut Assets<T>>;

    pub fn state<T: Asset>(&self, handle: &Handle<T>) -> AssetLoadState;
    pub fn state_by_id(&self, id: AssetId) -> AssetLoadState;

    pub fn error_by_id(&self, id: AssetId) -> Option<&AssetError>;

    pub fn is_ready<T: Asset>(&self, handle: &Handle<T>) -> bool;
    pub fn is_ready_by_id(&self, id: AssetId) -> bool;

    pub fn is_ready_with_dependencies<T: Asset>(&self, handle: &Handle<T>) -> bool;
}
```

---

## 9.6 更新与任务推进

```rust
impl AssetServer {
    /// 每帧调用一次。
    /// 推进 IO 完成任务、CPU 解码完成任务、GPU 上传状态、GC 和热重载事件。
    pub fn update(&mut self, frame_index: u64);

    /// 只推进异步加载任务。
    pub fn update_loading(&mut self);

    /// 只处理热重载。
    pub fn update_hot_reload(&mut self);

    /// 只处理资源 GC。
    pub fn update_gc(&mut self, frame_index: u64);
}
```

---

## 9.7 重新加载与卸载

```rust
impl AssetServer {
    pub fn reload<T: Asset>(&mut self, handle: &Handle<T>) -> Result<(), AssetError>;

    pub fn reload_by_id(&mut self, id: AssetId) -> Result<(), AssetError>;

    pub fn reload_by_path(&mut self, path: &AssetPath) -> Result<(), AssetError>;

    pub fn unload<T: Asset>(&mut self, handle: Handle<T>);

    pub fn unload_by_id(&mut self, id: AssetId) -> Result<(), AssetError>;

    pub fn unload_unused(&mut self);

    pub fn set_asset_resident(&mut self, id: AssetId, resident: bool);

    pub fn is_asset_resident(&self, id: AssetId) -> bool;
}
```

---

## 9.8 元数据与注册表访问

```rust
impl AssetServer {
    pub fn registry(&self) -> &AssetRegistry;
    pub fn registry_mut(&mut self) -> &mut AssetRegistry;

    pub fn metadata(&self, id: AssetId) -> Option<&AssetMetadata>;
    pub fn metadata_by_path(&self, path: &AssetPath) -> Option<&AssetMetadata>;

    pub fn id_from_path(&self, path: &AssetPath) -> Option<AssetId>;
    pub fn path_from_id(&self, id: AssetId) -> Option<&AssetPath>;
}
```

---

## 10. 加载优先级与请求

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadPriority {
    Immediate,
    High,
    Normal,
    Low,
    Background,
}
```

```rust
#[derive(Clone, Debug)]
pub struct LoadRequest {
    pub id: AssetId,
    pub path: Option<AssetPath>,
    pub asset_type: AssetTypeId,
    pub priority: LoadPriority,
    pub recursive_dependencies: bool,
    pub reload: bool,
}
```

```rust
pub struct LoadScheduler {
    // private
}

impl LoadScheduler {
    pub fn new() -> Self;

    pub fn enqueue(&mut self, request: LoadRequest);
    pub fn cancel(&mut self, id: AssetId) -> Option<LoadRequest>;
    pub fn contains(&self, id: AssetId) -> bool;

    pub fn pop_next(&mut self) -> Option<LoadRequest>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

`AssetServer::update_loading` 每帧最多 dispatch
`min(max_io_jobs_per_frame.max(1), max_cpu_jobs_per_frame.max(1))` 个加载请求。同步路径会在
当前线程执行 IO 读取和 CPU decode；启用 `async_loading` 时会先收集已完成的 worker 结果，维护可复用
worker pool，再按同一预算把 queued 请求派发到后台 worker。状态、取消、错误、依赖等待、事件和 GPU
handoff 都保留同一套可观察语义。

---

## 11. `AssetIo`

`AssetIo` 抽象资源来源。

```rust
pub trait AssetIo: Send + Sync + 'static {
    fn exists(&self, path: &str) -> bool;

    fn read(&self, path: &str) -> Result<Vec<u8>, AssetIoError>;

    fn read_range(
        &self,
        path: &str,
        offset: u64,
        length: u64,
    ) -> Result<Vec<u8>, AssetIoError>;

    fn metadata(&self, path: &str) -> Result<AssetIoMetadata, AssetIoError>;

    fn list(&self, directory: &str) -> Result<Vec<String>, AssetIoError>;
}
```

```rust
#[derive(Clone, Debug)]
pub struct AssetIoMetadata {
    pub path: String,
    pub size: u64,
    pub modified_time: Option<std::time::SystemTime>,
    pub hash: Option<ContentHash>,
}
```

## 11.1 文件系统 IO

```rust
pub struct FileSystemAssetIo {
    pub root: std::path::PathBuf,
}

impl FileSystemAssetIo {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self;
}
```

当 Cargo `filesystem` feature 关闭时，`FileSystemAssetIo` 作为稳定公开类型仍可构造，
但 `exists` 恒为 `false`，`read`/`metadata`/`list` 会返回带 action/path/message 的
`AssetIoError::ReadFailed`，message 为 `asset filesystem feature is disabled`。
因此 `AssetServer::new` 和 `AssetDatabase::new` 的默认 filesystem-backed IO 在
`--no-default-features` 下不会静默读盘；调用方仍可用 `set_io(MemoryAssetIo::new())`
注入内存/测试 IO。

## 11.2 Bundle IO

```rust
pub struct BundleAssetIo {
    // private
}

impl BundleAssetIo {
    pub fn new(reader: BundleReader) -> Self;
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AssetError>;
    pub fn from_bytes_with_loading_policy(
        bytes: &[u8],
        chunk_loading_policy: BundleChunkLoadingPolicy,
    ) -> Result<Self, AssetError>;
    pub fn manifest(&self) -> &BundleManifest;
    pub fn read_with_report(&self, path: &str) -> AssetResult<(Vec<u8>, BundleChunkReadReport)>;
    pub fn read_range_with_report(
        &self,
        path: &str,
        offset: u64,
        length: u64,
    ) -> AssetResult<(Vec<u8>, BundleChunkReadReport)>;
    pub fn chunk_cache_stats(&self) -> BundleChunkCacheStats;
    pub fn prefetch_chunk(&self, chunk_index: u32) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn prefetch_chunks(&self, chunk_indices: &[u32]) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn prefetch_path(&self, path: &str) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn prefetch_paths(&self, paths: &[&str]) -> AssetResult<BundleChunkPrefetchReport>;
}
```

`BundleAssetIo::read_range` 使用 `BundleReader::read_path_range`，因此对压缩 bundle 会在
chunk 解码后返回 entry 局部范围，而不是要求调用方理解 bundle data section 布局。使用
`from_bytes_with_loading_policy(..., BundleChunkLoadingPolicy::OnDemandCached*)` 时，
`read*_with_report`、`prefetch_*` 和 `chunk_cache_stats` 可观察 chunk 首次解码、后续
cache hit、prefetch 和 bounded cache eviction。

## 11.3 Composite IO

```rust
pub enum AssetIoLayerKind {
    Source,
    Mod,
    Patch,
    Bundle,
    BaseBundle,
    Memory,
    FileSystem,
    Custom,
}

pub struct AssetIoLayerInfo {
    pub name: String,
    pub kind: AssetIoLayerKind,
    pub priority: usize,
}

pub struct AssetIoResolution {
    pub path: String,
    pub layer: AssetIoLayerInfo,
}

pub struct AssetIoListedPath {
    pub path: String,
    pub layer: AssetIoLayerInfo,
}

pub struct CompositeAssetIo {
    // private layers
}

impl CompositeAssetIo {
    pub fn new() -> Self;
    pub fn with_layer<I: AssetIo>(self, io: I) -> Self;
    pub fn with_named_layer<I: AssetIo>(
        self,
        name: impl Into<String>,
        kind: AssetIoLayerKind,
        io: I,
    ) -> Self;
    pub fn push_layer<I: AssetIo>(&mut self, io: I);
    pub fn push_named_layer<I: AssetIo>(
        &mut self,
        name: impl Into<String>,
        kind: AssetIoLayerKind,
        io: I,
    );
    pub fn layers(&self) -> Vec<AssetIoLayerInfo>;
    pub fn resolve(&self, path: &str) -> Option<AssetIoResolution>;
    pub fn read_with_diagnostics(
        &self,
        path: &str,
    ) -> Result<(Vec<u8>, AssetIoResolution), AssetIoError>;
    pub fn metadata_with_diagnostics(
        &self,
        path: &str,
    ) -> Result<(AssetIoMetadata, AssetIoResolution), AssetIoError>;
    pub fn list_with_diagnostics(
        &self,
        directory: &str,
    ) -> Result<Vec<AssetIoListedPath>, AssetIoError>;
}
```

搜索顺序建议按高优先级到低优先级注册，`CompositeAssetIo` 使用 first-layer-wins 语义：

```text
source/
mods/
patch/
base bundle/
```

`resolve`、`read_with_diagnostics`、`metadata_with_diagnostics` 和
`list_with_diagnostics` 用于显示某个路径最终由哪个层提供；`list` 会去重，
并保留最高优先级层提供的路径。

## 11.4 Package Registry And Override Audit

持久化 mod/patch/base bundle 顺序使用 `AssetPackageRegistry`。Registry 记录 package
的 bundle id、名称、IO 层类型、优先级、启用状态、payload 路径和 manifest
元数据；payload 字节仍由调用方提供。

```rust
pub struct AssetPackageRecord {
    pub bundle_id: BundleId,
    pub name: String,
    pub kind: AssetIoLayerKind,
    pub priority: usize,
    pub enabled: bool,
    pub bundle_path: String,
    pub package_version: u32,
    pub minimum_runtime_version: u32,
    pub package_dependencies: Vec<AssetPackageDependency>,
    pub manifest: BundleManifest,
}

impl AssetPackageRecord {
    pub const DEFAULT_PACKAGE_VERSION: u32 = 1;
    pub const CURRENT_RUNTIME_VERSION: u32 = 1;

    pub fn new(
        bundle_id: BundleId,
        name: impl Into<String>,
        kind: AssetIoLayerKind,
        priority: usize,
        enabled: bool,
        bundle_path: impl Into<String>,
        manifest: BundleManifest,
    ) -> Self;
    pub fn with_package_version(self, package_version: u32) -> Self;
    pub fn with_minimum_runtime_version(self, minimum_runtime_version: u32) -> Self;
    pub fn with_package_dependency(self, dependency: AssetPackageDependency) -> Self;
    pub fn with_package_dependencies(self, dependencies: Vec<AssetPackageDependency>) -> Self;
}

pub struct AssetPackageDependency {
    pub package: String,
    pub min_version: u32,
    pub max_version: Option<u32>,
}

impl AssetPackageDependency {
    pub fn new(package: impl Into<String>, min_version: u32) -> Self;
    pub fn with_max_version(self, max_version: u32) -> Self;
}

pub struct AssetPackageLayerInfo {
    pub bundle_id: BundleId,
    pub name: String,
    pub kind: AssetIoLayerKind,
    pub priority: usize,
    pub bundle_path: String,
    pub package_version: u32,
    pub minimum_runtime_version: u32,
}

pub struct AssetPackageConflict {
    pub path: AssetPath,
    pub winner: AssetPackageLayerInfo,
    pub shadowed: Vec<AssetPackageLayerInfo>,
}

pub struct AssetPackageConflictReport {
    pub conflicts: Vec<AssetPackageConflict>,
}

impl AssetPackageConflictReport {
    pub fn has_conflicts(&self) -> bool;
}

pub struct AssetPackageAssetInfo {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub content_hash: ContentHash,
    pub dependencies: Vec<AssetId>,
}

pub struct AssetPackageDependencyProvider {
    pub dependency: AssetId,
    pub provider: Option<AssetPackageLayerInfo>,
}

pub enum AssetPackageAssetOverrideIssueKind {
    AssetIdChanged,
    AssetTypeChanged,
    ContentHashChanged,
    DependenciesChanged,
    DependencyProvidersChanged,
}

pub struct AssetPackageAssetOverride {
    pub path: AssetPath,
    pub winner: AssetPackageLayerInfo,
    pub shadowed: AssetPackageLayerInfo,
    pub winner_asset: AssetPackageAssetInfo,
    pub shadowed_asset: AssetPackageAssetInfo,
    pub winner_dependency_providers: Vec<AssetPackageDependencyProvider>,
    pub shadowed_dependency_providers: Vec<AssetPackageDependencyProvider>,
    pub issues: Vec<AssetPackageAssetOverrideIssueKind>,
}

impl AssetPackageAssetOverride {
    pub fn has_issues(&self) -> bool;
}

pub struct AssetPackageAssetOverrideReport {
    pub overrides: Vec<AssetPackageAssetOverride>,
}

impl AssetPackageAssetOverrideReport {
    pub fn has_overrides(&self) -> bool;
    pub fn has_issues(&self) -> bool;
}

pub struct AssetPackageAssetCompatibilityPolicy {
    pub require_stable_asset_ids: bool,
    pub require_matching_asset_types: bool,
    pub require_matching_content_hashes: bool,
    pub require_matching_dependencies: bool,
    pub require_matching_dependency_providers: bool,
}

impl AssetPackageAssetCompatibilityPolicy {
    pub const fn permissive() -> Self;
    pub const fn strict() -> Self;
    pub fn with_stable_asset_ids_required(self, required: bool) -> Self;
    pub fn with_matching_asset_types_required(self, required: bool) -> Self;
    pub fn with_matching_content_hashes_required(self, required: bool) -> Self;
    pub fn with_matching_dependencies_required(self, required: bool) -> Self;
    pub fn with_matching_dependency_providers_required(self, required: bool) -> Self;
}

pub enum AssetPackageCompatibilityIssueKind {
    RuntimeTooOld,
    VersionDowngrade,
    MissingPackageDependency,
    PackageDependencyTooOld,
    PackageDependencyTooNew,
    AssetIdChanged,
    AssetTypeChanged,
    AssetContentHashChanged,
    AssetDependenciesChanged,
    AssetDependencyProvidersChanged,
}

pub struct AssetPackageCompatibilityIssue {
    pub package: String,
    pub kind: AssetPackageCompatibilityIssueKind,
    pub previous_version: Option<u32>,
    pub next_version: u32,
    pub runtime_version: u32,
    pub minimum_runtime_version: u32,
    pub dependency: Option<String>,
    pub dependency_version: Option<u32>,
    pub required_min_version: Option<u32>,
    pub required_max_version: Option<u32>,
    pub asset_override: Option<AssetPackageAssetOverride>,
    pub message: String,
}

pub struct AssetPackageUpdatePolicy {
    pub runtime_version: u32,
    pub allow_version_downgrade: bool,
    pub asset_compatibility: AssetPackageAssetCompatibilityPolicy,
}

impl AssetPackageUpdatePolicy {
    pub fn new(runtime_version: u32) -> Self;
    pub fn with_version_downgrade_allowed(self, allow_version_downgrade: bool) -> Self;
    pub fn with_asset_compatibility(
        self,
        asset_compatibility: AssetPackageAssetCompatibilityPolicy,
    ) -> Self;
}

pub struct AssetPackageUpdateChange {
    pub name: String,
    pub previous_version: Option<u32>,
    pub next_version: Option<u32>,
}

pub struct AssetPackageUpdateReport {
    pub policy: AssetPackageUpdatePolicy,
    pub added: Vec<AssetPackageUpdateChange>,
    pub removed: Vec<AssetPackageUpdateChange>,
    pub updated: Vec<AssetPackageUpdateChange>,
    pub enabled: Vec<AssetPackageUpdateChange>,
    pub disabled: Vec<AssetPackageUpdateChange>,
    pub compatibility_issues: Vec<AssetPackageCompatibilityIssue>,
    pub conflicts: AssetPackageConflictReport,
    pub asset_overrides: AssetPackageAssetOverrideReport,
}

impl AssetPackageUpdateReport {
    pub fn is_compatible(&self) -> bool;
    pub fn require_compatible(&self) -> AssetResult<()>;
}

pub struct AssetPackageActivation {
    pub report: AssetPackageUpdateReport,
    pub mounted_bundles: Vec<MountedBundle>,
}

pub struct AssetPackageInstallRequest {
    pub bundle_id: BundleId,
    pub name: String,
    pub kind: AssetIoLayerKind,
    pub priority: usize,
    pub enabled: bool,
    pub bundle_path: String,
    pub package_version: u32,
    pub minimum_runtime_version: u32,
    pub package_dependencies: Vec<AssetPackageDependency>,
}

impl AssetPackageInstallRequest {
    pub fn new(
        bundle_id: BundleId,
        name: impl Into<String>,
        kind: AssetIoLayerKind,
        priority: usize,
        bundle_path: impl Into<String>,
    ) -> Self;
    pub fn with_enabled(self, enabled: bool) -> Self;
    pub fn with_package_version(self, package_version: u32) -> Self;
    pub fn with_minimum_runtime_version(self, minimum_runtime_version: u32) -> Self;
    pub fn with_package_dependency(self, dependency: AssetPackageDependency) -> Self;
    pub fn with_package_dependencies(self, dependencies: Vec<AssetPackageDependency>) -> Self;
}

pub struct AssetPackageInstallReport {
    pub record: AssetPackageRecord,
    pub replaced: Option<AssetPackageRecord>,
    pub artifact_path: std::path::PathBuf,
    pub payload_size: u64,
    pub payload_hash: ContentHash,
    pub conflicts: AssetPackageConflictReport,
}

pub struct AssetPackageRemoveReport {
    pub removed: AssetPackageRecord,
    pub artifact_path: std::path::PathBuf,
    pub artifact_removed: bool,
    pub conflicts: AssetPackageConflictReport,
}

pub struct AssetPackageArtifactStatus {
    pub package: String,
    pub bundle_path: String,
    pub artifact_path: std::path::PathBuf,
    pub exists: bool,
    pub payload_size: Option<u64>,
    pub payload_hash: Option<ContentHash>,
    pub manifest_matches: Option<bool>,
    pub message: Option<String>,
}

impl AssetPackageArtifactStatus {
    pub fn is_available(&self) -> bool;
}

pub struct AssetPackageArtifactReport {
    pub root: std::path::PathBuf,
    pub packages: Vec<AssetPackageArtifactStatus>,
}

impl AssetPackageArtifactReport {
    pub fn all_available(&self) -> bool;
    pub fn require_available(&self) -> AssetResult<()>;
}

pub struct AssetPackageArtifactStore {
    // package artifact root
}

impl AssetPackageArtifactStore {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self;
    pub fn root(&self) -> &std::path::Path;
    pub fn artifact_path_for_record(
        &self,
        record: &AssetPackageRecord,
    ) -> AssetResult<std::path::PathBuf>;
    pub fn artifact_path(&self, bundle_path: &str) -> AssetResult<std::path::PathBuf>;
    pub fn install_package_bytes(
        &self,
        registry: &mut AssetPackageRegistry,
        request: AssetPackageInstallRequest,
        bytes: &[u8],
    ) -> AssetResult<AssetPackageInstallReport>;
    pub fn remove_package(
        &self,
        registry: &mut AssetPackageRegistry,
        name: &str,
        delete_artifact: bool,
    ) -> AssetResult<AssetPackageRemoveReport>;
    pub fn load_package_bytes(&self, record: &AssetPackageRecord) -> AssetResult<Vec<u8>>;
    pub fn verify_registry(
        &self,
        registry: &AssetPackageRegistry,
    ) -> AssetResult<AssetPackageArtifactReport>;
    pub fn build_composite_io(
        &self,
        registry: &AssetPackageRegistry,
    ) -> AssetResult<CompositeAssetIo>;
}

pub struct AssetPackageRegistry {
    // sorted package records
}

impl AssetPackageRegistry {
    pub fn new(packages: Vec<AssetPackageRecord>) -> AssetResult<Self>;
    pub fn empty() -> Self;
    pub fn packages(&self) -> &[AssetPackageRecord];
    pub fn enabled_packages(&self) -> impl Iterator<Item = &AssetPackageRecord>;
    pub fn into_packages(self) -> Vec<AssetPackageRecord>;
    pub fn validate(&self) -> AssetResult<()>;
    pub fn conflict_report(&self) -> AssetPackageConflictReport;
    pub fn asset_override_report(&self) -> AssetPackageAssetOverrideReport;
    pub fn update_report(
        &self,
        next: &AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
    ) -> AssetResult<AssetPackageUpdateReport>;
    pub fn build_composite_io<F>(&self, load_bundle: F) -> AssetResult<CompositeAssetIo>
    where
        F: FnMut(&AssetPackageRecord) -> AssetResult<Vec<u8>>;
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> AssetResult<Self>;
    pub fn to_text(&self) -> String;
    pub fn from_text(text: &str) -> AssetResult<Self>;
}
```

`AssetPackageRegistry::new` 会按 `priority` 排序，并在读取或 restore 前校验空名称、
空 bundle path、版本字段必须大于 0、package dependency 名称/版本范围、自依赖、
重复 package dependency、重复名称、重复 bundle id、重复 priority、重复 manifest path、
非法 layer kind 和截断 manifest。`from_text` 支持 V1/V2/V3 registry，并把旧版本缺失的
package version/minimum runtime version/package dependencies 迁移为 `1`/empty；
`to_text` 写出 V3。
`conflict_report` 在所有 enabled package 上按优先级计算 first-layer-wins 结果，
并列出被覆盖的 lower-priority package。
`asset_override_report` 在相同 path 被多个 enabled package 提供时，记录 winner/shadowed
package、winner/shadowed asset id/type/content hash/dependencies、dependency provider
package，以及 asset id/type/hash/dependency/provider 是否发生语义变化。
`build_composite_io` 会先校验 registry，再用调用方提供的 bundle bytes 构造
`BundleAssetIo`，并检查 payload manifest 与 registry manifest 一致。
`update_report` 比较当前 registry 和下一个 registry，报告新增/移除/更新/启用/禁用、
冲突、runtime version 不满足 package `minimum_runtime_version`、以及默认禁止的
package version downgrade。对 enabled package，`update_report` 还会检查
`package_dependencies`：依赖 package 必须启用，provider package version 必须满足
`min_version`/`max_version`，否则返回 missing/too-old/too-new compatibility issue。
`AssetPackageUpdatePolicy::asset_compatibility` 控制 asset override 是否只是诊断还是激活阻断：
默认要求相同 path 的 override 保持 asset type 一致，允许 asset id/content hash/dependency
变化但会在 `asset_overrides` 中报告；`AssetPackageAssetCompatibilityPolicy::strict()` 会要求
asset id、type、content hash、dependencies 和 dependency provider 都一致，不满足时
`activate_asset_package_registry` 会在修改 runtime registry 前返回 incompatibility error。
`AssetPackageArtifactStore` 管理实际 bundle payload 文件：install 会解析 bundle manifest、
写入 `bundle_path` 对应的 artifact、替换同名或同 bundle id 的 registry record，并返回
payload hash/size 和冲突报告；remove 会从 registry 删除 package，并可选择删除 artifact
文件；verify/build 会在读取前检查 enabled package 的 artifact 是否存在、是否可解析为 bundle、
以及 payload manifest 是否仍匹配 registry metadata。

---

## 12. Loader 系统

Loader 负责把运行时可读文件转换为 `Asset`。

```text
Cooked File / Source Runtime File
  │
  ▼
AssetLoader
  │
  ├── register dependency
  ├── emit labeled sub-assets
  ├── construct CPU asset
  └── queue GPU upload if needed
```

## 12.1 `AssetLoader`

```rust
pub trait AssetLoader: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn extensions(&self) -> &[&'static str];

    fn asset_type(&self) -> AssetTypeId;

    fn load(
        &self,
        ctx: &mut LoadContext,
        bytes: &[u8],
        settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError>;
}
```

---

## 12.2 `LoaderSettings`

```rust
#[derive(Clone, Debug, Default)]
pub struct LoaderSettings {
    // string key/value settings
}

impl LoaderSettings {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>);
    pub fn get(&self, key: &str) -> Option<&str>;
}
```

---

## 12.3 `LoadContext`

```rust
pub struct LoadContext<'a> {
    // private
}

impl<'a> LoadContext<'a> {
    pub fn id(&self) -> AssetId;
    pub fn path(&self) -> &AssetPath;

    pub fn dependency<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T>;

    pub fn add_dependency(&mut self, path: AssetPath, asset_type: AssetTypeId) -> AssetId;

    pub fn add_subresource(&mut self, path: AssetPath, asset_type: AssetTypeId) -> AssetId;
}
```

---

## 12.4 `LoadedAsset`

```rust
pub struct LoadedAsset {
    pub asset_type: AssetTypeId,
    pub asset: Box<dyn std::any::Any + Send + Sync>,
    pub gpu_upload: Option<GpuUploadCommand>,
    pub asset_dependencies: Vec<AssetDependencyReference>,
}

impl LoadedAsset {
    pub fn new<T: Asset>(asset: T) -> Self;
    pub fn new_with_asset_dependencies<T: Asset + AssetDependencies>(asset: T) -> Self;
    pub fn with_gpu_upload(self, upload: GpuUploadCommand) -> Self;
    pub fn with_dependency_handles(self, dependencies: Vec<UntypedHandle>) -> Self;
    pub fn with_dependency_refs(self, dependencies: Vec<AssetDependencyReference>) -> Self;
}
```

---

## 12.5 Loader 注册表

```rust
pub struct AssetLoaderRegistry {
    // private
}

impl AssetLoaderRegistry {
    pub fn new() -> Self;

    pub fn register<L: AssetLoader>(&mut self, loader: L);
    pub fn register_boxed(&mut self, loader: Box<dyn AssetLoader>);

    pub fn loader_for_extension(&self, extension: &str) -> Option<std::sync::Arc<dyn AssetLoader>>;
    pub fn loader_for_type(&self, ty: AssetTypeId) -> Option<std::sync::Arc<dyn AssetLoader>>;

    pub fn loader_for_path_and_type(
        &self,
        path: Option<&AssetPath>,
        asset_type: AssetTypeId,
    ) -> Result<std::sync::Arc<dyn AssetLoader>, AssetError>;

    pub fn asset_type_for_extension(&self, extension: &str) -> Option<AssetTypeId>;
}
```

`LoadContext` 记录 loader 声明的依赖和子资源路径；`LoadedAsset::new_with_asset_dependencies`
会通过 `AssetDependencies` 从 CPU asset 本体收集 `AssetDependencyReference` 依赖，`AssetRef<T>`
可通过 `visit_dependency` 直接贡献 id/type/fallback path，避免自定义 asset 手动构造 untyped handles。
`with_dependency_handles` 仍可直接附加已经解析好的 dependency handles，`with_dependency_refs`
可附加带 fallback path 的依赖。`AssetServer` 会把 loader-context 依赖和 asset-provided dependency
references 合并写入运行时依赖图、metadata，并递归排队加载已知 metadata/path/fallback path 的依赖。
asset-provided dependency 如果没有 live state，也没有 registry metadata/path/fallback path 可供递归加载，
会把该 dependency id 标记为 `AssetNotFound`，等待中的父资源随后收到 `DependencyFailed`，不会无限期
停在 `WaitingForDependencies`。这样自定义 CPU-only asset 可以通过 `AssetDependencies` 暴露持久化
reference 依赖，而不必在 loader 中重复调用 `LoadContext::dependency`。若 loader 返回的
`LoadedAsset::asset_type` 与请求类型不一致，加载会失败并记录 `AssetError::TypeMismatch`。

---

## 13. Importer 系统

Importer 只在编辑器和构建期运行。它处理源文件格式、导入设置、依赖分析和 cooked 文件生成。

```text
Source Asset
  │
  ▼
AssetImporter
  │
  ├── parse source file
  ├── generate AssetId
  ├── emit imported sub-assets
  ├── write metadata
  └── request cooking
```

## 13.1 `AssetImporter`

```rust
pub trait AssetImporter: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn version(&self) -> u32;

    fn extensions(&self) -> &[&'static str];

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError>;
}
```

---

## 13.2 `SourceAsset`

```rust
#[derive(Clone, Debug)]
pub struct SourceAsset {
    pub path: AssetPath,
    pub bytes: Vec<u8>,
    pub hash: ContentHash,
}
```

---

## 13.3 `ImporterSettings`

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImporterSettings {
    // string key/value settings
}

impl ImporterSettings {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>);
    pub fn get(&self, key: &str) -> Option<&str>;
    pub fn describe(&self) -> String;
    pub fn to_sorted_pairs(&self) -> Vec<(String, String)>;
}
```

---

## 13.4 `ImportContext`

```rust
pub struct ImportContext {
    // private generated assets, known registry assets, and dependencies
}

impl ImportContext {
    pub fn with_registry(registry: &AssetRegistry) -> Self;

    pub fn add_known_asset(
        &mut self,
        path: AssetPath,
        id: AssetId,
        asset_type: AssetTypeId,
    );

    pub fn add_generated_asset(&mut self, asset: ImportGeneratedAsset);

    pub fn add_dependency(&mut self, id: AssetId);

    pub fn dependency<T: Asset>(&mut self, path: impl Into<AssetPath>)
        -> Result<AssetId, ImportError>;

    pub fn add_dependency_by_path(
        &mut self,
        path: AssetPath,
        asset_type: AssetTypeId,
    ) -> Result<AssetId, ImportError>;

    pub fn finish(self) -> (Vec<ImportGeneratedAsset>, Vec<AssetId>);
}
```

`AssetDatabase` 会用当前 registry 创建 `ImportContext::with_registry`，因此内建或自定义
importer 可以把 source 中的路径引用解析为稳定 `AssetId`。路径不存在或类型不匹配会返回
可见 import error；直接已知 id 仍可通过 `add_dependency` 添加。

---

## 13.5 `ImportOutput`

```rust
#[derive(Clone, Debug)]
pub struct ImportOutput {
    pub metadata: AssetMetadata,
    pub generated: Vec<ImportGeneratedAsset>,
    pub dependencies: Vec<AssetId>,
    pub version_hash: VersionHash,
}
```

```rust
#[derive(Clone, Debug)]
pub struct ImportGeneratedAsset {
    pub id: AssetId,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,
    pub bytes: Vec<u8>,
    pub labels: Vec<String>,
    pub dependencies: Vec<AssetId>,
}
```

`AssetDatabase::import_asset_path_with_settings` 会把 `ImportOutput::dependencies`
合并进主资源 metadata，并为 `generated` 中的资源写入 registry metadata。生成资源会
记录 source path、cooked path、labels、dependencies、importer 名称/版本、source hash 和 version hash；
如果 registry 中已经存在相同 generated path，会复用旧 `AssetId`。
生成资源的 bytes 会写入 imported 目录，后续 `cook_asset` 可以直接烘焙这些 generated
outputs，而不需要手动补文件。
Importer 返回的 `AssetError::Import` 会由 `AssetDatabase` 补充 importer 名称、源路径和
`ImporterSettings::describe()` 结果，便于编辑器直接展示失败上下文。
`AssetDatabase` 会把 `ImporterSettings::to_sorted_pairs()` 写入 metadata，registry 和
sidecar 重新加载后仍可恢复同一组稳定排序的导入设置。
当前 `MaterialImporter` 会按运行时 `MaterialLoader` 使用的文本格式解析 `shader=...` 和
`texture.<name>=...` 行，并把这些引用写入 `ImportOutput::dependencies`。

---

## 14. Asset Metadata

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,

    pub importer: String,
    pub importer_version: u32,

    pub source_hash: ContentHash,
    pub settings_hash: ContentHash,
    pub importer_settings: Vec<(String, String)>,
    pub cooked_hash: Option<ContentHash>,

    pub dependencies: Vec<AssetDependency>,
    pub labels: Vec<String>,

    pub bundle: Option<BundleId>,
    pub address: Option<String>,

    pub created_at: Option<u64>,
    pub modified_at: Option<u64>,
}
```

---

## 14.1 依赖描述

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AssetDependency {
    pub id: AssetId,
    pub path: Option<AssetPath>,
    pub kind: DependencyKind,
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DependencyKind {
    /// 必须加载。依赖失败则 root asset 失败。
    Required,

    /// 可选依赖。失败不会阻止 root asset。
    Optional,

    /// 只在运行时需要。
    RuntimeOnly,

    /// 只在编辑器需要。
    EditorOnly,

    /// 用于打包分析，但不自动加载。
    BuildOnly,
}
```

---

## 14.2 `.meta` 文件示例

```ron
(
    id: "1b32d520-3ab1-4f80-93d3-7b6a0e6fa9a1",
    path: (path: "textures/hero_albedo.png", label: None),
    asset_type: "Texture",
    importer: "TextureImporter",
    importer_version: 3,
    source_hash: 1823948123,
    settings_hash: 99312312,
    importer_settings: [("mipmaps", "true"), ("quality", "high")],
    cooked_hash: Some(7712312),
    dependencies: [],
    labels: ["hero", "character"],
    bundle: Some("base.bundle"),
    address: Some("hero/albedo"),
)
```

---

## 15. Asset Registry

`AssetRegistry` 是资源索引库。

```rust
pub struct AssetRegistry {
    // private
}
```

内部可以包含：

```rust
pub struct AssetRegistryData {
    pub by_id: std::collections::HashMap<AssetId, AssetMetadata>,
    pub by_path: std::collections::HashMap<AssetPath, AssetId>,
    pub by_label: std::collections::HashMap<String, Vec<AssetId>>,
    pub by_address: std::collections::HashMap<String, AssetId>,
    pub redirects: std::collections::HashMap<AssetId, AssetId>,
}
```

API：

```rust
impl AssetRegistry {
    pub fn new() -> Self;

    pub fn register(&mut self, metadata: AssetMetadata) -> Option<AssetMetadata>;
    pub fn unregister(&mut self, id: AssetId) -> Option<AssetMetadata>;

    pub fn get_by_id(&self, id: AssetId) -> Option<&AssetMetadata>;
    pub fn get_by_path(&self, path: &AssetPath) -> Option<&AssetMetadata>;
    pub fn get_by_address(&self, address: &str) -> Option<&AssetMetadata>;

    pub fn id_from_path(&self, path: &AssetPath) -> Option<AssetId>;
    pub fn path_from_id(&self, id: AssetId) -> Option<&AssetPath>;

    pub fn find_by_label(&self, label: &str) -> &[AssetId];
    pub fn find_by_type(&self, ty: AssetTypeId) -> Vec<AssetId>;

    pub fn dependencies(&self, id: AssetId) -> &[AssetDependency];

    pub fn add_redirect(&mut self, old: AssetId, new: AssetId);
    pub fn resolve_redirect(&self, id: AssetId) -> AssetId;

    pub fn iter(&self) -> impl Iterator<Item = &AssetMetadata>;

    pub fn load_from_file(path: &std::path::Path) -> Result<Self, AssetError>;
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), AssetError>;
}
```

---

## 16. 依赖图

```rust
pub struct DependencyGraph {
    // private
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DependencyGraphReport {
    pub assets: Vec<AssetId>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyScopeReport {
    pub root: AssetId,
    pub direct_dependencies: Vec<AssetId>,
    pub transitive_dependencies: Vec<AssetId>,
    pub direct_dependents: Vec<AssetId>,
    pub transitive_dependents: Vec<AssetId>,
    pub graph: DependencyGraphReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DependencyEdge {
    pub asset: AssetId,
    pub dependency: AssetId,
}
```

API：

```rust
impl DependencyGraph {
    pub fn new() -> Self;

    pub fn add_asset(&mut self, id: AssetId);

    pub fn set_dependencies(&mut self, id: AssetId, dependencies: Vec<AssetId>);

    pub fn add_dependency(&mut self, id: AssetId, dependency: AssetId);

    pub fn direct_dependencies(&self, id: AssetId) -> &[AssetId];

    pub fn reverse_dependencies(&self, id: AssetId) -> &[AssetId];

    pub fn direct_dependents(&self, id: AssetId) -> Vec<AssetId>;

    pub fn transitive_dependencies(&self, id: AssetId) -> Vec<AssetId>;

    pub fn transitive_dependents(&self, id: AssetId) -> Vec<AssetId>;

    pub fn topological_order(&self, root: AssetId) -> Result<Vec<AssetId>, AssetError>;

    pub fn has_cycle_from(&self, root: AssetId) -> bool;

    pub fn report(&self) -> DependencyGraphReport;

    pub fn scoped_report(&self, root: AssetId) -> AssetResult<DependencyScopeReport>;
}
```

```rust
impl DependencyGraphReport {
    pub fn to_text(&self) -> String;
    pub fn to_dot(&self) -> String;
    pub fn to_json(&self) -> String;
    pub fn to_html(&self) -> String;
    pub fn to_html_with_labels(
        &self,
        labels: impl IntoIterator<Item = (AssetId, String)>,
    ) -> String;
    pub fn save_text(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn save_dot(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn save_json(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn save_html(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
}

impl DependencyScopeReport {
    pub fn to_text(&self) -> String;
    pub fn to_dot(&self) -> String;
    pub fn to_json(&self) -> String;
    pub fn to_html(&self) -> String;
    pub fn to_html_with_labels(
        &self,
        labels: impl IntoIterator<Item = (AssetId, String)>,
    ) -> String;
    pub fn save_text(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn save_dot(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn save_json(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn save_html(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
}
```

```rust
impl AssetServer {
    pub fn dependency_report(&self) -> DependencyGraphReport;
    pub fn scoped_dependency_report(&self, root: AssetId) -> AssetResult<DependencyScopeReport>;
    pub fn dependency_report_text(&self) -> String;
    pub fn dependency_report_dot(&self) -> String;
    pub fn dependency_report_json(&self) -> String;
    pub fn dependency_report_html(&self) -> String;
    pub fn scoped_dependency_report_text(&self, root: AssetId) -> AssetResult<String>;
    pub fn scoped_dependency_report_dot(&self, root: AssetId) -> AssetResult<String>;
    pub fn scoped_dependency_report_json(&self, root: AssetId) -> AssetResult<String>;
    pub fn scoped_dependency_report_html(&self, root: AssetId) -> AssetResult<String>;
    pub fn save_dependency_report_text(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_dependency_report_dot(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_dependency_report_json(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_dependency_report_html(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_text(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_dot(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_json(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_html(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
}

impl AssetDatabase {
    pub fn dependency_report(&self) -> DependencyGraphReport;
    pub fn scoped_dependency_report(&self, root: AssetId) -> AssetResult<DependencyScopeReport>;
    pub fn dependency_report_text(&self) -> String;
    pub fn dependency_report_dot(&self) -> String;
    pub fn dependency_report_json(&self) -> String;
    pub fn dependency_report_html(&self) -> String;
    pub fn scoped_dependency_report_text(&self, root: AssetId) -> AssetResult<String>;
    pub fn scoped_dependency_report_dot(&self, root: AssetId) -> AssetResult<String>;
    pub fn scoped_dependency_report_json(&self, root: AssetId) -> AssetResult<String>;
    pub fn scoped_dependency_report_html(&self, root: AssetId) -> AssetResult<String>;
    pub fn save_dependency_report_text(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_dependency_report_dot(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_dependency_report_json(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_dependency_report_html(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_text(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_dot(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_json(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
    pub fn save_scoped_dependency_report_html(
        &self,
        root: AssetId,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;
}
```

`to_text` 输出 `NGA_DEPENDENCY_GRAPH_V1` 行格式，适合稳定 diff；
`to_dot` 输出 `digraph AssetDependencies`，可供图形工具和编辑器审计面板消费；
`to_json` 输出稳定排序的 `{"version":1,"assets":[...],"edges":[...]}`，其中 id 用字符串
表示以避免 JSON number 精度问题。
`to_html` 输出自包含、稳定排序的 HTML 审计报告，包含 summary、asset table、edge table
和 adjacency 视图；`to_html_with_labels` 可传入 `AssetId -> String` 标签并会对标签做
HTML 转义。`AssetServer` 和 `AssetDatabase` 的 HTML helper 会用 registry metadata
补充 path/type 标签，适合编辑器 pane 或保存为审计附件。
`DependencyScopeReport` 针对单个 root 额外列出 direct/transitive dependencies 和
direct/transitive dependents，并导出由这些节点诱导出的 subgraph；root 不存在时返回
`AssetError::AssetNotFound`。

加载规则：

```text
load root asset
  │
  ▼
resolve metadata
  │
  ▼
compute dependencies
  │
  ▼
load dependencies first
  │
  ▼
when required dependencies ready, decode root asset
  │
  ▼
if GPU upload required, queue upload
  │
  ▼
mark Ready
```

---

## 17. Cooker 系统

Cooker 将 imported asset 转换为运行时高效格式。

```rust
pub trait AssetCooker: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn asset_type(&self) -> AssetTypeId;

    fn cook(
        &self,
        ctx: &mut CookContext,
        input: &ImportedAssetInfo,
        settings: &CookSettings,
    ) -> Result<CookedAssetInfo, CookError>;
}
```

---

## 17.1 Cook 配置

```rust
#[derive(Clone, Debug)]
pub struct CookSettings {
    pub target_platform: TargetPlatform,
    pub compression: CompressionKind,
    pub strip_editor_data: bool,
    pub generate_debug_names: bool,
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    Windows,
    Linux,
    MacOs,
    Android,
    Ios,
    Web,
    Console,
}
```

---

## 17.2 Cook 输出

```rust
#[derive(Clone, Debug)]
pub struct CookedAssetInfo {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub path: std::path::PathBuf,
    pub hash: ContentHash,
    pub size_bytes: u64,
    pub dependencies: Vec<AssetDependency>,
}
```

```rust
pub struct CookContext<'a> {
    pub target_platform: TargetPlatform,

    // private
    marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> CookContext<'a> {
    pub fn write_bytes(&mut self, relative_path: &str, bytes: &[u8]) -> Result<std::path::PathBuf, CookError>;

    pub fn add_dependency(&mut self, dependency: AssetId, kind: DependencyKind);
}
```

Cooker 返回的 `AssetError::Cook` 会由 `AssetDatabase::cook_asset` 补充 cooker 名称、
asset id、source path 和目标平台；未注册 cooker 的错误也会保留 asset id/path
上下文。

---

## 18. Bundle 系统

Bundle 是运行时资源包。

## 18.1 Bundle 基础类型

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BundleId(pub u64);
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleManifest {
    pub name: String,
    pub compression: CompressionKind,
    pub chunks: Vec<BundleChunk>,
    pub entries: Vec<BundleEntry>,
}

impl BundleManifest {
    pub fn entry(&self, id: AssetId) -> Option<&BundleEntry>;
    pub fn entry_by_path(&self, path: &AssetPath) -> Option<&BundleEntry>;
    pub fn dependencies(&self, id: AssetId) -> Option<&[AssetId]>;
    pub fn chunk(&self, index: u32) -> Option<&BundleChunk>;
    pub fn total_uncompressed_bytes(&self) -> u64;
}
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleEntry {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub path: Option<AssetPath>,
    pub chunk_index: u32,
    pub offset: u64,
    pub length: u64,
    pub content_hash: ContentHash,
    pub dependencies: Vec<AssetId>,
}
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleChunk {
    pub index: u32,
    pub offset: u64,
    pub compressed_length: u64,
    pub uncompressed_length: u64,
    pub compression: CompressionKind,
    pub content_hash: ContentHash,
}
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleAsset {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub path: AssetPath,
    pub bytes: Vec<u8>,
    pub dependencies: Vec<AssetId>,
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompressionKind {
    None,
    Rle,
    Zstd,
}
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleCompressionCodecReport {
    pub compression: CompressionKind,
    pub supported: bool,
    pub codec_name: &'static str,
    pub reason: Option<String>,
}

impl BundleCompressionCodecReport {
    pub fn for_compression(compression: CompressionKind) -> Self;
}
```

`CompressionKind::Rle` 是当前内建的轻量压缩 codec，会在 `BundleWriter` 中写入压缩
data section，并由 `BundleReader` 解码后按 entry offset 提供资源字节。`CompressionKind::Zstd`
由默认启用的 `zstd` feature 提供，当前通过纯 Rust `ruzstd` 后端写入/读取 Zstandard
frame；当只启用 `bundle` 而关闭 `zstd` 时，codec report 会标记 unsupported，并在 writer、
manifest 或 chunk 读取入口返回可见 `AssetError`。压缩 chunk 读取失败会指出 codec、
chunk index 或损坏的 RLE/Zstd payload。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BundleChunkPartitionPolicy {
    SingleChunk,
    MaxUncompressedBytes(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BundleChunkLoadingPolicy {
    Eager,
    OnDemandCached,
    OnDemandCachedLimited { max_decoded_chunks: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BundleChunkCacheStatus {
    Preloaded,
    Hit,
    Miss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleBuildOptions {
    pub compression: CompressionKind,
    pub chunk_policy: BundleChunkPartitionPolicy,
}

impl BundleBuildOptions {
    pub fn new(compression: CompressionKind) -> Self;
    pub fn with_chunk_policy(self, chunk_policy: BundleChunkPartitionPolicy) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleChunkCacheStats {
    pub policy: BundleChunkLoadingPolicy,
    pub chunks_total: usize,
    pub max_decoded_chunks: Option<usize>,
    pub decoded_chunks: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub prefetched_chunks: u64,
    pub decoded_bytes: u64,
}

pub struct BundleChunkPrefetchReport {
    pub requested_chunks: Vec<u32>,
    pub decoded_chunks: Vec<u32>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evicted_chunks: Vec<u32>,
}
```

`BundleChunkPartitionPolicy::MaxUncompressedBytes` 会按 asset 顺序确定性地把 bundle entry
分配到多个 chunk；单个资源大于阈值时会独占一个超限 chunk，而不会拆分资源字节。
`BundleChunkLoadingPolicy::Eager` 是默认验证路径，会在读取 bundle 时解码所有 chunk；
`OnDemandCached` 会在首次读取 entry/range 时解码对应 chunk 并缓存，后续读取通过
cache hit 报告。`OnDemandCachedLimited { max_decoded_chunks }` 在同样的 lazy decode
路径上增加 LRU 上限，超过上限时淘汰最久未使用 chunk；`max_decoded_chunks = 0`
会产生可见 `AssetError::Bundle`。`BundleChunkCacheStats` 记录当前 decoded chunk 数、
cache hit/miss、eviction 和 prefetch 计数。

```rust
pub struct BundleBuilder {
    // private
}

impl BundleBuilder {
    pub fn new(name: impl Into<String>, compression: CompressionKind) -> Self;
    pub fn add_asset(&mut self, asset: BundleAsset);
    pub fn add_chunk(&mut self, chunk: BundleChunk);
    pub fn add_entry(&mut self, entry: BundleEntry);
    pub fn set_chunk_policy(&mut self, chunk_policy: BundleChunkPartitionPolicy);
    pub fn build(self) -> AssetResult<BundleManifest>;
    pub fn build_bytes(self) -> AssetResult<Vec<u8>>;
}

pub struct BundleWriter;

impl BundleWriter {
    pub fn write_file(
        path: impl AsRef<std::path::Path>,
        name: impl Into<String>,
        compression: CompressionKind,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<BundleManifest>;

    pub fn build_bytes(
        name: impl Into<String>,
        compression: CompressionKind,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<Vec<u8>>;

    pub fn build_bytes_with_options(
        name: impl Into<String>,
        options: BundleBuildOptions,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<Vec<u8>>;
}
```

`BundleWriter` 当前输出 `NGA_BUNDLE_V2` manifest：全局 `compression` 是默认包级策略，
`chunks` 描述 data section 内的压缩 chunk 布局，`BundleEntry::chunk_index + offset + length`
定位解压后 chunk 内的资源字节。`CompressionKind::None`、`CompressionKind::Rle` 和默认启用的
`CompressionKind::Zstd` 都有真实读写执行路径；`BundleReader` 会保留 manifest 中的
compressed/uncompressed chunk 元数据，
并在内部保存解压后的 chunk payload 供 entry/range 读取。旧 `NGA_BUNDLE_V1` manifest 仍会被映射成
单个 chunk。

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleReader {
    // private
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleChunkReadReport {
    pub entry: AssetId,
    pub path: Option<AssetPath>,
    pub chunk_index: u32,
    pub chunk_compression: CompressionKind,
    pub chunk_compressed_length: u64,
    pub chunk_uncompressed_length: u64,
    pub entry_offset: u64,
    pub entry_length: u64,
    pub range_offset: u64,
    pub range_length: u64,
    pub bytes_returned: u64,
    pub cache_status: BundleChunkCacheStatus,
}

impl BundleReader {
    pub fn from_bytes(bytes: &[u8]) -> AssetResult<Self>;
    pub fn from_bytes_with_loading_policy(
        bytes: &[u8],
        chunk_loading_policy: BundleChunkLoadingPolicy,
    ) -> AssetResult<Self>;
    pub fn manifest(&self) -> &BundleManifest;
    pub fn chunk_loading_policy(&self) -> BundleChunkLoadingPolicy;
    pub fn chunk_cache_stats(&self) -> BundleChunkCacheStats;
    pub fn prefetch_chunk(&self, chunk_index: u32) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn prefetch_chunks(&self, chunk_indices: &[u32]) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn prefetch_path(&self, path: &AssetPath) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn prefetch_paths(&self, paths: &[AssetPath]) -> AssetResult<BundleChunkPrefetchReport>;
    pub fn read_entry(&self, id: AssetId) -> AssetResult<Vec<u8>>;
    pub fn read_entry_range(
        &self,
        id: AssetId,
        offset: u64,
        length: u64,
    ) -> AssetResult<Vec<u8>>;
    pub fn read_path(&self, path: &AssetPath) -> AssetResult<Vec<u8>>;
    pub fn read_path_range(
        &self,
        path: &AssetPath,
        offset: u64,
        length: u64,
    ) -> AssetResult<Vec<u8>>;
    pub fn read_path_with_report(
        &self,
        path: &AssetPath,
    ) -> AssetResult<(Vec<u8>, BundleChunkReadReport)>;
    pub fn read_path_range_with_report(
        &self,
        path: &AssetPath,
        offset: u64,
        length: u64,
    ) -> AssetResult<(Vec<u8>, BundleChunkReadReport)>;
}
```

---

## 18.2 Bundle Runtime API

```rust
pub struct LoadedBundle {
    pub manifest: BundleManifest,
    // private
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountedBundle {
    pub id: BundleId,
    pub name: String,
    pub manifest: BundleManifest,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MountedBundleRegistry {
    // sorted mounted bundle snapshot
}

impl MountedBundleRegistry {
    pub fn new(bundles: Vec<MountedBundle>) -> Self;
    pub fn from_mounted_bundles<'a>(
        bundles: impl IntoIterator<Item = &'a MountedBundle>,
    ) -> Self;
    pub fn bundles(&self) -> &[MountedBundle];
    pub fn into_bundles(self) -> Vec<MountedBundle>;
    pub fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> AssetResult<()>;
    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> AssetResult<Self>;
    pub fn to_text(&self) -> String;
    pub fn from_text(text: &str) -> AssetResult<Self>;
}
```

```rust
impl AssetServer {
    pub fn mount_bundle_bytes(&mut self, bytes: &[u8]) -> AssetResult<MountedBundle>;

    pub fn mount_bundle_manifest(&mut self, manifest: BundleManifest) -> MountedBundle;

    pub fn mounted_bundle(&self, id: BundleId) -> Option<&MountedBundle>;

    pub fn mounted_bundles(&self) -> impl Iterator<Item = &MountedBundle>;

    pub fn mounted_bundle_registry(&self) -> MountedBundleRegistry;

    pub fn restore_mounted_bundle_registry(
        &mut self,
        registry: MountedBundleRegistry,
    ) -> Vec<MountedBundle>;

    pub fn save_mounted_bundle_registry(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;

    pub fn load_mounted_bundle_registry(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<Vec<MountedBundle>>;

    pub fn asset_package_registry(&self) -> &AssetPackageRegistry;

    pub fn asset_package_conflict_report(&self) -> AssetPackageConflictReport;

    pub fn preview_asset_package_update(
        &self,
        registry: &AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
    ) -> AssetResult<AssetPackageUpdateReport>;

    pub fn activate_asset_package_registry(
        &mut self,
        registry: AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
    ) -> AssetResult<AssetPackageActivation>;

    pub fn verify_asset_package_artifacts(
        &self,
        registry: &AssetPackageRegistry,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> AssetResult<AssetPackageArtifactReport>;

    pub fn activate_asset_package_registry_from_artifacts(
        &mut self,
        registry: AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
        artifact_root: impl AsRef<std::path::Path>,
    ) -> AssetResult<AssetPackageActivation>;

    pub fn restore_asset_package_registry(
        &mut self,
        registry: AssetPackageRegistry,
    ) -> AssetResult<Vec<MountedBundle>>;

    pub fn save_asset_package_registry(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<()>;

    pub fn load_asset_package_registry(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> AssetResult<Vec<MountedBundle>>;

    pub fn unmount_bundle(&mut self, id: BundleId) -> AssetResult<MountedBundle>;

    pub fn preload_bundle(&mut self, bundle: &MountedBundle) -> AssetLoadGroup;
}
```

`preload_bundle` 会把 manifest entry 写入运行时 `AssetRegistry`，保留
`cooked_hash` 和 manifest 中记录的依赖，再按 entry 的 `AssetId`/`AssetTypeId`
创建加载组。
运行时加载资源时，`AssetServer` 会把 loader/context/asset-provided dependencies 与 registry/bundle
metadata 中可解析到路径或已有状态的 dependency ids 合并到 dependency graph；只存在于 manifest
metadata、但当前运行时没有路径或状态的外部依赖会保留在 metadata 中用于审计和打包，不会把本次加载永久阻塞。

`MountedBundleRegistry` 只持久化已挂载 bundle 的 manifest 元数据，不持久化 payload
字节本身；运行时仍需要通过 `BundleAssetIo`、文件系统 IO 或组合 IO 提供实际资源字节。
`AssetPackageRegistry` 是 package/update 层的持久化入口：它额外记录启用状态、
priority、layer kind、package version、minimum runtime version 和 bundle payload path。
`preview_asset_package_update` 在不变更运行时状态的情况下生成版本/兼容性/冲突报告；
`activate_asset_package_registry` 会先执行相同检查，`require_compatible` 失败时不会修改
mounted bundle 或已加载资源，restore 过程中若发生错误也会回滚 registry/mounted
manifest 状态。`activate_asset_package_registry_from_artifacts` 会额外验证 enabled
package 的 artifact 文件存在且 manifest 与 registry metadata 匹配，失败时同样不会修改
runtime mounted package registry。`restore_asset_package_registry` 是低层直接恢复入口，只替换 package
registry 管理的 mounted bundle manifest，不卸载已加载资源，因此已有 `Ready` asset
状态和 handle 仍保持稳定；后续可对返回的 `MountedBundle` 调用 `preload_bundle`
来登记 manifest metadata 并触发实际加载。

---

## 18.3 Bundle Builder

```rust
pub struct BundleBuilder {
    // private
}

impl BundleBuilder {
    pub fn new(name: impl Into<String>, compression: CompressionKind) -> Self;
    pub fn add_entry(&mut self, entry: BundleEntry);
    pub fn add_asset(&mut self, asset: BundleAsset);
    pub fn build(self) -> AssetResult<BundleManifest>;
    pub fn build_bytes(self) -> AssetResult<Vec<u8>>;
}
```

`AssetDatabaseBundleBuild` 是当前数据库侧的高层构建入口；它从 cooked metadata 和
cooked payload 生成同样的 bundle 字节，见 23.2。

---

## 19. 热重载

## 19.1 Hot Reload 配置

```rust
pub struct AssetServerConfig {
    pub enable_hot_reload: bool,
    pub hot_reload_dependency_policy: HotReloadDependencyPolicy,
    // ...
}
```

---

## 19.2 API

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotReloadDependencyPolicy {
    Direct,
    Transitive,
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
    pub fn has_dependents(&self) -> bool;
    pub fn reload_order(&self) -> Vec<AssetId>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotReloadRollbackRetention {
    None,
    Cpu,
    CpuAndGpu,
}

impl HotReloadRollbackRetention {
    pub fn retains_cpu(self) -> bool;
    pub fn retains_gpu(self) -> bool;
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
    pub fn can_retain_previous_ready_state(&self) -> bool;
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
    pub fn watched_paths(&self) -> usize;
    pub fn queued_changes(&self) -> usize;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotReloadWatchBackend {
    PollingMetadata,
    AsyncNotification,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HotReloadAsyncWatchLifecycle {
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
    pub fn is_running(&self) -> bool;
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
```

```rust
impl AssetServer {
    pub fn reload_by_path(&mut self, path: &AssetPath) -> AssetResult<()>;

    pub fn reload_by_id(&mut self, id: AssetId) -> AssetResult<()>;

    pub fn update_hot_reload(&mut self);

    pub fn queue_hot_reload_path(&mut self, path: impl Into<AssetPath>);

    pub fn try_queue_hot_reload_path(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> AssetResult<()>;

    pub fn queue_hot_reload_id(&mut self, id: AssetId) -> AssetResult<()>;

    pub fn queue_hot_reload_change(&mut self, change: HotReloadChange);

    pub fn hot_reload_dependency_plan_by_id(
        &self,
        id: AssetId,
        policy: HotReloadDependencyPolicy,
    ) -> AssetResult<HotReloadDependencyPlan>;

    pub fn hot_reload_dependency_plan_by_path(
        &self,
        path: &AssetPath,
        policy: HotReloadDependencyPolicy,
    ) -> AssetResult<HotReloadDependencyPlan>;

    pub fn hot_reload_rollback_policy_for_type(
        &self,
        asset_type: AssetTypeId,
    ) -> HotReloadRollbackPolicyReport;

    pub fn hot_reload_rollback_policies(&self) -> Vec<HotReloadRollbackPolicyReport>;

    pub fn hot_reload_rollback_report_by_id(
        &self,
        id: AssetId,
    ) -> AssetResult<HotReloadRollbackAssetReport>;

    pub fn set_hot_reload_rollback_override(
        &mut self,
        asset_type: AssetTypeId,
        retention: HotReloadRollbackRetention,
    );

    pub fn clear_hot_reload_rollback_override(
        &mut self,
        asset_type: AssetTypeId,
    ) -> Option<HotReloadRollbackRetention>;

    pub fn hot_reload_policy_report(&self) -> HotReloadPolicyReport;

    pub fn watch_hot_reload_path(&mut self, path: impl Into<AssetPath>) -> AssetResult<()>;

    pub fn watch_hot_reload_path_with_backend(
        &mut self,
        path: impl Into<AssetPath>,
        backend: HotReloadWatchBackend,
    ) -> AssetResult<()>;

    pub fn unwatch_hot_reload_path(&mut self, path: &AssetPath) -> bool;

    pub fn hot_reload_watch(&self, path: &AssetPath) -> Option<&HotReloadWatch>;

    pub fn hot_reload_watches(&self) -> impl Iterator<Item = &HotReloadWatch>;

    pub fn start_hot_reload_async_watch_backend(&mut self)
        -> AssetResult<HotReloadAsyncWatchReport>;

    pub fn stop_hot_reload_async_watch_backend(&mut self)
        -> AssetResult<HotReloadAsyncWatchReport>;

    pub fn notify_hot_reload_async_watch_change(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> AssetResult<bool>;

    pub fn hot_reload_async_watch_report(&self) -> HotReloadAsyncWatchReport;

    pub fn poll_hot_reload_watches(&mut self) -> AssetResult<HotReloadPollReport>;

    pub fn last_hot_reload_poll_report(&self) -> &HotReloadPollReport;
}
```

`watch_hot_reload_path` 默认注册 `PollingMetadata` watch；`poll_hot_reload_watches` 会比较这些
watch 的 `AssetIo::metadata` 快照。Memory IO 使用内容 hash，FileSystem IO 使用文件大小和修改时间；
检测到变化后会把路径加入热重载队列。如果同一路径已经在队列中，本次轮询会计入
`debounced_changes`，避免重复排队。
`AsyncNotification` watch 是 host/editor 驱动的异步通知入口：先注册 watch 并启动
`start_hot_reload_async_watch_backend`，再由外部 watcher 或编辑器桥接调用
`notify_hot_reload_async_watch_change`。后续 `poll_hot_reload_watches` 只处理已收到的 async
notification，不会像 `PollingMetadata` 一样扫描所有 async watch；处理 notification 时仍会读取
metadata 来刷新状态并把 metadata 错误写入 poll/report。`stop_hot_reload_async_watch_backend` 会停止
接收并丢弃尚未处理的 notification，丢弃数量通过 report 可见。
`update_hot_reload` 会自动轮询已注册 watch，再处理队列。
`hot_reload_dependency_plan_by_id` 和 `hot_reload_dependency_plan_by_path` 是只读规划入口；
`Direct` 返回当前执行路径会重载的直接反向依赖，`Transitive` 返回稳定顺序的全部上层依赖，
用于工具在修改运行时状态前展示影响范围。
`AssetServerConfig::hot_reload_dependency_policy` 默认为 `Direct`，因此现有热重载执行保持只
重载直接上层依赖；显式设为 `Transitive` 后，队列处理会使用同一规划结果重载全部上层依赖。
`hot_reload_rollback_policy_for_type` 和 `hot_reload_rollback_report_by_id` 暴露失败回滚能力：
所有已注册 storage 的资源类型都能在前一状态为 `Ready` 时保留旧 CPU 对象；`Texture`、
`Mesh`、`Shader` 和 `Material` 还会报告 `CpuAndGpu`，表示旧 GPU handle 也由旧 asset 对象保留。
自定义资源类型默认报告 `Cpu`，因此失败回滚语义是显式可见的，而不是隐藏在 storage 实现里。
`set_hot_reload_rollback_override` 可为某个 `AssetTypeId` 覆盖 retention；设为 `None`
会让失败 reload 保持 `Failed` 状态而不是恢复旧 `Ready`，并在 policy report 中标记
`overridden=true`。
`hot_reload_policy_report` 是 editor/debugger 面向的只读快照，汇总当前依赖执行策略、
rollback policy 列表、watch 列表、watch backend/status、待处理 reload 队列和最近一次
poll report。`watch_statuses` 会标明每个 path 使用 `PollingMetadata` 还是 `AsyncNotification`、
是否已在队列中，以及最近一次 poll 是否对该 path 产生了 metadata error；`async_watch` 汇总
异步通知 backend 的 running/stopped 状态、watched path 数、pending/received/delivered/dropped
notification 计数和最近的 async notification metadata 错误。

---

## 19.3 热重载行为规则

```text
1. Handle<T> 不变。
2. AssetId 不变。
3. AssetStorage<T> 中的资源内容被替换。
4. 依赖该资源的上层资源可选择自动重载。
5. 正在使用的 GPU 资源延迟销毁，避免渲染线程读写冲突。
```

---

## 20. GPU 上传队列

资源加载不应该直接操作 Renderer。资源系统只提交上传命令，渲染系统在合适阶段执行。

## 20.1 GPU 句柄

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuResourceHandle(pub u64);
```

---

## 20.2 上传命令

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuUploadKind {
    Texture,
    Mesh,
    Material,
    Shader,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuUploadCommand {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub kind: GpuUploadKind,
    pub label: Option<String>,
    pub metadata: GpuUploadMetadata,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuUploadMetadata {
    None,
    Mesh(MeshUploadMetadata),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshUploadMetadata {
    pub layout: MeshVertexLayout,
    pub vertex_buffer_bytes: u32,
    pub index_buffer_bytes: u32,
    pub index_count: u32,
    pub index_format: MeshIndexFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshVertexLayout {
    pub vertex_count: u32,
    pub stride: u32,
    pub attributes: Vec<MeshVertexAttribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshVertexAttribute {
    pub semantic: MeshVertexSemantic,
    pub format: MeshVertexFormat,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshVertexSemantic {
    Position,
    Normal,
    TexCoord(u8),
    Tangent,
    Joints,
    Weights,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshVertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
    Uint16x4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshIndexFormat {
    Uint16,
    Uint32,
}
```

Mesh uploads use `GpuUploadMetadata::Mesh`; `bytes` contains the packed interleaved vertex buffer
followed by little-endian index bytes in `MeshUploadMetadata::index_format`
(`Uint16` or `Uint32`). `MeshUploadMetadata::vertex_buffer_bytes` and `index_buffer_bytes`
define the split point, while `layout.attributes` exposes renderer-facing semantic/format/offset
metadata.

---

## 20.3 上传结果

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuUploadResult {
    pub id: AssetId,
    pub result: Result<GpuResourceHandle, String>,
}

impl GpuUploadResult {
    pub fn ok(id: AssetId, handle: GpuResourceHandle) -> Self;
    pub fn failed(id: AssetId, message: impl Into<String>) -> Self;
}
```

---

## 20.4 `GpuUploadQueue`

```rust
pub struct GpuUploadQueue {
    // private
}

impl GpuUploadQueue {
    pub fn new() -> Self;

    pub fn push(&mut self, command: GpuUploadCommand);

    pub fn drain(&mut self) -> impl Iterator<Item = GpuUploadCommand> + '_;

    pub fn submit_result(&mut self, result: GpuUploadResult);

    pub fn drain_results(&mut self) -> impl Iterator<Item = GpuUploadResult> + '_;

    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

```rust
impl AssetServer {
    pub fn drain_gpu_uploads(&mut self) -> impl Iterator<Item = GpuUploadCommand> + '_;

    pub fn finish_gpu_uploads(&mut self, results: impl IntoIterator<Item = GpuUploadResult>);
}
```

`drain_gpu_uploads` 每次最多返回 `AssetServerConfig::max_gpu_uploads_per_frame`
条命令，未取出的上传会留到下一次 drain。上传失败会把初次加载资源置为
`Failed` 并记录 `AssetError::GpuUpload`；热重载上传失败会回滚到旧的 ready 资源。

---

## 21. GC 与内存预算

## 21.1 配置

```rust
#[derive(Clone, Debug)]
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

impl Default for AssetGcConfig {
    fn default() -> Self;
}

impl AssetTypeMemoryBudget {
    pub fn total(asset_type: AssetTypeId, bytes: u64) -> Self;
    pub fn cpu(asset_type: AssetTypeId, bytes: u64) -> Self;
    pub fn gpu(asset_type: AssetTypeId, bytes: u64) -> Self;
    pub fn cpu_gpu(asset_type: AssetTypeId, cpu_bytes: u64, gpu_bytes: u64) -> Self;
    pub fn has_budget(&self) -> bool;
}
```

`memory_budget_bytes` 是全局 CPU+GPU byte 预算；`type_memory_budgets` 会按
`AssetTypeId` 单独执行 total/CPU/GPU 预算。类型预算只会驱逐对应类型中最久未使用且
未受 strong handle、dependency ref 或 resident 保护的资源，不会为了某个类型超预算而驱逐其他类型。

---

## 21.2 内存信息

```rust
#[derive(Clone, Debug, Default)]
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
```

```rust
#[derive(Clone, Debug, Default)]
pub struct AssetMemoryReport {
    pub total_cpu_bytes: u64,
    pub total_gpu_bytes: u64,
    pub asset_count: usize,
    pub assets: Vec<AssetMemoryInfo>,
    pub by_type: Vec<AssetTypeMemoryReport>,
}

#[derive(Clone, Debug, Default)]
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
```

---

## 21.3 GC API

```rust
impl AssetServer {
    pub fn collect_unused(&mut self);

    pub fn collect_until_budget(&mut self);

    pub fn memory_info(&self, id: AssetId) -> Option<AssetMemoryInfo>;

    pub fn memory_report(&self) -> AssetMemoryReport;

    pub fn set_asset_resident(&mut self, id: AssetId, resident: bool);

    pub fn is_asset_resident(&self, id: AssetId) -> bool;
}
```

卸载条件：

```text
strong_count == 0
dependency_ref_count == 0
resident == false
state == Ready 或 Failed
last_used_frame 超过 unload_after_unused_frames
```

`strong_count` / `weak_count` 来自 `AssetServer` 创建的 handle lifecycle tracker，并会在
loading/update/GC 刷新时写回 storage entry。强引用保护 GC；弱引用只用于观测和
serialized/reference 场景，不阻止 unused unload。
`memory_info` / `memory_report` 直接读取当前 lifecycle tracker 和依赖计数，按 asset/type
返回 CPU/GPU byte、strong/weak/dependency counts、resident 状态和 `last_used_frame`；
`memory_stats` 是保留的紧凑总览。

---

## 22. Streaming

适用于开放世界、关卡分块、流式场景。

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StreamingRegionId(pub u64);

#[derive(Clone, Debug)]
pub struct StreamingRegion {
    pub id: StreamingRegionId,
    pub name: String,
    pub priority: LoadPriority,
    pub assets: Vec<UntypedHandle>,
    pub resident: bool,
}
```

```rust
impl AssetServer {
    pub fn register_streaming_region(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        assets: Vec<UntypedHandle>,
    ) -> StreamingRegionId;

    pub fn register_streaming_region_paths(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        paths: &[AssetPath],
    ) -> AssetResult<StreamingRegionId>;

    pub fn register_streaming_region_bundle(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        bundle: BundleId,
    ) -> AssetResult<StreamingRegionId>;

    pub fn register_streaming_region_bundle_subset(
        &mut self,
        name: impl Into<String>,
        priority: LoadPriority,
        bundle: BundleId,
        assets: &[AssetId],
    ) -> AssetResult<StreamingRegionId>;

    pub fn streaming_region(&self, id: StreamingRegionId) -> Option<&StreamingRegion>;

    pub fn remove_streaming_region(
        &mut self,
        id: StreamingRegionId,
    ) -> AssetResult<StreamingRegion>;

    pub fn set_streaming_region_resident(
        &mut self,
        id: StreamingRegionId,
        resident: bool,
    ) -> AssetResult<()>;

    pub fn set_streaming_region_priority(
        &mut self,
        id: StreamingRegionId,
        priority: LoadPriority,
    ) -> AssetResult<LoadPriority>;

    pub fn preload_streaming_region(
        &mut self,
        id: StreamingRegionId,
    ) -> AssetResult<AssetLoadGroup>;

    pub fn unload_streaming_region(&mut self, id: StreamingRegionId) -> AssetResult<usize>;

    pub fn streaming_region_progress(&self, id: StreamingRegionId) -> AssetResult<LoadProgress>;

    pub fn streaming_region_state(&self, id: StreamingRegionId) -> AssetResult<AssetLoadState>;
}
```

`register_streaming_region_bundle` 使用已挂载 bundle manifest 中的全部 entry 创建区域；
`register_streaming_region_bundle_subset` 只使用指定 `AssetId`，并在 id 不属于该
bundle manifest 时返回 `AssetError::AssetNotFound`。两者都会把 manifest 中的路径、
类型、`cooked_hash` 和依赖同步到运行时 registry，然后通过真实加载状态汇总进度。

推荐流程：

```text
玩家靠近区域
  │
  ▼
preload_streaming_region
  │
  ▼
资源 Ready
  │
  ▼
set_streaming_region_resident(true) 并实例化场景块
  │
  ▼
玩家远离
  │
  ▼
set_streaming_region_resident(false) 或 unload_streaming_region
  │
  ▼
GC 之后卸载资源
```

---

## 23. AssetDatabase 编辑器 API

`AssetDatabase` 是编辑器和构建期入口。

```rust
pub struct AssetDatabase {
    // private
}
```

## 23.1 配置

```rust
#[derive(Clone, Debug)]
pub struct AssetDatabaseConfig {
    pub source_root: std::path::PathBuf,
    pub imported_root: std::path::PathBuf,
    pub cooked_root: std::path::PathBuf,
    pub registry_path: std::path::PathBuf,
}
```

---

## 23.2 Bundle 构建请求

```rust
pub struct AssetDatabaseBundleBuild {
    pub name: String,
    pub compression: CompressionKind,
    pub chunk_policy: BundleChunkPartitionPolicy,
    pub assets: Vec<AssetId>,
}

impl AssetDatabaseBundleBuild {
    pub fn new(name: impl Into<String>, assets: Vec<AssetId>) -> Self;
    pub fn with_compression(self, compression: CompressionKind) -> Self;
    pub fn with_chunk_policy(self, chunk_policy: BundleChunkPartitionPolicy) -> Self;
}

pub struct AssetDatabaseBundleBuildOutput {
    pub bytes: Vec<u8>,
    pub asset_count: usize,
}
```

## 23.3 API

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetDatabaseScanReport {
    pub sources: Vec<AssetPath>,
    pub metadata: Vec<AssetMetadata>,
    pub diagnostics: Vec<AssetDatabaseDiagnostic>,
    pub added: Vec<AssetPath>,
    pub changed: Vec<AssetPath>,
    pub unchanged: Vec<AssetPath>,
    pub removed: Vec<AssetPath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetDatabaseDiagnostic {
    MissingMetadata { path: AssetPath },
    StaleMetadata { id: AssetId, path: AssetPath },
    ChangedSource {
        id: AssetId,
        path: AssetPath,
        previous_hash: ContentHash,
        current_hash: ContentHash,
    },
    MovedSourcePath {
        id: AssetId,
        old_path: AssetPath,
        new_path: AssetPath,
    },
}
```

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetMetadataMigrationReport {
    pub mode: AssetMetadataMigrationMode,
    pub files: Vec<AssetMetadataMigrationFileReport>,
}

impl AssetMetadataMigrationReport {
    pub fn written_files(&self) -> usize;
    pub fn total_entries(&self) -> usize;
    pub fn upgradeable_entries(&self) -> usize;
    pub fn has_blocking_errors(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetMetadataMigrationFileReport {
    pub kind: AssetMetadataMigrationFileKind,
    pub path: std::path::PathBuf,
    pub header: Option<String>,
    pub target_header: String,
    pub status: AssetMetadataMigrationStatus,
    pub written: bool,
    pub entries: Vec<AssetMetadataMigrationEntry>,
    pub errors: Vec<String>,
}

impl AssetMetadataMigrationFileReport {
    pub fn current_entries(&self) -> usize;
    pub fn upgradeable_entries(&self) -> usize;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetMetadataMigrationFileKind {
    Registry,
    Sidecar,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssetMetadataMigrationMode {
    DryRun,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetMetadataMigrationStatus {
    Current,
    Upgradeable,
    UnsupportedVersion,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetMetadataMigrationEntry {
    pub line: usize,
    pub id: Option<AssetId>,
    pub field_count: usize,
    pub status: AssetMetadataMigrationStatus,
    pub message: Option<String>,
}
```

```rust
impl AssetDatabase {
    pub fn new(config: AssetDatabaseConfig) -> Self;

    pub fn config(&self) -> &AssetDatabaseConfig;

    pub fn set_io<I: AssetIo>(&mut self, io: I);

    pub fn register_importer<I: AssetImporter>(&mut self, importer: I);

    pub fn register_cooker<C: AssetCooker>(&mut self, cooker: C);

    pub fn register_builtin_importers(&mut self);

    pub fn register_builtin_cookers(&mut self);

    pub fn diagnostics(&self) -> &[AssetDatabaseDiagnostic];

    pub fn drain_diagnostics(&mut self) -> impl Iterator<Item = AssetDatabaseDiagnostic> + '_;

    pub fn scan(&self) -> AssetResult<Vec<AssetPath>>;

    pub fn scan_with_metadata(&mut self) -> AssetResult<AssetDatabaseScanReport>;

    pub fn import_asset_path(&mut self, path: &AssetPath) -> AssetResult<AssetId>;

    pub fn import_asset_path_with_settings(
        &mut self,
        path: &AssetPath,
        settings: &ImporterSettings,
    ) -> AssetResult<AssetId>;

    pub fn cook_asset(&mut self, id: AssetId, target: TargetPlatform) -> AssetResult<CookOutput>;

    pub fn build_bundle(
        &self,
        build: &AssetDatabaseBundleBuild,
    ) -> AssetResult<AssetDatabaseBundleBuildOutput>;

    pub fn build_bundle_bytes(&self, build: &AssetDatabaseBundleBuild) -> AssetResult<Vec<u8>>;

    pub fn registry(&self) -> &AssetRegistry;

    pub fn registry_mut(&mut self) -> &mut AssetRegistry;

    pub fn save_registry(&self) -> AssetResult<()>;

    pub fn load_registry(&mut self) -> AssetResult<()>;

    pub fn metadata_migration_report(&self) -> AssetResult<AssetMetadataMigrationReport>;

    pub fn migrate_metadata(
        &self,
        mode: AssetMetadataMigrationMode,
    ) -> AssetResult<AssetMetadataMigrationReport>;
}
```

`scan_with_metadata` 会先加载 sidecar，再按当前 source 列表生成增量分类：
`added` 表示没有 metadata 的新源文件，`changed` 表示已有 metadata 但 source hash
变化或路径发生迁移，`unchanged` 表示 hash 未变，`removed` 表示 registry/sidecar 中仍有
metadata 但 source 列表中已经没有对应文件。诊断列表会保留缺失 metadata、stale
metadata、changed source 和 moved source 的结构化信息。
OBJ source（`.obj` 或 `NGA_MODEL_OBJ_V1` `.model`）的 source hash 会把可读取的同目录及
子目录 `.mtl` context source hash 一起折叠进去，因此仅修改 material library 也会在
`scan_with_metadata` 中把对应 model source 标记为 changed 并触发稳定 `AssetId` reimport。
导入时传入的 `ImporterSettings` 会以排序后的 key/value 对保存到 `AssetMetadata`，
并随 registry/sidecar 一起持久化；`settings_hash` 仍用于判断导入设置内容是否变化。
内建 `MaterialImporter` 会通过 registry 快照解析 material source 中的 shader/texture
路径，导入前需要相关 shader/texture metadata 已存在；成功后依赖会随 metadata、bundle
manifest 和 dependency report 一起保存。

`build_bundle` 使用 `AssetMetadata::cooked_path` 从 `cooked_root` 读取 payload，
使用 `AssetMetadata::path` 作为 bundle 内的运行时路径，并把
`AssetMetadata::dependencies` 写入 manifest。缺少 metadata、缺少 cooked path、
缺少 cooked 文件或 cooked hash 不匹配都会返回可见错误。

---

## 23.4 Registry 与 Sidecar 兼容性

当前写出文本格式是显式 V1：

```text
NGA_ASSET_REGISTRY_V1
NGA_ASSET_META_V1
```

registry payload 和 sidecar payload 都使用同一行 `|` 分隔元数据布局；当前格式是
14 字段，兼容读取旧的 12 字段和 13 字段 V1 payload，并把缺失的 `labels` 或
`importer_settings` 迁移为空列表。
迁移工具还识别 legacy `NGA_ASSET_REGISTRY_V0` / `NGA_ASSET_META_V0` 输入；
V0 payload 是 11 字段布局，保留 id/type、path/source/cooked path、importer、
hash 和 dependency id 列表，迁移到 V1 时会把缺失的 `version_hash`、`labels` 和
`importer_settings` 写为空。普通 `load_registry` / `load_metadata_sidecars` 仍要求 V1
header；V0 文件需要先通过 migration report/write-back 流程升级。
读取时会校验 header、字段数量、数字 id/hash/version 字段和依赖 id 列表。
registry 错误会带有 `registry line N` 上下文；sidecar 错误会带有具体 `.meta`
文件路径上下文。未知 `NGA_ASSET_REGISTRY_V*` 或 `NGA_ASSET_META_V*` 版本会返回包含
`run metadata migration report` 的可执行诊断；非版本化错误 header 仍按无效 header 报告。
`metadata_migration_report` 是 `DryRun`：扫描 registry 文件和 imported root 下的 `.meta`
sidecar，按文件和 entry 标记 `Current`、`Upgradeable`、`UnsupportedVersion` 或
`Invalid`，并不会写回文件或修改 `AssetRegistry`。
`migrate_metadata(AssetMetadataMigrationMode::Write)` 会把 `Upgradeable` 文件写回当前
14 字段格式，并在每个 file report 上标记 `written=true`；`Invalid` 和
`UnsupportedVersion` 文件不会被覆盖，但仍保留诊断。写回只修改磁盘上的 registry/sidecar
文本，不会隐式加载或修改当前 `AssetDatabase` 内存 registry。

---

## 24. 内建资源类型

## 24.1 Texture

```rust
#[derive(Clone, Debug)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub mip_count: u32,
    pub array_layers: u32,
    pub usage: TextureUsage,
    pub gpu: Option<GpuTextureHandle>,
}
```

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8Srgb,
    Bgra8Unorm,
    Bgra8Srgb,
    Rg16Float,
    Rgba16Float,
    R32Float,
    Depth32Float,
    Bc1,
    Bc3,
    Bc5,
    Bc7,
    Etc2,
    Astc4x4,
}
```

```rust
bitflags::bitflags! {
    pub struct TextureUsage: u32 {
        const SAMPLED           = 1 << 0;
        const STORAGE           = 1 << 1;
        const RENDER_ATTACHMENT = 1 << 2;
        const COPY_SRC          = 1 << 3;
        const COPY_DST          = 1 << 4;
    }
}
```

```rust
#[derive(Clone, Debug)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub mip_count: u32,
    pub usage: TextureUsage,
}

#[derive(Clone, Debug)]
pub struct TextureUploadData {
    pub bytes: Vec<u8>,
    pub mip_offsets: Vec<u64>,
}
```

---

## 24.2 Texture Import Settings

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TextureImportSettings {
    pub srgb: bool,
    pub generate_mips: bool,
    pub normal_map: bool,
    pub compression: TextureCompression,
    pub max_size: Option<u32>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum TextureCompression {
    None,
    Auto,
    Bc7,
    Bc5,
    Etc2,
    Astc,
}
```

---

## 24.3 Mesh

```rust
#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub uv_sets: Vec<Vec<[f32; 2]>>,
    pub tangents: Vec<[f32; 4]>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub index_format: MeshIndexFormat,
    pub vertex_buffer: MeshVertexBuffer,
    pub gpu: Option<GpuResourceHandle>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshVertexBuffer {
    pub layout: MeshVertexLayout,
    pub bytes: Vec<u8>,
}
```

`MeshLoader` 当前支持一个最小文本 payload：

```text
v 0 0 0
v 1 0 0
v 0 1 0
n 0 0 1
n 0 0 1
n 0 0 1
uv 0 0
uv 1 0
uv 0 1
uv1 0.25 0.25
uv1 0.75 0.25
uv1 0.25 0.75
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
j 0 1 2 3
j 0 0 0 0
j 1 2 3 4
w 0.7 0.2 0.1 0
w 1 0 0 0
w 0.25 0.25 0.25 0.25
i 0 1 2
```

`v` 行声明一个三维顶点，`n` 行声明一个三维 normal，`uv` 行声明二维 texture coordinate，
并填充 `Mesh::uvs` 作为 UV0；`uv1`、`uv2` 等行声明额外 UV set，并按 UV1+ 顺序填充
`Mesh::uv_sets`。`t` 行声明四维 tangent/handedness，`j`/`joints` 行声明四个 `u16`
skin joint index，`w`/`weights` 行声明四个 skin weight，`i` 行声明一个三角形的三个
`u32` 索引。`n`、`uv`、extra UV sets、`t` 和 skinning block 可省略；如果存在，则数量必须
分别等于 vertex 数量，skin joint 和 weight 数量也必须彼此匹配。loader 会校验每行参数数量、
数字解析、至少一个顶点、attribute 数量、非负且总和为正并在 `0.001` 容差内归一化到 `1.0`
的 skin weight，以及索引不能越过顶点数量；成功后
会产生 `GpuUploadKind::Mesh` 上传命令。加载成功时会同时生成 `Mesh::vertex_buffer`：它是按
position、normal、UV0、UV1+、tangent、joints、weights 顺序 interleave 的 little-endian
binary vertex buffer；GPU 上传命令的 `metadata` 会携带同一份 vertex layout、vertex byte
长度、index byte 长度和 `Uint32` index format，`bytes` 则为 vertex buffer 后接 `u32`
index buffer。

`MeshLoader` 也支持二进制 runtime payload。Payload 以 ASCII magic
`NGA_MESH_BINARY_V1\n` 开头，后接 little-endian header：

```text
u32 vertex_count
u32 index_count
u32 flags
u32 secondary_uv_mask
```

`flags` bit 为 `1 = normals`、`2 = UV0`、`4 = tangents`、`8 = skinning`、`16 = u16 indices`。`secondary_uv_mask`
的 bit 0 表示 UV1，bit 1 表示 UV2，依此类推；非零 secondary UV mask 必须同时设置 UV0 flag。
Header 后的数据按固定顺序紧密排列：positions、可选 normals、可选 UV0、按 mask bit 升序出现的
UV1+ blocks、可选 tangents、可选 skin joints、可选 skin weights、indices。每个 position/normal 是
3 个 little-endian `f32`，UV 是 2 个 `f32`，tangent/weight 是 4 个 `f32`，joint 是
4 个 little-endian `u16`；indices 默认为 little-endian `u32`，设置 `16 = u16 indices`
flag 时为 little-endian `u16` 并在 decode 后扩展为 `Mesh::indices: Vec<u32>`，同时把
`Mesh::index_format` 保留为 `MeshIndexFormat::Uint16`，让后续 GPU upload metadata、
upload bytes 和 GPU byte accounting 继续使用 16-bit index buffer。Loader 会校验 unsupported flag、secondary UV 必须有 UV0、
index count 必须是 3 的倍数、payload byte length 必须精确匹配 header、所有 `f32` 必须有限、
至少一个顶点、attribute 数量、skin joint/weight 数量、非负且总和为正并在 `0.001` 容差内归一化到 `1.0`
的 skin weight 和索引范围；成功后
生成的 `Mesh`、`Mesh::vertex_buffer` 和 GPU upload metadata 与 payload 的 index width 一致。

---

## 24.4 Model Import Settings

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ModelImportSettings {
    pub import_meshes: bool,
    pub import_materials: bool,
    pub import_animations: bool,
    pub import_skeleton: bool,
    pub import_physics_meshes: bool,
    pub optimize_meshes: bool,
    pub generate_tangents: bool,
    pub generate_lods: bool,
    pub scale: f32,
}
```

`ModelImporter` 会从通用 `ImporterSettings` 读取同名 key。`import_meshes`、
`import_materials`、`import_animations`、`import_skeleton` 和 `import_physics_meshes` 默认为 `true`，用于过滤
generated subresource，并会移除指向已过滤 generated label 的本地依赖 metadata。
`scale` 默认为 `1.0`，必须是 finite 且大于 0，会统一缩放 manifest/OBJ 生成 mesh payload
中的 position，以及 manifest 生成 physics mesh payload 中的 `v` 顶点；`generate_tangents`
默认为 `true`，控制 OBJ UV mesh 是否生成 `t` tangent 行。
`optimize_meshes` 默认为 `false`，为 `true` 时会在缩放后解码 generated mesh payload，
删除未被 index 引用的顶点，按 position/normal/UV/secondary-UV/tangent/skinning attribute tuple
去重，剔除重复顶点 index 或几何零面积的退化 triangle，并重写 triangle indices 为确定性文本
mesh payload；若带 index 的 mesh 全部 triangle 都被剔除，会返回带 importer/source/settings
上下文的 `ModelImporter` import error。`generate_lods` 默认为 `false`，
为 `true` 时会为每个至少包含两个 triangle 的 generated mesh 追加一个 `<Label>.LOD1`
generated mesh，剔除重复 index 或几何零面积的退化 triangle 后保留隔一个非退化 triangle
的确定性子集，重写 referenced vertex/attribute tuple 和 indices，复制原 mesh 的 material/skin
本地 dependency metadata，并对无效 LOD 输入或 LOD 输入全部 triangle 都是退化 triangle 的情况返回
带 importer/source/settings 上下文的 `ModelImporter` import error。非法 bool 或非法 scale
同样会返回带 importer/source/settings 上下文的 `ModelImporter` import error。

---

## 24.5 Material

```rust
#[derive(Clone, Debug)]
pub struct Material {
    pub name: Option<String>,
    pub shader: Option<Handle<Shader>>,
    pub properties: MaterialProperties,
    pub textures: Vec<MaterialTextureBinding>,
    pub render_state: MaterialRenderState,
    pub gpu: Option<GpuResourceHandle>,
}
```

```rust
#[derive(Clone, Debug, Default)]
pub struct MaterialProperties {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub alpha_cutoff: Option<f32>,
    pub custom: std::collections::HashMap<String, MaterialPropertyValue>,
}
```

```rust
#[derive(Clone, Debug)]
pub enum MaterialPropertyValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    Bool(bool),
}
```

```rust
#[derive(Clone, Debug)]
pub struct MaterialTextureBinding {
    pub name: String,
    pub texture: Handle<Texture>,
    pub sampler: SamplerDesc,
    pub options: MaterialTextureOptions,
}

#[derive(Clone, Copy, Debug)]
pub struct MaterialTextureOptions {
    pub transform: MaterialTextureTransform,
    pub bump_scale: Option<f32>,
    pub color_remap: Option<[f32; 2]>,
    pub source_channel: Option<MaterialTextureChannel>,
    pub boost: Option<f32>,
    pub blend_u: Option<bool>,
    pub blend_v: Option<bool>,
    pub color_correction: Option<bool>,
    pub color_space: Option<MaterialTextureColorSpace>,
    pub projection: Option<MaterialTextureProjection>,
    pub texture_resolution: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub enum MaterialTextureChannel {
    Red,
    Green,
    Blue,
    Matte,
    Luminance,
    Depth,
}

#[derive(Clone, Copy, Debug)]
pub enum MaterialTextureColorSpace {
    Srgb,
    Linear,
    NonColor,
    Raw,
}

#[derive(Clone, Copy, Debug)]
pub enum MaterialTextureProjection {
    Flat,
    Sphere,
    CubeTop,
    CubeBottom,
    CubeFront,
    CubeBack,
    CubeLeft,
    CubeRight,
}

#[derive(Clone, Copy, Debug)]
pub struct MaterialTextureTransform {
    pub offset: [f32; 3],
    pub scale: [f32; 3],
    pub turbulence: [f32; 3],
}
```

```rust
#[derive(Clone, Copy, Debug)]
pub struct MaterialRenderState {
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    pub depth_write: bool,
    pub depth_test: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}
```

`MaterialLoader` 当前支持一个文本 payload：

```text
name=hero
shader=shaders/pbr.wgsl
texture.albedo=textures/albedo.texture
texture.albedo.sampler.address=clamp_to_edge
texture.albedo.sampler.filter=linear
texture.albedo.transform.offset=0.25,0.5,0
texture.albedo.transform.scale=2,3,1
texture.albedo.transform.turbulence=0.01,0.02,0.03
texture.albedo.bump_scale=0.3
texture.albedo.color_remap=0.1,0.9
texture.albedo.source_channel=green
texture.albedo.boost=1.5
texture.albedo.blend_u=false
texture.albedo.blend_v=true
texture.albedo.color_correction=true
texture.albedo.color_space=srgb
texture.albedo.projection=sphere
texture.albedo.texture_resolution=1024
base_color=1,0.5,0.25,1
metallic=0.2
roughness=0.7
emissive=0.1,0.2,0.3
alpha_cutoff=0.45
alpha_mode=mask
double_sided=true
depth_write=false
depth_test=false
custom.clearcoat.float=0.7
custom.tint.vec3=0.1,0.2,0.3
custom.illumination_model.int=2
```

`shader` 和 `texture.<name>` 会通过 `LoadContext` 注册依赖；`texture.<name>.sampler.address`
会写入对应 `MaterialTextureBinding::sampler.address`，支持 `repeat`、`clamp`、
`clamp_to_edge`；`texture.<name>.sampler.filter` 会写入 sampler filter，支持 `nearest`
和 `linear`。`texture.<name>.transform.offset/scale/turbulence` 会写入三维 texture
transform，`texture.<name>.bump_scale` 会写入 bump/normal map multiplier，
`texture.<name>.color_remap` 会写入二维 color remap range，
`texture.<name>.source_channel` 会写入 texture channel（`red`/`green`/`blue`/`matte`/
`luminance`/`depth`，也接受 MTL 单字母），`texture.<name>.boost` 会写入 mipmap sharpness/
contrast boost，`texture.<name>.blend_u`/`blend_v` 会写入 U/V blend flags，
`texture.<name>.color_correction` 会写入 MTL color-correction flag，
`texture.<name>.color_space` 会写入 texture color-space hint（支持 `srgb`、`linear`、
`non_color`、`raw`，并兼容 `sRGB`、`Non-Color` 等大小写/分隔写法），
`texture.<name>.projection` 会写入 projection type（`flat`/`sphere`/`cube_top`/
`cube_bottom`/`cube_front`/`cube_back`/`cube_left`/`cube_right`），
`texture.<name>.texture_resolution` 会写入非零 texture resolution hint。Texture metadata
可以写在对应 `texture.<name>` 前后，且不会被 importer 当作 texture dependency。
`base_color` 必须是四个逗号分隔的 `f32`；`emissive` 必须是三个逗号分隔的 `f32`；
`metallic`、`roughness`、`alpha_cutoff` 和自定义未知 key 当前按 `f32` 解析；
`custom.<name>` 是兼容的 float custom property，`custom.<name>.float`、
`custom.<name>.vec2`、`custom.<name>.vec3`、`custom.<name>.vec4`、`custom.<name>.int`
和 `custom.<name>.bool` 会写入对应 `MaterialPropertyValue` variant；
`alpha_mode` 支持 `opaque`、`mask`、`blend`；`double_sided`、`depth_write` 和
`depth_test` 按 bool 解析。缺少 `=`、非法 float/bool/sampler/alpha-mode 值、非法
texture channel/color-space/projection/resolution metadata、非法 UTF-8 或错误数量的
`base_color`/`emissive`/texture transform/color remap/custom vector 值都会返回
`AssetError::Decode`。

---

## 24.6 Shader

```rust
#[derive(Clone, Debug)]
pub struct Shader {
    pub stages: Vec<ShaderStageSource>,
    pub reflection: Option<ShaderReflection>,
    pub gpu: Option<GpuResourceHandle>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct ShaderStageSource {
    pub stage: ShaderStage,
    pub source: ShaderSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Clone, Debug)]
pub enum ShaderSource {
    Wgsl(String),
    Glsl(String),
    Spirv(Vec<u32>),
}
```

```rust
#[derive(Clone, Debug)]
pub struct ShaderDesc {
    pub label: Option<String>,
    pub stages: Vec<ShaderStage>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct ShaderReflection {
    pub bind_groups: Vec<String>,
    pub vertex_inputs: Vec<String>,
}
```

`ShaderLoader` 当前接受非空 UTF-8 WGSL/GLSL-like source，并按资源 label 选择 stage：
无 label 或 `#fragment` 作为 fragment，`#vertex` 作为 vertex，`#compute` 作为 compute。
加载时会做轻量源码结构诊断：`()`、`[]`、`{}` 必须匹配，带 `@group(...)` 或
`@binding(...)` 的资源声明必须同时包含两者。WGSL reflection 会填充
`ShaderReflection::bind_groups`，格式为 `group=<u32>,binding=<u32>,name=<identifier>`；
当 stage 为 vertex 时，也会从 `@location(<u32>) <identifier>:` 声明提取
`ShaderReflection::vertex_inputs`，格式为 `location=<u32>,name=<identifier>`。未知 label、
空 source、非法 UTF-8、括号/大括号不匹配或无效 binding/location attribute 会返回
`AssetError::Decode`。加载成功后会产生 `GpuUploadKind::Shader` 上传命令。

---

## 24.7 AudioClip

```rust
#[derive(Clone, Debug)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: AudioSamples,
    pub duration_seconds: f32,
    pub streaming: bool,
}
```

```rust
#[derive(Clone, Debug)]
pub enum AudioSamples {
    I16(Vec<i16>),
    F32(Vec<f32>),
    Streaming(AudioStreamHandle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AudioStreamHandle(pub u64);
```

`AudioLoader` 支持一个最小文本 payload，用于测试和 pass-through cooked bytes：

```text
NGA_AUDIO_V1
sample_rate=48000
channels=2
format=i16
samples=0,1000,-1000,0
streaming=false
```

`format` 可以是 `i16` 或 `f32`，`samples` 是逗号分隔的交错采样值，数量必须是
`channels` 的非零倍数。加载成功后不会产生 GPU upload command；无效 header、
缺失字段、未知 key、非法采样值或不匹配的 sample count 都会返回 `AssetError::Decode`。

`AudioLoader` 还会解析基础 RIFF/WAVE payload：当前支持 PCM `format=1`/16-bit
little-endian 采样并生成 `AudioSamples::I16`，以及 IEEE float `format=3`/32-bit
little-endian 采样并生成 `AudioSamples::F32`。`fmt ` 和 `data` chunk 必须存在，
`channels`/`sample_rate` 必须非零，`block_align` 必须匹配声道数和采样字节数；
无效或不支持的 WAV payload 会返回 `AssetError::Decode`，加载成功同样是 CPU-only
`Ready`，不会产生 GPU upload command。

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AudioImportSettings {
    pub force_mono: bool,
    pub normalize: bool,
    pub streaming: bool,
    pub compression: AudioCompression,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum AudioCompression {
    None,
    Vorbis,
    Opus,
}
```

---

## 24.8 AnimationClip

```rust
#[derive(Clone, Debug)]
pub struct AnimationClip {
    pub duration: f32,
    pub ticks_per_second: f32,
    pub tracks: Vec<AnimationTrack>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct AnimationTrack {
    pub target: AnimationTarget,
    pub translations: Vec<Keyframe<[f32; 3]>>,
    pub rotations: Vec<Keyframe<[f32; 4]>>,
    pub scales: Vec<Keyframe<[f32; 3]>>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
}
```

```rust
#[derive(Clone, Debug)]
pub enum AnimationTarget {
    NodeName(String),
    NodeIndex(u32),
    BoneName(String),
}
```

`AnimationLoader` 当前支持一个最小文本 payload：

```text
NGA_ANIMATION_V1
duration=1
ticks_per_second=60
track=bone:Root
translation=0:0,0,0
rotation=0:0,0,0,1
scale=0:1,1,1
```

第一行必须是 `NGA_ANIMATION_V1`。`duration` 和 `ticks_per_second` 必填，必须是 finite
数字，且必须大于 0。
`track` 可使用 `node:<name>`、`bone:<name>` 或 `node_index:<u32>`。`translation`、
`rotation`、`scale` 行会追加到最近声明的 track，格式为 `time:x,y,z` 或
`time:x,y,z,w`。Keyframe time 必须是 finite、非负、不大于 clip `duration`，且同一
track/channel 内按时间非递减排序。每个 track 必须至少包含一个 translation、rotation 或 scale
keyframe，且同一 clip 内不能重复声明同一个 target。无效 header、缺失必填字段、keyframe 出现在 track
之前、空 track、重复 target、非法 target、非法数字、非 finite 数字、错误数量的向量值、越界或乱序
keyframe time 都会返回 `AssetError::Decode`。当前为 CPU-only 资源，
不会产生 GPU upload command。

---

## 24.9 Skeleton

```rust
#[derive(Clone, Debug)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub inverse_bind_poses: Vec<glam::Mat4>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct Bone {
    pub name: String,
    pub parent: Option<u32>,
    pub local_bind_transform: glam::Mat4,
}
```

`SkeletonLoader` 当前支持一个最小文本 payload：

```text
NGA_SKELETON_V1
bone=Root
bone=Spine;parent=0
```

第一行必须是 `NGA_SKELETON_V1`。`bone=<name>` 声明一个 bone，`parent=<index>` 可选且
只能引用已经声明过的更早 bone。Bone 字段还可声明 `bind=`/`local_bind=` 和
`inverse_bind=`/`inverse_bind_pose=`，值为 16 个逗号或空白分隔的有限 `f32`，按 row-major
写入 `local_bind_transform` 和 `Skeleton::inverse_bind_poses`；未声明时默认为 identity。
如果显式声明 `inverse_bind`，loader 会把 parent 链上的 local bind 矩阵累积为 model-space
bind pose，并要求 `bind_pose * inverse_bind` 在 `0.001` 容差内等于 identity。
无效 header、缺少 bone、非法 parent、parent 指向未来 bone、未知 key、未知 bone 字段、
bind/inverse-bind matrix 数量错误、非有限数值或显式 inverse-bind 不匹配都会返回
`AssetError::Decode`。当前为 CPU-only 资源，
不会产生 GPU upload command。

---

## 24.10 SceneAsset

```rust
#[derive(Clone, Debug)]
pub struct SceneAsset {
    pub name: String,
    pub entities: Vec<SerializedEntity>,
    pub dependencies: Vec<UntypedHandle>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct SerializedEntity {
    pub name: Option<String>,
    pub parent: Option<u64>,
    pub components: Vec<SerializedComponent>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct SerializedComponent {
    pub type_name: String,
    pub data: Vec<u8>,
}
```

`SceneLoader` 当前支持一个最小文本 payload：

```text
NGA_SCENE_V1
name=level
dependency=meshes/tri.mesh
dependency=materials/hero.material
entity=Root
component=Transform|translation=0,0,0
entity=Hero;parent=0
component=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material
```

第一行必须是 `NGA_SCENE_V1`，`name` 必填。`dependency=<path>` 会通过
`LoadContext` 注册依赖并在 `SceneAsset.dependencies` 中保存弱 `UntypedHandle`；
当前按路径扩展名识别 `texture`/`tex`/`rgba`、`mesh`、`wgsl`/`glsl`/`shader`、
`material`/`mat`、`audio`/`wav`/`ogg`、`scene`、`prefab`、`skeleton`/`skel` 和
`physics`/`physicsmesh`/`pmesh`。`entity=<name>` 创建实体，
可用 `entity=<name>;parent=<index>` 指向父实体索引。`component=<type>|<data>`
会追加到最近声明的实体，`data` 以 UTF-8 字节保存。已知组件 schema 中的 asset 字段也会
注册同样的运行时依赖并写入 `SceneAsset.dependencies`：`MeshRenderer.mesh/material`、
`SkinnedMeshRenderer.mesh/skeleton/material`、`AudioSource.clip`、`PhysicsCollider.mesh`
或 `physics_mesh`、`SceneInstance.scene` 和 `PrefabInstance.prefab`。未知组件类型和非 asset
字段仍按原始字节透传。非法 header、缺失 `name`、未知 key、组件出现在实体之前、非法 parent、
缺失 `|`、已知组件字段不是 `key=value`、asset 字段为空、字段扩展名类型不匹配或不支持的依赖扩展名都会返回
`AssetError::Decode`。

---

## 24.11 Prefab

```rust
#[derive(Clone, Debug)]
pub struct Prefab {
    pub root: SerializedEntity,
    pub children: Vec<SerializedEntity>,
    pub dependencies: Vec<UntypedHandle>,
}
```

`PrefabLoader` 当前支持一个最小文本 payload：

```text
NGA_PREFAB_V1
dependency=meshes/tri.mesh
dependency=materials/hero.material
root=Hero
component=Transform|translation=0,0,0
child=Weapon;parent=0
component=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material
```

第一行必须是 `NGA_PREFAB_V1`，`root=<name>` 必填且只能出现一次。`dependency=<path>`
按与 `SceneLoader` 相同的扩展名规则注册依赖，并在 `Prefab.dependencies` 中保存弱
`UntypedHandle`。`child=<name>` 创建子实体，可用 `child=<name>;parent=<index>`
保存父实体索引。`component=<type>|<data>` 追加到最近声明的 root 或 child，并按与
`SceneLoader` 相同的已知组件 asset 字段规则注册依赖。非法 header、缺失 root、重复 root、
root 带 parent、child 出现在 root 之前、组件出现在实体之前、非法 parent、缺失 `|`、
已知组件字段不是 `key=value`、asset 字段为空、字段扩展名类型不匹配或不支持的依赖扩展名都会返回
`AssetError::Decode`。

---

## 24.12 Font

```rust
#[derive(Clone, Debug)]
pub struct Font {
    pub family_name: String,
    pub data: FontData,
}

#[derive(Clone, Debug)]
pub enum FontData {
    TrueType(Vec<u8>),
    OpenType(Vec<u8>),
    Bitmap(BitmapFont),
}
```

`FontLoader` 当前支持一个最小 bitmap 文本 payload：

```text
NGA_FONT_V1
family=Debug Sans
glyph=char=A;size=2x1;bitmap=0,255
```

第一行必须是 `NGA_FONT_V1`。`family` 必填，至少需要一个 `glyph`。`glyph` 字段使用
`char=<单字符>;size=<width>x<height>;bitmap=<u8,u8,...>`，bitmap 字节数必须等于
`width * height`。无效 header、缺失 family、缺失 glyph、非法 size、非法 bitmap 值、
bitmap 长度不匹配或未知字段都会返回 `AssetError::Decode`。当前生成 `FontData::Bitmap`，
不会产生 GPU upload command。

---

## 24.13 PhysicsMesh

```rust
#[derive(Clone, Debug)]
pub struct PhysicsMesh {
    pub vertices: Vec<glam::Vec3>,
    pub indices: Vec<[u32; 3]>,
    pub kind: PhysicsMeshKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsMeshKind {
    TriMesh,
    ConvexHull,
    HeightField,
}
```

`PhysicsMeshLoader` 当前支持一个最小文本 payload：

```text
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
j 0 0 0 0
j 0 0 0 0
j 0 0 0 0
w 1 0 0 0
w 1 0 0 0
w 1 0 0 0
i 0 1 2
```

第一行必须是 `NGA_PHYSICS_MESH_V1`。`kind` 可为 `trimesh`/`tri_mesh`、
`convex`/`convex_hull`、`heightfield`/`height_field`。`v` 行声明三维顶点，`i` 行声明
三个 `u32` 索引。loader 会校验 kind、参数数量、数字解析、至少一个顶点、非 convex
mesh 至少一个 triangle，以及索引不能越过顶点数量；无效输入返回 `AssetError::Decode`。
当前为 CPU-only 资源，不会产生 GPU upload command。

---

## 25. ECS 集成

## 25.1 资源组件

```rust
#[derive(Clone, Debug)]
pub struct MeshRendererComponent {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct SkinnedMeshRendererComponent {
    pub mesh: Handle<Mesh>,
    pub skeleton: Handle<Skeleton>,
    pub material: Handle<Material>,
}
```

```rust
#[derive(Clone, Debug)]
pub struct AudioSourceComponent {
    pub clip: Handle<AudioClip>,
    pub looping: bool,
    pub volume: f32,
}
```

```rust
#[derive(Clone, Debug)]
pub struct PhysicsColliderComponent {
    pub mesh: Handle<PhysicsMesh>,
    pub dynamic: bool,
}

pub trait InstantiationSink {
    fn spawn_entity(&mut self, entity_index: usize, name: Option<&str>, parent: Option<u64>);
    fn attach_component(&mut self, entity_index: usize, type_name: &str, data: &[u8]);
}

pub trait HostInstantiationSink {
    type Entity: Clone;
    type Error;

    fn spawn_entity(
        &mut self,
        entity_index: usize,
        name: Option<&str>,
        parent: Option<&Self::Entity>,
    ) -> Result<Self::Entity, Self::Error>;

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        type_name: &str,
        data: &[u8],
    ) -> Result<(), Self::Error>;
}

pub trait TypedHostInstantiationSink {
    type Entity: Clone;
    type Error;

    fn spawn_entity(
        &mut self,
        entity_index: usize,
        name: Option<&str>,
        parent: Option<&Self::Entity>,
    ) -> Result<Self::Entity, Self::Error>;

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        component: EcsComponentInstance,
    ) -> Result<(), Self::Error>;
}
```

```rust
#[derive(Clone, Debug)]
pub struct SceneInstanceComponent {
    pub scene: Handle<SceneAsset>,
    pub loaded: bool,
}

#[derive(Clone, Debug)]
pub struct PrefabInstanceComponent {
    pub prefab: Handle<Prefab>,
    pub loaded: bool,
}
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneInstantiationPlan {
    pub scene: AssetId,
    pub entity_count: usize,
    pub component_count: usize,
    pub dependency_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrefabInstantiationPlan {
    pub prefab: AssetId,
    pub entity_count: usize,
    pub component_count: usize,
    pub dependency_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstantiationAssetReference {
    pub entity_index: usize,
    pub component_index: usize,
    pub component_type: String,
    pub field: String,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInstantiationReport<Entity> {
    pub source: AssetId,
    pub entities: Vec<Entity>,
    pub root_entities: Vec<Entity>,
    pub attached_component_count: usize,
}
```

```rust
#[derive(Clone, Debug)]
pub enum EcsComponentInstance {
    MeshRenderer(MeshRendererComponent),
    SkinnedMeshRenderer(SkinnedMeshRendererComponent),
    AudioSource(AudioSourceComponent),
    PhysicsCollider(PhysicsColliderComponent),
    SceneInstance(SceneInstanceComponent),
    PrefabInstance(PrefabInstanceComponent),
    Unknown { type_name: String, data: Vec<u8> },
}

pub fn materialize_serialized_component(
    assets: &mut AssetServer,
    component: &SerializedComponent,
) -> Result<EcsComponentInstance, EcsComponentMaterializationError>;
```

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneInstantiationCommand {
    SpawnEntity {
        entity_index: usize,
        name: Option<String>,
        parent: Option<u64>,
    },
    AttachComponent {
        entity_index: usize,
        type_name: String,
        data: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrefabInstantiationCommand {
    SpawnEntity {
        entity_index: usize,
        name: Option<String>,
        parent: Option<u64>,
    },
    AttachComponent {
        entity_index: usize,
        type_name: String,
        data: Vec<u8>,
    },
}
```

`SceneInstanceComponent::instantiation_plan(&AssetServer)` 只在 scene handle Ready、其依赖也
Ready、且 `loaded == false` 时返回计划。计划只统计待实例化实体数、组件数和 scene 中记录的
依赖 handle 数，不会修改 `AssetServer`，也不会取得 scene 资源所有权。

`SceneInstanceComponent::instantiation_commands(&AssetServer)` 会在同样的 ready 条件下，按
实体顺序导出稳定的 `SpawnEntity` / `AttachComponent` 命令序列，供宿主 ECS 或场景系统消费。
它仍然不修改 `AssetServer`，也不会直接接管实体生命周期。

`SceneInstantiationPlan::apply(&SceneAsset, &mut impl InstantiationSink)` 和
`PrefabInstantiationPlan::apply(&Prefab, &mut impl InstantiationSink)` 会把同样的实体/组件序列
直接投递到宿主 sink。它们还是纯数据桥，不负责实体存储、组件反序列化或生命周期管理。

`SceneInstanceComponent::instantiation_asset_references(&AssetServer)` 和
`PrefabInstanceComponent::instantiation_asset_references(&AssetServer)` 会导出结构化组件资产引用，
包括 entity index、component index、组件类型、字段名、路径和资产类型。该 API 使用与
scene/prefab loader 相同的已知组件字段 schema，避免宿主 ECS 为依赖审计、预绑定或诊断重复解析
component bytes。

`SceneInstanceComponent::instantiate_host` / `PrefabInstanceComponent::instantiate_host`
接收 `HostInstantiationSink`，将序列化 parent index 映射为宿主实体句柄，并在成功后把 instance
component 的 `loaded` 置为 `true`。如果 parent index 无法解析，则返回
`HostInstantiationError::MissingParent`，并保持 `loaded == false`。

`SceneInstanceComponent::instantiate_typed_host` / `PrefabInstanceComponent::instantiate_typed_host`
接收 `TypedHostInstantiationSink`，会把已知的 `MeshRenderer`、`SkinnedMeshRenderer`、
`AudioSource`、`PhysicsCollider`、`SceneInstance` 和 `PrefabInstance` serialized component
materialize 成 `EcsComponentInstance`。这些 typed component 内部持有通过 `AssetServer::load`
创建的 server-tracked handles，因此能参与常规加载和 GC 引用计数。未知组件保留原始
`type_name` 和 bytes 透传给宿主。Typed host 实例化会先验证 parent index 并完成组件
materialization，然后才调用宿主 sink 的 spawn/attach；如果 materialization 失败，则不会向宿主
产生部分实体或组件，instance component 也保持 `loaded == false`。

`PrefabInstanceComponent::instantiation_plan(&AssetServer)` 和
`PrefabInstanceComponent::instantiation_commands(&AssetServer)` 语义与 scene 版本一致，只是
输入从 `SceneAsset` 换成了 `Prefab` 的 root/children 结构。Prefab 命令也按实体顺序导出稳定的
`SpawnEntity` / `AttachComponent` 序列，供宿主 ECS 直接消费。

---

## 25.2 ECS 系统顺序

```text
AssetRequestSystem
AssetServerUpdateSystem
GpuUploadPrepareSystem
AssetEventDispatchSystem
SceneInstantiationSystem
RenderPrepareSystem
AudioPrepareSystem
AssetGcSystem
```

### 推荐职责

```text
AssetRequestSystem:
    收集 gameplay / scene 的资源加载请求。

AssetServerUpdateSystem:
    推进异步加载、完成解码任务、派发加载状态。

GpuUploadPrepareSystem:
    从 AssetServer 取出 GpuUploadCommand，交给 Renderer 执行。

AssetEventDispatchSystem:
    处理 Ready / Failed / Reloaded 等事件。

SceneInstantiationSystem:
    SceneAsset Ready 后实例化实体。

AssetGcSystem:
    在帧尾执行资源卸载。
```

---

## 26. 错误类型

```rust
pub type AssetResult<T> = Result<T, AssetError>;
```

```rust
#[derive(Clone, Debug, thiserror::Error)]
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
    TypeMismatch {
        expected: String,
        actual: String,
    },

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
    DependencyFailed {
        asset: AssetId,
        dependency: AssetId,
    },

    #[error("cyclic dependency detected")]
    CyclicDependency,

    #[error("asset is already loaded: {id:?}")]
    AlreadyLoaded { id: AssetId },

    #[error("asset is not loaded: {id:?}")]
    NotLoaded { id: AssetId },

    #[error("unsupported asset capability: {0}")]
    Unsupported(&'static str),
}
```

`AlreadyLoaded` 由 `AssetServer::insert_loaded*` 显式产生，用于拒绝覆盖
queued/loading/ready/reloading/unloading 等 live asset；卸载后或失败/取消状态可再次插入。

```rust
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum AssetIoError {
    #[error("{action} failed: file not found: {path}")]
    NotFound {
        path: String,
        action: AssetIoAction,
    },

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

impl AssetIoError {
    pub fn action(&self) -> AssetIoAction;
    pub fn path(&self) -> &str;
    pub fn message(&self) -> Option<&str>;
    pub fn with_action(self, action: AssetIoAction) -> Self;
}
```

`AssetIoError` 转为 `AssetError::Io` 时会保留 action/path/source message，
因此 runtime-facing missing/read/list/metadata 错误可直接从 `AssetError`
文本中定位失败操作和逻辑资源路径。

```rust
pub type AssetLoadError = AssetError;
pub type ImportError = AssetError;
pub type CookError = AssetError;
```

---

## 27. 内建 Loader 示例 API

## 27.1 TextureLoader

```rust
pub struct TextureLoader;

impl TextureLoader {
    pub fn new() -> Self;
}

impl AssetLoader for TextureLoader {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &[&'static str];
    fn asset_type(&self) -> AssetTypeId;

    fn load(
        &self,
        ctx: &mut LoadContext,
        bytes: &[u8],
        settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError>;
}
```

---

## 27.2 MeshLoader

```rust
pub struct MeshLoader;

impl MeshLoader {
    pub fn new() -> Self;
}
```

`MeshLoader` 注册 `mesh` 扩展名，解析 `v`/`n`/`uv`/`t`/`i` 文本 mesh payload，并为 renderer
handoff 生成 `GpuUploadKind::Mesh`。无效 UTF-8、未知 directive、缺失/多余参数、非法数字、
空 vertex 列表、normal/uv/tangent 数量不匹配或越界 index 都会返回 `AssetError::Decode`。

---

## 27.3 ShaderLoader

```rust
pub struct ShaderLoader;

impl ShaderLoader {
    pub fn new() -> Self;
}
```

`ShaderLoader` 注册 `wgsl`、`glsl`、`shader` 扩展名。示例 payload：

```text
@fragment fn main() {}
```

默认 stage 为 fragment；路径 label `#vertex`、`#fragment`、`#compute` 会显式选择
对应 `ShaderStage`，其他 label 会返回 decode error。Loader 会填充 WGSL bind group reflection，
vertex stage 还会提取 `@location` vertex input reflection；括号/大括号不匹配或只写
`@group`/`@binding` 其中之一的资源声明会作为 decode error 返回。成功加载会为 renderer
handoff 生成 `GpuUploadKind::Shader`。

---

## 27.4 MaterialLoader

```rust
pub struct MaterialLoader;

impl MaterialLoader {
    pub fn new() -> Self;
}
```

`MaterialLoader` 注册 `material`、`mat` 扩展名，解析文档中的 material 文本 payload，
并在 `LoadContext` 中注册依赖：

```text
Material
  ├── Shader
  ├── Albedo Texture
  ├── Normal Texture
  └── MetallicRoughness Texture
```

成功加载会为 renderer handoff 生成 `GpuUploadKind::Material`。无效语法、非法数值和非法
bool 都会返回 decode error。

---

## 27.5 AudioLoader

```rust
pub struct AudioLoader;

impl AudioLoader {
    pub fn new() -> Self;
}
```

`AudioLoader` 注册 `audio`、`wav`、`ogg` 扩展名，解析文档中的 `NGA_AUDIO_V1`
文本 payload，也解析基础 RIFF/WAVE PCM16 和 IEEE-float32 payload。对于 Ogg 数据封包，当
`ogg` 页面头可识别为 Opus/Vorbis 标识头时，会返回 `AudioClip` 且 `streaming=true`，并在
`AudioSamples` 中使用 `Streaming` 占位符。未识别的 Ogg payload 会返回可见 decode error。
CPU-only `AudioClip` 仍适用于 `NGA_AUDIO_V1` 和 RIFF/WAVE。

---

## 27.6 AnimationLoader

```rust
pub struct AnimationLoader;

impl AnimationLoader {
    pub fn new() -> Self;
}
```

`AnimationLoader` 注册 `animation`、`anim` 扩展名，解析文档中的 `NGA_ANIMATION_V1`
文本 payload，并直接生成 CPU-only `AnimationClip`。

---

## 27.7 SkeletonLoader

```rust
pub struct SkeletonLoader;

impl SkeletonLoader {
    pub fn new() -> Self;
}
```

`SkeletonLoader` 注册 `skeleton`、`skel` 扩展名，解析文档中的 `NGA_SKELETON_V1`
文本 payload，并直接生成 CPU-only `Skeleton`。同一 skeleton 内 bone name 必须唯一，重复
bone name 会返回 decode error。

---

## 27.8 SceneLoader

```rust
pub struct SceneLoader;

impl SceneLoader {
    pub fn new() -> Self;
}
```

`SceneLoader` 注册 `scene` 扩展名，解析文档中的 `NGA_SCENE_V1` 文本 payload。
`dependency=<path>` 行和已知组件 schema 的 asset 字段会注册运行时依赖，场景本体在所有直接和传递依赖
Ready 后才会进入 Ready。Scene 当前是 CPU-only 资源，不会产生 GPU upload command。

---

## 27.9 PrefabLoader

```rust
pub struct PrefabLoader;

impl PrefabLoader {
    pub fn new() -> Self;
}
```

`PrefabLoader` 注册 `prefab` 扩展名，解析文档中的 `NGA_PREFAB_V1` 文本 payload。
`dependency=<path>` 行和已知组件 schema 的 asset 字段会注册运行时依赖，Prefab 本体在所有直接和传递依赖
Ready 后才会进入 Ready。Prefab 当前是 CPU-only 资源，不会产生 GPU upload command。

`AssetDatabase::register_builtin_importers()` 默认也会注册 `SceneImporter` 与 `PrefabImporter`，
它们会把 `NGA_SCENE_V1` / `NGA_PREFAB_V1` 源文档导入成 runtime 资源，并把显式依赖路径与
已知组件 asset 字段都写入 metadata dependencies。
`AssetDatabase::register_builtin_cookers()` 默认也会注册 `SceneCooker` 与 `PrefabCooker`，
它们会把 scene/prefab runtime 文档写入 cooked bundle，供 `SceneLoader` / `PrefabLoader`
在运行时直接加载。

---

## 27.10 FontLoader

```rust
pub struct FontLoader;

impl FontLoader {
    pub fn new() -> Self;
}
```

`FontLoader` 注册 `font` 扩展名，解析文档中的 `NGA_FONT_V1` bitmap 文本 payload，
并直接生成 CPU-only `Font`。

---

## 27.11 PhysicsMeshLoader

```rust
pub struct PhysicsMeshLoader;

impl PhysicsMeshLoader {
    pub fn new() -> Self;
}
```

`PhysicsMeshLoader` 注册 `physics`、`physicsmesh`、`pmesh` 扩展名，解析文档中的
`NGA_PHYSICS_MESH_V1` 文本 payload，并直接生成 CPU-only `PhysicsMesh`。

---

## 28. 内建 Importer 示例 API

## 28.1 TextureImporter

```rust
pub struct TextureImporter;

impl TextureImporter {
    pub fn new() -> Self;
}

impl AssetImporter for TextureImporter {
    fn name(&self) -> &'static str;
    fn version(&self) -> u32;
    fn extensions(&self) -> &[&'static str];

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError>;
}
```

`TextureImporter` 保持二进制 runtime texture payload 的 pass-through 兼容性；如果 source
以 `NGA_TEXTURE_SOURCE_V1` 开头，则会把文本文档转换成 `TextureLoader` 使用的 runtime
bytes（little-endian width/height + RGBA8 数据）：

```text
NGA_TEXTURE_SOURCE_V1
size=2x1
rgba=255,0,0,255;0,255,0,255
```

也可用 `width=`/`height=` 代替 `size=`；`rgba`/`pixels` 中的字节用逗号分隔，
分号可用于分隔像素。导入错误会包含 `TextureImporter`、source path 和 settings 上下文。
导入时同 id generated bytes 会写入 imported root，`cook_asset` 会优先使用该 imported
payload，因此 text source 可以经过 cook、bundle 和 runtime `AssetServer` 正常加载成
`Texture`。

---

## 28.2 MaterialImporter

```rust
pub struct MaterialImporter;

impl MaterialImporter {
    pub fn new() -> Self;
}
```

`MaterialImporter::version()` 当前为 `4`。`MaterialImporter` 会按运行时 material 文本格式验证并规范化 source，去掉空行和注释、
修剪 key/value 空白，并以稳定的 `key=value\n` 写入 imported/cooked 输入；同时会解析：

```text
shader=shaders/pbr.wgsl
texture.albedo=textures/albedo.texture
```

这些路径必须已存在于 import context 的 registry 快照中。导入成功后，material metadata
会保存 shader 和 texture 的 `AssetId` 依赖，dependency report 和 bundle manifest 会复用
同一组依赖。typed custom property 也会按 `MaterialLoader` 规则验证并保留为 canonical
runtime material bytes。缺失依赖、非 UTF-8 或无效 key/value 行会通过 `AssetError::Import` 返回，
并带有 importer、source path 和 settings 上下文。

---

## 28.3 ModelImporter

```rust
pub struct ModelImporter;

impl ModelImporter {
    pub fn new() -> Self;
}
```

`ModelImporter` 当前解析一个文档化的 manifest payload：

```text
NGA_MODEL_V1
mesh=Mesh0|v 0 0 0;v 1 0 0;v 0 1 0;i 0 1 2
material=Material/Hero|name=hero;shader=shaders/pbr.wgsl;texture.albedo=textures/albedo.texture;base_color=1,1,1,1
skeleton=Skeleton|bone=Root
animation=Animation/Idle|duration=1;ticks_per_second=60
physics_mesh=Collision|NGA_PHYSICS_MESH_V1;kind=trimesh;v 0 0 0;v 1 0 0;v 0 1 0;i 0 1 2
```

也支持 multiline block 形式；block 以 `end` 结束，`depends=` 可声明对同一 model 中其他
generated label 的依赖：

```text
NGA_MODEL_V1
mesh=Body
material=HeroMaterial
skin=Rig
skin_influence_limit=2
physics_mesh=Collision
---
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
physics_mesh=Collision
depends=mesh:Body
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1 0 0
v 0 1 0
i 0 1 2
end
skeleton=Rig
NGA_SKELETON_V1
bone=Root
end
animation=Walk
target_skeleton=Rig
NGA_ANIMATION_V1
duration=1
ticks_per_second=24
track=bone:Root
translation=0:0,0,0
rotation=0:0,0,0,1
scale=0:1,1,1
end
material=HeroMaterial
name=hero
shader=shaders/pbr.wgsl
base_color=0.8,0.7,0.6,1
end
```

Inline `mesh`、`material` 和 `physics_mesh` payload 中用 `;` 表示换行，导入后会生成如
`models/hero.Mesh0.mesh`、`models/hero.Material_Hero.material`、`models/hero.Collision.physics`
这样的 generated
runtime path，并在 metadata `labels` 中保留原始 label（例如 `Material/Hero`）。
生成路径会先把 label 中非 ASCII 字母数字、`_`、`-` 的字符 canonicalize 为 `_`；
如果两个不同 label 解析到同一个 generated path，导入会失败，避免覆盖 generated bytes 或写入歧义
metadata。
Material payload 会按 `MaterialImporter` 相同规则解析 shader/texture 依赖，并验证 runtime
material field 语法和值；这些依赖会写入主 model metadata 和 generated material metadata。Block `depends=` 依赖会写入对应 generated
metadata；例如 animation 可以依赖同 model 内的 skeleton，material 可以额外依赖 generated
mesh。`depends=` 可使用裸 label，也可使用 `mesh:<Label>`、`material:<Label>`、
`skeleton:<Label>`、`animation:<Label>`、`physics_mesh:<Label>` 声明期望的 generated 类型；这些
typed 前缀是保留语法，未知前缀或空 label（例如 `mesh:`）会作为无效 metadata 报错；带类型前缀的依赖会在导入
阶段验证目标 label 存在且类型匹配；同一 block 中由 `depends=`、`material=`/`materials=`、
`lod=`/`lods=`、`physics_mesh=`/`physics_meshes=` 或 typed target metadata 写入的 generated dependency label 不能重复。
这些逗号分隔 dependency list 也会拒绝空列表、尾随逗号或连续逗号导致的空 generated label。
Material block 可在 payload 前使用
`mesh=<MeshLabel>` 或 `target_mesh=<MeshLabel>` 声明同 model generated mesh target；该
metadata 会写入 generated material dependency，导入时会验证目标 label 是 mesh。该字段必须只出现一次，
并且只能命名一个 mesh label。Mesh block 还可在 payload 前使用 `material=<MaterialLabel>` 或
`materials=<MaterialLabelA>,<MaterialLabelB>` 声明同 model generated material binding；该绑定会写入
generated mesh metadata dependency，并验证目标 label 存在且类型是 material。Mesh block 还可使用
`lod=<MeshLabel>` 或 `lods=<MeshLabelA>,<MeshLabelB>` 声明同 model generated LOD mesh
binding；这些 binding 会写入 generated mesh-to-mesh metadata dependency，并验证目标 label 存在且类型是 mesh。
Mesh block 也可使用
`skin=<SkeletonLabel>` 或 `skeleton=<SkeletonLabel>` 声明同 model generated skeleton 的 skin
binding；该绑定会写入 generated mesh metadata dependency，导入时会验证目标 label 是 skeleton，
skeleton payload 可由 `SkeletonLoader` 解析、bone name 唯一且显式 inverse-bind 与累积 bind pose 匹配，
mesh payload 可由 `MeshLoader` 解析并包含 skin joint/weight attributes，且所有 `j`/`joints`
skin joint index 都小于目标 skeleton bone 数量，每个顶点的 skin weight 总和必须为正并在 `0.001`
容差内归一化到 `1.0`，且同一 vertex 中权重大于 `0.001` 的 active skin joint index 不能重复。
Mesh block 可在 skin metadata 旁声明 `max_skin_joints=<N>` 或
`skin_joint_limit=<N>`，用于记录该 mesh 允许的 skeleton palette 上限；该值必须是大于 0 的整数，
必须和 `skin=`/`skeleton=` 一起使用，并且目标 skeleton 的 bone 数量不能超过该上限。
Mesh block 还可在同一 skin binding 中声明 `skin_root=<BoneName>`、`root_bone=<BoneName>` 或
`skin_root_bone=<BoneName>`；该字段必须只出现一次、必须和 `skin=`/`skeleton=` 一起使用，并验证命名
bone 存在于目标 skeleton 中，且所有带有效权重的 skin joints 都落在该 bone 子树内。若目标
skeleton 有多个 root bone，skinned mesh 必须显式声明 `skin_root`、`root_bone` 或
`skin_root_bone`，以便导入阶段有确定的 skinning scope。
Mesh block 还可声明 `max_skin_influences=<N>` 或 `skin_influence_limit=<N>`，用于约束每个 vertex
允许的有效 skin weight 数量；该值必须是 `1..=4`，必须和 `skin=`/`skeleton=` 一起使用，并验证每个
vertex 中大于 `0.001` 的权重数量不超过该上限。
Mesh block 也可使用 `physics_mesh=<PhysicsMeshLabel>` 或
`physics_meshes=<PhysicsMeshLabelA>,<PhysicsMeshLabelB>` 声明同 model generated physics mesh
binding；这些 binding 会写入 generated mesh metadata dependency，并验证目标 label 存在且类型是
`physics_mesh`。
Physics_mesh block 可在 payload 前使用 `mesh=<MeshLabel>` 或
`target_mesh=<MeshLabel>` 声明同 model generated render mesh target；该 metadata 会写入 generated
physics mesh dependency，导入时会验证目标 label 存在且类型是 mesh。该字段必须只出现一次，并且只能命名一个 mesh label。
Animation block 可使用 `skeleton=<SkeletonLabel>` 或 `target_skeleton=<SkeletonLabel>` 声明
同 model generated skeleton target；该 metadata 会写入 generated animation dependency，
并验证目标 label 存在且类型是 skeleton。该字段必须只出现一次，并且只能命名一个 skeleton label。
即使 skeleton 或 animation block 没有被其他 block 引用，导入阶段也会用对应 loader 解析
`NGA_SKELETON_V1` 与 `NGA_ANIMATION_V1` payload，提前报告重复 bone、无 track、无效 keyframe
等 standalone generated payload 错误。
Physics_mesh block 使用 runtime `NGA_PHYSICS_MESH_V1` payload，导入时会用 `PhysicsMeshLoader`
相同解析器验证 payload，写入 generated physics mesh metadata，支持本地 `depends=` 与 typed
`depends=physics_mesh:<Label>` 依赖，并会在 `scale != 1.0` 时缩放 `v` 顶点。
导入阶段会在 generated path 稳定到 `AssetId` 后，把 mesh/material/skeleton/animation/physics_mesh
generated ids 也加入主 model metadata 依赖，并会把 generated metadata 中的临时 generated
dependency id 重映射到稳定 id，因此重复导入不会留下失效的 generated dependency。
如果 animation block 通过 `skeleton=`/`target_skeleton=` 或 `depends=` 依赖同 model 内的
skeleton，导入时会解析 `NGA_ANIMATION_V1` payload，并验证 `track=bone:<BoneName>` 与
`track=node:<NodeName>` 目标存在于 skeleton bone names 中，以及 `track=node_index:<u32>`
没有越过 skeleton bone 数量；同时复用 animation loader 对空 track、重复 target 和 keyframe time
的校验。
Scoped dependency report 可以从 model root 看到外部 shader/texture、generated subresource
边，以及 generated-to-generated 边；这些依赖会保留到 sidecar、dependency report 和 bundle
manifest。
同一个 importer 也注册 `.obj` extension；`.model` source 可用 `NGA_MODEL_OBJ_V1` 作为第一行
显式选择 OBJ 子集解析。OBJ 主 source 与 `.mtl` source 都会在解析前剥离行尾 `#` 注释，
因此 authoring 文件可在 `NGA_MODEL_OBJ_V1`、`mtllib`、`o`/`g`、`v`、`vt`、`vn`、`usemtl` 和 `f` 行后保留说明文字。
支持的 OBJ 指令包括 `o`/`g` object label、`v x y z [w]` 顶点、
`vt u [v] [w]` texture coordinate、`vn` normal、`f` face、`mtllib` material library marker，以及
`usemtl` material binding；`s` smoothing group 会被接受，且当 OBJ face 未显式引用 `vn` 时，
`s <group>` 会按共享 vertex/group 累积 face normal 生成 smoothed normals，`s off`（值大小写不敏感）/`s 0`
会生成 per-face flat normals。Face index
支持 `1`、`1/2`、`1/2/3`、`1//3` 写法，也支持 OBJ relative index（例如 `-1` 指向最近声明的
vertex/uv/normal），并会验证引用到的 vertex、`vt`、`vn` index 已存在。`v` 的可选第四个 homogeneous coordinate 必须 finite 且非 0，导入时会先将 xyz 除以该值，再进入 model scale/optimization 流程。`vt` 的可选 `v` 默认 0；可选 `w` 只能为 0，因为 runtime `Mesh` 当前只保存二维 UV，非零 `w` 会返回 import error。OBJ face
tuple 会展开为 runtime mesh 的唯一 `(position, uv, normal)` vertex；当 face 带 UV 时会派生
runtime `t x y z w` tangent。四边形或多边形按 fan triangulation 转成 runtime mesh
`i a b c` 三角形。每个使用到的 `usemtl` 会生成
`Material/<name>` material subresource；active `usemtl` 会跨后续 `o`/`g` label 保持，直到另一个 `usemtl` 改变 binding；只使用单个 material 的 OBJ object 保持原 object label，
包含多个 material assignment 的同一 object 会按 material 拆成 `<Object>.Material/<name>`
mesh subresources，每个 generated mesh metadata 只依赖对应 generated material id。`AssetDatabase` 导入 `.obj` / OBJ-header `.model` 时会把同目录及其子目录的
`.mtl` source 文件加入 import context；`mtllib` 会按 model source 所在目录解析相对路径，
路径必须是无 label、无 `..` 的相对 source path。若对应 `.mtl` 可用，`ModelImporter` 会读取
`newmtl`、`Kd`、`Ka`、`Ks`、`Ke`、`Tf`、`Ni`/`ior`、`illum`、`d [-halo]`/`Tr`、`Ns`、
`Pr`/`roughness`、`Pm`/`metallic`、`Ps`/`sheen`、`Pc`/`clearcoat`、
`Pcr`/`clearcoat_roughness`、`aniso`/`anisotropy`、`anisor`/`anisotropy_rotation`、`sharpness`、`map_aat`，
以及常见 texture map directive，并 canonicalize 为
generated material payload 中的 `base_color`、`emissive`、`roughness`、`metallic`、
`custom.ambient_color.vec3`、`custom.specular_color.vec3`、
`custom.transmission_filter.vec3`、`custom.index_of_refraction.float`、
`custom.illumination_model.int`、`custom.sharpness.float`、`custom.texture_antialias.bool`、Exocortex PBR extension 的 `custom.sheen.float`、
`custom.clearcoat.float`、`custom.clearcoat_roughness.float`、`custom.anisotropy.float`、
`custom.anisotropy_rotation.float` 和 `texture.<channel>`。`Ns` 会按
`1 - sqrt(clamp(Ns / 1000, 0, 1))` 映射为 roughness；`Pr`/`roughness` 会直接 clamp 到
`0..=1` 写入 roughness。`d`/`Tr` 产生的 alpha 小于 `1.0` 时会同时写入
`alpha_mode=blend`，确保 runtime `MaterialRenderState` 不会把透明 MTL material 当作 opaque。
`d -halo <alpha>`（`-halo` 选项大小写不敏感）会同样写入 alpha 和 `alpha_mode=blend`，并额外保留
`custom.dissolve_halo.bool=true` 供材质系统区分 halo dissolve 语义。
`map_aat on/off` 会保留为 `custom.texture_antialias.bool`（bool 值大小写不敏感），让上层材质系统可继续区分
Wavefront texture anti-aliasing hint。
`map_d` alpha texture 也会写入 `texture.alpha` 和 `alpha_mode=blend`，避免带 alpha map 的材质
在 runtime 仍保持 opaque。
`map_Tr` 作为 `map_d` 的透明度别名也会走同样的 `texture.alpha` + `alpha_mode=blend` 语义。
这些可读取的 `.mtl` context source hash 也会参与 OBJ model 的 database source hash，
所以 MTL-only changes 会通过 incremental scan 标记 model changed，而不是静默保持 unchanged。
Texture map path 会按 `.mtl` 文件所在目录解析，并走 material importer 相同的
registry-backed texture dependency 规则；未注册的 texture 会返回 visible import error。
Texture map directive 的 path extraction 会识别常见 MTL option：`-o`、`-s`、`-t`、
`-bm`、`-boost`、`-mm`、`-clamp`、`-blendu`、`-blendv`、`-cc`、`-colorspace`、`-imfchan`、`-texres`
和 `-type`，option 名按 ASCII 大小写不敏感匹配，并允许 option 出现在 path 前后；未知或缺少必要参数的 map option 会返回
visible import error。`-clamp on/off`（bool 值大小写不敏感）会进一步 canonicalize 为对应
`texture.<channel>.sampler.address=clamp_to_edge/repeat` runtime material payload；`-o`、
`-s`、`-t` 会写入 `texture.<channel>.transform.offset/scale/turbulence`，`-bm` 会写入
`texture.<channel>.bump_scale`，`-mm` 会写入 `texture.<channel>.color_remap`，`-imfchan`
会写入 `texture.<channel>.source_channel`（接受 `r/red`、`g/green`、`b/blue`、`m/matte`、`l/luminance`、`z/depth`，大小写不敏感），`-boost` 会写入 `texture.<channel>.boost`，
`-blendu`/`-blendv` 会写入 `texture.<channel>.blend_u/blend_v`（bool 值大小写不敏感），`-cc` 会写入
`texture.<channel>.color_correction`，`-colorspace` 会写入规范化的
`texture.<channel>.color_space`（`srgb`/`linear`/`non_color`/`raw`），`-type` 会写入 `texture.<channel>.projection`（接受 `flat`/`sphere`/`cube_top`/`cube_bottom`/`cube_front`/`cube_back`/`cube_left`/`cube_right`，大小写不敏感），
`-texres` 会写入 `texture.<channel>.texture_resolution`。
Texture map directive 名称按 ASCII 大小写不敏感匹配，输出仍使用规范化的 runtime texture channel。当前 map channel 映射为：`map_Kd -> texture.albedo`、`map_Bump`/`map_bump`/`bump`/`norm`/
`normal`/`map_Kn`/`map_kn`/`map_Normal`/`map_normal -> texture.normal`、
`map_Pr`/`map_Ns -> texture.roughness`、`map_Pm -> texture.metallic`、`map_Ke -> texture.emissive`、
`map_Tf -> texture.transmission_filter`、`map_Ni -> texture.index_of_refraction`、
`map_Ks -> texture.specular`、`map_Ka -> texture.occlusion`、`map_d -> texture.alpha` / `map_Tr -> texture.alpha`（并启用
`alpha_mode=blend`）、
`map_Ps`/`map_sheen -> texture.sheen`、`map_Pc`/`map_clearcoat -> texture.clearcoat`、
`map_Pcr`/`map_clearcoat_roughness -> texture.clearcoat_roughness`、
`map_aniso`/`map_anisotropy -> texture.anisotropy`、
`map_anisor`/`map_anisotropy_rotation -> texture.anisotropy_rotation`、
`disp`/`map_Disp`/`map_disp`/`map_displacement -> texture.displacement`、
`decal`/`map_decal -> texture.decal`、`refl`/`map_refl -> texture.reflection`。
不可用的 material library 会保留为 `# mtllib ...` provenance comment 和 `name=...`，以兼容
只暴露主 source 的 IO；已读取 library 中的无效数值或无效 texture path 会返回
`ModelImporter` import error。同一 `.mtl` 或多个 `mtllib` source 中重复的 `newmtl`
material 名称会返回带两个 source/line 位置的 visible import error，避免不确定覆盖。若所有
declared `mtllib` source 都可读取，`usemtl` 名称必须匹配其中一个 `newmtl`；只要存在不可读取
library，则继续生成 minimal material payload 作为兼容 fallback。

```text
MTLLIB prop.mtl
O Prop
V 0 0 0
V 1 0 0
V 1 1 0
V 0 1 0
VT 0 0
VT 1 0
VT 1 1
VT 0 1
VN 0 0 1
USEMTL Red
F 1/1/1 2/2/1 3/3/1 4/4/1
```

```text
# models/prop.mtl
NewMtl Red
MAP_KD -BOOST 1.5 -BLENDU OFF -BlendV ON -CC TRUE -TEXRES 1024 -S 2 3 -O 0.25 0.5 -T 0.01 0.02 0.03 textures/prop_albedo.texture
MAP_NORMAL -BM 0.3 -COLORSPACE Non-Color textures/prop_normal.texture
MAP_PR textures/prop_roughness.texture -Clamp True
map_Pm -MM 0 1 textures/prop_metallic.texture
map_Ke -TYPE Sphere -IMFCHAN R textures/prop_emissive.texture
kD 0.8 0.2 0.1
kE 0.1 0.2 0.3
D 0.75
nS 250
pM 0.5
```

上面的 `.obj` source 会生成 `models/prop.Prop.mesh` 和
`models/prop.Material_Red.material`，mesh payload 会包含四个 `v`、四个 `n`、四个 `uv`、
四个 generated `t` tangent，以及两个 triangulated indices：`i 0 1 2` 与 `i 0 2 3`；
material payload 会包含
`# mtllib prop.mtl` comment、`name=Red`、
`texture.albedo=models/textures/prop_albedo.texture`、`texture.albedo.transform.offset=0.25,0.5,0`、
`texture.albedo.transform.scale=2,3,1`、`texture.albedo.transform.turbulence=0.01,0.02,0.03`、
`texture.albedo.boost=1.5`、`texture.albedo.blend_u=false`、`texture.albedo.blend_v=true`、
`texture.albedo.color_correction=true`、`texture.albedo.texture_resolution=1024`、
`texture.normal=...`、`texture.normal.bump_scale=0.3`、
`texture.normal.color_space=non_color`、
`texture.roughness=...`、`texture.roughness.sampler.address=clamp_to_edge`、
`texture.metallic=...`、`texture.metallic.color_remap=0,1`、`texture.emissive=...`、
`texture.emissive.source_channel=red`、`texture.emissive.projection=sphere`、
`base_color=0.8,0.2,0.1,0.75`、`emissive=0.1,0.2,0.3`、`metallic=0.5` 和
`roughness=0.5`，generated material
metadata 会依赖已注册的 texture map assets。无效 face arity、非 finite/zero OBJ vertex homogeneous coordinate、非零 OBJ texture-coordinate `w`、index 为 0、越界正向或 relative
vertex/texture-coordinate/normal index、混合有/无 uv 或 normal 的 face vertex、空
object/material/mtllib label、无效 `mtllib` path、已读取 `.mtl` 中的无效 property、未注册的
material texture dependency、重复 `newmtl` material 名称、已完整读取 MTL 集合中缺失的 `usemtl`
material 名称、未知或格式错误的 material texture map option、生成 label 规范化后的
generated path 冲突，或未知 OBJ directive 会返回带 source path 的 `ModelImporter` import error。
无效 generated physics mesh payload、重复 generated dependency metadata、无效 generated dependency list syntax（未知 typed kind、空 typed label、空列表、尾随逗号或连续逗号）、
无效 typed generated dependency target（未知 label，或 `depends=mesh:<Label>` /
`depends=physics_mesh:<Label>` 等类型前缀与目标
generated asset 类型不匹配）、无效 material target mesh metadata（未知 mesh label、目标 label 不是 mesh、重复字段或多个 label）、无效 physics_mesh target mesh metadata（未知 mesh label、目标 label 不是 mesh、重复字段或多个 label）、无效 mesh LOD binding（未知 LOD mesh label，或目标 label 不是 mesh）、无效 mesh material binding（未知 material label，或目标 label 不是 material）、无效 mesh skin binding（未知 skeleton label、目标 skeleton payload 无法解析、包含重复 bone
name、或显式 inverse-bind 不匹配，mesh payload 无法解析、声明 skin 但没有 skin joint/weight attributes，或 joint index 越过
skeleton bone 数量，skin weight 总和为 0/未归一化、同一 vertex 有重复 active skin joint，或 `max_skin_joints`/`skin_joint_limit` 缺失 skin target、重复、为 0、或小于 skeleton bone 数量，或 `max_skin_influences`/`skin_influence_limit` 缺失 skin target、重复、为 0、超过 4、或 vertex 有效权重数量超过上限，或多 root skin skeleton 未显式声明 root scope，或 `skin_root`/`root_bone`/`skin_root_bone` 缺失 skin target、重复、命名缺失 bone、或带权重 joint 落在 root bone 子树外）、无效 standalone generated material/skeleton/animation payload、无效 mesh physics mesh binding（未知 physics mesh label，或目标 label 不是 physics_mesh）、无效 animation target skeleton metadata（未知 skeleton label、目标 label 不是 skeleton、重复字段或多个 label），以及依赖 skeleton 的 animation track 指向缺失 bone/node、越界 node index、空 track、重复 target，或 generated animation keyframe time 非 finite/为负/超过 duration/乱序，
也会返回带 source path 的 `ModelImporter` import error。
`ModelImporter::version()` 当前为 `66`，覆盖 manifest/OBJ 解析、OBJ relative index 解析、OBJ directive case-insensitive parsing, OBJ homogeneous vertex coordinate parsing、OBJ texture-coordinate non-zero `w` diagnostics、OBJ inline comment parsing、model mesh optimization degenerate-triangle culling、generated LOD degenerate-triangle culling、
generated path collision diagnostics、duplicate generated dependency metadata validation、generated dependency list syntax validation、typed generated dependency metadata validation、inline `label|payload` 语法校验、generated dependency remapping、OBJ material-map option preservation、material target mesh metadata、mesh material dependency metadata、skin skeleton
dependency metadata、mesh LOD binding metadata、mesh physics mesh binding metadata、physics mesh target mesh metadata、skin joint limit validation、skin influence limit validation、duplicate active skin joint validation、multi-root skin skeleton root-scope validation、skin root bone subtree validation、standalone generated material/skeleton/animation payload validation、explicit animation skeleton target metadata、model generated physics mesh subresource validation、skin binding validation、explicit inverse-bind validation、duplicate MTL material diagnostics、MTL emissive color canonicalization、MTL bool value case-insensitive canonicalization、MTL source-channel case-insensitive canonicalization、MTL projection case-insensitive canonicalization、MTL texture-map directive case-insensitive canonicalization, MTL texture option-name case-insensitive parsing, MTL material property directive case-insensitive parsing、OBJ `usemtl` state persistence across `o`/`g` labels、OBJ directive case-insensitive parsing, OBJ homogeneous vertex coordinate parsing、OBJ smoothing-group normal generation、OBJ smoothing off value case-insensitive flat-normal parsing、OBJ `d`/`Tr` alpha-to-`alpha_mode=blend` material state mapping、OBJ `map_d` alpha texture-to-`alpha_mode=blend` material state mapping、OBJ `d -halo` option case-insensitive dissolve-halo alpha/custom material state mapping、OBJ `sharpness` custom material property, `map_bump`/`map_Kn`/`map_normal` normal map alias mapping, `map_aat` texture antialias custom bool preservation, and `map_Tf`/`map_Ni` transmission/IOR texture dependency mapping、OBJ MTL `-colorspace` texture color-space metadata preservation、OBJ Exocortex PBR scalar/texture material extension mapping、OBJ `Pr`/`roughness` scalar roughness and `map_Ns` roughness texture mapping、OBJ legacy `disp`/`decal`/`refl` texture map dependencies、MTL `Ka`/`Ks`/`Tf`/`Ni`/`illum` typed custom material property case-insensitive preservation、model import settings filtering/scale/tangent/mesh optimization/LOD generation control including degenerate-triangle culling、OBJ per-material mesh splitting、loaded-MTL `usemtl` validation，以及 skeleton-dependent animation bone/node/index target、track shape 和 keyframe time validation。
`skeleton` 和 `animation` payload 使用对应 `NGA_SKELETON_V1` 与
`NGA_ANIMATION_V1` 文本格式，生成的 bytes 会写入 imported 目录，随后可通过内建
`SkeletonCooker` / `AnimationCooker` 验证 runtime payload 后烘焙；直接输入
`NGA_SKELETON_SOURCE_V1` / `NGA_ANIMATION_SOURCE_V1` source document 时会规范化为
runtime header（当前 cooker version 为 2），随后可由运行时 loader 加载。

---

## 28.4 AudioImporter

```rust
pub struct AudioImporter;

impl AudioImporter {
    pub fn new() -> Self;
}
```

`AudioImporter` 保持 runtime `NGA_AUDIO_V1` payload 和 `.wav` payload 的 pass-through
兼容性；如果 source 以 `NGA_AUDIO_SOURCE_V1` 开头，则会验证并规范化为
`AudioLoader` 使用的 runtime 文本：

```text
NGA_AUDIO_SOURCE_V1
sample_rate=48000
channels=2
format=f32
frames=0.0, 0.5; -0.5, 1.0
streaming=true
```

会输出：

```text
NGA_AUDIO_V1
sample_rate=48000
channels=2
format=f32
samples=0,0.5,-0.5,1
streaming=true
```

`samples=` 和 `frames=` 都可使用；逗号和分号都会作为 sample 分隔符。Importer 会验证
`sample_rate`、`channels`、`format`、sample 类型、finite `f32` sample、以及 sample 数量是否为
channel 数的非零倍数。`ImporterSettings` 可为 `NGA_AUDIO_SOURCE_V1` 转换设置
`force_mono=true`、`normalize=true`、`streaming=true/false` 和 `compression`：`force_mono`
会先按 frame 平均多声道采样并输出 `channels=1`，`normalize` 随后按峰值绝对振幅缩放
`i16`/`f32` samples，`streaming` 会覆盖 source 文本中的同名字段，`compression` 支持
`none`、`vorbis`、`opus`；当值为 `vorbis`/`opus` 且输入为 Ogg 封装时保留编码载荷并在
runtime 阶段解析为 `AudioSamples::Streaming`。非法 boolean/压缩值设置会作为 import
error 返回。导入错误会包含 `AudioImporter`、source path 和 settings 上下文。转换后的
bytes 会写入 imported root，后续 `cook_asset`、bundle 和 runtime load 都使用该规范化 payload。

---

## 28.5 FontImporter

```rust
pub struct FontImporter;

impl FontImporter {
    pub fn new() -> Self;
}
```

`FontImporter` 保持 runtime `NGA_FONT_V1` bitmap font payload 的 pass-through 兼容性；如果
source 以 `NGA_FONT_SOURCE_V1` 开头，则会验证并规范化为 `FontLoader` 使用的 runtime 文本：

```text
NGA_FONT_SOURCE_V1
family = Debug Sans
glyph = char=B; size=1x1; bitmap=128
glyph=char=A;size=2x1;bitmap=0, 255
```

会输出稳定排序和修剪后的 runtime payload：

```text
NGA_FONT_V1
family=Debug Sans
glyph=char=A;size=2x1;bitmap=0,255
glyph=char=B;size=1x1;bitmap=128
```

Importer 会验证 `family`、至少一个 glyph、单字符 `char`、非零 `size`、bitmap 字节值和
`width * height` 字节数，并拒绝重复 family 或重复 glyph。导入错误会包含
`FontImporter`、source path 和 settings 上下文。转换后的 bytes 会写入 imported root，
后续 `cook_asset`、bundle 和 runtime load 都使用该规范化 payload。

---

## 28.6 PhysicsMeshImporter

```rust
pub struct PhysicsMeshImporter;

impl PhysicsMeshImporter {
    pub fn new() -> Self;
}
```

`PhysicsMeshImporter` 保持 runtime `NGA_PHYSICS_MESH_V1` payload 的 pass-through 兼容性；
如果 source 以 `NGA_PHYSICS_MESH_SOURCE_V1` 开头，则会验证并规范化为
`PhysicsMeshLoader` 使用的 runtime 文本：

```text
NGA_PHYSICS_MESH_SOURCE_V1
kind = tri_mesh
vertex = 0.0, 0.0, 0.0
v 1.50 0 0
v 0 1 0
triangle = 0, 1, 2
```

会输出：

```text
NGA_PHYSICS_MESH_V1
kind=trimesh
v 0 0 0
v 1.5 0 0
v 0 1 0
i 0 1 2
```

Importer 会规范化 `kind`（`tri_mesh` -> `trimesh`、`convex_hull` -> `convex`、
`height_field` -> `heightfield`），接受 `vertex=`/`triangle=` 或 `v`/`i` directive，
验证有限 f32 顶点、u32 三角索引、非空顶点，以及非 convex mesh 的非空三角形。索引越界、
未知字段或格式错误会通过 `AssetError::Import` 返回，并带有 `PhysicsMeshImporter`、
source path 和 settings 上下文。转换后的 bytes 会写入 imported root，后续 `cook_asset`、
bundle 和 runtime load 都使用该规范化 payload。

---

## 28.7 MeshImporter

```rust
pub struct MeshImporter;

impl MeshImporter {
    pub fn new() -> Self;
}
```

`MeshImporter` 保持 runtime mesh 文本 payload 的 pass-through 兼容性；如果 source 以
`NGA_MESH_BINARY_V1\n` 开头，则会按 `MeshLoader` 的二进制 runtime 格式完整验证后原样写入
imported root。
如果 source 以
`NGA_MESH_SOURCE_V1` 开头，则会验证并规范化为 `MeshLoader` 使用的
`v`/`n`/`uv`/`uv1+`/`t`/`j`/`w`/`i` 文本：

```text
NGA_MESH_SOURCE_V1
vertex = 0.0, 0.0, 0.0
v 1.50 0 0
v 0 1 0
normal = 0, 0, 1
n 0 0 1
n 0 0 1
uv = 0, 0
uv 1 0
uv 0 1
uv1 = 0.25, 0.25
uv1 0.75 0.25
uv1 0.25 0.75
tangent = 1, 0, 0, 1
t 1 0 0 1
t 1 0 0 1
joint = 0, 1, 2, 3
j 0 0 0 0
joints 1 2 3 4
weight = 0.7, 0.2, 0.1, 0
w 1 0 0 0
weights 0.25 0.25 0.25 0.25
triangle = 0, 1, 2
```

会输出：

```text
v 0 0 0
v 1.5 0 0
v 0 1 0
n 0 0 1
n 0 0 1
n 0 0 1
uv 0 0
uv 1 0
uv 0 1
uv1 0.25 0.25
uv1 0.75 0.25
uv1 0.25 0.75
t 1 0 0 1
t 1 0 0 1
t 1 0 0 1
j 0 1 2 3
j 0 0 0 0
j 1 2 3 4
w 0.7 0.2 0.1 0
w 1 0 0 0
w 0.25 0.25 0.25 0.25
i 0 1 2
```

Importer 接受 `vertex=`/`normal=`/`uv=`/`uv1=`/`tangent=`/`joint=`/`weight=`/`triangle=`
或 `v`/`n`/`uv`/`uv1+`/`t`/`j`/`w`/`i` directive，验证有限 f32
顶点/normal/uv/tangent/weight、非负且总和为正并在 `0.001` 容差内归一化到 `1.0` 的 skin weight、`u16` joint index、attribute 数量、
skin joint/weight 数量、`u32` 三角索引、非空顶点和索引范围。转换后的 bytes 会写入
imported root；`MeshCooker` 会在 cook 阶段解码 imported text 或 binary mesh bytes，并输出
deterministic `NGA_MESH_BINARY_V1` cooked payload，bundle 和 runtime load 会使用这份 cooked
binary bytes。无效
source 或无效 binary payload 会通过 `AssetError::Import` 返回，并带有 `MeshImporter`、
source path 和 settings 上下文。`MeshImporter::version()` 当前为 `4`，覆盖文本 source
canonicalization、binary runtime payload validation、skin weight 归一化校验和 pass-through 行为。

`MeshCooker::version()` 当前为 `4`。它要求非空 source bytes，使用与 `MeshLoader` 相同的 mesh
decode 验证路径接受 imported text 或 `NGA_MESH_BINARY_V1` payload，再按上述 binary header 和数据顺序
重新编码 cooked bytes，并用 cooked bytes 计算 `content_hash`。`Windows`/`MacOs`/`Linux`
target 默认写入 `u32` indices；`Android`/`Ios`/`Web` target 会先对被 index 引用的 vertex 做精确
attribute tuple compaction/deduplication（保留 position/normal/UV/tangent/skinning 语义并移除未引用
vertex），再在所有 index 均不超过 `u16::MAX` 时设置 `16 = u16 indices` 并写入 `u16`
index block，否则保留 `u32` indices。运行时加载这些 cooked `u16` mesh 时会保留
`MeshIndexFormat::Uint16` 并向 renderer handoff 16-bit index bytes，而不是在 GPU 上传阶段扩回
32-bit index buffer。

---

## 28.8 ShaderImporter

```rust
pub struct ShaderImporter;

impl ShaderImporter {
    pub fn new() -> Self;
}
```

`ShaderImporter` 保持现有 WGSL/GLSL-like runtime source 的 pass-through 兼容性；如果 source
以 `NGA_SHADER_SOURCE_V1` 开头，则会验证并规范化为 `ShaderLoader` 使用的纯 shader source：

```text
NGA_SHADER_SOURCE_V1
language=wgsl
stage=fragment
---
  @fragment fn main() {}
```

会输出：

```text
@fragment fn main() {}
```

`language` 当前支持 `wgsl`、`glsl` 和 `spv`。`source=` 可用于单行 body，也可用 `---` 后的多行 body；文档键名不区分大小写，`language`、`stage`、`entry`、`source` 最多各出现一次；重复声明会返回导入错误。
`stage` 和 `entry` 作为导入侧描述字段保留校验入口：`stage` 必须是
`vertex`/`fragment`/`compute`，`entry` 必须是 ASCII 标识符；运行时 stage 仍由 path label
`#vertex`/`#fragment`/`#compute` 决定。`ShaderImporter::version()` 当前为 `3`。Importer 会拒绝缺失 language、缺失 body、空 body、
无效 stage/entry、重复 body 或未知字段；错误会包含 `ShaderImporter`、source path 和 settings 上下文。
转换后的 bytes 会写入 imported root，后续 `cook_asset`、bundle 和 runtime load 都使用该
规范化 shader source。

---

## 29. 资源生命周期

## 29.1 加载生命周期

```text
Unloaded
  │ load()
  ▼
Queued
  │ scheduler pop
  ▼
LoadingBytes
  │ IO complete
  ▼
DecodingCpu
  │ decode complete
  ▼
WaitingForDependencies
  │ dependencies ready
  ▼
LoadedCpu
  │ queue GPU upload if needed
  ▼
UploadingGpu
  │ renderer returns upload result
  ▼
Ready
```

失败路径：

```text
LoadingBytes / DecodingCpu / WaitingForDependencies / UploadingGpu
  │
  ▼
Failed
```

热重载路径：

```text
Ready
  │ file changed
  ▼
Reloading
  │ reload complete
  ▼
Ready
```

卸载路径：

```text
Ready / Failed
  │ strong_count == 0 && dependency_ref_count == 0
  ▼
Unloading
  │ CPU/GPU resource destroyed
  ▼
Unloaded
```

---

## 30. 使用示例

## 30.1 初始化 AssetServer

```rust
use engine_asset::prelude::*;

let mut assets = AssetServer::new(AssetServerConfig {
    root: "assets/source".into(),
    cooked_root: "assets/cooked".into(),
    enable_hot_reload: true,
    enable_async_loading: true,
    ..Default::default()
});

assets.register_builtin_asset_types();
assets.register_builtin_loaders();
```

---

## 30.2 加载纹理

```rust
let hero_albedo: Handle<Texture> = assets.load("textures/hero_albedo.texture");

assets.update(frame_index);

if assets.is_ready(&hero_albedo) {
    let texture = assets.get(&hero_albedo).unwrap();
    println!("texture: {}x{}", texture.width, texture.height);
}
```

---

## 30.3 加载模型子资源

```rust
let hero_mesh: Handle<Mesh> = assets.load("models/hero.model#Mesh0");
let hero_run: Handle<AnimationClip> = assets.load("models/hero.model#Animation/Run");
```

---

## 30.4 加载材质并等待依赖

```rust
let material: Handle<Material> = assets.load("materials/hero.material");

if assets.is_ready_with_dependencies(&material) {
    let mat = assets.get(&material).unwrap();
    // shader 和 texture 依赖也已 Ready
}
```

---

## 30.5 批量加载场景资源

```rust
let group = assets.load_group(&[
    AssetPath::parse("scenes/forest.scene"),
    AssetPath::parse("ui/hud.prefab"),
    AssetPath::parse("audio/forest_ambience.audio"),
]);

loop {
    assets.update(frame_index);

    let progress = assets.group_progress(&group);
    println!("loading {}/{}", progress.ready_assets, progress.total_assets);

    if assets.group_state(&group) == AssetLoadState::Ready {
        break;
    }
}
```

---

## 30.6 处理事件

```rust
let mut cursor = AssetEventCursor::default();

for event in assets.events_since(&mut cursor) {
    match event {
        AssetEvent::Ready { id } => {
            println!("asset ready: {:?}", id);
        }
        AssetEvent::Failed { id, error } => {
            eprintln!("asset failed: {:?}: {:?}", id, error);
        }
        AssetEvent::Reloaded { id } => {
            println!("asset reloaded: {:?}", id);
        }
        _ => {}
    }
}
```

---

## 30.7 GPU 上传集成

```rust
fn render_prepare_system(assets: &mut AssetServer, renderer: &mut Renderer) {
    let commands: Vec<_> = assets.drain_gpu_uploads().collect();
    let mut results = Vec::new();

    for command in commands {
        let result = renderer.execute_upload(command);
        results.push(result);
    }

    assets.finish_gpu_uploads(results);
}
```

---

## 30.8 编辑器导入

```rust
let mut database = AssetDatabase::new(AssetDatabaseConfig {
    source_root: "assets/source".into(),
    imported_root: "assets/imported".into(),
    cooked_root: "assets/cooked".into(),
    registry_path: "assets/asset_registry.ron".into(),
});

database.register_importer(TextureImporter::new());
database.register_importer(ModelImporter::new());
database.register_importer(AudioImporter::new());

database.scan()?;
let id = database.import_asset_path(&AssetPath::parse("textures/hero_albedo.png"))?;
database.cook_asset(id, TargetPlatform::Windows)?;
database.save_registry()?;
```

---

## 30.9 构建 Bundle

```rust
let build = AssetDatabaseBundleBuild::new(
    "level_01",
    vec![level_01_scene_id],
).with_compression(CompressionKind::None);

let output = database.build_bundle(&build)?;
std::fs::write("assets/bundles/level_01.bundle", &output.bytes)?;
```

---

## 30.10 运行时挂载 Bundle

```rust
let bytes = std::fs::read("assets/bundles/level_01.bundle")?;
let bundle = assets.mount_bundle_bytes(&bytes)?;
let load = assets.preload_bundle(&bundle);

while assets.group_state(&load) != AssetLoadState::Ready {
    assets.update(frame_index);
}
```

Bundle 也可以直接作为流式区域来源：

```rust
let region = assets.register_streaming_region_bundle(
    "level_01",
    LoadPriority::High,
    bundle.id,
)?;
let load = assets.preload_streaming_region(region)?;
```

---

## 31. 推荐 Feature Flags

```toml
[features]
default = [
    "filesystem",
    "serde",
    "bundle",
    "hot_reload",
    "streaming",
    "editor",
    "importers",
    "cookers",
    "zstd",
]

filesystem = []
bundle = []
hot_reload = []
streaming = []
serde = ["dep:serde"]
async_loading = []
editor = ["importers", "cookers"]
importers = [
    "texture_importer",
    "model_importer",
    "material_importer",
    "audio_importer",
    "shader_importer",
]
cookers = [
    "texture_cooker",
    "model_cooker",
    "material_cooker",
    "audio_cooker",
    "shader_cooker",
]
texture_importer = []
model_importer = []
material_importer = []
audio_importer = []
shader_importer = []
texture_cooker = []
model_cooker = []
material_cooker = []
audio_cooker = []
shader_cooker = []
parallel = []
zstd = ["dep:ruzstd"]
```

`filesystem` 关闭时不会移除 `FileSystemAssetIo` 类型，但会禁用实际文件系统访问并返回可见
IO 错误；`MemoryAssetIo` 保持可用，用于无 filesystem feature 的测试和嵌入式场景。

运行时可以查询和要求特定能力：

```rust
pub enum AssetFeature {
    Bundle,
    HotReload,
    Streaming,
    Importers,
    Cookers,
    TextureImporter,
    ModelImporter,
    MaterialImporter,
    AudioImporter,
    ShaderImporter,
    TextureCooker,
    ModelCooker,
    MaterialCooker,
    AudioCooker,
    ShaderCooker,
    AsyncLoading,
    Parallel,
    Serde,
    Zstd,
    // ...
}

pub struct AssetFeatureStatus {
    pub feature: AssetFeature,
    pub name: &'static str,
    pub enabled: bool,
}

pub fn asset_feature_status(feature: AssetFeature) -> AssetFeatureStatus;
pub fn asset_feature_enabled(feature: AssetFeature) -> bool;
pub fn require_asset_feature(feature: AssetFeature) -> Result<(), AssetError>;
```

当 `bundle`、`hot_reload`、`streaming`、`importers`、`cookers` 或 `zstd` 被关闭时，
对应的 fallible 入口会返回 `AssetError::Unsupported`，而不是继续执行不完整路径。
当前实现还会用 `cfg` 裁剪内建 importer/cooker 的具体实现和注册路径；`bundle` 关闭时，
`AssetDatabase::build_bundle` 保留公开入口，但内部 bundle 构建实现不会编译进来，并会直接返回
`AssetError::Unsupported`。`AssetServer` 也会在关闭对应 feature 时裁剪 bundle registry/reader、
hot-reload queue/watch、streaming region/residency 等运行时状态；对应 fallible 入口保留为
`Unsupported` stub，便于无 feature 构建在公共 API 边界得到可见错误。
`async_loading` 与 `parallel` 通过 `AssetLoadingPolicyReport` 暴露启用状态和配置诊断：
未启用 feature 时请求相关能力会得到 `Unsupported`，启用 `async_loading` 时报告
`WorkerAsync` 模式；启用 `parallel` 时 `worker_threads > 1` 会成为 async loading 的有效
in-flight worker 上限。启用 `async_loading` 时，`AssetServer::async_worker_pool_report` 和
`shutdown_async_worker_pool` 可观察可复用 worker pool 的存活 worker、in-flight job、累计
dispatch/complete、线程启动数和显式 shutdown 次数。
`zstd` feature 是默认 feature 的一部分；关闭它时 `CompressionKind::Zstd` 仍可在 manifest
中被解析，但写入或读取 zstd chunk 会通过 codec report 和 `AssetError` 暴露禁用原因。

---

## 32. 推荐文件结构

```text
src/
  asset/
    mod.rs
    prelude.rs

    id.rs
    path.rs
    asset.rs
    handle.rs
    ref_asset.rs
    storage.rs
    server.rs
    config.rs
    registry.rs
    metadata.rs
    dependency.rs
    events.rs
    error.rs

    io/
      mod.rs
      filesystem.rs
      bundle_io.rs
      composite.rs
      memory.rs

    loader/
      mod.rs
      registry.rs
      context.rs
      texture_loader.rs
      mesh_loader.rs
      material_loader.rs
      shader_loader.rs
      audio_loader.rs
      scene_loader.rs

    importer/
      mod.rs
      registry.rs
      context.rs
      texture_importer.rs
      model_importer.rs
      audio_importer.rs
      shader_importer.rs
      material_importer.rs

    cooker/
      mod.rs
      context.rs
      texture_cooker.rs
      mesh_cooker.rs
      bundle_cooker.rs

    bundle/
      mod.rs
      manifest.rs
      builder.rs
      reader.rs
      writer.rs

    gpu_upload.rs
    hot_reload.rs
    gc.rs
    streaming.rs

    ecs/
      mod.rs
      components.rs
      systems.rs

    assets/
      mod.rs
      texture.rs
      mesh.rs
      material.rs
      shader.rs
      audio.rs
      animation.rs
      skeleton.rs
      scene.rs
      prefab.rs
      font.rs
      physics_mesh.rs
```

---

## 33. MVP 实现顺序

第一阶段，最小可运行闭环：

```text
AssetId
AssetPath
Asset trait
Handle<T>
UntypedHandle
Assets<T>
AssetServer::load
AssetServer::get
AssetLoadState
AssetEvent
FileSystemAssetIo
AssetLoader
TextureLoader
MeshLoader
MaterialLoader
GpuUploadQueue
```

目标：

```text
load texture
load mesh
load material with dependencies
GPU upload
get resource by Handle<T>
```

第二阶段：

```text
AssetMetadata
AssetRegistry
DependencyGraph
AssetDatabase
Importer
Cooker
.meta 文件
热重载
GC
```

第三阶段：

```text
Bundle
StreamingRegion
资源组加载
内存预算
DLC / Patch / Mod 覆盖
资源审计工具
依赖可视化
```

---

## 34. 关键边界

Asset 系统负责：

```text
资源身份
资源路径解析
资源加载
资源依赖
资源状态
资源事件
资源缓存
资源热重载
资源打包
资源卸载
资源内存统计
```

Asset 系统不负责：

```text
渲染 draw call
音频播放
场景实体实例化的具体 ECS 细节
动画状态机
物理模拟
网络同步协议
玩法逻辑
```

它只提供资源。至于资源被谁使用、怎么播放、怎么渲染、怎么实例化，是渲染、音频、场景、动画和 gameplay 系统的工作。

---

## 35. 最终推荐 API 核心

如果只保留最核心的外部 API，应该是这组：

```rust
impl AssetServer {
    pub fn new(config: AssetServerConfig) -> Self;

    pub fn register_asset_type<T: Asset>(&mut self);
    pub fn register_loader<L: AssetLoader>(&mut self, loader: L);

    pub fn load<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T>;
    pub fn load_by_id<T: Asset>(&mut self, id: AssetId) -> Handle<T>;

    pub fn get<T: Asset>(&self, handle: &Handle<T>) -> Option<&T>;
    pub fn get_mut<T: Asset>(&mut self, handle: &Handle<T>) -> Option<&mut T>;

    pub fn state<T: Asset>(&self, handle: &Handle<T>) -> AssetLoadState;
    pub fn is_ready<T: Asset>(&self, handle: &Handle<T>) -> bool;
    pub fn is_ready_with_dependencies<T: Asset>(&self, handle: &Handle<T>) -> bool;

    pub fn reload<T: Asset>(&mut self, handle: &Handle<T>) -> Result<(), AssetError>;
    pub fn unload<T: Asset>(&mut self, handle: Handle<T>);

    pub fn update(&mut self, frame_index: u64);

    pub fn events(&self) -> &[AssetEvent];
    pub fn drain_events(&mut self) -> impl Iterator<Item = AssetEvent> + '_;

    pub fn drain_gpu_uploads(&mut self) -> impl Iterator<Item = GpuUploadCommand> + '_;
    pub fn finish_gpu_uploads(&mut self, results: impl IntoIterator<Item = GpuUploadResult>);
}
```

这一组就是资源系统的脊梁。其他 Importer、Cooker、Bundle、Streaming、HotReload、GC 都是围绕它长出来的器官。

---

## 36. 总结

`engine_asset` 的核心模型：

```text
AssetId     ：稳定身份
AssetPath   ：路径入口
Handle<T>   ：类型安全引用
Assets<T>   ：类型化存储
AssetServer ：运行时主入口
AssetLoader ：运行时加载器
AssetDatabase：编辑器资源数据库
AssetImporter：源资源导入器
AssetCooker ：平台资源烘焙器
AssetRegistry：资源索引
DependencyGraph：资源依赖图
Bundle      ：资源包
GpuUploadQueue：GPU 上传桥梁
GC          ：资源卸载与内存预算
```

一句话：

**Asset 管理系统是游戏引擎的资源港口，它不只读文件，而是给每个资源发身份证、查族谱、排船期、管仓库、修航线、补货、卸货，还要保证码头不被纹理箱子堵死。**

