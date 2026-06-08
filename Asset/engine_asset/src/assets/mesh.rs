use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    gpu_upload::{
        GpuResourceHandle, GpuUploadMetadata, MeshIndexFormat, MeshUploadMetadata,
        MeshVertexAttribute, MeshVertexBuffer, MeshVertexFormat, MeshVertexLayout,
        MeshVertexSemantic,
    },
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

pub(crate) const MESH_BINARY_MAGIC: &[u8] = b"NGA_MESH_BINARY_V1\n";

const MESH_BINARY_FLAG_NORMALS: u32 = 1 << 0;
const MESH_BINARY_FLAG_UVS: u32 = 1 << 1;
const MESH_BINARY_FLAG_TANGENTS: u32 = 1 << 2;
const MESH_BINARY_FLAG_SKINNING: u32 = 1 << 3;
const MESH_BINARY_FLAG_INDEX_U16: u32 = 1 << 4;
const MESH_BINARY_SUPPORTED_FLAGS: u32 = MESH_BINARY_FLAG_NORMALS
    | MESH_BINARY_FLAG_UVS
    | MESH_BINARY_FLAG_TANGENTS
    | MESH_BINARY_FLAG_SKINNING
    | MESH_BINARY_FLAG_INDEX_U16;
const SKIN_WEIGHT_SUM_EPSILON: f32 = 0.001;

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub uv_sets: Vec<Vec<[f32; 2]>>,
    pub tangents: Vec<[f32; 4]>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    pub index_format: MeshIndexFormat,
    pub vertex_buffer: MeshVertexBuffer,
    pub gpu: Option<GpuResourceHandle>,
}

impl Asset for Mesh {
    const TYPE_NAME: &'static str = "Mesh";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0002);
}

impl AssetMemoryUsage for Mesh {
    fn cpu_bytes(&self) -> u64 {
        (self.vertices.len() * std::mem::size_of::<[f32; 3]>()
            + self.normals.len() * std::mem::size_of::<[f32; 3]>()
            + self.uvs.len() * std::mem::size_of::<[f32; 2]>()
            + self
                .uv_sets
                .iter()
                .map(|uvs| uvs.len() * std::mem::size_of::<[f32; 2]>())
                .sum::<usize>()
            + self.tangents.len() * std::mem::size_of::<[f32; 4]>()
            + self.joints.len() * std::mem::size_of::<[u16; 4]>()
            + self.weights.len() * std::mem::size_of::<[f32; 4]>()
            + self.vertex_buffer.bytes.len()
            + self.indices.len() * std::mem::size_of::<u32>()) as u64
    }

    fn gpu_bytes(&self) -> u64 {
        self.vertex_buffer.bytes.len() as u64
            + self.indices.len() as u64 * u64::from(self.index_format.byte_size())
    }
}

pub struct MeshLoader;

impl MeshLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MeshLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for MeshLoader {
    fn name(&self) -> &'static str {
        "MeshLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["mesh"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Mesh::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        let mesh = decode_mesh(bytes)?;
        let upload_metadata = mesh_upload_metadata(&mesh);
        let upload_bytes = mesh_upload_bytes(&mesh);
        Ok(LoadedAsset::new(mesh).mesh_upload_with_metadata(
            ctx.id(),
            Mesh::TYPE_ID,
            Some(ctx.path().display_string()),
            upload_metadata,
            upload_bytes,
        ))
    }
}

pub(crate) fn decode_mesh(bytes: &[u8]) -> Result<Mesh, AssetError> {
    decode_mesh_with_options(
        bytes,
        MeshDecodeOptions {
            validate_skin_weight_totals: true,
        },
    )
}

#[cfg(feature = "model_importer")]
pub(crate) fn decode_mesh_for_model_import(bytes: &[u8]) -> Result<Mesh, AssetError> {
    decode_mesh_with_options(
        bytes,
        MeshDecodeOptions {
            validate_skin_weight_totals: false,
        },
    )
}

