use std::collections::HashMap;

use crate::{
    id::{AssetId, AssetTypeId},
    metadata::AssetMetadata,
    path::AssetPath,
};

#[derive(Clone, Debug, Default)]
pub struct AssetRegistry {
    by_id: HashMap<AssetId, AssetMetadata>,
    path_to_id: HashMap<AssetPath, AssetId>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create(&mut self, path: AssetPath, asset_type: AssetTypeId) -> AssetId {
        if let Some(id) = self.path_to_id.get(&path).copied() {
            return id;
        }
        let id = AssetId::new();
        let metadata = AssetMetadata::runtime(id, path.clone(), asset_type);
        self.path_to_id.insert(path, id);
        self.by_id.insert(id, metadata);
        id
    }

    pub fn insert(&mut self, metadata: AssetMetadata) {
        if let Some(existing) = self.by_id.get(&metadata.id) {
            if existing.path != metadata.path {
                if let Some(path) = &existing.path {
                    self.path_to_id.remove(path);
                }
            }
        }
        if let Some(path) = &metadata.path {
            self.path_to_id.insert(path.clone(), metadata.id);
        }
        self.by_id.insert(metadata.id, metadata);
    }

    pub fn get(&self, id: AssetId) -> Option<&AssetMetadata> {
        self.by_id.get(&id)
    }

    pub fn get_mut(&mut self, id: AssetId) -> Option<&mut AssetMetadata> {
        self.by_id.get_mut(&id)
    }

    pub fn id_from_path(&self, path: &AssetPath) -> Option<AssetId> {
        self.path_to_id.get(path).copied()
    }

    pub fn path_from_id(&self, id: AssetId) -> Option<&AssetPath> {
        self.by_id
            .get(&id)
            .and_then(|metadata| metadata.path.as_ref())
    }

    pub fn metadata_by_path(&self, path: &AssetPath) -> Option<&AssetMetadata> {
        self.id_from_path(path).and_then(|id| self.get(id))
    }

    pub fn rename_path(&mut self, id: AssetId, new_path: AssetPath) -> bool {
        let Some(metadata) = self.by_id.get_mut(&id) else {
            return false;
        };
        if let Some(old_path) = metadata.path.replace(new_path.clone()) {
            self.path_to_id.remove(&old_path);
        }
        self.path_to_id.insert(new_path, id);
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetMetadata)> {
        self.by_id.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &AssetMetadata> {
        self.by_id.values()
    }

    pub fn clear(&mut self) {
        self.by_id.clear();
        self.path_to_id.clear();
    }
}
