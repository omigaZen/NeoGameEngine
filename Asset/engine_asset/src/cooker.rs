use crate::{
    error::CookError,
    id::{AssetId, AssetTypeId, ContentHash, VersionHash},
    metadata::AssetMetadata,
    path::AssetPath,
};

#[cfg(any(
    feature = "texture_cooker",
    feature = "model_cooker",
    feature = "material_cooker",
    feature = "audio_cooker",
    feature = "shader_cooker",
    feature = "cookers"
))]
use crate::asset::Asset;
#[cfg(feature = "audio_cooker")]
use crate::assets::AudioClip;
#[cfg(feature = "material_cooker")]
use crate::assets::Material;
#[cfg(feature = "shader_cooker")]
use crate::assets::Shader;
#[cfg(feature = "texture_cooker")]
use crate::assets::Texture;
#[cfg(feature = "model_cooker")]
use crate::assets::{AnimationClip, Mesh, Skeleton};
#[cfg(feature = "cookers")]
use crate::assets::{Font, PhysicsMesh};
#[cfg(feature = "cookers")]
use crate::assets::{Prefab, SceneAsset};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    Windows,
    MacOs,
    Linux,
    Android,
    Ios,
    Web,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookContext {
    pub target: TargetPlatform,
    pub source_path: Option<AssetPath>,
    pub source_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookOutput {
    pub id: AssetId,
    pub bytes: Vec<u8>,
    pub content_hash: ContentHash,
    pub version_hash: VersionHash,
    pub metadata: AssetMetadata,
}

pub trait AssetCooker: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn version(&self) -> u32;
    fn asset_type(&self) -> AssetTypeId;
    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError>;
}

#[derive(Default)]
pub struct CookerRegistry {
    cookers: Vec<Box<dyn AssetCooker>>,
}

impl CookerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<C: AssetCooker>(&mut self, cooker: C) {
        self.cookers.push(Box::new(cooker));
    }

    pub fn cooker_for_type(&self, asset_type: AssetTypeId) -> Option<&dyn AssetCooker> {
        self.cookers
            .iter()
            .map(Box::as_ref)
            .find(|cooker| cooker.asset_type() == asset_type)
    }
}

#[cfg(feature = "texture_cooker")]
pub struct TextureCooker;

#[cfg(feature = "texture_cooker")]
impl TextureCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "texture_cooker")]
impl Default for TextureCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "texture_cooker")]
impl AssetCooker for TextureCooker {
    fn name(&self) -> &'static str {
        "TextureCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        Texture::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "TextureCooker requires source bytes".to_owned(),
            });
        }
        let bytes = if ctx.source_bytes.starts_with(b"NGA_TEXTURE_SOURCE_V1") {
            let text = std::str::from_utf8(&ctx.source_bytes).map_err(|error| CookError::Cook {
                message: format!("TextureCooker failed to canonicalize texture source: {error}"),
            })?;
            crate::assets::texture::canonical_texture_source_document(text).map_err(|error| {
                CookError::Cook {
                    message: format!(
                        "TextureCooker failed to canonicalize texture source: {error}"
                    ),
                }
            })?
        } else {
            crate::assets::texture::canonical_texture_runtime_bytes(&ctx.source_bytes).map_err(
                |error| CookError::Cook {
                    message: format!(
                        "TextureCooker failed to canonicalize texture source: {error}"
                    ),
                },
            )?
        };
        Ok(CookOutput {
            id: metadata.id,
            bytes: bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}
#[cfg(feature = "model_cooker")]
pub struct MeshCooker;

#[cfg(feature = "model_cooker")]
impl MeshCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "model_cooker")]
impl Default for MeshCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "model_cooker")]
impl AssetCooker for MeshCooker {
    fn name(&self) -> &'static str {
        "MeshCooker"
    }

    fn version(&self) -> u32 {
        4
    }

    fn asset_type(&self) -> AssetTypeId {
        Mesh::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "MeshCooker requires source bytes".to_owned(),
            });
        }
        let mesh = crate::assets::mesh::decode_mesh(&ctx.source_bytes).map_err(|error| {
            CookError::Cook {
                message: format!("MeshCooker failed to decode mesh source: {error}"),
            }
        })?;
        let mesh = mesh_for_cook_target(&mesh, ctx.target);
        let index_encoding = mesh_index_encoding_for_target(&mesh, ctx.target);
        let bytes = match index_encoding {
            crate::assets::mesh::MeshBinaryIndexEncoding::U32 => {
                crate::assets::mesh::encode_binary_mesh(&mesh)
            }
            crate::assets::mesh::MeshBinaryIndexEncoding::U16 => {
                crate::assets::mesh::encode_binary_mesh_with_index_encoding(&mesh, index_encoding)
            }
        }
        .map_err(|error| CookError::Cook {
            message: format!("MeshCooker failed to encode binary mesh: {error}"),
        })?;
        Ok(CookOutput {
            id: metadata.id,
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
            bytes,
        })
    }
}

