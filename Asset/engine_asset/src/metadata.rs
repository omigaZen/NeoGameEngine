use crate::{
    id::{AssetId, AssetTypeId, ContentHash, VersionHash},
    path::AssetPath,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub path: Option<AssetPath>,
    pub asset_type: AssetTypeId,
    pub source_path: Option<AssetPath>,
    pub cooked_path: Option<AssetPath>,
    pub importer: Option<String>,
    pub importer_version: u32,
    pub source_hash: Option<ContentHash>,
    pub settings_hash: Option<ContentHash>,
    pub cooked_hash: Option<ContentHash>,
    pub version_hash: Option<VersionHash>,
    pub labels: Vec<String>,
    pub dependencies: Vec<AssetId>,
    pub importer_settings: Vec<(String, String)>,
}

impl AssetMetadata {
    pub fn runtime(id: AssetId, path: AssetPath, asset_type: AssetTypeId) -> Self {
        Self {
            id,
            path: Some(path),
            asset_type,
            source_path: None,
            cooked_path: None,
            importer: None,
            importer_version: 0,
            source_hash: None,
            settings_hash: None,
            cooked_hash: None,
            version_hash: None,
            labels: Vec::new(),
            dependencies: Vec::new(),
            importer_settings: Vec::new(),
        }
    }
}
