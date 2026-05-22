use std::collections::HashMap;

use crate::{
    asset::Asset,
    error::{AssetError, ImportError},
    id::{AssetId, AssetTypeId, ContentHash, VersionHash},
    metadata::AssetMetadata,
    path::AssetPath,
    registry::AssetRegistry,
};

#[cfg(feature = "importers")]
use crate::assets::Font;
#[cfg(any(feature = "material_importer", feature = "model_importer"))]
use crate::assets::Material;
#[cfg(any(
    feature = "shader_importer",
    feature = "material_importer",
    feature = "model_importer"
))]
use crate::assets::Shader;
#[cfg(any(
    feature = "texture_importer",
    feature = "material_importer",
    feature = "model_importer"
))]
use crate::assets::Texture;
#[cfg(feature = "model_importer")]
use crate::assets::{AnimationClip, AnimationTarget, Mesh, PhysicsMesh, Skeleton};
#[cfg(feature = "audio_importer")]
use crate::assets::{AudioClip, AudioCompression};

#[cfg(feature = "model_importer")]
const SKIN_WEIGHT_SUM_EPSILON: f32 = 0.001;

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModelImportSettings {
    pub import_meshes: bool,
    pub import_physics_meshes: bool,
    pub import_materials: bool,
    pub import_animations: bool,
    pub import_skeleton: bool,
    pub optimize_meshes: bool,
    pub generate_tangents: bool,
    pub generate_lods: bool,
    pub scale: f32,
}

#[cfg(feature = "model_importer")]
impl Default for ModelImportSettings {
    fn default() -> Self {
        Self {
            import_meshes: true,
            import_physics_meshes: true,
            import_materials: true,
            import_animations: true,
            import_skeleton: true,
            optimize_meshes: false,
            generate_tangents: true,
            generate_lods: false,
            scale: 1.0,
        }
    }
}

#[cfg(feature = "model_importer")]
impl ModelImportSettings {
    pub fn from_importer_settings(settings: &ImporterSettings) -> Result<Self, ImportError> {
        let mut model_settings = Self::default();
        model_settings.import_meshes = parse_optional_model_import_bool(
            settings,
            "import_meshes",
            model_settings.import_meshes,
        )?;
        model_settings.import_materials = parse_optional_model_import_bool(
            settings,
            "import_materials",
            model_settings.import_materials,
        )?;
        model_settings.import_physics_meshes = parse_optional_model_import_bool(
            settings,
            "import_physics_meshes",
            model_settings.import_physics_meshes,
        )?;
        model_settings.import_animations = parse_optional_model_import_bool(
            settings,
            "import_animations",
            model_settings.import_animations,
        )?;
        model_settings.import_skeleton = parse_optional_model_import_bool(
            settings,
            "import_skeleton",
            model_settings.import_skeleton,
        )?;
        model_settings.optimize_meshes = parse_optional_model_import_bool(
            settings,
            "optimize_meshes",
            model_settings.optimize_meshes,
        )?;
        model_settings.generate_tangents = parse_optional_model_import_bool(
            settings,
            "generate_tangents",
            model_settings.generate_tangents,
        )?;
        model_settings.generate_lods = parse_optional_model_import_bool(
            settings,
            "generate_lods",
            model_settings.generate_lods,
        )?;
        model_settings.scale =
            parse_optional_model_import_scale(settings, "scale", model_settings.scale)?;
        Ok(model_settings)
    }

    fn imports_kind(&self, kind: &str) -> bool {
        match kind {
            "mesh" => self.import_meshes,
            "physics_mesh" => self.import_physics_meshes,
            "material" => self.import_materials,
            "animation" => self.import_animations,
            "skeleton" => self.import_skeleton,
            _ => true,
        }
    }
}

#[cfg(feature = "model_importer")]
fn parse_optional_model_import_bool(
    settings: &ImporterSettings,
    key: &str,
    default: bool,
) -> Result<bool, ImportError> {
    let Some(value) = settings.get(key) else {
        return Ok(default);
    };
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(AssetError::Import {
            message: format!(
                "invalid model import setting `{key}` value `{other}`; expected true or false"
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_optional_model_import_scale(
    settings: &ImporterSettings,
    key: &str,
    default: f32,
) -> Result<f32, ImportError> {
    let Some(value) = settings.get(key) else {
        return Ok(default);
    };
    let scale = value.parse::<f32>().map_err(|error| AssetError::Import {
        message: format!("invalid model import setting `{key}` value `{value}`: {error}"),
    })?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(AssetError::Import {
            message: format!(
                "invalid model import setting `{key}` value `{value}`; expected a finite positive scale"
            ),
        });
    }
    Ok(scale)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAsset {
    pub path: AssetPath,
    pub bytes: Vec<u8>,
    pub hash: ContentHash,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImporterSettings {
    values: HashMap<String, String>,
}

impl ImporterSettings {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn describe(&self) -> String {
        if self.values.is_empty() {
            return "<default>".to_owned();
        }
        self.to_sorted_pairs()
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn to_sorted_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = self
            .values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        pairs.sort_by(|left, right| left.0.cmp(&right.0));
        pairs
    }
}

#[derive(Default)]
pub struct ImportContext {
    generated: Vec<ImportGeneratedAsset>,
    dependencies: Vec<AssetId>,
    known_assets: HashMap<AssetPath, (AssetId, AssetTypeId)>,
    source_files: HashMap<AssetPath, SourceAsset>,
}

impl ImportContext {
    pub fn with_registry(registry: &AssetRegistry) -> Self {
        let mut context = Self::default();
        for metadata in registry.values() {
            if let Some(path) = &metadata.path {
                context.add_known_asset(path.clone(), metadata.id, metadata.asset_type);
            }
            if let Some(path) = &metadata.source_path {
                context.add_known_asset(path.clone(), metadata.id, metadata.asset_type);
            }
        }
        context
    }

    pub fn add_known_asset(&mut self, path: AssetPath, id: AssetId, asset_type: AssetTypeId) {
        self.known_assets.insert(path, (id, asset_type));
    }

    pub fn add_source_file(&mut self, source: SourceAsset) {
        self.source_files
            .insert(source.path.without_label(), source);
    }

    pub fn source_file(&self, path: &AssetPath) -> Option<&SourceAsset> {
        self.source_files.get(&path.without_label())
    }

    pub fn add_generated_asset(&mut self, asset: ImportGeneratedAsset) {
        self.generated.push(asset);
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }

    pub fn dependency<T: Asset>(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> Result<AssetId, ImportError> {
        self.add_dependency_by_path(path.into(), T::TYPE_ID)
    }

    pub fn add_dependency_by_path(
        &mut self,
        path: AssetPath,
        asset_type: AssetTypeId,
    ) -> Result<AssetId, ImportError> {
        let Some((id, actual_type)) = self.known_assets.get(&path).copied() else {
            return Err(AssetError::Import {
                message: format!(
                    "dependency `{}` is not registered in the asset registry",
                    path.display_string()
                ),
            });
        };
        if actual_type != asset_type {
            return Err(AssetError::Import {
                message: format!(
                    "dependency `{}` has asset type {:?}, expected {:?}",
                    path.display_string(),
                    actual_type,
                    asset_type
                ),
            });
        }
        self.add_dependency(id);
        Ok(id)
    }

    pub fn finish(self) -> (Vec<ImportGeneratedAsset>, Vec<AssetId>) {
        (self.generated, self.dependencies)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportGeneratedAsset {
    pub id: AssetId,
    pub path: AssetPath,
    pub asset_type: AssetTypeId,
    pub bytes: Vec<u8>,
    pub labels: Vec<String>,
    pub dependencies: Vec<AssetId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportOutput {
    pub metadata: AssetMetadata,
    pub generated: Vec<ImportGeneratedAsset>,
    pub dependencies: Vec<AssetId>,
    pub version_hash: VersionHash,
}

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

#[derive(Default)]
pub struct ImporterRegistry {
    importers: Vec<Box<dyn AssetImporter>>,
}

impl ImporterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<I: AssetImporter>(&mut self, importer: I) {
        self.importers.push(Box::new(importer));
    }

    pub fn importer_for_extension(&self, extension: &str) -> Option<&dyn AssetImporter> {
        self.importers.iter().map(Box::as_ref).find(|importer| {
            importer
                .extensions()
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
    }

    pub fn import(
        &self,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let extension = source.path.extension().unwrap_or("");
        let importer =
            self.importer_for_extension(extension)
                .ok_or_else(|| AssetError::Import {
                    message: format!("no importer registered for extension `{extension}`"),
                })?;
        let mut ctx = ImportContext::default();
        ctx.add_source_file(source.clone());
        importer.import(&mut ctx, source, settings)
    }
}

#[cfg(feature = "texture_importer")]
pub struct TextureImporter;

#[cfg(feature = "texture_importer")]
impl TextureImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "texture_importer")]
impl Default for TextureImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "texture_importer")]
impl AssetImporter for TextureImporter {
    fn name(&self) -> &'static str {
        "TextureImporter"
    }

    fn version(&self) -> u32 {
        2
    }

    fn extensions(&self) -> &[&'static str] {
        &["texture", "tex", "rgba"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let id = AssetId::new();
        let bytes = import_texture_bytes(source)?;
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), Texture::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: Texture::TYPE_ID,
            bytes,
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated.clone());
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "texture_importer")]
fn import_texture_bytes(source: &SourceAsset) -> Result<Vec<u8>, ImportError> {
    if !source.bytes.starts_with(b"NGA_TEXTURE_SOURCE_V1") {
        return Ok(source.bytes.clone());
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
        message: format!("texture source must be UTF-8: {error}"),
    })?;
    parse_texture_source(text)
}

#[cfg(feature = "texture_importer")]
fn parse_texture_source(text: &str) -> Result<Vec<u8>, ImportError> {
    let mut lines = text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_TEXTURE_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "texture source must start with NGA_TEXTURE_SOURCE_V1".to_owned(),
        });
    }
    let mut width = None;
    let mut height = None;
    let mut pixels = None;
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid texture source line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "width" => width = Some(parse_texture_dimension(value, "width", line_number)?),
            "height" => height = Some(parse_texture_dimension(value, "height", line_number)?),
            "size" => {
                let Some((width_value, height_value)) = value.split_once('x') else {
                    return Err(AssetError::Import {
                        message: format!("texture source size on line {line_number} must be WxH"),
                    });
                };
                width = Some(parse_texture_dimension(
                    width_value.trim(),
                    "width",
                    line_number,
                )?);
                height = Some(parse_texture_dimension(
                    height_value.trim(),
                    "height",
                    line_number,
                )?);
            }
            "rgba" | "pixels" => pixels = Some(parse_texture_pixels(value, line_number)?),
            other => {
                return Err(AssetError::Import {
                    message: format!("unknown texture source key `{other}` on line {line_number}"),
                })
            }
        }
    }
    let width = width.ok_or_else(|| AssetError::Import {
        message: "texture source missing width or size".to_owned(),
    })?;
    let height = height.ok_or_else(|| AssetError::Import {
        message: "texture source missing height or size".to_owned(),
    })?;
    let pixels = pixels.ok_or_else(|| AssetError::Import {
        message: "texture source missing rgba pixels".to_owned(),
    })?;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AssetError::Import {
            message: "texture source dimensions overflow pixel count".to_owned(),
        })?;
    if pixels.len() != expected {
        return Err(AssetError::Import {
            message: format!(
                "texture source rgba byte count {} did not match expected {expected}",
                pixels.len()
            ),
        });
    }
    let mut bytes = Vec::with_capacity(8 + pixels.len());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&pixels);
    Ok(bytes)
}

#[cfg(feature = "texture_importer")]
fn parse_texture_dimension(
    value: &str,
    name: &str,
    line_number: usize,
) -> Result<u32, ImportError> {
    let dimension = value.parse::<u32>().map_err(|error| AssetError::Import {
        message: format!("invalid texture source {name} on line {line_number}: {error}"),
    })?;
    if dimension == 0 {
        return Err(AssetError::Import {
            message: format!("texture source {name} on line {line_number} must be non-zero"),
        });
    }
    Ok(dimension)
}

#[cfg(feature = "texture_importer")]
fn parse_texture_pixels(value: &str, line_number: usize) -> Result<Vec<u8>, ImportError> {
    value
        .replace(';', ",")
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<u8>().map_err(|error| AssetError::Import {
                message: format!(
                    "invalid texture source rgba byte `{part}` on line {line_number}: {error}"
                ),
            })
        })
        .collect()
}

#[cfg(feature = "model_importer")]
pub struct MeshImporter;

#[cfg(feature = "model_importer")]
impl MeshImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "model_importer")]
impl Default for MeshImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "model_importer")]
impl AssetImporter for MeshImporter {
    fn name(&self) -> &'static str {
        "MeshImporter"
    }

    fn version(&self) -> u32 {
        4
    }

    fn extensions(&self) -> &[&'static str] {
        &["mesh"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let id = AssetId::new();
        let bytes = import_mesh_bytes(source)?;
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), Mesh::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: Mesh::TYPE_ID,
            bytes,
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated);
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "model_importer")]
fn import_mesh_bytes(source: &SourceAsset) -> Result<Vec<u8>, ImportError> {
    if source
        .bytes
        .starts_with(crate::assets::mesh::MESH_BINARY_MAGIC)
    {
        crate::assets::mesh::decode_mesh(&source.bytes).map_err(|error| AssetError::Import {
            message: format!("mesh binary source is invalid: {error}"),
        })?;
        return Ok(source.bytes.clone());
    }
    if !source.bytes.starts_with(b"NGA_MESH_SOURCE_V1") {
        return Ok(source.bytes.clone());
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
        message: format!("mesh source must be UTF-8: {error}"),
    })?;
    canonical_mesh_source(text).map(String::into_bytes)
}

#[cfg(feature = "model_importer")]
fn canonical_mesh_source(source_text: &str) -> Result<String, ImportError> {
    let mut lines = source_text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_MESH_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "mesh source must start with NGA_MESH_SOURCE_V1".to_owned(),
        });
    }
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut uv_sets = Vec::<Vec<[f32; 2]>>::new();
    let mut tangents = Vec::new();
    let mut joints = Vec::new();
    let mut weights = Vec::new();
    let mut indices = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "vertex" | "v" => {
                    vertices.push(parse_mesh_source_vertex(value.trim(), line_number)?)
                }
                "normal" | "n" => {
                    normals.push(parse_mesh_source_normal(value.trim(), line_number)?)
                }
                key if mesh_source_uv_key_index(key).is_some() => {
                    let set_index =
                        mesh_source_uv_key_index(key).expect("matched mesh source uv key");
                    let value = parse_mesh_source_uv(value.trim(), line_number)?;
                    push_mesh_source_uv(&mut uvs, &mut uv_sets, set_index, value);
                }
                "tangent" | "t" => {
                    tangents.push(parse_mesh_source_tangent(value.trim(), line_number)?)
                }
                "joint" | "joints" | "j" => {
                    joints.push(parse_mesh_source_joint(value.trim(), line_number)?)
                }
                "weight" | "weights" | "w" => {
                    weights.push(parse_mesh_source_weight(value.trim(), line_number)?)
                }
                "triangle" | "i" => {
                    indices.push(parse_mesh_source_index(value.trim(), line_number)?)
                }
                other => {
                    return Err(AssetError::Import {
                        message: format!("unknown mesh source key `{other}` on line {line_number}"),
                    })
                }
            }
            continue;
        }

        let mut parts = line.splitn(2, char::is_whitespace);
        let directive = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("").trim();
        match directive {
            "v" => vertices.push(parse_mesh_source_vertex(value, line_number)?),
            "n" => normals.push(parse_mesh_source_normal(value, line_number)?),
            directive if mesh_source_uv_key_index(directive).is_some() => {
                let set_index =
                    mesh_source_uv_key_index(directive).expect("matched mesh source uv directive");
                let value = parse_mesh_source_uv(value, line_number)?;
                push_mesh_source_uv(&mut uvs, &mut uv_sets, set_index, value);
            }
            "t" => tangents.push(parse_mesh_source_tangent(value, line_number)?),
            "j" | "joints" => joints.push(parse_mesh_source_joint(value, line_number)?),
            "w" | "weights" => weights.push(parse_mesh_source_weight(value, line_number)?),
            "i" => indices.push(parse_mesh_source_index(value, line_number)?),
            other => {
                return Err(AssetError::Import {
                    message: format!(
                        "unknown mesh source directive `{other}` on line {line_number}"
                    ),
                })
            }
        }
    }
    if vertices.is_empty() {
        return Err(AssetError::Import {
            message: "mesh source must contain at least one vertex".to_owned(),
        });
    }
    if !normals.is_empty() && normals.len() != vertices.len() {
        return Err(AssetError::Import {
            message: format!(
                "mesh source normal count {} must match vertex count {}",
                normals.len(),
                vertices.len()
            ),
        });
    }
    if !uvs.is_empty() && uvs.len() != vertices.len() {
        return Err(AssetError::Import {
            message: format!(
                "mesh source uv count {} must match vertex count {}",
                uvs.len(),
                vertices.len()
            ),
        });
    }
    if !uv_sets.is_empty() && uvs.is_empty() {
        return Err(AssetError::Import {
            message: "mesh source secondary uv sets require primary uv coordinates".to_owned(),
        });
    }
    validate_mesh_source_uv_sets(&uv_sets, vertices.len())?;
    if !tangents.is_empty() && tangents.len() != vertices.len() {
        return Err(AssetError::Import {
            message: format!(
                "mesh source tangent count {} must match vertex count {}",
                tangents.len(),
                vertices.len()
            ),
        });
    }
    validate_mesh_source_skinning(&joints, &weights, vertices.len())?;
    for triangle in &indices {
        for index in triangle {
            if *index as usize >= vertices.len() {
                return Err(AssetError::Import {
                    message: format!(
                        "mesh source index {index} references missing vertex; vertex count is {}",
                        vertices.len()
                    ),
                });
            }
        }
    }

    let mut canonical = String::new();
    for vertex in vertices {
        canonical.push_str(&format!(
            "v {} {} {}\n",
            canonical_mesh_f32(vertex[0]),
            canonical_mesh_f32(vertex[1]),
            canonical_mesh_f32(vertex[2])
        ));
    }
    for normal in normals {
        canonical.push_str(&format!(
            "n {} {} {}\n",
            canonical_mesh_f32(normal[0]),
            canonical_mesh_f32(normal[1]),
            canonical_mesh_f32(normal[2])
        ));
    }
    for uv in uvs {
        canonical.push_str(&format!(
            "uv {} {}\n",
            canonical_mesh_f32(uv[0]),
            canonical_mesh_f32(uv[1])
        ));
    }
    for (index, uvs) in uv_sets.into_iter().enumerate() {
        for uv in uvs {
            canonical.push_str(&format!(
                "uv{} {} {}\n",
                index + 1,
                canonical_mesh_f32(uv[0]),
                canonical_mesh_f32(uv[1])
            ));
        }
    }
    for tangent in tangents {
        canonical.push_str(&format!(
            "t {} {} {} {}\n",
            canonical_mesh_f32(tangent[0]),
            canonical_mesh_f32(tangent[1]),
            canonical_mesh_f32(tangent[2]),
            canonical_mesh_f32(tangent[3])
        ));
    }
    for joint in joints {
        canonical.push_str(&format!(
            "j {} {} {} {}\n",
            joint[0], joint[1], joint[2], joint[3]
        ));
    }
    for weight in weights {
        canonical.push_str(&format!(
            "w {} {} {} {}\n",
            canonical_mesh_f32(weight[0]),
            canonical_mesh_f32(weight[1]),
            canonical_mesh_f32(weight[2]),
            canonical_mesh_f32(weight[3])
        ));
    }
    for triangle in indices {
        canonical.push_str(&format!(
            "i {} {} {}\n",
            triangle[0], triangle[1], triangle[2]
        ));
    }
    Ok(canonical)
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_vertex(value: &str, line_number: usize) -> Result<[f32; 3], ImportError> {
    parse_mesh_source_triplet(value, line_number, "vertex")
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_normal(value: &str, line_number: usize) -> Result<[f32; 3], ImportError> {
    parse_mesh_source_triplet(value, line_number, "normal")
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_triplet(
    value: &str,
    line_number: usize,
    kind: &str,
) -> Result<[f32; 3], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0.0; 3];
    for component in &mut values {
        *component = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing mesh source {kind} value on line {line_number}"),
            })?
            .parse::<f32>()
            .map_err(|error| AssetError::Import {
                message: format!("invalid mesh source {kind} value on line {line_number}: {error}"),
            })?;
        if !component.is_finite() {
            return Err(AssetError::Import {
                message: format!("mesh source {kind} value must be finite on line {line_number}"),
            });
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many mesh source {kind} values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_uv(value: &str, line_number: usize) -> Result<[f32; 2], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0.0; 2];
    for component in &mut values {
        *component = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing mesh source uv value on line {line_number}"),
            })?
            .parse::<f32>()
            .map_err(|error| AssetError::Import {
                message: format!("invalid mesh source uv value on line {line_number}: {error}"),
            })?;
        if !component.is_finite() {
            return Err(AssetError::Import {
                message: format!("mesh source uv value must be finite on line {line_number}"),
            });
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many mesh source uv values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn mesh_source_uv_key_index(key: &str) -> Option<usize> {
    match key {
        "uv" | "texcoord" | "texture_coordinate" => Some(0),
        other => other
            .strip_prefix("uv")
            .filter(|suffix| !suffix.is_empty())
            .and_then(|suffix| suffix.parse::<usize>().ok()),
    }
}

#[cfg(feature = "model_importer")]
fn push_mesh_source_uv(
    uvs: &mut Vec<[f32; 2]>,
    uv_sets: &mut Vec<Vec<[f32; 2]>>,
    set_index: usize,
    value: [f32; 2],
) {
    if set_index == 0 {
        uvs.push(value);
        return;
    }
    while uv_sets.len() < set_index {
        uv_sets.push(Vec::new());
    }
    uv_sets[set_index - 1].push(value);
}

#[cfg(feature = "model_importer")]
fn validate_mesh_source_uv_sets(
    uv_sets: &[Vec<[f32; 2]>],
    vertex_count: usize,
) -> Result<(), ImportError> {
    for (index, uvs) in uv_sets.iter().enumerate() {
        if !uvs.is_empty() && uvs.len() != vertex_count {
            return Err(AssetError::Import {
                message: format!(
                    "mesh source uv{} count {} must match vertex count {}",
                    index + 1,
                    uvs.len(),
                    vertex_count
                ),
            });
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_tangent(value: &str, line_number: usize) -> Result<[f32; 4], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0.0; 4];
    for component in &mut values {
        *component = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing mesh source tangent value on line {line_number}"),
            })?
            .parse::<f32>()
            .map_err(|error| AssetError::Import {
                message: format!(
                    "invalid mesh source tangent value on line {line_number}: {error}"
                ),
            })?;
        if !component.is_finite() {
            return Err(AssetError::Import {
                message: format!("mesh source tangent value must be finite on line {line_number}"),
            });
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many mesh source tangent values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_joint(value: &str, line_number: usize) -> Result<[u16; 4], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0; 4];
    for component in &mut values {
        *component = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing mesh source joint value on line {line_number}"),
            })?
            .parse::<u16>()
            .map_err(|error| AssetError::Import {
                message: format!("invalid mesh source joint value on line {line_number}: {error}"),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many mesh source joint values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_weight(value: &str, line_number: usize) -> Result<[f32; 4], ImportError> {
    let values = parse_mesh_source_tangent(value, line_number).map_err(|error| match error {
        AssetError::Import { message } => AssetError::Import {
            message: message.replace("tangent", "weight"),
        },
        other => other,
    })?;
    if values.iter().any(|value| *value < 0.0) {
        return Err(AssetError::Import {
            message: format!("mesh source weight value must be non-negative on line {line_number}"),
        });
    }
    validate_mesh_source_skin_weight_total(values, line_number)?;
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn validate_mesh_source_skin_weight_total(
    values: [f32; 4],
    line_number: usize,
) -> Result<(), ImportError> {
    let total = values.iter().sum::<f32>();
    if total <= f32::EPSILON {
        return Err(AssetError::Import {
            message: format!(
                "mesh source skin weight total must be positive on line {line_number}"
            ),
        });
    }
    if (total - 1.0).abs() > SKIN_WEIGHT_SUM_EPSILON {
        return Err(AssetError::Import {
            message: format!(
                "mesh source skin weights on line {line_number} must sum to 1.0, found {total}"
            ),
        });
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_mesh_source_skinning(
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    vertex_count: usize,
) -> Result<(), ImportError> {
    if joints.len() != weights.len() {
        return Err(AssetError::Import {
            message: format!(
                "mesh source skin joint count {} must match skin weight count {}",
                joints.len(),
                weights.len()
            ),
        });
    }
    if !joints.is_empty() && joints.len() != vertex_count {
        return Err(AssetError::Import {
            message: format!(
                "mesh source skin joint count {} must match vertex count {}",
                joints.len(),
                vertex_count
            ),
        });
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn parse_mesh_source_index(value: &str, line_number: usize) -> Result<[u32; 3], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0; 3];
    for index in &mut values {
        *index = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing mesh source index value on line {line_number}"),
            })?
            .parse::<u32>()
            .map_err(|error| AssetError::Import {
                message: format!("invalid mesh source index value on line {line_number}: {error}"),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many mesh source index values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn canonical_mesh_f32(value: f32) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(feature = "shader_importer")]
pub struct ShaderImporter;

#[cfg(feature = "shader_importer")]
impl ShaderImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "shader_importer")]
impl Default for ShaderImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "shader_importer")]
impl AssetImporter for ShaderImporter {
    fn name(&self) -> &'static str {
        "ShaderImporter"
    }

    fn version(&self) -> u32 {
        2
    }

    fn extensions(&self) -> &[&'static str] {
        &["wgsl", "glsl", "shader"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let id = AssetId::new();
        let bytes = import_shader_bytes(source)?;
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), Shader::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: Shader::TYPE_ID,
            bytes,
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated);
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "shader_importer")]
fn import_shader_bytes(source: &SourceAsset) -> Result<Vec<u8>, ImportError> {
    if !source.bytes.starts_with(b"NGA_SHADER_SOURCE_V1") {
        return Ok(source.bytes.clone());
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
        message: format!("shader source must be UTF-8: {error}"),
    })?;
    canonical_shader_source(text).map(String::into_bytes)
}