#[cfg(feature = "model_cooker")]
fn mesh_for_cook_target(mesh: &Mesh, target: TargetPlatform) -> Mesh {
    if mesh.indices.is_empty() || !mesh_target_compacts_vertices(target) {
        return mesh.clone();
    }
    compact_mesh_vertices(mesh)
}

#[cfg(feature = "model_cooker")]
fn mesh_target_compacts_vertices(target: TargetPlatform) -> bool {
    matches!(
        target,
        TargetPlatform::Android | TargetPlatform::Ios | TargetPlatform::Web
    )
}

#[cfg(feature = "model_cooker")]
fn mesh_index_encoding_for_target(
    mesh: &Mesh,
    target: TargetPlatform,
) -> crate::assets::mesh::MeshBinaryIndexEncoding {
    let mobile_or_web = mesh_target_compacts_vertices(target);
    if mobile_or_web && mesh.indices.iter().all(|index| *index <= u16::MAX as u32) {
        crate::assets::mesh::MeshBinaryIndexEncoding::U16
    } else {
        crate::assets::mesh::MeshBinaryIndexEncoding::U32
    }
}

#[cfg(feature = "model_cooker")]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MeshCookVertexKey {
    position: [u32; 3],
    normal: Option<[u32; 3]>,
    uv: Option<[u32; 2]>,
    secondary_uvs: Vec<Option<[u32; 2]>>,
    tangent: Option<[u32; 4]>,
    joints: Option<[u16; 4]>,
    weights: Option<[u32; 4]>,
}

#[cfg(feature = "model_cooker")]
fn compact_mesh_vertices(mesh: &Mesh) -> Mesh {
    let mut compacted = mesh.clone();
    compacted.vertices.clear();
    compacted.normals.clear();
    compacted.uvs.clear();
    compacted.uv_sets = mesh.uv_sets.iter().map(|_| Vec::new()).collect();
    compacted.tangents.clear();
    compacted.joints.clear();
    compacted.weights.clear();
    compacted.indices.clear();

    let mut remapped = std::collections::HashMap::<MeshCookVertexKey, u32>::new();
    for source_index in &mesh.indices {
        let source_index = *source_index as usize;
        let key = mesh_cook_vertex_key(mesh, source_index);
        let compacted_index = if let Some(compacted_index) = remapped.get(&key) {
            *compacted_index
        } else {
            let compacted_index = compacted.vertices.len() as u32;
            push_compacted_mesh_vertex(&mut compacted, mesh, source_index);
            remapped.insert(key, compacted_index);
            compacted_index
        };
        compacted.indices.push(compacted_index);
    }

    compacted
}

#[cfg(feature = "model_cooker")]
fn mesh_cook_vertex_key(mesh: &Mesh, index: usize) -> MeshCookVertexKey {
    MeshCookVertexKey {
        position: mesh_cook_f32_bits(mesh.vertices[index]),
        normal: mesh.normals.get(index).copied().map(mesh_cook_f32_bits),
        uv: mesh.uvs.get(index).copied().map(mesh_cook_f32_bits),
        secondary_uvs: mesh
            .uv_sets
            .iter()
            .map(|uv_set| uv_set.get(index).copied().map(mesh_cook_f32_bits))
            .collect(),
        tangent: mesh.tangents.get(index).copied().map(mesh_cook_f32_bits),
        joints: mesh.joints.get(index).copied(),
        weights: mesh.weights.get(index).copied().map(mesh_cook_f32_bits),
    }
}