#[derive(Clone, Copy)]
struct MeshDecodeOptions {
    validate_skin_weight_totals: bool,
}

fn decode_mesh_with_options(bytes: &[u8], options: MeshDecodeOptions) -> Result<Mesh, AssetError> {
    if let Some(payload) = bytes.strip_prefix(MESH_BINARY_MAGIC) {
        return decode_binary_mesh(payload, options);
    }

    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("mesh source must be UTF-8: {error}"),
    })?;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut uv_sets = Vec::<Vec<[f32; 2]>>::new();
    let mut tangents = Vec::new();
    let mut joints = Vec::new();
    let mut weights = Vec::new();
    let mut indices = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let values = parse_f32_triplet(parts, line_index, "vertex")?;
                vertices.push(values);
            }
            Some("n") => {
                let values = parse_f32_triplet(parts, line_index, "normal")?;
                normals.push(values);
            }
            Some("uv") => {
                let values = parse_f32_pair(parts, line_index, "uv")?;
                uvs.push(values);
            }
            Some(directive) if mesh_uv_set_directive_index(directive).is_some() => {
                let set_index = mesh_uv_set_directive_index(directive)
                    .expect("directive matched a mesh uv set");
                let values = parse_f32_pair(parts, line_index, "uv set")?;
                ensure_uv_set(&mut uv_sets, set_index).push(values);
            }
            Some("t") => {
                let values = parse_f32_quad(parts, line_index, "tangent")?;
                tangents.push(values);
            }
            Some("j") | Some("joints") => {
                let values = parse_u16_quad(parts, line_index, "joint")?;
                joints.push(values);
            }
            Some("w") | Some("weights") => {
                let values = parse_f32_quad(parts, line_index, "weight")?;
                validate_skin_weights(values, line_index, options.validate_skin_weight_totals)?;
                weights.push(values);
            }
            Some("i") => {
                let values = parse_u32_triplet(parts, line_index)?;
                indices.extend_from_slice(&values);
            }
            Some(other) => {
                return Err(AssetError::Decode {
                    message: format!(
                        "unknown mesh directive `{other}` on line {}",
                        line_index + 1
                    ),
                });
            }
            None => {}
        }
    }
    mesh_from_components(
        vertices,
        normals,
        uvs,
        uv_sets,
        tangents,
        joints,
        weights,
        indices,
        MeshIndexFormat::Uint32,
    )
}

fn mesh_from_components(
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    uv_sets: Vec<Vec<[f32; 2]>>,
    tangents: Vec<[f32; 4]>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    indices: Vec<u32>,
    index_format: MeshIndexFormat,
) -> Result<Mesh, AssetError> {
    if !uv_sets.is_empty() && uvs.is_empty() {
        return Err(AssetError::Decode {
            message: "mesh secondary uv sets require primary uv coordinates".to_owned(),
        });
    }
    validate_uv_sets(&uv_sets, vertices.len())?;
    validate_skinning_counts(&joints, &weights, vertices.len())?;
    if vertices.is_empty() {
        return Err(AssetError::Decode {
            message: "mesh must contain at least one vertex".to_owned(),
        });
    }
    if !normals.is_empty() && normals.len() != vertices.len() {
        return Err(AssetError::Decode {
            message: format!(
                "mesh normal count {} must match vertex count {}",
                normals.len(),
                vertices.len()
            ),
        });
    }
    if !uvs.is_empty() && uvs.len() != vertices.len() {
        return Err(AssetError::Decode {
            message: format!(
                "mesh uv count {} must match vertex count {}",
                uvs.len(),
                vertices.len()
            ),
        });
    }
    if !tangents.is_empty() && tangents.len() != vertices.len() {
        return Err(AssetError::Decode {
            message: format!(
                "mesh tangent count {} must match vertex count {}",
                tangents.len(),
                vertices.len()
            ),
        });
    }
    for index in &indices {
        if *index as usize >= vertices.len() {
            return Err(AssetError::Decode {
                message: format!(
                    "mesh index {index} references missing vertex; vertex count is {}",
                    vertices.len()
                ),
            });
        }
    }
    let vertex_buffer = build_mesh_vertex_buffer(
        &vertices, &normals, &uvs, &uv_sets, &tangents, &joints, &weights,
    )?;
    Ok(Mesh {
        vertices,
        normals,
        uvs,
        uv_sets,
        tangents,
        joints,
        weights,
        indices,
        index_format,
        vertex_buffer,
        gpu: None,
    })
}

