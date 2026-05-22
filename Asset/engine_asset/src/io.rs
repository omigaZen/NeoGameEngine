use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::{
    error::{AssetIoAction, AssetIoError},
    id::ContentHash,
};

pub trait AssetIo: Send + Sync + 'static {
    fn exists(&self, path: &str) -> bool;
    fn read(&self, path: &str) -> Result<Vec<u8>, AssetIoError>;

    fn read_range(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>, AssetIoError> {
        let bytes = self
            .read(path)
            .map_err(|error| error.with_action(AssetIoAction::ReadRange))?;
        let offset = offset as usize;
        let length = length as usize;
        if offset >= bytes.len() {
            return Ok(Vec::new());
        }
        Ok(bytes[offset..bytes.len().min(offset + length)].to_vec())
    }

    fn metadata(&self, path: &str) -> Result<AssetIoMetadata, AssetIoError>;
    fn list(&self, directory: &str) -> Result<Vec<String>, AssetIoError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIoMetadata {
    pub path: String,
    pub size: u64,
    pub modified_time: Option<SystemTime>,
    pub hash: Option<ContentHash>,
}

#[derive(Clone, Debug)]
pub struct FileSystemAssetIo {
    root: PathBuf,
}

impl FileSystemAssetIo {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn full_path(&self, path: &str) -> PathBuf {
        self.root.join(path.replace('\\', "/"))
    }
}

impl AssetIo for FileSystemAssetIo {
    fn exists(&self, path: &str) -> bool {
        if !cfg!(feature = "filesystem") {
            let _ = path;
            return false;
        }
        self.full_path(path).exists()
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, AssetIoError> {
        if !cfg!(feature = "filesystem") {
            return Err(filesystem_disabled_error(path, AssetIoAction::Read));
        }
        fs::read(self.full_path(path)).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => AssetIoError::NotFound {
                path: path.to_owned(),
                action: AssetIoAction::Read,
            },
            std::io::ErrorKind::PermissionDenied => AssetIoError::PermissionDenied {
                path: path.to_owned(),
                action: AssetIoAction::Read,
                message: error.to_string(),
            },
            _ => AssetIoError::ReadFailed {
                path: path.to_owned(),
                action: AssetIoAction::Read,
                message: error.to_string(),
            },
        })
    }

    fn metadata(&self, path: &str) -> Result<AssetIoMetadata, AssetIoError> {
        if !cfg!(feature = "filesystem") {
            return Err(filesystem_disabled_error(path, AssetIoAction::Metadata));
        }
        let metadata = fs::metadata(self.full_path(path)).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => AssetIoError::NotFound {
                path: path.to_owned(),
                action: AssetIoAction::Metadata,
            },
            std::io::ErrorKind::PermissionDenied => AssetIoError::PermissionDenied {
                path: path.to_owned(),
                action: AssetIoAction::Metadata,
                message: error.to_string(),
            },
            _ => AssetIoError::ReadFailed {
                path: path.to_owned(),
                action: AssetIoAction::Metadata,
                message: error.to_string(),
            },
        })?;
        Ok(AssetIoMetadata {
            path: path.to_owned(),
            size: metadata.len(),
            modified_time: metadata.modified().ok(),
            hash: None,
        })
    }

    fn list(&self, directory: &str) -> Result<Vec<String>, AssetIoError> {
        if !cfg!(feature = "filesystem") {
            return Err(filesystem_disabled_error(directory, AssetIoAction::List));
        }
        let directory_path = self.full_path(directory);
        let mut entries = Vec::new();
        collect_files(&self.root, &directory_path, directory, &mut entries)?;
        Ok(entries)
    }
}

fn filesystem_disabled_error(path: &str, action: AssetIoAction) -> AssetIoError {
    AssetIoError::ReadFailed {
        path: path.to_owned(),
        action,
        message: "asset filesystem feature is disabled".to_owned(),
    }
}