#[cfg(feature = "model_cooker")]
fn mesh_cook_f32_bits<const N: usize>(values: [f32; N]) -> [u32; N] {
    values.map(mesh_cook_f32_bit)
}

#[cfg(feature = "model_cooker")]
fn mesh_cook_f32_bit(value: f32) -> u32 {
    if value == 0.0 {
        0.0f32.to_bits()
    } else {
        value.to_bits()
    }
}

#[cfg(feature = "model_cooker")]
fn push_compacted_mesh_vertex(compacted: &mut Mesh, source: &Mesh, index: usize) {
    compacted.vertices.push(source.vertices[index]);
    if !source.normals.is_empty() {
        compacted.normals.push(source.normals[index]);
    }
    if !source.uvs.is_empty() {
        compacted.uvs.push(source.uvs[index]);
    }
    for (compacted_uvs, source_uvs) in compacted.uv_sets.iter_mut().zip(&source.uv_sets) {
        if !source_uvs.is_empty() {
            compacted_uvs.push(source_uvs[index]);
        }
    }
    if !source.tangents.is_empty() {
        compacted.tangents.push(source.tangents[index]);
    }
    if !source.joints.is_empty() {
        compacted.joints.push(source.joints[index]);
    }
    if !source.weights.is_empty() {
        compacted.weights.push(source.weights[index]);
    }
}
#[cfg(feature = "material_cooker")]
pub struct MaterialCooker;

#[cfg(feature = "material_cooker")]
impl MaterialCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "material_cooker")]
impl Default for MaterialCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "material_cooker")]
impl AssetCooker for MaterialCooker {
    fn name(&self) -> &'static str {
        "MaterialCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        Material::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "MaterialCooker requires source bytes".to_owned(),
            });
        }
        let bytes = crate::assets::material::canonical_material_runtime_bytes(&ctx.source_bytes)
            .map_err(|error| CookError::Cook {
                message: format!("MaterialCooker failed to canonicalize material source: {error}"),
            })?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}
#[cfg(feature = "shader_cooker")]
pub struct ShaderCooker;

#[cfg(feature = "shader_cooker")]
impl ShaderCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "shader_cooker")]
impl Default for ShaderCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "shader_cooker")]
impl AssetCooker for ShaderCooker {
    fn name(&self) -> &'static str {
        "ShaderCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        Shader::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "ShaderCooker requires source bytes".to_owned(),
            });
        }
        let bytes = crate::assets::shader::canonical_shader_runtime_bytes(&ctx.source_bytes)
            .map_err(|error| CookError::Cook {
                message: format!("ShaderCooker failed to canonicalize shader source: {error}"),
            })?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}
#[cfg(feature = "audio_cooker")]
pub struct AudioCooker;

#[cfg(feature = "audio_cooker")]
impl AudioCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "audio_cooker")]
impl Default for AudioCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "audio_cooker")]
impl AssetCooker for AudioCooker {
    fn name(&self) -> &'static str {
        "AudioCooker"
    }

    fn version(&self) -> u32 {
        8
    }

    fn asset_type(&self) -> AssetTypeId {
        AudioClip::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "AudioCooker requires source bytes".to_owned(),
            });
        }
        let bytes = crate::assets::audio::canonical_audio_runtime_bytes(&ctx.source_bytes)
            .map_err(|error| CookError::Cook {
                message: format!("AudioCooker failed to canonicalize audio source: {error}"),
            })?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}
#[cfg(feature = "model_cooker")]
pub struct SkeletonCooker;

#[cfg(feature = "model_cooker")]
impl SkeletonCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "model_cooker")]
impl Default for SkeletonCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "model_cooker")]
impl AssetCooker for SkeletonCooker {
    fn name(&self) -> &'static str {
        "SkeletonCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        Skeleton::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "SkeletonCooker requires source bytes".to_owned(),
            });
        }
        let bytes = canonical_skeleton_cook_bytes(&ctx.source_bytes)?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}

#[cfg(feature = "model_cooker")]
pub struct AnimationCooker;