fn decode_binary_mesh(payload: &[u8], options: MeshDecodeOptions) -> Result<Mesh, AssetError> {
    let mut reader = MeshBinaryReader::new(payload);
    let vertex_count = reader.read_u32("vertex_count")?;
    let index_count = reader.read_u32("index_count")?;
    let flags = reader.read_u32("flags")?;
    let secondary_uv_mask = reader.read_u32("secondary_uv_mask")?;
    if flags & !MESH_BINARY_SUPPORTED_FLAGS != 0 {
        return Err(AssetError::Decode {
            message: format!(
                "mesh binary payload has unsupported flags 0x{:x}",
                flags & !MESH_BINARY_SUPPORTED_FLAGS
            ),
        });
    }
    if secondary_uv_mask != 0 && flags & MESH_BINARY_FLAG_UVS == 0 {
        return Err(AssetError::Decode {
            message: "mesh binary secondary uv sets require primary uv coordinates".to_owned(),
        });
    }
    if index_count % 3 != 0 {
        return Err(AssetError::Decode {
            message: format!("mesh binary index count {index_count} must be divisible by 3"),
        });
    }

    let vertex_count = usize::try_from(vertex_count).map_err(|_| AssetError::Decode {
        message: "mesh binary vertex count does not fit this platform".to_owned(),
    })?;
    let index_count = usize::try_from(index_count).map_err(|_| AssetError::Decode {
        message: "mesh binary index count does not fit this platform".to_owned(),
    })?;
    let secondary_uv_block_count = secondary_uv_mask.count_ones() as usize;
    let expected_bytes = expected_binary_mesh_data_bytes(
        vertex_count,
        index_count,
        flags,
        secondary_uv_block_count,
    )?;
    if reader.remaining() != expected_bytes {
        return Err(AssetError::Decode {
            message: format!(
                "mesh binary payload byte length mismatch: expected {expected_bytes} data bytes, found {}",
                reader.remaining()
            ),
        });
    }

    let vertices = reader.read_f32_triplets(vertex_count, "position")?;
    let normals = if flags & MESH_BINARY_FLAG_NORMALS != 0 {
        reader.read_f32_triplets(vertex_count, "normal")?
    } else {
        Vec::new()
    };
    let uvs = if flags & MESH_BINARY_FLAG_UVS != 0 {
        reader.read_f32_pairs(vertex_count, "uv")?
    } else {
        Vec::new()
    };
    let mut uv_sets = Vec::new();
    for channel_index in 0..u32::BITS {
        if secondary_uv_mask & (1 << channel_index) == 0 {
            continue;
        }
        while uv_sets.len() <= channel_index as usize {
            uv_sets.push(Vec::new());
        }
        uv_sets[channel_index as usize] =
            reader.read_f32_pairs(vertex_count, &format!("uv{}", channel_index + 1))?;
    }
    let tangents = if flags & MESH_BINARY_FLAG_TANGENTS != 0 {
        reader.read_f32_quads(vertex_count, "tangent")?
    } else {
        Vec::new()
    };
    let (joints, weights) = if flags & MESH_BINARY_FLAG_SKINNING != 0 {
        (
            reader.read_u16_quads(vertex_count, "joint")?,
            reader.read_f32_quads(vertex_count, "weight")?,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    validate_binary_skin_weights(&weights, options.validate_skin_weight_totals)?;
    let index_format = if flags & MESH_BINARY_FLAG_INDEX_U16 != 0 {
        MeshIndexFormat::Uint16
    } else {
        MeshIndexFormat::Uint32
    };
    let indices = if index_format == MeshIndexFormat::Uint16 {
        reader
            .read_u16s(index_count, "index")?
            .into_iter()
            .map(u32::from)
            .collect()
    } else {
        reader.read_u32s(index_count, "index")?
    };

    mesh_from_components(
        vertices,
        normals,
        uvs,
        uv_sets,
        tangents,
        joints,
        weights,
        indices,
        index_format,
    )
}

#[cfg(feature = "model_cooker")]
pub(crate) fn encode_binary_mesh(mesh: &Mesh) -> Result<Vec<u8>, AssetError> {
    encode_binary_mesh_with_index_encoding(mesh, MeshBinaryIndexEncoding::U32)
}

#[cfg(feature = "model_cooker")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MeshBinaryIndexEncoding {
    U16,
    U32,
}

#[cfg(feature = "model_cooker")]
pub(crate) fn encode_binary_mesh_with_index_encoding(
    mesh: &Mesh,
    index_encoding: MeshBinaryIndexEncoding,
) -> Result<Vec<u8>, AssetError> {
    let mut flags = 0;
    if !mesh.normals.is_empty() {
        flags |= MESH_BINARY_FLAG_NORMALS;
    }
    if !mesh.uvs.is_empty() {
        flags |= MESH_BINARY_FLAG_UVS;
    }
    if !mesh.tangents.is_empty() {
        flags |= MESH_BINARY_FLAG_TANGENTS;
    }
    if !mesh.joints.is_empty() {
        flags |= MESH_BINARY_FLAG_SKINNING;
    }
    if index_encoding == MeshBinaryIndexEncoding::U16 {
        flags |= MESH_BINARY_FLAG_INDEX_U16;
        if mesh.indices.iter().any(|index| *index > u16::MAX as u32) {
            return Err(AssetError::Decode {
                message: "mesh binary encode u16 index exceeds u16 range".to_owned(),
            });
        }
    }

    if !mesh.uv_sets.is_empty() && mesh.uvs.is_empty() {
        return Err(AssetError::Decode {
            message: "mesh binary encode secondary uv sets require primary uv coordinates"
                .to_owned(),
        });
    }
    if mesh.uv_sets.len() > u32::BITS as usize {
        return Err(AssetError::Decode {
            message: "mesh binary encode has too many secondary uv sets".to_owned(),
        });
    }

    let mut secondary_uv_mask = 0u32;
    for (index, uv_set) in mesh.uv_sets.iter().enumerate() {
        if !uv_set.is_empty() {
            secondary_uv_mask |= 1 << index;
        }
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(MESH_BINARY_MAGIC);
    push_binary_u32(
        &mut bytes,
        u32::try_from(mesh.vertices.len()).map_err(|_| AssetError::Decode {
            message: "mesh binary encode vertex count exceeds u32".to_owned(),
        })?,
    );
    push_binary_u32(
        &mut bytes,
        u32::try_from(mesh.indices.len()).map_err(|_| AssetError::Decode {
            message: "mesh binary encode index count exceeds u32".to_owned(),
        })?,
    );
    push_binary_u32(&mut bytes, flags);
    push_binary_u32(&mut bytes, secondary_uv_mask);

    for vertex in &mesh.vertices {
        push_binary_f32s(&mut bytes, vertex);
    }
    for normal in &mesh.normals {
        push_binary_f32s(&mut bytes, normal);
    }
    for uv in &mesh.uvs {
        push_binary_f32s(&mut bytes, uv);
    }
    for uv_set in &mesh.uv_sets {
        if !uv_set.is_empty() {
            for uv in uv_set {
                push_binary_f32s(&mut bytes, uv);
            }
        }
    }
    for tangent in &mesh.tangents {
        push_binary_f32s(&mut bytes, tangent);
    }
    for joint in &mesh.joints {
        for value in joint {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for weight in &mesh.weights {
        push_binary_f32s(&mut bytes, weight);
    }
    match index_encoding {
        MeshBinaryIndexEncoding::U16 => {
            for index in &mesh.indices {
                bytes.extend_from_slice(&(*index as u16).to_le_bytes());
            }
        }
        MeshBinaryIndexEncoding::U32 => {
            for index in &mesh.indices {
                push_binary_u32(&mut bytes, *index);
            }
        }
    }

    Ok(bytes)
}

#[cfg(feature = "model_cooker")]
fn push_binary_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[cfg(feature = "model_cooker")]
fn push_binary_f32s<const N: usize>(bytes: &mut Vec<u8>, values: &[f32; N]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn expected_binary_mesh_data_bytes(
    vertex_count: usize,
    index_count: usize,
    flags: u32,
    secondary_uv_block_count: usize,
) -> Result<usize, AssetError> {
    let mut vertex_bytes = 12usize;
    if flags & MESH_BINARY_FLAG_NORMALS != 0 {
        vertex_bytes = vertex_bytes.checked_add(12).ok_or_else(binary_size_error)?;
    }
    if flags & MESH_BINARY_FLAG_UVS != 0 {
        vertex_bytes = vertex_bytes.checked_add(8).ok_or_else(binary_size_error)?;
    }
    vertex_bytes = vertex_bytes
        .checked_add(
            secondary_uv_block_count
                .checked_mul(8)
                .ok_or_else(binary_size_error)?,
        )
        .ok_or_else(binary_size_error)?;
    if flags & MESH_BINARY_FLAG_TANGENTS != 0 {
        vertex_bytes = vertex_bytes.checked_add(16).ok_or_else(binary_size_error)?;
    }
    if flags & MESH_BINARY_FLAG_SKINNING != 0 {
        vertex_bytes = vertex_bytes.checked_add(24).ok_or_else(binary_size_error)?;
    }
    let vertex_bytes = vertex_count
        .checked_mul(vertex_bytes)
        .ok_or_else(binary_size_error)?;
    let index_size = if flags & MESH_BINARY_FLAG_INDEX_U16 != 0 {
        std::mem::size_of::<u16>()
    } else {
        std::mem::size_of::<u32>()
    };
    let index_bytes = index_count
        .checked_mul(index_size)
        .ok_or_else(binary_size_error)?;
    vertex_bytes
        .checked_add(index_bytes)
        .ok_or_else(binary_size_error)
}

fn binary_size_error() -> AssetError {
    AssetError::Decode {
        message: "mesh binary payload size overflow".to_owned(),
    }
}

struct MeshBinaryReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> MeshBinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    fn read_u32(&mut self, field: &str) -> Result<u32, AssetError> {
        Ok(u32::from_le_bytes(self.read_array(field)?))
    }

    fn read_u16(&mut self, field: &str) -> Result<u16, AssetError> {
        Ok(u16::from_le_bytes(self.read_array(field)?))
    }

    fn read_f32(&mut self, field: &str) -> Result<f32, AssetError> {
        let value = f32::from_le_bytes(self.read_array(field)?);
        if !value.is_finite() {
            return Err(AssetError::Decode {
                message: format!("mesh binary {field} value must be finite"),
            });
        }
        Ok(value)
    }

    fn read_array<const N: usize>(&mut self, field: &str) -> Result<[u8; N], AssetError> {
        let end = self.cursor.checked_add(N).ok_or_else(binary_size_error)?;
        let Some(slice) = self.bytes.get(self.cursor..end) else {
            return Err(AssetError::Decode {
                message: format!("mesh binary payload ended while reading {field}"),
            });
        };
        self.cursor = end;
        let mut bytes = [0; N];
        bytes.copy_from_slice(slice);
        Ok(bytes)
    }

    fn read_f32_pairs(&mut self, count: usize, field: &str) -> Result<Vec<[f32; 2]>, AssetError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push([self.read_f32(field)?, self.read_f32(field)?]);
        }
        Ok(values)
    }

    fn read_f32_triplets(
        &mut self,
        count: usize,
        field: &str,
    ) -> Result<Vec<[f32; 3]>, AssetError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push([
                self.read_f32(field)?,
                self.read_f32(field)?,
                self.read_f32(field)?,
            ]);
        }
        Ok(values)
    }

    fn read_f32_quads(&mut self, count: usize, field: &str) -> Result<Vec<[f32; 4]>, AssetError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push([
                self.read_f32(field)?,
                self.read_f32(field)?,
                self.read_f32(field)?,
                self.read_f32(field)?,
            ]);
        }
        Ok(values)
    }

    fn read_u16_quads(&mut self, count: usize, field: &str) -> Result<Vec<[u16; 4]>, AssetError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push([
                self.read_u16(field)?,
                self.read_u16(field)?,
                self.read_u16(field)?,
                self.read_u16(field)?,
            ]);
        }
        Ok(values)
    }

    fn read_u16s(&mut self, count: usize, field: &str) -> Result<Vec<u16>, AssetError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_u16(field)?);
        }
        Ok(values)
    }

    fn read_u32s(&mut self, count: usize, field: &str) -> Result<Vec<u32>, AssetError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.read_u32(field)?);
        }
        Ok(values)
    }
}