fn collect_files(
    root: &Path,
    directory_path: &Path,
    logical_directory: &str,
    entries: &mut Vec<String>,
) -> Result<(), AssetIoError> {
    for entry in fs::read_dir(directory_path).map_err(|error| AssetIoError::ReadFailed {
        path: logical_directory.to_owned(),
        action: AssetIoAction::List,
        message: error.to_string(),
    })? {
        let entry = entry.map_err(|error| AssetIoError::ReadFailed {
            path: logical_directory.to_owned(),
            action: AssetIoAction::List,
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, logical_directory, entries)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            entries.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct MemoryAssetIo {
    files: HashMap<String, Vec<u8>>,
}

impl MemoryAssetIo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(mut self, path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.insert(path, bytes);
        self
    }

    pub fn insert(&mut self, path: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.files
            .insert(path.into().replace('\\', "/"), bytes.into());
    }
}

impl AssetIo for MemoryAssetIo {
    fn exists(&self, path: &str) -> bool {
        self.files.contains_key(&path.replace('\\', "/"))
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, AssetIoError> {
        self.files
            .get(&path.replace('\\', "/"))
            .cloned()
            .ok_or_else(|| AssetIoError::NotFound {
                path: path.to_owned(),
                action: AssetIoAction::Read,
            })
    }

    fn metadata(&self, path: &str) -> Result<AssetIoMetadata, AssetIoError> {
        let bytes =
            self.files
                .get(&path.replace('\\', "/"))
                .ok_or_else(|| AssetIoError::NotFound {
                    path: path.to_owned(),
                    action: AssetIoAction::Metadata,
                })?;
        Ok(AssetIoMetadata {
            path: path.to_owned(),
            size: bytes.len() as u64,
            modified_time: None,
            hash: Some(ContentHash(stable_hash(bytes))),
        })
    }

    fn list(&self, directory: &str) -> Result<Vec<String>, AssetIoError> {
        let prefix = directory.trim_matches('/').to_owned();
        Ok(self
            .files
            .keys()
            .filter(|path| prefix.is_empty() || path.starts_with(&prefix))
            .cloned()
            .collect())
    }
}

pub(crate) fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIoLayerInfo {
    pub name: String,
    pub kind: AssetIoLayerKind,
    pub priority: usize,
}

impl AssetIoLayerInfo {
    pub fn new(name: impl Into<String>, kind: AssetIoLayerKind, priority: usize) -> Self {
        Self {
            name: name.into(),
            kind,
            priority,
        }
    }

    pub fn unnamed(priority: usize) -> Self {
        Self::new(
            format!("layer_{priority}"),
            AssetIoLayerKind::Custom,
            priority,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIoResolution {
    pub path: String,
    pub layer: AssetIoLayerInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetIoListedPath {
    pub path: String,
    pub layer: AssetIoLayerInfo,
}

struct CompositeAssetIoLayer {
    info: AssetIoLayerInfo,
    io: Box<dyn AssetIo>,
}

#[derive(Default)]
pub struct CompositeAssetIo {
    layers: Vec<CompositeAssetIoLayer>,
}

impl CompositeAssetIo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_layer<I: AssetIo>(mut self, layer: I) -> Self {
        self.push_layer(layer);
        self
    }

    pub fn with_named_layer<I: AssetIo>(
        mut self,
        name: impl Into<String>,
        kind: AssetIoLayerKind,
        layer: I,
    ) -> Self {
        self.push_named_layer(name, kind, layer);
        self
    }

    pub fn push_layer<I: AssetIo>(&mut self, layer: I) {
        let priority = self.layers.len();
        self.push_layer_with_info(AssetIoLayerInfo::unnamed(priority), layer);
    }

    pub fn push_named_layer<I: AssetIo>(
        &mut self,
        name: impl Into<String>,
        kind: AssetIoLayerKind,
        layer: I,
    ) {
        let priority = self.layers.len();
        self.push_layer_with_info(AssetIoLayerInfo::new(name, kind, priority), layer);
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn layers(&self) -> Vec<AssetIoLayerInfo> {
        self.layers.iter().map(|layer| layer.info.clone()).collect()
    }

    pub fn resolve(&self, path: &str) -> Option<AssetIoResolution> {
        self.layers
            .iter()
            .find(|layer| layer.io.exists(path))
            .map(|layer| AssetIoResolution {
                path: path.to_owned(),
                layer: layer.info.clone(),
            })
    }

    pub fn read_with_diagnostics(
        &self,
        path: &str,
    ) -> Result<(Vec<u8>, AssetIoResolution), AssetIoError> {
        let mut not_found = None;
        for layer in &self.layers {
            if !layer.io.exists(path) {
                continue;
            }
            match layer.io.read(path) {
                Ok(bytes) => {
                    return Ok((
                        bytes,
                        AssetIoResolution {
                            path: path.to_owned(),
                            layer: layer.info.clone(),
                        },
                    ));
                }
                Err(AssetIoError::NotFound { path, action }) => {
                    not_found = Some(AssetIoError::NotFound { path, action });
                }
                Err(error) => return Err(error),
            }
        }
        Err(not_found.unwrap_or_else(|| AssetIoError::NotFound {
            path: path.to_owned(),
            action: AssetIoAction::Read,
        }))
    }

    pub fn metadata_with_diagnostics(
        &self,
        path: &str,
    ) -> Result<(AssetIoMetadata, AssetIoResolution), AssetIoError> {
        for layer in &self.layers {
            if layer.io.exists(path) {
                return Ok((
                    layer.io.metadata(path)?,
                    AssetIoResolution {
                        path: path.to_owned(),
                        layer: layer.info.clone(),
                    },
                ));
            }
        }
        Err(AssetIoError::NotFound {
            path: path.to_owned(),
            action: AssetIoAction::Metadata,
        })
    }

    pub fn list_with_diagnostics(
        &self,
        directory: &str,
    ) -> Result<Vec<AssetIoListedPath>, AssetIoError> {
        let mut seen = HashSet::new();
        let mut merged = Vec::new();
        for layer in &self.layers {
            for path in layer.io.list(directory)? {
                let path = path.replace('\\', "/");
                if seen.insert(path.clone()) {
                    merged.push(AssetIoListedPath {
                        path,
                        layer: layer.info.clone(),
                    });
                }
            }
        }
        merged.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(merged)
    }

    fn push_layer_with_info<I: AssetIo>(&mut self, info: AssetIoLayerInfo, layer: I) {
        self.layers.push(CompositeAssetIoLayer {
            info,
            io: Box::new(layer),
        });
    }
}

impl AssetIo for CompositeAssetIo {
    fn exists(&self, path: &str) -> bool {
        self.layers.iter().any(|layer| layer.io.exists(path))
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, AssetIoError> {
        self.read_with_diagnostics(path).map(|(bytes, _)| bytes)
    }

    fn metadata(&self, path: &str) -> Result<AssetIoMetadata, AssetIoError> {
        self.metadata_with_diagnostics(path)
            .map(|(metadata, _)| metadata)
    }

    fn list(&self, directory: &str) -> Result<Vec<String>, AssetIoError> {
        Ok(self
            .list_with_diagnostics(directory)?
            .into_iter()
            .map(|entry| entry.path)
            .collect())
    }
}
