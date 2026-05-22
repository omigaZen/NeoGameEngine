use std::{any::Any, collections::HashMap};

use crate::{
    asset::{Asset, AssetMemoryUsage},
    assets::{Material, Mesh, Shader, Texture},
    error::AssetError,
    events::AssetLoadState,
    gpu_upload::GpuResourceHandle,
    handle::Handle,
    id::AssetId,
    metadata::AssetMetadata,
};

#[derive(Clone, Debug)]
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
    pub cpu_bytes: u64,
    pub gpu_bytes: u64,
}

impl<T: Asset> AssetEntry<T> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            asset: None,
            state: AssetLoadState::Unloaded,
            metadata: None,
            strong_count: 0,
            weak_count: 0,
            dependency_ref_count: 0,
            last_used_frame: 0,
            resident: false,
            error: None,
            cpu_bytes: 0,
            gpu_bytes: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Assets<T: Asset> {
    entries: HashMap<AssetId, AssetEntry<T>>,
}

impl<T: Asset> Default for Assets<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Asset> Assets<T> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, id: AssetId) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn entry(&self, id: AssetId) -> Option<&AssetEntry<T>> {
        self.entries.get(&id)
    }

    pub fn entry_mut(&mut self, id: AssetId) -> Option<&mut AssetEntry<T>> {
        self.entries.get_mut(&id)
    }

    pub fn ensure_entry(&mut self, id: AssetId) -> &mut AssetEntry<T> {
        self.entries
            .entry(id)
            .or_insert_with(|| AssetEntry::new(id))
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.get_by_id(handle.id())
    }

    pub fn get_by_id(&self, id: AssetId) -> Option<&T> {
        self.entries.get(&id).and_then(|entry| {
            (entry.state == AssetLoadState::Ready).then_some(())?;
            entry.asset.as_ref()
        })
    }

    pub fn get_cpu_by_id(&self, id: AssetId) -> Option<&T> {
        self.entries.get(&id).and_then(|entry| entry.asset.as_ref())
    }

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.get_mut_by_id(handle.id())
    }

    pub fn get_mut_by_id(&mut self, id: AssetId) -> Option<&mut T> {
        self.entries.get_mut(&id).and_then(|entry| {
            (entry.state == AssetLoadState::Ready).then_some(())?;
            entry.asset.as_mut()
        })
    }

    pub fn insert(&mut self, id: AssetId, asset: T) -> Option<T> {
        let entry = self.ensure_entry(id);
        entry.state = AssetLoadState::Ready;
        entry.error = None;
        entry.asset.replace(asset)
    }

    pub fn insert_with_state(&mut self, id: AssetId, asset: T, state: AssetLoadState) -> Option<T> {
        let entry = self.ensure_entry(id);
        entry.state = state;
        entry.error = None;
        entry.asset.replace(asset)
    }

    pub fn remove(&mut self, id: AssetId) -> Option<T> {
        self.entries.remove(&id).and_then(|entry| entry.asset)
    }

    pub fn state(&self, id: AssetId) -> AssetLoadState {
        self.entries
            .get(&id)
            .map(|entry| entry.state)
            .unwrap_or(AssetLoadState::Unloaded)
    }

    pub fn set_state(&mut self, id: AssetId, state: AssetLoadState) {
        self.ensure_entry(id).state = state;
    }

    pub fn error(&self, id: AssetId) -> Option<&AssetError> {
        self.entries.get(&id).and_then(|entry| entry.error.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId, &T)> {
        self.entries
            .iter()
            .filter_map(|(id, entry)| entry.asset.as_ref().map(|asset| (*id, asset)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetId, &mut T)> {
        self.entries
            .iter_mut()
            .filter_map(|(id, entry)| entry.asset.as_mut().map(|asset| (*id, asset)))
    }

    pub fn mark_used(&mut self, id: AssetId, frame: u64) {
        self.ensure_entry(id).last_used_frame = frame;
    }
}

pub(crate) trait ErasedAssets: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn state(&self, id: AssetId) -> AssetLoadState;
    fn set_state(&mut self, id: AssetId, state: AssetLoadState);
    fn set_error(&mut self, id: AssetId, error: AssetError);
    fn set_error_with_state(&mut self, id: AssetId, error: AssetError, state: AssetLoadState);
    fn error(&self, id: AssetId) -> Option<&AssetError>;
    fn insert_boxed(
        &mut self,
        id: AssetId,
        asset: Box<dyn Any + Send + Sync>,
        state: AssetLoadState,
    ) -> Result<(), AssetError>;
    fn set_metadata(&mut self, id: AssetId, metadata: AssetMetadata);
    fn set_counts(&mut self, id: AssetId, strong: usize, weak: usize, dependency: usize);
    fn set_resident(&mut self, id: AssetId, resident: bool);
    fn is_resident(&self, id: AssetId) -> bool;
    fn remove(&mut self, id: AssetId) -> bool;
    fn apply_gpu_upload(&mut self, id: AssetId, gpu: GpuResourceHandle) -> bool;
    fn cpu_gpu_bytes(&self, id: AssetId) -> (u64, u64);
    fn ids(&self) -> Vec<AssetId>;
    fn last_used_frame(&self, id: AssetId) -> u64;
    fn mark_used(&mut self, id: AssetId, frame: u64);
}

impl<T: Asset> ErasedAssets for Assets<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn state(&self, id: AssetId) -> AssetLoadState {
        self.state(id)
    }

    fn set_state(&mut self, id: AssetId, state: AssetLoadState) {
        self.set_state(id, state);
    }

    fn set_error(&mut self, id: AssetId, error: AssetError) {
        let entry = self.ensure_entry(id);
        entry.state = AssetLoadState::Failed;
        entry.error = Some(error);
    }

    fn set_error_with_state(&mut self, id: AssetId, error: AssetError, state: AssetLoadState) {
        let entry = self.ensure_entry(id);
        entry.state = state;
        entry.error = Some(error);
    }

    fn error(&self, id: AssetId) -> Option<&AssetError> {
        self.error(id)
    }

    fn insert_boxed(
        &mut self,
        id: AssetId,
        asset: Box<dyn Any + Send + Sync>,
        state: AssetLoadState,
    ) -> Result<(), AssetError> {
        let asset = asset
            .downcast::<T>()
            .map_err(|_| AssetError::TypeMismatch {
                expected: T::TYPE_NAME.to_owned(),
                actual: "boxed asset".to_owned(),
            })?;
        self.insert_with_state(id, *asset, state);
        let (cpu, gpu) = self
            .entry(id)
            .and_then(|entry| entry.asset.as_ref())
            .map(asset_memory_bytes)
            .unwrap_or((0, 0));
        let entry = self.ensure_entry(id);
        entry.cpu_bytes = cpu;
        entry.gpu_bytes = gpu;
        Ok(())
    }

    fn set_metadata(&mut self, id: AssetId, metadata: AssetMetadata) {
        self.ensure_entry(id).metadata = Some(metadata);
    }

    fn set_counts(&mut self, id: AssetId, strong: usize, weak: usize, dependency: usize) {
        let entry = self.ensure_entry(id);
        entry.strong_count = strong;
        entry.weak_count = weak;
        entry.dependency_ref_count = dependency;
    }

    fn set_resident(&mut self, id: AssetId, resident: bool) {
        self.ensure_entry(id).resident = resident;
    }

    fn is_resident(&self, id: AssetId) -> bool {
        self.entry(id).map(|entry| entry.resident).unwrap_or(false)
    }

    fn remove(&mut self, id: AssetId) -> bool {
        self.entries.remove(&id).is_some()
    }

    fn apply_gpu_upload(&mut self, id: AssetId, gpu: GpuResourceHandle) -> bool {
        let Some(entry) = self.entry_mut(id) else {
            return false;
        };
        let Some(asset) = entry.asset.as_mut() else {
            return false;
        };
        let any = asset as &mut dyn Any;
        if let Some(texture) = any.downcast_mut::<Texture>() {
            texture.gpu = Some(gpu);
            return true;
        }
        if let Some(mesh) = any.downcast_mut::<Mesh>() {
            mesh.gpu = Some(gpu);
            return true;
        }
        if let Some(material) = any.downcast_mut::<Material>() {
            material.gpu = Some(gpu);
            return true;
        }
        if let Some(shader) = any.downcast_mut::<Shader>() {
            shader.gpu = Some(gpu);
            return true;
        }
        false
    }

    fn cpu_gpu_bytes(&self, id: AssetId) -> (u64, u64) {
        self.entry(id)
            .map(|entry| (entry.cpu_bytes, entry.gpu_bytes))
            .unwrap_or((0, 0))
    }

    fn ids(&self) -> Vec<AssetId> {
        self.entries.keys().copied().collect()
    }

    fn last_used_frame(&self, id: AssetId) -> u64 {
        self.entry(id)
            .map(|entry| entry.last_used_frame)
            .unwrap_or(0)
    }

    fn mark_used(&mut self, id: AssetId, frame: u64) {
        self.mark_used(id, frame);
    }
}

fn asset_memory_bytes<T: Asset>(asset: &T) -> (u64, u64) {
    let any = asset as &dyn Any;
    if let Some(texture) = any.downcast_ref::<Texture>() {
        return (texture.cpu_bytes(), texture.gpu_bytes());
    }
    if let Some(mesh) = any.downcast_ref::<Mesh>() {
        return (mesh.cpu_bytes(), mesh.gpu_bytes());
    }
    if let Some(material) = any.downcast_ref::<Material>() {
        return (material.cpu_bytes(), material.gpu_bytes());
    }
    if let Some(shader) = any.downcast_ref::<Shader>() {
        return (shader.cpu_bytes(), shader.gpu_bytes());
    }
    (0, 0)
}