fn build_mesh_vertex_buffer(
    vertices: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    uv_sets: &[Vec<[f32; 2]>],
    tangents: &[[f32; 4]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
) -> Result<MeshVertexBuffer, AssetError> {
    let mut attributes = Vec::new();
    let mut offset = 0;
    push_vertex_attribute(
        &mut attributes,
        &mut offset,
        MeshVertexSemantic::Position,
        MeshVertexFormat::Float32x3,
    );
    if !normals.is_empty() {
        push_vertex_attribute(
            &mut attributes,
            &mut offset,
            MeshVertexSemantic::Normal,
            MeshVertexFormat::Float32x3,
        );
    }
    if !uvs.is_empty() {
        push_vertex_attribute(
            &mut attributes,
            &mut offset,
            MeshVertexSemantic::TexCoord(0),
            MeshVertexFormat::Float32x2,
        );
    }
    for (index, uv_set) in uv_sets.iter().enumerate() {
        if uv_set.is_empty() {
            continue;
        }
        let channel = u8::try_from(index + 1).map_err(|_| AssetError::Decode {
            message: "mesh has too many secondary uv sets for GPU metadata".to_owned(),
        })?;
        push_vertex_attribute(
            &mut attributes,
            &mut offset,
            MeshVertexSemantic::TexCoord(channel),
            MeshVertexFormat::Float32x2,
        );
    }
    if !tangents.is_empty() {
        push_vertex_attribute(
            &mut attributes,
            &mut offset,
            MeshVertexSemantic::Tangent,
            MeshVertexFormat::Float32x4,
        );
    }
    if !joints.is_empty() {
        push_vertex_attribute(
            &mut attributes,
            &mut offset,
            MeshVertexSemantic::Joints,
            MeshVertexFormat::Uint16x4,
        );
        push_vertex_attribute(
            &mut attributes,
            &mut offset,
            MeshVertexSemantic::Weights,
            MeshVertexFormat::Float32x4,
        );
    }

    let vertex_count = u32::try_from(vertices.len()).map_err(|_| AssetError::Decode {
        message: "mesh has too many vertices for GPU metadata".to_owned(),
    })?;
    let mut bytes = Vec::with_capacity(vertices.len() * offset as usize);
    for index in 0..vertices.len() {
        push_f32s(&mut bytes, &vertices[index]);
        if !normals.is_empty() {
            push_f32s(&mut bytes, &normals[index]);
        }
        if !uvs.is_empty() {
            push_f32s(&mut bytes, &uvs[index]);
        }
        for uv_set in uv_sets {
            if !uv_set.is_empty() {
                push_f32s(&mut bytes, &uv_set[index]);
            }
        }
        if !tangents.is_empty() {
            push_f32s(&mut bytes, &tangents[index]);
        }
        if !joints.is_empty() {
            for joint in joints[index] {
                bytes.extend_from_slice(&joint.to_le_bytes());
            }
            push_f32s(&mut bytes, &weights[index]);
        }
    }

    Ok(MeshVertexBuffer {
        layout: MeshVertexLayout {
            vertex_count,
            stride: offset,
            attributes,
        },
        bytes,
    })
}

fn push_vertex_attribute(
    attributes: &mut Vec<MeshVertexAttribute>,
    offset: &mut u32,
    semantic: MeshVertexSemantic,
    format: MeshVertexFormat,
) {
    attributes.push(MeshVertexAttribute {
        semantic,
        format,
        offset: *offset,
    });
    *offset += format.byte_size();
}

fn push_f32s<const N: usize>(bytes: &mut Vec<u8>, values: &[f32; N]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn mesh_upload_metadata(mesh: &Mesh) -> GpuUploadMetadata {
    GpuUploadMetadata::Mesh(MeshUploadMetadata {
        layout: mesh.vertex_buffer.layout.clone(),
        vertex_buffer_bytes: mesh.vertex_buffer.bytes.len() as u32,
        index_buffer_bytes: mesh.indices.len() as u32 * mesh.index_format.byte_size(),
        index_count: mesh.indices.len() as u32,
        index_format: mesh.index_format,
    })
}

fn mesh_upload_bytes(mesh: &Mesh) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        mesh.vertex_buffer.bytes.len()
            + mesh.indices.len() * mesh.index_format.byte_size() as usize,
    );
    bytes.extend_from_slice(&mesh.vertex_buffer.bytes);
    match mesh.index_format {
        MeshIndexFormat::Uint16 => {
            for index in &mesh.indices {
                bytes.extend_from_slice(&(*index as u16).to_le_bytes());
            }
        }
        MeshIndexFormat::Uint32 => {
            for index in &mesh.indices {
                bytes.extend_from_slice(&index.to_le_bytes());
            }
        }
    }
    bytes
}

