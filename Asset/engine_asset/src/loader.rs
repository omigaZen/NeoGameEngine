use std::{any::Any, collections::HashMap, sync::Arc};

use crate::{
    asset::{Asset, AssetDependencies, AssetDependencyReference},
    error::{AssetError, AssetLoadError},
    gpu_upload::{GpuUploadCommand, GpuUploadKind, GpuUploadMetadata},
    handle::{Handle, UntypedHandle},
    id::{AssetId, AssetTypeId},
    path::AssetPath,
    registry::AssetRegistry,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoadPriority {
    Immediate,
    High,
    Normal,
    Low,
    Background,
}

impl Default for LoadPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Debug)]
pub struct LoadRequest {
    pub id: AssetId,
    pub path: Option<AssetPath>,
    pub asset_type: AssetTypeId,
    pub priority: LoadPriority,
    pub recursive_dependencies: bool,
    pub reload: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LoadScheduler {
    queue: Vec<LoadRequest>,
}

impl LoadScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, request: LoadRequest) {
        if self.queue.iter().any(|queued| queued.id == request.id) {
            return;
        }
        self.queue.push(request);
    }

    pub fn cancel(&mut self, id: AssetId) -> Option<LoadRequest> {
        let index = self.queue.iter().position(|request| request.id == id)?;
        Some(self.queue.remove(index))
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.queue.iter().any(|request| request.id == id)
    }

    pub fn pop_next(&mut self) -> Option<LoadRequest> {
        let (index, _) = self
            .queue
            .iter()
            .enumerate()
            .min_by_key(|(_, request)| priority_rank(request.priority))?;
        Some(self.queue.remove(index))
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

fn priority_rank(priority: LoadPriority) -> u8 {
    match priority {
        LoadPriority::Immediate => 0,
        LoadPriority::High => 1,
        LoadPriority::Normal => 2,
        LoadPriority::Low => 3,
        LoadPriority::Background => 4,
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoaderSettings {
    values: HashMap<String, String>,
}

impl LoaderSettings {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadDependency {
    pub id: AssetId,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,
}

pub struct LoadContext<'a> {
    id: AssetId,
    path: AssetPath,
    registry: &'a mut AssetRegistry,
    dependencies: Vec<LoadDependency>,
    subresources: Vec<(AssetPath, AssetTypeId, AssetId)>,
}

impl<'a> LoadContext<'a> {
    pub fn new(id: AssetId, path: AssetPath, registry: &'a mut AssetRegistry) -> Self {
        Self {
            id,
            path,
            registry,
            dependencies: Vec::new(),
            subresources: Vec::new(),
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn dependency<T: Asset>(&mut self, path: impl Into<AssetPath>) -> Handle<T> {
        let path = path.into();
        let id = self.registry.get_or_create(path.clone(), T::TYPE_ID);
        self.dependencies.push(LoadDependency {
            id,
            path,
            asset_type: T::TYPE_ID,
        });
        Handle::weak(id)
    }

    pub fn add_dependency(&mut self, path: AssetPath, asset_type: AssetTypeId) -> AssetId {
        let id = self.registry.get_or_create(path.clone(), asset_type);
        self.dependencies.push(LoadDependency {
            id,
            path,
            asset_type,
        });
        id
    }

    pub fn add_subresource(&mut self, path: AssetPath, asset_type: AssetTypeId) -> AssetId {
        let id = self.registry.get_or_create(path.clone(), asset_type);
        self.subresources.push((path, asset_type, id));
        id
    }

    pub(crate) fn finish(self) -> (Vec<LoadDependency>, Vec<(AssetPath, AssetTypeId, AssetId)>) {
        (self.dependencies, self.subresources)
    }
}

pub struct LoadedAsset {
    pub asset_type: AssetTypeId,
    pub asset: Box<dyn Any + Send + Sync>,
    pub gpu_upload: Option<GpuUploadCommand>,
    pub asset_dependencies: Vec<AssetDependencyReference>,
}

impl LoadedAsset {
    pub fn new<T: Asset>(asset: T) -> Self {
        Self {
            asset_type: T::TYPE_ID,
            asset: Box::new(asset),
            gpu_upload: None,
            asset_dependencies: Vec::new(),
        }
    }

    pub fn new_with_asset_dependencies<T: Asset + AssetDependencies>(asset: T) -> Self {
        let mut asset_dependencies = Vec::new();
        asset.visit_dependencies(&mut |dependency| {
            if !asset_dependencies
                .iter()
                .any(|existing: &AssetDependencyReference| existing.id() == dependency.id())
            {
                asset_dependencies.push(dependency);
            }
        });
        Self {
            asset_type: T::TYPE_ID,
            asset: Box::new(asset),
            gpu_upload: None,
            asset_dependencies,
        }
    }

    pub fn with_gpu_upload(mut self, upload: GpuUploadCommand) -> Self {
        self.gpu_upload = Some(upload);
        self
    }

    pub fn with_dependency_handles(mut self, dependencies: Vec<UntypedHandle>) -> Self {
        for dependency in dependencies {
            let dependency = AssetDependencyReference::from_handle(dependency);
            if !self
                .asset_dependencies
                .iter()
                .any(|existing| existing.id() == dependency.id())
            {
                self.asset_dependencies.push(dependency);
            }
        }
        self
    }

    pub fn with_dependency_refs(mut self, dependencies: Vec<AssetDependencyReference>) -> Self {
        for dependency in dependencies {
            if !self
                .asset_dependencies
                .iter()
                .any(|existing| existing.id() == dependency.id())
            {
                self.asset_dependencies.push(dependency);
            }
        }
        self
    }

    pub fn texture_upload(
        mut self,
        id: AssetId,
        asset_type: AssetTypeId,
        label: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        self.gpu_upload = Some(GpuUploadCommand {
            id,
            asset_type,
            kind: GpuUploadKind::Texture,
            label,
            metadata: GpuUploadMetadata::None,
            bytes,
        });
        self
    }

    pub fn mesh_upload(
        mut self,
        id: AssetId,
        asset_type: AssetTypeId,
        label: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        self.gpu_upload = Some(GpuUploadCommand {
            id,
            asset_type,
            kind: GpuUploadKind::Mesh,
            label,
            metadata: GpuUploadMetadata::None,
            bytes,
        });
        self
    }

    pub fn mesh_upload_with_metadata(
        mut self,
        id: AssetId,
        asset_type: AssetTypeId,
        label: Option<String>,
        metadata: GpuUploadMetadata,
        bytes: Vec<u8>,
    ) -> Self {
        self.gpu_upload = Some(GpuUploadCommand {
            id,
            asset_type,
            kind: GpuUploadKind::Mesh,
            label,
            metadata,
            bytes,
        });
        self
    }

    pub fn shader_upload(
        mut self,
        id: AssetId,
        asset_type: AssetTypeId,
        label: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        self.gpu_upload = Some(GpuUploadCommand {
            id,
            asset_type,
            kind: GpuUploadKind::Shader,
            label,
            metadata: GpuUploadMetadata::None,
            bytes,
        });
        self
    }
}

pub trait AssetLoader: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &[&'static str];
    fn asset_type(&self) -> AssetTypeId;

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError>;
}

#[derive(Clone, Default)]
pub struct AssetLoaderRegistry {
    loaders: Vec<Arc<dyn AssetLoader>>,
}

impl AssetLoaderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<L: AssetLoader>(&mut self, loader: L) {
        self.loaders.push(Arc::new(loader));
    }

    pub fn register_boxed(&mut self, loader: Box<dyn AssetLoader>) {
        self.loaders.push(Arc::from(loader));
    }

    pub fn loader_for_extension(&self, extension: &str) -> Option<Arc<dyn AssetLoader>> {
        self.loaders
            .iter()
            .find(|loader| {
                loader
                    .extensions()
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            })
            .cloned()
    }

    pub fn loader_for_type(&self, asset_type: AssetTypeId) -> Option<Arc<dyn AssetLoader>> {
        self.loaders
            .iter()
            .find(|loader| loader.asset_type() == asset_type)
            .cloned()
    }

    pub fn loader_for_path_and_type(
        &self,
        path: Option<&AssetPath>,
        asset_type: AssetTypeId,
    ) -> Result<Arc<dyn AssetLoader>, AssetError> {
        if let Some(path) = path {
            if let Some(extension) = path.extension() {
                if let Some(loader) = self.loader_for_extension(extension) {
                    if loader.asset_type() == asset_type || asset_type == AssetTypeId::NIL {
                        return Ok(loader);
                    }
                }
            }
        }
        self.loader_for_type(asset_type)
            .ok_or(AssetError::LoaderForTypeNotFound { asset_type })
    }

    pub fn asset_type_for_extension(&self, extension: &str) -> Option<AssetTypeId> {
        self.loader_for_extension(extension)
            .map(|loader| loader.asset_type())
    }

    pub fn len(&self) -> usize {
        self.loaders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loaders.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AssetLoadGroupId(pub u64);

#[derive(Clone, Debug)]
pub struct AssetLoadGroup {
    pub id: AssetLoadGroupId,
    pub assets: Vec<crate::handle::UntypedHandle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
