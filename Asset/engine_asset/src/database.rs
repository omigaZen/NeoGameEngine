use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    bundle::{BundleChunkPartitionPolicy, CompressionKind},
    cooker::{AssetCooker, CookContext, CookOutput, CookerRegistry, TargetPlatform},
    dependency::{DependencyGraph, DependencyGraphReport, DependencyScopeReport},
    error::{AssetError, AssetResult},
    features::{require_asset_feature, AssetFeature},
    id::{AssetId, AssetTypeId, ContentHash, VersionHash},
    importer::{
        AssetImporter, ImportContext, ImportGeneratedAsset, ImporterRegistry, ImporterSettings,
        SourceAsset,
    },
    io::{stable_hash, AssetIo, FileSystemAssetIo},
    metadata::AssetMetadata,
    path::AssetPath,
    registry::AssetRegistry,
};

#[cfg(feature = "bundle")]
use crate::bundle::{BundleAsset, BundleBuildOptions, BundleWriter};
#[cfg(feature = "audio_cooker")]
use crate::cooker::AudioCooker;
#[cfg(feature = "material_cooker")]
use crate::cooker::MaterialCooker;
#[cfg(feature = "shader_cooker")]
use crate::cooker::ShaderCooker;
#[cfg(feature = "texture_cooker")]
use crate::cooker::TextureCooker;
#[cfg(feature = "model_cooker")]
use crate::cooker::{AnimationCooker, MeshCooker, SkeletonCooker};
#[cfg(feature = "cookers")]
use crate::cooker::{FontCooker, PhysicsMeshCooker, PrefabCooker, SceneCooker};
#[cfg(feature = "audio_importer")]
use crate::importer::AudioImporter;
#[cfg(feature = "material_importer")]
use crate::importer::MaterialImporter;
#[cfg(feature = "shader_importer")]
use crate::importer::ShaderImporter;
#[cfg(feature = "texture_importer")]
use crate::importer::TextureImporter;
#[cfg(feature = "importers")]
use crate::importer::{AnimationImporter, SkeletonImporter};
#[cfg(feature = "importers")]
use crate::importer::{FontImporter, PhysicsMeshImporter, PrefabImporter, SceneImporter};
#[cfg(feature = "model_importer")]
use crate::importer::{MeshImporter, ModelImporter};

const ASSET_REGISTRY_HEADER_V1: &str = "NGA_ASSET_REGISTRY_V1";
const ASSET_META_HEADER_V1: &str = "NGA_ASSET_META_V1";
const ASSET_REGISTRY_HEADER_V0: &str = "NGA_ASSET_REGISTRY_V0";
const ASSET_META_HEADER_V0: &str = "NGA_ASSET_META_V0";

#[derive(Clone, Debug)]
pub struct AssetDatabaseConfig {
    pub source_root: PathBuf,
    pub imported_root: PathBuf,
    pub cooked_root: PathBuf,
    pub registry_path: PathBuf,
}

impl Default for AssetDatabaseConfig {
    fn default() -> Self {
        Self {
            source_root: "assets/source".into(),
            imported_root: "assets/imported".into(),
            cooked_root: "assets/cooked".into(),
            registry_path: "assets/asset_registry.txt".into(),
        }
    }
}

pub struct AssetDatabase {
    config: AssetDatabaseConfig,
    io: Box<dyn AssetIo>,
    registry: AssetRegistry,
    importers: ImporterRegistry,
    cookers: CookerRegistry,
    diagnostics: Vec<AssetDatabaseDiagnostic>,
}

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
pub struct AssetDatabaseBundleBuild {
    pub name: String,
    pub compression: CompressionKind,
    pub chunk_policy: BundleChunkPartitionPolicy,
    pub assets: Vec<AssetId>,
}

impl AssetDatabaseBundleBuild {
    pub fn new(name: impl Into<String>, assets: Vec<AssetId>) -> Self {
        Self {
            name: name.into(),
            compression: CompressionKind::None,
            chunk_policy: BundleChunkPartitionPolicy::SingleChunk,
            assets,
        }
    }

    pub fn with_compression(mut self, compression: CompressionKind) -> Self {
        self.compression = compression;
        self
    }