fn mesh_uv_set_directive_index(directive: &str) -> Option<usize> {
    directive
        .strip_prefix("uv")
        .filter(|suffix| !suffix.is_empty())
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .filter(|index| *index > 0)
        .map(|index| index - 1)
}

fn ensure_uv_set(uv_sets: &mut Vec<Vec<[f32; 2]>>, set_index: usize) -> &mut Vec<[f32; 2]> {
    while uv_sets.len() <= set_index {
        uv_sets.push(Vec::new());
    }
    &mut uv_sets[set_index]
}

fn validate_uv_sets(uv_sets: &[Vec<[f32; 2]>], vertex_count: usize) -> Result<(), AssetError> {
    for (index, uvs) in uv_sets.iter().enumerate() {
        if !uvs.is_empty() && uvs.len() != vertex_count {
            return Err(AssetError::Decode {
                message: format!(
                    "mesh uv{} count {} must match vertex count {}",
                    index + 1,
                    uvs.len(),
                    vertex_count
                ),
            });
        }
    }
    Ok(())
}

fn validate_skinning_counts(
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    vertex_count: usize,
) -> Result<(), AssetError> {
    if joints.len() != weights.len() {
        return Err(AssetError::Decode {
            message: format!(
                "mesh skin joint count {} must match skin weight count {}",
                joints.len(),
                weights.len()
            ),
        });
    }
    if !joints.is_empty() && joints.len() != vertex_count {
        return Err(AssetError::Decode {
            message: format!(
                "mesh skin joint count {} must match vertex count {}",
                joints.len(),
                vertex_count
            ),
        });
    }
    Ok(())
}

