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

#[cfg(any(
    feature = "texture_cooker",
    feature = "model_cooker",
    feature = "material_cooker",
    feature = "audio_cooker",
    feature = "shader_cooker",
    feature = "cookers"
))]
macro_rules! define_passthrough_cooker {
    ($name:ident, $asset:ty, $version:expr) => {
        pub struct $name;

        impl $name {
            pub fn new() -> Self {
                Self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl AssetCooker for $name {
            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn version(&self) -> u32 {
                $version
            }

            fn asset_type(&self) -> AssetTypeId {
                <$asset>::TYPE_ID
            }

            fn cook(
                &self,
                ctx: &CookContext,
                metadata: &AssetMetadata,
            ) -> Result<CookOutput, CookError> {
                if ctx.source_bytes.is_empty() {
                    return Err(CookError::Cook {
                        message: format!("{} requires source bytes", self.name()),
                    });
                }
                Ok(CookOutput {
                    id: metadata.id,
                    bytes: ctx.source_bytes.clone(),
                    content_hash: ContentHash(crate::io::stable_hash(&ctx.source_bytes)),
                    version_hash: VersionHash(self.version() as u64),
                    metadata: metadata.clone(),
                })
            }
        }
    };
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
define_passthrough_cooker!(SceneCooker, SceneAsset, 1);
#[cfg(feature = "cookers")]
define_passthrough_cooker!(PrefabCooker, Prefab, 1);
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
define_passthrough_cooker!(PhysicsMeshCooker, PhysicsMesh, 1);