#[cfg(feature = "model_cooker")]
impl AnimationCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "model_cooker")]
impl Default for AnimationCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "model_cooker")]
impl AssetCooker for AnimationCooker {
    fn name(&self) -> &'static str {
        "AnimationCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        AnimationClip::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "AnimationCooker requires source bytes".to_owned(),
            });
        }
        let bytes = canonical_animation_cook_bytes(&ctx.source_bytes)?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}

#[cfg(feature = "model_cooker")]
fn canonical_skeleton_cook_bytes(bytes: &[u8]) -> Result<Vec<u8>, CookError> {
    let text = std::str::from_utf8(bytes).map_err(|error| CookError::Cook {
        message: format!("SkeletonCooker failed to canonicalize skeleton source: {error}"),
    })?;
    let header = text.lines().next().unwrap_or("").trim();
    let cooked = match header {
        "NGA_SKELETON_SOURCE_V1" => text
            .replacen("NGA_SKELETON_SOURCE_V1", "NGA_SKELETON_V1", 1)
            .into_bytes(),
        "NGA_SKELETON_V1" => bytes.to_vec(),
        _ => {
            return Err(CookError::Cook {
                message: "SkeletonCooker source must start with NGA_SKELETON_V1 or NGA_SKELETON_SOURCE_V1"
                    .to_owned(),
            });
        }
    };
    crate::assets::skeleton::parse_skeleton(&cooked).map_err(|error| CookError::Cook {
        message: format!("SkeletonCooker failed to validate skeleton source: {error}"),
    })?;
    Ok(cooked)
}

#[cfg(feature = "model_cooker")]
fn canonical_animation_cook_bytes(bytes: &[u8]) -> Result<Vec<u8>, CookError> {
    let text = std::str::from_utf8(bytes).map_err(|error| CookError::Cook {
        message: format!("AnimationCooker failed to canonicalize animation source: {error}"),
    })?;
    let header = text.lines().next().unwrap_or("").trim();
    let cooked = match header {
        "NGA_ANIMATION_SOURCE_V1" => text
            .replacen("NGA_ANIMATION_SOURCE_V1", "NGA_ANIMATION_V1", 1)
            .into_bytes(),
        "NGA_ANIMATION_V1" => bytes.to_vec(),
        _ => {
            return Err(CookError::Cook {
                message: "AnimationCooker source must start with NGA_ANIMATION_V1 or NGA_ANIMATION_SOURCE_V1"
                    .to_owned(),
            });
        }
    };
    crate::assets::animation::parse_animation_clip(&cooked).map_err(|error| CookError::Cook {
        message: format!("AnimationCooker failed to validate animation source: {error}"),
    })?;
    Ok(cooked)
}

#[cfg(feature = "cookers")]
pub struct SceneCooker;

#[cfg(feature = "cookers")]
impl SceneCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "cookers")]
impl Default for SceneCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "cookers")]
impl AssetCooker for SceneCooker {
    fn name(&self) -> &'static str {
        "SceneCooker"
    }

    fn version(&self) -> u32 {
        1
    }

    fn asset_type(&self) -> AssetTypeId {
        SceneAsset::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "SceneCooker requires source bytes".to_owned(),
            });
        }
        validate_scene_cook_bytes(&ctx.source_bytes).map_err(|error| CookError::Cook {
            message: format!("SceneCooker failed to validate scene source: {error}"),
        })?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: ctx.source_bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&ctx.source_bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}

#[cfg(feature = "cookers")]
pub struct PrefabCooker;

#[cfg(feature = "cookers")]
impl PrefabCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "cookers")]
impl Default for PrefabCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "cookers")]
impl AssetCooker for PrefabCooker {
    fn name(&self) -> &'static str {
        "PrefabCooker"
    }

    fn version(&self) -> u32 {
        1
    }

    fn asset_type(&self) -> AssetTypeId {
        Prefab::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "PrefabCooker requires source bytes".to_owned(),
            });
        }
        validate_prefab_cook_bytes(&ctx.source_bytes).map_err(|error| CookError::Cook {
            message: format!("PrefabCooker failed to validate prefab source: {error}"),
        })?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: ctx.source_bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&ctx.source_bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}