fn parse_f32_triplet<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_index: usize,
    kind: &str,
) -> Result<[f32; 3], AssetError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing {kind} value on line {}", line_index + 1),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!("invalid {kind} value on line {}: {error}", line_index + 1),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many {kind} values on line {}", line_index + 1),
        });
    }
    Ok(values)
}

fn parse_f32_quad<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_index: usize,
    kind: &str,
) -> Result<[f32; 4], AssetError> {
    let mut values = [0.0; 4];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing {kind} value on line {}", line_index + 1),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!("invalid {kind} value on line {}: {error}", line_index + 1),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many {kind} values on line {}", line_index + 1),
        });
    }
    Ok(values)
}

fn validate_skin_weights(
    values: [f32; 4],
    line_index: usize,
    validate_total: bool,
) -> Result<(), AssetError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(AssetError::Decode {
            message: format!("mesh skin weight must be finite on line {}", line_index + 1),
        });
    }
    if values.iter().any(|value| *value < 0.0) {
        return Err(AssetError::Decode {
            message: format!(
                "mesh skin weight must be non-negative on line {}",
                line_index + 1
            ),
        });
    }
    if validate_total {
        validate_skin_weight_total(values, "mesh", &format!("on line {}", line_index + 1))?;
    }
    Ok(())
}