#[cfg(feature = "shader_importer")]
fn canonical_shader_source(source_text: &str) -> Result<String, ImportError> {
    let mut lines = source_text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_SHADER_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "shader source must start with NGA_SHADER_SOURCE_V1".to_owned(),
        });
    }
    let mut language = None;
    let mut inline_source = None;
    let mut body_lines = Vec::new();
    let mut in_body = false;
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        if in_body {
            body_lines.push(line);
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "---" {
            in_body = true;
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid shader source line {line_number}"),
            });
        };
        match key.trim() {
            "language" => language = Some(parse_shader_source_language(value.trim(), line_number)?),
            "source" => {
                if inline_source.is_some() || !body_lines.is_empty() {
                    return Err(AssetError::Import {
                        message: format!("shader source body is repeated on line {line_number}"),
                    });
                }
                inline_source = Some(value.trim().to_owned());
            }
            "entry" | "stage" => {}
            other => {
                return Err(AssetError::Import {
                    message: format!("unknown shader source key `{other}` on line {line_number}"),
                })
            }
        }
    }
    let _language = language.ok_or_else(|| AssetError::Import {
        message: "shader source missing language".to_owned(),
    })?;
    let source = match (inline_source, body_lines.is_empty()) {
        (Some(source), true) => source,
        (Some(_), false) => {
            return Err(AssetError::Import {
                message: "shader source body is repeated".to_owned(),
            })
        }
        (None, false) => body_lines.join("\n"),
        (None, true) => {
            return Err(AssetError::Import {
                message: "shader source missing body".to_owned(),
            })
        }
    };
    let source = source.trim();
    if source.is_empty() {
        return Err(AssetError::Import {
            message: "shader source body is empty".to_owned(),
        });
    }
    Ok(format!("{source}\n"))
}

#[cfg(feature = "shader_importer")]
fn parse_shader_source_language(
    value: &str,
    line_number: usize,
) -> Result<&'static str, ImportError> {
    match value {
        "wgsl" => Ok("wgsl"),
        other => Err(AssetError::Import {
            message: format!("unsupported shader source language `{other}` on line {line_number}"),
        }),
    }
}

#[cfg(feature = "importers")]
pub struct FontImporter;

#[cfg(feature = "importers")]
impl FontImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "importers")]
impl Default for FontImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "importers")]
impl AssetImporter for FontImporter {
    fn name(&self) -> &'static str {
        "FontImporter"
    }

    fn version(&self) -> u32 {
        2
    }

    fn extensions(&self) -> &[&'static str] {
        &["font"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let id = AssetId::new();
        let bytes = import_font_bytes(source)?;
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), Font::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: Font::TYPE_ID,
            bytes,
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated);
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "importers")]
fn import_font_bytes(source: &SourceAsset) -> Result<Vec<u8>, ImportError> {
    if !source.bytes.starts_with(b"NGA_FONT_SOURCE_V1") {
        return Ok(source.bytes.clone());
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
        message: format!("font source must be UTF-8: {error}"),
    })?;
    canonical_font_source(text).map(String::into_bytes)
}

#[cfg(feature = "importers")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct CanonicalFontGlyph {
    codepoint: char,
    width: u32,
    height: u32,
    bitmap: Vec<u8>,
}

#[cfg(feature = "importers")]
fn canonical_font_source(source_text: &str) -> Result<String, ImportError> {
    let mut lines = source_text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_FONT_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "font source must start with NGA_FONT_SOURCE_V1".to_owned(),
        });
    }
    let mut family_name = None;
    let mut glyphs = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid font source line {line_number}"),
            });
        };
        match key.trim() {
            "family" => {
                if family_name.is_some() {
                    return Err(AssetError::Import {
                        message: format!("font source repeats family on line {line_number}"),
                    });
                }
                let value = value.trim();
                if value.is_empty() {
                    return Err(AssetError::Import {
                        message: format!("font source family is empty on line {line_number}"),
                    });
                }
                family_name = Some(value.to_owned());
            }
            "glyph" => glyphs.push(parse_font_source_glyph(value.trim(), line_number)?),
            other => {
                return Err(AssetError::Import {
                    message: format!("unknown font source key `{other}` on line {line_number}"),
                })
            }
        }
    }
    let family_name = family_name.ok_or_else(|| AssetError::Import {
        message: "font source missing family".to_owned(),
    })?;
    if glyphs.is_empty() {
        return Err(AssetError::Import {
            message: "font source must contain at least one glyph".to_owned(),
        });
    }
    glyphs.sort_by_key(|glyph| glyph.codepoint);
    for pair in glyphs.windows(2) {
        if pair[0].codepoint == pair[1].codepoint {
            return Err(AssetError::Import {
                message: format!("font source repeats glyph `{}`", pair[0].codepoint),
            });
        }
    }

    let mut canonical = format!("NGA_FONT_V1\nfamily={family_name}\n");
    for glyph in glyphs {
        let bitmap = glyph
            .bitmap
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",");
        canonical.push_str(&format!(
            "glyph=char={};size={}x{};bitmap={bitmap}\n",
            glyph.codepoint, glyph.width, glyph.height
        ));
    }
    Ok(canonical)
}

#[cfg(feature = "importers")]
fn parse_font_source_glyph(
    value: &str,
    line_number: usize,
) -> Result<CanonicalFontGlyph, ImportError> {
    let mut codepoint = None;
    let mut size = None;
    let mut bitmap = None;
    for part in value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let Some((key, value)) = part.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid font source glyph field on line {line_number}"),
            });
        };
        match (key.trim(), value.trim()) {
            ("char", value) => {
                let mut chars = value.chars();
                let Some(character) = chars.next() else {
                    return Err(AssetError::Import {
                        message: format!("font source glyph char is empty on line {line_number}"),
                    });
                };
                if chars.next().is_some() {
                    return Err(AssetError::Import {
                        message: format!(
                            "font source glyph char must be one scalar on line {line_number}"
                        ),
                    });
                }
                codepoint = Some(character);
            }
            ("size", value) => size = Some(parse_font_source_glyph_size(value, line_number)?),
            ("bitmap", value) => {
                bitmap = Some(parse_font_source_bitmap(value, line_number)?);
            }
            (other, _) => {
                return Err(AssetError::Import {
                    message: format!(
                        "unknown font source glyph field `{other}` on line {line_number}"
                    ),
                })
            }
        }
    }
    let codepoint = codepoint.ok_or_else(|| AssetError::Import {
        message: format!("font source glyph missing char on line {line_number}"),
    })?;
    let (width, height) = size.ok_or_else(|| AssetError::Import {
        message: format!("font source glyph missing size on line {line_number}"),
    })?;
    let bitmap = bitmap.ok_or_else(|| AssetError::Import {
        message: format!("font source glyph missing bitmap on line {line_number}"),
    })?;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| AssetError::Import {
            message: format!("font source glyph bitmap dimensions overflow on line {line_number}"),
        })?;
    if bitmap.len() != expected {
        return Err(AssetError::Import {
            message: format!(
                "font source glyph bitmap on line {line_number} has {} bytes, expected {expected}",
                bitmap.len()
            ),
        });
    }
    Ok(CanonicalFontGlyph {
        codepoint,
        width,
        height,
        bitmap,
    })
}

#[cfg(feature = "importers")]
fn parse_font_source_glyph_size(
    value: &str,
    line_number: usize,
) -> Result<(u32, u32), ImportError> {
    let Some((width, height)) = value.split_once('x') else {
        return Err(AssetError::Import {
            message: format!("invalid font source glyph size on line {line_number}"),
        });
    };
    let width = width
        .trim()
        .parse::<u32>()
        .map_err(|error| AssetError::Import {
            message: format!("invalid font source glyph width on line {line_number}: {error}"),
        })?;
    let height = height
        .trim()
        .parse::<u32>()
        .map_err(|error| AssetError::Import {
            message: format!("invalid font source glyph height on line {line_number}: {error}"),
        })?;
    if width == 0 || height == 0 {
        return Err(AssetError::Import {
            message: format!("font source glyph size must be non-zero on line {line_number}"),
        });
    }
    Ok((width, height))
}

#[cfg(feature = "importers")]
fn parse_font_source_bitmap(value: &str, line_number: usize) -> Result<Vec<u8>, ImportError> {
    if value.trim().is_empty() {
        return Err(AssetError::Import {
            message: format!("font source glyph bitmap is empty on line {line_number}"),
        });
    }
    value
        .split(',')
        .map(str::trim)
        .map(|part| {
            part.parse::<u8>().map_err(|error| AssetError::Import {
                message: format!(
                    "invalid font source glyph bitmap value on line {line_number}: {error}"
                ),
            })
        })
        .collect()
}

#[cfg(feature = "importers")]
pub struct PhysicsMeshImporter;

#[cfg(feature = "importers")]
impl PhysicsMeshImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "importers")]
impl Default for PhysicsMeshImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "importers")]
impl AssetImporter for PhysicsMeshImporter {
    fn name(&self) -> &'static str {
        "PhysicsMeshImporter"
    }

    fn version(&self) -> u32 {
        2
    }

    fn extensions(&self) -> &[&'static str] {
        &["physics", "physicsmesh", "pmesh"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let id = AssetId::new();
        let bytes = import_physics_mesh_bytes(source)?;
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), PhysicsMesh::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: PhysicsMesh::TYPE_ID,
            bytes,
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated);
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "importers")]
fn import_physics_mesh_bytes(source: &SourceAsset) -> Result<Vec<u8>, ImportError> {
    if !source.bytes.starts_with(b"NGA_PHYSICS_MESH_SOURCE_V1") {
        return Ok(source.bytes.clone());
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
        message: format!("physics mesh source must be UTF-8: {error}"),
    })?;
    canonical_physics_mesh_source(text).map(String::into_bytes)
}

#[cfg(feature = "importers")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CanonicalPhysicsMeshKind {
    TriMesh,
    ConvexHull,
    HeightField,
}

#[cfg(feature = "importers")]
impl CanonicalPhysicsMeshKind {
    fn runtime_name(self) -> &'static str {
        match self {
            Self::TriMesh => "trimesh",
            Self::ConvexHull => "convex",
            Self::HeightField => "heightfield",
        }
    }
}

#[cfg(feature = "importers")]
fn canonical_physics_mesh_source(source_text: &str) -> Result<String, ImportError> {
    let mut lines = source_text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_PHYSICS_MESH_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "physics mesh source must start with NGA_PHYSICS_MESH_SOURCE_V1".to_owned(),
        });
    }
    let mut kind = None;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "kind" => {
                    kind = Some(parse_physics_mesh_source_kind(value.trim(), line_number)?);
                }
                "vertex" | "v" => {
                    vertices.push(parse_physics_mesh_source_vertex(value.trim(), line_number)?);
                }
                "triangle" | "i" => {
                    indices.push(parse_physics_mesh_source_index(value.trim(), line_number)?);
                }
                other => {
                    return Err(AssetError::Import {
                        message: format!(
                            "unknown physics mesh source key `{other}` on line {line_number}"
                        ),
                    })
                }
            }
            continue;
        }

        let mut parts = line.splitn(2, char::is_whitespace);
        let directive = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("").trim();
        match directive {
            "v" => vertices.push(parse_physics_mesh_source_vertex(value, line_number)?),
            "i" => indices.push(parse_physics_mesh_source_index(value, line_number)?),
            other => {
                return Err(AssetError::Import {
                    message: format!(
                        "unknown physics mesh source directive `{other}` on line {line_number}"
                    ),
                })
            }
        }
    }
    let kind = kind.ok_or_else(|| AssetError::Import {
        message: "physics mesh source missing kind".to_owned(),
    })?;
    if vertices.is_empty() {
        return Err(AssetError::Import {
            message: "physics mesh source must contain at least one vertex".to_owned(),
        });
    }
    if kind != CanonicalPhysicsMeshKind::ConvexHull && indices.is_empty() {
        return Err(AssetError::Import {
            message: "physics mesh source must contain at least one triangle".to_owned(),
        });
    }
    for triangle in &indices {
        for index in triangle {
            if *index as usize >= vertices.len() {
                return Err(AssetError::Import {
                    message: format!(
                        "physics mesh source index {index} references missing vertex; vertex count is {}",
                        vertices.len()
                    ),
                });
            }
        }
    }

    let mut canonical = format!("NGA_PHYSICS_MESH_V1\nkind={}\n", kind.runtime_name());
    for vertex in vertices {
        canonical.push_str(&format!(
            "v {} {} {}\n",
            canonical_physics_mesh_f32(vertex[0]),
            canonical_physics_mesh_f32(vertex[1]),
            canonical_physics_mesh_f32(vertex[2])
        ));
    }
    for triangle in indices {
        canonical.push_str(&format!(
            "i {} {} {}\n",
            triangle[0], triangle[1], triangle[2]
        ));
    }
    Ok(canonical)
}

#[cfg(feature = "importers")]
fn parse_physics_mesh_source_kind(
    value: &str,
    line_number: usize,
) -> Result<CanonicalPhysicsMeshKind, ImportError> {
    match value {
        "trimesh" | "tri_mesh" => Ok(CanonicalPhysicsMeshKind::TriMesh),
        "convex" | "convex_hull" => Ok(CanonicalPhysicsMeshKind::ConvexHull),
        "heightfield" | "height_field" => Ok(CanonicalPhysicsMeshKind::HeightField),
        other => Err(AssetError::Import {
            message: format!("unknown physics mesh source kind `{other}` on line {line_number}"),
        }),
    }
}

#[cfg(feature = "importers")]
fn parse_physics_mesh_source_vertex(
    value: &str,
    line_number: usize,
) -> Result<[f32; 3], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0.0; 3];
    for component in &mut values {
        *component = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing physics mesh source vertex value on line {line_number}"),
            })?
            .parse::<f32>()
            .map_err(|error| AssetError::Import {
                message: format!(
                    "invalid physics mesh source vertex value on line {line_number}: {error}"
                ),
            })?;
        if !component.is_finite() {
            return Err(AssetError::Import {
                message: format!(
                    "physics mesh source vertex value must be finite on line {line_number}"
                ),
            });
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many physics mesh source vertex values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "importers")]
fn parse_physics_mesh_source_index(
    value: &str,
    line_number: usize,
) -> Result<[u32; 3], ImportError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let mut values = [0; 3];
    for index in &mut values {
        *index = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing physics mesh source index value on line {line_number}"),
            })?
            .parse::<u32>()
            .map_err(|error| AssetError::Import {
                message: format!(
                    "invalid physics mesh source index value on line {line_number}: {error}"
                ),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many physics mesh source index values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "importers")]
fn canonical_physics_mesh_f32(value: f32) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(feature = "audio_importer")]
pub struct AudioImporter;

#[cfg(feature = "audio_importer")]
impl AudioImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "audio_importer")]
impl Default for AudioImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "audio_importer")]
impl AssetImporter for AudioImporter {
    fn name(&self) -> &'static str {
        "AudioImporter"
    }

    fn version(&self) -> u32 {
        3
    }

    fn extensions(&self) -> &[&'static str] {
        &["audio", "wav", "ogg"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let id = AssetId::new();
        let bytes = import_audio_bytes(source, settings)?;
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), AudioClip::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: AudioClip::TYPE_ID,
            bytes,
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated.clone());
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "audio_importer")]
#[derive(Clone, Copy, Debug, Default)]
struct AudioImporterOptions {
    force_mono: bool,
    normalize: bool,
    streaming: Option<bool>,
    compression: Option<AudioCompression>,
}

#[cfg(feature = "audio_importer")]
impl AudioImporterOptions {
    fn from_importer_settings(settings: &ImporterSettings) -> Result<Self, ImportError> {
        Ok(Self {
            force_mono: parse_optional_audio_import_bool(settings, "force_mono", false)?,
            normalize: parse_optional_audio_import_bool(settings, "normalize", false)?,
            streaming: parse_optional_audio_import_bool_option(settings, "streaming")?,
            compression: parse_optional_audio_import_compression(settings)?,
        })
    }
}

#[cfg(feature = "audio_importer")]
fn parse_optional_audio_import_bool_option(
    settings: &ImporterSettings,
    key: &str,
) -> Result<Option<bool>, ImportError> {
    let Some(value) = settings.get(key) else {
        return Ok(None);
    };
    match value {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        other => Err(AssetError::Import {
            message: format!(
                "invalid audio import setting `{key}` value `{other}`; expected true or false"
            ),
        }),
    }
}

#[cfg(feature = "audio_importer")]
fn parse_optional_audio_import_compression(
    settings: &ImporterSettings,
) -> Result<Option<AudioCompression>, ImportError> {
    let Some(value) = settings.get("compression") else {
        return Ok(None);
    };
    let compression = match value {
        "none" => AudioCompression::None,
        "vorbis" => AudioCompression::Vorbis,
        "opus" => AudioCompression::Opus,
        other => {
            return Err(AssetError::Import {
                message: format!(
                    "invalid audio import setting `compression` value `{other}`; expected none, vorbis, or opus"
                ),
            })
        }
    };
    Ok(Some(compression))
}

#[cfg(feature = "audio_importer")]
fn parse_optional_audio_import_bool(
    settings: &ImporterSettings,
    key: &str,
    default: bool,
) -> Result<bool, ImportError> {
    let Some(value) = settings.get(key) else {
        return Ok(default);
    };
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(AssetError::Import {
            message: format!(
                "invalid audio import setting `{key}` value `{other}`; expected true or false"
            ),
        }),
    }
}