#[cfg(feature = "cookers")]
fn validate_scene_cook_bytes(bytes: &[u8]) -> Result<(), CookError> {
    let source = std::str::from_utf8(bytes).map_err(|error| CookError::Decode {
        message: format!("scene source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_SCENE_V1" {
        return Err(CookError::Decode {
            message: "scene source must start with NGA_SCENE_V1".to_owned(),
        });
    }

    let mut name = None;
    let mut entities = Vec::new();
    let mut dependency_keys = Vec::new();
    let mut current_entity = None;

    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(CookError::Decode {
                message: format!("invalid scene line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match crate::assets::scene::scene_prefab_document_key(key).as_str() {
            "name" | "scenename" => {
                if value.is_empty() {
                    return Err(CookError::Decode {
                        message: format!("scene name is empty on line {line_number}"),
                    });
                }
                name = Some(value.to_owned());
            }
            key if crate::assets::scene::is_scene_prefab_dependency_key(key) => {
                let path = AssetPath::parse(value);
                let asset_type =
                    crate::assets::scene::dependency_type_for_path(&path, line_number, "scene")?;
                register_scene_prefab_cook_dependency(&mut dependency_keys, path, asset_type);
            }
            "entity" | "node" | "gameobject" => {
                let entity =
                    crate::assets::scene::parse_serialized_entity(value, line_number, "scene")?;
                entities.push(entity);
                current_entity = Some(entities.len() - 1);
            }
            "component" | "cmp" => {
                let Some(entity_index) = current_entity else {
                    return Err(CookError::Decode {
                        message: format!("scene component on line {line_number} has no entity"),
                    });
                };
                let component =
                    crate::assets::scene::parse_serialized_component(value, line_number, "scene")?;
                for (path, asset_type) in
                    crate::assets::scene::serialized_component_asset_dependencies(
                        &component,
                        line_number,
                        "scene",
                    )?
                {
                    register_scene_prefab_cook_dependency(&mut dependency_keys, path, asset_type);
                }
                entities[entity_index].components.push(component);
            }
            _ => {
                return Err(CookError::Decode {
                    message: format!("unknown scene key `{key}` on line {line_number}"),
                });
            }
        }
    }

    name.ok_or_else(|| CookError::Decode {
        message: "scene source missing name".to_owned(),
    })?;
    Ok(())
}

#[cfg(feature = "cookers")]
fn validate_prefab_cook_bytes(bytes: &[u8]) -> Result<(), CookError> {
    let source = std::str::from_utf8(bytes).map_err(|error| CookError::Decode {
        message: format!("prefab source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_PREFAB_V1" {
        return Err(CookError::Decode {
            message: "prefab source must start with NGA_PREFAB_V1".to_owned(),
        });
    }

    #[derive(Clone, Copy)]
    enum PrefabEntityTarget {
        Root,
        Child(usize),
    }

    let mut root = None;
    let mut children = Vec::new();
    let mut dependency_keys = Vec::new();
    let mut current_entity = None;

    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(CookError::Decode {
                message: format!("invalid prefab line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match crate::assets::scene::scene_prefab_document_key(key).as_str() {
            key if crate::assets::scene::is_scene_prefab_dependency_key(key) => {
                let path = AssetPath::parse(value);
                let asset_type =
                    crate::assets::scene::dependency_type_for_path(&path, line_number, "prefab")?;
                register_scene_prefab_cook_dependency(&mut dependency_keys, path, asset_type);
            }
            "root" | "rootentity" | "rootnode" => {
                if root.is_some() {
                    return Err(CookError::Decode {
                        message: format!("duplicate prefab root on line {line_number}"),
                    });
                }
                let root_entity =
                    crate::assets::scene::parse_serialized_entity(value, line_number, "prefab")?;
                if root_entity.parent.is_some() {
                    return Err(CookError::Decode {
                        message: format!("prefab root cannot have parent on line {line_number}"),
                    });
                }
                root = Some(root_entity);
                current_entity = Some(PrefabEntityTarget::Root);
            }
            "child" | "childentity" | "childnode" => {
                if root.is_none() {
                    return Err(CookError::Decode {
                        message: format!("prefab child on line {line_number} has no root"),
                    });
                }
                let child =
                    crate::assets::scene::parse_serialized_entity(value, line_number, "prefab")?;
                children.push(child);
                current_entity = Some(PrefabEntityTarget::Child(children.len() - 1));
            }
            "component" | "cmp" => {
                let component =
                    crate::assets::scene::parse_serialized_component(value, line_number, "prefab")?;
                for (path, asset_type) in
                    crate::assets::scene::serialized_component_asset_dependencies(
                        &component,
                        line_number,
                        "prefab",
                    )?
                {
                    register_scene_prefab_cook_dependency(&mut dependency_keys, path, asset_type);
                }
                match current_entity {
                    Some(PrefabEntityTarget::Root) => {
                        if let Some(root) = root.as_mut() {
                            root.components.push(component);
                        }
                    }
                    Some(PrefabEntityTarget::Child(index)) => {
                        children[index].components.push(component);
                    }
                    None => {
                        return Err(CookError::Decode {
                            message: format!(
                                "prefab component on line {line_number} has no entity"
                            ),
                        });
                    }
                }
            }
            _ => {
                return Err(CookError::Decode {
                    message: format!("unknown prefab key `{key}` on line {line_number}"),
                });
            }
        }
    }

    root.ok_or_else(|| CookError::Decode {
        message: "prefab source missing root".to_owned(),
    })?;
    Ok(())
}

#[cfg(feature = "cookers")]
fn register_scene_prefab_cook_dependency(
    dependency_keys: &mut Vec<(AssetPath, AssetTypeId)>,
    path: AssetPath,
    asset_type: AssetTypeId,
) {
    if dependency_keys
        .iter()
        .any(|(existing_path, existing_type)| {
            existing_path == &path && *existing_type == asset_type
        })
    {
        return;
    }
    dependency_keys.push((path, asset_type));
}
#[cfg(feature = "cookers")]
pub struct FontCooker;

#[cfg(feature = "cookers")]
impl FontCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "cookers")]
impl Default for FontCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "cookers")]
impl AssetCooker for FontCooker {
    fn name(&self) -> &'static str {
        "FontCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        Font::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "FontCooker requires source bytes".to_owned(),
            });
        }
        let source_path = ctx
            .source_path
            .as_ref()
            .or(metadata.source_path.as_ref())
            .or(metadata.path.as_ref())
            .ok_or_else(|| CookError::Cook {
                message: "FontCooker requires source path metadata".to_owned(),
            })?;
        crate::assets::font::parse_font_from_path(source_path, &ctx.source_bytes).map_err(
            |error| CookError::Cook {
                message: format!("FontCooker failed to validate font source: {error}"),
            },
        )?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: ctx.source_bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&ctx.source_bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}

#[cfg(feature = "cookers")]
pub struct PhysicsMeshCooker;

#[cfg(feature = "cookers")]
impl PhysicsMeshCooker {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "cookers")]
impl Default for PhysicsMeshCooker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "cookers")]
impl AssetCooker for PhysicsMeshCooker {
    fn name(&self) -> &'static str {
        "PhysicsMeshCooker"
    }

    fn version(&self) -> u32 {
        2
    }

    fn asset_type(&self) -> AssetTypeId {
        PhysicsMesh::TYPE_ID
    }

    fn cook(&self, ctx: &CookContext, metadata: &AssetMetadata) -> Result<CookOutput, CookError> {
        if ctx.source_bytes.is_empty() {
            return Err(CookError::Cook {
                message: "PhysicsMeshCooker requires source bytes".to_owned(),
            });
        }
        crate::assets::physics_mesh::parse_physics_mesh(&ctx.source_bytes).map_err(|error| {
            CookError::Cook {
                message: format!(
                    "PhysicsMeshCooker failed to validate physics mesh source: {error}"
                ),
            }
        })?;
        Ok(CookOutput {
            id: metadata.id,
            bytes: ctx.source_bytes.clone(),
            content_hash: ContentHash(crate::io::stable_hash(&ctx.source_bytes)),
            version_hash: VersionHash(self.version() as u64),
            metadata: metadata.clone(),
        })
    }
}
