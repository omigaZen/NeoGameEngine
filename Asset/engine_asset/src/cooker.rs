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
define_passthrough_cooker!(TextureCooker, Texture, 1);
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
define_passthrough_cooker!(MaterialCooker, Material, 1);
#[cfg(feature = "shader_cooker")]
define_passthrough_cooker!(ShaderCooker, Shader, 1);
#[cfg(feature = "audio_cooker")]
define_passthrough_cooker!(AudioCooker, AudioClip, 1);
#[cfg(feature = "model_cooker")]
define_passthrough_cooker!(SkeletonCooker, Skeleton, 1);
#[cfg(feature = "model_cooker")]
define_passthrough_cooker!(AnimationCooker, AnimationClip, 1);
#[cfg(feature = "cookers")]
define_passthrough_cooker!(FontCooker, Font, 1);
#[cfg(feature = "cookers")]
define_passthrough_cooker!(PhysicsMeshCooker, PhysicsMesh, 1);