#[cfg(feature = "audio_importer")]
fn import_audio_bytes(
    source: &SourceAsset,
    settings: &ImporterSettings,
) -> Result<Vec<u8>, ImportError> {
    if !source.bytes.starts_with(b"NGA_AUDIO_SOURCE_V1") {
        return Ok(source.bytes.clone());
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
        message: format!("audio source must be UTF-8: {error}"),
    })?;
    let options = AudioImporterOptions::from_importer_settings(settings)?;
    canonical_audio_source(text, options).map(String::into_bytes)
}

#[cfg(feature = "audio_importer")]
fn canonical_audio_source(
    source_text: &str,
    options: AudioImporterOptions,
) -> Result<String, ImportError> {
    let mut lines = source_text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_AUDIO_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "audio source must start with NGA_AUDIO_SOURCE_V1".to_owned(),
        });
    }
    let mut sample_rate = None;
    let mut channels = None;
    let mut sample_format = None;
    let mut samples = None;
    let mut streaming = false;
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid audio source line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "sample_rate" => {
                sample_rate = Some(parse_audio_sample_rate(value, line_number)?);
            }
            "channels" => {
                channels = Some(parse_audio_channels(value, line_number)?);
            }
            "format" => sample_format = Some(parse_audio_format(value, line_number)?),
            "samples" | "frames" => samples = Some(value.to_owned()),
            "streaming" => streaming = parse_audio_bool(value, line_number)?,
            other => {
                return Err(AssetError::Import {
                    message: format!("unknown audio source key `{other}` on line {line_number}"),
                })
            }
        }
    }
    let sample_rate = sample_rate.ok_or_else(|| AssetError::Import {
        message: "audio source missing sample_rate".to_owned(),
    })?;
    let channels = channels.ok_or_else(|| AssetError::Import {
        message: "audio source missing channels".to_owned(),
    })?;
    let sample_format = sample_format.ok_or_else(|| AssetError::Import {
        message: "audio source missing format".to_owned(),
    })?;
    let samples = samples.ok_or_else(|| AssetError::Import {
        message: "audio source missing samples".to_owned(),
    })?;
    let streaming = options.streaming.unwrap_or(streaming);
    match options.compression.unwrap_or(AudioCompression::None) {
        AudioCompression::None => {}
        AudioCompression::Vorbis => {
            return Err(AssetError::Import {
                message: "unsupported audio import compression `vorbis`; expected `none`"
                    .to_owned(),
            })
        }
        AudioCompression::Opus => {
            return Err(AssetError::Import {
                message: "unsupported audio import compression `opus`; expected `none`".to_owned(),
            })
        }
    }
    let (samples, channels) = canonical_audio_samples(&samples, &sample_format, channels, options)?;
    Ok(format!(
        "NGA_AUDIO_V1\nsample_rate={sample_rate}\nchannels={channels}\nformat={sample_format}\nsamples={samples}\nstreaming={streaming}\n"
    ))
}

#[cfg(feature = "audio_importer")]
fn parse_audio_sample_rate(value: &str, line_number: usize) -> Result<u32, ImportError> {
    let sample_rate = value.parse::<u32>().map_err(|error| AssetError::Import {
        message: format!("invalid audio source sample_rate on line {line_number}: {error}"),
    })?;
    if sample_rate == 0 {
        return Err(AssetError::Import {
            message: format!("audio source sample_rate on line {line_number} must be non-zero"),
        });
    }
    Ok(sample_rate)
}

#[cfg(feature = "audio_importer")]
fn parse_audio_channels(value: &str, line_number: usize) -> Result<u16, ImportError> {
    let channels = value.parse::<u16>().map_err(|error| AssetError::Import {
        message: format!("invalid audio source channels on line {line_number}: {error}"),
    })?;
    if channels == 0 {
        return Err(AssetError::Import {
            message: format!("audio source channels on line {line_number} must be non-zero"),
        });
    }
    Ok(channels)
}

#[cfg(feature = "audio_importer")]
fn parse_audio_format(value: &str, line_number: usize) -> Result<String, ImportError> {
    match value {
        "i16" | "f32" => Ok(value.to_owned()),
        other => Err(AssetError::Import {
            message: format!("unsupported audio source format `{other}` on line {line_number}"),
        }),
    }
}

#[cfg(feature = "audio_importer")]
fn parse_audio_bool(value: &str, line_number: usize) -> Result<bool, ImportError> {
    value.parse::<bool>().map_err(|error| AssetError::Import {
        message: format!("invalid audio source streaming flag on line {line_number}: {error}"),
    })
}

#[cfg(feature = "audio_importer")]
fn canonical_audio_samples(
    value: &str,
    sample_format: &str,
    channels: u16,
    options: AudioImporterOptions,
) -> Result<(String, u16), ImportError> {
    match sample_format {
        "i16" => canonical_audio_i16_samples(value, channels, options),
        "f32" => canonical_audio_f32_samples(value, channels, options),
        _ => unreachable!("sample format validated before parsing samples"),
    }
}

#[cfg(feature = "audio_importer")]
fn canonical_audio_i16_samples(
    value: &str,
    channels: u16,
    options: AudioImporterOptions,
) -> Result<(String, u16), ImportError> {
    let mut samples = parse_audio_source_samples(value, "i16", |part| {
        part.parse::<i16>().map_err(|error| error.to_string())
    })?;
    validate_audio_source_sample_count(samples.len(), channels)?;
    let output_channels = if options.force_mono && channels > 1 {
        samples = force_mono_i16_samples(&samples, channels);
        1
    } else {
        channels
    };
    if options.normalize {
        normalize_i16_samples(&mut samples);
    }
    Ok((
        samples
            .into_iter()
            .map(|sample| sample.to_string())
            .collect::<Vec<_>>()
            .join(","),
        output_channels,
    ))
}

#[cfg(feature = "audio_importer")]
fn canonical_audio_f32_samples(
    value: &str,
    channels: u16,
    options: AudioImporterOptions,
) -> Result<(String, u16), ImportError> {
    let mut samples = parse_audio_source_samples(value, "f32", |part| {
        let sample = part.parse::<f32>().map_err(|error| error.to_string())?;
        if !sample.is_finite() {
            return Err("sample must be finite".to_owned());
        }
        Ok(sample)
    })?;
    validate_audio_source_sample_count(samples.len(), channels)?;
    let output_channels = if options.force_mono && channels > 1 {
        samples = force_mono_f32_samples(&samples, channels);
        1
    } else {
        channels
    };
    if options.normalize {
        normalize_f32_samples(&mut samples);
    }
    Ok((
        samples
            .into_iter()
            .map(canonical_audio_f32)
            .collect::<Vec<_>>()
            .join(","),
        output_channels,
    ))
}

#[cfg(feature = "audio_importer")]
fn parse_audio_source_samples<T>(
    value: &str,
    sample_format: &str,
    parse: impl Fn(&str) -> Result<T, String>,
) -> Result<Vec<T>, ImportError> {
    value
        .replace(';', ",")
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            parse(part).map_err(|error| AssetError::Import {
                message: format!("invalid audio source {sample_format} sample: {error}"),
            })
        })
        .collect()
}

#[cfg(feature = "audio_importer")]
fn validate_audio_source_sample_count(
    sample_count: usize,
    channels: u16,
) -> Result<(), ImportError> {
    let channel_count = usize::from(channels);
    if sample_count == 0 || sample_count % channel_count != 0 {
        return Err(AssetError::Import {
            message: format!(
                "audio source sample count {sample_count} must be a non-zero multiple of channels {channel_count}"
            ),
        });
    }
    Ok(())
}

#[cfg(feature = "audio_importer")]
fn force_mono_i16_samples(samples: &[i16], channels: u16) -> Vec<i16> {
    let channel_count = usize::from(channels);
    let channel_count_i32 = i32::from(channels);
    samples
        .chunks_exact(channel_count)
        .map(|frame| {
            let sum = frame.iter().map(|sample| i32::from(*sample)).sum::<i32>();
            (sum / channel_count_i32) as i16
        })
        .collect()
}

#[cfg(feature = "audio_importer")]
fn force_mono_f32_samples(samples: &[f32], channels: u16) -> Vec<f32> {
    let channel_count = usize::from(channels);
    let scale = f32::from(channels);
    samples
        .chunks_exact(channel_count)
        .map(|frame| frame.iter().sum::<f32>() / scale)
        .collect()
}

#[cfg(feature = "audio_importer")]
fn normalize_i16_samples(samples: &mut [i16]) {
    let max_abs = samples
        .iter()
        .map(|sample| i32::from(*sample).abs())
        .max()
        .unwrap_or(0);
    if max_abs == 0 {
        return;
    }
    let scale = f32::from(i16::MAX) / max_abs as f32;
    for sample in samples {
        let scaled = f32::from(*sample) * scale;
        *sample = scaled
            .round()
            .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
    }
}

#[cfg(feature = "audio_importer")]
fn normalize_f32_samples(samples: &mut [f32]) {
    let max_abs = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    if max_abs == 0.0 {
        return;
    }
    for sample in samples {
        *sample /= max_abs;
    }
}

#[cfg(feature = "audio_importer")]
fn canonical_audio_f32(value: f32) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(feature = "material_importer")]
pub struct MaterialImporter;

#[cfg(feature = "material_importer")]
impl MaterialImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "material_importer")]
impl Default for MaterialImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "material_importer")]
impl AssetImporter for MaterialImporter {
    fn name(&self) -> &'static str {
        "MaterialImporter"
    }

    fn version(&self) -> u32 {
        3
    }

    fn extensions(&self) -> &[&'static str] {
        &["material", "mat"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let source_text =
            std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
                message: format!("material source must be UTF-8: {error}"),
            })?;
        crate::assets::material::validate_material_source(source_text).map_err(|error| {
            AssetError::Import {
                message: format!("invalid material source: {error}"),
            }
        })?;
        let canonical_source = canonical_material_source(source_text)?;
        let _ = collect_material_dependencies(ctx, source_text)?;

        let id = AssetId::new();
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), Material::TYPE_ID);
        metadata.source_path = Some(source.path.clone());
        metadata.cooked_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let generated = ImportGeneratedAsset {
            id,
            path: source.path.clone(),
            asset_type: Material::TYPE_ID,
            bytes: canonical_source.into_bytes(),
            labels: Vec::new(),
            dependencies: Vec::new(),
        };
        ctx.add_generated_asset(generated);
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "material_importer")]
fn canonical_material_source(source_text: &str) -> Result<String, ImportError> {
    let mut lines = Vec::new();
    for (line_index, line) in source_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid material line {}", line_index + 1),
            });
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() {
            return Err(AssetError::Import {
                message: format!("material key is empty on line {}", line_index + 1),
            });
        }
        lines.push(format!("{key}={value}"));
    }
    let mut canonical = lines.join("\n");
    if !canonical.is_empty() {
        canonical.push('\n');
    }
    Ok(canonical)
}

#[cfg(any(feature = "material_importer", feature = "model_importer"))]
fn collect_material_dependencies(
    ctx: &mut ImportContext,
    source_text: &str,
) -> Result<Vec<AssetId>, ImportError> {
    let mut dependencies = Vec::new();
    for (line_index, line) in source_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid material line {}", line_index + 1),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "shader" => {
                dependencies.push(add_material_dependency::<Shader>(
                    ctx, key, value, line_index,
                )?);
            }
            key if is_material_texture_metadata_key(key, line_index)? => {}
            key if key.starts_with("texture.") => {
                dependencies.push(add_material_dependency::<Texture>(
                    ctx, key, value, line_index,
                )?);
            }
            _ => {}
        }
    }
    Ok(dependencies)
}

#[cfg(any(feature = "material_importer", feature = "model_importer"))]
fn is_material_texture_metadata_key(key: &str, line_index: usize) -> Result<bool, ImportError> {
    if !key.starts_with("texture.") {
        return Ok(false);
    }
    let suffix = key.trim_start_matches("texture.");
    if let Some((name, field)) = suffix.split_once(".sampler.") {
        if name.is_empty() || field.is_empty() {
            return Err(AssetError::Import {
                message: format!(
                    "invalid material texture sampler key on line {}",
                    line_index + 1
                ),
            });
        }
        return Ok(true);
    }
    if let Some((name, field)) = suffix.split_once(".transform.") {
        if name.is_empty() || field.is_empty() {
            return Err(AssetError::Import {
                message: format!(
                    "invalid material texture transform key on line {}",
                    line_index + 1
                ),
            });
        }
        return Ok(true);
    }
    if let Some(name) = suffix.strip_suffix(".bump_scale") {
        if name.is_empty() {
            return Err(AssetError::Import {
                message: format!(
                    "invalid material texture bump scale key on line {}",
                    line_index + 1
                ),
            });
        }
        return Ok(true);
    }
    for option_suffix in [
        ".color_remap",
        ".source_channel",
        ".boost",
        ".blend_u",
        ".blend_v",
        ".color_correction",
        ".projection",
        ".texture_resolution",
    ] {
        if let Some(name) = suffix.strip_suffix(option_suffix) {
            if name.is_empty() {
                return Err(AssetError::Import {
                    message: format!(
                        "invalid material texture option key on line {}",
                        line_index + 1
                    ),
                });
            }
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(any(feature = "material_importer", feature = "model_importer"))]
fn add_material_dependency<T: Asset>(
    ctx: &mut ImportContext,
    key: &str,
    value: &str,
    line_index: usize,
) -> Result<AssetId, ImportError> {
    ctx.dependency::<T>(value)
        .map_err(|error| AssetError::Import {
            message: format!(
                "material dependency `{key}` on line {} references `{value}`: {error}",
                line_index + 1
            ),
        })
}

#[cfg(feature = "model_importer")]
pub struct ModelImporter;

#[cfg(feature = "model_importer")]
impl ModelImporter {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "model_importer")]
impl Default for ModelImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "model_importer")]
impl AssetImporter for ModelImporter {
    fn name(&self) -> &'static str {
        "ModelImporter"
    }

    fn version(&self) -> u32 {
        50
    }

    fn extensions(&self) -> &[&'static str] {
        &["model", "obj"]
    }

    fn import(
        &self,
        ctx: &mut ImportContext,
        source: &SourceAsset,
        settings: &ImporterSettings,
    ) -> Result<ImportOutput, ImportError> {
        let source_text =
            std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
                message: format!("model manifest must be UTF-8: {error}"),
            })?;
        let model_settings = ModelImportSettings::from_importer_settings(settings)?;
        let subresources = parse_model_source(
            ctx,
            source,
            source_text,
            source.path.extension().unwrap_or(""),
            &model_settings,
        )?;
        let subresources = apply_model_import_settings(subresources, &model_settings);
        let subresources = generate_model_lods(subresources, &model_settings)?;
        let mut generated_ids = HashMap::new();
        for subresource in &subresources {
            if generated_ids
                .insert(subresource.label.clone(), AssetId::new())
                .is_some()
            {
                return Err(AssetError::Import {
                    message: format!(
                        "model manifest repeats generated label `{}` on line {}",
                        subresource.label, subresource.line_number
                    ),
                });
            }
        }
        validate_model_generated_paths(source, &subresources)?;
        validate_model_physics_mesh_payloads(&subresources, &model_settings)?;
        validate_model_mesh_lod_bindings(&subresources)?;
        validate_model_mesh_material_bindings(&subresources)?;
        validate_model_mesh_physics_mesh_bindings(&subresources)?;
        validate_model_material_mesh_bindings(&subresources)?;
        validate_model_material_payloads(&subresources)?;
        validate_model_skin_bindings(&subresources)?;
        validate_model_animation_skeleton_targets(&subresources)?;
        validate_model_skeleton_payloads(&subresources)?;
        validate_model_animation_payloads(&subresources)?;

        for subresource in &subresources {
            let local_dependencies =
                model_local_dependencies(subresource, &subresources, &generated_ids)?;
            match subresource.kind.as_str() {
                "mesh" => ctx.add_generated_asset(model_generated_asset(
                    source,
                    "mesh",
                    &subresource.label,
                    Mesh::TYPE_ID,
                    model_mesh_payload_bytes(&subresource.payload, &model_settings)?,
                    local_dependencies,
                    generated_ids[&subresource.label],
                )),
                "physics_mesh" => ctx.add_generated_asset(model_generated_asset(
                    source,
                    "physics",
                    &subresource.label,
                    PhysicsMesh::TYPE_ID,
                    model_physics_mesh_payload_bytes(subresource, &model_settings)?,
                    local_dependencies,
                    generated_ids[&subresource.label],
                )),
                "material" => {
                    let material_source = model_payload_text(&subresource.payload);
                    let mut dependencies = collect_material_dependencies(ctx, &material_source)?;
                    dependencies.extend(local_dependencies);
                    ctx.add_generated_asset(model_generated_asset(
                        source,
                        "material",
                        &subresource.label,
                        Material::TYPE_ID,
                        material_source.into_bytes(),
                        dependencies,
                        generated_ids[&subresource.label],
                    ));
                }
                "skeleton" => ctx.add_generated_asset(model_generated_asset(
                    source,
                    "skeleton",
                    &subresource.label,
                    Skeleton::TYPE_ID,
                    model_payload_bytes(&subresource.payload),
                    local_dependencies,
                    generated_ids[&subresource.label],
                )),
                "animation" => ctx.add_generated_asset(model_generated_asset(
                    source,
                    "animation",
                    &subresource.label,
                    AnimationClip::TYPE_ID,
                    model_payload_bytes(&subresource.payload),
                    local_dependencies,
                    generated_ids[&subresource.label],
                )),
                _ => unreachable!("model subresource kind validated while parsing manifest"),
            }
        }

        let id = AssetId::new();
        let mut metadata = AssetMetadata::runtime(id, source.path.clone(), AssetTypeId::NIL);
        metadata.source_path = Some(source.path.clone());
        metadata.importer = Some(self.name().to_owned());
        metadata.importer_version = self.version();
        metadata.source_hash = Some(source.hash);
        metadata.settings_hash = Some(ContentHash(settings_hash(settings)));
        metadata.version_hash = Some(VersionHash(self.version() as u64));
        let (generated, dependencies) = std::mem::take(ctx).finish();
        Ok(ImportOutput {
            metadata,
            generated,
            dependencies,
            version_hash: VersionHash(self.version() as u64),
        })
    }
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelSubresource {
    kind: String,
    label: String,
    payload: String,
    dependency_labels: Vec<ModelDependencyLabel>,
    skin_skeleton_label: Option<String>,
    skin_joint_limit: Option<usize>,
    skin_influence_limit: Option<usize>,
    skin_root_bone: Option<String>,
    animation_skeleton_label: Option<String>,
    material_mesh_label: Option<String>,
    material_labels: Vec<String>,
    physics_mesh_labels: Vec<String>,
    lod_mesh_labels: Vec<String>,
    line_number: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelDependencyLabel {
    label: String,
    expected_kind: Option<&'static str>,
}

#[cfg(feature = "model_importer")]
impl ModelDependencyLabel {
    fn new(label: String, expected_kind: Option<&'static str>) -> Self {
        Self {
            label,
            expected_kind,
        }
    }
}

#[cfg(feature = "model_importer")]
fn parse_model_source(
    ctx: &ImportContext,
    source: &SourceAsset,
    source_text: &str,
    extension: &str,
    settings: &ModelImportSettings,
) -> Result<Vec<ModelSubresource>, ImportError> {
    let first_line = source_text.lines().next().unwrap_or("").trim();
    if first_line == "NGA_MODEL_V1" {
        return parse_model_manifest(source_text);
    }
    if first_line == "NGA_MODEL_OBJ_V1" || extension.eq_ignore_ascii_case("obj") {
        return parse_model_obj_source(
            ctx,
            source,
            source_text,
            first_line == "NGA_MODEL_OBJ_V1",
            settings,
        );
    }
    Err(AssetError::Import {
        message: "model source must start with NGA_MODEL_V1 or be an OBJ source".to_owned(),
    })
}

#[cfg(feature = "model_importer")]
fn apply_model_import_settings(
    subresources: Vec<ModelSubresource>,
    settings: &ModelImportSettings,
) -> Vec<ModelSubresource> {
    let original_labels = subresources
        .iter()
        .map(|subresource| subresource.label.clone())
        .collect::<Vec<_>>();
    let mut filtered = subresources
        .into_iter()
        .filter(|subresource| settings.imports_kind(&subresource.kind))
        .collect::<Vec<_>>();
    let kept_labels = filtered
        .iter()
        .map(|subresource| subresource.label.clone())
        .collect::<Vec<_>>();
    for subresource in &mut filtered {
        subresource.dependency_labels.retain(|dependency| {
            !original_labels.contains(&dependency.label) || kept_labels.contains(&dependency.label)
        });
        subresource
            .material_labels
            .retain(|label| !original_labels.contains(label) || kept_labels.contains(label));
        subresource
            .physics_mesh_labels
            .retain(|label| !original_labels.contains(label) || kept_labels.contains(label));
        subresource
            .lod_mesh_labels
            .retain(|label| !original_labels.contains(label) || kept_labels.contains(label));
        if subresource
            .skin_skeleton_label
            .as_ref()
            .is_some_and(|label| original_labels.contains(label) && !kept_labels.contains(label))
        {
            subresource.skin_skeleton_label = None;
            subresource.skin_joint_limit = None;
            subresource.skin_influence_limit = None;
            subresource.skin_root_bone = None;
        }
        if subresource
            .animation_skeleton_label
            .as_ref()
            .is_some_and(|label| original_labels.contains(label) && !kept_labels.contains(label))
        {
            subresource.animation_skeleton_label = None;
        }
        if subresource
            .material_mesh_label
            .as_ref()
            .is_some_and(|label| original_labels.contains(label) && !kept_labels.contains(label))
        {
            subresource.material_mesh_label = None;
        }
    }
    filtered
}

#[cfg(feature = "model_importer")]
fn generate_model_lods(
    subresources: Vec<ModelSubresource>,
    settings: &ModelImportSettings,
) -> Result<Vec<ModelSubresource>, ImportError> {
    if !settings.generate_lods {
        return Ok(subresources);
    }

    let mut output = Vec::new();
    for subresource in subresources {
        let lod = if subresource.kind == "mesh" {
            model_lod_subresource(&subresource)?
        } else {
            None
        };
        output.push(subresource);
        if let Some(lod) = lod {
            output.push(lod);
        }
    }
    Ok(output)
}

#[cfg(feature = "model_importer")]
fn model_lod_subresource(mesh: &ModelSubresource) -> Result<Option<ModelSubresource>, ImportError> {
    let Some(payload) = model_lod_mesh_payload_text(mesh)? else {
        return Ok(None);
    };
    Ok(Some(ModelSubresource {
        kind: mesh.kind.clone(),
        label: format!("{}.LOD1", mesh.label),
        payload,
        dependency_labels: mesh.dependency_labels.clone(),
        skin_skeleton_label: mesh.skin_skeleton_label.clone(),
        skin_joint_limit: mesh.skin_joint_limit,
        skin_influence_limit: mesh.skin_influence_limit,
        skin_root_bone: mesh.skin_root_bone.clone(),
        animation_skeleton_label: None,
        material_mesh_label: None,
        material_labels: mesh.material_labels.clone(),
        physics_mesh_labels: mesh.physics_mesh_labels.clone(),
        lod_mesh_labels: mesh.lod_mesh_labels.clone(),
        line_number: mesh.line_number,
    }))
}

#[cfg(feature = "model_importer")]
fn parse_model_manifest(source_text: &str) -> Result<Vec<ModelSubresource>, ImportError> {
    let lines = source_text.lines().collect::<Vec<_>>();
    if lines.first().copied().unwrap_or("").trim() != "NGA_MODEL_V1" {
        return Err(AssetError::Import {
            message: "model manifest must start with NGA_MODEL_V1".to_owned(),
        });
    }

    let mut subresources = Vec::new();
    let mut index = 1;
    while index < lines.len() {
        let line_number = index + 1;
        let line = lines[index].trim();
        if line.is_empty() || line.starts_with('#') {
            index += 1;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid model manifest line {line_number}"),
            });
        };
        let key = key.trim();
        validate_model_subresource_kind(key, line_number)?;
        let value = value.trim();
        if value.contains('|') {
            let (label, payload) = parse_model_inline_subresource(value, key, line_number)?;
            subresources.push(ModelSubresource {
                kind: key.to_owned(),
                label: label.to_owned(),
                payload: payload.to_owned(),
                dependency_labels: Vec::new(),
                skin_skeleton_label: None,
                skin_joint_limit: None,
                skin_influence_limit: None,
                skin_root_bone: None,
                animation_skeleton_label: None,
                material_mesh_label: None,
                material_labels: Vec::new(),
                physics_mesh_labels: Vec::new(),
                lod_mesh_labels: Vec::new(),
                line_number,
            });
            index += 1;
            continue;
        }

        let label = parse_model_label(value, key, line_number)?;
        let (
            payload,
            dependency_labels,
            skin_skeleton_label,
            skin_joint_limit,
            skin_influence_limit,
            skin_root_bone,
            animation_skeleton_label,
            material_mesh_label,
            material_labels,
            physics_mesh_labels,
            lod_mesh_labels,
            next_index,
        ) = parse_model_block_payload(&lines, index + 1, key, &label, line_number)?;
        subresources.push(ModelSubresource {
            kind: key.to_owned(),
            label,
            payload,
            dependency_labels,
            skin_skeleton_label,
            skin_joint_limit,
            skin_influence_limit,
            skin_root_bone,
            animation_skeleton_label,
            material_mesh_label,
            material_labels,
            physics_mesh_labels,
            lod_mesh_labels,
            line_number,
        });
        index = next_index;
    }
    Ok(subresources)
}

