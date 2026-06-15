use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{
    error::{AssetError, AssetIoAction, AssetIoError, AssetResult},
    id::{AssetId, AssetTypeId, ContentHash},
    io::{stable_hash, AssetIo, AssetIoLayerKind, AssetIoMetadata, CompositeAssetIo},
    path::AssetPath,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompressionKind {
    None,
    Rle,
    Zstd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleCompressionCodecReport {
    pub compression: CompressionKind,
    pub supported: bool,
    pub codec_name: &'static str,
    pub reason: Option<String>,
}

impl BundleCompressionCodecReport {
    pub fn for_compression(compression: CompressionKind) -> Self {
        match compression {
            CompressionKind::None => Self {
                compression,
                supported: true,
                codec_name: "none",
                reason: None,
            },
            CompressionKind::Rle => Self {
                compression,
                supported: true,
                codec_name: "rle",
                reason: None,
            },
            CompressionKind::Zstd => Self {
                compression,
                supported: cfg!(feature = "zstd"),
                codec_name: "zstd",
                reason: if cfg!(feature = "zstd") {
                    None
                } else {
                    Some("asset zstd feature is disabled".to_owned())
                },
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BundleChunkPartitionPolicy {
    SingleChunk,
    MaxUncompressedBytes(usize),
}

impl Default for BundleChunkPartitionPolicy {
    fn default() -> Self {
        Self::SingleChunk
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BundleChunkLoadingPolicy {
    Eager,
    OnDemandCached,
    OnDemandCachedLimited { max_decoded_chunks: usize },
}

impl Default for BundleChunkLoadingPolicy {
    fn default() -> Self {
        Self::Eager
    }
}

impl BundleChunkLoadingPolicy {
    pub fn is_on_demand_cached(self) -> bool {
        matches!(
            self,
            Self::OnDemandCached | Self::OnDemandCachedLimited { .. }
        )
    }

    pub fn max_decoded_chunks(self) -> Option<usize> {
        match self {
            Self::OnDemandCachedLimited { max_decoded_chunks } => Some(max_decoded_chunks),
            Self::Eager | Self::OnDemandCached => None,
        }
    }
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
    pub fn new(compression: CompressionKind) -> Self {
        Self {
            compression,
            chunk_policy: BundleChunkPartitionPolicy::SingleChunk,
        }
    }

    pub fn with_chunk_policy(mut self, chunk_policy: BundleChunkPartitionPolicy) -> Self {
        self.chunk_policy = chunk_policy;
        self
    }
}

impl Default for BundleBuildOptions {
    fn default() -> Self {
        Self::new(CompressionKind::None)
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleChunkPrefetchReport {
    pub requested_chunks: Vec<u32>,
    pub decoded_chunks: Vec<u32>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evicted_chunks: Vec<u32>,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleChunk {
    pub index: u32,
    pub offset: u64,
    pub compressed_length: u64,
    pub uncompressed_length: u64,
    pub compression: CompressionKind,
    pub content_hash: ContentHash,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleManifest {
    pub name: String,
    pub compression: CompressionKind,
    pub chunks: Vec<BundleChunk>,
    pub entries: Vec<BundleEntry>,
}

impl BundleManifest {
    pub fn entry(&self, id: AssetId) -> Option<&BundleEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn entry_by_path(&self, path: &AssetPath) -> Option<&BundleEntry> {
        self.entries
            .iter()
            .find(|entry| entry.path.as_ref() == Some(path))
    }

    pub fn dependencies(&self, id: AssetId) -> Option<&[AssetId]> {
        self.entry(id).map(|entry| entry.dependencies.as_slice())
    }

    pub fn chunk(&self, index: u32) -> Option<&BundleChunk> {
        self.chunks.iter().find(|chunk| chunk.index == index)
    }

    pub fn total_uncompressed_bytes(&self) -> u64 {
        self.chunks
            .iter()
            .map(|chunk| chunk.uncompressed_length)
            .sum()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BundleId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountedBundle {
    pub id: BundleId,
    pub name: String,
    pub manifest: BundleManifest,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MountedBundleRegistry {
    bundles: Vec<MountedBundle>,
}

impl MountedBundleRegistry {
    pub fn new(mut bundles: Vec<MountedBundle>) -> Self {
        bundles.sort_by_key(|bundle| bundle.id.0);
        Self { bundles }
    }

    pub fn from_mounted_bundles<'a>(bundles: impl IntoIterator<Item = &'a MountedBundle>) -> Self {
        Self::new(bundles.into_iter().cloned().collect())
    }

    pub fn bundles(&self) -> &[MountedBundle] {
        &self.bundles
    }

    pub fn into_bundles(self) -> Vec<MountedBundle> {
        self.bundles
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_text()).map_err(|error| filesystem_error("write", path, error))
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> AssetResult<Self> {
        let path = path.as_ref();
        let text =
            fs::read_to_string(path).map_err(|error| filesystem_error("read", path, error))?;
        Self::from_text(&text)
    }

    pub fn to_text(&self) -> String {
        let mut lines = vec![
            "NGA_MOUNTED_BUNDLE_REGISTRY_V1".to_owned(),
            format!("bundles={}", self.bundles.len()),
        ];
        for bundle in &self.bundles {
            let manifest = serialize_manifest(&bundle.manifest);
            let manifest_line_count = manifest.lines().count();
            lines.push(format!("bundle|{}|{manifest_line_count}", bundle.id.0));
            lines.extend(manifest.lines().map(str::to_owned));
        }
        lines.join("\n")
    }

    pub fn from_text(text: &str) -> AssetResult<Self> {
        let lines = text.lines().collect::<Vec<_>>();
        let mut index = 0;
        if lines.get(index).copied() != Some("NGA_MOUNTED_BUNDLE_REGISTRY_V1") {
            return Err(AssetError::Bundle {
                message: "invalid mounted bundle registry header".to_owned(),
            });
        }
        index += 1;
        let bundle_count: usize = parse_prefixed_line(lines.get(index).copied(), "bundles=")?
            .parse()
            .map_err(|error| AssetError::Bundle {
                message: format!("invalid mounted bundle count: {error}"),
            })?;
        index += 1;

        let mut bundles = Vec::with_capacity(bundle_count);
        for bundle_index in 0..bundle_count {
            let line = lines
                .get(index)
                .copied()
                .ok_or_else(|| AssetError::Bundle {
                    message: format!("missing mounted bundle line {bundle_index}"),
                })?;
            index += 1;
            let fields = line.split('|').collect::<Vec<_>>();
            if fields.len() != 3 || fields[0] != "bundle" {
                return Err(AssetError::Bundle {
                    message: format!("invalid mounted bundle line {bundle_index}"),
                });
            }
            let id = BundleId(parse_u64(fields[1], "mounted bundle id")?);
            let manifest_line_count = parse_usize(fields[2], "mounted bundle manifest line count")?;
            let end = index
                .checked_add(manifest_line_count)
                .ok_or_else(|| AssetError::Bundle {
                    message: "mounted bundle manifest line count overflow".to_owned(),
                })?;
            let manifest_lines = lines.get(index..end).ok_or_else(|| AssetError::Bundle {
                message: format!("mounted bundle {bundle_index} manifest is truncated"),
            })?;
            index = end;
            let manifest = deserialize_manifest(&manifest_lines.join("\n"))?;
            bundles.push(MountedBundle {
                id,
                name: manifest.name.clone(),
                manifest,
            });
        }

        if index != lines.len() {
            return Err(AssetError::Bundle {
                message: "unexpected trailing mounted bundle registry data".to_owned(),
            });
        }

        Ok(Self::new(bundles))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    ) -> Self {
        Self {
            bundle_id,
            name: name.into(),
            kind,
            priority,
            enabled,
            bundle_path: bundle_path.into().replace('\\', "/"),
            package_version: Self::DEFAULT_PACKAGE_VERSION,
            minimum_runtime_version: Self::CURRENT_RUNTIME_VERSION,
            package_dependencies: Vec::new(),
            manifest,
        }
    }

    pub fn with_package_version(mut self, package_version: u32) -> Self {
        self.package_version = package_version;
        self
    }

    pub fn with_minimum_runtime_version(mut self, minimum_runtime_version: u32) -> Self {
        self.minimum_runtime_version = minimum_runtime_version;
        self
    }

    pub fn with_package_dependency(mut self, dependency: AssetPackageDependency) -> Self {
        self.package_dependencies.push(dependency);
        self
    }

    pub fn with_package_dependencies(mut self, dependencies: Vec<AssetPackageDependency>) -> Self {
        self.package_dependencies = dependencies;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageDependency {
    pub package: String,
    pub min_version: u32,
    pub max_version: Option<u32>,
}

impl AssetPackageDependency {
    pub fn new(package: impl Into<String>, min_version: u32) -> Self {
        Self {
            package: package.into(),
            min_version,
            max_version: None,
        }
    }

    pub fn with_max_version(mut self, max_version: u32) -> Self {
        self.max_version = Some(max_version);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageLayerInfo {
    pub bundle_id: BundleId,
    pub name: String,
    pub kind: AssetIoLayerKind,
    pub priority: usize,
    pub bundle_path: String,
    pub package_version: u32,
    pub minimum_runtime_version: u32,
}

impl AssetPackageLayerInfo {
    fn from_record(record: &AssetPackageRecord) -> Self {
        Self {
            bundle_id: record.bundle_id,
            name: record.name.clone(),
            kind: record.kind,
            priority: record.priority,
            bundle_path: record.bundle_path.clone(),
            package_version: record.package_version,
            minimum_runtime_version: record.minimum_runtime_version,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageConflict {
    pub path: AssetPath,
    pub winner: AssetPackageLayerInfo,
    pub shadowed: Vec<AssetPackageLayerInfo>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetPackageConflictReport {
    pub conflicts: Vec<AssetPackageConflict>,
}

impl AssetPackageConflictReport {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageAssetInfo {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub content_hash: ContentHash,
    pub dependencies: Vec<AssetId>,
}

impl AssetPackageAssetInfo {
    fn from_entry(entry: &BundleEntry) -> Self {
        Self {
            id: entry.id,
            asset_type: entry.asset_type,
            content_hash: entry.content_hash,
            dependencies: entry.dependencies.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageDependencyProvider {
    pub dependency: AssetId,
    pub provider: Option<AssetPackageLayerInfo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetPackageAssetOverrideIssueKind {
    AssetIdChanged,
    AssetTypeChanged,
    ContentHashChanged,
    DependenciesChanged,
    DependencyProvidersChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetPackageAssetOverrideReport {
    pub overrides: Vec<AssetPackageAssetOverride>,
}

impl AssetPackageAssetOverrideReport {
    pub fn has_overrides(&self) -> bool {
        !self.overrides.is_empty()
    }

    pub fn has_issues(&self) -> bool {
        self.overrides
            .iter()
            .any(AssetPackageAssetOverride::has_issues)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetPackageAssetCompatibilityPolicy {
    pub require_stable_asset_ids: bool,
    pub require_matching_asset_types: bool,
    pub require_matching_content_hashes: bool,
    pub require_matching_dependencies: bool,
    pub require_matching_dependency_providers: bool,
}

impl AssetPackageAssetCompatibilityPolicy {
    pub const fn permissive() -> Self {
        Self {
            require_stable_asset_ids: false,
            require_matching_asset_types: false,
            require_matching_content_hashes: false,
            require_matching_dependencies: false,
            require_matching_dependency_providers: false,
        }
    }

    pub const fn strict() -> Self {
        Self {
            require_stable_asset_ids: true,
            require_matching_asset_types: true,
            require_matching_content_hashes: true,
            require_matching_dependencies: true,
            require_matching_dependency_providers: true,
        }
    }

    pub fn with_stable_asset_ids_required(mut self, required: bool) -> Self {
        self.require_stable_asset_ids = required;
        self
    }

    pub fn with_matching_asset_types_required(mut self, required: bool) -> Self {
        self.require_matching_asset_types = required;
        self
    }

    pub fn with_matching_content_hashes_required(mut self, required: bool) -> Self {
        self.require_matching_content_hashes = required;
        self
    }

    pub fn with_matching_dependencies_required(mut self, required: bool) -> Self {
        self.require_matching_dependencies = required;
        self
    }

    pub fn with_matching_dependency_providers_required(mut self, required: bool) -> Self {
        self.require_matching_dependency_providers = required;
        self
    }
}

impl Default for AssetPackageAssetCompatibilityPolicy {
    fn default() -> Self {
        Self {
            require_stable_asset_ids: false,
            require_matching_asset_types: true,
            require_matching_content_hashes: false,
            require_matching_dependencies: false,
            require_matching_dependency_providers: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetPackageUpdatePolicy {
    pub runtime_version: u32,
    pub allow_version_downgrade: bool,
    pub asset_compatibility: AssetPackageAssetCompatibilityPolicy,
}

impl AssetPackageUpdatePolicy {
    pub fn new(runtime_version: u32) -> Self {
        Self {
            runtime_version,
            allow_version_downgrade: false,
            asset_compatibility: AssetPackageAssetCompatibilityPolicy::default(),
        }
    }

    pub fn with_version_downgrade_allowed(mut self, allow_version_downgrade: bool) -> Self {
        self.allow_version_downgrade = allow_version_downgrade;
        self
    }

    pub fn with_asset_compatibility(
        mut self,
        asset_compatibility: AssetPackageAssetCompatibilityPolicy,
    ) -> Self {
        self.asset_compatibility = asset_compatibility;
        self
    }
}

impl Default for AssetPackageUpdatePolicy {
    fn default() -> Self {
        Self::new(AssetPackageRecord::CURRENT_RUNTIME_VERSION)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageUpdateChange {
    pub name: String,
    pub previous_version: Option<u32>,
    pub next_version: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn is_compatible(&self) -> bool {
        self.compatibility_issues.is_empty()
    }

    pub fn require_compatible(&self) -> AssetResult<()> {
        if let Some(issue) = self.compatibility_issues.first() {
            Err(AssetError::Bundle {
                message: format!("asset package update is incompatible: {}", issue.message),
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageActivation {
    pub report: AssetPackageUpdateReport,
    pub mounted_bundles: Vec<MountedBundle>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    ) -> Self {
        Self {
            bundle_id,
            name: name.into(),
            kind,
            priority,
            enabled: true,
            bundle_path: bundle_path.into().replace('\\', "/"),
            package_version: AssetPackageRecord::DEFAULT_PACKAGE_VERSION,
            minimum_runtime_version: AssetPackageRecord::CURRENT_RUNTIME_VERSION,
            package_dependencies: Vec::new(),
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_package_version(mut self, package_version: u32) -> Self {
        self.package_version = package_version;
        self
    }

    pub fn with_minimum_runtime_version(mut self, minimum_runtime_version: u32) -> Self {
        self.minimum_runtime_version = minimum_runtime_version;
        self
    }

    pub fn with_package_dependency(mut self, dependency: AssetPackageDependency) -> Self {
        self.package_dependencies.push(dependency);
        self
    }

    pub fn with_package_dependencies(mut self, dependencies: Vec<AssetPackageDependency>) -> Self {
        self.package_dependencies = dependencies;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageInstallReport {
    pub record: AssetPackageRecord,
    pub replaced: Option<AssetPackageRecord>,
    pub artifact_path: PathBuf,
    pub payload_size: u64,
    pub payload_hash: ContentHash,
    pub conflicts: AssetPackageConflictReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageRemoveReport {
    pub removed: AssetPackageRecord,
    pub artifact_path: PathBuf,
    pub artifact_removed: bool,
    pub conflicts: AssetPackageConflictReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageArtifactStatus {
    pub package: String,
    pub bundle_path: String,
    pub artifact_path: PathBuf,
    pub exists: bool,
    pub payload_size: Option<u64>,
    pub payload_hash: Option<ContentHash>,
    pub manifest_matches: Option<bool>,
    pub message: Option<String>,
}

impl AssetPackageArtifactStatus {
    pub fn is_available(&self) -> bool {
        self.exists && self.manifest_matches.unwrap_or(false) && self.message.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageArtifactReport {
    pub root: PathBuf,
    pub packages: Vec<AssetPackageArtifactStatus>,
}

impl AssetPackageArtifactReport {
    pub fn all_available(&self) -> bool {
        self.packages
            .iter()
            .all(AssetPackageArtifactStatus::is_available)
    }

    pub fn require_available(&self) -> AssetResult<()> {
        if let Some(status) = self.packages.iter().find(|status| !status.is_available()) {
            Err(AssetError::Bundle {
                message: format!(
                    "asset package `{}` artifact `{}` is unavailable: {}",
                    status.package,
                    status.bundle_path,
                    status
                        .message
                        .clone()
                        .unwrap_or_else(|| "manifest does not match registry metadata".to_owned())
                ),
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPackageArtifactStore {
    root: PathBuf,
}

impl AssetPackageArtifactStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn artifact_path_for_record(&self, record: &AssetPackageRecord) -> AssetResult<PathBuf> {
        self.artifact_path(&record.bundle_path)
    }

    pub fn artifact_path(&self, bundle_path: &str) -> AssetResult<PathBuf> {
        resolve_package_artifact_path(&self.root, bundle_path)
    }

    pub fn install_package_bytes(
        &self,
        registry: &mut AssetPackageRegistry,
        request: AssetPackageInstallRequest,
        bytes: &[u8],
    ) -> AssetResult<AssetPackageInstallReport> {
        let manifest = BundleReader::from_bytes(bytes)?.manifest().clone();
        let record = AssetPackageRecord::new(
            request.bundle_id,
            request.name,
            request.kind,
            request.priority,
            request.enabled,
            request.bundle_path,
            manifest,
        )
        .with_package_version(request.package_version)
        .with_minimum_runtime_version(request.minimum_runtime_version)
        .with_package_dependencies(request.package_dependencies);
        let artifact_path = self.artifact_path_for_record(&record)?;
        if let Some(parent) = artifact_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| filesystem_error("create", parent, error))?;
        }
        fs::write(&artifact_path, bytes)
            .map_err(|error| filesystem_error("write", &artifact_path, error))?;

        let mut packages = registry.clone().into_packages();
        let replaced_index = packages.iter().position(|package| {
            package.name == record.name || package.bundle_id == record.bundle_id
        });
        let replaced = replaced_index.map(|index| packages.remove(index));
        packages.push(record.clone());
        *registry = AssetPackageRegistry::new(packages)?;

        Ok(AssetPackageInstallReport {
            record,
            replaced,
            artifact_path,
            payload_size: bytes.len() as u64,
            payload_hash: ContentHash(stable_hash(bytes)),
            conflicts: registry.conflict_report(),
        })
    }

    pub fn remove_package(
        &self,
        registry: &mut AssetPackageRegistry,
        name: &str,
        delete_artifact: bool,
    ) -> AssetResult<AssetPackageRemoveReport> {
        let mut packages = registry.clone().into_packages();
        let index = packages
            .iter()
            .position(|package| package.name == name)
            .ok_or_else(|| AssetError::Bundle {
                message: format!("asset package `{name}` is not registered"),
            })?;
        let removed = packages.remove(index);
        let artifact_path = self.artifact_path_for_record(&removed)?;
        let artifact_removed = if delete_artifact && artifact_path.exists() {
            fs::remove_file(&artifact_path)
                .map_err(|error| filesystem_error("remove", &artifact_path, error))?;
            true
        } else {
            false
        };
        *registry = AssetPackageRegistry::new(packages)?;
        Ok(AssetPackageRemoveReport {
            removed,
            artifact_path,
            artifact_removed,
            conflicts: registry.conflict_report(),
        })
    }

    pub fn load_package_bytes(&self, record: &AssetPackageRecord) -> AssetResult<Vec<u8>> {
        let path = self.artifact_path_for_record(record)?;
        fs::read(&path).map_err(|error| filesystem_error("read", &path, error))
    }

    pub fn verify_registry(
        &self,
        registry: &AssetPackageRegistry,
    ) -> AssetResult<AssetPackageArtifactReport> {
        registry.validate()?;
        let mut packages = Vec::new();
        for package in registry.enabled_packages() {
            packages.push(self.verify_package(package)?);
        }
        Ok(AssetPackageArtifactReport {
            root: self.root.clone(),
            packages,
        })
    }

    pub fn build_composite_io(
        &self,
        registry: &AssetPackageRegistry,
    ) -> AssetResult<CompositeAssetIo> {
        self.verify_registry(registry)?.require_available()?;
        registry.build_composite_io(|package| self.load_package_bytes(package))
    }

    fn verify_package(
        &self,
        package: &AssetPackageRecord,
    ) -> AssetResult<AssetPackageArtifactStatus> {
        let artifact_path = self.artifact_path_for_record(package)?;
        if !artifact_path.exists() {
            return Ok(AssetPackageArtifactStatus {
                package: package.name.clone(),
                bundle_path: package.bundle_path.clone(),
                artifact_path,
                exists: false,
                payload_size: None,
                payload_hash: None,
                manifest_matches: None,
                message: Some("artifact file is missing".to_owned()),
            });
        }
        let bytes = match fs::read(&artifact_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(AssetPackageArtifactStatus {
                    package: package.name.clone(),
                    bundle_path: package.bundle_path.clone(),
                    artifact_path,
                    exists: true,
                    payload_size: None,
                    payload_hash: None,
                    manifest_matches: None,
                    message: Some(format!("failed to read artifact: {error}")),
                });
            }
        };
        let payload_size = bytes.len() as u64;
        let payload_hash = ContentHash(stable_hash(&bytes));
        match BundleReader::from_bytes(&bytes) {
            Ok(reader) => {
                let manifest_matches = reader.manifest() == &package.manifest;
                Ok(AssetPackageArtifactStatus {
                    package: package.name.clone(),
                    bundle_path: package.bundle_path.clone(),
                    artifact_path,
                    exists: true,
                    payload_size: Some(payload_size),
                    payload_hash: Some(payload_hash),
                    manifest_matches: Some(manifest_matches),
                    message: (!manifest_matches)
                        .then(|| "payload manifest does not match registry metadata".to_owned()),
                })
            }
            Err(error) => Ok(AssetPackageArtifactStatus {
                package: package.name.clone(),
                bundle_path: package.bundle_path.clone(),
                artifact_path,
                exists: true,
                payload_size: Some(payload_size),
                payload_hash: Some(payload_hash),
                manifest_matches: None,
                message: Some(format!("failed to parse artifact bundle: {error}")),
            }),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetPackageRegistry {
    packages: Vec<AssetPackageRecord>,
}

impl AssetPackageRegistry {
    pub fn new(mut packages: Vec<AssetPackageRecord>) -> AssetResult<Self> {
        packages.sort_by_key(|package| package.priority);
        let registry = Self { packages };
        registry.validate()?;
        Ok(registry)
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn packages(&self) -> &[AssetPackageRecord] {
        &self.packages
    }

    pub fn enabled_packages(&self) -> impl Iterator<Item = &AssetPackageRecord> {
        self.packages.iter().filter(|package| package.enabled)
    }

    pub fn into_packages(self) -> Vec<AssetPackageRecord> {
        self.packages
    }

    pub fn validate(&self) -> AssetResult<()> {
        let mut names = HashSet::new();
        let mut ids = HashSet::new();
        let mut priorities = HashSet::new();
        for package in &self.packages {
            validate_package_token("package name", &package.name)?;
            validate_package_token("package bundle path", &package.bundle_path)?;
            if package.package_version == 0 {
                return Err(AssetError::Bundle {
                    message: format!(
                        "asset package `{}` version must be greater than zero",
                        package.name
                    ),
                });
            }
            if package.minimum_runtime_version == 0 {
                return Err(AssetError::Bundle {
                    message: format!(
                        "asset package `{}` minimum runtime version must be greater than zero",
                        package.name
                    ),
                });
            }
            validate_package_dependencies(package)?;
            if !names.insert(package.name.clone()) {
                return Err(AssetError::Bundle {
                    message: format!("duplicate asset package name `{}`", package.name),
                });
            }
            if !ids.insert(package.bundle_id) {
                return Err(AssetError::Bundle {
                    message: format!("duplicate asset package bundle id {:?}", package.bundle_id),
                });
            }
            if !priorities.insert(package.priority) {
                return Err(AssetError::Bundle {
                    message: format!("duplicate asset package priority {}", package.priority),
                });
            }
            validate_package_manifest_paths(package)?;
        }
        Ok(())
    }

    pub fn conflict_report(&self) -> AssetPackageConflictReport {
        let mut paths: BTreeMap<AssetPath, Vec<AssetPackageLayerInfo>> = BTreeMap::new();
        for package in self.enabled_packages() {
            let layer = AssetPackageLayerInfo::from_record(package);
            for entry in &package.manifest.entries {
                let Some(path) = &entry.path else {
                    continue;
                };
                paths.entry(path.clone()).or_default().push(layer.clone());
            }
        }
        let conflicts = paths
            .into_iter()
            .filter_map(|(path, layers)| {
                if layers.len() < 2 {
                    return None;
                }
                let winner = layers[0].clone();
                let shadowed = layers[1..].to_vec();
                Some(AssetPackageConflict {
                    path,
                    winner,
                    shadowed,
                })
            })
            .collect();
        AssetPackageConflictReport { conflicts }
    }

    pub fn asset_override_report(&self) -> AssetPackageAssetOverrideReport {
        let mut providers = HashMap::new();
        for package in self.enabled_packages() {
            let layer = AssetPackageLayerInfo::from_record(package);
            for entry in &package.manifest.entries {
                providers.entry(entry.id).or_insert_with(|| layer.clone());
            }
        }

        let mut paths: BTreeMap<AssetPath, Vec<(&AssetPackageRecord, &BundleEntry)>> =
            BTreeMap::new();
        for package in self.enabled_packages() {
            for entry in &package.manifest.entries {
                let Some(path) = &entry.path else {
                    continue;
                };
                paths
                    .entry(path.clone())
                    .or_default()
                    .push((package, entry));
            }
        }

        let mut overrides = Vec::new();
        for (path, entries) in paths {
            if entries.len() < 2 {
                continue;
            }
            let (winner_package, winner_entry) = entries[0];
            for (shadowed_package, shadowed_entry) in entries.iter().skip(1).copied() {
                overrides.push(asset_override_for_entries(
                    path.clone(),
                    winner_package,
                    winner_entry,
                    shadowed_package,
                    shadowed_entry,
                    &providers,
                ));
            }
        }
        AssetPackageAssetOverrideReport { overrides }
    }

    pub fn update_report(
        &self,
        next: &AssetPackageRegistry,
        policy: AssetPackageUpdatePolicy,
    ) -> AssetResult<AssetPackageUpdateReport> {
        self.validate()?;
        next.validate()?;
        let previous_by_name = self
            .packages
            .iter()
            .map(|package| (package.name.as_str(), package))
            .collect::<HashMap<_, _>>();
        let next_by_name = next
            .packages
            .iter()
            .map(|package| (package.name.as_str(), package))
            .collect::<HashMap<_, _>>();
        let next_enabled_by_name = next
            .enabled_packages()
            .map(|package| (package.name.as_str(), package))
            .collect::<HashMap<_, _>>();

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut updated = Vec::new();
        let mut enabled = Vec::new();
        let mut disabled = Vec::new();
        let mut compatibility_issues = Vec::new();
        let asset_overrides = next.asset_override_report();

        for package in &next.packages {
            match previous_by_name.get(package.name.as_str()).copied() {
                Some(previous) => {
                    if package_changed(previous, package) {
                        updated.push(package_update_change(Some(previous), Some(package)));
                    }
                    if !previous.enabled && package.enabled {
                        enabled.push(package_update_change(Some(previous), Some(package)));
                    } else if previous.enabled && !package.enabled {
                        disabled.push(package_update_change(Some(previous), Some(package)));
                    }
                    if package.enabled
                        && package.package_version < previous.package_version
                        && !policy.allow_version_downgrade
                    {
                        compatibility_issues.push(AssetPackageCompatibilityIssue {
                            package: package.name.clone(),
                            kind: AssetPackageCompatibilityIssueKind::VersionDowngrade,
                            previous_version: Some(previous.package_version),
                            next_version: package.package_version,
                            runtime_version: policy.runtime_version,
                            minimum_runtime_version: package.minimum_runtime_version,
                            dependency: None,
                            dependency_version: None,
                            required_min_version: None,
                            required_max_version: None,
                            asset_override: None,
                            message: format!(
                                "package `{}` version downgrade {} -> {} is not allowed",
                                package.name, previous.package_version, package.package_version
                            ),
                        });
                    }
                }
                None => added.push(package_update_change(None, Some(package))),
            }

            if package.enabled && package.minimum_runtime_version > policy.runtime_version {
                compatibility_issues.push(AssetPackageCompatibilityIssue {
                    package: package.name.clone(),
                    kind: AssetPackageCompatibilityIssueKind::RuntimeTooOld,
                    previous_version: previous_by_name
                        .get(package.name.as_str())
                        .map(|previous| previous.package_version),
                    next_version: package.package_version,
                    runtime_version: policy.runtime_version,
                    minimum_runtime_version: package.minimum_runtime_version,
                    dependency: None,
                    dependency_version: None,
                    required_min_version: None,
                    required_max_version: None,
                    asset_override: None,
                    message: format!(
                        "package `{}` requires runtime package version {}, current runtime is {}",
                        package.name, package.minimum_runtime_version, policy.runtime_version
                    ),
                });
            }

            if package.enabled {
                for dependency in &package.package_dependencies {
                    match next_enabled_by_name
                        .get(dependency.package.as_str())
                        .copied()
                    {
                        Some(provider) => {
                            if provider.package_version < dependency.min_version {
                                compatibility_issues.push(AssetPackageCompatibilityIssue {
                                    package: package.name.clone(),
                                    kind: AssetPackageCompatibilityIssueKind::PackageDependencyTooOld,
                                    previous_version: previous_by_name
                                        .get(package.name.as_str())
                                        .map(|previous| previous.package_version),
                                    next_version: package.package_version,
                                    runtime_version: policy.runtime_version,
                                    minimum_runtime_version: package.minimum_runtime_version,
                                    dependency: Some(dependency.package.clone()),
                                    dependency_version: Some(provider.package_version),
                                    required_min_version: Some(dependency.min_version),
                                    required_max_version: dependency.max_version,
                                    asset_override: None,
                                    message: format!(
                                        "package `{}` requires package `{}` version >= {}, found {}",
                                        package.name,
                                        dependency.package,
                                        dependency.min_version,
                                        provider.package_version
                                    ),
                                });
                            }
                            if let Some(max_version) = dependency.max_version {
                                if provider.package_version > max_version {
                                    compatibility_issues.push(AssetPackageCompatibilityIssue {
                                        package: package.name.clone(),
                                        kind: AssetPackageCompatibilityIssueKind::PackageDependencyTooNew,
                                        previous_version: previous_by_name
                                            .get(package.name.as_str())
                                            .map(|previous| previous.package_version),
                                        next_version: package.package_version,
                                        runtime_version: policy.runtime_version,
                                        minimum_runtime_version: package.minimum_runtime_version,
                                        dependency: Some(dependency.package.clone()),
                                        dependency_version: Some(provider.package_version),
                                        required_min_version: Some(dependency.min_version),
                                        required_max_version: dependency.max_version,
                                        asset_override: None,
                                        message: format!(
                                            "package `{}` requires package `{}` version <= {}, found {}",
                                            package.name,
                                            dependency.package,
                                            max_version,
                                            provider.package_version
                                        ),
                                    });
                                }
                            }
                        }
                        None => compatibility_issues.push(AssetPackageCompatibilityIssue {
                            package: package.name.clone(),
                            kind: AssetPackageCompatibilityIssueKind::MissingPackageDependency,
                            previous_version: previous_by_name
                                .get(package.name.as_str())
                                .map(|previous| previous.package_version),
                            next_version: package.package_version,
                            runtime_version: policy.runtime_version,
                            minimum_runtime_version: package.minimum_runtime_version,
                            dependency: Some(dependency.package.clone()),
                            dependency_version: None,
                            required_min_version: Some(dependency.min_version),
                            required_max_version: dependency.max_version,
                            asset_override: None,
                            message: format!(
                                "package `{}` requires enabled package `{}`",
                                package.name, dependency.package
                            ),
                        }),
                    }
                }
            }
        }

        for package in &self.packages {
            if !next_by_name.contains_key(package.name.as_str()) {
                removed.push(package_update_change(Some(package), None));
            }
        }

        for asset_override in &asset_overrides.overrides {
            for issue in &asset_override.issues {
                if !asset_override_issue_is_incompatible(*issue, policy.asset_compatibility) {
                    continue;
                }
                compatibility_issues.push(AssetPackageCompatibilityIssue {
                    package: asset_override.winner.name.clone(),
                    kind: asset_override_compatibility_issue_kind(*issue),
                    previous_version: previous_by_name
                        .get(asset_override.winner.name.as_str())
                        .map(|previous| previous.package_version),
                    next_version: asset_override.winner.package_version,
                    runtime_version: policy.runtime_version,
                    minimum_runtime_version: asset_override.winner.minimum_runtime_version,
                    dependency: None,
                    dependency_version: None,
                    required_min_version: None,
                    required_max_version: None,
                    asset_override: Some(asset_override.clone()),
                    message: asset_override_issue_message(asset_override, *issue),
                });
            }
        }

        Ok(AssetPackageUpdateReport {
            policy,
            added,
            removed,
            updated,
            enabled,
            disabled,
            compatibility_issues,
            conflicts: next.conflict_report(),
            asset_overrides,
        })
    }

    pub fn build_composite_io<F>(&self, mut load_bundle: F) -> AssetResult<CompositeAssetIo>
    where
        F: FnMut(&AssetPackageRecord) -> AssetResult<Vec<u8>>,
    {
        self.validate()?;
        let mut composite = CompositeAssetIo::new();
        for package in self.enabled_packages() {
            let bytes = load_bundle(package)?;
            let bundle_io = BundleAssetIo::from_bytes(&bytes)?;
            if bundle_io.manifest() != &package.manifest {
                return Err(AssetError::Bundle {
                    message: format!(
                        "asset package `{}` payload manifest does not match registry metadata",
                        package.name
                    ),
                });
            }
            composite.push_named_layer(package.name.clone(), package.kind, bundle_io);
        }
        Ok(composite)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> AssetResult<()> {
        let path = path.as_ref();
        fs::write(path, self.to_text()).map_err(|error| filesystem_error("write", path, error))
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> AssetResult<Self> {
        let path = path.as_ref();
        let text =
            fs::read_to_string(path).map_err(|error| filesystem_error("read", path, error))?;
        Self::from_text(&text)
    }

    pub fn to_text(&self) -> String {
        let mut lines = vec![
            "NGA_ASSET_PACKAGE_REGISTRY_V3".to_owned(),
            format!("packages={}", self.packages.len()),
        ];
        for package in &self.packages {
            let manifest = serialize_manifest(&package.manifest);
            let manifest_line_count = manifest.lines().count();
            lines.push(
                [
                    "package".to_owned(),
                    package.bundle_id.0.to_string(),
                    package.priority.to_string(),
                    package.enabled.to_string(),
                    asset_io_layer_kind_to_str(package.kind).to_owned(),
                    package.name.clone(),
                    package.bundle_path.clone(),
                    package.package_version.to_string(),
                    package.minimum_runtime_version.to_string(),
                    serialize_package_dependencies(&package.package_dependencies),
                    manifest_line_count.to_string(),
                ]
                .join("|"),
            );
            lines.extend(manifest.lines().map(str::to_owned));
        }
        lines.join("\n")
    }

    pub fn from_text(text: &str) -> AssetResult<Self> {
        let lines = text.lines().collect::<Vec<_>>();
        let mut index = 0;
        let version = match lines.get(index).copied() {
            Some("NGA_ASSET_PACKAGE_REGISTRY_V3") => 3,
            Some("NGA_ASSET_PACKAGE_REGISTRY_V2") => 2,
            Some("NGA_ASSET_PACKAGE_REGISTRY_V1") => 1,
            _ => {
                return Err(AssetError::Bundle {
                    message: "invalid asset package registry header".to_owned(),
                })
            }
        };
        index += 1;
        let package_count: usize = parse_prefixed_line(lines.get(index).copied(), "packages=")?
            .parse()
            .map_err(|error| AssetError::Bundle {
                message: format!("invalid asset package count: {error}"),
            })?;
        index += 1;

        let mut packages = Vec::with_capacity(package_count);
        for package_index in 0..package_count {
            let line = lines
                .get(index)
                .copied()
                .ok_or_else(|| AssetError::Bundle {
                    message: format!("missing asset package line {package_index}"),
                })?;
            index += 1;
            let fields = line.split('|').collect::<Vec<_>>();
            let expected_fields = match version {
                1 => 8,
                2 => 10,
                _ => 11,
            };
            if fields.len() != expected_fields || fields[0] != "package" {
                return Err(AssetError::Bundle {
                    message: format!("invalid asset package line {package_index}"),
                });
            }
            let (package_version, minimum_runtime_version, package_dependencies, manifest_field) =
                match version {
                    1 => (
                        AssetPackageRecord::DEFAULT_PACKAGE_VERSION,
                        AssetPackageRecord::CURRENT_RUNTIME_VERSION,
                        Vec::new(),
                        7,
                    ),
                    2 => (
                        parse_u32(fields[7], "asset package version")?,
                        parse_u32(fields[8], "asset package minimum runtime version")?,
                        Vec::new(),
                        9,
                    ),
                    _ => (
                        parse_u32(fields[7], "asset package version")?,
                        parse_u32(fields[8], "asset package minimum runtime version")?,
                        deserialize_package_dependencies(fields[9])?,
                        10,
                    ),
                };
            let manifest_line_count =
                parse_usize(fields[manifest_field], "asset package manifest line count")?;
            let end = index
                .checked_add(manifest_line_count)
                .ok_or_else(|| AssetError::Bundle {
                    message: "asset package manifest line count overflow".to_owned(),
                })?;
            let manifest_lines = lines.get(index..end).ok_or_else(|| AssetError::Bundle {
                message: format!("asset package {package_index} manifest is truncated"),
            })?;
            index = end;
            let manifest = deserialize_manifest(&manifest_lines.join("\n"))?;
            packages.push(
                AssetPackageRecord::new(
                    BundleId(parse_u64(fields[1], "asset package bundle id")?),
                    fields[5],
                    parse_asset_io_layer_kind(fields[4])?,
                    parse_usize(fields[2], "asset package priority")?,
                    parse_bool(fields[3], "asset package enabled")?,
                    fields[6],
                    manifest,
                )
                .with_package_version(package_version)
                .with_minimum_runtime_version(minimum_runtime_version)
                .with_package_dependencies(package_dependencies),
            );
        }

        if index != lines.len() {
            return Err(AssetError::Bundle {
                message: "unexpected trailing asset package registry data".to_owned(),
            });
        }

        Self::new(packages)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleAsset {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub path: AssetPath,
    pub bytes: Vec<u8>,
    pub dependencies: Vec<AssetId>,
}

pub struct BundleBuilder {
    manifest: BundleManifest,
    assets: Vec<BundleAsset>,
    chunk_policy: BundleChunkPartitionPolicy,
}

impl BundleBuilder {
    pub fn new(name: impl Into<String>, compression: CompressionKind) -> Self {
        Self {
            manifest: BundleManifest {
                name: name.into(),
                compression,
                chunks: Vec::new(),
                entries: Vec::new(),
            },
            assets: Vec::new(),
            chunk_policy: BundleChunkPartitionPolicy::SingleChunk,
        }
    }

    pub fn add_chunk(&mut self, chunk: BundleChunk) {
        self.manifest.chunks.push(chunk);
    }

    pub fn add_entry(&mut self, entry: BundleEntry) {
        self.manifest.entries.push(entry);
    }

    pub fn add_asset(&mut self, asset: BundleAsset) {
        self.assets.push(asset);
    }

    pub fn set_chunk_policy(&mut self, chunk_policy: BundleChunkPartitionPolicy) {
        self.chunk_policy = chunk_policy;
    }

    pub fn build(self) -> AssetResult<BundleManifest> {
        require_bundle_compression(self.manifest.compression, "builder manifest")?;
        if self.assets.is_empty() {
            return Ok(self.manifest);
        }
        Ok(BundleWriter::manifest_for_assets(
            self.manifest.name,
            BundleBuildOptions::new(self.manifest.compression).with_chunk_policy(self.chunk_policy),
            &self.assets,
        )?)
    }

    pub fn build_bytes(self) -> AssetResult<Vec<u8>> {
        BundleWriter::build_bytes_with_options(
            self.manifest.name,
            BundleBuildOptions::new(self.manifest.compression).with_chunk_policy(self.chunk_policy),
            self.assets,
        )
    }
}

pub struct BundleWriter;

impl BundleWriter {
    pub fn write_file(
        path: impl AsRef<Path>,
        name: impl Into<String>,
        compression: CompressionKind,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<BundleManifest> {
        Self::write_file_with_options(path, name, BundleBuildOptions::new(compression), assets)
    }

    pub fn write_file_with_options(
        path: impl AsRef<Path>,
        name: impl Into<String>,
        options: BundleBuildOptions,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<BundleManifest> {
        let path = path.as_ref();
        let bytes = Self::build_bytes_with_options(name, options, assets)?;
        fs::write(path, &bytes).map_err(|error| filesystem_error("write", path, error))?;
        Ok(BundleReader::from_bytes(&bytes)?.manifest().clone())
    }

    pub fn build_bytes(
        name: impl Into<String>,
        compression: CompressionKind,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<Vec<u8>> {
        Self::build_bytes_with_options(name, BundleBuildOptions::new(compression), assets)
    }

    pub fn build_bytes_with_options(
        name: impl Into<String>,
        options: BundleBuildOptions,
        assets: Vec<BundleAsset>,
    ) -> AssetResult<Vec<u8>> {
        let (manifest, data) = Self::layout_for_assets(name.into(), options, &assets)?;
        let mut header = serialize_manifest(&manifest).into_bytes();
        header.extend_from_slice(b"\nDATA\n");
        header.extend_from_slice(&data);
        Ok(header)
    }

    fn manifest_for_assets(
        name: String,
        options: BundleBuildOptions,
        assets: &[BundleAsset],
    ) -> AssetResult<BundleManifest> {
        Ok(Self::layout_for_assets(name, options, assets)?.0)
    }

    fn layout_for_assets(
        name: String,
        options: BundleBuildOptions,
        assets: &[BundleAsset],
    ) -> AssetResult<(BundleManifest, Vec<u8>)> {
        let compression = options.compression;
        require_bundle_compression(compression, "writer")?;
        validate_chunk_policy(options.chunk_policy)?;
        let groups = partition_assets(assets, options.chunk_policy)?;
        let mut encoded_data = Vec::new();
        let mut chunks = Vec::with_capacity(groups.len());
        let mut entries = Vec::with_capacity(assets.len());
        for (chunk_index, asset_indices) in groups.into_iter().enumerate() {
            let chunk_index = u32::try_from(chunk_index).map_err(|error| AssetError::Bundle {
                message: format!("bundle chunk index overflow: {error}"),
            })?;
            let mut chunk_bytes = Vec::new();
            let mut chunk_offset = 0_u64;
            for asset_index in asset_indices {
                let asset = &assets[asset_index];
                let length = asset.bytes.len() as u64;
                entries.push(BundleEntry {
                    id: asset.id,
                    asset_type: asset.asset_type,
                    path: Some(asset.path.clone()),
                    chunk_index,
                    offset: chunk_offset,
                    length,
                    content_hash: ContentHash(stable_hash(&asset.bytes)),
                    dependencies: asset.dependencies.clone(),
                });
                chunk_bytes.extend_from_slice(&asset.bytes);
                chunk_offset =
                    chunk_offset
                        .checked_add(length)
                        .ok_or_else(|| AssetError::Bundle {
                            message: "bundle chunk length overflow".to_owned(),
                        })?;
            }
            let encoded_chunk = encode_bundle_chunk(compression, &chunk_bytes)?;
            let data_offset = encoded_data.len() as u64;
            let compressed_length = encoded_chunk.len() as u64;
            let uncompressed_length = chunk_bytes.len() as u64;
            encoded_data.extend_from_slice(&encoded_chunk);
            chunks.push(BundleChunk {
                index: chunk_index,
                offset: data_offset,
                compressed_length,
                uncompressed_length,
                compression,
                content_hash: ContentHash(stable_hash(&chunk_bytes)),
            });
        }
        Ok((
            BundleManifest {
                name,
                compression,
                chunks,
                entries,
            },
            encoded_data,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BundleReader {
    manifest: BundleManifest,
    chunk_loading_policy: BundleChunkLoadingPolicy,
    chunk_data: HashMap<u32, Vec<u8>>,
    decoded_chunks: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
    decoded_chunk_lru: Arc<Mutex<VecDeque<u32>>>,
    cache_stats: Arc<Mutex<BundleChunkCacheStats>>,
    path_to_entry: HashMap<AssetPath, usize>,
}

impl BundleReader {
    pub fn from_bytes(bytes: &[u8]) -> AssetResult<Self> {
        Self::from_bytes_with_loading_policy(bytes, BundleChunkLoadingPolicy::Eager)
    }

    pub fn from_bytes_with_loading_policy(
        bytes: &[u8],
        chunk_loading_policy: BundleChunkLoadingPolicy,
    ) -> AssetResult<Self> {
        validate_chunk_loading_policy(chunk_loading_policy)?;
        let marker = b"\nDATA\n";
        let marker_index = bytes
            .windows(marker.len())
            .position(|window| window == marker)
            .ok_or_else(|| AssetError::Bundle {
                message: "bundle missing DATA section".to_owned(),
            })?;
        let manifest_text =
            std::str::from_utf8(&bytes[..marker_index]).map_err(|error| AssetError::Bundle {
                message: format!("bundle manifest is not UTF-8: {error}"),
            })?;
        let mut manifest = deserialize_manifest(manifest_text)?;
        require_bundle_compression(manifest.compression, "manifest")?;
        let data = bytes[marker_index + marker.len()..].to_vec();
        if manifest.chunks.is_empty() && !manifest.entries.is_empty() {
            manifest.chunks.push(BundleChunk {
                index: 0,
                offset: 0,
                compressed_length: data.len() as u64,
                uncompressed_length: data.len() as u64,
                compression: CompressionKind::None,
                content_hash: ContentHash(stable_hash(&data)),
            });
        }
        let mut must_decode_for_legacy_hash = false;
        if manifest.chunks.len() == 1 && manifest.chunks[0].content_hash == ContentHash(0) {
            let chunk = &mut manifest.chunks[0];
            chunk.offset = 0;
            chunk.compressed_length = data.len() as u64;
            if chunk.compression == CompressionKind::None {
                chunk.uncompressed_length = data.len() as u64;
            }
            must_decode_for_legacy_hash = true;
        }
        let mut chunk_data = HashMap::new();
        let mut decoded_chunks = HashMap::new();
        for chunk_index in 0..manifest.chunks.len() {
            let chunk = manifest.chunks[chunk_index].clone();
            require_bundle_compression(chunk.compression, &format!("chunk {}", chunk.index))?;
            let end = chunk.offset.saturating_add(chunk.compressed_length);
            if end as usize > data.len() {
                return Err(AssetError::Bundle {
                    message: format!("bundle chunk {} exceeds data section", chunk.index),
                });
            }
            let start = chunk.offset as usize;
            let end = end as usize;
            let compressed = data[start..end].to_vec();
            if chunk_loading_policy == BundleChunkLoadingPolicy::Eager
                || must_decode_for_legacy_hash
            {
                let decoded = decode_bundle_chunk(
                    chunk.compression,
                    &compressed,
                    chunk.uncompressed_length,
                    chunk.index,
                )?;
                let actual_hash = ContentHash(stable_hash(&decoded));
                if chunk.content_hash == ContentHash(0) {
                    manifest.chunks[chunk_index].content_hash = actual_hash;
                } else if chunk.content_hash != actual_hash {
                    return Err(AssetError::Bundle {
                        message: format!("bundle chunk {} hash mismatch", chunk.index),
                    });
                }
                if chunk_loading_policy == BundleChunkLoadingPolicy::Eager {
                    decoded_chunks.insert(chunk.index, decoded);
                }
            }
            chunk_data.insert(chunk.index, compressed);
        }
        let cache_stats = BundleChunkCacheStats {
            policy: chunk_loading_policy,
            chunks_total: manifest.chunks.len(),
            max_decoded_chunks: chunk_loading_policy.max_decoded_chunks(),
            decoded_chunks: decoded_chunks.len(),
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            prefetched_chunks: 0,
            decoded_bytes: decoded_chunks
                .values()
                .map(|chunk| chunk.len() as u64)
                .sum(),
        };
        let mut path_to_entry = HashMap::new();
        for (index, entry) in manifest.entries.iter().enumerate() {
            if let Some(path) = &entry.path {
                path_to_entry.insert(path.clone(), index);
            }
            let Some(chunk) = manifest.chunk(entry.chunk_index) else {
                return Err(AssetError::Bundle {
                    message: format!(
                        "bundle entry {:?} references missing chunk {}",
                        entry.id, entry.chunk_index
                    ),
                });
            };
            let end = entry.offset.saturating_add(entry.length);
            if end > chunk.uncompressed_length {
                return Err(AssetError::Bundle {
                    message: format!("bundle entry {:?} exceeds chunk {}", entry.id, chunk.index),
                });
            }
            if chunk_loading_policy == BundleChunkLoadingPolicy::Eager {
                let chunk_bytes = decoded_chunks
                    .get(&entry.chunk_index)
                    .expect("bundle entry chunk was decoded before entry validation");
                let start = entry.offset as usize;
                let end = end as usize;
                let actual_hash = ContentHash(stable_hash(&chunk_bytes[start..end]));
                if entry.content_hash != actual_hash {
                    return Err(AssetError::Bundle {
                        message: format!("bundle entry {:?} hash mismatch", entry.id),
                    });
                }
            }
        }
        Ok(Self {
            manifest,
            chunk_loading_policy,
            chunk_data,
            decoded_chunk_lru: Arc::new(Mutex::new(decoded_chunks.keys().copied().collect())),
            decoded_chunks: Arc::new(Mutex::new(decoded_chunks)),
            cache_stats: Arc::new(Mutex::new(cache_stats)),
            path_to_entry,
        })
    }

    pub fn manifest(&self) -> &BundleManifest {
        &self.manifest
    }

    pub fn read_entry(&self, id: AssetId) -> AssetResult<Vec<u8>> {
        let entry = self
            .manifest
            .entry(id)
            .ok_or(AssetError::AssetNotFound { id })?;
        Ok(self.entry_bytes(entry)?.0)
    }

    pub fn read_entry_range(&self, id: AssetId, offset: u64, length: u64) -> AssetResult<Vec<u8>> {
        let entry = self
            .manifest
            .entry(id)
            .ok_or(AssetError::AssetNotFound { id })?;
        Ok(range_bytes(&self.entry_bytes(entry)?.0, offset, length))
    }

    pub fn read_path(&self, path: &AssetPath) -> AssetResult<Vec<u8>> {
        let entry = self.entry_for_path(path)?;
        Ok(self.entry_bytes(entry)?.0)
    }

    pub fn read_path_range(
        &self,
        path: &AssetPath,
        offset: u64,
        length: u64,
    ) -> AssetResult<Vec<u8>> {
        let entry = self.entry_for_path(path)?;
        Ok(range_bytes(&self.entry_bytes(entry)?.0, offset, length))
    }

    pub fn read_path_with_report(
        &self,
        path: &AssetPath,
    ) -> AssetResult<(Vec<u8>, BundleChunkReadReport)> {
        let entry = self.entry_for_path(path)?;
        let (bytes, cache_status) = self.entry_bytes(entry)?;
        let report =
            self.entry_read_report(entry, 0, entry.length, bytes.len() as u64, cache_status);
        Ok((bytes, report))
    }

    pub fn read_path_range_with_report(
        &self,
        path: &AssetPath,
        offset: u64,
        length: u64,
    ) -> AssetResult<(Vec<u8>, BundleChunkReadReport)> {
        let entry = self.entry_for_path(path)?;
        let (entry_bytes, cache_status) = self.entry_bytes(entry)?;
        let bytes = range_bytes(&entry_bytes, offset, length);
        let report =
            self.entry_read_report(entry, offset, length, bytes.len() as u64, cache_status);
        Ok((bytes, report))
    }

    pub fn chunk_loading_policy(&self) -> BundleChunkLoadingPolicy {
        self.chunk_loading_policy
    }

    pub fn chunk_cache_stats(&self) -> BundleChunkCacheStats {
        let decoded_chunks = self
            .decoded_chunks
            .lock()
            .expect("bundle decoded chunk cache mutex poisoned");
        let mut stats = self
            .cache_stats
            .lock()
            .expect("bundle chunk cache stats mutex poisoned")
            .clone();
        stats.decoded_chunks = decoded_chunks.len();
        stats.decoded_bytes = decoded_chunks
            .values()
            .map(|chunk| chunk.len() as u64)
            .sum();
        stats
    }

    pub fn prefetch_chunk(&self, chunk_index: u32) -> AssetResult<BundleChunkPrefetchReport> {
        self.prefetch_chunks(&[chunk_index])
    }

    pub fn prefetch_chunks(&self, chunk_indices: &[u32]) -> AssetResult<BundleChunkPrefetchReport> {
        let mut report = BundleChunkPrefetchReport {
            requested_chunks: chunk_indices.to_vec(),
            decoded_chunks: Vec::new(),
            cache_hits: 0,
            cache_misses: 0,
            evicted_chunks: Vec::new(),
        };
        let mut processed = HashSet::new();
        for chunk_index in chunk_indices {
            if !processed.insert(*chunk_index) {
                continue;
            }
            let (_, cache_status, evicted_chunks) = self.decode_chunk_by_index(*chunk_index)?;
            match cache_status {
                BundleChunkCacheStatus::Preloaded | BundleChunkCacheStatus::Hit => {
                    report.cache_hits += 1;
                }
                BundleChunkCacheStatus::Miss => {
                    report.cache_misses += 1;
                    report.decoded_chunks.push(*chunk_index);
                }
            }
            report.evicted_chunks.extend(evicted_chunks);
        }
        if !report.decoded_chunks.is_empty() {
            self.cache_stats
                .lock()
                .expect("bundle chunk cache stats mutex poisoned")
                .prefetched_chunks += report.decoded_chunks.len() as u64;
        }
        Ok(report)
    }

    pub fn prefetch_path(&self, path: &AssetPath) -> AssetResult<BundleChunkPrefetchReport> {
        let entry = self.entry_for_path(path)?;
        self.prefetch_chunk(entry.chunk_index)
    }

    pub fn prefetch_paths(&self, paths: &[AssetPath]) -> AssetResult<BundleChunkPrefetchReport> {
        let mut chunk_indices = Vec::with_capacity(paths.len());
        for path in paths {
            chunk_indices.push(self.entry_for_path(path)?.chunk_index);
        }
        self.prefetch_chunks(&chunk_indices)
    }

    fn entry_for_path(&self, path: &AssetPath) -> AssetResult<&BundleEntry> {
        let index = self
            .path_to_entry
            .get(path)
            .copied()
            .ok_or_else(|| AssetError::PathNotFound { path: path.clone() })?;
        Ok(&self.manifest.entries[index])
    }

    fn entry_bytes(&self, entry: &BundleEntry) -> AssetResult<(Vec<u8>, BundleChunkCacheStatus)> {
        let (chunk_bytes, cache_status) = self.decode_chunk_for_entry(entry)?;
        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        let bytes = chunk_bytes[start..end].to_vec();
        let actual_hash = ContentHash(stable_hash(&bytes));
        if entry.content_hash != actual_hash {
            return Err(AssetError::Bundle {
                message: format!("bundle entry {:?} hash mismatch", entry.id),
            });
        }
        Ok((bytes, cache_status))
    }

    fn decode_chunk_for_entry(
        &self,
        entry: &BundleEntry,
    ) -> AssetResult<(Vec<u8>, BundleChunkCacheStatus)> {
        let (decoded, cache_status, _) = self.decode_chunk_by_index(entry.chunk_index)?;
        Ok((decoded, cache_status))
    }

    fn decode_chunk_by_index(
        &self,
        chunk_index: u32,
    ) -> AssetResult<(Vec<u8>, BundleChunkCacheStatus, Vec<u32>)> {
        let chunk = self
            .manifest
            .chunk(chunk_index)
            .ok_or_else(|| AssetError::Bundle {
                message: format!("bundle chunk {chunk_index} is not in manifest"),
            })?;
        if let Some(decoded) = self
            .decoded_chunks
            .lock()
            .expect("bundle decoded chunk cache mutex poisoned")
            .get(&chunk_index)
            .cloned()
        {
            if self.chunk_loading_policy.is_on_demand_cached() {
                self.touch_decoded_chunk(chunk_index);
                self.cache_stats
                    .lock()
                    .expect("bundle chunk cache stats mutex poisoned")
                    .cache_hits += 1;
                return Ok((decoded, BundleChunkCacheStatus::Hit, Vec::new()));
            }
            return Ok((decoded, BundleChunkCacheStatus::Preloaded, Vec::new()));
        }

        let compressed = self
            .chunk_data
            .get(&chunk_index)
            .ok_or_else(|| AssetError::Bundle {
                message: format!("bundle chunk {chunk_index} data is missing"),
            })?;
        let decoded = decode_bundle_chunk(
            chunk.compression,
            compressed,
            chunk.uncompressed_length,
            chunk.index,
        )?;
        let actual_hash = ContentHash(stable_hash(&decoded));
        if chunk.content_hash != actual_hash {
            return Err(AssetError::Bundle {
                message: format!("bundle chunk {} hash mismatch", chunk.index),
            });
        }
        let evicted_chunks = self.insert_decoded_chunk(chunk_index, decoded.clone());
        {
            let mut stats = self
                .cache_stats
                .lock()
                .expect("bundle chunk cache stats mutex poisoned");
            stats.cache_misses += 1;
            stats.cache_evictions += evicted_chunks.len() as u64;
        }
        Ok((decoded, BundleChunkCacheStatus::Miss, evicted_chunks))
    }

    fn touch_decoded_chunk(&self, chunk_index: u32) {
        let mut lru = self
            .decoded_chunk_lru
            .lock()
            .expect("bundle decoded chunk LRU mutex poisoned");
        if let Some(position) = lru.iter().position(|cached| *cached == chunk_index) {
            lru.remove(position);
        }
        lru.push_back(chunk_index);
    }

    fn insert_decoded_chunk(&self, chunk_index: u32, decoded: Vec<u8>) -> Vec<u32> {
        self.decoded_chunks
            .lock()
            .expect("bundle decoded chunk cache mutex poisoned")
            .insert(chunk_index, decoded);
        self.touch_decoded_chunk(chunk_index);
        self.enforce_decoded_chunk_limit()
    }

    fn enforce_decoded_chunk_limit(&self) -> Vec<u32> {
        let Some(max_decoded_chunks) = self.chunk_loading_policy.max_decoded_chunks() else {
            return Vec::new();
        };
        let mut evicted = Vec::new();
        loop {
            let evict = {
                let mut lru = self
                    .decoded_chunk_lru
                    .lock()
                    .expect("bundle decoded chunk LRU mutex poisoned");
                if lru.len() <= max_decoded_chunks {
                    None
                } else {
                    lru.pop_front()
                }
            };
            let Some(chunk_index) = evict else {
                break;
            };
            self.decoded_chunks
                .lock()
                .expect("bundle decoded chunk cache mutex poisoned")
                .remove(&chunk_index);
            evicted.push(chunk_index);
        }
        evicted
    }

    fn entry_read_report(
        &self,
        entry: &BundleEntry,
        range_offset: u64,
        range_length: u64,
        bytes_returned: u64,
        cache_status: BundleChunkCacheStatus,
    ) -> BundleChunkReadReport {
        let chunk = self
            .manifest
            .chunk(entry.chunk_index)
            .expect("bundle entry chunk was validated on read");
        BundleChunkReadReport {
            entry: entry.id,
            path: entry.path.clone(),
            chunk_index: entry.chunk_index,
            chunk_compression: chunk.compression,
            chunk_compressed_length: chunk.compressed_length,
            chunk_uncompressed_length: chunk.uncompressed_length,
            entry_offset: entry.offset,
            entry_length: entry.length,
            range_offset,
            range_length,
            bytes_returned,
            cache_status,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BundleAssetIo {
    reader: BundleReader,
}

impl BundleAssetIo {
    pub fn new(reader: BundleReader) -> Self {
        Self { reader }
    }

    pub fn from_bytes(bytes: &[u8]) -> AssetResult<Self> {
        Ok(Self::new(BundleReader::from_bytes(bytes)?))
    }

    pub fn from_bytes_with_loading_policy(
        bytes: &[u8],
        chunk_loading_policy: BundleChunkLoadingPolicy,
    ) -> AssetResult<Self> {
        Ok(Self::new(BundleReader::from_bytes_with_loading_policy(
            bytes,
            chunk_loading_policy,
        )?))
    }

    pub fn manifest(&self) -> &BundleManifest {
        self.reader.manifest()
    }

    pub fn read_with_report(&self, path: &str) -> AssetResult<(Vec<u8>, BundleChunkReadReport)> {
        self.reader.read_path_with_report(&AssetPath::parse(path))
    }

    pub fn read_range_with_report(
        &self,
        path: &str,
        offset: u64,
        length: u64,
    ) -> AssetResult<(Vec<u8>, BundleChunkReadReport)> {
        self.reader
            .read_path_range_with_report(&AssetPath::parse(path), offset, length)
    }

    pub fn chunk_cache_stats(&self) -> BundleChunkCacheStats {
        self.reader.chunk_cache_stats()
    }

    pub fn prefetch_chunk(&self, chunk_index: u32) -> AssetResult<BundleChunkPrefetchReport> {
        self.reader.prefetch_chunk(chunk_index)
    }

    pub fn prefetch_chunks(&self, chunk_indices: &[u32]) -> AssetResult<BundleChunkPrefetchReport> {
        self.reader.prefetch_chunks(chunk_indices)
    }

    pub fn prefetch_path(&self, path: &str) -> AssetResult<BundleChunkPrefetchReport> {
        self.reader.prefetch_path(&AssetPath::parse(path))
    }

    pub fn prefetch_paths(&self, paths: &[&str]) -> AssetResult<BundleChunkPrefetchReport> {
        let paths = paths
            .iter()
            .map(|path| AssetPath::parse(path))
            .collect::<Vec<_>>();
        self.reader.prefetch_paths(&paths)
    }
}

impl AssetIo for BundleAssetIo {
    fn exists(&self, path: &str) -> bool {
        self.reader
            .manifest
            .entry_by_path(&AssetPath::parse(path))
            .is_some()
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, AssetIoError> {
        self.reader
            .read_path(&AssetPath::parse(path))
            .map_err(|error| bundle_error_to_io_error(error, AssetIoAction::Read, path))
    }

    fn read_range(&self, path: &str, offset: u64, length: u64) -> Result<Vec<u8>, AssetIoError> {
        self.reader
            .read_path_range(&AssetPath::parse(path), offset, length)
            .map_err(|error| bundle_error_to_io_error(error, AssetIoAction::ReadRange, path))
    }

    fn metadata(&self, path: &str) -> Result<AssetIoMetadata, AssetIoError> {
        let asset_path = AssetPath::parse(path);
        let entry = self
            .reader
            .entry_for_path(&asset_path)
            .map_err(|error| bundle_error_to_io_error(error, AssetIoAction::Metadata, path))?;
        Ok(AssetIoMetadata {
            path: path.to_owned(),
            size: entry.length,
            modified_time: None,
            hash: Some(entry.content_hash),
        })
    }

    fn list(&self, directory: &str) -> Result<Vec<String>, AssetIoError> {
        let prefix = directory.trim_matches('/');
        let mut entries = self
            .reader
            .manifest
            .entries
            .iter()
            .filter_map(|entry| entry.path.as_ref())
            .map(AssetPath::display_string)
            .filter(|path| prefix.is_empty() || path.starts_with(prefix))
            .collect::<Vec<_>>();
        entries.sort();
        Ok(entries)
    }
}

fn bundle_error_to_io_error(
    error: AssetError,
    action: AssetIoAction,
    requested_path: &str,
) -> AssetIoError {
    match error {
        AssetError::PathNotFound { path } => AssetIoError::NotFound {
            path: path.display_string(),
            action,
        },
        AssetError::AssetNotFound { id } => AssetIoError::NotFound {
            path: format!("{id:?}"),
            action,
        },
        other => AssetIoError::ReadFailed {
            path: requested_path.to_owned(),
            action,
            message: other.to_string(),
        },
    }
}

fn validate_chunk_policy(policy: BundleChunkPartitionPolicy) -> AssetResult<()> {
    match policy {
        BundleChunkPartitionPolicy::SingleChunk => Ok(()),
        BundleChunkPartitionPolicy::MaxUncompressedBytes(0) => Err(AssetError::Bundle {
            message: "bundle max chunk size must be greater than zero".to_owned(),
        }),
        BundleChunkPartitionPolicy::MaxUncompressedBytes(_) => Ok(()),
    }
}

fn validate_chunk_loading_policy(policy: BundleChunkLoadingPolicy) -> AssetResult<()> {
    match policy {
        BundleChunkLoadingPolicy::OnDemandCachedLimited {
            max_decoded_chunks: 0,
        } => Err(AssetError::Bundle {
            message: "bundle max decoded chunks must be greater than zero".to_owned(),
        }),
        BundleChunkLoadingPolicy::Eager
        | BundleChunkLoadingPolicy::OnDemandCached
        | BundleChunkLoadingPolicy::OnDemandCachedLimited { .. } => Ok(()),
    }
}

fn partition_assets(
    assets: &[BundleAsset],
    policy: BundleChunkPartitionPolicy,
) -> AssetResult<Vec<Vec<usize>>> {
    if assets.is_empty() {
        return Ok(Vec::new());
    }
    match policy {
        BundleChunkPartitionPolicy::SingleChunk => Ok(vec![(0..assets.len()).collect()]),
        BundleChunkPartitionPolicy::MaxUncompressedBytes(max_bytes) => {
            let mut groups = Vec::new();
            let mut current = Vec::new();
            let mut current_len = 0_usize;
            for (index, asset) in assets.iter().enumerate() {
                let asset_len = asset.bytes.len();
                if !current.is_empty()
                    && current_len
                        .checked_add(asset_len)
                        .ok_or_else(|| AssetError::Bundle {
                            message: "bundle chunk partition length overflow".to_owned(),
                        })?
                        > max_bytes
                {
                    groups.push(current);
                    current = Vec::new();
                    current_len = 0;
                }
                current.push(index);
                current_len =
                    current_len
                        .checked_add(asset_len)
                        .ok_or_else(|| AssetError::Bundle {
                            message: "bundle chunk partition length overflow".to_owned(),
                        })?;
            }
            if !current.is_empty() {
                groups.push(current);
            }
            Ok(groups)
        }
    }
}

fn range_bytes(bytes: &[u8], offset: u64, length: u64) -> Vec<u8> {
    let offset = offset as usize;
    let length = length as usize;
    if offset >= bytes.len() {
        return Vec::new();
    }
    bytes[offset..bytes.len().min(offset + length)].to_vec()
}

fn serialize_manifest(manifest: &BundleManifest) -> String {
    let mut lines = vec![
        "NGA_BUNDLE_V2".to_owned(),
        format!("name={}", manifest.name),
        format!("compression={}", compression_to_str(manifest.compression)),
        format!("chunks={}", manifest.chunks.len()),
    ];
    for chunk in &manifest.chunks {
        lines.push(
            [
                "chunk".to_owned(),
                chunk.index.to_string(),
                chunk.offset.to_string(),
                chunk.compressed_length.to_string(),
                chunk.uncompressed_length.to_string(),
                compression_to_str(chunk.compression).to_owned(),
                chunk.content_hash.0.to_string(),
            ]
            .join("|"),
        );
    }
    lines.push(format!("entries={}", manifest.entries.len()));
    for entry in &manifest.entries {
        lines.push(serialize_entry_v2(entry));
    }
    lines.join("\n")
}

fn serialize_entry_v2(entry: &BundleEntry) -> String {
    [
        "entry".to_owned(),
        entry.id.raw().to_string(),
        entry.asset_type.raw().to_string(),
        entry
            .path
            .as_ref()
            .map(AssetPath::display_string)
            .unwrap_or_default(),
        entry.chunk_index.to_string(),
        entry.offset.to_string(),
        entry.length.to_string(),
        entry.content_hash.0.to_string(),
        entry
            .dependencies
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join(","),
    ]
    .join("|")
}

fn deserialize_manifest(text: &str) -> AssetResult<BundleManifest> {
    let mut lines = text.lines();
    match lines.next() {
        Some("NGA_BUNDLE_V2") => deserialize_manifest_v2(lines),
        Some("NGA_BUNDLE_V1") => deserialize_manifest_v1(lines),
        _ => Err(AssetError::Bundle {
            message: "invalid bundle header".to_owned(),
        }),
    }
}

fn deserialize_manifest_v2<'a>(
    mut lines: impl Iterator<Item = &'a str>,
) -> AssetResult<BundleManifest> {
    let name = parse_prefixed_line(lines.next(), "name=")?;
    let compression = parse_compression(parse_prefixed_line(lines.next(), "compression=")?)?;
    let chunk_count: usize = parse_prefixed_line(lines.next(), "chunks=")?
        .parse()
        .map_err(|error| AssetError::Bundle {
            message: format!("invalid bundle chunk count: {error}"),
        })?;
    let mut chunks = Vec::with_capacity(chunk_count);
    for index in 0..chunk_count {
        let line = lines.next().ok_or_else(|| AssetError::Bundle {
            message: format!("missing bundle chunk line {index}"),
        })?;
        chunks.push(deserialize_chunk(line)?);
    }
    let entry_count: usize = parse_prefixed_line(lines.next(), "entries=")?
        .parse()
        .map_err(|error| AssetError::Bundle {
            message: format!("invalid bundle entry count: {error}"),
        })?;
    let mut entries = Vec::with_capacity(entry_count);
    for index in 0..entry_count {
        let line = lines.next().ok_or_else(|| AssetError::Bundle {
            message: format!("missing bundle entry line {index}"),
        })?;
        entries.push(deserialize_entry_v2(line)?);
    }
    Ok(BundleManifest {
        name: name.to_owned(),
        compression,
        chunks,
        entries,
    })
}

fn deserialize_manifest_v1<'a>(
    mut lines: impl Iterator<Item = &'a str>,
) -> AssetResult<BundleManifest> {
    let name = parse_prefixed_line(lines.next(), "name=")?;
    let compression = parse_compression(parse_prefixed_line(lines.next(), "compression=")?)?;
    let entry_count: usize = parse_prefixed_line(lines.next(), "entries=")?
        .parse()
        .map_err(|error| AssetError::Bundle {
            message: format!("invalid bundle entry count: {error}"),
        })?;
    let mut entries = Vec::with_capacity(entry_count);
    for index in 0..entry_count {
        let line = lines.next().ok_or_else(|| AssetError::Bundle {
            message: format!("missing bundle entry line {index}"),
        })?;
        entries.push(deserialize_entry_v1(line)?);
    }
    let chunk_length = entries
        .iter()
        .map(|entry| entry.offset.saturating_add(entry.length))
        .max()
        .unwrap_or(0);
    let chunks = if chunk_length == 0 {
        Vec::new()
    } else {
        vec![BundleChunk {
            index: 0,
            offset: 0,
            compressed_length: chunk_length,
            uncompressed_length: chunk_length,
            compression,
            content_hash: ContentHash(0),
        }]
    };
    Ok(BundleManifest {
        name: name.to_owned(),
        compression,
        chunks,
        entries,
    })
}

fn deserialize_chunk(line: &str) -> AssetResult<BundleChunk> {
    let fields = line.split('|').collect::<Vec<_>>();
    if fields.len() != 7 || fields[0] != "chunk" {
        return Err(AssetError::Bundle {
            message: "invalid bundle chunk line".to_owned(),
        });
    }
    Ok(BundleChunk {
        index: parse_u32(fields[1], "chunk index")?,
        offset: parse_u64(fields[2], "chunk offset")?,
        compressed_length: parse_u64(fields[3], "chunk compressed length")?,
        uncompressed_length: parse_u64(fields[4], "chunk uncompressed length")?,
        compression: parse_compression(fields[5])?,
        content_hash: ContentHash(parse_u64(fields[6], "chunk hash")?),
    })
}

fn deserialize_entry_v1(line: &str) -> AssetResult<BundleEntry> {
    let fields = line.split('|').collect::<Vec<_>>();
    if fields.len() != 8 || fields[0] != "entry" {
        return Err(AssetError::Bundle {
            message: "invalid bundle entry line".to_owned(),
        });
    }
    Ok(BundleEntry {
        id: AssetId::from_u128(parse_u128(fields[1], "entry id")?),
        asset_type: AssetTypeId::from_u128(parse_u128(fields[2], "entry asset type")?),
        path: (!fields[3].is_empty()).then(|| AssetPath::parse(fields[3])),
        chunk_index: 0,
        offset: parse_u64(fields[4], "entry offset")?,
        length: parse_u64(fields[5], "entry length")?,
        content_hash: ContentHash(parse_u64(fields[6], "entry hash")?),
        dependencies: if fields[7].is_empty() {
            Vec::new()
        } else {
            fields[7]
                .split(',')
                .map(|value| parse_u128(value, "entry dependency").map(AssetId::from_u128))
                .collect::<AssetResult<Vec<_>>>()?
        },
    })
}

fn deserialize_entry_v2(line: &str) -> AssetResult<BundleEntry> {
    let fields = line.split('|').collect::<Vec<_>>();
    if fields.len() != 9 || fields[0] != "entry" {
        return Err(AssetError::Bundle {
            message: "invalid bundle entry line".to_owned(),
        });
    }
    Ok(BundleEntry {
        id: AssetId::from_u128(parse_u128(fields[1], "entry id")?),
        asset_type: AssetTypeId::from_u128(parse_u128(fields[2], "entry asset type")?),
        path: (!fields[3].is_empty()).then(|| AssetPath::parse(fields[3])),
        chunk_index: parse_u32(fields[4], "entry chunk index")?,
        offset: parse_u64(fields[5], "entry offset")?,
        length: parse_u64(fields[6], "entry length")?,
        content_hash: ContentHash(parse_u64(fields[7], "entry hash")?),
        dependencies: if fields[8].is_empty() {
            Vec::new()
        } else {
            fields[8]
                .split(',')
                .map(|value| parse_u128(value, "entry dependency").map(AssetId::from_u128))
                .collect::<AssetResult<Vec<_>>>()?
        },
    })
}

fn parse_prefixed_line<'a>(line: Option<&'a str>, prefix: &str) -> AssetResult<&'a str> {
    let line = line.ok_or_else(|| AssetError::Bundle {
        message: format!("missing `{prefix}` line"),
    })?;
    line.strip_prefix(prefix).ok_or_else(|| AssetError::Bundle {
        message: format!("expected `{prefix}` line"),
    })
}

fn parse_compression(value: &str) -> AssetResult<CompressionKind> {
    match value {
        "none" => Ok(CompressionKind::None),
        "rle" => Ok(CompressionKind::Rle),
        "zstd" => Ok(CompressionKind::Zstd),
        other => Err(AssetError::Bundle {
            message: format!("unknown compression `{other}`"),
        }),
    }
}

fn parse_asset_io_layer_kind(value: &str) -> AssetResult<AssetIoLayerKind> {
    match value {
        "source" => Ok(AssetIoLayerKind::Source),
        "mod" => Ok(AssetIoLayerKind::Mod),
        "patch" => Ok(AssetIoLayerKind::Patch),
        "bundle" => Ok(AssetIoLayerKind::Bundle),
        "base_bundle" => Ok(AssetIoLayerKind::BaseBundle),
        "memory" => Ok(AssetIoLayerKind::Memory),
        "filesystem" => Ok(AssetIoLayerKind::FileSystem),
        "custom" => Ok(AssetIoLayerKind::Custom),
        other => Err(AssetError::Bundle {
            message: format!("unknown asset package layer kind `{other}`"),
        }),
    }
}

fn compression_to_str(compression: CompressionKind) -> &'static str {
    match compression {
        CompressionKind::None => "none",
        CompressionKind::Rle => "rle",
        CompressionKind::Zstd => "zstd",
    }
}

fn asset_io_layer_kind_to_str(kind: AssetIoLayerKind) -> &'static str {
    match kind {
        AssetIoLayerKind::Source => "source",
        AssetIoLayerKind::Mod => "mod",
        AssetIoLayerKind::Patch => "patch",
        AssetIoLayerKind::Bundle => "bundle",
        AssetIoLayerKind::BaseBundle => "base_bundle",
        AssetIoLayerKind::Memory => "memory",
        AssetIoLayerKind::FileSystem => "filesystem",
        AssetIoLayerKind::Custom => "custom",
    }
}

fn parse_bool(value: &str, name: &str) -> AssetResult<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(AssetError::Bundle {
            message: format!("invalid {name}: `{other}`"),
        }),
    }
}

fn asset_override_for_entries(
    path: AssetPath,
    winner_package: &AssetPackageRecord,
    winner_entry: &BundleEntry,
    shadowed_package: &AssetPackageRecord,
    shadowed_entry: &BundleEntry,
    providers: &HashMap<AssetId, AssetPackageLayerInfo>,
) -> AssetPackageAssetOverride {
    let winner_asset = AssetPackageAssetInfo::from_entry(winner_entry);
    let shadowed_asset = AssetPackageAssetInfo::from_entry(shadowed_entry);
    let winner_dependency_providers = dependency_providers(winner_entry, providers);
    let shadowed_dependency_providers = dependency_providers(shadowed_entry, providers);
    let mut issues = Vec::new();
    if winner_asset.id != shadowed_asset.id {
        issues.push(AssetPackageAssetOverrideIssueKind::AssetIdChanged);
    }
    if winner_asset.asset_type != shadowed_asset.asset_type {
        issues.push(AssetPackageAssetOverrideIssueKind::AssetTypeChanged);
    }
    if winner_asset.content_hash != shadowed_asset.content_hash {
        issues.push(AssetPackageAssetOverrideIssueKind::ContentHashChanged);
    }
    if winner_asset.dependencies != shadowed_asset.dependencies {
        issues.push(AssetPackageAssetOverrideIssueKind::DependenciesChanged);
    }
    if winner_dependency_providers != shadowed_dependency_providers {
        issues.push(AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged);
    }
    AssetPackageAssetOverride {
        path,
        winner: AssetPackageLayerInfo::from_record(winner_package),
        shadowed: AssetPackageLayerInfo::from_record(shadowed_package),
        winner_asset,
        shadowed_asset,
        winner_dependency_providers,
        shadowed_dependency_providers,
        issues,
    }
}

fn dependency_providers(
    entry: &BundleEntry,
    providers: &HashMap<AssetId, AssetPackageLayerInfo>,
) -> Vec<AssetPackageDependencyProvider> {
    entry
        .dependencies
        .iter()
        .map(|dependency| AssetPackageDependencyProvider {
            dependency: *dependency,
            provider: providers.get(dependency).cloned(),
        })
        .collect()
}

fn asset_override_issue_is_incompatible(
    issue: AssetPackageAssetOverrideIssueKind,
    policy: AssetPackageAssetCompatibilityPolicy,
) -> bool {
    match issue {
        AssetPackageAssetOverrideIssueKind::AssetIdChanged => policy.require_stable_asset_ids,
        AssetPackageAssetOverrideIssueKind::AssetTypeChanged => policy.require_matching_asset_types,
        AssetPackageAssetOverrideIssueKind::ContentHashChanged => {
            policy.require_matching_content_hashes
        }
        AssetPackageAssetOverrideIssueKind::DependenciesChanged => {
            policy.require_matching_dependencies
        }
        AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged => {
            policy.require_matching_dependency_providers
        }
    }
}

fn asset_override_compatibility_issue_kind(
    issue: AssetPackageAssetOverrideIssueKind,
) -> AssetPackageCompatibilityIssueKind {
    match issue {
        AssetPackageAssetOverrideIssueKind::AssetIdChanged => {
            AssetPackageCompatibilityIssueKind::AssetIdChanged
        }
        AssetPackageAssetOverrideIssueKind::AssetTypeChanged => {
            AssetPackageCompatibilityIssueKind::AssetTypeChanged
        }
        AssetPackageAssetOverrideIssueKind::ContentHashChanged => {
            AssetPackageCompatibilityIssueKind::AssetContentHashChanged
        }
        AssetPackageAssetOverrideIssueKind::DependenciesChanged => {
            AssetPackageCompatibilityIssueKind::AssetDependenciesChanged
        }
        AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged => {
            AssetPackageCompatibilityIssueKind::AssetDependencyProvidersChanged
        }
    }
}

fn asset_override_issue_message(
    asset_override: &AssetPackageAssetOverride,
    issue: AssetPackageAssetOverrideIssueKind,
) -> String {
    match issue {
        AssetPackageAssetOverrideIssueKind::AssetIdChanged => format!(
            "package `{}` overrides `{}` from package `{}` with asset id {:?}, expected {:?}",
            asset_override.winner.name,
            asset_override.path.display_string(),
            asset_override.shadowed.name,
            asset_override.winner_asset.id,
            asset_override.shadowed_asset.id
        ),
        AssetPackageAssetOverrideIssueKind::AssetTypeChanged => format!(
            "package `{}` overrides `{}` from package `{}` with asset type {:?}, expected {:?}",
            asset_override.winner.name,
            asset_override.path.display_string(),
            asset_override.shadowed.name,
            asset_override.winner_asset.asset_type,
            asset_override.shadowed_asset.asset_type
        ),
        AssetPackageAssetOverrideIssueKind::ContentHashChanged => format!(
            "package `{}` overrides `{}` from package `{}` with content hash {:?}, expected {:?}",
            asset_override.winner.name,
            asset_override.path.display_string(),
            asset_override.shadowed.name,
            asset_override.winner_asset.content_hash,
            asset_override.shadowed_asset.content_hash
        ),
        AssetPackageAssetOverrideIssueKind::DependenciesChanged => format!(
            "package `{}` overrides `{}` from package `{}` with dependencies {:?}, expected {:?}",
            asset_override.winner.name,
            asset_override.path.display_string(),
            asset_override.shadowed.name,
            asset_override.winner_asset.dependencies,
            asset_override.shadowed_asset.dependencies
        ),
        AssetPackageAssetOverrideIssueKind::DependencyProvidersChanged => format!(
            "package `{}` overrides `{}` from package `{}` with dependency providers {:?}, expected {:?}",
            asset_override.winner.name,
            asset_override.path.display_string(),
            asset_override.shadowed.name,
            asset_override.winner_dependency_providers,
            asset_override.shadowed_dependency_providers
        ),
    }
}

fn package_changed(previous: &AssetPackageRecord, next: &AssetPackageRecord) -> bool {
    previous.bundle_id != next.bundle_id
        || previous.kind != next.kind
        || previous.priority != next.priority
        || previous.bundle_path != next.bundle_path
        || previous.package_version != next.package_version
        || previous.minimum_runtime_version != next.minimum_runtime_version
        || previous.package_dependencies != next.package_dependencies
        || previous.manifest != next.manifest
}

fn package_update_change(
    previous: Option<&AssetPackageRecord>,
    next: Option<&AssetPackageRecord>,
) -> AssetPackageUpdateChange {
    let name = next
        .or(previous)
        .map(|package| package.name.clone())
        .unwrap_or_default();
    AssetPackageUpdateChange {
        name,
        previous_version: previous.map(|package| package.package_version),
        next_version: next.map(|package| package.package_version),
    }
}

fn resolve_package_artifact_path(root: &Path, bundle_path: &str) -> AssetResult<PathBuf> {
    validate_package_token("package bundle path", bundle_path)?;
    let normalized = bundle_path.replace('\\', "/");
    let relative = Path::new(&normalized);
    if relative.is_absolute() {
        return Err(AssetError::Bundle {
            message: format!("asset package bundle path `{bundle_path}` must be relative"),
        });
    }
    let mut resolved = PathBuf::from(root);
    for component in relative.components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AssetError::Bundle {
                    message: format!(
                        "asset package bundle path `{bundle_path}` cannot escape the artifact root"
                    ),
                });
            }
        }
    }
    Ok(resolved)
}

fn serialize_package_dependencies(dependencies: &[AssetPackageDependency]) -> String {
    dependencies
        .iter()
        .map(|dependency| {
            format!(
                "{}:{}:{}",
                dependency.package,
                dependency.min_version,
                dependency
                    .max_version
                    .map(|version| version.to_string())
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn deserialize_package_dependencies(value: &str) -> AssetResult<Vec<AssetPackageDependency>> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|entry| {
            let fields = entry.split(':').collect::<Vec<_>>();
            if fields.len() != 3 {
                return Err(AssetError::Bundle {
                    message: "invalid asset package dependency field".to_owned(),
                });
            }
            let min_version = parse_u32(fields[1], "asset package dependency min version")?;
            let max_version = if fields[2].is_empty() {
                None
            } else {
                Some(parse_u32(
                    fields[2],
                    "asset package dependency max version",
                )?)
            };
            Ok(AssetPackageDependency {
                package: fields[0].to_owned(),
                min_version,
                max_version,
            })
        })
        .collect()
}

fn validate_package_token(name: &str, value: &str) -> AssetResult<()> {
    if value.trim().is_empty() {
        return Err(AssetError::Bundle {
            message: format!("asset {name} cannot be empty"),
        });
    }
    if value.contains('|') || value.contains('\n') || value.contains('\r') {
        return Err(AssetError::Bundle {
            message: format!("asset {name} cannot contain registry separators"),
        });
    }
    Ok(())
}

fn validate_package_dependencies(package: &AssetPackageRecord) -> AssetResult<()> {
    let mut dependencies = HashSet::new();
    for dependency in &package.package_dependencies {
        validate_package_dependency_name(&dependency.package)?;
        if dependency.package == package.name {
            return Err(AssetError::Bundle {
                message: format!("asset package `{}` cannot depend on itself", package.name),
            });
        }
        if dependency.min_version == 0 {
            return Err(AssetError::Bundle {
                message: format!(
                    "asset package `{}` dependency `{}` min version must be greater than zero",
                    package.name, dependency.package
                ),
            });
        }
        if let Some(max_version) = dependency.max_version {
            if max_version < dependency.min_version {
                return Err(AssetError::Bundle {
                    message: format!(
                        "asset package `{}` dependency `{}` max version is lower than min version",
                        package.name, dependency.package
                    ),
                });
            }
        }
        if !dependencies.insert(dependency.package.clone()) {
            return Err(AssetError::Bundle {
                message: format!(
                    "asset package `{}` has duplicate dependency `{}`",
                    package.name, dependency.package
                ),
            });
        }
    }
    Ok(())
}

fn validate_package_dependency_name(value: &str) -> AssetResult<()> {
    validate_package_token("package dependency", value)?;
    if value.contains(',') || value.contains(':') {
        return Err(AssetError::Bundle {
            message: "asset package dependency cannot contain dependency separators".to_owned(),
        });
    }
    Ok(())
}

fn validate_package_manifest_paths(package: &AssetPackageRecord) -> AssetResult<()> {
    let mut paths = HashSet::new();
    for entry in &package.manifest.entries {
        let Some(path) = &entry.path else {
            continue;
        };
        if !paths.insert(path.clone()) {
            return Err(AssetError::Bundle {
                message: format!(
                    "asset package `{}` manifest has duplicate path `{}`",
                    package.name,
                    path.display_string()
                ),
            });
        }
    }
    Ok(())
}

fn require_bundle_compression(compression: CompressionKind, context: &str) -> AssetResult<()> {
    let report = BundleCompressionCodecReport::for_compression(compression);
    if report.supported {
        Ok(())
    } else {
        Err(unsupported_bundle_compression(compression, context))
    }
}

fn encode_bundle_chunk(compression: CompressionKind, bytes: &[u8]) -> AssetResult<Vec<u8>> {
    match compression {
        CompressionKind::None => Ok(bytes.to_vec()),
        CompressionKind::Rle => Ok(encode_rle(bytes)),
        CompressionKind::Zstd => encode_zstd(bytes),
    }
}

fn decode_bundle_chunk(
    compression: CompressionKind,
    bytes: &[u8],
    uncompressed_length: u64,
    chunk_index: u32,
) -> AssetResult<Vec<u8>> {
    match compression {
        CompressionKind::None => {
            if bytes.len() as u64 != uncompressed_length {
                return Err(AssetError::Bundle {
                    message: format!(
                        "uncompressed bundle chunk {chunk_index} has mismatched compressed and uncompressed lengths"
                    ),
                });
            }
            Ok(bytes.to_vec())
        }
        CompressionKind::Rle => decode_rle(bytes, uncompressed_length, chunk_index),
        CompressionKind::Zstd => decode_zstd(bytes, uncompressed_length, chunk_index),
    }
}

fn encode_rle(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let value = bytes[index];
        let mut run = 1_usize;
        while index + run < bytes.len() && bytes[index + run] == value && run < u8::MAX as usize {
            run += 1;
        }
        encoded.push(run as u8);
        encoded.push(value);
        index += run;
    }
    encoded
}

fn decode_rle(bytes: &[u8], expected_length: u64, chunk_index: u32) -> AssetResult<Vec<u8>> {
    if bytes.len() % 2 != 0 {
        return Err(AssetError::Bundle {
            message: format!("rle bundle chunk {chunk_index} has a truncated run"),
        });
    }
    let expected_length = usize::try_from(expected_length).map_err(|error| AssetError::Bundle {
        message: format!("rle bundle chunk {chunk_index} length is too large: {error}"),
    })?;
    let mut decoded = Vec::with_capacity(expected_length);
    for pair in bytes.chunks_exact(2) {
        let run = usize::from(pair[0]);
        if run == 0 {
            return Err(AssetError::Bundle {
                message: format!("rle bundle chunk {chunk_index} contains a zero-length run"),
            });
        }
        if decoded.len().saturating_add(run) > expected_length {
            return Err(AssetError::Bundle {
                message: format!("rle bundle chunk {chunk_index} expands past its manifest length"),
            });
        }
        decoded.extend(std::iter::repeat(pair[1]).take(run));
    }
    if decoded.len() != expected_length {
        return Err(AssetError::Bundle {
            message: format!("rle bundle chunk {chunk_index} expanded to the wrong length"),
        });
    }
    Ok(decoded)
}

#[cfg(feature = "zstd")]
fn encode_zstd(bytes: &[u8]) -> AssetResult<Vec<u8>> {
    Ok(ruzstd::encoding::compress_to_vec(
        bytes,
        ruzstd::encoding::CompressionLevel::Fastest,
    ))
}

#[cfg(not(feature = "zstd"))]
fn encode_zstd(_bytes: &[u8]) -> AssetResult<Vec<u8>> {
    Err(unsupported_bundle_compression(
        CompressionKind::Zstd,
        "writer",
    ))
}

#[cfg(feature = "zstd")]
fn decode_zstd(bytes: &[u8], expected_length: u64, chunk_index: u32) -> AssetResult<Vec<u8>> {
    use ruzstd::io::Read as _;

    let expected_length = usize::try_from(expected_length).map_err(|error| AssetError::Bundle {
        message: format!("zstd bundle chunk {chunk_index} length is too large: {error}"),
    })?;
    let mut source = bytes;
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(&mut source).map_err(|error| {
        AssetError::Bundle {
            message: format!("zstd bundle chunk {chunk_index} failed to decode: {error}"),
        }
    })?;
    let mut decoded = Vec::with_capacity(expected_length);
    decoder
        .read_to_end(&mut decoded)
        .map_err(|error| AssetError::Bundle {
            message: format!("zstd bundle chunk {chunk_index} failed to decode: {error}"),
        })?;
    if decoded.len() != expected_length {
        return Err(AssetError::Bundle {
            message: format!("zstd bundle chunk {chunk_index} expanded to the wrong length"),
        });
    }
    Ok(decoded)
}

#[cfg(not(feature = "zstd"))]
fn decode_zstd(_bytes: &[u8], _expected_length: u64, chunk_index: u32) -> AssetResult<Vec<u8>> {
    Err(unsupported_bundle_compression(
        CompressionKind::Zstd,
        &format!("chunk {chunk_index}"),
    ))
}

fn unsupported_bundle_compression(compression: CompressionKind, context: &str) -> AssetError {
    let report = BundleCompressionCodecReport::for_compression(compression);
    match (report.codec_name, context) {
        ("zstd", "builder manifest") => {
            AssetError::Unsupported("asset zstd feature is disabled for builder manifest")
        }
        ("zstd", "writer") => {
            AssetError::Unsupported("asset zstd feature is disabled for bundle writer")
        }
        ("zstd", "manifest") => {
            AssetError::Unsupported("asset zstd feature is disabled for bundle manifest")
        }
        _ => AssetError::Bundle {
            message: format!(
                "bundle compression codec `{}` is disabled for {context}",
                report.codec_name
            ),
        },
    }
}

fn parse_u128(value: &str, name: &str) -> AssetResult<u128> {
    value.parse().map_err(|error| AssetError::Bundle {
        message: format!("invalid {name}: {error}"),
    })
}

fn parse_u64(value: &str, name: &str) -> AssetResult<u64> {
    value.parse().map_err(|error| AssetError::Bundle {
        message: format!("invalid {name}: {error}"),
    })
}

fn parse_u32(value: &str, name: &str) -> AssetResult<u32> {
    value.parse().map_err(|error| AssetError::Bundle {
        message: format!("invalid {name}: {error}"),
    })
}

fn parse_usize(value: &str, name: &str) -> AssetResult<usize> {
    value.parse().map_err(|error| AssetError::Bundle {
        message: format!("invalid {name}: {error}"),
    })
}

fn filesystem_error(action: &str, path: &Path, error: std::io::Error) -> AssetError {
    AssetError::Io {
        message: format!("failed to {action} `{}`: {error}", path.display()),
    }
}
