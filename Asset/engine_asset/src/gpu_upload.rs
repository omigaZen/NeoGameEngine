use std::collections::VecDeque;

use crate::id::{AssetId, AssetTypeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpuResourceHandle(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuUploadKind {
    Texture,
    Mesh,
    Material,
    Shader,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuUploadCommand {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub kind: GpuUploadKind,
    pub label: Option<String>,
    pub metadata: GpuUploadMetadata,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuUploadMetadata {
    None,
    Mesh(MeshUploadMetadata),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshUploadMetadata {
    pub layout: MeshVertexLayout,
    pub vertex_buffer_bytes: u32,
    pub index_buffer_bytes: u32,
    pub index_count: u32,
    pub index_format: MeshIndexFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshVertexBuffer {
    pub layout: MeshVertexLayout,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshVertexLayout {
    pub vertex_count: u32,
    pub stride: u32,
    pub attributes: Vec<MeshVertexAttribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshVertexAttribute {
    pub semantic: MeshVertexSemantic,
    pub format: MeshVertexFormat,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshVertexSemantic {
    Position,
    Normal,
    TexCoord(u8),
    Tangent,
    Joints,
    Weights,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshVertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
    Uint16x4,
}

impl MeshVertexFormat {
    pub fn byte_size(self) -> u32 {
        match self {
            Self::Float32x2 => 8,
            Self::Float32x3 => 12,
            Self::Float32x4 => 16,
            Self::Uint16x4 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshIndexFormat {
    Uint16,
    Uint32,
}

impl MeshIndexFormat {
    pub fn byte_size(self) -> u32 {
        match self {
            Self::Uint16 => std::mem::size_of::<u16>() as u32,
            Self::Uint32 => std::mem::size_of::<u32>() as u32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuUploadResult {
    pub id: AssetId,
    pub result: Result<GpuResourceHandle, String>,
}

impl GpuUploadResult {
    pub fn ok(id: AssetId, handle: GpuResourceHandle) -> Self {
        Self {
            id,
            result: Ok(handle),
        }
    }

    pub fn failed(id: AssetId, message: impl Into<String>) -> Self {
        Self {
            id,
            result: Err(message.into()),
        }
    }
}

#[derive(Default)]
pub struct GpuUploadQueue {
    commands: VecDeque<GpuUploadCommand>,
    results: VecDeque<GpuUploadResult>,
}

impl GpuUploadQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: GpuUploadCommand) {
        self.commands.push_back(command);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = GpuUploadCommand> + '_ {
        self.commands.drain(..)
    }

    pub fn drain_limit(&mut self, limit: usize) -> impl Iterator<Item = GpuUploadCommand> + '_ {
        let count = limit.min(self.commands.len());
        self.commands.drain(..count)
    }

    pub fn submit_result(&mut self, result: GpuUploadResult) {
        self.results.push_back(result);
    }

    pub fn drain_results(&mut self) -> impl Iterator<Item = GpuUploadResult> + '_ {
        self.results.drain(..)
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