#[cfg(feature = "model_importer")]
fn validate_model_subresource_kind(kind: &str, line_number: usize) -> Result<(), ImportError> {
    match kind {
        "mesh" | "physics_mesh" | "material" | "skeleton" | "animation" => Ok(()),
        other => Err(AssetError::Import {
            message: format!("unknown model manifest key `{other}` on line {line_number}"),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_model_inline_subresource<'a>(
    value: &'a str,
    key: &str,
    line_number: usize,
) -> Result<(&'a str, &'a str), ImportError> {
    let Some((label, payload)) = value.split_once('|') else {
        return Err(AssetError::Import {
            message: format!("model {key} on line {line_number} must use label|payload"),
        });
    };
    let label = label.trim();
    if label.is_empty() {
        return Err(AssetError::Import {
            message: format!("model {key} label is empty on line {line_number}"),
        });
    }
    if payload.contains('|') {
        return Err(AssetError::Import {
            message: format!(
                "model {key} on line {line_number} inline payload must use exactly one `|` separator"
            ),
        });
    }
    Ok((label, payload.trim()))
}

#[cfg(feature = "model_importer")]
fn parse_model_label(label: &str, key: &str, line_number: usize) -> Result<String, ImportError> {
    let label = label.trim();
    if label.is_empty() {
        return Err(AssetError::Import {
            message: format!("model {key} label is empty on line {line_number}"),
        });
    }
    Ok(label.to_owned())
}

#[cfg(feature = "model_importer")]
fn parse_model_block_payload(
    lines: &[&str],
    mut index: usize,
    key: &str,
    label: &str,
    line_number: usize,
) -> Result<
    (
        String,
        Vec<ModelDependencyLabel>,
        Option<String>,
        Option<usize>,
        Option<usize>,
        Option<String>,
        Option<String>,
        Option<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        usize,
    ),
    ImportError,
> {
    let mut dependency_labels = Vec::new();
    let mut skin_skeleton_label = None;
    let mut skin_joint_limit = None;
    let mut skin_influence_limit = None;
    let mut skin_root_bone = None;
    let mut animation_skeleton_label = None;
    let mut material_mesh_label = None;
    let mut material_labels = Vec::new();
    let mut physics_mesh_labels = Vec::new();
    let mut lod_mesh_labels = Vec::new();
    let mut payload_lines = Vec::new();
    let mut in_payload = false;
    while index < lines.len() {
        let current_line_number = index + 1;
        let raw_line = lines[index];
        let line = raw_line.trim();
        if line == "end" {
            let payload = payload_lines.join("\n");
            if payload.trim().is_empty() {
                return Err(AssetError::Import {
                    message: format!(
                        "model {key} `{label}` block on line {line_number} has empty payload"
                    ),
                });
            }
            return Ok((
                payload,
                dependency_labels,
                skin_skeleton_label,
                skin_joint_limit,
                skin_influence_limit,
                skin_root_bone,
                animation_skeleton_label,
                material_mesh_label,
                material_labels,
                physics_mesh_labels,
                lod_mesh_labels,
                index + 1,
            ));
        }
        if !in_payload {
            if line.is_empty() || line.starts_with('#') {
                index += 1;
                continue;
            }
            if line == "---" {
                in_payload = true;
                index += 1;
                continue;
            }
            if let Some((metadata_key, metadata_value)) = line.split_once('=') {
                match metadata_key.trim() {
                    "depends" | "dependency" => {
                        for dependency in parse_model_dependency_labels(
                            metadata_value.trim(),
                            current_line_number,
                        )? {
                            push_model_dependency_label(
                                &mut dependency_labels,
                                dependency,
                                key,
                                label,
                                line_number,
                                current_line_number,
                            )?;
                        }
                        index += 1;
                        continue;
                    }
                    "skin" | "skeleton" if key == "mesh" => {
                        let skeleton_label = parse_model_skin_skeleton_label(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        if skin_skeleton_label
                            .replace(skeleton_label.clone())
                            .is_some()
                        {
                            return Err(AssetError::Import {
                                message: format!(
                                    "model mesh `{label}` block on line {line_number} repeats skin skeleton metadata"
                                ),
                            });
                        }
                        push_model_dependency_label(
                            &mut dependency_labels,
                            ModelDependencyLabel::new(skeleton_label, Some("skeleton")),
                            key,
                            label,
                            line_number,
                            current_line_number,
                        )?;
                        index += 1;
                        continue;
                    }
                    "max_skin_joints" | "skin_joint_limit" if key == "mesh" => {
                        let limit = parse_model_skin_joint_limit(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        if skin_joint_limit.replace(limit).is_some() {
                            return Err(AssetError::Import {
                                message: format!(
                                    "model mesh `{label}` block on line {line_number} repeats skin joint limit metadata"
                                ),
                            });
                        }
                        index += 1;
                        continue;
                    }
                    "max_skin_influences" | "skin_influence_limit" if key == "mesh" => {
                        let limit = parse_model_skin_influence_limit(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        if skin_influence_limit.replace(limit).is_some() {
                            return Err(AssetError::Import {
                                message: format!(
                                    "model mesh `{label}` block on line {line_number} repeats skin influence limit metadata"
                                ),
                            });
                        }
                        index += 1;
                        continue;
                    }
                    "skin_root" | "root_bone" | "skin_root_bone" if key == "mesh" => {
                        let root_bone = parse_model_skin_root_bone_label(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        if skin_root_bone.replace(root_bone).is_some() {
                            return Err(AssetError::Import {
                                message: format!(
                                    "model mesh `{label}` block on line {line_number} repeats skin root bone metadata"
                                ),
                            });
                        }
                        index += 1;
                        continue;
                    }
                    "skeleton" | "target_skeleton" if key == "animation" => {
                        let skeleton_label = parse_model_animation_skeleton_label(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        if animation_skeleton_label
                            .replace(skeleton_label.clone())
                            .is_some()
                        {
                            return Err(AssetError::Import {
                                message: format!(
                                    "model animation `{label}` block on line {line_number} repeats target skeleton metadata"
                                ),
                            });
                        }
                        push_model_dependency_label(
                            &mut dependency_labels,
                            ModelDependencyLabel::new(skeleton_label, Some("skeleton")),
                            key,
                            label,
                            line_number,
                            current_line_number,
                        )?;
                        index += 1;
                        continue;
                    }
                    "material" | "materials" if key == "mesh" => {
                        let labels = parse_model_plain_dependency_labels(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        for material_label in labels {
                            push_model_dependency_label(
                                &mut dependency_labels,
                                ModelDependencyLabel::new(material_label.clone(), Some("material")),
                                key,
                                label,
                                line_number,
                                current_line_number,
                            )?;
                            material_labels.push(material_label);
                        }
                        index += 1;
                        continue;
                    }
                    "physics_mesh" | "physics_meshes" if key == "mesh" => {
                        let labels = parse_model_plain_dependency_labels(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        for physics_mesh_label in labels {
                            push_model_dependency_label(
                                &mut dependency_labels,
                                ModelDependencyLabel::new(
                                    physics_mesh_label.clone(),
                                    Some("physics_mesh"),
                                ),
                                key,
                                label,
                                line_number,
                                current_line_number,
                            )?;
                            physics_mesh_labels.push(physics_mesh_label);
                        }
                        index += 1;
                        continue;
                    }
                    "lod" | "lods" if key == "mesh" => {
                        let labels = parse_model_plain_dependency_labels(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        for lod_mesh_label in labels {
                            push_model_dependency_label(
                                &mut dependency_labels,
                                ModelDependencyLabel::new(lod_mesh_label.clone(), Some("mesh")),
                                key,
                                label,
                                line_number,
                                current_line_number,
                            )?;
                            lod_mesh_labels.push(lod_mesh_label);
                        }
                        index += 1;
                        continue;
                    }
                    "mesh" | "target_mesh" if key == "material" => {
                        let mesh_label = parse_model_material_mesh_label(
                            metadata_value.trim(),
                            current_line_number,
                        )?;
                        if material_mesh_label.replace(mesh_label.clone()).is_some() {
                            return Err(AssetError::Import {
                                message: format!(
                                    "model material `{label}` block on line {line_number} repeats target mesh metadata"
                                ),
                            });
                        }
                        push_model_dependency_label(
                            &mut dependency_labels,
                            ModelDependencyLabel::new(mesh_label, Some("mesh")),
                            key,
                            label,
                            line_number,
                            current_line_number,
                        )?;
                        index += 1;
                        continue;
                    }
                    _ => {}
                }
            }
            in_payload = true;
        }
        if in_payload && !(line.is_empty() || line.starts_with('#')) {
            payload_lines.push(line.to_owned());
        }
        index += 1;
    }
    Err(AssetError::Import {
        message: format!("model {key} `{label}` block on line {line_number} is missing end"),
    })
}

#[cfg(feature = "model_importer")]
fn parse_model_dependency_labels(
    value: &str,
    line_number: usize,
) -> Result<Vec<ModelDependencyLabel>, ImportError> {
    parse_model_plain_dependency_labels(value, line_number)?
        .into_iter()
        .map(|label| parse_model_dependency_label(label, line_number))
        .collect()
}

#[cfg(feature = "model_importer")]
fn parse_model_plain_dependency_labels(
    value: &str,
    line_number: usize,
) -> Result<Vec<String>, ImportError> {
    if value.trim().is_empty() {
        return Err(AssetError::Import {
            message: format!("model dependency list is empty on line {line_number}"),
        });
    }

    let mut labels = Vec::new();
    for raw_label in value.split(',') {
        let label = raw_label.trim();
        if label.is_empty() {
            return Err(AssetError::Import {
                message: format!(
                    "model dependency list on line {line_number} contains an empty generated label"
                ),
            });
        }
        labels.push(label.to_owned());
    }
    Ok(labels)
}

#[cfg(feature = "model_importer")]
fn parse_model_dependency_label(
    value: String,
    line_number: usize,
) -> Result<ModelDependencyLabel, ImportError> {
    if let Some((kind, label)) = value.split_once(':') {
        let kind = kind.trim();
        let expected_kind = match kind {
            "mesh" => Some("mesh"),
            "material" => Some("material"),
            "skeleton" => Some("skeleton"),
            "animation" => Some("animation"),
            "physics_mesh" => Some("physics_mesh"),
            _ => None,
        };
        if let Some(expected_kind) = expected_kind {
            let label = label.trim();
            if label.is_empty() {
                return Err(AssetError::Import {
                    message: format!(
                        "model dependency `{kind}:` on line {line_number} must name a generated {expected_kind} label"
                    ),
                });
            }
            return Ok(ModelDependencyLabel::new(
                label.to_owned(),
                Some(expected_kind),
            ));
        }
        return Err(AssetError::Import {
            message: format!(
                "unknown model generated dependency kind `{kind}` on line {line_number}; expected mesh, material, skeleton, animation, or physics_mesh"
            ),
        });
    }
    Ok(ModelDependencyLabel::new(value, None))
}

#[cfg(feature = "model_importer")]
fn push_model_dependency_label(
    dependency_labels: &mut Vec<ModelDependencyLabel>,
    dependency: ModelDependencyLabel,
    owner_kind: &str,
    owner_label: &str,
    owner_line_number: usize,
    line_number: usize,
) -> Result<(), ImportError> {
    if dependency_labels
        .iter()
        .any(|existing| existing.label == dependency.label)
    {
        return Err(AssetError::Import {
            message: format!(
                "model {owner_kind} `{owner_label}` block on line {owner_line_number} repeats generated dependency `{}` on line {line_number}",
                dependency.label
            ),
        });
    }
    dependency_labels.push(dependency);
    Ok(())
}

#[cfg(feature = "model_importer")]
fn parse_model_skin_skeleton_label(value: &str, line_number: usize) -> Result<String, ImportError> {
    let labels = parse_model_plain_dependency_labels(value, line_number)?;
    if labels.len() != 1 {
        return Err(AssetError::Import {
            message: format!(
                "model mesh skin skeleton on line {line_number} must name exactly one generated skeleton label"
            ),
        });
    }
    Ok(labels.into_iter().next().unwrap())
}

#[cfg(feature = "model_importer")]
fn parse_model_skin_joint_limit(value: &str, line_number: usize) -> Result<usize, ImportError> {
    let limit = value.parse::<usize>().map_err(|error| AssetError::Import {
        message: format!("invalid model mesh skin joint limit on line {line_number}: {error}"),
    })?;
    if limit == 0 {
        return Err(AssetError::Import {
            message: format!(
                "model mesh skin joint limit on line {line_number} must be greater than zero"
            ),
        });
    }
    Ok(limit)
}

#[cfg(feature = "model_importer")]
fn parse_model_skin_influence_limit(value: &str, line_number: usize) -> Result<usize, ImportError> {
    let limit = value.parse::<usize>().map_err(|error| AssetError::Import {
        message: format!("invalid model mesh skin influence limit on line {line_number}: {error}"),
    })?;
    if limit == 0 {
        return Err(AssetError::Import {
            message: format!(
                "model mesh skin influence limit on line {line_number} must be greater than zero"
            ),
        });
    }
    if limit > 4 {
        return Err(AssetError::Import {
            message: format!(
                "model mesh skin influence limit on line {line_number} must not exceed 4"
            ),
        });
    }
    Ok(limit)
}

#[cfg(feature = "model_importer")]
fn parse_model_skin_root_bone_label(
    value: &str,
    line_number: usize,
) -> Result<String, ImportError> {
    let labels = parse_model_plain_dependency_labels(value, line_number)?;
    if labels.len() != 1 {
        return Err(AssetError::Import {
            message: format!(
                "model mesh skin root bone on line {line_number} must name exactly one skeleton bone"
            ),
        });
    }
    Ok(labels.into_iter().next().unwrap())
}

#[cfg(feature = "model_importer")]
fn parse_model_animation_skeleton_label(
    value: &str,
    line_number: usize,
) -> Result<String, ImportError> {
    let labels = parse_model_plain_dependency_labels(value, line_number)?;
    if labels.len() != 1 {
        return Err(AssetError::Import {
            message: format!(
                "model animation target skeleton on line {line_number} must name exactly one generated skeleton label"
            ),
        });
    }
    Ok(labels.into_iter().next().unwrap())
}

#[cfg(feature = "model_importer")]
fn parse_model_material_mesh_label(value: &str, line_number: usize) -> Result<String, ImportError> {
    let labels = parse_model_plain_dependency_labels(value, line_number)?;
    if labels.len() != 1 {
        return Err(AssetError::Import {
            message: format!(
                "model material target mesh on line {line_number} must name exactly one generated mesh label"
            ),
        });
    }
    Ok(labels.into_iter().next().unwrap())
}

#[cfg(feature = "model_importer")]
fn validate_model_mesh_lod_bindings(subresources: &[ModelSubresource]) -> Result<(), ImportError> {
    for mesh in subresources
        .iter()
        .filter(|subresource| subresource.kind == "mesh")
    {
        for lod_mesh_label in &mesh.lod_mesh_labels {
            let Some(lod_mesh) = subresources
                .iter()
                .find(|subresource| subresource.label == *lod_mesh_label)
            else {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} references unknown LOD mesh `{lod_mesh_label}`",
                        mesh.label, mesh.line_number
                    ),
                });
            };
            if lod_mesh.kind != "mesh" {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} LOD binding `{lod_mesh_label}` references generated {} `{}` instead of a mesh",
                        mesh.label,
                        mesh.line_number,
                        lod_mesh.kind,
                        lod_mesh.label
                    ),
                });
            }
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_mesh_material_bindings(
    subresources: &[ModelSubresource],
) -> Result<(), ImportError> {
    for mesh in subresources
        .iter()
        .filter(|subresource| subresource.kind == "mesh")
    {
        for material_label in &mesh.material_labels {
            let Some(material) = subresources
                .iter()
                .find(|subresource| subresource.label == *material_label)
            else {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} references unknown material `{material_label}`",
                        mesh.label, mesh.line_number
                    ),
                });
            };
            if material.kind != "material" {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} material binding `{material_label}` references generated {} `{}` instead of a material",
                        mesh.label,
                        mesh.line_number,
                        material.kind,
                        material.label
                    ),
                });
            }
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_mesh_physics_mesh_bindings(
    subresources: &[ModelSubresource],
) -> Result<(), ImportError> {
    for mesh in subresources
        .iter()
        .filter(|subresource| subresource.kind == "mesh")
    {
        for physics_mesh_label in &mesh.physics_mesh_labels {
            let Some(physics_mesh) = subresources
                .iter()
                .find(|subresource| subresource.label == *physics_mesh_label)
            else {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} references unknown physics mesh `{physics_mesh_label}`",
                        mesh.label, mesh.line_number
                    ),
                });
            };
            if physics_mesh.kind != "physics_mesh" {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} physics mesh binding `{physics_mesh_label}` references generated {} `{}` instead of a physics_mesh",
                        mesh.label,
                        mesh.line_number,
                        physics_mesh.kind,
                        physics_mesh.label
                    ),
                });
            }
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_material_mesh_bindings(
    subresources: &[ModelSubresource],
) -> Result<(), ImportError> {
    for material in subresources
        .iter()
        .filter(|subresource| subresource.kind == "material")
    {
        let Some(mesh_label) = &material.material_mesh_label else {
            continue;
        };
        let Some(target) = subresources
            .iter()
            .find(|subresource| subresource.label == *mesh_label)
        else {
            return Err(AssetError::Import {
                message: format!(
                    "model material `{}` on line {} references unknown target mesh `{mesh_label}`",
                    material.label, material.line_number
                ),
            });
        };
        if target.kind != "mesh" {
            return Err(AssetError::Import {
                message: format!(
                    "model material `{}` on line {} target mesh `{mesh_label}` references generated {} `{}` instead of a mesh",
                    material.label, material.line_number, target.kind, target.label
                ),
            });
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_physics_mesh_payloads(
    subresources: &[ModelSubresource],
    settings: &ModelImportSettings,
) -> Result<(), ImportError> {
    for physics_mesh in subresources
        .iter()
        .filter(|subresource| subresource.kind == "physics_mesh")
    {
        model_physics_mesh_payload_bytes(physics_mesh, settings)?;
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_material_payloads(subresources: &[ModelSubresource]) -> Result<(), ImportError> {
    for material in subresources
        .iter()
        .filter(|subresource| subresource.kind == "material")
    {
        crate::assets::material::validate_material_source(&model_payload_text(&material.payload))
            .map_err(|error| AssetError::Import {
            message: format!(
                "model material `{}` on line {} payload is invalid: {error}",
                material.label, material.line_number
            ),
        })?;
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_skeleton_payloads(subresources: &[ModelSubresource]) -> Result<(), ImportError> {
    for skeleton in subresources
        .iter()
        .filter(|subresource| subresource.kind == "skeleton")
    {
        crate::assets::skeleton::parse_skeleton(&model_payload_bytes(&skeleton.payload)).map_err(
            |error| AssetError::Import {
                message: format!(
                    "model skeleton `{}` on line {} payload is invalid: {error}",
                    skeleton.label, skeleton.line_number
                ),
            },
        )?;
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_animation_payloads(subresources: &[ModelSubresource]) -> Result<(), ImportError> {
    for animation in subresources
        .iter()
        .filter(|subresource| subresource.kind == "animation")
    {
        crate::assets::animation::parse_animation_clip(&model_payload_bytes(&animation.payload))
            .map_err(|error| AssetError::Import {
                message: format!(
                    "model animation `{}` on line {} payload is invalid: {error}",
                    animation.label, animation.line_number
                ),
            })?;
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_skin_bindings(subresources: &[ModelSubresource]) -> Result<(), ImportError> {
    for mesh in subresources
        .iter()
        .filter(|subresource| subresource.kind == "mesh")
    {
        let Some(skeleton_label) = &mesh.skin_skeleton_label else {
            if mesh.skin_joint_limit.is_some() {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} declares skin joint limit metadata without skin skeleton metadata",
                        mesh.label, mesh.line_number
                    ),
                });
            }
            if mesh.skin_influence_limit.is_some() {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} declares skin influence limit metadata without skin skeleton metadata",
                        mesh.label, mesh.line_number
                    ),
                });
            }
            if mesh.skin_root_bone.is_some() {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} declares skin root bone metadata without skin skeleton metadata",
                        mesh.label, mesh.line_number
                    ),
                });
            }
            continue;
        };
        let skeleton = subresources
            .iter()
            .find(|subresource| {
                subresource.kind == "skeleton" && subresource.label == *skeleton_label
            })
            .ok_or_else(|| AssetError::Import {
                message: format!(
                    "model mesh `{}` on line {} references unknown skin skeleton `{skeleton_label}`",
                    mesh.label, mesh.line_number
                ),
            })?;
        let skeleton_asset =
            crate::assets::skeleton::parse_skeleton(&model_payload_bytes(&skeleton.payload))
                .map_err(|error| AssetError::Import {
                    message: format!(
                "model mesh `{}` on line {} skin skeleton `{skeleton_label}` is invalid: {error}",
                mesh.label, mesh.line_number
            ),
                })?;
        let mesh_asset =
            crate::assets::mesh::decode_mesh_for_model_import(&model_payload_bytes(&mesh.payload))
                .map_err(|error| AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} skin payload is invalid: {error}",
                        mesh.label, mesh.line_number
                    ),
                })?;
        if mesh_asset.joints.is_empty() || mesh_asset.weights.is_empty() {
            return Err(AssetError::Import {
                message: format!(
                    "model mesh `{}` on line {} declares skin skeleton `{skeleton_label}` but has no skin joint/weight attributes",
                    mesh.label, mesh.line_number
                ),
            });
        }
        if let Some(limit) = mesh.skin_joint_limit {
            if skeleton_asset.bones.len() > limit {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} skin skeleton `{skeleton_label}` has {} bones which exceeds declared skin joint limit {limit}",
                        mesh.label,
                        mesh.line_number,
                        skeleton_asset.bones.len()
                    ),
                });
            }
        }
        validate_model_skin_weights(mesh, skeleton_label, &mesh_asset.weights)?;
        if let Some(limit) = mesh.skin_influence_limit {
            validate_model_skin_influence_limit(mesh, skeleton_label, &mesh_asset.weights, limit)?;
        }
        let bone_count = skeleton_asset.bones.len();
        for (vertex_index, joints) in mesh_asset.joints.iter().enumerate() {
            for joint in joints {
                if *joint as usize >= bone_count {
                    return Err(AssetError::Import {
                        message: format!(
                            "model mesh `{}` on line {} skin joint {joint} at vertex {vertex_index} references missing skeleton bone; skeleton `{skeleton_label}` has {bone_count} bones",
                            mesh.label, mesh.line_number
                        ),
                    });
                }
            }
        }
        validate_model_unique_active_skin_joints(
            mesh,
            skeleton_label,
            &mesh_asset.joints,
            &mesh_asset.weights,
        )?;
        validate_model_skin_skeleton_root_scope(mesh, skeleton_label, &skeleton_asset)?;
        validate_model_skin_root_bone(mesh, skeleton_label, &skeleton_asset, &mesh_asset)?;
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_skin_weights(
    mesh: &ModelSubresource,
    skeleton_label: &str,
    weights: &[[f32; 4]],
) -> Result<(), ImportError> {
    for (vertex_index, weights) in weights.iter().enumerate() {
        let total = weights.iter().sum::<f32>();
        if total <= f32::EPSILON {
            return Err(AssetError::Import {
                message: format!(
                    "model mesh `{}` on line {} skin weights at vertex {vertex_index} for skeleton `{skeleton_label}` must have a positive total",
                    mesh.label, mesh.line_number
                ),
            });
        }
        if (total - 1.0).abs() > SKIN_WEIGHT_SUM_EPSILON {
            return Err(AssetError::Import {
                message: format!(
                    "model mesh `{}` on line {} skin weights at vertex {vertex_index} for skeleton `{skeleton_label}` must sum to 1.0, found {total}",
                    mesh.label, mesh.line_number
                ),
            });
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_skin_influence_limit(
    mesh: &ModelSubresource,
    skeleton_label: &str,
    weights: &[[f32; 4]],
    limit: usize,
) -> Result<(), ImportError> {
    for (vertex_index, weights) in weights.iter().enumerate() {
        let active_influences = weights
            .iter()
            .filter(|weight| **weight > SKIN_WEIGHT_SUM_EPSILON)
            .count();
        if active_influences > limit {
            return Err(AssetError::Import {
                message: format!(
                    "model mesh `{}` on line {} skin weights at vertex {vertex_index} for skeleton `{skeleton_label}` use {active_influences} influences which exceeds declared skin influence limit {limit}",
                    mesh.label, mesh.line_number
                ),
            });
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_unique_active_skin_joints(
    mesh: &ModelSubresource,
    skeleton_label: &str,
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
) -> Result<(), ImportError> {
    for (vertex_index, (joints, weights)) in joints.iter().zip(weights.iter()).enumerate() {
        let mut seen = Vec::new();
        for (slot, joint) in joints.iter().enumerate() {
            if weights[slot] <= SKIN_WEIGHT_SUM_EPSILON {
                continue;
            }
            if seen.contains(joint) {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} skin joint {joint} appears more than once with active weights at vertex {vertex_index} for skeleton `{skeleton_label}`",
                        mesh.label, mesh.line_number
                    ),
                });
            }
            seen.push(*joint);
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_skin_skeleton_root_scope(
    mesh: &ModelSubresource,
    skeleton_label: &str,
    skeleton: &Skeleton,
) -> Result<(), ImportError> {
    if mesh.skin_root_bone.is_some() {
        return Ok(());
    }
    let root_bones = skeleton
        .bones
        .iter()
        .filter(|bone| bone.parent.is_none())
        .map(|bone| bone.name.as_str())
        .collect::<Vec<_>>();
    if root_bones.len() > 1 {
        return Err(AssetError::Import {
            message: format!(
                "model mesh `{}` on line {} skin skeleton `{skeleton_label}` has multiple root bones ({}); declare skin_root, root_bone, or skin_root_bone metadata to scope skinning",
                mesh.label,
                mesh.line_number,
                root_bones.join(", ")
            ),
        });
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_model_skin_root_bone(
    mesh: &ModelSubresource,
    skeleton_label: &str,
    skeleton: &Skeleton,
    mesh_asset: &Mesh,
) -> Result<(), ImportError> {
    let Some(root_bone) = &mesh.skin_root_bone else {
        return Ok(());
    };
    let root_index = skeleton
        .bones
        .iter()
        .position(|bone| bone.name == *root_bone)
        .ok_or_else(|| AssetError::Import {
            message: format!(
                "model mesh `{}` on line {} skin root bone `{root_bone}` is missing from skeleton `{skeleton_label}`",
                mesh.label, mesh.line_number
            ),
        })?;
    for (vertex_index, (joints, weights)) in mesh_asset
        .joints
        .iter()
        .zip(mesh_asset.weights.iter())
        .enumerate()
    {
        for (slot, joint) in joints.iter().enumerate() {
            if weights[slot] <= SKIN_WEIGHT_SUM_EPSILON {
                continue;
            }
            let joint_index = *joint as usize;
            if !model_skeleton_bone_is_in_subtree(skeleton, joint_index, root_index) {
                return Err(AssetError::Import {
                    message: format!(
                        "model mesh `{}` on line {} skin joint {joint} at vertex {vertex_index} targets bone `{}` outside skin root `{root_bone}` in skeleton `{skeleton_label}`",
                        mesh.label,
                        mesh.line_number,
                        skeleton.bones[joint_index].name
                    ),
                });
            }
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn model_skeleton_bone_is_in_subtree(
    skeleton: &Skeleton,
    mut bone_index: usize,
    root_index: usize,
) -> bool {
    loop {
        if bone_index == root_index {
            return true;
        }
        let Some(parent_index) = skeleton.bones[bone_index].parent else {
            return false;
        };
        bone_index = parent_index as usize;
    }
}

#[cfg(feature = "model_importer")]
fn validate_model_animation_skeleton_targets(
    subresources: &[ModelSubresource],
) -> Result<(), ImportError> {
    for animation in subresources
        .iter()
        .filter(|subresource| subresource.kind == "animation")
    {
        if let Some(skeleton_label) = &animation.animation_skeleton_label {
            let Some(target) = subresources
                .iter()
                .find(|subresource| subresource.label == *skeleton_label)
            else {
                return Err(AssetError::Import {
                    message: format!(
                        "model animation `{}` on line {} references unknown target skeleton `{skeleton_label}`",
                        animation.label, animation.line_number
                    ),
                });
            };
            if target.kind != "skeleton" {
                return Err(AssetError::Import {
                    message: format!(
                        "model animation `{}` on line {} target skeleton `{skeleton_label}` references generated {} `{}` instead of a skeleton",
                        animation.label,
                        animation.line_number,
                        target.kind,
                        target.label
                    ),
                });
            }
        }
        let skeletons = animation
            .dependency_labels
            .iter()
            .filter_map(|dependency| {
                subresources
                    .iter()
                    .find(|subresource| {
                        subresource.kind == "skeleton" && subresource.label == dependency.label
                    })
                    .map(|skeleton| (&dependency.label, skeleton))
            })
            .collect::<Vec<_>>();
        if skeletons.is_empty() {
            continue;
        }

        let animation_clip = crate::assets::animation::parse_animation_clip(&model_payload_bytes(
            &animation.payload,
        ))
        .map_err(|error| AssetError::Import {
            message: format!(
                "model animation `{}` on line {} payload is invalid: {error}",
                animation.label, animation.line_number
            ),
        })?;
        for (skeleton_label, skeleton) in skeletons {
            let skeleton_asset =
                crate::assets::skeleton::parse_skeleton(&model_payload_bytes(&skeleton.payload))
                    .map_err(|error| AssetError::Import {
                        message: format!(
                            "model animation `{}` on line {} dependency skeleton `{skeleton_label}` is invalid: {error}",
                            animation.label, animation.line_number
                        ),
                    })?;
            validate_animation_clip_targets_skeleton(
                animation,
                &animation_clip,
                skeleton_label,
                &skeleton_asset,
            )?;
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn validate_animation_clip_targets_skeleton(
    animation: &ModelSubresource,
    animation_clip: &AnimationClip,
    skeleton_label: &str,
    skeleton: &Skeleton,
) -> Result<(), ImportError> {
    for (track_index, track) in animation_clip.tracks.iter().enumerate() {
        match &track.target {
            AnimationTarget::BoneName(name) => {
                if !skeleton.bones.iter().any(|bone| bone.name == *name) {
                    return Err(AssetError::Import {
                        message: format!(
                            "model animation `{}` on line {} track {track_index} targets missing skeleton bone `{name}` in skeleton `{skeleton_label}`",
                            animation.label, animation.line_number
                        ),
                    });
                }
            }
            AnimationTarget::NodeIndex(index) => {
                if *index as usize >= skeleton.bones.len() {
                    return Err(AssetError::Import {
                        message: format!(
                            "model animation `{}` on line {} track {track_index} node_index {index} references missing skeleton bone; skeleton `{skeleton_label}` has {} bones",
                            animation.label,
                            animation.line_number,
                            skeleton.bones.len()
                        ),
                    });
                }
            }
            AnimationTarget::NodeName(name) => {
                if !skeleton.bones.iter().any(|bone| bone.name == *name) {
                    return Err(AssetError::Import {
                        message: format!(
                            "model animation `{}` on line {} track {track_index} targets missing skeleton node `{name}` in skeleton `{skeleton_label}`",
                            animation.label, animation.line_number
                        ),
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
struct ObjMeshSource {
    label: String,
    material_groups: Vec<ObjMeshMaterialGroup>,
    line_number: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
struct ObjMeshMaterialGroup {
    material_label: Option<String>,
    triangles: Vec<ObjMeshTriangle>,
    line_number: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObjVertexRef {
    vertex: usize,
    texture_coord: Option<usize>,
    normal: Option<usize>,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjMeshTriangle {
    vertices: [ObjVertexRef; 3],
    smoothing_group: ObjSmoothingGroup,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
enum ObjSmoothingGroup {
    Unspecified,
    Off,
    Group(String),
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjExpandedVertex {
    vertex_ref: ObjVertexRef,
    generated_normal: Option<ObjGeneratedNormalKey>,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
enum ObjGeneratedNormalKey {
    Flat(usize),
    Smooth(String),
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjMaterialLibraryRef {
    name: String,
    line_number: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjMaterialUse {
    name: String,
    line_number: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, Default, PartialEq)]
struct ObjMaterialProperties {
    base_color: Option<[f32; 4]>,
    textures: Vec<ObjMaterialTexture>,
    metallic: Option<f32>,
    roughness: Option<f32>,
    emissive: Option<[f32; 3]>,
    ambient_color: Option<[f32; 3]>,
    specular_color: Option<[f32; 3]>,
    transmission_filter: Option<[f32; 3]>,
    index_of_refraction: Option<f32>,
    illumination_model: Option<i32>,
    sharpness: Option<f32>,
    texture_antialias: Option<bool>,
    sheen: Option<f32>,
    clearcoat: Option<f32>,
    clearcoat_roughness: Option<f32>,
    anisotropy: Option<f32>,
    anisotropy_rotation: Option<f32>,
    dissolve_halo: bool,
    alpha_blend: bool,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
struct ObjMaterialDefinition {
    name: String,
    properties: ObjMaterialProperties,
    line_number: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, Default, PartialEq)]
struct ObjMaterialLibraryProperties {
    materials: HashMap<String, ObjMaterialProperties>,
    loaded_library_count: usize,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
struct ObjMaterialTexture {
    channel: String,
    path: String,
    options: ObjMaterialTextureOptions,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, Default, PartialEq)]
struct ObjMaterialTextureOptions {
    sampler_address: Option<String>,
    transform_offset: Option<[f32; 3]>,
    transform_scale: Option<[f32; 3]>,
    transform_turbulence: Option<[f32; 3]>,
    bump_scale: Option<f32>,
    color_remap: Option<[f32; 2]>,
    source_channel: Option<String>,
    boost: Option<f32>,
    blend_u: Option<bool>,
    blend_v: Option<bool>,
    color_correction: Option<bool>,
    projection: Option<String>,
    texture_resolution: Option<u32>,
}

#[cfg(feature = "model_importer")]
fn parse_model_obj_source(
    ctx: &ImportContext,
    source: &SourceAsset,
    source_text: &str,
    has_header: bool,
    settings: &ModelImportSettings,
) -> Result<Vec<ModelSubresource>, ImportError> {
    let mut vertices = Vec::new();
    let mut texture_coords = Vec::new();
    let mut normals = Vec::new();
    let mut meshes = Vec::new();
    let mut material_uses = Vec::<ObjMaterialUse>::new();
    let mut material_libraries = Vec::new();
    let mut current = ObjMeshSource {
        label: "Mesh0".to_owned(),
        material_groups: Vec::new(),
        line_number: if has_header { 2 } else { 1 },
    };
    let mut current_material_label = None;
    let mut current_smoothing_group = ObjSmoothingGroup::Unspecified;

    for (line_index, line) in source_text.lines().enumerate() {
        let line_number = line_index + 1;
        if has_header && line_number == 1 {
            continue;
        }
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let directive = parts.next().unwrap_or("");
        match directive {
            "o" | "g" => {
                let label = line[directive.len()..].trim();
                if label.is_empty() {
                    return Err(AssetError::Import {
                        message: format!("OBJ {directive} label is empty on line {line_number}"),
                    });
                }
                if obj_mesh_has_triangles(&current) {
                    meshes.push(current);
                }
                current = ObjMeshSource {
                    label: label.to_owned(),
                    material_groups: Vec::new(),
                    line_number,
                };
            }
            "v" => vertices.push(parse_obj_vertex(parts, line_number)?),
            "vt" => texture_coords.push(parse_obj_texture_coord(parts, line_number)?),
            "vn" => normals.push(parse_obj_normal(parts, line_number)?),
            "f" => {
                let triangles = parse_obj_face(
                    parts,
                    vertices.len(),
                    texture_coords.len(),
                    normals.len(),
                    line_number,
                )?;
                push_obj_mesh_triangles(
                    &mut current,
                    current_material_label.as_ref(),
                    triangles,
                    &current_smoothing_group,
                    line_number,
                );
            }
            "usemtl" => {
                let name = line["usemtl".len()..].trim();
                if name.is_empty() {
                    return Err(AssetError::Import {
                        message: format!("OBJ usemtl name is empty on line {line_number}"),
                    });
                }
                let label = format!("Material/{name}");
                if !material_uses.iter().any(|existing| existing.name == name) {
                    material_uses.push(ObjMaterialUse {
                        name: name.to_owned(),
                        line_number,
                    });
                }
                current_material_label = Some(label);
            }
            "mtllib" => parse_obj_material_libraries(
                line["mtllib".len()..].trim(),
                line_number,
                &mut material_libraries,
            )?,
            "s" => {
                current_smoothing_group = parse_obj_smoothing_group(parts, line_number)?;
            }
            other => {
                return Err(AssetError::Import {
                    message: format!("unknown OBJ directive `{other}` on line {line_number}"),
                })
            }
        }
    }
    if obj_mesh_has_triangles(&current) {
        meshes.push(current);
    }
    if vertices.is_empty() {
        return Err(AssetError::Import {
            message: "OBJ model must contain at least one vertex".to_owned(),
        });
    }
    if meshes.is_empty() {
        return Err(AssetError::Import {
            message: "OBJ model must contain at least one face".to_owned(),
        });
    }

    let material_properties =
        obj_material_properties_from_libraries(ctx, &source.path, &material_libraries)?;
    validate_obj_material_uses(&material_uses, &material_libraries, &material_properties)?;
    let mut subresources = Vec::new();
    for mesh in meshes {
        let split_material_groups = mesh.material_groups.len() > 1;
        for (group_index, group) in mesh.material_groups.into_iter().enumerate() {
            let label = if split_material_groups {
                obj_material_group_mesh_label(
                    &mesh.label,
                    group.material_label.as_deref(),
                    group_index,
                )
            } else {
                mesh.label.clone()
            };
            let line_number = if split_material_groups {
                group.line_number
            } else {
                mesh.line_number
            };
            let payload = obj_mesh_payload(
                &vertices,
                &texture_coords,
                &normals,
                &group.triangles,
                &label,
                line_number,
                settings.generate_tangents,
            )?;
            subresources.push(ModelSubresource {
                kind: "mesh".to_owned(),
                label,
                payload,
                dependency_labels: group
                    .material_label
                    .into_iter()
                    .map(|label| ModelDependencyLabel::new(label, Some("material")))
                    .collect(),
                skin_skeleton_label: None,
                skin_joint_limit: None,
                skin_influence_limit: None,
                skin_root_bone: None,
                animation_skeleton_label: None,
                material_mesh_label: None,
                material_labels: Vec::new(),
                physics_mesh_labels: Vec::new(),
                lod_mesh_labels: Vec::new(),
                line_number,
            });
        }
    }
    for material_use in material_uses {
        subresources.push(ModelSubresource {
            kind: "material".to_owned(),
            label: format!("Material/{}", material_use.name),
            payload: obj_material_payload(
                &material_use.name,
                &material_libraries,
                material_properties.materials.get(&material_use.name),
            ),
            dependency_labels: Vec::new(),
            skin_skeleton_label: None,
            skin_joint_limit: None,
            skin_influence_limit: None,
            skin_root_bone: None,
            animation_skeleton_label: None,
            material_mesh_label: None,
            material_labels: Vec::new(),
            physics_mesh_labels: Vec::new(),
            lod_mesh_labels: Vec::new(),
            line_number: material_use.line_number,
        });
    }
    Ok(subresources)
}

#[cfg(feature = "model_importer")]
fn obj_mesh_has_triangles(mesh: &ObjMeshSource) -> bool {
    mesh.material_groups
        .iter()
        .any(|group| !group.triangles.is_empty())
}

#[cfg(feature = "model_importer")]
fn push_obj_mesh_triangles(
    mesh: &mut ObjMeshSource,
    material_label: Option<&String>,
    triangles: Vec<[ObjVertexRef; 3]>,
    smoothing_group: &ObjSmoothingGroup,
    line_number: usize,
) {
    let material_label = material_label.cloned();
    let triangles = triangles
        .into_iter()
        .map(|vertices| ObjMeshTriangle {
            vertices,
            smoothing_group: smoothing_group.clone(),
        })
        .collect::<Vec<_>>();
    if let Some(group) = mesh
        .material_groups
        .iter_mut()
        .find(|group| group.material_label == material_label)
    {
        group.triangles.extend(triangles);
        return;
    }
    mesh.material_groups.push(ObjMeshMaterialGroup {
        material_label,
        triangles,
        line_number,
    });
}

#[cfg(feature = "model_importer")]
fn obj_material_group_mesh_label(
    mesh_label: &str,
    material_label: Option<&str>,
    group_index: usize,
) -> String {
    if let Some(material_name) = material_label.and_then(|label| label.strip_prefix("Material/")) {
        format!("{mesh_label}.Material/{material_name}")
    } else {
        format!("{mesh_label}.Material/Unassigned{group_index}")
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_vertex<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<[f32; 3], ImportError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing OBJ vertex value on line {line_number}"),
            })?
            .parse::<f32>()
            .map_err(|error| AssetError::Import {
                message: format!("invalid OBJ vertex value on line {line_number}: {error}"),
            })?;
        if !value.is_finite() {
            return Err(AssetError::Import {
                message: format!("OBJ vertex value must be finite on line {line_number}"),
            });
        }
    }
    if let Some(weight) = parts.next() {
        let weight = weight.parse::<f32>().map_err(|error| AssetError::Import {
            message: format!(
                "invalid OBJ vertex homogeneous coordinate on line {line_number}: {error}"
            ),
        })?;
        if !weight.is_finite() || weight == 0.0 {
            return Err(AssetError::Import {
                message: format!(
                    "OBJ vertex homogeneous coordinate must be finite and non-zero on line {line_number}"
                ),
            });
        }
        for value in &mut values {
            *value /= weight;
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many OBJ vertex values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_obj_texture_coord<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<[f32; 3], ImportError> {
    let mut values = [0.0; 3];
    let mut count = 0;
    for value in &mut values {
        let Some(part) = parts.next() else {
            break;
        };
        *value = part.parse::<f32>().map_err(|error| AssetError::Import {
            message: format!("invalid OBJ texture coordinate value on line {line_number}: {error}"),
        })?;
        if !value.is_finite() {
            return Err(AssetError::Import {
                message: format!(
                    "OBJ texture coordinate value must be finite on line {line_number}"
                ),
            });
        }
        count += 1;
    }
    if count == 0 {
        return Err(AssetError::Import {
            message: format!("missing OBJ texture coordinate value on line {line_number}"),
        });
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many OBJ texture coordinate values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_obj_normal<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<[f32; 3], ImportError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Import {
                message: format!("missing OBJ normal value on line {line_number}"),
            })?
            .parse::<f32>()
            .map_err(|error| AssetError::Import {
                message: format!("invalid OBJ normal value on line {line_number}: {error}"),
            })?;
        if !value.is_finite() {
            return Err(AssetError::Import {
                message: format!("OBJ normal value must be finite on line {line_number}"),
            });
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many OBJ normal values on line {line_number}"),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_obj_smoothing_group<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<ObjSmoothingGroup, ImportError> {
    let value = parts.next().ok_or_else(|| AssetError::Import {
        message: format!("missing OBJ smoothing group value on line {line_number}"),
    })?;
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!("too many OBJ smoothing group values on line {line_number}"),
        });
    }
    Ok(match value {
        "off" | "0" => ObjSmoothingGroup::Off,
        group => ObjSmoothingGroup::Group(group.to_owned()),
    })
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_libraries(
    value: &str,
    line_number: usize,
    material_libraries: &mut Vec<ObjMaterialLibraryRef>,
) -> Result<(), ImportError> {
    let names = value
        .split_whitespace()
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if names.is_empty() {
        return Err(AssetError::Import {
            message: format!("OBJ mtllib is empty on line {line_number}"),
        });
    }
    for name in names {
        if !material_libraries
            .iter()
            .any(|existing| existing.name == name)
        {
            material_libraries.push(ObjMaterialLibraryRef {
                name: name.to_owned(),
                line_number,
            });
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn parse_obj_face<'a>(
    parts: impl Iterator<Item = &'a str>,
    vertex_count: usize,
    texture_coord_count: usize,
    normal_count: usize,
    line_number: usize,
) -> Result<Vec<[ObjVertexRef; 3]>, ImportError> {
    let indices = parts
        .map(|part| {
            parse_obj_face_index(
                part,
                vertex_count,
                texture_coord_count,
                normal_count,
                line_number,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if indices.len() < 3 {
        return Err(AssetError::Import {
            message: format!("OBJ face on line {line_number} must contain at least 3 vertices"),
        });
    }
    let mut triangles = Vec::new();
    for index in 1..indices.len() - 1 {
        triangles.push([indices[0], indices[index], indices[index + 1]]);
    }
    Ok(triangles)
}

#[cfg(feature = "model_importer")]
fn parse_obj_face_index(
    token: &str,
    vertex_count: usize,
    texture_coord_count: usize,
    normal_count: usize,
    line_number: usize,
) -> Result<ObjVertexRef, ImportError> {
    let parts = token.split('/').collect::<Vec<_>>();
    if parts.len() > 3 {
        return Err(AssetError::Import {
            message: format!("invalid OBJ face tuple `{token}` on line {line_number}"),
        });
    }
    let zero_based = parse_obj_index_reference(
        parts.first().copied().unwrap_or(""),
        token,
        "face",
        "vertex",
        vertex_count,
        line_number,
    )?;
    let texture_coord =
        if let Some(texture_coord) = parts.get(1).copied().filter(|part| !part.is_empty()) {
            Some(parse_obj_index_reference(
                texture_coord,
                token,
                "texture coordinate",
                "texture coordinate",
                texture_coord_count,
                line_number,
            )?)
        } else {
            None
        };
    let normal = if let Some(normal) = parts.get(2).copied().filter(|part| !part.is_empty()) {
        Some(parse_obj_index_reference(
            normal,
            token,
            "normal",
            "normal",
            normal_count,
            line_number,
        )?)
    } else {
        None
    };
    Ok(ObjVertexRef {
        vertex: zero_based,
        texture_coord,
        normal,
    })
}

#[cfg(feature = "model_importer")]
fn parse_obj_index_reference(
    value: &str,
    token: &str,
    kind: &str,
    referenced_kind: &str,
    count: usize,
    line_number: usize,
) -> Result<usize, ImportError> {
    let index = value.parse::<i64>().map_err(|error| AssetError::Import {
        message: format!("invalid OBJ {kind} index `{token}` on line {line_number}: {error}"),
    })?;
    if index == 0 {
        return Err(AssetError::Import {
            message: format!("OBJ {kind} index 0 on line {line_number} must be non-zero"),
        });
    }
    let zero_based = if index > 0 {
        index - 1
    } else {
        count as i64 + index
    };
    if zero_based < 0 || zero_based as usize >= count {
        return Err(AssetError::Import {
            message: format!(
                "OBJ {kind} index {index} on line {line_number} references missing {referenced_kind}; {referenced_kind} count is {count}"
            ),
        });
    }
    Ok(zero_based as usize)
}

#[cfg(feature = "model_importer")]
fn obj_mesh_payload(
    vertices: &[[f32; 3]],
    texture_coords: &[[f32; 3]],
    normals: &[[f32; 3]],
    triangles: &[ObjMeshTriangle],
    label: &str,
    line_number: usize,
    generate_tangents: bool,
) -> Result<String, ImportError> {
    let generate_smoothing_normals = triangles
        .iter()
        .any(|triangle| triangle.smoothing_group != ObjSmoothingGroup::Unspecified);
    let mut expanded: Vec<ObjExpandedVertex> = Vec::new();
    let mut indices = Vec::new();
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let mut expanded_triangle = [0; 3];
        let generated_normal =
            generate_smoothing_normals.then(|| match &triangle.smoothing_group {
                ObjSmoothingGroup::Group(group) => ObjGeneratedNormalKey::Smooth(group.clone()),
                ObjSmoothingGroup::Off | ObjSmoothingGroup::Unspecified => {
                    ObjGeneratedNormalKey::Flat(triangle_index)
                }
            });
        for (corner, vertex_ref) in triangle.vertices.iter().enumerate() {
            let expanded_vertex = ObjExpandedVertex {
                vertex_ref: *vertex_ref,
                generated_normal: generated_normal.clone(),
            };
            let index = if let Some(index) = expanded
                .iter()
                .position(|existing| existing == &expanded_vertex)
            {
                index
            } else {
                expanded.push(expanded_vertex);
                expanded.len() - 1
            };
            expanded_triangle[corner] = u32::try_from(index).map_err(|_| AssetError::Import {
                message: format!("OBJ mesh `{label}` on line {line_number} has too many vertices"),
            })?;
        }
        indices.push(expanded_triangle);
    }

    let has_texture_coords = expanded
        .iter()
        .any(|expanded: &ObjExpandedVertex| expanded.vertex_ref.texture_coord.is_some());
    let has_explicit_normals = expanded
        .iter()
        .any(|expanded: &ObjExpandedVertex| expanded.vertex_ref.normal.is_some());
    if has_texture_coords
        && expanded
            .iter()
            .any(|expanded: &ObjExpandedVertex| expanded.vertex_ref.texture_coord.is_none())
    {
        return Err(AssetError::Import {
            message: format!(
                "OBJ mesh `{label}` on line {line_number} mixes vertices with and without texture coordinates"
            ),
        });
    }
    if has_explicit_normals
        && expanded
            .iter()
            .any(|expanded: &ObjExpandedVertex| expanded.vertex_ref.normal.is_none())
    {
        return Err(AssetError::Import {
            message: format!(
                "OBJ mesh `{label}` on line {line_number} mixes vertices with and without normals"
            ),
        });
    }
    let generated_normals = (!has_explicit_normals && generate_smoothing_normals)
        .then(|| obj_mesh_generated_normals(&expanded, vertices, triangles));

    let mut payload = String::new();
    for expanded_vertex in &expanded {
        let vertex = vertices[expanded_vertex.vertex_ref.vertex];
        payload.push_str(&format!(
            "v {} {} {}\n",
            canonical_mesh_f32(vertex[0]),
            canonical_mesh_f32(vertex[1]),
            canonical_mesh_f32(vertex[2])
        ));
    }
    if has_explicit_normals {
        for expanded_vertex in &expanded {
            let normal = normals[expanded_vertex
                .vertex_ref
                .normal
                .expect("normal presence validated for expanded OBJ vertices")];
            payload.push_str(&format!(
                "n {} {} {}\n",
                canonical_mesh_f32(normal[0]),
                canonical_mesh_f32(normal[1]),
                canonical_mesh_f32(normal[2])
            ));
        }
    } else if let Some(generated_normals) = &generated_normals {
        for normal in generated_normals {
            payload.push_str(&format!(
                "n {} {} {}\n",
                canonical_mesh_f32(normal[0]),
                canonical_mesh_f32(normal[1]),
                canonical_mesh_f32(normal[2])
            ));
        }
    }
    if has_texture_coords {
        for expanded_vertex in &expanded {
            let texture_coord = texture_coords[expanded_vertex
                .vertex_ref
                .texture_coord
                .expect("texture coordinate presence validated for expanded OBJ vertices")];
            payload.push_str(&format!(
                "uv {} {}\n",
                canonical_mesh_f32(texture_coord[0]),
                canonical_mesh_f32(texture_coord[1])
            ));
        }
        for tangent in generate_tangents
            .then(|| {
                obj_mesh_tangents(
                    &expanded,
                    vertices,
                    texture_coords,
                    normals,
                    generated_normals.as_deref(),
                    &indices,
                )
            })
            .into_iter()
            .flatten()
        {
            payload.push_str(&format!(
                "t {} {} {} {}\n",
                canonical_mesh_f32(tangent[0]),
                canonical_mesh_f32(tangent[1]),
                canonical_mesh_f32(tangent[2]),
                canonical_mesh_f32(tangent[3])
            ));
        }
    }
    for triangle in &indices {
        payload.push_str(&format!(
            "i {} {} {}\n",
            triangle[0], triangle[1], triangle[2]
        ));
    }
    Ok(payload)
}

#[cfg(feature = "model_importer")]
fn obj_mesh_generated_normals(
    expanded: &[ObjExpandedVertex],
    vertices: &[[f32; 3]],
    triangles: &[ObjMeshTriangle],
) -> Vec<[f32; 3]> {
    let face_normals = triangles
        .iter()
        .map(|triangle| obj_mesh_triangle_normal(vertices, &triangle.vertices))
        .collect::<Vec<_>>();
    let mut smooth_normals = HashMap::<(usize, String), [f32; 3]>::new();
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        let ObjSmoothingGroup::Group(group) = &triangle.smoothing_group else {
            continue;
        };
        for vertex_ref in triangle.vertices {
            let entry = smooth_normals
                .entry((vertex_ref.vertex, group.clone()))
                .or_insert([0.0, 0.0, 0.0]);
            *entry = mesh_vec3_add(*entry, face_normals[triangle_index]);
        }
    }

    expanded
        .iter()
        .map(|expanded_vertex| match &expanded_vertex.generated_normal {
            Some(ObjGeneratedNormalKey::Flat(triangle_index)) => face_normals[*triangle_index],
            Some(ObjGeneratedNormalKey::Smooth(group)) => {
                let normal = smooth_normals
                    .get(&(expanded_vertex.vertex_ref.vertex, group.clone()))
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0]);
                mesh_vec3_normalize_or(normal, [0.0, 1.0, 0.0])
            }
            None => [0.0, 1.0, 0.0],
        })
        .collect()
}

#[cfg(feature = "model_importer")]
fn obj_mesh_triangle_normal(vertices: &[[f32; 3]], triangle: &[ObjVertexRef; 3]) -> [f32; 3] {
    let p0 = vertices[triangle[0].vertex];
    let p1 = vertices[triangle[1].vertex];
    let p2 = vertices[triangle[2].vertex];
    mesh_vec3_normalize_or(
        mesh_vec3_cross(mesh_vec3_sub(p1, p0), mesh_vec3_sub(p2, p0)),
        [0.0, 1.0, 0.0],
    )
}

#[cfg(feature = "model_importer")]
fn obj_mesh_tangents(
    expanded: &[ObjExpandedVertex],
    vertices: &[[f32; 3]],
    texture_coords: &[[f32; 3]],
    normals: &[[f32; 3]],
    generated_normals: Option<&[[f32; 3]]>,
    indices: &[[u32; 3]],
) -> Vec<[f32; 4]> {
    let mut tangents = vec![[0.0, 0.0, 0.0]; expanded.len()];
    let mut bitangents = vec![[0.0, 0.0, 0.0]; expanded.len()];
    for triangle in indices {
        let [i0, i1, i2] = *triangle;
        let (i0, i1, i2) = (i0 as usize, i1 as usize, i2 as usize);
        let p0 = vertices[expanded[i0].vertex_ref.vertex];
        let p1 = vertices[expanded[i1].vertex_ref.vertex];
        let p2 = vertices[expanded[i2].vertex_ref.vertex];
        let uv0 = texture_coords[expanded[i0]
            .vertex_ref
            .texture_coord
            .expect("texture coordinate presence validated for OBJ tangent generation")];
        let uv1 = texture_coords[expanded[i1]
            .vertex_ref
            .texture_coord
            .expect("texture coordinate presence validated for OBJ tangent generation")];
        let uv2 = texture_coords[expanded[i2]
            .vertex_ref
            .texture_coord
            .expect("texture coordinate presence validated for OBJ tangent generation")];
        let delta_pos1 = mesh_vec3_sub(p1, p0);
        let delta_pos2 = mesh_vec3_sub(p2, p0);
        let delta_uv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
        let delta_uv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];
        let determinant = delta_uv1[0] * delta_uv2[1] - delta_uv2[0] * delta_uv1[1];
        if determinant.abs() <= f32::EPSILON {
            continue;
        }
        let inverse = 1.0 / determinant;
        let tangent = [
            inverse * (delta_uv2[1] * delta_pos1[0] - delta_uv1[1] * delta_pos2[0]),
            inverse * (delta_uv2[1] * delta_pos1[1] - delta_uv1[1] * delta_pos2[1]),
            inverse * (delta_uv2[1] * delta_pos1[2] - delta_uv1[1] * delta_pos2[2]),
        ];
        let bitangent = [
            inverse * (-delta_uv2[0] * delta_pos1[0] + delta_uv1[0] * delta_pos2[0]),
            inverse * (-delta_uv2[0] * delta_pos1[1] + delta_uv1[0] * delta_pos2[1]),
            inverse * (-delta_uv2[0] * delta_pos1[2] + delta_uv1[0] * delta_pos2[2]),
        ];
        for index in [i0, i1, i2] {
            tangents[index] = mesh_vec3_add(tangents[index], tangent);
            bitangents[index] = mesh_vec3_add(bitangents[index], bitangent);
        }
    }

    tangents
        .into_iter()
        .enumerate()
        .map(|(index, tangent)| {
            let tangent = mesh_vec3_normalize_or(tangent, [1.0, 0.0, 0.0]);
            let normal = expanded[index]
                .vertex_ref
                .normal
                .map(|normal_index| normals[normal_index])
                .or_else(|| generated_normals.map(|normals| normals[index]));
            let handedness = if let Some(normal) = normal {
                if mesh_vec3_dot(mesh_vec3_cross(normal, tangent), bitangents[index]) < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            } else {
                1.0
            };
            [tangent[0], tangent[1], tangent[2], handedness]
        })
        .collect()
}

#[cfg(feature = "model_importer")]
fn mesh_vec3_sub(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

#[cfg(feature = "model_importer")]
fn mesh_vec3_add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

#[cfg(feature = "model_importer")]
fn mesh_vec3_dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

#[cfg(feature = "model_importer")]
fn mesh_vec3_cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

#[cfg(feature = "model_importer")]
fn mesh_vec3_normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length = mesh_vec3_dot(value, value).sqrt();
    if length <= f32::EPSILON {
        fallback
    } else {
        [value[0] / length, value[1] / length, value[2] / length]
    }
}

#[cfg(feature = "model_importer")]
fn obj_material_properties_from_libraries(
    ctx: &ImportContext,
    source_path: &AssetPath,
    material_libraries: &[ObjMaterialLibraryRef],
) -> Result<ObjMaterialLibraryProperties, ImportError> {
    let mut materials = HashMap::new();
    let mut material_sources = HashMap::new();
    let mut loaded_library_count = 0;
    for library in material_libraries {
        let path = resolve_obj_material_library_path(source_path, library)?;
        let Some(source) = ctx.source_file(&path) else {
            continue;
        };
        loaded_library_count += 1;
        let source_text =
            std::str::from_utf8(&source.bytes).map_err(|error| AssetError::Import {
                message: format!(
                    "OBJ material library `{}` at `{}` must be UTF-8: {error}",
                    library.name,
                    path.display_string()
                ),
            })?;
        let path_display = path.display_string();
        for material in parse_obj_material_library_text(source_text, library, &path)? {
            if let Some((previous_library, previous_path, previous_line)) =
                material_sources.get(&material.name)
            {
                return Err(AssetError::Import {
                    message: format!(
                        "OBJ material `{}` is defined by multiple mtllib sources: `{previous_library}` at `{previous_path}` line {previous_line} and `{}` at `{path_display}` line {}",
                        material.name,
                        library.name,
                        material.line_number
                    ),
                });
            }
            material_sources.insert(
                material.name.clone(),
                (
                    library.name.clone(),
                    path_display.clone(),
                    material.line_number,
                ),
            );
            materials.insert(material.name, material.properties);
        }
    }
    Ok(ObjMaterialLibraryProperties {
        materials,
        loaded_library_count,
    })
}

#[cfg(feature = "model_importer")]
fn validate_obj_material_uses(
    material_uses: &[ObjMaterialUse],
    material_libraries: &[ObjMaterialLibraryRef],
    material_properties: &ObjMaterialLibraryProperties,
) -> Result<(), ImportError> {
    if material_libraries.is_empty()
        || material_properties.loaded_library_count != material_libraries.len()
    {
        return Ok(());
    }
    for material_use in material_uses {
        if !material_properties
            .materials
            .contains_key(&material_use.name)
        {
            let libraries = material_libraries
                .iter()
                .map(|library| library.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AssetError::Import {
                message: format!(
                    "OBJ usemtl `{}` on line {} is not defined by loaded mtllib source(s): {libraries}",
                    material_use.name, material_use.line_number
                ),
            });
        }
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn resolve_obj_material_library_path(
    source_path: &AssetPath,
    library: &ObjMaterialLibraryRef,
) -> Result<AssetPath, ImportError> {
    let normalized = normalize_obj_relative_source_path(&library.name).map_err(|()| {
        AssetError::Import {
            message: format!(
                "OBJ mtllib `{}` on line {} must be a relative source path without labels or `..` segments",
                library.name, library.line_number
            ),
        }
    })?;
    Ok(obj_relative_source_path(source_path, normalized))
}

#[cfg(feature = "model_importer")]
fn resolve_obj_material_texture_path(
    material_library_path: &AssetPath,
    texture_path: &str,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    line_number: usize,
) -> Result<AssetPath, ImportError> {
    let normalized = normalize_obj_relative_source_path(texture_path).map_err(|()| {
        AssetError::Import {
            message: format!(
                "OBJ material library `{}` at `{}` {directive} texture path `{texture_path}` on line {line_number} must be a relative source path without labels or `..` segments",
                library.name,
                material_library_path.display_string()
            ),
        }
    })?;
    Ok(obj_relative_source_path(material_library_path, normalized))
}

#[cfg(feature = "model_importer")]
fn normalize_obj_relative_source_path(value: &str) -> Result<String, ()> {
    let mut normalized = value.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized.contains('#')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(());
    }
    Ok(normalized)
}

#[cfg(feature = "model_importer")]
fn obj_relative_source_path(base_path: &AssetPath, normalized: String) -> AssetPath {
    let directory = base_path
        .path()
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    if directory.is_empty() {
        AssetPath::new(normalized)
    } else {
        AssetPath::new(format!("{directory}/{normalized}"))
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_library_text(
    source_text: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
) -> Result<Vec<ObjMaterialDefinition>, ImportError> {
    let mut materials = Vec::new();
    let mut defined_names = Vec::<String>::new();
    let mut current_name = None;
    let mut current_properties = ObjMaterialProperties::default();
    for (line_index, line) in source_text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.split_once('#').map(|(value, _)| value).unwrap_or(line);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let directive = parts.next().unwrap_or("");
        match directive {
            "newmtl" => {
                if let Some((name, line_number)) = current_name.take() {
                    materials.push(ObjMaterialDefinition {
                        name,
                        properties: current_properties,
                        line_number,
                    });
                    current_properties = ObjMaterialProperties::default();
                }
                let name = line["newmtl".len()..].trim();
                if name.is_empty() {
                    return Err(AssetError::Import {
                        message: format!(
                            "OBJ material library `{}` at `{}` has empty newmtl on line {line_number}",
                            library.name,
                            path.display_string()
                        ),
                    });
                }
                if defined_names
                    .iter()
                    .any(|defined_name| defined_name.as_str() == name)
                {
                    return Err(AssetError::Import {
                        message: format!(
                            "OBJ material library `{}` at `{}` defines duplicate newmtl `{name}` on line {line_number}",
                            library.name,
                            path.display_string()
                        ),
                    });
                }
                defined_names.push(name.to_owned());
                current_name = Some((name.to_owned(), line_number));
            }
            "Kd" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                let color = parse_obj_material_rgb(parts, directive, library, path, line_number)?;
                let alpha = current_properties
                    .base_color
                    .map(|base_color| base_color[3])
                    .unwrap_or(1.0);
                current_properties.base_color = Some([color[0], color[1], color[2], alpha]);
            }
            "Ka" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.ambient_color = Some(parse_obj_material_rgb(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "Ks" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.specular_color = Some(parse_obj_material_rgb(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "Ke" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.emissive = Some(parse_obj_material_rgb(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "d" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                let (alpha, halo) =
                    parse_obj_material_dissolve(parts, directive, library, path, line_number)?;
                set_obj_material_alpha(&mut current_properties, alpha);
                current_properties.dissolve_halo |= halo;
            }
            "Tr" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                let transparency =
                    parse_obj_material_scalar(parts, directive, library, path, line_number)?;
                set_obj_material_alpha(&mut current_properties, 1.0 - transparency);
            }
            "Ns" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                let shininess =
                    parse_obj_material_scalar(parts, directive, library, path, line_number)?;
                let gloss = (shininess / 1000.0).clamp(0.0, 1.0).sqrt();
                current_properties.roughness = Some((1.0 - gloss).clamp(0.0, 1.0));
            }
            "Pr" | "roughness" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.roughness = Some(
                    parse_obj_material_scalar(parts, directive, library, path, line_number)?
                        .clamp(0.0, 1.0),
                );
            }
            "Tf" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.transmission_filter = Some(parse_obj_material_rgb(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "Ni" | "ior" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.index_of_refraction = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "illum" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.illumination_model = Some(parse_obj_material_integer(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "sharpness" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.sharpness = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "map_aat" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.texture_antialias = Some(parse_obj_material_bool(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "Pm" | "metallic" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.metallic = Some(
                    parse_obj_material_scalar(parts, directive, library, path, line_number)?
                        .clamp(0.0, 1.0),
                );
            }
            "Ps" | "sheen" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.sheen = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "Pc" | "clearcoat" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.clearcoat = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "Pcr" | "clearcoat_roughness" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.clearcoat_roughness = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "aniso" | "anisotropy" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.anisotropy = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            "anisor" | "anisotropy_rotation" => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                current_properties.anisotropy_rotation = Some(parse_obj_material_scalar(
                    parts,
                    directive,
                    library,
                    path,
                    line_number,
                )?);
            }
            directive if obj_material_texture_channel(directive).is_some() => {
                require_obj_material_current(&current_name, directive, library, path, line_number)?;
                let texture_map =
                    parse_obj_material_texture_map(parts, directive, library, path, line_number)?;
                let texture_path = resolve_obj_material_texture_path(
                    path,
                    &texture_map.path,
                    directive,
                    library,
                    line_number,
                )?
                .display_string();
                set_obj_material_texture(
                    &mut current_properties,
                    obj_material_texture_channel(directive)
                        .expect("directive matched known OBJ material texture channel"),
                    texture_path,
                    texture_map.options,
                );
            }
            _ => {}
        }
    }
    if let Some((name, line_number)) = current_name {
        materials.push(ObjMaterialDefinition {
            name,
            properties: current_properties,
            line_number,
        });
    }
    Ok(materials)
}

#[cfg(feature = "model_importer")]
fn require_obj_material_current(
    current_name: &Option<(String, usize)>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<(), ImportError> {
    if current_name.is_none() {
        return Err(AssetError::Import {
            message: format!(
                "OBJ material library `{}` at `{}` has {directive} before newmtl on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_rgb<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<[f32; 3], ImportError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = parse_obj_material_f32(
            parts.next().ok_or_else(|| AssetError::Import {
                message: format!(
                    "missing OBJ material library `{}` at `{}` {directive} value on line {line_number}",
                    library.name,
                    path.display_string()
                ),
            })?,
            directive,
            library,
            path,
            line_number,
        )?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!(
                "too many OBJ material library `{}` at `{}` {directive} values on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(values)
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_scalar<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<f32, ImportError> {
    let value = parse_obj_material_f32(
        parts.next().ok_or_else(|| AssetError::Import {
            message: format!(
                "missing OBJ material library `{}` at `{}` {directive} value on line {line_number}",
                library.name,
                path.display_string()
            ),
        })?,
        directive,
        library,
        path,
        line_number,
    )?;
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!(
                "too many OBJ material library `{}` at `{}` {directive} values on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(value)
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_dissolve<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<(f32, bool), ImportError> {
    let first = parts.next().ok_or_else(|| AssetError::Import {
        message: format!(
            "missing OBJ material library `{}` at `{}` {directive} value on line {line_number}",
            library.name,
            path.display_string()
        ),
    })?;
    let (value, halo) = if first == "-halo" {
        (
            parts.next().ok_or_else(|| AssetError::Import {
                message: format!(
                    "missing OBJ material library `{}` at `{}` {directive} value on line {line_number}",
                    library.name,
                    path.display_string()
                ),
            })?,
            true,
        )
    } else if first.starts_with('-') {
        return Err(AssetError::Import {
            message: format!(
                "unknown OBJ material library `{}` at `{}` {directive} option {first} on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    } else {
        (first, false)
    };
    let value = parse_obj_material_f32(value, directive, library, path, line_number)?;
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!(
                "too many OBJ material library `{}` at `{}` {directive} values on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok((value, halo))
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_integer<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<i32, ImportError> {
    let value = parse_obj_material_i32(
        parts.next().ok_or_else(|| AssetError::Import {
            message: format!(
                "missing OBJ material library `{}` at `{}` {directive} value on line {line_number}",
                library.name,
                path.display_string()
            ),
        })?,
        directive,
        library,
        path,
        line_number,
    )?;
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!(
                "too many OBJ material library `{}` at `{}` {directive} values on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(value)
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_bool<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<bool, ImportError> {
    let value = parse_obj_material_bool_value(
        parts.next().ok_or_else(|| AssetError::Import {
            message: format!(
                "missing OBJ material library `{}` at `{}` {directive} value on line {line_number}",
                library.name,
                path.display_string()
            ),
        })?,
        directive,
        library,
        path,
        line_number,
    )?;
    if parts.next().is_some() {
        return Err(AssetError::Import {
            message: format!(
                "too many OBJ material library `{}` at `{}` {directive} values on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(value)
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_bool_value(
    value: &str,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<bool, ImportError> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(AssetError::Import {
            message: format!(
                "invalid OBJ material library `{}` at `{}` {directive} value `{other}` on line {line_number}",
                library.name,
                path.display_string()
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_map<'a>(
    parts: impl Iterator<Item = &'a str>,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<ObjMaterialTextureMap, ImportError> {
    let parts = parts.collect::<Vec<_>>();
    let mut texture_path = None;
    let mut options = ObjMaterialTextureOptions::default();
    let mut index = 0;
    while index < parts.len() {
        let token = parts[index];
        if token.starts_with('-') {
            let option = parse_obj_material_texture_option(
                &parts,
                index,
                directive,
                library,
                path,
                line_number,
            )?;
            merge_obj_material_texture_options(&mut options, option.options);
            index = option.next_index;
            continue;
        }
        if texture_path.replace(token.to_owned()).is_some() {
            return Err(AssetError::Import {
                message: format!(
                    "too many OBJ material library `{}` at `{}` {directive} texture paths on line {line_number}",
                    library.name,
                    path.display_string()
                ),
            });
        }
        index += 1;
    }
    let path = texture_path.ok_or_else(|| AssetError::Import {
            message: format!(
                "missing OBJ material library `{}` at `{}` {directive} texture path on line {line_number}",
                library.name,
                path.display_string()
            ),
        })?;
    Ok(ObjMaterialTextureMap { path, options })
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
struct ObjMaterialTextureMap {
    path: String,
    options: ObjMaterialTextureOptions,
}

#[cfg(feature = "model_importer")]
#[derive(Clone, Debug, PartialEq)]
struct ObjMaterialTextureOption {
    next_index: usize,
    options: ObjMaterialTextureOptions,
}

#[cfg(feature = "model_importer")]
fn merge_obj_material_texture_options(
    target: &mut ObjMaterialTextureOptions,
    source: ObjMaterialTextureOptions,
) {
    if source.sampler_address.is_some() {
        target.sampler_address = source.sampler_address;
    }
    if source.transform_offset.is_some() {
        target.transform_offset = source.transform_offset;
    }
    if source.transform_scale.is_some() {
        target.transform_scale = source.transform_scale;
    }
    if source.transform_turbulence.is_some() {
        target.transform_turbulence = source.transform_turbulence;
    }
    if source.bump_scale.is_some() {
        target.bump_scale = source.bump_scale;
    }
    if source.color_remap.is_some() {
        target.color_remap = source.color_remap;
    }
    if source.source_channel.is_some() {
        target.source_channel = source.source_channel;
    }
    if source.boost.is_some() {
        target.boost = source.boost;
    }
    if source.blend_u.is_some() {
        target.blend_u = source.blend_u;
    }
    if source.blend_v.is_some() {
        target.blend_v = source.blend_v;
    }
    if source.color_correction.is_some() {
        target.color_correction = source.color_correction;
    }
    if source.projection.is_some() {
        target.projection = source.projection;
    }
    if source.texture_resolution.is_some() {
        target.texture_resolution = source.texture_resolution;
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_option(
    parts: &[&str],
    index: usize,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<ObjMaterialTextureOption, ImportError> {
    let option = parts[index];
    match option {
        "-blendu" | "-blendv" | "-cc" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            let value = parse_obj_material_texture_option_bool(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            match option {
                "-blendu" => options.blend_u = Some(value),
                "-blendv" => options.blend_v = Some(value),
                "-cc" => options.color_correction = Some(value),
                _ => unreachable!("matched texture boolean option"),
            }
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-type" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            options.projection = Some(parse_obj_material_texture_projection(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?);
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-imfchan" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            options.source_channel = Some(parse_obj_material_texture_source_channel(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?);
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-clamp" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            options.sampler_address = Some(parse_obj_material_texture_clamp(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?);
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-texres" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            options.texture_resolution = Some(parse_obj_material_texture_resolution(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?);
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-boost" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            options.boost = Some(parse_obj_material_texture_option_number(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?);
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-bm" => {
            let value = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            options.bump_scale = Some(parse_obj_material_texture_option_number(
                value,
                directive,
                option,
                library,
                path,
                line_number,
            )?);
            Ok(ObjMaterialTextureOption {
                next_index: index + 2,
                options,
            })
        }
        "-mm" => {
            let first = require_obj_material_texture_option_arg(
                parts,
                index,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let second = parts.get(index + 2).copied().ok_or_else(|| {
                missing_obj_material_texture_option_value(
                    directive,
                    option,
                    library,
                    path,
                    line_number,
                )
            })?;
            parse_obj_material_texture_option_number(
                first,
                directive,
                option,
                library,
                path,
                line_number,
            )?;
            let color_remap = [
                parse_obj_material_texture_option_number(
                    first,
                    directive,
                    option,
                    library,
                    path,
                    line_number,
                )?,
                parse_obj_material_texture_option_number(
                    second,
                    directive,
                    option,
                    library,
                    path,
                    line_number,
                )?,
            ];
            let mut options = ObjMaterialTextureOptions::default();
            options.color_remap = Some(color_remap);
            Ok(ObjMaterialTextureOption {
                next_index: index + 3,
                options,
            })
        }
        "-o" | "-s" | "-t" => {
            let default = if option == "-s" { 1.0 } else { 0.0 };
            let (next_index, value) = parse_obj_material_texture_option_vec3(
                parts,
                index,
                directive,
                option,
                default,
                library,
                path,
                line_number,
            )?;
            let mut options = ObjMaterialTextureOptions::default();
            match option {
                "-o" => options.transform_offset = Some(value),
                "-s" => options.transform_scale = Some(value),
                "-t" => options.transform_turbulence = Some(value),
                _ => unreachable!("matched texture transform option"),
            }
            Ok(ObjMaterialTextureOption {
                next_index,
                options,
            })
        }
        _ => Err(AssetError::Import {
            message: format!(
                "unknown OBJ material library `{}` at `{}` {directive} option {option} on line {line_number}",
                library.name,
                path.display_string()
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_clamp(
    value: &str,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<String, ImportError> {
    match value {
        "on" | "true" | "1" => Ok("clamp_to_edge".to_owned()),
        "off" | "false" | "0" => Ok("repeat".to_owned()),
        other => Err(AssetError::Import {
            message: format!(
                "invalid OBJ material library `{}` at `{}` {directive} option {option} value `{other}` on line {line_number}",
                library.name,
                path.display_string()
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_option_bool(
    value: &str,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<bool, ImportError> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        other => Err(AssetError::Import {
            message: format!(
                "invalid OBJ material library `{}` at `{}` {directive} option {option} value `{other}` on line {line_number}",
                library.name,
                path.display_string()
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_projection(
    value: &str,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<String, ImportError> {
    match value {
        "flat" | "sphere" | "cube_top" | "cube_bottom" | "cube_front" | "cube_back"
        | "cube_left" | "cube_right" => Ok(value.to_owned()),
        other => Err(AssetError::Import {
            message: format!(
                "invalid OBJ material library `{}` at `{}` {directive} option {option} value `{other}` on line {line_number}",
                library.name,
                path.display_string()
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_resolution(
    value: &str,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<u32, ImportError> {
    let resolution = value.parse::<u32>().map_err(|error| AssetError::Import {
        message: format!(
            "invalid OBJ material library `{}` at `{}` {directive} option {option} value `{value}` on line {line_number}: {error}",
            library.name,
            path.display_string()
        ),
    })?;
    if resolution == 0 {
        return Err(AssetError::Import {
            message: format!(
                "OBJ material library `{}` at `{}` {directive} option {option} value must be greater than zero on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(resolution)
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_source_channel(
    value: &str,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<String, ImportError> {
    match value {
        "r" | "red" => Ok("red".to_owned()),
        "g" | "green" => Ok("green".to_owned()),
        "b" | "blue" => Ok("blue".to_owned()),
        "m" | "matte" => Ok("matte".to_owned()),
        "l" | "luminance" => Ok("luminance".to_owned()),
        "z" | "depth" => Ok("depth".to_owned()),
        other => Err(AssetError::Import {
            message: format!(
                "invalid OBJ material library `{}` at `{}` {directive} option {option} value `{other}` on line {line_number}",
                library.name,
                path.display_string()
            ),
        }),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_option_vec3(
    parts: &[&str],
    index: usize,
    directive: &str,
    option: &str,
    default: f32,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<(usize, [f32; 3]), ImportError> {
    let mut cursor = index + 1;
    let mut count = 0;
    let mut values = [default; 3];
    while cursor < parts.len() && count < 3 && is_obj_material_texture_option_number(parts[cursor])
    {
        values[count] = parse_obj_material_texture_option_number(
            parts[cursor],
            directive,
            option,
            library,
            path,
            line_number,
        )?;
        cursor += 1;
        count += 1;
    }
    if count == 0 {
        let value = parts.get(cursor).copied().ok_or_else(|| {
            missing_obj_material_texture_option_value(directive, option, library, path, line_number)
        })?;
        parse_obj_material_texture_option_number(
            value,
            directive,
            option,
            library,
            path,
            line_number,
        )?;
    }
    Ok((cursor, values))
}

#[cfg(feature = "model_importer")]
fn require_obj_material_texture_option_arg<'a>(
    parts: &'a [&str],
    index: usize,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<&'a str, ImportError> {
    parts.get(index + 1).copied().ok_or_else(|| {
        missing_obj_material_texture_option_value(directive, option, library, path, line_number)
    })
}

#[cfg(feature = "model_importer")]
fn missing_obj_material_texture_option_value(
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> ImportError {
    AssetError::Import {
        message: format!(
            "missing OBJ material library `{}` at `{}` {directive} option {option} value on line {line_number}",
            library.name,
            path.display_string()
        ),
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_texture_option_number(
    value: &str,
    directive: &str,
    option: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<f32, ImportError> {
    let value = value.parse::<f32>().map_err(|error| AssetError::Import {
        message: format!(
            "invalid OBJ material library `{}` at `{}` {directive} option {option} value `{value}` on line {line_number}: {error}",
            library.name,
            path.display_string()
        ),
    })?;
    if !value.is_finite() {
        return Err(AssetError::Import {
            message: format!(
                "OBJ material library `{}` at `{}` {directive} option {option} value must be finite on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(value)
}

#[cfg(feature = "model_importer")]
fn is_obj_material_texture_option_number(value: &str) -> bool {
    match value.parse::<f32>() {
        Ok(value) => value.is_finite(),
        Err(_) => false,
    }
}

#[cfg(feature = "model_importer")]
fn obj_material_texture_channel(directive: &str) -> Option<&'static str> {
    match directive {
        "map_Kd" => Some("albedo"),
        "map_Ks" => Some("specular"),
        "map_Ka" => Some("occlusion"),
        "map_Ke" => Some("emissive"),
        "map_Tf" => Some("transmission_filter"),
        "map_d" | "map_Tr" => Some("alpha"),
        "map_Bump" | "map_bump" | "bump" | "norm" => Some("normal"),
        "map_Pr" | "map_Ns" => Some("roughness"),
        "map_Ni" => Some("index_of_refraction"),
        "map_Pm" => Some("metallic"),
        "map_Ps" | "map_sheen" => Some("sheen"),
        "map_Pc" | "map_clearcoat" => Some("clearcoat"),
        "map_Pcr" | "map_clearcoat_roughness" => Some("clearcoat_roughness"),
        "map_aniso" | "map_anisotropy" => Some("anisotropy"),
        "map_anisor" | "map_anisotropy_rotation" => Some("anisotropy_rotation"),
        "disp" | "map_Disp" | "map_disp" | "map_displacement" => Some("displacement"),
        "decal" | "map_decal" => Some("decal"),
        "refl" | "map_refl" => Some("reflection"),
        _ => None,
    }
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_f32(
    value: &str,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<f32, ImportError> {
    let value = value.parse::<f32>().map_err(|error| AssetError::Import {
        message: format!(
            "invalid OBJ material library `{}` at `{}` {directive} value on line {line_number}: {error}",
            library.name,
            path.display_string()
        ),
    })?;
    if !value.is_finite() {
        return Err(AssetError::Import {
            message: format!(
                "OBJ material library `{}` at `{}` {directive} value must be finite on line {line_number}",
                library.name,
                path.display_string()
            ),
        });
    }
    Ok(value)
}

#[cfg(feature = "model_importer")]
fn parse_obj_material_i32(
    value: &str,
    directive: &str,
    library: &ObjMaterialLibraryRef,
    path: &AssetPath,
    line_number: usize,
) -> Result<i32, ImportError> {
    value.parse::<i32>().map_err(|error| AssetError::Import {
        message: format!(
            "invalid OBJ material library `{}` at `{}` {directive} value on line {line_number}: {error}",
            library.name,
            path.display_string()
        ),
    })
}

#[cfg(feature = "model_importer")]
fn set_obj_material_alpha(properties: &mut ObjMaterialProperties, alpha: f32) {
    let mut base_color = properties.base_color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
    let alpha = alpha.clamp(0.0, 1.0);
    base_color[3] = alpha;
    properties.alpha_blend = alpha < 1.0;
    properties.base_color = Some(base_color);
}

#[cfg(feature = "model_importer")]
fn set_obj_material_texture(
    properties: &mut ObjMaterialProperties,
    channel: &str,
    path: String,
    options: ObjMaterialTextureOptions,
) {
    if let Some(texture) = properties
        .textures
        .iter_mut()
        .find(|texture| texture.channel == channel)
    {
        texture.path = path;
        texture.options = options;
    } else {
        properties.textures.push(ObjMaterialTexture {
            channel: channel.to_owned(),
            path,
            options,
        });
    }
    if channel == "alpha" {
        properties.alpha_blend = true;
    }
}

#[cfg(feature = "model_importer")]
fn obj_material_payload(
    material_name: &str,
    material_libraries: &[ObjMaterialLibraryRef],
    properties: Option<&ObjMaterialProperties>,
) -> String {
    let mut payload = String::new();
    for library in material_libraries {
        payload.push_str(&format!("# mtllib {}\n", library.name));
    }
    payload.push_str(&format!("name={material_name}"));
    if let Some(properties) = properties {
        for texture in &properties.textures {
            payload.push_str(&format!("\ntexture.{}={}", texture.channel, texture.path));
            if let Some(address) = &texture.options.sampler_address {
                payload.push_str(&format!(
                    "\ntexture.{}.sampler.address={address}",
                    texture.channel
                ));
            }
            if let Some(offset) = texture.options.transform_offset {
                push_obj_material_texture_vec3(
                    &mut payload,
                    &texture.channel,
                    "transform.offset",
                    offset,
                );
            }
            if let Some(scale) = texture.options.transform_scale {
                push_obj_material_texture_vec3(
                    &mut payload,
                    &texture.channel,
                    "transform.scale",
                    scale,
                );
            }
            if let Some(turbulence) = texture.options.transform_turbulence {
                push_obj_material_texture_vec3(
                    &mut payload,
                    &texture.channel,
                    "transform.turbulence",
                    turbulence,
                );
            }
            if let Some(bump_scale) = texture.options.bump_scale {
                payload.push_str(&format!(
                    "\ntexture.{}.bump_scale={}",
                    texture.channel,
                    canonical_mesh_f32(bump_scale)
                ));
            }
            if let Some(color_remap) = texture.options.color_remap {
                payload.push_str(&format!(
                    "\ntexture.{}.color_remap={},{}",
                    texture.channel,
                    canonical_mesh_f32(color_remap[0]),
                    canonical_mesh_f32(color_remap[1])
                ));
            }
            if let Some(source_channel) = &texture.options.source_channel {
                payload.push_str(&format!(
                    "\ntexture.{}.source_channel={source_channel}",
                    texture.channel
                ));
            }
            if let Some(boost) = texture.options.boost {
                payload.push_str(&format!(
                    "\ntexture.{}.boost={}",
                    texture.channel,
                    canonical_mesh_f32(boost)
                ));
            }
            if let Some(blend_u) = texture.options.blend_u {
                payload.push_str(&format!("\ntexture.{}.blend_u={blend_u}", texture.channel));
            }
            if let Some(blend_v) = texture.options.blend_v {
                payload.push_str(&format!("\ntexture.{}.blend_v={blend_v}", texture.channel));
            }
            if let Some(color_correction) = texture.options.color_correction {
                payload.push_str(&format!(
                    "\ntexture.{}.color_correction={color_correction}",
                    texture.channel
                ));
            }
            if let Some(projection) = &texture.options.projection {
                payload.push_str(&format!(
                    "\ntexture.{}.projection={projection}",
                    texture.channel
                ));
            }
            if let Some(texture_resolution) = texture.options.texture_resolution {
                payload.push_str(&format!(
                    "\ntexture.{}.texture_resolution={texture_resolution}",
                    texture.channel
                ));
            }
        }
        if let Some(ambient_color) = properties.ambient_color {
            push_obj_material_custom_vec3(&mut payload, "ambient_color", ambient_color);
        }
        if let Some(specular_color) = properties.specular_color {
            push_obj_material_custom_vec3(&mut payload, "specular_color", specular_color);
        }
        if let Some(transmission_filter) = properties.transmission_filter {
            push_obj_material_custom_vec3(&mut payload, "transmission_filter", transmission_filter);
        }
        if let Some(index_of_refraction) = properties.index_of_refraction {
            payload.push_str(&format!(
                "\ncustom.index_of_refraction.float={}",
                canonical_mesh_f32(index_of_refraction)
            ));
        }
        if let Some(illumination_model) = properties.illumination_model {
            payload.push_str(&format!(
                "\ncustom.illumination_model.int={illumination_model}"
            ));
        }
        if properties.dissolve_halo {
            payload.push_str("\ncustom.dissolve_halo.bool=true");
        }
        if let Some(sharpness) = properties.sharpness {
            push_obj_material_custom_float(&mut payload, "sharpness", sharpness);
        }
        if let Some(texture_antialias) = properties.texture_antialias {
            payload.push_str(&format!(
                "\ncustom.texture_antialias.bool={texture_antialias}"
            ));
        }
        if let Some(sheen) = properties.sheen {
            push_obj_material_custom_float(&mut payload, "sheen", sheen);
        }
        if let Some(clearcoat) = properties.clearcoat {
            push_obj_material_custom_float(&mut payload, "clearcoat", clearcoat);
        }
        if let Some(clearcoat_roughness) = properties.clearcoat_roughness {
            push_obj_material_custom_float(
                &mut payload,
                "clearcoat_roughness",
                clearcoat_roughness,
            );
        }
        if let Some(anisotropy) = properties.anisotropy {
            push_obj_material_custom_float(&mut payload, "anisotropy", anisotropy);
        }
        if let Some(anisotropy_rotation) = properties.anisotropy_rotation {
            push_obj_material_custom_float(
                &mut payload,
                "anisotropy_rotation",
                anisotropy_rotation,
            );
        }
        if let Some(base_color) = properties.base_color {
            payload.push_str(&format!(
                "\nbase_color={},{},{},{}",
                canonical_mesh_f32(base_color[0]),
                canonical_mesh_f32(base_color[1]),
                canonical_mesh_f32(base_color[2]),
                canonical_mesh_f32(base_color[3])
            ));
        }
        if properties.alpha_blend {
            payload.push_str("\nalpha_mode=blend");
        }
        if let Some(metallic) = properties.metallic {
            payload.push_str(&format!("\nmetallic={}", canonical_mesh_f32(metallic)));
        }
        if let Some(roughness) = properties.roughness {
            payload.push_str(&format!("\nroughness={}", canonical_mesh_f32(roughness)));
        }
        if let Some(emissive) = properties.emissive {
            payload.push_str(&format!(
                "\nemissive={},{},{}",
                canonical_mesh_f32(emissive[0]),
                canonical_mesh_f32(emissive[1]),
                canonical_mesh_f32(emissive[2])
            ));
        }
    }
    payload
}

#[cfg(feature = "model_importer")]
fn push_obj_material_custom_vec3(payload: &mut String, name: &str, value: [f32; 3]) {
    payload.push_str(&format!(
        "\ncustom.{name}.vec3={},{},{}",
        canonical_mesh_f32(value[0]),
        canonical_mesh_f32(value[1]),
        canonical_mesh_f32(value[2])
    ));
}

#[cfg(feature = "model_importer")]
fn push_obj_material_custom_float(payload: &mut String, name: &str, value: f32) {
    payload.push_str(&format!(
        "\ncustom.{name}.float={}",
        canonical_mesh_f32(value)
    ));
}

#[cfg(feature = "model_importer")]
fn push_obj_material_texture_vec3(payload: &mut String, channel: &str, key: &str, value: [f32; 3]) {
    payload.push_str(&format!(
        "\ntexture.{channel}.{key}={},{},{}",
        canonical_mesh_f32(value[0]),
        canonical_mesh_f32(value[1]),
        canonical_mesh_f32(value[2])
    ));
}

#[cfg(feature = "model_importer")]
fn model_local_dependencies(
    subresource: &ModelSubresource,
    subresources: &[ModelSubresource],
    generated_ids: &HashMap<String, AssetId>,
) -> Result<Vec<AssetId>, ImportError> {
    let mut dependencies = Vec::new();
    for dependency_label in &subresource.dependency_labels {
        if dependency_label.label == subresource.label {
            return Err(AssetError::Import {
                message: format!(
                    "model {} `{}` on line {} depends on itself",
                    subresource.kind, subresource.label, subresource.line_number
                ),
            });
        }
        let target = subresources
            .iter()
            .find(|candidate| candidate.label == dependency_label.label)
            .ok_or_else(|| AssetError::Import {
                message: format!(
                    "model {} `{}` on line {} references unknown generated dependency `{}`",
                    subresource.kind,
                    subresource.label,
                    subresource.line_number,
                    dependency_label.label
                ),
            })?;
        if let Some(expected_kind) = dependency_label.expected_kind {
            if target.kind != expected_kind {
                return Err(AssetError::Import {
                    message: format!(
                        "model {} `{}` on line {} dependency `{}` expected generated {expected_kind} but found {} `{}`",
                        subresource.kind,
                        subresource.label,
                        subresource.line_number,
                        dependency_label.label,
                        target.kind,
                        target.label
                    ),
                });
            }
        }
        let dependency = generated_ids[&dependency_label.label];
        if !dependencies.contains(&dependency) {
            dependencies.push(dependency);
        }
    }
    Ok(dependencies)
}

#[cfg(feature = "model_importer")]
fn validate_model_generated_paths(
    source: &SourceAsset,
    subresources: &[ModelSubresource],
) -> Result<(), ImportError> {
    let mut paths: HashMap<AssetPath, (&str, &str, usize)> = HashMap::new();
    for subresource in subresources {
        let extension = model_subresource_extension(&subresource.kind);
        let path = model_generated_path(&source.path, extension, &subresource.label);
        if let Some((existing_kind, existing_label, existing_line)) = paths.get(&path) {
            return Err(AssetError::Import {
                message: format!(
                    "model generated {} `{}` on line {} resolves to duplicate generated path `{}`; first declared by {} `{}` on line {}",
                    subresource.kind,
                    subresource.label,
                    subresource.line_number,
                    path.display_string(),
                    existing_kind,
                    existing_label,
                    existing_line
                ),
            });
        }
        paths.insert(
            path,
            (
                subresource.kind.as_str(),
                subresource.label.as_str(),
                subresource.line_number,
            ),
        );
    }
    Ok(())
}

#[cfg(feature = "model_importer")]
fn model_subresource_extension(kind: &str) -> &'static str {
    match kind {
        "mesh" => "mesh",
        "physics_mesh" => "physics",
        "material" => "material",
        "skeleton" => "skeleton",
        "animation" => "animation",
        _ => unreachable!("model subresource kind validated while parsing manifest"),
    }
}

#[cfg(feature = "model_importer")]
fn model_generated_asset(
    source: &SourceAsset,
    extension: &str,
    label: &str,
    asset_type: AssetTypeId,
    bytes: Vec<u8>,
    dependencies: Vec<AssetId>,
    id: AssetId,
) -> ImportGeneratedAsset {
    ImportGeneratedAsset {
        id,
        path: model_generated_path(&source.path, extension, label),
        asset_type,
        bytes,
        labels: vec![label.to_owned()],
        dependencies,
    }
}

#[cfg(feature = "model_importer")]
fn model_generated_path(source_path: &AssetPath, extension: &str, label: &str) -> AssetPath {
    let path = source_path.without_label().path;
    let stem = path
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(path.as_str());
    AssetPath::new(format!(
        "{stem}.{}.{}",
        sanitize_model_label(label),
        extension
    ))
}

#[cfg(feature = "model_importer")]
fn sanitize_model_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(feature = "model_importer")]
fn model_mesh_payload_bytes(
    payload: &str,
    settings: &ModelImportSettings,
) -> Result<Vec<u8>, ImportError> {
    let mut text = model_payload_text(payload);
    if settings.scale != 1.0 {
        text = scale_model_mesh_payload_text(&text, settings.scale)?;
    }
    if settings.optimize_meshes {
        text = optimize_model_mesh_payload_text(&text)?;
    }
    Ok(text.into_bytes())
}

#[cfg(feature = "model_importer")]
fn model_physics_mesh_payload_bytes(
    physics_mesh: &ModelSubresource,
    settings: &ModelImportSettings,
) -> Result<Vec<u8>, ImportError> {
    let mut text = model_payload_text(&physics_mesh.payload);
    if settings.scale != 1.0 {
        text = scale_model_physics_mesh_payload_text(&text, settings.scale)?;
    }
    crate::assets::physics_mesh::parse_physics_mesh(text.as_bytes()).map_err(|error| {
        AssetError::Import {
            message: format!(
                "model physics_mesh `{}` on line {} payload is invalid: {error}",
                physics_mesh.label, physics_mesh.line_number
            ),
        }
    })?;
    Ok(text.into_bytes())
}

#[cfg(feature = "model_importer")]
fn scale_model_mesh_payload_text(text: &str, scale: f32) -> Result<String, ImportError> {
    let mut scaled = String::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if let Some(values) = trimmed.strip_prefix("v ") {
            let vertex = parse_mesh_source_vertex(values, line_number)?;
            let vertex = scale_model_vertex(vertex, scale);
            scaled.push_str(&format!(
                "v {} {} {}\n",
                canonical_mesh_f32(vertex[0]),
                canonical_mesh_f32(vertex[1]),
                canonical_mesh_f32(vertex[2])
            ));
        } else {
            scaled.push_str(trimmed);
            scaled.push('\n');
        }
    }
    Ok(scaled)
}

#[cfg(feature = "model_importer")]
fn scale_model_physics_mesh_payload_text(text: &str, scale: f32) -> Result<String, ImportError> {
    let mut scaled = String::new();
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        if parts.next() == Some("v") {
            let values = parts.next().unwrap_or("").trim();
            let vertex = parse_mesh_source_vertex(values, line_number).map_err(|error| {
                AssetError::Import {
                    message: format!(
                        "model physics mesh vertex on payload line {line_number} is invalid: {error}"
                    ),
                }
            })?;
            let vertex = scale_model_vertex(vertex, scale);
            scaled.push_str(&format!(
                "v {} {} {}\n",
                canonical_mesh_f32(vertex[0]),
                canonical_mesh_f32(vertex[1]),
                canonical_mesh_f32(vertex[2])
            ));
        } else {
            scaled.push_str(trimmed);
            scaled.push('\n');
        }
    }
    Ok(scaled)
}

#[cfg(feature = "model_importer")]
fn optimize_model_mesh_payload_text(text: &str) -> Result<String, ImportError> {
    let mesh =
        crate::assets::mesh::decode_mesh_for_model_import(text.as_bytes()).map_err(|error| {
            AssetError::Import {
                message: format!("model mesh optimization input is invalid: {error}"),
            }
        })?;
    let mut unique_indices = Vec::new();
    let mut remap = vec![u32::MAX; mesh.vertices.len()];
    let mut referenced_vertices = vec![true; mesh.vertices.len()];
    if !mesh.indices.is_empty() {
        referenced_vertices.fill(false);
        for index in &mesh.indices {
            referenced_vertices[*index as usize] = true;
        }
    }

    for original_index in 0..mesh.vertices.len() {
        if !referenced_vertices[original_index] {
            continue;
        }
        let optimized_index = if let Some(existing_index) =
            unique_indices.iter().position(|candidate_index| {
                model_mesh_vertex_attributes_equal(&mesh, *candidate_index, original_index)
            }) {
            existing_index
        } else {
            let optimized_index = unique_indices.len();
            unique_indices.push(original_index);
            optimized_index
        };
        remap[original_index] = u32::try_from(optimized_index).map_err(|_| AssetError::Import {
            message: "model mesh optimization produced too many vertices".to_owned(),
        })?;
    }

    let mut optimized_indices = Vec::with_capacity(mesh.indices.len());
    for index in &mesh.indices {
        let optimized_index = remap[*index as usize];
        if optimized_index == u32::MAX {
            return Err(AssetError::Import {
                message: format!(
                    "model mesh optimization failed to remap referenced vertex {index}"
                ),
            });
        }
        optimized_indices.push(optimized_index);
    }
    model_mesh_payload_text_from_mesh(&mesh, &unique_indices, &optimized_indices)
}

#[cfg(feature = "model_importer")]
fn model_lod_mesh_payload_text(mesh: &ModelSubresource) -> Result<Option<String>, ImportError> {
    let mesh_asset =
        crate::assets::mesh::decode_mesh_for_model_import(&model_payload_bytes(&mesh.payload))
            .map_err(|error| AssetError::Import {
                message: format!(
                    "model mesh `{}` on line {} LOD input is invalid: {error}",
                    mesh.label, mesh.line_number
                ),
            })?;
    if mesh_asset.indices.len() < 6 {
        return Ok(None);
    }

    let mut source_indices = Vec::new();
    let mut remap = vec![u32::MAX; mesh_asset.vertices.len()];
    let mut lod_indices = Vec::new();
    for (triangle_index, triangle) in mesh_asset.indices.chunks_exact(3).enumerate() {
        if triangle_index % 2 != 0 {
            continue;
        }
        for index in triangle {
            let source_index = *index as usize;
            if remap[source_index] == u32::MAX {
                remap[source_index] =
                    u32::try_from(source_indices.len()).map_err(|_| AssetError::Import {
                        message: format!(
                            "model mesh `{}` on line {} LOD generation produced too many vertices",
                            mesh.label, mesh.line_number
                        ),
                    })?;
                source_indices.push(source_index);
            }
            lod_indices.push(remap[source_index]);
        }
    }

    model_mesh_payload_text_from_mesh(&mesh_asset, &source_indices, &lod_indices).map(Some)
}

#[cfg(feature = "model_importer")]
fn model_mesh_vertex_attributes_equal(mesh: &Mesh, left: usize, right: usize) -> bool {
    mesh.vertices[left] == mesh.vertices[right]
        && model_mesh_optional_attributes_equal(&mesh.normals, left, right)
        && model_mesh_optional_attributes_equal(&mesh.uvs, left, right)
        && mesh
            .uv_sets
            .iter()
            .all(|uv_set| model_mesh_optional_attributes_equal(uv_set, left, right))
        && model_mesh_optional_attributes_equal(&mesh.tangents, left, right)
        && model_mesh_optional_attributes_equal(&mesh.joints, left, right)
        && model_mesh_optional_attributes_equal(&mesh.weights, left, right)
}

#[cfg(feature = "model_importer")]
fn model_mesh_optional_attributes_equal<T: PartialEq>(
    attributes: &[T],
    left: usize,
    right: usize,
) -> bool {
    attributes.is_empty() || attributes[left] == attributes[right]
}

#[cfg(feature = "model_importer")]
fn model_mesh_payload_text_from_mesh(
    mesh: &Mesh,
    source_indices: &[usize],
    indices: &[u32],
) -> Result<String, ImportError> {
    let mut payload = String::new();
    for source_index in source_indices {
        let vertex = mesh.vertices[*source_index];
        payload.push_str(&format!(
            "v {} {} {}\n",
            canonical_mesh_f32(vertex[0]),
            canonical_mesh_f32(vertex[1]),
            canonical_mesh_f32(vertex[2])
        ));
    }
    if !mesh.normals.is_empty() {
        for source_index in source_indices {
            let normal = mesh.normals[*source_index];
            payload.push_str(&format!(
                "n {} {} {}\n",
                canonical_mesh_f32(normal[0]),
                canonical_mesh_f32(normal[1]),
                canonical_mesh_f32(normal[2])
            ));
        }
    }
    if !mesh.uvs.is_empty() {
        for source_index in source_indices {
            let uv = mesh.uvs[*source_index];
            payload.push_str(&format!(
                "uv {} {}\n",
                canonical_mesh_f32(uv[0]),
                canonical_mesh_f32(uv[1])
            ));
        }
    }
    for (uv_set_index, uv_set) in mesh.uv_sets.iter().enumerate() {
        if uv_set.is_empty() {
            continue;
        }
        for source_index in source_indices {
            let uv = uv_set[*source_index];
            payload.push_str(&format!(
                "uv{} {} {}\n",
                uv_set_index + 1,
                canonical_mesh_f32(uv[0]),
                canonical_mesh_f32(uv[1])
            ));
        }
    }
    if !mesh.tangents.is_empty() {
        for source_index in source_indices {
            let tangent = mesh.tangents[*source_index];
            payload.push_str(&format!(
                "t {} {} {} {}\n",
                canonical_mesh_f32(tangent[0]),
                canonical_mesh_f32(tangent[1]),
                canonical_mesh_f32(tangent[2]),
                canonical_mesh_f32(tangent[3])
            ));
        }
    }
    if !mesh.joints.is_empty() {
        for source_index in source_indices {
            let joint = mesh.joints[*source_index];
            payload.push_str(&format!(
                "j {} {} {} {}\n",
                joint[0], joint[1], joint[2], joint[3]
            ));
        }
    }
    if !mesh.weights.is_empty() {
        for source_index in source_indices {
            let weight = mesh.weights[*source_index];
            payload.push_str(&format!(
                "w {} {} {} {}\n",
                canonical_mesh_f32(weight[0]),
                canonical_mesh_f32(weight[1]),
                canonical_mesh_f32(weight[2]),
                canonical_mesh_f32(weight[3])
            ));
        }
    }

    let mut chunks = indices.chunks_exact(3);
    for triangle in &mut chunks {
        payload.push_str(&format!(
            "i {} {} {}\n",
            triangle[0], triangle[1], triangle[2]
        ));
    }
    if !chunks.remainder().is_empty() {
        return Err(AssetError::Import {
            message: "model mesh optimization produced a non-triangle index count".to_owned(),
        });
    }
    Ok(payload)
}

#[cfg(feature = "model_importer")]
fn scale_model_vertex(vertex: [f32; 3], scale: f32) -> [f32; 3] {
    [vertex[0] * scale, vertex[1] * scale, vertex[2] * scale]
}

#[cfg(feature = "model_importer")]
fn model_payload_bytes(payload: &str) -> Vec<u8> {
    model_payload_text(payload).into_bytes()
}

#[cfg(feature = "model_importer")]
fn model_payload_text(payload: &str) -> String {
    let parts = if payload.contains('\n') {
        payload
            .lines()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    } else {
        payload
            .split(';')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    };
    let mut text = parts.join("\n");
    text.push('\n');
    text
}

#[cfg(any(
    feature = "texture_importer",
    feature = "model_importer",
    feature = "material_importer",
    feature = "audio_importer",
    feature = "shader_importer",
    feature = "importers"
))]
fn settings_hash(settings: &ImporterSettings) -> u64 {
    let mut pairs = settings.values.iter().collect::<Vec<_>>();
    pairs.sort_by(|left, right| left.0.cmp(right.0));
    let mut bytes = Vec::new();
    for (key, value) in pairs {
        bytes.extend_from_slice(key.as_bytes());
        bytes.push(b'=');
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(b'\n');
    }
    crate::io::stable_hash(&bytes)
}