fn validate_binary_skin_weights(
    weights: &[[f32; 4]],
    validate_total: bool,
) -> Result<(), AssetError> {
    for (vertex_index, values) in weights.iter().copied().enumerate() {
        if values.iter().any(|value| *value < 0.0) {
            return Err(AssetError::Decode {
                message: format!(
                    "mesh binary skin weight must be non-negative at vertex {vertex_index}"
                ),
            });
        }
        if validate_total {
            validate_skin_weight_total(
                values,
                "mesh binary",
                &format!("at vertex {vertex_index}"),
            )?;
        }
    }
    Ok(())
}

fn validate_skin_weight_total(
    values: [f32; 4],
    prefix: &str,
    context: &str,
) -> Result<(), AssetError> {
    let total = values.iter().sum::<f32>();
    if total <= f32::EPSILON {
        return Err(AssetError::Decode {
            message: format!("{prefix} skin weight total must be positive {context}"),
        });
    }
    if (total - 1.0).abs() > SKIN_WEIGHT_SUM_EPSILON {
        return Err(AssetError::Decode {
            message: format!("{prefix} skin weights {context} must sum to 1.0, found {total}"),
        });
    }
    Ok(())
}

fn parse_f32_pair<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_index: usize,
    kind: &str,
) -> Result<[f32; 2], AssetError> {
    let mut values = [0.0; 2];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing {kind} value on line {}", line_index + 1),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!("invalid {kind} value on line {}: {error}", line_index + 1),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many {kind} values on line {}", line_index + 1),
        });
    }
    Ok(values)
}

fn parse_u16_quad<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_index: usize,
    kind: &str,
) -> Result<[u16; 4], AssetError> {
    let mut values = [0; 4];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing {kind} value on line {}", line_index + 1),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!("invalid {kind} value on line {}: {error}", line_index + 1),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many {kind} values on line {}", line_index + 1),
        });
    }
    Ok(values)
}

fn parse_u32_triplet<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_index: usize,
) -> Result<[u32; 3], AssetError> {
    let mut values = [0; 3];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing index value on line {}", line_index + 1),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!("invalid index value on line {}: {error}", line_index + 1),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many index values on line {}", line_index + 1),
        });
    }
    Ok(values)
}