    pub fn with_chunk_policy(mut self, chunk_policy: BundleChunkPartitionPolicy) -> Self {
        self.chunk_policy = chunk_policy;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetDatabaseBundleBuildOutput {
    pub bytes: Vec<u8>,
    pub asset_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetDatabaseDiagnostic {
    MissingMetadata {
        path: AssetPath,
    },
    StaleMetadata {
        id: AssetId,
        path: AssetPath,
    },
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetMetadataMigrationReport {
    pub mode: AssetMetadataMigrationMode,
    pub files: Vec<AssetMetadataMigrationFileReport>,
}

impl AssetMetadataMigrationReport {
    pub fn written_files(&self) -> usize {
        self.files.iter().filter(|file| file.written).count()
    }

    pub fn total_entries(&self) -> usize {
        self.files.iter().map(|file| file.entries.len()).sum()
    }

    pub fn upgradeable_entries(&self) -> usize {
        self.files
            .iter()
            .map(|file| file.upgradeable_entries())
            .sum()
    }

    pub fn has_blocking_errors(&self) -> bool {
        self.files.iter().any(|file| {
            matches!(
                file.status,
                AssetMetadataMigrationStatus::UnsupportedVersion
                    | AssetMetadataMigrationStatus::Invalid
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetMetadataMigrationFileReport {
    pub kind: AssetMetadataMigrationFileKind,
    pub path: PathBuf,
    pub header: Option<String>,
    pub target_header: String,
    pub status: AssetMetadataMigrationStatus,
    pub written: bool,
    pub entries: Vec<AssetMetadataMigrationEntry>,
    pub errors: Vec<String>,
}

impl AssetMetadataMigrationFileReport {
    pub fn current_entries(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == AssetMetadataMigrationStatus::Current)
            .count()
    }

    pub fn upgradeable_entries(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == AssetMetadataMigrationStatus::Upgradeable)
            .count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetMetadataMigrationFileKind {
    Registry,
    Sidecar,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AssetMetadataMigrationMode {
    #[default]
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

impl AssetDatabase {
    pub fn new(config: AssetDatabaseConfig) -> Self {
        let io: Box<dyn AssetIo> = Box::new(FileSystemAssetIo::new(config.source_root.clone()));
        Self {
            config,
            io,
            registry: AssetRegistry::new(),
            importers: ImporterRegistry::new(),
            cookers: CookerRegistry::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn config(&self) -> &AssetDatabaseConfig {
        &self.config
    }

    pub fn registry(&self) -> &AssetRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut AssetRegistry {
        &mut self.registry
    }

    pub fn set_io<I: AssetIo>(&mut self, io: I) {
        self.io = Box::new(io);
    }

    pub fn register_importer<I: AssetImporter>(&mut self, importer: I) {
        self.importers.register(importer);
    }

    pub fn register_cooker<C: AssetCooker>(&mut self, cooker: C) {
        self.cookers.register(cooker);
    }

    pub fn register_builtin_importers(&mut self) {
        #[cfg(feature = "texture_importer")]
        if crate::features::asset_feature_enabled(AssetFeature::TextureImporter) {
            self.register_importer(TextureImporter::new());
        }
        #[cfg(feature = "model_importer")]
        if crate::features::asset_feature_enabled(AssetFeature::ModelImporter) {
            self.register_importer(MeshImporter::new());
            self.register_importer(ModelImporter::new());
        }
        #[cfg(feature = "shader_importer")]
        if crate::features::asset_feature_enabled(AssetFeature::ShaderImporter) {
            self.register_importer(ShaderImporter::new());
        }
        #[cfg(feature = "material_importer")]
        if crate::features::asset_feature_enabled(AssetFeature::MaterialImporter) {
            self.register_importer(MaterialImporter::new());
        }
        #[cfg(feature = "audio_importer")]
        if crate::features::asset_feature_enabled(AssetFeature::AudioImporter) {
            self.register_importer(AudioImporter::new());
        }
        #[cfg(feature = "importers")]
        if crate::features::asset_feature_enabled(AssetFeature::Importers) {
            self.register_importer(AnimationImporter::new());
            self.register_importer(SceneImporter::new());
            self.register_importer(PrefabImporter::new());
            self.register_importer(SkeletonImporter::new());
            self.register_importer(FontImporter::new());
            self.register_importer(PhysicsMeshImporter::new());
        }
    }

    pub fn try_register_builtin_importers(&mut self) -> AssetResult<()> {
        require_asset_feature(AssetFeature::Importers)?;
        self.register_builtin_importers();
        Ok(())
    }

    pub fn register_builtin_cookers(&mut self) {
        #[cfg(feature = "texture_cooker")]
        if crate::features::asset_feature_enabled(AssetFeature::TextureCooker) {
            self.register_cooker(TextureCooker::new());
        }
        #[cfg(feature = "model_cooker")]
        if crate::features::asset_feature_enabled(AssetFeature::ModelCooker) {
            self.register_cooker(MeshCooker::new());
            self.register_cooker(SkeletonCooker::new());
            self.register_cooker(AnimationCooker::new());
        }
        #[cfg(feature = "material_cooker")]
        if crate::features::asset_feature_enabled(AssetFeature::MaterialCooker) {
            self.register_cooker(MaterialCooker::new());
        }
        #[cfg(feature = "shader_cooker")]
        if crate::features::asset_feature_enabled(AssetFeature::ShaderCooker) {
            self.register_cooker(ShaderCooker::new());
        }
        #[cfg(feature = "audio_cooker")]
        if crate::features::asset_feature_enabled(AssetFeature::AudioCooker) {
            self.register_cooker(AudioCooker::new());
        }
        #[cfg(feature = "cookers")]
        if crate::features::asset_feature_enabled(AssetFeature::Cookers) {
            self.register_cooker(SceneCooker::new());
            self.register_cooker(PrefabCooker::new());
            self.register_cooker(FontCooker::new());
            self.register_cooker(PhysicsMeshCooker::new());
        }
    }

    pub fn try_register_builtin_cookers(&mut self) -> AssetResult<()> {
        require_asset_feature(AssetFeature::Cookers)?;
        self.register_builtin_cookers();
        Ok(())
    }

    pub fn diagnostics(&self) -> &[AssetDatabaseDiagnostic] {
        &self.diagnostics
    }

    pub fn drain_diagnostics(&mut self) -> impl Iterator<Item = AssetDatabaseDiagnostic> + '_ {
        self.diagnostics.drain(..)
    }

    pub fn scan(&self) -> AssetResult<Vec<AssetPath>> {
        let mut paths = self
            .io
            .list("")
            .map_err(AssetError::from)?
            .into_iter()
            .map(|path| AssetPath::parse(&path))
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }

    pub fn scan_with_metadata(&mut self) -> AssetResult<AssetDatabaseScanReport> {
        self.load_metadata_sidecars()?;
        let sources = self.scan()?;
        let source_set = sources.iter().cloned().collect::<HashSet<_>>();
        let mut diagnostics = Vec::new();
        let mut added = Vec::new();
        let mut changed = Vec::new();
        let mut unchanged = Vec::new();
        let mut removed = Vec::new();

        for source in &sources {
            let source_hash = self.source_hash_for_path(source)?;
            if let Some(metadata) = self.registry.metadata_by_path(source) {
                match metadata.source_hash {
                    Some(previous_hash) if previous_hash == source_hash => {
                        unchanged.push(source.clone());
                    }
                    Some(previous_hash) => {
                        changed.push(source.clone());
                        diagnostics.push(AssetDatabaseDiagnostic::ChangedSource {
                            id: metadata.id,
                            path: source.clone(),
                            previous_hash,
                            current_hash: source_hash,
                        });
                    }
                    None => {
                        changed.push(source.clone());
                    }
                }
                continue;
            }
            if let Some(mut metadata) = self.metadata_with_source_hash(source_hash) {
                let old_path = metadata
                    .source_path
                    .clone()
                    .or(metadata.path.clone())
                    .unwrap_or_else(|| source.clone());
                metadata.path = Some(source.clone());
                metadata.source_path = Some(source.clone());
                self.registry.insert(metadata.clone());
                diagnostics.push(AssetDatabaseDiagnostic::MovedSourcePath {
                    id: metadata.id,
                    old_path,
                    new_path: source.clone(),
                });
                changed.push(source.clone());
            } else {
                added.push(source.clone());
                diagnostics.push(AssetDatabaseDiagnostic::MissingMetadata {
                    path: source.clone(),
                });
            }
        }

        for metadata in self.registry.values() {
            let Some(path) = metadata.source_path.as_ref().or(metadata.path.as_ref()) else {
                continue;
            };
            if !source_set.contains(path) {
                removed.push(path.clone());
                diagnostics.push(AssetDatabaseDiagnostic::StaleMetadata {
                    id: metadata.id,
                    path: path.clone(),
                });
            }
        }
        removed.sort();

        let metadata = self.registry.values().cloned().collect::<Vec<_>>();
        self.diagnostics = diagnostics.clone();
        Ok(AssetDatabaseScanReport {
            sources,
            metadata,
            diagnostics,
            added,
            changed,
            unchanged,
            removed,
        })
    }

    pub fn import_asset_path(&mut self, path: &AssetPath) -> AssetResult<AssetId> {
        self.import_asset_path_with_settings(path, &ImporterSettings::default())
    }

    pub fn import_asset_path_with_settings(
        &mut self,
        path: &AssetPath,
        settings: &ImporterSettings,
    ) -> AssetResult<AssetId> {
        let bytes = self.io.read(path.path()).map_err(AssetError::from)?;
        let io_metadata = self.io.metadata(path.path()).map_err(AssetError::from)?;
        let base_source_hash = io_metadata.hash.unwrap_or(ContentHash(stable_hash(&bytes)));
        let source_hash = self.source_hash_with_import_context(path, &bytes, base_source_hash);
        let source = SourceAsset {
            path: path.clone(),
            bytes,
            hash: source_hash,
        };
        let extension = path.extension().unwrap_or("");
        let importer = self
            .importers
            .importer_for_extension(extension)
            .ok_or_else(|| AssetError::Import {
                message: format!("no importer registered for extension `{extension}`"),
            })?;
        let importer_name = importer.name().to_owned();
        let importer_version = importer.version();
        let mut ctx = ImportContext::with_registry(&self.registry);
        populate_import_source_files(self.io.as_ref(), &mut ctx, &source, extension);
        let mut output = importer
            .import(&mut ctx, &source, settings)
            .map_err(|error| AssetError::Import {
                message: format!(
                    "importer `{importer_name}` failed for `{}` with settings {}: {error}",
                    path.display_string(),
                    settings.describe()
                ),
            })?;
        let stable_id = self.stable_id_for_import(path, source_hash, output.metadata.id);
        output.metadata.id = stable_id;
        output.metadata.path = Some(path.clone());
        output.metadata.source_path = Some(path.clone());
        output.metadata.source_hash = Some(source_hash);
        output.metadata.importer = Some(importer_name);
        output.metadata.importer_version = importer_version;
        output.metadata.version_hash = Some(output.version_hash);
        output.metadata.importer_settings = settings.to_sorted_pairs();
        for dependency in &output.dependencies {
            if !output.metadata.dependencies.contains(dependency) {
                output.metadata.dependencies.push(*dependency);
            }
        }
        let mut generated_id_remaps = Vec::new();
        for generated in &mut output.generated {
            let original_id = generated.id;
            if generated.path == *path {
                generated.id = stable_id;
            } else if let Some(existing_id) = self.registry.id_from_path(&generated.path) {
                generated.id = existing_id;
            }
            if generated.id != original_id {
                generated_id_remaps.push((original_id, generated.id));
            }
        }
        if !generated_id_remaps.is_empty() {
            remap_metadata_dependencies(&mut output.metadata.dependencies, &generated_id_remaps);
            for generated in &mut output.generated {
                remap_metadata_dependencies(&mut generated.dependencies, &generated_id_remaps);
            }
        }
        for generated in &output.generated {
            if generated.id != stable_id && !output.metadata.dependencies.contains(&generated.id) {
                output.metadata.dependencies.push(generated.id);
            }
        }
        self.registry.insert(output.metadata.clone());
        for generated in output.generated {
            if generated.id != stable_id {
                let mut metadata = AssetMetadata::runtime(
                    generated.id,
                    generated.path.clone(),
                    generated.asset_type,
                );
                metadata.source_path = Some(path.clone());
                metadata.cooked_path = Some(generated.path.clone());
                metadata.labels = generated.labels.clone();
                metadata.dependencies = generated.dependencies.clone();
                metadata.importer = output.metadata.importer.clone();
                metadata.importer_version = output.metadata.importer_version;
                metadata.source_hash = Some(source_hash);
                metadata.settings_hash = output.metadata.settings_hash;
                metadata.version_hash = Some(output.version_hash);
                metadata.importer_settings = output.metadata.importer_settings.clone();
                self.save_generated_asset_bytes(&generated)?;
                self.registry.insert(metadata);
            } else {
                self.save_generated_asset_bytes(&generated)?;
            }
        }
        self.save_metadata_sidecar(stable_id)?;
        Ok(stable_id)
    }

    pub fn cook_asset(&mut self, id: AssetId, target: TargetPlatform) -> AssetResult<CookOutput> {
        let metadata = self
            .registry
            .get(id)
            .cloned()
            .ok_or(AssetError::AssetNotFound { id })?;
        let cooker = self
            .cookers
            .cooker_for_type(metadata.asset_type)
            .ok_or_else(|| AssetError::Cook {
                message: format!(
                    "no cooker registered for asset type {:?} for asset {:?} path {}",
                    metadata.asset_type,
                    id,
                    metadata
                        .path
                        .as_ref()
                        .map(AssetPath::display_string)
                        .unwrap_or_else(|| "<unknown>".to_owned())
                ),
            })?;
        let cooker_name = cooker.name().to_owned();
        let source_path = metadata
            .source_path
            .as_ref()
            .or(metadata.path.as_ref())
            .cloned()
            .ok_or_else(|| AssetError::Cook {
                message: format!("asset metadata has no source path: {id:?}"),
            })?;
        let source_bytes = self.source_bytes_for_cook(&metadata, &source_path)?;
        let mut output = cooker
            .cook(
                &CookContext {
                    target,
                    source_path: Some(source_path.clone()),
                    source_bytes,
                },
                &metadata,
            )
            .map_err(|error| AssetError::Cook {
                message: format!(
                    "cooker `{cooker_name}` failed for asset {:?} path {} target {:?}: {error}",
                    id,
                    source_path.display_string(),
                    target
                ),
            })?;
        let cooked_path = metadata
            .cooked_path
            .clone()
            .unwrap_or_else(|| source_path.clone());
        let cooked_file = self.cooked_file_path(&cooked_path);
        if let Some(parent) = cooked_file.parent() {
            fs::create_dir_all(parent).map_err(|error| AssetError::Io {
                message: error.to_string(),
            })?;
        }
        fs::write(&cooked_file, &output.bytes).map_err(|error| AssetError::Io {
            message: error.to_string(),
        })?;
        output.metadata.cooked_path = Some(cooked_path.clone());
        output.metadata.cooked_hash = Some(output.content_hash);
        output.metadata.version_hash = Some(output.version_hash);
        if let Some(metadata) = self.registry.get_mut(id) {
            metadata.cooked_path = Some(cooked_path);
            metadata.cooked_hash = Some(output.content_hash);
            metadata.version_hash = Some(output.version_hash);
        }
        Ok(output)
    }

    #[cfg(feature = "bundle")]
    pub fn build_bundle(
        &self,
        build: &AssetDatabaseBundleBuild,
    ) -> AssetResult<AssetDatabaseBundleBuildOutput> {
        require_asset_feature(AssetFeature::Bundle)?;
        let assets = build
            .assets
            .iter()
            .map(|id| self.bundle_asset_for_id(*id))
            .collect::<AssetResult<Vec<_>>>()?;
        let asset_count = assets.len();
        let bytes = BundleWriter::build_bytes_with_options(
            build.name.clone(),
            BundleBuildOptions::new(build.compression).with_chunk_policy(build.chunk_policy),
            assets,
        )?;
        Ok(AssetDatabaseBundleBuildOutput { bytes, asset_count })
    }

    #[cfg(not(feature = "bundle"))]
    pub fn build_bundle(
        &self,
        build: &AssetDatabaseBundleBuild,
    ) -> AssetResult<AssetDatabaseBundleBuildOutput> {
        let _ = build;
        require_asset_feature(AssetFeature::Bundle)?;
        unreachable!("bundle feature is disabled")
    }

    pub fn build_bundle_bytes(&self, build: &AssetDatabaseBundleBuild) -> AssetResult<Vec<u8>> {
        Ok(self.build_bundle(build)?.bytes)
    }

    pub fn save_registry(&self) -> AssetResult<()> {
        self.save_registry_to_path(&self.config.registry_path)
    }

    pub fn save_registry_to_path(&self, path: &Path) -> AssetResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| AssetError::Io {
                message: error.to_string(),
            })?;
        }
        fs::write(path, self.registry_to_string()).map_err(|error| AssetError::Io {
            message: error.to_string(),
        })
    }

    pub fn save_metadata_sidecar(&self, id: AssetId) -> AssetResult<PathBuf> {
        let metadata = self
            .registry
            .get(id)
            .ok_or(AssetError::AssetNotFound { id })?;
        let path = self
            .metadata_sidecar_path(metadata)
            .ok_or_else(|| AssetError::Io {
                message: format!("asset metadata has no path for sidecar: {id:?}"),
            })?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| AssetError::Io {
                message: error.to_string(),
            })?;
        }
        fs::write(&path, metadata_to_sidecar_string(metadata)).map_err(|error| AssetError::Io {
            message: error.to_string(),
        })?;
        Ok(path)
    }

    pub fn save_all_metadata_sidecars(&self) -> AssetResult<Vec<PathBuf>> {
        self.registry
            .values()
            .map(|metadata| self.save_metadata_sidecar(metadata.id))
            .collect()
    }

    pub fn load_metadata_sidecars(&mut self) -> AssetResult<Vec<AssetMetadata>> {
        let mut paths = Vec::new();
        collect_sidecar_paths(&self.config.imported_root, &mut paths)?;
        let mut metadata_entries = Vec::new();
        for path in paths {
            let text = fs::read_to_string(&path).map_err(|error| AssetError::Io {
                message: error.to_string(),
            })?;
            let metadata = metadata_from_sidecar_str(&text).map_err(|message| AssetError::Io {
                message: format!("{}: {message}", path.display()),
            })?;
            self.registry.insert(metadata.clone());
            metadata_entries.push(metadata);
        }
        Ok(metadata_entries)
    }

    pub fn load_registry(&mut self) -> AssetResult<()> {
        let text =
            fs::read_to_string(&self.config.registry_path).map_err(|error| AssetError::Io {
                message: error.to_string(),
            })?;
        self.load_registry_from_str(&text)
    }

    pub fn registry_to_string(&self) -> String {
        let mut lines = vec![ASSET_REGISTRY_HEADER_V1.to_owned()];
        let mut entries = self.registry.values().collect::<Vec<_>>();
        entries.sort_by_key(|metadata| metadata.id);
        for metadata in entries {
            lines.push(serialize_metadata(metadata));
        }
        lines.join("\n")
    }

    pub fn load_registry_from_str(&mut self, text: &str) -> AssetResult<()> {
        let mut lines = text.lines();
        parse_metadata_header(
            lines.next(),
            "asset registry",
            ASSET_REGISTRY_HEADER_V1,
            "NGA_ASSET_REGISTRY_V",
        )
        .map_err(|message| AssetError::Io { message })?;
        self.registry.clear();
        for (line_index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let metadata = deserialize_metadata(line).map_err(|message| AssetError::Io {
                message: format!("registry line {}: {message}", line_index + 2),
            })?;
            self.registry.insert(metadata);
        }
        Ok(())
    }

    pub fn metadata_migration_report(&self) -> AssetResult<AssetMetadataMigrationReport> {
        self.migrate_metadata(AssetMetadataMigrationMode::DryRun)
    }

    pub fn migrate_metadata(
        &self,
        mode: AssetMetadataMigrationMode,
    ) -> AssetResult<AssetMetadataMigrationReport> {
        let mut files = Vec::new();
        if self.config.registry_path.exists() {
            let text =
                fs::read_to_string(&self.config.registry_path).map_err(|error| AssetError::Io {
                    message: format!(
                        "failed to read asset registry `{}` for migration report: {error}",
                        self.config.registry_path.display()
                    ),
                })?;
            files.push(metadata_migration_file_report(
                AssetMetadataMigrationFileKind::Registry,
                self.config.registry_path.clone(),
                &text,
                ASSET_REGISTRY_HEADER_V1,
                "NGA_ASSET_REGISTRY_V",
            ));
        }

        let mut sidecars = Vec::new();
        collect_sidecar_paths(&self.config.imported_root, &mut sidecars)?;
        sidecars.sort();
        for path in sidecars {
            let text = fs::read_to_string(&path).map_err(|error| AssetError::Io {
                message: format!(
                    "failed to read metadata sidecar `{}` for migration report: {error}",
                    path.display()
                ),
            })?;
            files.push(metadata_migration_file_report(
                AssetMetadataMigrationFileKind::Sidecar,
                path,
                &text,
                ASSET_META_HEADER_V1,
                "NGA_ASSET_META_V",
            ));
        }

        if mode == AssetMetadataMigrationMode::Write {
            for file in &mut files {
                if file.status != AssetMetadataMigrationStatus::Upgradeable {
                    continue;
                }
                let text = fs::read_to_string(&file.path).map_err(|error| AssetError::Io {
                    message: format!(
                        "failed to read metadata file `{}` for migration write-back: {error}",
                        file.path.display()
                    ),
                })?;
                let migrated = match file.kind {
                    AssetMetadataMigrationFileKind::Registry => migrate_metadata_text(
                        &text,
                        ASSET_REGISTRY_HEADER_V1,
                        "NGA_ASSET_REGISTRY_V",
                    ),
                    AssetMetadataMigrationFileKind::Sidecar => {
                        migrate_metadata_text(&text, ASSET_META_HEADER_V1, "NGA_ASSET_META_V")
                    }
                }
                .map_err(|message| AssetError::Io {
                    message: format!(
                        "failed to migrate metadata file `{}`: {message}",
                        file.path.display()
                    ),
                })?;
                fs::write(&file.path, migrated).map_err(|error| AssetError::Io {
                    message: format!(
                        "failed to write migrated metadata file `{}`: {error}",
                        file.path.display()
                    ),
                })?;
                file.written = true;
            }
        }

        Ok(AssetMetadataMigrationReport { mode, files })
    }

    pub fn dependency_report(&self) -> DependencyGraphReport {
        self.dependency_graph().report()
    }

    pub fn scoped_dependency_report(&self, root: AssetId) -> AssetResult<DependencyScopeReport> {
        self.dependency_graph().scoped_report(root)
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
        fs::write(path, report.to_html_with_labels(labels)).map_err(|error| AssetError::Io {
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
        fs::write(path, report.to_html_with_labels(labels)).map_err(|error| AssetError::Io {
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
                self.registry()
                    .get(*asset)
                    .map(|metadata| (*asset, dependency_report_metadata_label(metadata)))
            })
            .collect()
    }

    fn stable_id_for_import(
        &mut self,
        path: &AssetPath,
        source_hash: ContentHash,
        fallback: AssetId,
    ) -> AssetId {
        if let Some(id) = self.registry.id_from_path(path) {
            return id;
        }
        if let Some(metadata) = self.metadata_with_source_hash(source_hash) {
            let old_path = metadata
                .source_path
                .clone()
                .or(metadata.path.clone())
                .unwrap_or_else(|| path.clone());
            if old_path != *path {
                self.diagnostics
                    .push(AssetDatabaseDiagnostic::MovedSourcePath {
                        id: metadata.id,
                        old_path,
                        new_path: path.clone(),
                    });
            }
            metadata.id
        } else {
            fallback
        }
    }

    fn dependency_graph(&self) -> DependencyGraph {
        let mut graph = DependencyGraph::new();
        for metadata in self.registry.values() {
            graph.set_dependencies(metadata.id, metadata.dependencies.clone());
        }
        graph
    }

    fn metadata_with_source_hash(&self, source_hash: ContentHash) -> Option<AssetMetadata> {
        self.registry
            .values()
            .find(|metadata| metadata.source_hash == Some(source_hash))
            .cloned()
    }

    fn source_hash_for_path(&self, path: &AssetPath) -> AssetResult<ContentHash> {
        let bytes = self.io.read(path.path()).map_err(AssetError::from)?;
        let base_source_hash = self
            .io
            .metadata(path.path())
            .map_err(AssetError::from)?
            .hash
            .unwrap_or(ContentHash(stable_hash(&bytes)));
        Ok(self.source_hash_with_import_context(path, &bytes, base_source_hash))
    }

    fn source_hash_with_import_context(
        &self,
        path: &AssetPath,
        bytes: &[u8],
        base_source_hash: ContentHash,
    ) -> ContentHash {
        if !source_uses_model_context_files(path, bytes) {
            return base_source_hash;
        }

        let mut context_hashes = model_context_source_hashes(self.io.as_ref(), path);
        if context_hashes.is_empty() {
            return base_source_hash;
        }
        context_hashes.sort_by(|left, right| left.0.cmp(&right.0));

        let mut manifest = format!("NGA_IMPORT_CONTEXT_HASH_V1\nroot|{}\n", base_source_hash.0);
        for (context_path, hash) in context_hashes {
            manifest.push_str(&context_path.display_string());
            manifest.push('|');
            manifest.push_str(&hash.0.to_string());
            manifest.push('\n');
        }
        ContentHash(stable_hash(manifest.as_bytes()))
    }

    fn metadata_sidecar_path(&self, metadata: &AssetMetadata) -> Option<PathBuf> {
        metadata
            .path
            .as_ref()
            .or(metadata.source_path.as_ref())
            .map(|path| self.metadata_sidecar_path_for_asset_path(path))
    }

    fn metadata_sidecar_path_for_asset_path(&self, path: &AssetPath) -> PathBuf {
        let mut relative = path.without_label().path().replace('\\', "/");
        relative.push_str(".meta");
        self.config.imported_root.join(relative)
    }

    fn cooked_file_path(&self, path: &AssetPath) -> PathBuf {
        self.config
            .cooked_root
            .join(path.without_label().path().replace('\\', "/"))
    }

    fn imported_asset_file_path(&self, path: &AssetPath) -> PathBuf {
        self.config
            .imported_root
            .join(path.without_label().path().replace('\\', "/"))
    }

    fn save_generated_asset_bytes(&self, generated: &ImportGeneratedAsset) -> AssetResult<PathBuf> {
        let path = self.imported_asset_file_path(&generated.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| AssetError::Io {
                message: error.to_string(),
            })?;
        }
        fs::write(&path, &generated.bytes).map_err(|error| AssetError::Io {
            message: error.to_string(),
        })?;
        Ok(path)
    }

    fn source_bytes_for_cook(
        &self,
        metadata: &AssetMetadata,
        source_path: &AssetPath,
    ) -> AssetResult<Vec<u8>> {
        if let (Some(path), Some(cooked_path)) =
            (metadata.path.as_ref(), metadata.cooked_path.as_ref())
        {
            if path == cooked_path {
                let imported_file = self.imported_asset_file_path(path);
                if imported_file.exists() {
                    return fs::read(&imported_file).map_err(|error| AssetError::Io {
                        message: error.to_string(),
                    });
                }
            }
        }
        self.io.read(source_path.path()).map_err(AssetError::from)
    }

    #[cfg(feature = "bundle")]
    fn bundle_asset_for_id(&self, id: AssetId) -> AssetResult<BundleAsset> {
        let metadata = self
            .registry
            .get(id)
            .cloned()
            .ok_or(AssetError::AssetNotFound { id })?;
        let cooked_path = metadata
            .cooked_path
            .as_ref()
            .ok_or_else(|| AssetError::Bundle {
                message: format!("asset has no cooked path for bundle: {id:?}"),
            })?;
        let bundle_path = metadata
            .path
            .as_ref()
            .or(metadata.cooked_path.as_ref())
            .cloned()
            .ok_or_else(|| AssetError::Bundle {
                message: format!("asset has no runtime path for bundle: {id:?}"),
            })?;
        let cooked_file = self.cooked_file_path(cooked_path);
        let bytes = fs::read(&cooked_file).map_err(|error| AssetError::Io {
            message: format!(
                "failed to read cooked asset {} for bundle: {error}",
                cooked_file.display()
            ),
        })?;
        if let Some(expected) = metadata.cooked_hash {
            let actual = ContentHash(stable_hash(&bytes));
            if actual != expected {
                return Err(AssetError::Bundle {
                    message: format!(
                        "cooked hash mismatch for bundle asset {}",
                        bundle_path.display_string()
                    ),
                });
            }
        }
        Ok(BundleAsset {
            id: metadata.id,
            asset_type: metadata.asset_type,
            path: bundle_path,
            bytes,
            dependencies: metadata.dependencies,
        })
    }
}

fn populate_import_source_files(
    io: &dyn AssetIo,
    ctx: &mut ImportContext,
    source: &SourceAsset,
    extension: &str,
) {
    ctx.add_source_file(source.clone());
    if !extension.eq_ignore_ascii_case("obj") && !extension.eq_ignore_ascii_case("model") {
        return;
    }

    let directory = source_asset_directory(source.path.path());
    let Ok(entries) = io.list(directory) else {
        return;
    };
    for entry in entries {
        let path = AssetPath::parse(&entry).without_label();
        if path == source.path.without_label() || !is_model_context_source_path(&path) {
            continue;
        }
        let Ok(bytes) = io.read(path.path()) else {
            continue;
        };
        let hash = io
            .metadata(path.path())
            .ok()
            .and_then(|metadata| metadata.hash)
            .unwrap_or(ContentHash(stable_hash(&bytes)));
        ctx.add_source_file(SourceAsset { path, bytes, hash });
    }
}

fn source_uses_model_context_files(path: &AssetPath, bytes: &[u8]) -> bool {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("obj"))
    {
        return true;
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|source| source.lines().next())
        .is_some_and(|line| line.trim() == "NGA_MODEL_OBJ_V1")
}

fn model_context_source_hashes(
    io: &dyn AssetIo,
    source_path: &AssetPath,
) -> Vec<(AssetPath, ContentHash)> {
    let directory = source_asset_directory(source_path.path());
    let Ok(entries) = io.list(directory) else {
        return Vec::new();
    };
    let mut hashes = Vec::new();
    for entry in entries {
        let path = AssetPath::parse(&entry).without_label();
        if path == source_path.without_label() || !is_model_context_source_path(&path) {
            continue;
        }
        let Ok(bytes) = io.read(path.path()) else {
            continue;
        };
        let hash = io
            .metadata(path.path())
            .ok()
            .and_then(|metadata| metadata.hash)
            .unwrap_or(ContentHash(stable_hash(&bytes)));
        hashes.push((path, hash));
    }
    hashes
}

fn is_model_context_source_path(path: &AssetPath) -> bool {
    path.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("mtl") || extension.eq_ignore_ascii_case("obj")
    })
}

fn source_asset_directory(path: &str) -> &str {
    path.rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("")
}

fn serialize_metadata(metadata: &AssetMetadata) -> String {
    [
        metadata.id.raw().to_string(),
        metadata.asset_type.raw().to_string(),
        metadata
            .path
            .as_ref()
            .map(AssetPath::display_string)
            .unwrap_or_default(),
        metadata
            .source_path
            .as_ref()
            .map(AssetPath::display_string)
            .unwrap_or_default(),
        metadata
            .cooked_path
            .as_ref()
            .map(AssetPath::display_string)
            .unwrap_or_default(),
        metadata.importer.clone().unwrap_or_default(),
        metadata.importer_version.to_string(),
        option_hash(metadata.source_hash),
        option_hash(metadata.settings_hash),
        option_hash(metadata.cooked_hash),
        metadata
            .version_hash
            .map(|hash| hash.0.to_string())
            .unwrap_or_default(),
        metadata
            .dependencies
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join(","),
        metadata.labels.join(","),
        settings_to_string(&metadata.importer_settings),
    ]
    .join("|")
}

fn remap_metadata_dependencies(dependencies: &mut [AssetId], remaps: &[(AssetId, AssetId)]) {
    for dependency in dependencies {
        if let Some((_, new_id)) = remaps.iter().find(|(old_id, _)| old_id == dependency) {
            *dependency = *new_id;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetMetadataTextFormat {
    V1,
    V0,
}

fn deserialize_metadata(line: &str) -> Result<AssetMetadata, String> {
    let fields = line.split('|').collect::<Vec<_>>();
    if fields.len() != 12 && fields.len() != 13 && fields.len() != 14 {
        return Err(format!(
            "expected 12, 13, or 14 fields, got {}",
            fields.len()
        ));
    }
    let id = AssetId::from_u128(parse_u128(fields[0], "id")?);
    let asset_type = AssetTypeId::from_u128(parse_u128(fields[1], "asset_type")?);
    Ok(AssetMetadata {
        id,
        asset_type,
        path: parse_path(fields[2]),
        source_path: parse_path(fields[3]),
        cooked_path: parse_path(fields[4]),
        importer: (!fields[5].is_empty()).then(|| fields[5].to_owned()),
        importer_version: fields[6]
            .parse()
            .map_err(|error| format!("invalid importer version: {error}"))?,
        source_hash: parse_hash(fields[7])?,
        settings_hash: parse_hash(fields[8])?,
        cooked_hash: parse_hash(fields[9])?,
        version_hash: if fields[10].is_empty() {
            None
        } else {
            Some(VersionHash(
                fields[10]
                    .parse()
                    .map_err(|error| format!("invalid version hash: {error}"))?,
            ))
        },
        dependencies: parse_dependency_list(fields[11])?,
        labels: if fields.get(12).copied().unwrap_or("").is_empty() {
            Vec::new()
        } else {
            fields[12].split(',').map(str::to_owned).collect()
        },
        importer_settings: parse_settings(fields.get(13).copied().unwrap_or(""))?,
    })
}

fn deserialize_legacy_v0_metadata(line: &str) -> Result<AssetMetadata, String> {
    let fields = line.split('|').collect::<Vec<_>>();
    if fields.len() != 11 {
        return Err(format!(
            "expected 11 legacy V0 fields, got {}",
            fields.len()
        ));
    }
    let id = AssetId::from_u128(parse_u128(fields[0], "id")?);
    let asset_type = AssetTypeId::from_u128(parse_u128(fields[1], "asset_type")?);
    Ok(AssetMetadata {
        id,
        asset_type,
        path: parse_path(fields[2]),
        source_path: parse_path(fields[3]),
        cooked_path: parse_path(fields[4]),
        importer: (!fields[5].is_empty()).then(|| fields[5].to_owned()),
        importer_version: fields[6]
            .parse()
            .map_err(|error| format!("invalid importer version: {error}"))?,
        source_hash: parse_hash(fields[7])?,
        settings_hash: parse_hash(fields[8])?,
        cooked_hash: parse_hash(fields[9])?,
        version_hash: None,
        dependencies: parse_dependency_list(fields[10])?,
        labels: Vec::new(),
        importer_settings: Vec::new(),
    })
}

fn deserialize_metadata_for_format(
    line: &str,
    format: AssetMetadataTextFormat,
) -> Result<AssetMetadata, String> {
    match format {
        AssetMetadataTextFormat::V1 => deserialize_metadata(line),
        AssetMetadataTextFormat::V0 => deserialize_legacy_v0_metadata(line),
    }
}

fn metadata_to_sidecar_string(metadata: &AssetMetadata) -> String {
    format!("{ASSET_META_HEADER_V1}\n{}", serialize_metadata(metadata))
}

fn metadata_from_sidecar_str(text: &str) -> Result<AssetMetadata, String> {
    let mut lines = text.lines();
    parse_metadata_header(
        lines.next(),
        "asset metadata sidecar",
        ASSET_META_HEADER_V1,
        "NGA_ASSET_META_V",
    )?;
    let line = lines
        .next()
        .ok_or_else(|| "missing asset metadata payload".to_owned())?;
    deserialize_metadata(line)
}

fn metadata_migration_file_report(
    kind: AssetMetadataMigrationFileKind,
    path: PathBuf,
    text: &str,
    expected_header: &str,
    versioned_prefix: &str,
) -> AssetMetadataMigrationFileReport {
    let mut lines = text.lines();
    let header = lines.next().map(str::to_owned);
    let target_header = expected_header.to_owned();
    let mut format = AssetMetadataTextFormat::V1;
    let mut header_upgradeable = false;
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut status = match header.as_deref() {
        Some(actual) if actual == expected_header => AssetMetadataMigrationStatus::Current,
        Some(actual) if legacy_metadata_header(expected_header) == Some(actual) => {
            format = AssetMetadataTextFormat::V0;
            header_upgradeable = true;
            AssetMetadataMigrationStatus::Upgradeable
        }
        Some(actual) if actual.starts_with(versioned_prefix) => {
            errors.push(format!(
                "unsupported metadata version `{actual}`; target `{expected_header}`"
            ));
            AssetMetadataMigrationStatus::UnsupportedVersion
        }
        Some(_) => {
            errors.push("invalid metadata header".to_owned());
            AssetMetadataMigrationStatus::Invalid
        }
        None => {
            errors.push("missing metadata header".to_owned());
            AssetMetadataMigrationStatus::Invalid
        }
    };

    if matches!(
        status,
        AssetMetadataMigrationStatus::Current | AssetMetadataMigrationStatus::Upgradeable
    ) {
        for (line_offset, line) in lines.enumerate() {
            let line_number = line_offset + 2;
            let fields = line.split('|').collect::<Vec<_>>();
            let field_count = fields.len();
            let mut payload_status = metadata_payload_status(format, field_count);
            let (id, message) = match deserialize_metadata_for_format(line, format) {
                Ok(metadata) => (Some(metadata.id), None),
                Err(message) => {
                    payload_status = AssetMetadataMigrationStatus::Invalid;
                    errors.push(format!("line {line_number}: {message}"));
                    (None, Some(message))
                }
            };
            entries.push(AssetMetadataMigrationEntry {
                line: line_number,
                id,
                field_count,
                status: payload_status,
                message,
            });
        }

        status = if entries
            .iter()
            .any(|entry| entry.status == AssetMetadataMigrationStatus::Invalid)
            || !errors.is_empty()
        {
            AssetMetadataMigrationStatus::Invalid
        } else if entries
            .iter()
            .any(|entry| entry.status == AssetMetadataMigrationStatus::Upgradeable)
            || header_upgradeable
        {
            AssetMetadataMigrationStatus::Upgradeable
        } else {
            AssetMetadataMigrationStatus::Current
        };
    }

    AssetMetadataMigrationFileReport {
        kind,
        path,
        header,
        target_header,
        status,
        written: false,
        entries,
        errors,
    }
}

fn migrate_metadata_text(
    text: &str,
    expected_header: &str,
    versioned_prefix: &str,
) -> Result<String, String> {
    let mut lines = text.lines();
    let format = parse_metadata_migration_header(
        lines.next(),
        "metadata migration input",
        expected_header,
        versioned_prefix,
    )?;
    let mut migrated = vec![expected_header.to_owned()];
    for (line_offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let metadata = deserialize_metadata_for_format(line, format)
            .map_err(|message| format!("line {}: {message}", line_offset + 2))?;
        migrated.push(serialize_metadata(&metadata));
    }
    Ok(migrated.join("\n"))
}

fn parse_metadata_header(
    header: Option<&str>,
    kind: &str,
    expected: &str,
    versioned_prefix: &str,
) -> Result<(), String> {
    match header {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) if actual.starts_with(versioned_prefix) => Err(format!(
            "unsupported {kind} version `{actual}`; expected `{expected}`; run metadata migration report"
        )),
        Some(_) => Err(format!("invalid {kind} header")),
        None => Err(format!("missing {kind} header")),
    }
}

fn parse_metadata_migration_header(
    header: Option<&str>,
    kind: &str,
    expected: &str,
    versioned_prefix: &str,
) -> Result<AssetMetadataTextFormat, String> {
    match header {
        Some(actual) if actual == expected => Ok(AssetMetadataTextFormat::V1),
        Some(actual) if legacy_metadata_header(expected) == Some(actual) => {
            Ok(AssetMetadataTextFormat::V0)
        }
        Some(actual) if actual.starts_with(versioned_prefix) => Err(format!(
            "unsupported {kind} version `{actual}`; expected `{expected}`; run metadata migration report"
        )),
        Some(_) => Err(format!("invalid {kind} header")),
        None => Err(format!("missing {kind} header")),
    }
}

fn legacy_metadata_header(expected: &str) -> Option<&'static str> {
    match expected {
        ASSET_REGISTRY_HEADER_V1 => Some(ASSET_REGISTRY_HEADER_V0),
        ASSET_META_HEADER_V1 => Some(ASSET_META_HEADER_V0),
        _ => None,
    }
}

fn metadata_payload_status(
    format: AssetMetadataTextFormat,
    field_count: usize,
) -> AssetMetadataMigrationStatus {
    match format {
        AssetMetadataTextFormat::V1 => match field_count {
            14 => AssetMetadataMigrationStatus::Current,
            12 | 13 => AssetMetadataMigrationStatus::Upgradeable,
            _ => AssetMetadataMigrationStatus::Invalid,
        },
        AssetMetadataTextFormat::V0 => match field_count {
            11 => AssetMetadataMigrationStatus::Upgradeable,
            _ => AssetMetadataMigrationStatus::Invalid,
        },
    }
}

fn collect_sidecar_paths(root: &Path, paths: &mut Vec<PathBuf>) -> AssetResult<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| AssetError::Io {
        message: error.to_string(),
    })? {
        let entry = entry.map_err(|error| AssetError::Io {
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_sidecar_paths(&path, paths)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("meta") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(())
}

fn option_hash(hash: Option<ContentHash>) -> String {
    hash.map(|hash| hash.0.to_string()).unwrap_or_default()
}

fn dependency_report_metadata_label(metadata: &AssetMetadata) -> String {
    let path = metadata
        .path
        .as_ref()
        .map(AssetPath::display_string)
        .unwrap_or_else(|| "unmapped".to_owned());
    format!("{path} | type {}", metadata.asset_type.raw())
}

fn settings_to_string(settings: &[(String, String)]) -> String {
    let mut settings = settings.to_vec();
    settings.sort_by(|left, right| left.0.cmp(&right.0));
    settings
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn parse_settings(value: &str) -> Result<Vec<(String, String)>, String> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let mut settings = value
        .split(';')
        .map(|pair| {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("invalid importer setting `{pair}`"))?;
            Ok((key.to_owned(), value.to_owned()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    settings.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(settings)
}

fn parse_hash(value: &str) -> Result<Option<ContentHash>, String> {
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse()
            .map(|value| Some(ContentHash(value)))
            .map_err(|error| format!("invalid hash: {error}"))
    }
}

fn parse_dependency_list(value: &str) -> Result<Vec<AssetId>, String> {
    if value.is_empty() {
        Ok(Vec::new())
    } else {
        value
            .split(',')
            .map(|value| parse_u128(value, "dependency").map(AssetId::from_u128))
            .collect::<Result<Vec<_>, _>>()
    }
}

fn parse_path(value: &str) -> Option<AssetPath> {
    (!value.is_empty()).then(|| AssetPath::parse(value))
}

fn parse_u128(value: &str, name: &str) -> Result<u128, String> {
    value
        .parse()
        .map_err(|error| format!("invalid {name}: {error}"))
}
