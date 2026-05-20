use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

#[cfg(feature = "backend-wgpu")]
use std::sync::mpsc;

use crate::{
    DepthFormat, RendererError, RendererFeature, TextureFormat, VertexFormat, VertexStepMode,
};

#[cfg(feature = "backend-wgpu")]
use graphics_wgpu::{wgpu, WgpuGraphics};

pub type RhiError = RendererError;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RhiCaps {
    pub backend_name: String,
    pub adapter_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollMode {
    Poll,
    Wait,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RhiResolveMode {
    Average,
    FirstSample,
    Sample(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubmissionIndex(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiBufferDesc {
    pub label: Option<String>,
    pub size: u64,
    pub usage: RhiBufferUsage,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RhiBufferUsage(pub u32);

impl RhiBufferUsage {
    pub const UNIFORM: Self = Self(1 << 0);
    pub const STORAGE: Self = Self(1 << 1);
    pub const VERTEX: Self = Self(1 << 2);
    pub const INDEX: Self = Self(1 << 3);
    pub const INDIRECT: Self = Self(1 << 4);
    pub const COPY_SRC: Self = Self(1 << 5);
    pub const COPY_DST: Self = Self(1 << 6);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for RhiBufferUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiTextureDesc {
    pub label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub format: TextureFormat,
    pub usage: RhiTextureUsage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiResolveShaderDesc {
    pub label: Option<String>,
    pub source: String,
    pub entry_point: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RhiCustomResolvePath {
    Rgba8StorageCompute,
    Rgba16FloatStorageCompute,
    Rgba32FloatStorageCompute,
    EightBitColorFragment,
    Depth32FloatFragment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiCustomResolvePathSupport {
    pub path: RhiCustomResolvePath,
    pub supported: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiCustomResolveSupport {
    pub paths: Vec<RhiCustomResolvePathSupport>,
}

impl RhiCustomResolveSupport {
    pub fn headless() -> Self {
        let reason = "headless RHI does not execute user-supplied WGSL resolve shaders";
        Self {
            paths: vec![
                unsupported_custom_resolve_path(RhiCustomResolvePath::Rgba8StorageCompute, reason),
                unsupported_custom_resolve_path(
                    RhiCustomResolvePath::Rgba16FloatStorageCompute,
                    reason,
                ),
                unsupported_custom_resolve_path(
                    RhiCustomResolvePath::Rgba32FloatStorageCompute,
                    reason,
                ),
                unsupported_custom_resolve_path(
                    RhiCustomResolvePath::EightBitColorFragment,
                    reason,
                ),
                unsupported_custom_resolve_path(RhiCustomResolvePath::Depth32FloatFragment, reason),
            ],
        }
    }

    pub fn backend_wgpu() -> Self {
        Self {
            paths: vec![
                supported_custom_resolve_path(RhiCustomResolvePath::Rgba8StorageCompute),
                supported_custom_resolve_path(RhiCustomResolvePath::Rgba16FloatStorageCompute),
                supported_custom_resolve_path(RhiCustomResolvePath::Rgba32FloatStorageCompute),
                supported_custom_resolve_path(RhiCustomResolvePath::EightBitColorFragment),
                supported_custom_resolve_path(RhiCustomResolvePath::Depth32FloatFragment),
            ],
        }
    }

    pub fn support_for(&self, path: RhiCustomResolvePath) -> Option<&RhiCustomResolvePathSupport> {
        self.paths.iter().find(|support| support.path == path)
    }

    pub fn supports(&self, path: RhiCustomResolvePath) -> bool {
        self.support_for(path)
            .map(|support| support.supported)
            .unwrap_or(false)
    }
}

fn supported_custom_resolve_path(path: RhiCustomResolvePath) -> RhiCustomResolvePathSupport {
    RhiCustomResolvePathSupport {
        path,
        supported: true,
        unsupported_reason: None,
    }
}

fn unsupported_custom_resolve_path(
    path: RhiCustomResolvePath,
    reason: &str,
) -> RhiCustomResolvePathSupport {
    RhiCustomResolvePathSupport {
        path,
        supported: false,
        unsupported_reason: Some(reason.to_owned()),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RhiTextureUsage(pub u32);

impl RhiTextureUsage {
    pub const SAMPLED: Self = Self(1 << 0);
    pub const STORAGE: Self = Self(1 << 1);
    pub const RENDER_ATTACHMENT: Self = Self(1 << 2);
    pub const COPY_SRC: Self = Self(1 << 3);
    pub const COPY_DST: Self = Self(1 << 4);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for RhiTextureUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RhiTextureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiSamplerDesc {
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiBindGroupDesc {
    pub label: Option<String>,
    pub pipeline: RhiGraphicsPipeline,
    pub group_index: u32,
    pub entries: Vec<RhiBindGroupEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiComputeBindGroupDesc {
    pub label: Option<String>,
    pub pipeline: RhiComputePipeline,
    pub group_index: u32,
    pub entries: Vec<RhiBindGroupEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RhiBindGroupEntry {
    Texture { binding: u32, texture: RhiTexture },
    Sampler { binding: u32, sampler: RhiSampler },
    Buffer { binding: u32, buffer: RhiBuffer },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiShaderModuleDesc {
    pub label: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiGraphicsPipelineDesc {
    pub label: Option<String>,
    pub vertex_shader: RhiShaderModule,
    pub vertex_entry: String,
    pub fragment_shader: Option<RhiShaderModule>,
    pub fragment_entry: Option<String>,
    pub color_format: Option<TextureFormat>,
    pub depth_format: Option<DepthFormat>,
    pub vertex_buffers: Vec<RhiVertexBufferLayout>,
    pub primitive: RhiPrimitiveState,
    pub depth: Option<RhiDepthState>,
    pub sample_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiComputePipelineDesc {
    pub label: Option<String>,
    pub shader: RhiShaderModule,
    pub entry_point: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiVertexBufferLayout {
    pub stride: u64,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<RhiVertexAttribute>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiVertexAttribute {
    pub location: u32,
    pub format: VertexFormat,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RhiPrimitiveState {
    pub topology: RhiPrimitiveTopology,
    pub cull_mode: Option<RhiFace>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RhiPrimitiveTopology {
    PointList,
    LineList,
    LineStrip,
    #[default]
    TriangleList,
    TriangleStrip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RhiFace {
    Front,
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RhiDepthState {
    pub format: DepthFormat,
    pub write_enabled: bool,
    pub compare: RhiCompareFunction,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RhiCompareFunction {
    Never,
    Less,
    Equal,
    #[default]
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiBuffer(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiTexture(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiSampler(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiBindGroup(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiShaderModule(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiGraphicsPipeline(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiComputePipeline(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiCommandBuffer(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiTimestampQuery(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiPipelineStatisticsQuery(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RhiOcclusionQuery(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiTimestampQueryDesc {
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiPipelineStatisticsQueryDesc {
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiOcclusionQueryDesc {
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiTimestampResult {
    pub query: RhiTimestampQuery,
    pub timestamp_ns: u64,
    pub available: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RhiPipelineStatistics {
    pub input_assembly_vertices: u64,
    pub input_assembly_primitives: u64,
    pub vertex_shader_invocations: u64,
    pub clipping_invocations: u64,
    pub clipping_primitives: u64,
    pub fragment_shader_invocations: u64,
    pub compute_shader_invocations: u64,
    pub draw_calls: u64,
    pub dispatch_calls: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiPipelineStatisticsResult {
    pub query: RhiPipelineStatisticsQuery,
    pub statistics: RhiPipelineStatistics,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiOcclusionQueryResult {
    pub query: RhiOcclusionQuery,
    pub samples_passed: u64,
    pub visible: bool,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiComputePassDesc {
    pub label: Option<String>,
    pub pipeline: RhiComputePipeline,
    pub bind_groups: Vec<RhiComputePassBindGroup>,
    pub workgroups: [u32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiComputePassBindGroup {
    pub index: u32,
    pub bind_group: RhiBindGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiRenderPassDesc {
    pub label: Option<String>,
    pub pipeline: RhiGraphicsPipeline,
    pub color_target: Option<RhiTexture>,
    pub depth_target: Option<RhiTexture>,
    pub vertex_buffers: Vec<RhiVertexBufferBinding>,
    pub index_buffer: Option<RhiIndexBufferBinding>,
    pub bind_groups: Vec<RhiRenderPassBindGroup>,
    pub vertex_count: u32,
    pub index_count: Option<u32>,
    pub instance_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiRenderPassBindGroup {
    pub index: u32,
    pub bind_group: RhiBindGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiVertexBufferBinding {
    pub slot: u32,
    pub buffer: RhiBuffer,
    pub offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiIndexBufferBinding {
    pub buffer: RhiBuffer,
    pub offset: u64,
    pub format: RhiIndexFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RhiIndexFormat {
    Uint16,
    Uint32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiIndirectRenderPassDesc {
    pub label: Option<String>,
    pub pipeline: RhiGraphicsPipeline,
    pub color_target: Option<RhiTexture>,
    pub depth_target: Option<RhiTexture>,
    pub vertex_buffers: Vec<RhiVertexBufferBinding>,
    pub bind_groups: Vec<RhiRenderPassBindGroup>,
    pub indirect_buffer: RhiBuffer,
    pub indirect_offset: u64,
    pub draw_count: u32,
    pub draw_stride: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiIndexedIndirectRenderPassDesc {
    pub label: Option<String>,
    pub pipeline: RhiGraphicsPipeline,
    pub color_target: Option<RhiTexture>,
    pub depth_target: Option<RhiTexture>,
    pub vertex_buffers: Vec<RhiVertexBufferBinding>,
    pub index_buffer: RhiIndexBufferBinding,
    pub bind_groups: Vec<RhiRenderPassBindGroup>,
    pub indirect_buffer: RhiBuffer,
    pub indirect_offset: u64,
    pub draw_count: u32,
    pub draw_stride: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RhiResource {
    Texture(RhiTexture),
    Buffer(RhiBuffer),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RhiAccessState {
    TextureSampled,
    TextureStorageRead,
    TextureStorageWrite,
    RenderAttachment,
    CopySrc,
    CopyDst,
    BufferUniform,
    BufferStorageRead,
    BufferStorageWrite,
    BufferVertex,
    BufferIndex,
    BufferIndirect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiResourceBarrierDesc {
    pub resource: RhiResource,
    pub before: Option<RhiAccessState>,
    pub after: RhiAccessState,
}

pub trait RhiCommandEncoder: Send {
    fn encode_resource_barrier(&mut self, desc: &RhiResourceBarrierDesc) -> Result<(), RhiError>;
    fn encode_compute_pass(&mut self, desc: &RhiComputePassDesc) -> Result<(), RhiError>;
    fn encode_render_pass(&mut self, desc: &RhiRenderPassDesc) -> Result<(), RhiError>;
    fn encode_indirect_render_pass(
        &mut self,
        desc: &RhiIndirectRenderPassDesc,
    ) -> Result<(), RhiError>;
    fn encode_indexed_indirect_render_pass(
        &mut self,
        desc: &RhiIndexedIndirectRenderPassDesc,
    ) -> Result<(), RhiError>;
    fn begin_pipeline_statistics(
        &mut self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<(), RhiError>;
    fn end_pipeline_statistics(
        &mut self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<(), RhiError>;
    fn begin_occlusion_query(&mut self, query: RhiOcclusionQuery) -> Result<(), RhiError>;
    fn end_occlusion_query(&mut self, query: RhiOcclusionQuery) -> Result<(), RhiError>;
    fn write_timestamp(&mut self, query: RhiTimestampQuery) -> Result<(), RhiError>;
    fn push_debug_group(&mut self, label: &str) -> Result<(), RhiError>;
    fn pop_debug_group(&mut self) -> Result<(), RhiError>;
    fn finish(self: Box<Self>) -> Result<RhiCommandBuffer, RhiError>;
}

pub trait RhiDevice: Send + Sync {
    fn caps(&self) -> &RhiCaps;

    fn create_buffer(&self, desc: &RhiBufferDesc) -> Result<RhiBuffer, RhiError>;
    fn buffer_usage(&self, buffer: RhiBuffer) -> Result<RhiBufferUsage, RhiError>;
    fn write_buffer(&self, buffer: RhiBuffer, offset: u64, data: &[u8]) -> Result<(), RhiError>;
    fn read_buffer(&self, buffer: RhiBuffer, offset: u64, size: u64) -> Result<Vec<u8>, RhiError>;
    fn create_texture(&self, desc: &RhiTextureDesc) -> Result<RhiTexture, RhiError>;
    fn texture_usage(&self, texture: RhiTexture) -> Result<RhiTextureUsage, RhiError>;
    fn texture_samples(&self, texture: RhiTexture) -> Result<u32, RhiError>;
    fn custom_resolve_support(&self) -> RhiCustomResolveSupport;
    fn resolve_texture_rgba8(&self, source: RhiTexture, target: RhiTexture)
        -> Result<(), RhiError>;
    fn resolve_texture_rgba8_with_mode(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        mode: RhiResolveMode,
    ) -> Result<(), RhiError>;
    fn resolve_texture_rgba8_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RhiError>;
    fn resolve_texture_rgba16f_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RhiError>;
    fn resolve_texture_rgba32f_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RhiError>;
    fn resolve_texture_8bit_color_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RhiError>;
    fn resolve_texture_depth32f_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RhiError>;
    fn write_texture_rgba8(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[u8],
    ) -> Result<(), RhiError>;
    fn write_texture_rgba16f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[u16],
    ) -> Result<(), RhiError>;
    fn write_texture_rgba32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[f32],
    ) -> Result<(), RhiError>;
    fn write_texture_depth32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[f32],
    ) -> Result<(), RhiError>;
    fn read_texture_rgba8(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<u8>, RhiError>;
    fn read_texture_rgba16f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<u16>, RhiError>;
    fn read_texture_rgba32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<f32>, RhiError>;
    fn read_texture_depth32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<f32>, RhiError>;
    fn create_sampler(&self, desc: &RhiSamplerDesc) -> Result<RhiSampler, RhiError>;
    fn create_bind_group(&self, desc: &RhiBindGroupDesc) -> Result<RhiBindGroup, RhiError>;
    fn create_compute_bind_group(
        &self,
        desc: &RhiComputeBindGroupDesc,
    ) -> Result<RhiBindGroup, RhiError>;
    fn create_shader_module(&self, desc: &RhiShaderModuleDesc)
        -> Result<RhiShaderModule, RhiError>;
    fn create_graphics_pipeline(
        &self,
        desc: &RhiGraphicsPipelineDesc,
    ) -> Result<RhiGraphicsPipeline, RhiError>;
    fn create_compute_pipeline(
        &self,
        desc: &RhiComputePipelineDesc,
    ) -> Result<RhiComputePipeline, RhiError>;
    fn create_timestamp_query(
        &self,
        desc: &RhiTimestampQueryDesc,
    ) -> Result<RhiTimestampQuery, RhiError>;
    fn timestamp_result(&self, query: RhiTimestampQuery) -> Result<RhiTimestampResult, RhiError>;
    fn create_pipeline_statistics_query(
        &self,
        desc: &RhiPipelineStatisticsQueryDesc,
    ) -> Result<RhiPipelineStatisticsQuery, RhiError>;
    fn pipeline_statistics_result(
        &self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<RhiPipelineStatisticsResult, RhiError>;
    fn create_occlusion_query(
        &self,
        desc: &RhiOcclusionQueryDesc,
    ) -> Result<RhiOcclusionQuery, RhiError>;
    fn occlusion_result(
        &self,
        query: RhiOcclusionQuery,
    ) -> Result<RhiOcclusionQueryResult, RhiError>;

    fn create_command_encoder(
        &self,
        label: Option<&str>,
    ) -> Result<Box<dyn RhiCommandEncoder>, RhiError>;
    fn submit(&self, commands: Vec<RhiCommandBuffer>) -> Result<SubmissionIndex, RhiError>;

    fn poll(&self, mode: PollMode);
}

pub struct RhiAccess<'a> {
    device: &'a dyn RhiDevice,
}

impl<'a> RhiAccess<'a> {
    pub fn new(device: &'a dyn RhiDevice) -> Self {
        Self { device }
    }

    pub fn caps(&self) -> &RhiCaps {
        self.device.caps()
    }

    pub fn create_command_encoder(
        &self,
        label: Option<&str>,
    ) -> Result<Box<dyn RhiCommandEncoder>, RhiError> {
        self.device.create_command_encoder(label)
    }

    pub fn submit(&self, commands: Vec<RhiCommandBuffer>) -> Result<SubmissionIndex, RhiError> {
        self.device.submit(commands)
    }

    pub fn poll(&self, mode: PollMode) {
        self.device.poll(mode);
    }

    pub fn device(&self) -> &'a dyn RhiDevice {
        self.device
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadlessRhiStats {
    pub buffers: usize,
    pub textures: usize,
    pub samplers: usize,
    pub bind_groups: usize,
    pub shader_modules: usize,
    pub graphics_pipelines: usize,
    pub compute_pipelines: usize,
    pub timestamp_queries: usize,
    pub pipeline_statistics_queries: usize,
    pub occlusion_queries: usize,
    pub uniform_buffers: usize,
    pub storage_buffers: usize,
    pub vertex_buffers: usize,
    pub index_buffers: usize,
    pub indirect_buffers: usize,
    pub copy_src_buffers: usize,
    pub copy_dst_buffers: usize,
    pub sampled_textures: usize,
    pub storage_textures: usize,
    pub render_attachment_textures: usize,
    pub copy_src_textures: usize,
    pub copy_dst_textures: usize,
    pub finished_command_buffers: usize,
    pub submitted_command_buffers: usize,
    pub submissions: usize,
    pub encoded_compute_dispatches: usize,
    pub encoded_render_draws: usize,
    pub encoded_indirect_draws: usize,
    pub encoded_barriers: usize,
    pub encoded_timestamp_writes: usize,
    pub encoded_debug_groups: usize,
    pub last_poll: Option<PollMode>,
}

#[cfg(feature = "backend-wgpu")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WgpuRhiStats {
    pub buffers: usize,
    pub textures: usize,
    pub samplers: usize,
    pub bind_groups: usize,
    pub shader_modules: usize,
    pub graphics_pipelines: usize,
    pub compute_pipelines: usize,
    pub timestamp_queries: usize,
    pub pipeline_statistics_queries: usize,
    pub occlusion_queries: usize,
    pub finished_command_buffers: usize,
    pub submitted_command_buffers: usize,
    pub submissions: usize,
    pub encoded_compute_dispatches: usize,
    pub encoded_render_draws: usize,
    pub encoded_indirect_draws: usize,
    pub encoded_barriers: usize,
    pub encoded_timestamp_writes: usize,
    pub encoded_debug_groups: usize,
    pub last_poll: Option<PollMode>,
}

#[derive(Clone, Debug)]
pub struct HeadlessRhiDevice {
    caps: RhiCaps,
    state: Arc<Mutex<HeadlessRhiState>>,
}

impl HeadlessRhiDevice {
    pub fn new() -> Self {
        Self::with_caps(RhiCaps {
            backend_name: "headless".to_owned(),
            adapter_name: "Headless RHI".to_owned(),
        })
    }

    pub fn with_caps(caps: RhiCaps) -> Self {
        Self {
            caps,
            state: Arc::new(Mutex::new(HeadlessRhiState::default())),
        }
    }

    pub fn stats(&self) -> HeadlessRhiStats {
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .stats()
    }

    fn allocate(&self) -> u64 {
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .allocate()
    }

    fn validate_label(label: Option<&str>) -> Result<(), RendererError> {
        if label.is_some_and(|value| value.trim().is_empty()) {
            return Err(RendererError::Validation(
                "RHI labels must not be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

impl Default for HeadlessRhiDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "backend-wgpu")]
#[derive(Clone)]
pub struct WgpuRhiDevice {
    caps: RhiCaps,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    state: Arc<Mutex<WgpuRhiState>>,
}

#[cfg(feature = "backend-wgpu")]
impl WgpuRhiDevice {
    pub fn new(graphics: &WgpuGraphics) -> Self {
        Self {
            caps: RhiCaps {
                backend_name: "wgpu".to_owned(),
                adapter_name: "wgpu adapter".to_owned(),
            },
            device: graphics.device_handle(),
            queue: graphics.queue_handle(),
            state: Arc::new(Mutex::new(WgpuRhiState::default())),
        }
    }

    pub fn stats(&self) -> WgpuRhiStats {
        self.state.lock().expect("wgpu RHI mutex poisoned").stats()
    }

    fn require_shader_module(
        state: &WgpuRhiState,
        shader: RhiShaderModule,
    ) -> Result<&wgpu::ShaderModule, RendererError> {
        state.shader_modules.get(&shader).ok_or_else(|| {
            RendererError::Validation(format!("unknown RHI shader module: {}", shader.0))
        })
    }

    fn allocate(&self) -> u64 {
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .allocate()
    }

    fn validate_label(label: Option<&str>) -> Result<(), RendererError> {
        if label.is_some_and(|value| value.trim().is_empty()) {
            return Err(RendererError::Validation(
                "RHI labels must not be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

impl RhiDevice for HeadlessRhiDevice {
    fn caps(&self) -> &RhiCaps {
        &self.caps
    }

    fn create_buffer(&self, desc: &RhiBufferDesc) -> Result<RhiBuffer, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.size == 0 {
            return Err(RendererError::Validation(
                "RHI buffers must have a non-zero size".to_owned(),
            ));
        }
        if desc.usage.is_empty() {
            return Err(RendererError::Validation(
                "RHI buffer usage must not be empty".to_owned(),
            ));
        }
        let handle = RhiBuffer(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .buffers
            .insert(handle, HeadlessBufferState::new(desc));
        Ok(handle)
    }

    fn buffer_usage(&self, buffer: RhiBuffer) -> Result<RhiBufferUsage, RendererError> {
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .buffers
            .get(&buffer)
            .map(|state| state.usage)
            .ok_or_else(|| RendererError::Validation(format!("unknown RHI buffer: {}", buffer.0)))
    }

    fn write_buffer(
        &self,
        buffer: RhiBuffer,
        offset: u64,
        data: &[u8],
    ) -> Result<(), RendererError> {
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(buffer_state) = state.buffers.get_mut(&buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI buffer: {}",
                buffer.0
            )));
        };
        if !buffer_state.usage.contains(RhiBufferUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "RHI buffer write requires COPY_DST usage".to_owned(),
            ));
        }
        let offset = usize::try_from(offset).map_err(|_| {
            RendererError::Validation("RHI buffer write offset is too large".to_owned())
        })?;
        let end = offset.checked_add(data.len()).ok_or_else(|| {
            RendererError::Validation("RHI buffer write range overflows usize".to_owned())
        })?;
        if end > buffer_state.bytes.len() {
            return Err(RendererError::Validation(
                "RHI buffer write range exceeds buffer size".to_owned(),
            ));
        }
        buffer_state.bytes[offset..end].copy_from_slice(data);
        Ok(())
    }

    fn read_buffer(
        &self,
        buffer: RhiBuffer,
        offset: u64,
        size: u64,
    ) -> Result<Vec<u8>, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(buffer_state) = state.buffers.get(&buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI buffer: {}",
                buffer.0
            )));
        };
        if !buffer_state.usage.contains(RhiBufferUsage::COPY_SRC) {
            return Err(RendererError::Validation(
                "RHI buffer read requires COPY_SRC usage".to_owned(),
            ));
        }
        let offset = usize::try_from(offset).map_err(|_| {
            RendererError::Validation("RHI buffer read offset is too large".to_owned())
        })?;
        let size = usize::try_from(size).map_err(|_| {
            RendererError::Validation("RHI buffer read size is too large".to_owned())
        })?;
        let end = offset.checked_add(size).ok_or_else(|| {
            RendererError::Validation("RHI buffer read range overflows usize".to_owned())
        })?;
        if end > buffer_state.bytes.len() {
            return Err(RendererError::Validation(
                "RHI buffer read range exceeds buffer size".to_owned(),
            ));
        }
        Ok(buffer_state.bytes[offset..end].to_vec())
    }

    fn create_texture(&self, desc: &RhiTextureDesc) -> Result<RhiTexture, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.width == 0 || desc.height == 0 {
            return Err(RendererError::Validation(
                "RHI textures must have non-zero dimensions".to_owned(),
            ));
        }
        if desc.samples == 0 || !desc.samples.is_power_of_two() {
            return Err(RendererError::Validation(
                "RHI texture samples must be a non-zero power of two".to_owned(),
            ));
        }
        if desc.usage.is_empty() {
            return Err(RendererError::Validation(
                "RHI texture usage must not be empty".to_owned(),
            ));
        }
        validate_texture_usage_format(desc.usage, desc.format)?;
        let handle = RhiTexture(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .textures
            .insert(handle, HeadlessTextureState::new(desc));
        Ok(handle)
    }

    fn texture_usage(&self, texture: RhiTexture) -> Result<RhiTextureUsage, RendererError> {
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .textures
            .get(&texture)
            .map(|state| {
                debug_assert!(state.samples != 0);
                state.usage
            })
            .ok_or_else(|| RendererError::Validation(format!("unknown RHI texture: {}", texture.0)))
    }

    fn texture_samples(&self, texture: RhiTexture) -> Result<u32, RendererError> {
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .textures
            .get(&texture)
            .map(|state| state.samples)
            .ok_or_else(|| RendererError::Validation(format!("unknown RHI texture: {}", texture.0)))
    }

    fn custom_resolve_support(&self) -> RhiCustomResolveSupport {
        RhiCustomResolveSupport::headless()
    }

    fn resolve_texture_rgba8(
        &self,
        source: RhiTexture,
        target: RhiTexture,
    ) -> Result<(), RendererError> {
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(source_state) = state.textures.get(&source).cloned() else {
            return Err(RendererError::Validation(format!(
                "unknown RHI resolve source texture: {}",
                source.0
            )));
        };
        let Some(target_state) = state.textures.get_mut(&target) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI resolve target texture: {}",
                target.0
            )));
        };
        validate_headless_rgba8_resolve_textures(source, &source_state, target, target_state)?;
        target_state.rgba8.copy_from_slice(&source_state.rgba8);
        Ok(())
    }

    fn resolve_texture_rgba8_with_mode(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        mode: RhiResolveMode,
    ) -> Result<(), RendererError> {
        match mode {
            RhiResolveMode::Average | RhiResolveMode::FirstSample => {
                self.resolve_texture_rgba8(source, target)
            }
            RhiResolveMode::Sample(sample_index) => {
                let source_samples = self.texture_samples(source)?;
                if sample_index >= source_samples {
                    return Err(RendererError::Validation(format!(
                        "RHI custom resolve sample index {sample_index} exceeds source sample count {source_samples}"
                    )));
                }
                self.resolve_texture_rgba8(source, target)
            }
        }
    }

    fn resolve_texture_rgba8_with_shader(
        &self,
        _source: RhiTexture,
        _target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        ))
    }

    fn resolve_texture_rgba16f_with_shader(
        &self,
        _source: RhiTexture,
        _target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        ))
    }

    fn resolve_texture_rgba32f_with_shader(
        &self,
        _source: RhiTexture,
        _target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        ))
    }

    fn resolve_texture_8bit_color_with_shader(
        &self,
        _source: RhiTexture,
        _target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        ))
    }

    fn resolve_texture_depth32f_with_shader(
        &self,
        _source: RhiTexture,
        _target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        ))
    }

    fn write_texture_rgba8(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[u8],
    ) -> Result<(), RendererError> {
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get_mut(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "RHI texture write requires COPY_DST usage".to_owned(),
            ));
        }
        if !is_rhi_8bit_color_texture_format(texture_state.format) {
            return Err(RendererError::Validation(
                "RHI RGBA8 texture write requires an 8-bit color format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        let expected = rgba8_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI RGBA8 texture write expected {expected} bytes but received {}",
                data.len()
            )));
        }
        copy_rgba8_region_to_texture(&mut texture_state.rgba8, texture_state.width, region, data);
        Ok(())
    }

    fn write_texture_rgba16f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[u16],
    ) -> Result<(), RendererError> {
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get_mut(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "RHI texture write requires COPY_DST usage".to_owned(),
            ));
        }
        if texture_state.format != TextureFormat::Rgba16Float {
            return Err(RendererError::Validation(
                "RHI RGBA16F texture write requires Rgba16Float format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        let expected = rgba16f_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI RGBA16F texture write expected {expected} channels but received {}",
                data.len()
            )));
        }
        copy_rgba16f_region_to_texture(
            &mut texture_state.rgba16f,
            texture_state.width,
            region,
            data,
        );
        Ok(())
    }

    fn write_texture_rgba32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[f32],
    ) -> Result<(), RendererError> {
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get_mut(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "RHI texture write requires COPY_DST usage".to_owned(),
            ));
        }
        if texture_state.format != TextureFormat::Rgba32Float {
            return Err(RendererError::Validation(
                "RHI RGBA32F texture write requires Rgba32Float format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        let expected = rgba32f_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI RGBA32F texture write expected {expected} channels but received {}",
                data.len()
            )));
        }
        if data.iter().any(|value| !value.is_finite()) {
            return Err(RendererError::Validation(
                "RHI RGBA32F texture writes require finite values".to_owned(),
            ));
        }
        copy_rgba32f_region_to_texture(
            &mut texture_state.rgba32f,
            texture_state.width,
            region,
            data,
        );
        Ok(())
    }

    fn write_texture_depth32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[f32],
    ) -> Result<(), RendererError> {
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get_mut(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "RHI texture write requires COPY_DST usage".to_owned(),
            ));
        }
        if texture_state.format != TextureFormat::Depth32Float {
            return Err(RendererError::Validation(
                "RHI depth32f texture write requires Depth32Float format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        let expected = depth32f_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI depth32f texture write expected {expected} values but received {}",
                data.len()
            )));
        }
        if data.iter().any(|value| !value.is_finite()) {
            return Err(RendererError::Validation(
                "RHI depth32f texture writes require finite values".to_owned(),
            ));
        }
        copy_depth32f_region_to_texture(
            &mut texture_state.depth32f,
            texture_state.width,
            region,
            data,
        );
        Ok(())
    }

    fn read_texture_rgba8(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<u8>, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
            return Err(RendererError::Validation(
                "RHI texture read requires COPY_SRC usage".to_owned(),
            ));
        }
        if !is_rhi_8bit_color_texture_format(texture_state.format) {
            return Err(RendererError::Validation(
                "RHI RGBA8 texture read requires an 8-bit color format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        Ok(copy_rgba8_region_from_texture(
            &texture_state.rgba8,
            texture_state.width,
            region,
        ))
    }

    fn read_texture_rgba16f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<u16>, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
            return Err(RendererError::Validation(
                "RHI texture read requires COPY_SRC usage".to_owned(),
            ));
        }
        if texture_state.format != TextureFormat::Rgba16Float {
            return Err(RendererError::Validation(
                "RHI RGBA16F texture reads require Rgba16Float format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        Ok(copy_rgba16f_region_from_texture(
            &texture_state.rgba16f,
            texture_state.width,
            region,
        ))
    }

    fn read_texture_rgba32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<f32>, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
            return Err(RendererError::Validation(
                "RHI texture read requires COPY_SRC usage".to_owned(),
            ));
        }
        if texture_state.format != TextureFormat::Rgba32Float {
            return Err(RendererError::Validation(
                "RHI RGBA32F texture reads require Rgba32Float format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        Ok(copy_rgba32f_region_from_texture(
            &texture_state.rgba32f,
            texture_state.width,
            region,
        ))
    }

    fn read_texture_depth32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<f32>, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(texture_state) = state.textures.get(&texture) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI texture: {}",
                texture.0
            )));
        };
        if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
            return Err(RendererError::Validation(
                "RHI texture read requires COPY_SRC usage".to_owned(),
            ));
        }
        if texture_state.format != TextureFormat::Depth32Float {
            return Err(RendererError::Validation(
                "RHI depth texture reads require Depth32Float format".to_owned(),
            ));
        }
        validate_texture_region(texture_state.width, texture_state.height, region)?;
        Ok(copy_depth32f_region_from_texture(
            &texture_state.depth32f,
            texture_state.width,
            region,
        ))
    }

    fn create_sampler(&self, desc: &RhiSamplerDesc) -> Result<RhiSampler, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiSampler(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .samplers
            .insert(handle);
        Ok(handle)
    }

    fn create_bind_group(&self, desc: &RhiBindGroupDesc) -> Result<RhiBindGroup, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiBindGroup(self.allocate());
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        validate_headless_bind_group_desc(&state, desc)?;
        state
            .bind_groups
            .insert(handle, HeadlessBindGroupState::new_graphics(desc));
        Ok(handle)
    }

    fn create_compute_bind_group(
        &self,
        desc: &RhiComputeBindGroupDesc,
    ) -> Result<RhiBindGroup, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiBindGroup(self.allocate());
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        validate_headless_compute_bind_group_desc(&state, desc)?;
        state
            .bind_groups
            .insert(handle, HeadlessBindGroupState::new_compute(desc));
        Ok(handle)
    }

    fn create_shader_module(
        &self,
        desc: &RhiShaderModuleDesc,
    ) -> Result<RhiShaderModule, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.source.trim().is_empty() {
            return Err(RendererError::ShaderCompile(
                "RHI shader source must not be empty".to_owned(),
            ));
        }
        let handle = RhiShaderModule(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .shader_modules
            .insert(handle);
        Ok(handle)
    }

    fn create_graphics_pipeline(
        &self,
        desc: &RhiGraphicsPipelineDesc,
    ) -> Result<RhiGraphicsPipeline, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        validate_graphics_pipeline_desc(desc)?;
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.shader_modules.contains(&desc.vertex_shader) {
            return Err(RendererError::Validation(format!(
                "unknown RHI shader module: {}",
                desc.vertex_shader.0
            )));
        }
        if let Some(fragment_shader) = desc.fragment_shader {
            if !state.shader_modules.contains(&fragment_shader) {
                return Err(RendererError::Validation(format!(
                    "unknown RHI shader module: {}",
                    fragment_shader.0
                )));
            }
        }
        drop(state);
        let handle = RhiGraphicsPipeline(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .graphics_pipelines
            .insert(handle, HeadlessGraphicsPipelineState::new(desc));
        Ok(handle)
    }

    fn create_compute_pipeline(
        &self,
        desc: &RhiComputePipelineDesc,
    ) -> Result<RhiComputePipeline, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.entry_point.trim().is_empty() {
            return Err(RendererError::PipelineCompile(
                "RHI compute pipeline entry point must not be empty".to_owned(),
            ));
        }
        if !self
            .state
            .lock()
            .expect("headless RHI mutex poisoned")
            .shader_modules
            .contains(&desc.shader)
        {
            return Err(RendererError::Validation(format!(
                "unknown RHI shader module: {}",
                desc.shader.0
            )));
        }
        let handle = RhiComputePipeline(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .compute_pipelines
            .insert(handle);
        Ok(handle)
    }

    fn create_timestamp_query(
        &self,
        desc: &RhiTimestampQueryDesc,
    ) -> Result<RhiTimestampQuery, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiTimestampQuery(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .timestamp_queries
            .insert(handle);
        Ok(handle)
    }

    fn timestamp_result(
        &self,
        query: RhiTimestampQuery,
    ) -> Result<RhiTimestampResult, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.timestamp_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI timestamp query: {}",
                query.0
            )));
        }
        Ok(RhiTimestampResult {
            query,
            timestamp_ns: state
                .timestamp_results
                .get(&query)
                .copied()
                .unwrap_or_default(),
            available: state.timestamp_results.contains_key(&query),
        })
    }

    fn create_pipeline_statistics_query(
        &self,
        desc: &RhiPipelineStatisticsQueryDesc,
    ) -> Result<RhiPipelineStatisticsQuery, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiPipelineStatisticsQuery(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .pipeline_statistics_queries
            .insert(handle);
        Ok(handle)
    }

    fn pipeline_statistics_result(
        &self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<RhiPipelineStatisticsResult, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.pipeline_statistics_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI pipeline statistics query: {}",
                query.0
            )));
        }
        Ok(RhiPipelineStatisticsResult {
            query,
            statistics: state
                .pipeline_statistics_results
                .get(&query)
                .cloned()
                .unwrap_or_default(),
            available: state.pipeline_statistics_results.contains_key(&query),
        })
    }

    fn create_occlusion_query(
        &self,
        desc: &RhiOcclusionQueryDesc,
    ) -> Result<RhiOcclusionQuery, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiOcclusionQuery(self.allocate());
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .occlusion_queries
            .insert(handle);
        Ok(handle)
    }

    fn occlusion_result(
        &self,
        query: RhiOcclusionQuery,
    ) -> Result<RhiOcclusionQueryResult, RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.occlusion_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI occlusion query: {}",
                query.0
            )));
        }
        let samples_passed = state
            .occlusion_results
            .get(&query)
            .copied()
            .unwrap_or_default();
        Ok(RhiOcclusionQueryResult {
            query,
            samples_passed,
            visible: samples_passed > 0,
            available: state.occlusion_results.contains_key(&query),
        })
    }

    fn create_command_encoder(
        &self,
        label: Option<&str>,
    ) -> Result<Box<dyn RhiCommandEncoder>, RendererError> {
        Self::validate_label(label)?;
        Ok(Box::new(HeadlessRhiCommandEncoder {
            state: Arc::clone(&self.state),
            encoded_barriers: 0,
            encoded_compute_dispatches: 0,
            encoded_render_draws: 0,
            encoded_indirect_draws: 0,
            encoded_debug_groups: 0,
            open_debug_groups: 0,
            timestamp_writes: Vec::new(),
            active_pipeline_statistics: HashMap::new(),
            pipeline_statistics_writes: Vec::new(),
            active_occlusion_queries: HashMap::new(),
            occlusion_writes: Vec::new(),
        }))
    }

    fn submit(&self, commands: Vec<RhiCommandBuffer>) -> Result<SubmissionIndex, RendererError> {
        if commands.is_empty() {
            return Err(RendererError::Validation(
                "RHI submissions require at least one command buffer".to_owned(),
            ));
        }

        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        for command in &commands {
            if !state.finished_command_buffers.contains(command) {
                return Err(RendererError::Validation(format!(
                    "unknown RHI command buffer: {}",
                    command.0
                )));
            }
            if state.submitted_command_buffers.contains(command) {
                return Err(RendererError::Validation(format!(
                    "RHI command buffer was already submitted: {}",
                    command.0
                )));
            }
        }
        for command in commands {
            state.submitted_command_buffers.insert(command);
        }

        let submission = SubmissionIndex(state.next_submission);
        state.next_submission += 1;
        Ok(submission)
    }

    fn poll(&self, mode: PollMode) {
        self.state
            .lock()
            .expect("headless RHI mutex poisoned")
            .last_poll = Some(mode);
    }
}

#[cfg(feature = "backend-wgpu")]
impl RhiDevice for WgpuRhiDevice {
    fn caps(&self) -> &RhiCaps {
        &self.caps
    }

    fn create_buffer(&self, desc: &RhiBufferDesc) -> Result<RhiBuffer, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.size == 0 {
            return Err(RendererError::Validation(
                "RHI buffers must have a non-zero size".to_owned(),
            ));
        }
        if desc.usage.is_empty() {
            return Err(RendererError::Validation(
                "RHI buffer usage must not be empty".to_owned(),
            ));
        }
        let handle = RhiBuffer(self.allocate());
        let backend_size = desc
            .size
            .checked_add(wgpu::COPY_BUFFER_ALIGNMENT - 1)
            .map(|value| value - (value % wgpu::COPY_BUFFER_ALIGNMENT))
            .ok_or_else(|| {
                RendererError::Validation("RHI buffer backend size overflows".to_owned())
            })?;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label.as_deref(),
            size: backend_size,
            usage: map_wgpu_buffer_usage(desc.usage),
            mapped_at_creation: false,
        });
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .buffers
            .insert(
                handle,
                WgpuBufferState {
                    buffer: Arc::new(buffer),
                    size: desc.size,
                    usage: desc.usage,
                },
            );
        Ok(handle)
    }

    fn buffer_usage(&self, buffer: RhiBuffer) -> Result<RhiBufferUsage, RendererError> {
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .buffers
            .get(&buffer)
            .map(|state| state.usage)
            .ok_or_else(|| RendererError::Validation(format!("unknown RHI buffer: {}", buffer.0)))
    }

    fn write_buffer(
        &self,
        buffer: RhiBuffer,
        offset: u64,
        data: &[u8],
    ) -> Result<(), RendererError> {
        let (wgpu_buffer, logical_size) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let buffer_state = state.buffers.get(&buffer).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI buffer: {}", buffer.0))
            })?;
            if !buffer_state.usage.contains(RhiBufferUsage::COPY_DST) {
                return Err(RendererError::Validation(
                    "RHI buffer write requires COPY_DST usage".to_owned(),
                ));
            }
            (Arc::clone(&buffer_state.buffer), buffer_state.size)
        };
        validate_buffer_range(logical_size, offset, data.len() as u64, "write")?;
        if data.is_empty() {
            return Ok(());
        }
        let alignment = wgpu::COPY_BUFFER_ALIGNMENT;
        if offset % alignment == 0 && data.len() as u64 % alignment == 0 {
            self.queue.write_buffer(&wgpu_buffer, offset, data);
            return Ok(());
        }
        let write_end = offset.checked_add(data.len() as u64).ok_or_else(|| {
            RendererError::Validation("RHI buffer write range overflows".to_owned())
        })?;
        let aligned_start = offset - (offset % alignment);
        let aligned_end = write_end
            .checked_add(alignment - 1)
            .map(|value| value - (value % alignment))
            .ok_or_else(|| {
                RendererError::Validation("RHI buffer aligned write range overflows".to_owned())
            })?;
        let buffer_size = wgpu_buffer.size();
        if aligned_end > buffer_size {
            return Err(RendererError::Validation(
                "unaligned backend-wgpu RHI buffer writes at the end of a non-aligned buffer are unsupported"
                    .to_owned(),
            ));
        }
        let aligned_size = aligned_end - aligned_start;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI buffer write read-modify-readback"),
            size: aligned_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI buffer write read-modify-readback"),
            });
        encoder.copy_buffer_to_buffer(
            &wgpu_buffer,
            aligned_start,
            &readback_buffer,
            0,
            aligned_size,
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);
        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI buffer write read-modify mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI buffer write read-modify callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let mut aligned_bytes = mapped.to_vec();
        drop(mapped);
        readback_buffer.unmap();
        let relative_start = (offset - aligned_start) as usize;
        let relative_end = relative_start + data.len();
        aligned_bytes[relative_start..relative_end].copy_from_slice(data);
        self.queue
            .write_buffer(&wgpu_buffer, aligned_start, &aligned_bytes);
        Ok(())
    }

    fn read_buffer(
        &self,
        buffer: RhiBuffer,
        offset: u64,
        size: u64,
    ) -> Result<Vec<u8>, RendererError> {
        if size == 0 {
            return Ok(Vec::new());
        }
        let (wgpu_buffer, logical_size) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let buffer_state = state.buffers.get(&buffer).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI buffer: {}", buffer.0))
            })?;
            if !buffer_state.usage.contains(RhiBufferUsage::COPY_SRC) {
                return Err(RendererError::Validation(
                    "RHI buffer read requires COPY_SRC usage".to_owned(),
                ));
            }
            (Arc::clone(&buffer_state.buffer), buffer_state.size)
        };
        validate_buffer_range(logical_size, offset, size, "read")?;
        let alignment = wgpu::COPY_BUFFER_ALIGNMENT;
        let read_end = offset.checked_add(size).ok_or_else(|| {
            RendererError::Validation("RHI buffer read range overflows".to_owned())
        })?;
        let aligned_start = offset - (offset % alignment);
        let aligned_end = read_end
            .checked_add(alignment - 1)
            .map(|value| value - (value % alignment))
            .ok_or_else(|| {
                RendererError::Validation("RHI buffer aligned read range overflows".to_owned())
            })?;
        let buffer_size = wgpu_buffer.size();
        if aligned_end > buffer_size {
            return Err(RendererError::Validation(
                "unaligned backend-wgpu RHI buffer reads at the end of a non-aligned buffer are unsupported"
                    .to_owned(),
            ));
        }
        let aligned_size = aligned_end - aligned_start;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI buffer readback"),
            size: aligned_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI buffer readback"),
            });
        encoder.copy_buffer_to_buffer(
            &wgpu_buffer,
            aligned_start,
            &readback_buffer,
            0,
            aligned_size,
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI buffer readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI buffer readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let relative_start = (offset - aligned_start) as usize;
        let relative_end = relative_start + size as usize;
        let bytes = mapped[relative_start..relative_end].to_vec();
        drop(mapped);
        readback_buffer.unmap();
        Ok(bytes)
    }

    fn create_texture(&self, desc: &RhiTextureDesc) -> Result<RhiTexture, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.width == 0 || desc.height == 0 {
            return Err(RendererError::Validation(
                "RHI textures must have non-zero dimensions".to_owned(),
            ));
        }
        if desc.samples == 0 || !desc.samples.is_power_of_two() {
            return Err(RendererError::Validation(
                "RHI texture samples must be a non-zero power of two".to_owned(),
            ));
        }
        if desc.usage.is_empty() {
            return Err(RendererError::Validation(
                "RHI texture usage must not be empty".to_owned(),
            ));
        }
        if desc.samples > 1
            && (desc.usage.contains(RhiTextureUsage::COPY_SRC)
                || desc.usage.contains(RhiTextureUsage::COPY_DST)
                || desc.usage.contains(RhiTextureUsage::STORAGE))
        {
            return Err(RendererError::Validation(
                "RHI multisampled textures do not support COPY_SRC, COPY_DST, or STORAGE usage"
                    .to_owned(),
            ));
        }
        if desc.samples > 1 && !desc.usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
            return Err(RendererError::Validation(
                "backend-wgpu RHI multisampled textures require RENDER_ATTACHMENT usage".to_owned(),
            ));
        }
        validate_texture_usage_format(desc.usage, desc.format)?;
        let handle = RhiTexture(self.allocate());
        let texture_format = map_rhi_texture_format(desc.format);
        if desc.samples > 1
            && !texture_format
                .guaranteed_format_features(self.device.features())
                .flags
                .sample_count_supported(desc.samples)
        {
            return Err(RendererError::Validation(format!(
                "backend-wgpu RHI texture format {:?} does not support {}x multisampling",
                desc.format, desc.samples
            )));
        }
        let mut usage = map_wgpu_texture_usage(desc.usage);
        if desc.format == TextureFormat::Depth32Float
            && desc.usage.contains(RhiTextureUsage::COPY_DST)
        {
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: desc.label.as_deref(),
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: desc.samples,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage,
            view_formats: &[],
        });
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .textures
            .insert(
                handle,
                WgpuTextureState {
                    texture: Arc::new(texture),
                    width: desc.width,
                    height: desc.height,
                    format: desc.format,
                    usage: desc.usage,
                    samples: desc.samples,
                },
            );
        Ok(handle)
    }

    fn texture_usage(&self, texture: RhiTexture) -> Result<RhiTextureUsage, RendererError> {
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .textures
            .get(&texture)
            .map(|state| state.usage)
            .ok_or_else(|| RendererError::Validation(format!("unknown RHI texture: {}", texture.0)))
    }

    fn texture_samples(&self, texture: RhiTexture) -> Result<u32, RendererError> {
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .textures
            .get(&texture)
            .map(|state| state.samples)
            .ok_or_else(|| RendererError::Validation(format!("unknown RHI texture: {}", texture.0)))
    }

    fn custom_resolve_support(&self) -> RhiCustomResolveSupport {
        RhiCustomResolveSupport::backend_wgpu()
    }

    fn resolve_texture_rgba8(
        &self,
        source: RhiTexture,
        target: RhiTexture,
    ) -> Result<(), RendererError> {
        let (source_texture, target_texture) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(source_state) = state.textures.get(&source) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve source texture: {}",
                    source.0
                )));
            };
            let Some(target_state) = state.textures.get(&target) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve target texture: {}",
                    target.0
                )));
            };
            validate_wgpu_rgba8_resolve_textures(source, source_state, target, target_state)?;
            (
                Arc::clone(&source_state.texture),
                Arc::clone(&target_state.texture),
            )
        };
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI explicit RGBA8 MSAA resolve"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RHI explicit RGBA8 MSAA resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &source_view,
                    resolve_target: Some(&target_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn resolve_texture_rgba8_with_mode(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        mode: RhiResolveMode,
    ) -> Result<(), RendererError> {
        match mode {
            RhiResolveMode::Average => self.resolve_texture_rgba8(source, target),
            RhiResolveMode::FirstSample | RhiResolveMode::Sample(_) => {
                let sample_index = match mode {
                    RhiResolveMode::FirstSample => 0,
                    RhiResolveMode::Sample(sample_index) => sample_index,
                    RhiResolveMode::Average => unreachable!("average mode is handled above"),
                };
                let (source_texture, target_texture, width, height) = {
                    let state = self.state.lock().expect("wgpu RHI mutex poisoned");
                    let Some(source_state) = state.textures.get(&source) else {
                        return Err(RendererError::Validation(format!(
                            "unknown RHI resolve source texture: {}",
                            source.0
                        )));
                    };
                    let Some(target_state) = state.textures.get(&target) else {
                        return Err(RendererError::Validation(format!(
                            "unknown RHI resolve target texture: {}",
                            target.0
                        )));
                    };
                    validate_wgpu_rgba8_custom_resolve_textures(
                        source,
                        source_state,
                        target,
                        target_state,
                    )?;
                    if sample_index >= source_state.samples {
                        return Err(RendererError::Validation(format!(
                            "RHI custom resolve sample index {sample_index} exceeds source sample count {}",
                            source_state.samples
                        )));
                    }
                    (
                        Arc::clone(&source_state.texture),
                        Arc::clone(&target_state.texture),
                        source_state.width,
                        source_state.height,
                    )
                };
                let source_view =
                    source_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let target_view =
                    target_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let shader_source = format!(
                    r#"
                        const SAMPLE_INDEX: i32 = {sample_index};

                        @group(0) @binding(0)
                        var source_tex: texture_multisampled_2d<f32>;

                        @group(0) @binding(1)
                        var target_tex: texture_storage_2d<rgba8unorm, write>;

                        @compute @workgroup_size(8, 8)
                        fn main(@builtin(global_invocation_id) id: vec3<u32>) {{
                            let dims = textureDimensions(target_tex);
                            if (id.x >= dims.x || id.y >= dims.y) {{
                                return;
                            }}
                            let value = textureLoad(source_tex, vec2<i32>(i32(id.x), i32(id.y)), SAMPLE_INDEX);
                            textureStore(target_tex, vec2<i32>(i32(id.x), i32(id.y)), value);
                        }}
                    "#
                );
                let shader = self
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("RHI first-sample RGBA8 MSAA resolve shader"),
                        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
                    });
                let bind_group_layout =
                    self.device
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some("RHI first-sample RGBA8 MSAA resolve layout"),
                            entries: &[
                                wgpu::BindGroupLayoutEntry {
                                    binding: 0,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::Texture {
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: false,
                                        },
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                        multisampled: true,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 1,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::StorageTexture {
                                        access: wgpu::StorageTextureAccess::WriteOnly,
                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                    },
                                    count: None,
                                },
                            ],
                        });
                let pipeline_layout =
                    self.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("RHI first-sample RGBA8 MSAA resolve pipeline layout"),
                            bind_group_layouts: &[&bind_group_layout],
                            push_constant_ranges: &[],
                        });
                let pipeline =
                    self.device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("RHI first-sample RGBA8 MSAA resolve pipeline"),
                            layout: Some(&pipeline_layout),
                            module: &shader,
                            entry_point: "main",
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                        });
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("RHI first-sample RGBA8 MSAA resolve bind group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&source_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&target_view),
                        },
                    ],
                });
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("RHI first-sample RGBA8 MSAA resolve"),
                        });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("RHI first-sample RGBA8 MSAA resolve"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
                }
                self.queue.submit([encoder.finish()]);
                Ok(())
            }
        }
    }

    fn resolve_texture_rgba8_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        let (source_texture, target_texture, width, height) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(source_state) = state.textures.get(&source) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve source texture: {}",
                    source.0
                )));
            };
            let Some(target_state) = state.textures.get(&target) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve target texture: {}",
                    target.0
                )));
            };
            validate_wgpu_rgba8_custom_resolve_textures(
                source,
                source_state,
                target,
                target_state,
            )?;
            (
                Arc::clone(&source_state.texture),
                Arc::clone(&target_state.texture),
                source_state.width,
                source_state.height,
            )
        };
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: shader.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(shader.source.clone().into()),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RHI custom RGBA8 MSAA resolve layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: true,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RHI custom RGBA8 MSAA resolve pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: shader.label.as_deref(),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: &shader.entry_point,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RHI custom RGBA8 MSAA resolve bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&target_view),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI custom RGBA8 MSAA resolve"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RHI custom RGBA8 MSAA resolve"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn resolve_texture_rgba16f_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        let (source_texture, target_texture, width, height) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(source_state) = state.textures.get(&source) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve source texture: {}",
                    source.0
                )));
            };
            let Some(target_state) = state.textures.get(&target) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve target texture: {}",
                    target.0
                )));
            };
            validate_wgpu_rgba16f_custom_resolve_textures(
                source,
                source_state,
                target,
                target_state,
            )?;
            (
                Arc::clone(&source_state.texture),
                Arc::clone(&target_state.texture),
                source_state.width,
                source_state.height,
            )
        };
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: shader.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(shader.source.clone().into()),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RHI custom RGBA16F MSAA resolve layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: true,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba16Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RHI custom RGBA16F MSAA resolve pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: shader.label.as_deref(),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: &shader.entry_point,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RHI custom RGBA16F MSAA resolve bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&target_view),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI custom RGBA16F MSAA resolve"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RHI custom RGBA16F MSAA resolve"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn resolve_texture_rgba32f_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        let (source_texture, target_texture, width, height) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(source_state) = state.textures.get(&source) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve source texture: {}",
                    source.0
                )));
            };
            let Some(target_state) = state.textures.get(&target) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve target texture: {}",
                    target.0
                )));
            };
            validate_wgpu_rgba32f_custom_resolve_textures(
                source,
                source_state,
                target,
                target_state,
            )?;
            (
                Arc::clone(&source_state.texture),
                Arc::clone(&target_state.texture),
                source_state.width,
                source_state.height,
            )
        };
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: shader.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(shader.source.clone().into()),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RHI custom RGBA32F MSAA resolve layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: true,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba32Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RHI custom RGBA32F MSAA resolve pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: shader.label.as_deref(),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: &shader.entry_point,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RHI custom RGBA32F MSAA resolve bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&target_view),
                },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI custom RGBA32F MSAA resolve"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RHI custom RGBA32F MSAA resolve"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn resolve_texture_8bit_color_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        let (source_texture, target_texture, target_format) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(source_state) = state.textures.get(&source) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve source texture: {}",
                    source.0
                )));
            };
            let Some(target_state) = state.textures.get(&target) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve target texture: {}",
                    target.0
                )));
            };
            validate_wgpu_8bit_color_fragment_resolve_textures(
                source,
                source_state,
                target,
                target_state,
            )?;
            (
                Arc::clone(&source_state.texture),
                Arc::clone(&target_state.texture),
                map_color_texture_format(target_state.format)?,
            )
        };
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("RHI custom 8-bit color MSAA resolve fullscreen vertex"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
                        @vertex
                        fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                            let x = f32((vertex_index << 1u) & 2u);
                            let y = f32(vertex_index & 2u);
                            return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                        }
                    "#
                    .into(),
                ),
            });
        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: shader.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(shader.source.clone().into()),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RHI custom 8-bit color MSAA resolve layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    }],
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RHI custom 8-bit color MSAA resolve pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let color_targets = [Some(wgpu::ColorTargetState {
            format: target_format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: shader.label.as_deref(),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: "vs",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: &shader.entry_point,
                    targets: &color_targets,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview: None,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RHI custom 8-bit color MSAA resolve bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            }],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI custom 8-bit color MSAA resolve"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RHI custom 8-bit color MSAA resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);
        Ok(())
    }

    fn resolve_texture_depth32f_with_shader(
        &self,
        source: RhiTexture,
        target: RhiTexture,
        shader: &RhiResolveShaderDesc,
    ) -> Result<(), RendererError> {
        validate_resolve_shader_desc(shader)?;
        let (source_texture, target_texture) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(source_state) = state.textures.get(&source) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve source texture: {}",
                    source.0
                )));
            };
            let Some(target_state) = state.textures.get(&target) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI resolve target texture: {}",
                    target.0
                )));
            };
            validate_wgpu_depth32f_custom_resolve_textures(
                source,
                source_state,
                target,
                target_state,
            )?;
            (
                Arc::clone(&source_state.texture),
                Arc::clone(&target_state.texture),
            )
        };
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("RHI custom Depth32F MSAA resolve fullscreen vertex"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
                        @vertex
                        fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                            let x = f32((vertex_index << 1u) & 2u);
                            let y = f32(vertex_index & 2u);
                            return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                        }
                    "#
                    .into(),
                ),
            });
        let fragment_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: shader.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(shader.source.clone().into()),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RHI custom Depth32F MSAA resolve layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    }],
                });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RHI custom Depth32F MSAA resolve pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: shader.label.as_deref(),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: "vs",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: &shader.entry_point,
                    targets: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview: None,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RHI custom Depth32F MSAA resolve bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            }],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI custom Depth32F MSAA resolve"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RHI custom Depth32F MSAA resolve"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);
        Ok(())
    }

    fn write_texture_rgba8(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[u8],
    ) -> Result<(), RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
                return Err(RendererError::Validation(
                    "RHI texture write requires COPY_DST usage".to_owned(),
                ));
            }
            if !is_rhi_8bit_color_texture_format(texture_state.format) {
                return Err(RendererError::Validation(
                    "RHI RGBA8 texture writes require an 8-bit color format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let expected = rgba8_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI RGBA8 texture write expected {expected} bytes but received {}",
                data.len()
            )));
        }
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(region.width * RGBA8_BYTES_PER_PIXEL),
                rows_per_image: Some(region.height),
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    fn write_texture_rgba16f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[u16],
    ) -> Result<(), RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
                return Err(RendererError::Validation(
                    "RHI texture write requires COPY_DST usage".to_owned(),
                ));
            }
            if texture_state.format != TextureFormat::Rgba16Float {
                return Err(RendererError::Validation(
                    "RHI RGBA16F texture writes require Rgba16Float format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let expected = rgba16f_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI RGBA16F texture write expected {expected} channels but received {}",
                data.len()
            )));
        }
        let bytes = data
            .iter()
            .flat_map(|channel| channel.to_le_bytes())
            .collect::<Vec<_>>();
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(region.width * RGBA16F_BYTES_PER_PIXEL),
                rows_per_image: Some(region.height),
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    fn write_texture_rgba32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[f32],
    ) -> Result<(), RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
                return Err(RendererError::Validation(
                    "RHI texture write requires COPY_DST usage".to_owned(),
                ));
            }
            if texture_state.format != TextureFormat::Rgba32Float {
                return Err(RendererError::Validation(
                    "RHI RGBA32F texture writes require Rgba32Float format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let expected = rgba32f_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI RGBA32F texture write expected {expected} channels but received {}",
                data.len()
            )));
        }
        if data.iter().any(|value| !value.is_finite()) {
            return Err(RendererError::Validation(
                "RHI RGBA32F texture writes require finite values".to_owned(),
            ));
        }
        let bytes = data
            .iter()
            .flat_map(|channel| channel.to_le_bytes())
            .collect::<Vec<_>>();
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(region.width * RGBA32F_BYTES_PER_PIXEL),
                rows_per_image: Some(region.height),
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    fn write_texture_depth32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
        data: &[f32],
    ) -> Result<(), RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_DST) {
                return Err(RendererError::Validation(
                    "RHI texture write requires COPY_DST usage".to_owned(),
                ));
            }
            if texture_state.format != TextureFormat::Depth32Float {
                return Err(RendererError::Validation(
                    "RHI depth32f texture writes require Depth32Float format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let expected = depth32f_region_len(region)?;
        if data.len() != expected {
            return Err(RendererError::Validation(format!(
                "RHI depth32f texture write expected {expected} values but received {}",
                data.len()
            )));
        }
        if data.iter().any(|value| !value.is_finite()) {
            return Err(RendererError::Validation(
                "RHI depth32f texture writes require finite values".to_owned(),
            ));
        }
        let byte_len = data
            .len()
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or_else(|| {
                RendererError::Validation("RHI depth32f write size overflows".to_owned())
            })?;
        let data_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI depth32f write values"),
            size: byte_len as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: true,
        });
        {
            let mut mapped = data_buffer.slice(..).get_mapped_range_mut();
            for (index, value) in data.iter().enumerate() {
                let byte_start = index * std::mem::size_of::<f32>();
                mapped[byte_start..byte_start + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        data_buffer.unmap();
        let mut params = [0_u8; 16];
        params[0..4].copy_from_slice(&region.x.to_le_bytes());
        params[4..8].copy_from_slice(&region.y.to_le_bytes());
        params[8..12].copy_from_slice(&region.width.to_le_bytes());
        params[12..16].copy_from_slice(&region.height.to_le_bytes());
        let params_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI depth32f write params"),
            size: params.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        params_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(&params);
        params_buffer.unmap();
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("RHI depth32f write shader"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
struct Params {
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<storage, read> depth_values: array<f32>;

@group(0) @binding(1)
var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @builtin(frag_depth) f32 {
    let x = u32(position.x) - params.origin_x;
    let y = u32(position.y) - params.origin_y;
    let index = y * params.width + x;
    return depth_values[index];
}
"#
                    .into(),
                ),
            });
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RHI depth32f write bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RHI depth32f write bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: data_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("RHI depth32f write pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("RHI depth32f write pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview: None,
            });
        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI depth32f write encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RHI depth32f write pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_scissor_rect(region.x, region.y, region.width, region.height);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);
        Ok(())
    }

    fn read_texture_rgba8(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<u8>, RendererError> {
        let (wgpu_texture, samples) = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if texture_state.samples == 1
                && !texture_state.usage.contains(RhiTextureUsage::COPY_SRC)
            {
                return Err(RendererError::Validation(
                    "RHI texture read requires COPY_SRC usage".to_owned(),
                ));
            }
            if texture_state.samples > 1
                && !texture_state
                    .usage
                    .contains(RhiTextureUsage::RENDER_ATTACHMENT)
            {
                return Err(RendererError::Validation(
                    "RHI multisampled texture read requires RENDER_ATTACHMENT usage for resolve"
                        .to_owned(),
                ));
            }
            if !is_rhi_8bit_color_texture_format(texture_state.format) {
                return Err(RendererError::Validation(
                    "RHI RGBA8 texture reads require an 8-bit color format".to_owned(),
                ));
            }
            (Arc::clone(&texture_state.texture), texture_state.samples)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let read_source = if samples > 1 {
            let resolve = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("RHI multisampled texture resolve"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu_texture.format(),
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let source_view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let resolve_view = resolve.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("RHI multisampled texture resolve"),
                });
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("RHI multisampled texture resolve"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &source_view,
                        resolve_target: Some(&resolve_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Discard,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            self.queue.submit([encoder.finish()]);
            self.device.poll(wgpu::Maintain::Wait);
            Arc::new(resolve)
        } else {
            Arc::clone(&wgpu_texture)
        };
        let row_bytes = region.width * RGBA8_BYTES_PER_PIXEL;
        let padded_row_bytes = align_to(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_size = padded_row_bytes as u64 * region.height as u64;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI texture readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI texture readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &read_source,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(region.height),
                },
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI texture readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI texture readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let mut rgba8 = Vec::with_capacity(rgba8_region_len(region)?);
        for row in 0..region.height as usize {
            let row_start = row * padded_row_bytes as usize;
            let row_end = row_start + row_bytes as usize;
            rgba8.extend_from_slice(&mapped[row_start..row_end]);
        }
        drop(mapped);
        readback_buffer.unmap();
        Ok(rgba8)
    }

    fn read_texture_rgba16f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<u16>, RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
                return Err(RendererError::Validation(
                    "RHI texture read requires COPY_SRC usage".to_owned(),
                ));
            }
            if texture_state.format != TextureFormat::Rgba16Float {
                return Err(RendererError::Validation(
                    "RHI RGBA16F texture reads require Rgba16Float format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let row_bytes = region.width * RGBA16F_BYTES_PER_PIXEL;
        let padded_row_bytes = align_to(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_size = padded_row_bytes as u64 * region.height as u64;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI RGBA16F texture readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI RGBA16F texture readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(region.height),
                },
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI RGBA16F texture readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI RGBA16F texture readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let mut channels = Vec::with_capacity(rgba16f_region_len(region)?);
        for row in 0..region.height as usize {
            let row_start = row * padded_row_bytes as usize;
            let row_end = row_start + row_bytes as usize;
            for bytes in mapped[row_start..row_end].chunks_exact(2) {
                channels.push(u16::from_le_bytes([bytes[0], bytes[1]]));
            }
        }
        drop(mapped);
        readback_buffer.unmap();
        Ok(channels)
    }

    fn read_texture_rgba32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<f32>, RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
                return Err(RendererError::Validation(
                    "RHI texture read requires COPY_SRC usage".to_owned(),
                ));
            }
            if texture_state.format != TextureFormat::Rgba32Float {
                return Err(RendererError::Validation(
                    "RHI RGBA32F texture reads require Rgba32Float format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let row_bytes = region.width * RGBA32F_BYTES_PER_PIXEL;
        let padded_row_bytes = align_to(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_size = padded_row_bytes as u64 * region.height as u64;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI RGBA32F texture readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI RGBA32F texture readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: region.x,
                    y: region.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(region.height),
                },
            },
            wgpu::Extent3d {
                width: region.width,
                height: region.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI RGBA32F texture readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI RGBA32F texture readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let mut channels = Vec::with_capacity(rgba32f_region_len(region)?);
        for row in 0..region.height as usize {
            let row_start = row * padded_row_bytes as usize;
            let row_end = row_start + row_bytes as usize;
            for bytes in mapped[row_start..row_end].chunks_exact(4) {
                channels.push(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
            }
        }
        drop(mapped);
        readback_buffer.unmap();
        Ok(channels)
    }

    fn read_texture_depth32f(
        &self,
        texture: RhiTexture,
        region: RhiTextureRegion,
    ) -> Result<Vec<f32>, RendererError> {
        let wgpu_texture = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let texture_state = state.textures.get(&texture).ok_or_else(|| {
                RendererError::Validation(format!("unknown RHI texture: {}", texture.0))
            })?;
            if !texture_state.usage.contains(RhiTextureUsage::COPY_SRC) {
                return Err(RendererError::Validation(
                    "RHI texture read requires COPY_SRC usage".to_owned(),
                ));
            }
            if texture_state.format != TextureFormat::Depth32Float {
                return Err(RendererError::Validation(
                    "RHI depth texture reads require Depth32Float format".to_owned(),
                ));
            }
            Arc::clone(&texture_state.texture)
        };
        let size = wgpu_texture.size();
        validate_texture_region(size.width, size.height, region)?;
        let row_bytes = size.width * DEPTH32F_BYTES_PER_PIXEL;
        let padded_row_bytes = align_to(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_size = padded_row_bytes as u64 * size.height as u64;
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RHI depth32f texture readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RHI depth32f texture readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: wgpu::TextureAspect::DepthOnly,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes),
                    rows_per_image: Some(size.height),
                },
            },
            wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::Maintain::Wait);

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI depth32f texture readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI depth32f texture readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let mut values = Vec::with_capacity(depth32f_region_len(region)?);
        for row in 0..region.height as usize {
            let row_start = (region.y as usize + row) * padded_row_bytes as usize;
            let value_start = row_start + region.x as usize * DEPTH32F_BYTES_PER_PIXEL as usize;
            let value_end = value_start + region.width as usize * DEPTH32F_BYTES_PER_PIXEL as usize;
            for bytes in mapped[value_start..value_end].chunks_exact(4) {
                values.push(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
            }
        }
        drop(mapped);
        readback_buffer.unmap();
        Ok(values)
    }

    fn create_sampler(&self, desc: &RhiSamplerDesc) -> Result<RhiSampler, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiSampler(self.allocate());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: desc.label.as_deref(),
            ..wgpu::SamplerDescriptor::default()
        });
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .samplers
            .insert(handle, Arc::new(sampler));
        Ok(handle)
    }

    fn create_bind_group(&self, desc: &RhiBindGroupDesc) -> Result<RhiBindGroup, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        validate_bind_group_desc_label_and_entries(desc)?;
        let handle = RhiBindGroup(self.allocate());
        let bind_group = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI graphics pipeline for bind group: {}",
                    desc.pipeline.0
                )));
            };
            let layout = pipeline.pipeline.get_bind_group_layout(desc.group_index);
            let mut owned_resources = Vec::with_capacity(desc.entries.len());
            for entry in &desc.entries {
                match *entry {
                    RhiBindGroupEntry::Texture { texture, .. } => {
                        let Some(texture_state) = state.textures.get(&texture) else {
                            return Err(RendererError::Validation(format!(
                                "unknown RHI bind group texture: {}",
                                texture.0
                            )));
                        };
                        if !texture_state.usage.contains(RhiTextureUsage::SAMPLED) {
                            return Err(RendererError::Validation(
                                "RHI bind group texture requires SAMPLED usage".to_owned(),
                            ));
                        }
                        owned_resources.push(WgpuBindGroupOwnedResource::TextureView(
                            texture_state
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ));
                    }
                    RhiBindGroupEntry::Sampler { sampler, .. } => {
                        let Some(sampler) = state.samplers.get(&sampler) else {
                            return Err(RendererError::Validation(format!(
                                "unknown RHI bind group sampler: {}",
                                sampler.0
                            )));
                        };
                        owned_resources
                            .push(WgpuBindGroupOwnedResource::Sampler(Arc::clone(sampler)));
                    }
                    RhiBindGroupEntry::Buffer { buffer, .. } => {
                        let Some(buffer_state) = state.buffers.get(&buffer) else {
                            return Err(RendererError::Validation(format!(
                                "unknown RHI bind group buffer: {}",
                                buffer.0
                            )));
                        };
                        validate_bind_group_buffer_usage(buffer_state.usage)?;
                        owned_resources.push(WgpuBindGroupOwnedResource::Buffer(Arc::clone(
                            &buffer_state.buffer,
                        )));
                    }
                }
            }
            let entries = desc
                .entries
                .iter()
                .zip(&owned_resources)
                .map(|(entry, resource)| {
                    let binding = match entry {
                        RhiBindGroupEntry::Texture { binding, .. }
                        | RhiBindGroupEntry::Sampler { binding, .. }
                        | RhiBindGroupEntry::Buffer { binding, .. } => *binding,
                    };
                    let resource = match resource {
                        WgpuBindGroupOwnedResource::TextureView(view) => {
                            wgpu::BindingResource::TextureView(view)
                        }
                        WgpuBindGroupOwnedResource::Sampler(sampler) => {
                            wgpu::BindingResource::Sampler(sampler)
                        }
                        WgpuBindGroupOwnedResource::Buffer(buffer) => buffer.as_entire_binding(),
                    };
                    wgpu::BindGroupEntry { binding, resource }
                })
                .collect::<Vec<_>>();
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label.as_deref(),
                layout: &layout,
                entries: &entries,
            })
        };
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .bind_groups
            .insert(handle, WgpuBindGroupState::new_graphics(bind_group, desc));
        Ok(handle)
    }

    fn create_compute_bind_group(
        &self,
        desc: &RhiComputeBindGroupDesc,
    ) -> Result<RhiBindGroup, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        validate_bind_group_entries(&desc.entries)?;
        let handle = RhiBindGroup(self.allocate());
        let bind_group = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(pipeline) = state.compute_pipelines.get(&desc.pipeline) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI compute pipeline for bind group: {}",
                    desc.pipeline.0
                )));
            };
            validate_wgpu_compute_bind_group_entries(&state, desc)?;
            let layout = pipeline.get_bind_group_layout(desc.group_index);
            let owned_resources = wgpu_bind_group_owned_resources(&state, &desc.entries)?;
            let entries = wgpu_bind_group_entries(&desc.entries, &owned_resources);
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: desc.label.as_deref(),
                layout: &layout,
                entries: &entries,
            })
        };
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .bind_groups
            .insert(handle, WgpuBindGroupState::new_compute(bind_group, desc));
        Ok(handle)
    }

    fn create_shader_module(
        &self,
        desc: &RhiShaderModuleDesc,
    ) -> Result<RhiShaderModule, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.source.trim().is_empty() {
            return Err(RendererError::ShaderCompile(
                "RHI shader source must not be empty".to_owned(),
            ));
        }
        let handle = RhiShaderModule(self.allocate());
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: desc.label.as_deref(),
                source: wgpu::ShaderSource::Wgsl(desc.source.clone().into()),
            });
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .shader_modules
            .insert(handle, module);
        Ok(handle)
    }

    fn create_graphics_pipeline(
        &self,
        desc: &RhiGraphicsPipelineDesc,
    ) -> Result<RhiGraphicsPipeline, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        validate_graphics_pipeline_desc(desc)?;
        let handle = RhiGraphicsPipeline(self.allocate());
        let pipeline = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let vertex_shader = Self::require_shader_module(&state, desc.vertex_shader)?;
            let fragment_shader = desc
                .fragment_shader
                .map(|shader| Self::require_shader_module(&state, shader))
                .transpose()?;
            let vertex_attributes = desc
                .vertex_buffers
                .iter()
                .map(|layout| {
                    layout
                        .attributes
                        .iter()
                        .map(|attribute| {
                            Ok(wgpu::VertexAttribute {
                                format: map_vertex_format(attribute.format)?,
                                offset: attribute.offset,
                                shader_location: attribute.location,
                            })
                        })
                        .collect::<Result<Vec<_>, RendererError>>()
                })
                .collect::<Result<Vec<_>, RendererError>>()?;
            let vertex_buffers = desc
                .vertex_buffers
                .iter()
                .zip(&vertex_attributes)
                .map(|(layout, attributes)| wgpu::VertexBufferLayout {
                    array_stride: layout.stride,
                    step_mode: map_vertex_step_mode(layout.step_mode),
                    attributes,
                })
                .collect::<Vec<_>>();
            let color_targets = desc
                .color_format
                .map(|format| {
                    Ok(Some(wgpu::ColorTargetState {
                        format: map_color_texture_format(format)?,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }))
                })
                .transpose()?
                .into_iter()
                .collect::<Vec<_>>();
            let fragment = fragment_shader.map(|module| wgpu::FragmentState {
                module,
                entry_point: desc
                    .fragment_entry
                    .as_deref()
                    .expect("fragment entry is validated when fragment shader is set"),
                targets: &color_targets,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });
            let depth_stencil = desc.depth.map(|depth| wgpu::DepthStencilState {
                format: map_depth_format(depth.format),
                depth_write_enabled: depth.write_enabled,
                depth_compare: map_compare_function(depth.compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            });
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: desc.label.as_deref(),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: vertex_shader,
                        entry_point: &desc.vertex_entry,
                        buffers: &vertex_buffers,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: map_primitive_topology(desc.primitive.topology),
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: desc.primitive.cull_mode.map(map_face),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil,
                    multisample: wgpu::MultisampleState {
                        count: desc.sample_count,
                        ..wgpu::MultisampleState::default()
                    },
                    fragment,
                    multiview: None,
                })
        };
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .graphics_pipelines
            .insert(handle, WgpuGraphicsPipelineState::new(pipeline, desc));
        Ok(handle)
    }

    fn create_compute_pipeline(
        &self,
        desc: &RhiComputePipelineDesc,
    ) -> Result<RhiComputePipeline, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        if desc.entry_point.trim().is_empty() {
            return Err(RendererError::PipelineCompile(
                "RHI compute pipeline entry point must not be empty".to_owned(),
            ));
        }
        let handle = RhiComputePipeline(self.allocate());
        let pipeline = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            let Some(module) = state.shader_modules.get(&desc.shader) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI shader module: {}",
                    desc.shader.0
                )));
            };
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: desc.label.as_deref(),
                    layout: None,
                    module,
                    entry_point: &desc.entry_point,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                })
        };
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .compute_pipelines
            .insert(handle, pipeline);
        Ok(handle)
    }

    fn create_timestamp_query(
        &self,
        desc: &RhiTimestampQueryDesc,
    ) -> Result<RhiTimestampQuery, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiTimestampQuery(self.allocate());
        let resource = if self
            .device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            let query_set = self.device.create_query_set(&wgpu::QuerySetDescriptor {
                label: desc.label.as_deref(),
                ty: wgpu::QueryType::Timestamp,
                count: 1,
            });
            let resolve_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: desc.label.as_deref(),
                size: std::mem::size_of::<u64>() as u64,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: desc.label.as_deref(),
                size: std::mem::size_of::<u64>() as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            WgpuTimestampQueryResource {
                query_set: Some(Arc::new(query_set)),
                resolve_buffer: Some(Arc::new(resolve_buffer)),
                readback_buffer: Some(Arc::new(readback_buffer)),
            }
        } else {
            WgpuTimestampQueryResource {
                query_set: None,
                resolve_buffer: None,
                readback_buffer: None,
            }
        };
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .timestamp_queries
            .insert(handle, resource);
        Ok(handle)
    }

    fn timestamp_result(
        &self,
        query: RhiTimestampQuery,
    ) -> Result<RhiTimestampResult, RendererError> {
        let readback_buffer = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            if !state.timestamp_queries.contains_key(&query) {
                return Err(RendererError::Validation(format!(
                    "unknown RHI timestamp query: {}",
                    query.0
                )));
            }
            if let Some(timestamp_ns) = state.timestamp_results.get(&query).copied() {
                return Ok(RhiTimestampResult {
                    query,
                    timestamp_ns,
                    available: true,
                });
            }
            state
                .timestamp_queries
                .get(&query)
                .and_then(|resource| resource.readback_buffer.clone())
        };

        let Some(readback_buffer) = readback_buffer else {
            return Ok(RhiTimestampResult {
                query,
                timestamp_ns: 0,
                available: false,
            });
        };

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI timestamp readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI timestamp readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let ticks = u64::from_le_bytes(
            mapped
                .get(..std::mem::size_of::<u64>())
                .expect("timestamp readback buffer contains one u64")
                .try_into()
                .expect("timestamp readback slice is exactly one u64"),
        );
        drop(mapped);
        readback_buffer.unmap();

        let timestamp_ns = (ticks as f64 * self.queue.get_timestamp_period() as f64) as u64;
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .timestamp_results
            .insert(query, timestamp_ns);

        Ok(RhiTimestampResult {
            query,
            timestamp_ns,
            available: true,
        })
    }

    fn create_pipeline_statistics_query(
        &self,
        desc: &RhiPipelineStatisticsQueryDesc,
    ) -> Result<RhiPipelineStatisticsQuery, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiPipelineStatisticsQuery(self.allocate());
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .pipeline_statistics_queries
            .insert(handle);
        Ok(handle)
    }

    fn pipeline_statistics_result(
        &self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<RhiPipelineStatisticsResult, RendererError> {
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        if !state.pipeline_statistics_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI pipeline statistics query: {}",
                query.0
            )));
        }
        Ok(RhiPipelineStatisticsResult {
            query,
            statistics: state
                .pipeline_statistics_results
                .get(&query)
                .cloned()
                .unwrap_or_default(),
            available: state.pipeline_statistics_results.contains_key(&query),
        })
    }

    fn create_occlusion_query(
        &self,
        desc: &RhiOcclusionQueryDesc,
    ) -> Result<RhiOcclusionQuery, RendererError> {
        Self::validate_label(desc.label.as_deref())?;
        let handle = RhiOcclusionQuery(self.allocate());
        let query_set = self.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: desc.label.as_deref(),
            ty: wgpu::QueryType::Occlusion,
            count: 1,
        });
        let resolve_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label.as_deref(),
            size: std::mem::size_of::<u64>() as u64,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: desc.label.as_deref(),
            size: std::mem::size_of::<u64>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .occlusion_queries
            .insert(
                handle,
                WgpuOcclusionQueryResource {
                    query_set: Arc::new(query_set),
                    resolve_buffer: Arc::new(resolve_buffer),
                    readback_buffer: Arc::new(readback_buffer),
                },
            );
        Ok(handle)
    }

    fn occlusion_result(
        &self,
        query: RhiOcclusionQuery,
    ) -> Result<RhiOcclusionQueryResult, RendererError> {
        let readback_buffer = {
            let state = self.state.lock().expect("wgpu RHI mutex poisoned");
            if !state.occlusion_queries.contains_key(&query) {
                return Err(RendererError::Validation(format!(
                    "unknown RHI occlusion query: {}",
                    query.0
                )));
            }
            if let Some(samples_passed) = state.occlusion_results.get(&query).copied() {
                return Ok(RhiOcclusionQueryResult {
                    query,
                    samples_passed,
                    visible: samples_passed > 0,
                    available: true,
                });
            }
            Arc::clone(&state.occlusion_queries[&query].readback_buffer)
        };

        let slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                return Err(RendererError::Backend(format!(
                    "RHI occlusion readback mapping failed: {err}"
                )));
            }
            Err(_) => {
                return Err(RendererError::Backend(
                    "RHI occlusion readback callback was canceled".to_owned(),
                ));
            }
        }
        let mapped = slice.get_mapped_range();
        let samples_passed = u64::from_le_bytes(
            mapped
                .get(..std::mem::size_of::<u64>())
                .expect("occlusion readback buffer contains one u64")
                .try_into()
                .expect("occlusion readback slice is exactly one u64"),
        );
        drop(mapped);
        readback_buffer.unmap();

        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .occlusion_results
            .insert(query, samples_passed);
        Ok(RhiOcclusionQueryResult {
            query,
            samples_passed,
            visible: samples_passed > 0,
            available: true,
        })
    }

    fn create_command_encoder(
        &self,
        label: Option<&str>,
    ) -> Result<Box<dyn RhiCommandEncoder>, RendererError> {
        Self::validate_label(label)?;
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label });
        Ok(Box::new(WgpuRhiCommandEncoder {
            state: Arc::clone(&self.state),
            encoder: Some(encoder),
            encoded_barriers: 0,
            encoded_compute_dispatches: 0,
            encoded_render_draws: 0,
            encoded_indirect_draws: 0,
            encoded_debug_groups: 0,
            open_debug_groups: 0,
            timestamp_writes: Vec::new(),
            active_pipeline_statistics: HashMap::new(),
            pipeline_statistics_writes: Vec::new(),
            active_occlusion_queries: HashMap::new(),
            occlusion_gpu_writes: HashSet::new(),
            occlusion_writes: Vec::new(),
        }))
    }

    fn submit(&self, commands: Vec<RhiCommandBuffer>) -> Result<SubmissionIndex, RendererError> {
        if commands.is_empty() {
            return Err(RendererError::Validation(
                "RHI submissions require at least one command buffer".to_owned(),
            ));
        }

        let mut state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let mut wgpu_commands = Vec::with_capacity(commands.len());
        for command in &commands {
            if state.submitted_command_buffers.contains(command) {
                return Err(RendererError::Validation(format!(
                    "RHI command buffer was already submitted: {}",
                    command.0
                )));
            }
            let Some(command_buffer) = state.finished_command_buffers.remove(command) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI command buffer: {}",
                    command.0
                )));
            };
            wgpu_commands.push(command_buffer);
        }
        for command in commands {
            state.submitted_command_buffers.insert(command);
        }
        self.queue.submit(wgpu_commands);
        let submission = SubmissionIndex(state.next_submission);
        state.next_submission += 1;
        Ok(submission)
    }

    fn poll(&self, mode: PollMode) {
        let maintain = match mode {
            PollMode::Poll => wgpu::Maintain::Poll,
            PollMode::Wait => wgpu::Maintain::Wait,
        };
        self.device.poll(maintain);
        self.state
            .lock()
            .expect("wgpu RHI mutex poisoned")
            .last_poll = Some(mode);
    }
}

fn validate_graphics_pipeline_desc(desc: &RhiGraphicsPipelineDesc) -> Result<(), RendererError> {
    validate_rhi_label(desc.label.as_deref())?;
    if desc.vertex_entry.trim().is_empty() {
        return Err(RendererError::PipelineCompile(
            "RHI graphics pipeline vertex entry point must not be empty".to_owned(),
        ));
    }
    if desc.sample_count == 0 || !desc.sample_count.is_power_of_two() {
        return Err(RendererError::PipelineCompile(
            "RHI graphics pipeline sample_count must be a non-zero power of two".to_owned(),
        ));
    }
    match (
        desc.fragment_shader,
        desc.fragment_entry.as_deref(),
        desc.color_format,
        desc.depth_format,
    ) {
        (Some(_), Some(entry), Some(_), _) if !entry.trim().is_empty() => {}
        (Some(_), Some(entry), None, Some(_)) if !entry.trim().is_empty() => {}
        (Some(_), _, _, _) => {
            return Err(RendererError::PipelineCompile(
                "RHI graphics pipeline fragment shader requires a non-empty entry point and a color or depth format"
                    .to_owned(),
            ));
        }
        (None, Some(_), _, _) => {
            return Err(RendererError::PipelineCompile(
                "RHI graphics pipeline fragment entry requires a fragment shader".to_owned(),
            ));
        }
        (None, None, _, _) => {}
    }
    if desc.color_format.is_some() && desc.fragment_shader.is_none() {
        return Err(RendererError::PipelineCompile(
            "RHI graphics pipeline color format requires a fragment shader".to_owned(),
        ));
    }
    if let (Some(depth_format), Some(depth)) = (desc.depth_format, desc.depth) {
        if depth_format != depth.format {
            return Err(RendererError::PipelineCompile(
                "RHI graphics pipeline depth_format must match depth state format".to_owned(),
            ));
        }
    }
    for buffer in &desc.vertex_buffers {
        if buffer.stride == 0 {
            return Err(RendererError::PipelineCompile(
                "RHI graphics pipeline vertex buffer stride must be non-zero".to_owned(),
            ));
        }
        for attribute in &buffer.attributes {
            if attribute.offset >= buffer.stride {
                return Err(RendererError::PipelineCompile(
                    "RHI graphics pipeline vertex attribute offset must be within stride"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_resolve_shader_desc(desc: &RhiResolveShaderDesc) -> Result<(), RendererError> {
    validate_rhi_label(desc.label.as_deref())?;
    if desc.source.trim().is_empty() {
        return Err(RendererError::ShaderCompile(
            "RHI resolve shader source must not be empty".to_owned(),
        ));
    }
    if desc.entry_point.trim().is_empty() {
        return Err(RendererError::ShaderCompile(
            "RHI resolve shader entry point must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_render_pass_desc(desc: &RhiRenderPassDesc) -> Result<(), RendererError> {
    validate_rhi_label(desc.label.as_deref())?;
    if desc.color_target.is_none() && desc.depth_target.is_none() {
        return Err(RendererError::Validation(
            "RHI render pass requires at least one color or depth target".to_owned(),
        ));
    }
    if desc.vertex_count == 0 || desc.instance_count == 0 {
        return Err(RendererError::Validation(
            "RHI render pass draw counts must be non-zero".to_owned(),
        ));
    }
    match (&desc.index_buffer, desc.index_count) {
        (Some(_), Some(index_count)) if index_count > 0 => {}
        (Some(_), _) => {
            return Err(RendererError::Validation(
                "RHI indexed render pass requires a non-zero index_count".to_owned(),
            ));
        }
        (None, Some(_)) => {
            return Err(RendererError::Validation(
                "RHI index_count requires an index buffer binding".to_owned(),
            ));
        }
        (None, None) => {}
    }
    let mut slots = HashSet::new();
    for binding in &desc.vertex_buffers {
        if !slots.insert(binding.slot) {
            return Err(RendererError::Validation(
                "RHI render pass vertex buffer slots must be unique".to_owned(),
            ));
        }
    }
    let mut bind_group_slots = HashSet::new();
    for binding in &desc.bind_groups {
        if !bind_group_slots.insert(binding.index) {
            return Err(RendererError::Validation(
                "RHI render pass bind group slots must be unique".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_bind_group_desc_label_and_entries(
    desc: &RhiBindGroupDesc,
) -> Result<(), RendererError> {
    validate_bind_group_entries(&desc.entries)
}

fn validate_bind_group_entries(entries: &[RhiBindGroupEntry]) -> Result<(), RendererError> {
    if entries.is_empty() {
        return Err(RendererError::Validation(
            "RHI bind groups require at least one entry".to_owned(),
        ));
    }
    let mut bindings = HashSet::new();
    for entry in entries {
        let binding = match entry {
            RhiBindGroupEntry::Texture { binding, .. }
            | RhiBindGroupEntry::Sampler { binding, .. }
            | RhiBindGroupEntry::Buffer { binding, .. } => *binding,
        };
        if !bindings.insert(binding) {
            return Err(RendererError::Validation(
                "RHI bind group entry bindings must be unique".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_indirect_render_pass_desc(
    desc: &RhiIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    validate_rhi_label(desc.label.as_deref())?;
    if desc.color_target.is_none() && desc.depth_target.is_none() {
        return Err(RendererError::Validation(
            "RHI indirect render pass requires at least one color or depth target".to_owned(),
        ));
    }
    if desc.draw_count == 0 {
        return Err(RendererError::Validation(
            "RHI indirect render pass draw_count must be non-zero".to_owned(),
        ));
    }
    if desc.draw_stride < RHI_DRAW_INDIRECT_BYTES {
        return Err(RendererError::Validation(
            "RHI indirect draw stride must fit a draw command".to_owned(),
        ));
    }
    if desc.indirect_offset % 4 != 0 || desc.draw_stride % 4 != 0 {
        return Err(RendererError::Validation(
            "RHI indirect draw offset and stride must be 4-byte aligned".to_owned(),
        ));
    }
    let mut vertex_slots = HashSet::new();
    for binding in &desc.vertex_buffers {
        if !vertex_slots.insert(binding.slot) {
            return Err(RendererError::Validation(
                "RHI indirect render pass vertex buffer slots must be unique".to_owned(),
            ));
        }
    }
    let mut bind_group_slots = HashSet::new();
    for binding in &desc.bind_groups {
        if !bind_group_slots.insert(binding.index) {
            return Err(RendererError::Validation(
                "RHI indirect render pass bind group slots must be unique".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_indexed_indirect_render_pass_desc(
    desc: &RhiIndexedIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    validate_rhi_label(desc.label.as_deref())?;
    if desc.color_target.is_none() && desc.depth_target.is_none() {
        return Err(RendererError::Validation(
            "RHI indexed indirect render pass requires at least one color or depth target"
                .to_owned(),
        ));
    }
    if desc.draw_count == 0 {
        return Err(RendererError::Validation(
            "RHI indexed indirect render pass draw_count must be non-zero".to_owned(),
        ));
    }
    if desc.draw_stride < RHI_INDEXED_DRAW_INDIRECT_BYTES {
        return Err(RendererError::Validation(
            "RHI indexed indirect draw stride must fit an indexed draw command".to_owned(),
        ));
    }
    if desc.indirect_offset % 4 != 0 || desc.draw_stride % 4 != 0 {
        return Err(RendererError::Validation(
            "RHI indexed indirect draw offset and stride must be 4-byte aligned".to_owned(),
        ));
    }
    let mut vertex_slots = HashSet::new();
    for binding in &desc.vertex_buffers {
        if !vertex_slots.insert(binding.slot) {
            return Err(RendererError::Validation(
                "RHI indexed indirect render pass vertex buffer slots must be unique".to_owned(),
            ));
        }
    }
    let mut bind_group_slots = HashSet::new();
    for binding in &desc.bind_groups {
        if !bind_group_slots.insert(binding.index) {
            return Err(RendererError::Validation(
                "RHI indexed indirect render pass bind group slots must be unique".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_indirect_buffer_range(
    buffer_size: u64,
    desc: &RhiIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    let last_draw_offset = desc
        .indirect_offset
        .checked_add(u64::from(desc.draw_count.saturating_sub(1)) * desc.draw_stride)
        .ok_or_else(|| RendererError::Validation("RHI indirect draw range overflows".to_owned()))?;
    let required = last_draw_offset
        .checked_add(RHI_DRAW_INDIRECT_BYTES)
        .ok_or_else(|| RendererError::Validation("RHI indirect draw range overflows".to_owned()))?;
    if required > buffer_size {
        return Err(RendererError::Validation(
            "RHI indirect draw commands exceed buffer bounds".to_owned(),
        ));
    }
    Ok(())
}

fn validate_indexed_indirect_buffer_range(
    buffer_size: u64,
    desc: &RhiIndexedIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    let last_draw_offset = desc
        .indirect_offset
        .checked_add(u64::from(desc.draw_count.saturating_sub(1)) * desc.draw_stride)
        .ok_or_else(|| {
            RendererError::Validation("RHI indexed indirect draw range overflows".to_owned())
        })?;
    let required = last_draw_offset
        .checked_add(RHI_INDEXED_DRAW_INDIRECT_BYTES)
        .ok_or_else(|| {
            RendererError::Validation("RHI indexed indirect draw range overflows".to_owned())
        })?;
    if required > buffer_size {
        return Err(RendererError::Validation(
            "RHI indexed indirect draw commands exceed buffer bounds".to_owned(),
        ));
    }
    Ok(())
}

fn validate_headless_vertex_buffer_bindings(
    state: &HeadlessRhiState,
    pipeline: &HeadlessGraphicsPipelineState,
    bindings: &[RhiVertexBufferBinding],
    vertex_count: u32,
    instance_count: u32,
) -> Result<(), RendererError> {
    validate_vertex_buffer_binding_slots(&pipeline.vertex_buffer_layouts, bindings)?;
    for binding in bindings {
        let Some(buffer) = state.buffers.get(&binding.buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI vertex buffer: {}",
                binding.buffer.0
            )));
        };
        if !buffer.usage.contains(RhiBufferUsage::VERTEX) {
            return Err(RendererError::Validation(
                "RHI vertex buffer binding requires VERTEX usage".to_owned(),
            ));
        }
        if binding.offset > buffer.bytes.len() as u64 {
            return Err(RendererError::Validation(
                "RHI vertex buffer binding offset exceeds buffer size".to_owned(),
            ));
        }
        validate_vertex_buffer_binding_range(
            buffer.bytes.len() as u64,
            binding,
            &pipeline.vertex_buffer_layouts[binding.slot as usize],
            vertex_count,
            instance_count,
        )?;
    }
    Ok(())
}

fn index_format_size(format: RhiIndexFormat) -> u64 {
    match format {
        RhiIndexFormat::Uint16 => 2,
        RhiIndexFormat::Uint32 => 4,
    }
}

fn validate_headless_index_buffer_binding(
    state: &HeadlessRhiState,
    binding: &RhiIndexBufferBinding,
    index_count: u32,
) -> Result<(), RendererError> {
    let Some(buffer) = state.buffers.get(&binding.buffer) else {
        return Err(RendererError::Validation(format!(
            "unknown RHI index buffer: {}",
            binding.buffer.0
        )));
    };
    if !buffer.usage.contains(RhiBufferUsage::INDEX) {
        return Err(RendererError::Validation(
            "RHI index buffer binding requires INDEX usage".to_owned(),
        ));
    }
    if binding.offset > buffer.bytes.len() as u64 {
        return Err(RendererError::Validation(
            "RHI index buffer binding offset exceeds buffer size".to_owned(),
        ));
    }
    if binding.offset % index_format_size(binding.format) != 0 {
        return Err(RendererError::Validation(
            "RHI index buffer binding offset must align to index format size".to_owned(),
        ));
    }
    validate_index_buffer_binding_range(buffer.bytes.len() as u64, binding, index_count)?;
    Ok(())
}

fn validate_headless_bind_group_desc(
    state: &HeadlessRhiState,
    desc: &RhiBindGroupDesc,
) -> Result<(), RendererError> {
    validate_bind_group_desc_label_and_entries(desc)?;
    if !state.graphics_pipelines.contains_key(&desc.pipeline) {
        return Err(RendererError::Validation(format!(
            "unknown RHI graphics pipeline for bind group: {}",
            desc.pipeline.0
        )));
    }
    for entry in &desc.entries {
        match *entry {
            RhiBindGroupEntry::Texture { texture, .. } => {
                let Some(texture_state) = state.textures.get(&texture) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group texture: {}",
                        texture.0
                    )));
                };
                if !texture_state.usage.contains(RhiTextureUsage::SAMPLED) {
                    return Err(RendererError::Validation(
                        "RHI bind group texture requires SAMPLED usage".to_owned(),
                    ));
                }
            }
            RhiBindGroupEntry::Sampler { sampler, .. } => {
                if !state.samplers.contains(&sampler) {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group sampler: {}",
                        sampler.0
                    )));
                }
            }
            RhiBindGroupEntry::Buffer { buffer, .. } => {
                let Some(buffer_state) = state.buffers.get(&buffer) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group buffer: {}",
                        buffer.0
                    )));
                };
                validate_bind_group_buffer_usage(buffer_state.usage)?;
            }
        }
    }
    Ok(())
}

fn validate_bind_group_buffer_usage(usage: RhiBufferUsage) -> Result<(), RendererError> {
    if !usage.contains(RhiBufferUsage::UNIFORM) && !usage.contains(RhiBufferUsage::STORAGE) {
        return Err(RendererError::Validation(
            "RHI bind group buffer requires UNIFORM or STORAGE usage".to_owned(),
        ));
    }
    Ok(())
}

fn validate_compute_bind_group_buffer_usage(usage: RhiBufferUsage) -> Result<(), RendererError> {
    if !usage.contains(RhiBufferUsage::UNIFORM) && !usage.contains(RhiBufferUsage::STORAGE) {
        return Err(RendererError::Validation(
            "RHI compute bind group buffer requires UNIFORM or STORAGE usage".to_owned(),
        ));
    }
    Ok(())
}

fn validate_headless_compute_bind_group_desc(
    state: &HeadlessRhiState,
    desc: &RhiComputeBindGroupDesc,
) -> Result<(), RendererError> {
    validate_bind_group_entries(&desc.entries)?;
    if !state.compute_pipelines.contains(&desc.pipeline) {
        return Err(RendererError::Validation(format!(
            "unknown RHI compute pipeline for bind group: {}",
            desc.pipeline.0
        )));
    }
    for entry in &desc.entries {
        match *entry {
            RhiBindGroupEntry::Texture { texture, .. } => {
                let Some(texture_state) = state.textures.get(&texture) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group texture: {}",
                        texture.0
                    )));
                };
                if !texture_state.usage.contains(RhiTextureUsage::SAMPLED)
                    && !texture_state.usage.contains(RhiTextureUsage::STORAGE)
                {
                    return Err(RendererError::Validation(
                        "RHI compute bind group texture requires SAMPLED or STORAGE usage"
                            .to_owned(),
                    ));
                }
            }
            RhiBindGroupEntry::Sampler { sampler, .. } => {
                if !state.samplers.contains(&sampler) {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group sampler: {}",
                        sampler.0
                    )));
                }
            }
            RhiBindGroupEntry::Buffer { buffer, .. } => {
                let Some(buffer_state) = state.buffers.get(&buffer) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group buffer: {}",
                        buffer.0
                    )));
                };
                validate_compute_bind_group_buffer_usage(buffer_state.usage)?;
            }
        }
    }
    Ok(())
}

fn validate_headless_render_bind_groups(
    state: &HeadlessRhiState,
    pipeline: RhiGraphicsPipeline,
    bindings: &[RhiRenderPassBindGroup],
) -> Result<(), RendererError> {
    for binding in bindings {
        let Some(bind_group) = state.bind_groups.get(&binding.bind_group) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI bind group: {}",
                binding.bind_group.0
            )));
        };
        if bind_group.owner != RhiBindGroupOwner::Graphics(pipeline) {
            return Err(RendererError::Validation(
                "RHI render pass bind group must belong to the active graphics pipeline".to_owned(),
            ));
        }
        if bind_group.group_index != binding.index {
            return Err(RendererError::Validation(
                "RHI render pass bind group index must match its bind group layout index"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_headless_compute_bind_groups(
    state: &HeadlessRhiState,
    pipeline: RhiComputePipeline,
    bindings: &[RhiComputePassBindGroup],
) -> Result<(), RendererError> {
    let mut slots = HashSet::new();
    for binding in bindings {
        if !slots.insert(binding.index) {
            return Err(RendererError::Validation(
                "RHI compute pass bind group slots must be unique".to_owned(),
            ));
        }
        let Some(bind_group) = state.bind_groups.get(&binding.bind_group) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI bind group: {}",
                binding.bind_group.0
            )));
        };
        if bind_group.owner != RhiBindGroupOwner::Compute(pipeline) {
            return Err(RendererError::Validation(
                "RHI compute pass bind group must belong to the active compute pipeline".to_owned(),
            ));
        }
        if bind_group.group_index != binding.index {
            return Err(RendererError::Validation(
                "RHI compute pass bind group index must match its bind group layout index"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_headless_render_targets_for_pipeline(
    state: &HeadlessRhiState,
    pipeline: &HeadlessGraphicsPipelineState,
    desc: &RhiRenderPassDesc,
) -> Result<(), RendererError> {
    let color_format = desc
        .color_target
        .map(|target| headless_texture_format(state, target, "color target"))
        .transpose()?;
    let depth_format = desc
        .depth_target
        .map(|target| headless_texture_format(state, target, "depth target"))
        .transpose()?;
    let color_samples = desc
        .color_target
        .map(|target| headless_texture_samples(state, target, "color target"))
        .transpose()?;
    let depth_samples = desc
        .depth_target
        .map(|target| headless_texture_samples(state, target, "depth target"))
        .transpose()?;
    validate_pipeline_target_formats(
        pipeline.color_format,
        pipeline.depth_format,
        color_format,
        depth_format,
    )?;
    validate_pipeline_target_samples(pipeline.sample_count, color_samples, depth_samples)
}

fn validate_headless_indirect_render_targets_for_pipeline(
    state: &HeadlessRhiState,
    pipeline: &HeadlessGraphicsPipelineState,
    desc: &RhiIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    let color_format = desc
        .color_target
        .map(|target| headless_texture_format(state, target, "color target"))
        .transpose()?;
    let depth_format = desc
        .depth_target
        .map(|target| headless_texture_format(state, target, "depth target"))
        .transpose()?;
    let color_samples = desc
        .color_target
        .map(|target| headless_texture_samples(state, target, "color target"))
        .transpose()?;
    let depth_samples = desc
        .depth_target
        .map(|target| headless_texture_samples(state, target, "depth target"))
        .transpose()?;
    validate_pipeline_target_formats(
        pipeline.color_format,
        pipeline.depth_format,
        color_format,
        depth_format,
    )?;
    validate_pipeline_target_samples(pipeline.sample_count, color_samples, depth_samples)
}

fn validate_headless_indexed_indirect_render_targets_for_pipeline(
    state: &HeadlessRhiState,
    pipeline: &HeadlessGraphicsPipelineState,
    desc: &RhiIndexedIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    let color_format = desc
        .color_target
        .map(|target| headless_texture_format(state, target, "color target"))
        .transpose()?;
    let depth_format = desc
        .depth_target
        .map(|target| headless_texture_format(state, target, "depth target"))
        .transpose()?;
    let color_samples = desc
        .color_target
        .map(|target| headless_texture_samples(state, target, "color target"))
        .transpose()?;
    let depth_samples = desc
        .depth_target
        .map(|target| headless_texture_samples(state, target, "depth target"))
        .transpose()?;
    validate_pipeline_target_formats(
        pipeline.color_format,
        pipeline.depth_format,
        color_format,
        depth_format,
    )?;
    validate_pipeline_target_samples(pipeline.sample_count, color_samples, depth_samples)
}

fn headless_texture_format(
    state: &HeadlessRhiState,
    texture: RhiTexture,
    role: &str,
) -> Result<TextureFormat, RendererError> {
    state
        .textures
        .get(&texture)
        .map(|texture_state| texture_state.format)
        .ok_or_else(|| {
            RendererError::Validation(format!("unknown RHI {role} texture: {}", texture.0))
        })
}

fn headless_texture_samples(
    state: &HeadlessRhiState,
    texture: RhiTexture,
    role: &str,
) -> Result<u32, RendererError> {
    state
        .textures
        .get(&texture)
        .map(|texture_state| texture_state.samples)
        .ok_or_else(|| {
            RendererError::Validation(format!("unknown RHI {role} texture: {}", texture.0))
        })
}

fn validate_headless_rgba8_resolve_textures(
    source: RhiTexture,
    source_state: &HeadlessTextureState,
    target: RhiTexture,
    target_state: &HeadlessTextureState,
) -> Result<(), RendererError> {
    validate_rgba8_resolve_shape_and_usage(
        source,
        source_state.width,
        source_state.height,
        source_state.samples,
        source_state.format,
        source_state.usage,
        target,
        target_state.width,
        target_state.height,
        target_state.samples,
        target_state.format,
        target_state.usage,
    )
}

fn validate_headless_resource_barrier(
    state: &HeadlessRhiState,
    desc: &RhiResourceBarrierDesc,
) -> Result<(), RendererError> {
    match desc.resource {
        RhiResource::Texture(texture) => {
            let Some(texture_state) = state.textures.get(&texture) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI barrier texture: {}",
                    texture.0
                )));
            };
            if let Some(before) = desc.before {
                validate_texture_barrier_access(texture_state.usage, before)?;
            }
            validate_texture_barrier_access(texture_state.usage, desc.after)?;
        }
        RhiResource::Buffer(buffer) => {
            let Some(buffer_state) = state.buffers.get(&buffer) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI barrier buffer: {}",
                    buffer.0
                )));
            };
            if let Some(before) = desc.before {
                validate_buffer_barrier_access(buffer_state.usage, before)?;
            }
            validate_buffer_barrier_access(buffer_state.usage, desc.after)?;
        }
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_vertex_buffer_bindings(
    state: &WgpuRhiState,
    pipeline: &WgpuGraphicsPipelineState,
    bindings: &[RhiVertexBufferBinding],
    vertex_count: u32,
    instance_count: u32,
) -> Result<Vec<(u32, Arc<wgpu::Buffer>, u64)>, RendererError> {
    validate_vertex_buffer_binding_slots(&pipeline.vertex_buffer_layouts, bindings)?;
    bindings
        .iter()
        .map(|binding| {
            let Some(buffer) = state.buffers.get(&binding.buffer) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI vertex buffer: {}",
                    binding.buffer.0
                )));
            };
            if !buffer.usage.contains(RhiBufferUsage::VERTEX) {
                return Err(RendererError::Validation(
                    "RHI vertex buffer binding requires VERTEX usage".to_owned(),
                ));
            }
            if binding.offset > buffer.buffer.size() {
                return Err(RendererError::Validation(
                    "RHI vertex buffer binding offset exceeds buffer size".to_owned(),
                ));
            }
            validate_vertex_buffer_binding_range(
                buffer.buffer.size(),
                binding,
                &pipeline.vertex_buffer_layouts[binding.slot as usize],
                vertex_count,
                instance_count,
            )?;
            Ok((binding.slot, Arc::clone(&buffer.buffer), binding.offset))
        })
        .collect()
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_index_buffer_binding(
    state: &WgpuRhiState,
    binding: &RhiIndexBufferBinding,
    index_count: u32,
) -> Result<(Arc<wgpu::Buffer>, u64, wgpu::IndexFormat), RendererError> {
    let Some(buffer) = state.buffers.get(&binding.buffer) else {
        return Err(RendererError::Validation(format!(
            "unknown RHI index buffer: {}",
            binding.buffer.0
        )));
    };
    if !buffer.usage.contains(RhiBufferUsage::INDEX) {
        return Err(RendererError::Validation(
            "RHI index buffer binding requires INDEX usage".to_owned(),
        ));
    }
    if binding.offset > buffer.buffer.size() {
        return Err(RendererError::Validation(
            "RHI index buffer binding offset exceeds buffer size".to_owned(),
        ));
    }
    if binding.offset % index_format_size(binding.format) != 0 {
        return Err(RendererError::Validation(
            "RHI index buffer binding offset must align to index format size".to_owned(),
        ));
    }
    validate_index_buffer_binding_range(buffer.buffer.size(), binding, index_count)?;
    Ok((
        Arc::clone(&buffer.buffer),
        binding.offset,
        map_index_format(binding.format),
    ))
}

#[cfg(feature = "backend-wgpu")]
enum WgpuBindGroupOwnedResource {
    TextureView(wgpu::TextureView),
    Sampler(Arc<wgpu::Sampler>),
    Buffer(Arc<wgpu::Buffer>),
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_compute_bind_group_entries(
    state: &WgpuRhiState,
    desc: &RhiComputeBindGroupDesc,
) -> Result<(), RendererError> {
    for entry in &desc.entries {
        match *entry {
            RhiBindGroupEntry::Texture { texture, .. } => {
                let Some(texture_state) = state.textures.get(&texture) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group texture: {}",
                        texture.0
                    )));
                };
                if !texture_state.usage.contains(RhiTextureUsage::SAMPLED)
                    && !texture_state.usage.contains(RhiTextureUsage::STORAGE)
                {
                    return Err(RendererError::Validation(
                        "RHI compute bind group texture requires SAMPLED or STORAGE usage"
                            .to_owned(),
                    ));
                }
            }
            RhiBindGroupEntry::Sampler { sampler, .. } => {
                if !state.samplers.contains_key(&sampler) {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group sampler: {}",
                        sampler.0
                    )));
                }
            }
            RhiBindGroupEntry::Buffer { buffer, .. } => {
                let Some(buffer_state) = state.buffers.get(&buffer) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group buffer: {}",
                        buffer.0
                    )));
                };
                validate_compute_bind_group_buffer_usage(buffer_state.usage)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn wgpu_bind_group_owned_resources(
    state: &WgpuRhiState,
    entries: &[RhiBindGroupEntry],
) -> Result<Vec<WgpuBindGroupOwnedResource>, RendererError> {
    let mut owned_resources = Vec::with_capacity(entries.len());
    for entry in entries {
        match *entry {
            RhiBindGroupEntry::Texture { texture, .. } => {
                let Some(texture_state) = state.textures.get(&texture) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group texture: {}",
                        texture.0
                    )));
                };
                owned_resources.push(WgpuBindGroupOwnedResource::TextureView(
                    texture_state
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                ));
            }
            RhiBindGroupEntry::Sampler { sampler, .. } => {
                let Some(sampler) = state.samplers.get(&sampler) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group sampler: {}",
                        sampler.0
                    )));
                };
                owned_resources.push(WgpuBindGroupOwnedResource::Sampler(Arc::clone(sampler)));
            }
            RhiBindGroupEntry::Buffer { buffer, .. } => {
                let Some(buffer_state) = state.buffers.get(&buffer) else {
                    return Err(RendererError::Validation(format!(
                        "unknown RHI bind group buffer: {}",
                        buffer.0
                    )));
                };
                owned_resources.push(WgpuBindGroupOwnedResource::Buffer(Arc::clone(
                    &buffer_state.buffer,
                )));
            }
        }
    }
    Ok(owned_resources)
}

#[cfg(feature = "backend-wgpu")]
fn wgpu_bind_group_entries<'a>(
    entries: &'a [RhiBindGroupEntry],
    owned_resources: &'a [WgpuBindGroupOwnedResource],
) -> Vec<wgpu::BindGroupEntry<'a>> {
    entries
        .iter()
        .zip(owned_resources)
        .map(|(entry, resource)| {
            let binding = match entry {
                RhiBindGroupEntry::Texture { binding, .. }
                | RhiBindGroupEntry::Sampler { binding, .. }
                | RhiBindGroupEntry::Buffer { binding, .. } => *binding,
            };
            let resource = match resource {
                WgpuBindGroupOwnedResource::TextureView(view) => {
                    wgpu::BindingResource::TextureView(view)
                }
                WgpuBindGroupOwnedResource::Sampler(sampler) => {
                    wgpu::BindingResource::Sampler(sampler)
                }
                WgpuBindGroupOwnedResource::Buffer(buffer) => buffer.as_entire_binding(),
            };
            wgpu::BindGroupEntry { binding, resource }
        })
        .collect()
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_render_bind_groups<'a>(
    state: &'a WgpuRhiState,
    pipeline: RhiGraphicsPipeline,
    bindings: &[RhiRenderPassBindGroup],
) -> Result<Vec<(u32, &'a wgpu::BindGroup)>, RendererError> {
    bindings
        .iter()
        .map(|binding| {
            let Some(bind_group) = state.bind_groups.get(&binding.bind_group) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI bind group: {}",
                    binding.bind_group.0
                )));
            };
            if bind_group.owner != RhiBindGroupOwner::Graphics(pipeline) {
                return Err(RendererError::Validation(
                    "RHI render pass bind group must belong to the active graphics pipeline"
                        .to_owned(),
                ));
            }
            if bind_group.group_index != binding.index {
                return Err(RendererError::Validation(
                    "RHI render pass bind group index must match its bind group layout index"
                        .to_owned(),
                ));
            }
            Ok((binding.index, &bind_group.bind_group))
        })
        .collect()
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_compute_bind_groups<'a>(
    state: &'a WgpuRhiState,
    pipeline: RhiComputePipeline,
    bindings: &[RhiComputePassBindGroup],
) -> Result<Vec<(u32, &'a wgpu::BindGroup)>, RendererError> {
    let mut slots = HashSet::new();
    bindings
        .iter()
        .map(|binding| {
            if !slots.insert(binding.index) {
                return Err(RendererError::Validation(
                    "RHI compute pass bind group slots must be unique".to_owned(),
                ));
            }
            let Some(bind_group) = state.bind_groups.get(&binding.bind_group) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI bind group: {}",
                    binding.bind_group.0
                )));
            };
            if bind_group.owner != RhiBindGroupOwner::Compute(pipeline) {
                return Err(RendererError::Validation(
                    "RHI compute pass bind group must belong to the active compute pipeline"
                        .to_owned(),
                ));
            }
            if bind_group.group_index != binding.index {
                return Err(RendererError::Validation(
                    "RHI compute pass bind group index must match its bind group layout index"
                        .to_owned(),
                ));
            }
            Ok((binding.index, &bind_group.bind_group))
        })
        .collect()
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_render_targets_for_pipeline(
    state: &WgpuRhiState,
    pipeline: &WgpuGraphicsPipelineState,
    desc: &RhiRenderPassDesc,
) -> Result<(), RendererError> {
    let color_format = desc
        .color_target
        .map(|target| wgpu_texture_format(state, target, "color target"))
        .transpose()?;
    let depth_format = desc
        .depth_target
        .map(|target| wgpu_texture_format(state, target, "depth target"))
        .transpose()?;
    let color_samples = desc
        .color_target
        .map(|target| wgpu_texture_samples(state, target, "color target"))
        .transpose()?;
    let depth_samples = desc
        .depth_target
        .map(|target| wgpu_texture_samples(state, target, "depth target"))
        .transpose()?;
    validate_pipeline_target_formats(
        pipeline.color_format,
        pipeline.depth_format,
        color_format,
        depth_format,
    )?;
    validate_pipeline_target_samples(pipeline.sample_count, color_samples, depth_samples)
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_indirect_render_targets_for_pipeline(
    state: &WgpuRhiState,
    pipeline: &WgpuGraphicsPipelineState,
    desc: &RhiIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    let color_format = desc
        .color_target
        .map(|target| wgpu_texture_format(state, target, "color target"))
        .transpose()?;
    let depth_format = desc
        .depth_target
        .map(|target| wgpu_texture_format(state, target, "depth target"))
        .transpose()?;
    let color_samples = desc
        .color_target
        .map(|target| wgpu_texture_samples(state, target, "color target"))
        .transpose()?;
    let depth_samples = desc
        .depth_target
        .map(|target| wgpu_texture_samples(state, target, "depth target"))
        .transpose()?;
    validate_pipeline_target_formats(
        pipeline.color_format,
        pipeline.depth_format,
        color_format,
        depth_format,
    )?;
    validate_pipeline_target_samples(pipeline.sample_count, color_samples, depth_samples)
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_indexed_indirect_render_targets_for_pipeline(
    state: &WgpuRhiState,
    pipeline: &WgpuGraphicsPipelineState,
    desc: &RhiIndexedIndirectRenderPassDesc,
) -> Result<(), RendererError> {
    let color_format = desc
        .color_target
        .map(|target| wgpu_texture_format(state, target, "color target"))
        .transpose()?;
    let depth_format = desc
        .depth_target
        .map(|target| wgpu_texture_format(state, target, "depth target"))
        .transpose()?;
    let color_samples = desc
        .color_target
        .map(|target| wgpu_texture_samples(state, target, "color target"))
        .transpose()?;
    let depth_samples = desc
        .depth_target
        .map(|target| wgpu_texture_samples(state, target, "depth target"))
        .transpose()?;
    validate_pipeline_target_formats(
        pipeline.color_format,
        pipeline.depth_format,
        color_format,
        depth_format,
    )?;
    validate_pipeline_target_samples(pipeline.sample_count, color_samples, depth_samples)
}

#[cfg(feature = "backend-wgpu")]
fn wgpu_texture_format(
    state: &WgpuRhiState,
    texture: RhiTexture,
    role: &str,
) -> Result<TextureFormat, RendererError> {
    state
        .textures
        .get(&texture)
        .map(|texture_state| texture_state.format)
        .ok_or_else(|| {
            RendererError::Validation(format!("unknown RHI {role} texture: {}", texture.0))
        })
}

#[cfg(feature = "backend-wgpu")]
fn wgpu_texture_samples(
    state: &WgpuRhiState,
    texture: RhiTexture,
    role: &str,
) -> Result<u32, RendererError> {
    state
        .textures
        .get(&texture)
        .map(|texture_state| texture_state.samples)
        .ok_or_else(|| {
            RendererError::Validation(format!("unknown RHI {role} texture: {}", texture.0))
        })
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_rgba8_resolve_textures(
    source: RhiTexture,
    source_state: &WgpuTextureState,
    target: RhiTexture,
    target_state: &WgpuTextureState,
) -> Result<(), RendererError> {
    validate_rgba8_resolve_shape_and_usage(
        source,
        source_state.width,
        source_state.height,
        source_state.samples,
        source_state.format,
        source_state.usage,
        target,
        target_state.width,
        target_state.height,
        target_state.samples,
        target_state.format,
        target_state.usage,
    )
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_rgba8_custom_resolve_textures(
    source: RhiTexture,
    source_state: &WgpuTextureState,
    target: RhiTexture,
    target_state: &WgpuTextureState,
) -> Result<(), RendererError> {
    if source_state.samples <= 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom resolve source texture {} must be multisampled",
            source.0
        )));
    }
    if target_state.samples != 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom resolve target texture {} must be single-sampled",
            target.0
        )));
    }
    if source_state.width != target_state.width || source_state.height != target_state.height {
        return Err(RendererError::Validation(
            "RHI custom resolve source and target dimensions must match".to_owned(),
        ));
    }
    if source_state.format != TextureFormat::Rgba8Unorm
        || target_state.format != TextureFormat::Rgba8Unorm
    {
        return Err(RendererError::Validation(
            "RHI custom RGBA8 resolve requires Rgba8Unorm source and target".to_owned(),
        ));
    }
    if !source_state.usage.contains(RhiTextureUsage::SAMPLED) {
        return Err(RendererError::Validation(
            "RHI custom resolve source requires SAMPLED usage".to_owned(),
        ));
    }
    if !target_state.usage.contains(RhiTextureUsage::STORAGE) {
        return Err(RendererError::Validation(
            "RHI custom resolve target requires STORAGE usage".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_rgba16f_custom_resolve_textures(
    source: RhiTexture,
    source_state: &WgpuTextureState,
    target: RhiTexture,
    target_state: &WgpuTextureState,
) -> Result<(), RendererError> {
    if source_state.samples <= 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom resolve source texture {} must be multisampled",
            source.0
        )));
    }
    if target_state.samples != 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom resolve target texture {} must be single-sampled",
            target.0
        )));
    }
    if source_state.width != target_state.width || source_state.height != target_state.height {
        return Err(RendererError::Validation(
            "RHI custom resolve source and target dimensions must match".to_owned(),
        ));
    }
    if source_state.format != TextureFormat::Rgba16Float
        || target_state.format != TextureFormat::Rgba16Float
    {
        return Err(RendererError::Validation(
            "RHI custom RGBA16F resolve requires Rgba16Float source and target".to_owned(),
        ));
    }
    if !source_state.usage.contains(RhiTextureUsage::SAMPLED) {
        return Err(RendererError::Validation(
            "RHI custom resolve source requires SAMPLED usage".to_owned(),
        ));
    }
    if !target_state.usage.contains(RhiTextureUsage::STORAGE) {
        return Err(RendererError::Validation(
            "RHI custom resolve target requires STORAGE usage".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_rgba32f_custom_resolve_textures(
    source: RhiTexture,
    source_state: &WgpuTextureState,
    target: RhiTexture,
    target_state: &WgpuTextureState,
) -> Result<(), RendererError> {
    if source_state.samples <= 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom resolve source texture {} must be multisampled",
            source.0
        )));
    }
    if target_state.samples != 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom resolve target texture {} must be single-sampled",
            target.0
        )));
    }
    if source_state.width != target_state.width || source_state.height != target_state.height {
        return Err(RendererError::Validation(
            "RHI custom resolve source and target dimensions must match".to_owned(),
        ));
    }
    if source_state.format != TextureFormat::Rgba32Float
        || target_state.format != TextureFormat::Rgba32Float
    {
        return Err(RendererError::Validation(
            "RHI custom RGBA32F resolve requires Rgba32Float source and target".to_owned(),
        ));
    }
    if !source_state.usage.contains(RhiTextureUsage::SAMPLED) {
        return Err(RendererError::Validation(
            "RHI custom resolve source requires SAMPLED usage".to_owned(),
        ));
    }
    if !target_state.usage.contains(RhiTextureUsage::STORAGE) {
        return Err(RendererError::Validation(
            "RHI custom resolve target requires STORAGE usage".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_8bit_color_fragment_resolve_textures(
    source: RhiTexture,
    source_state: &WgpuTextureState,
    target: RhiTexture,
    target_state: &WgpuTextureState,
) -> Result<(), RendererError> {
    if source_state.samples <= 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom 8-bit color resolve source texture {} must be multisampled",
            source.0
        )));
    }
    if target_state.samples != 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom 8-bit color resolve target texture {} must be single-sampled",
            target.0
        )));
    }
    if source_state.width != target_state.width || source_state.height != target_state.height {
        return Err(RendererError::Validation(
            "RHI custom 8-bit color resolve source and target dimensions must match".to_owned(),
        ));
    }
    if !is_rhi_8bit_color_texture_format(source_state.format)
        || !is_rhi_8bit_color_texture_format(target_state.format)
    {
        return Err(RendererError::Validation(
            "RHI custom 8-bit color resolve requires 8-bit color source and target".to_owned(),
        ));
    }
    if source_state.format != target_state.format {
        return Err(RendererError::Validation(
            "RHI custom 8-bit color resolve requires matching source and target formats".to_owned(),
        ));
    }
    if !source_state.usage.contains(RhiTextureUsage::SAMPLED) {
        return Err(RendererError::Validation(
            "RHI custom 8-bit color resolve source requires SAMPLED usage".to_owned(),
        ));
    }
    if !target_state
        .usage
        .contains(RhiTextureUsage::RENDER_ATTACHMENT)
    {
        return Err(RendererError::Validation(
            "RHI custom 8-bit color resolve target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_depth32f_custom_resolve_textures(
    source: RhiTexture,
    source_state: &WgpuTextureState,
    target: RhiTexture,
    target_state: &WgpuTextureState,
) -> Result<(), RendererError> {
    if source_state.samples <= 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom depth resolve source texture {} must be multisampled",
            source.0
        )));
    }
    if target_state.samples != 1 {
        return Err(RendererError::Validation(format!(
            "RHI custom depth resolve target texture {} must be single-sampled",
            target.0
        )));
    }
    if source_state.width != target_state.width || source_state.height != target_state.height {
        return Err(RendererError::Validation(
            "RHI custom depth resolve source and target dimensions must match".to_owned(),
        ));
    }
    if source_state.format != TextureFormat::Depth32Float
        || target_state.format != TextureFormat::Depth32Float
    {
        return Err(RendererError::Validation(
            "RHI custom depth resolve requires Depth32Float source and target".to_owned(),
        ));
    }
    if !source_state.usage.contains(RhiTextureUsage::SAMPLED) {
        return Err(RendererError::Validation(
            "RHI custom depth resolve source requires SAMPLED usage".to_owned(),
        ));
    }
    if !target_state
        .usage
        .contains(RhiTextureUsage::RENDER_ATTACHMENT)
    {
        return Err(RendererError::Validation(
            "RHI custom depth resolve target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_resource_barrier(
    state: &WgpuRhiState,
    desc: &RhiResourceBarrierDesc,
) -> Result<(), RendererError> {
    match desc.resource {
        RhiResource::Texture(texture) => {
            let Some(texture_state) = state.textures.get(&texture) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI barrier texture: {}",
                    texture.0
                )));
            };
            if let Some(before) = desc.before {
                validate_texture_barrier_access(texture_state.usage, before)?;
            }
            validate_texture_barrier_access(texture_state.usage, desc.after)?;
        }
        RhiResource::Buffer(buffer) => {
            let Some(buffer_state) = state.buffers.get(&buffer) else {
                return Err(RendererError::Validation(format!(
                    "unknown RHI barrier buffer: {}",
                    buffer.0
                )));
            };
            if let Some(before) = desc.before {
                validate_buffer_barrier_access(buffer_state.usage, before)?;
            }
            validate_buffer_barrier_access(buffer_state.usage, desc.after)?;
        }
    }
    Ok(())
}

fn validate_texture_barrier_access(
    usage: RhiTextureUsage,
    access: RhiAccessState,
) -> Result<(), RendererError> {
    let valid = match access {
        RhiAccessState::TextureSampled => usage.contains(RhiTextureUsage::SAMPLED),
        RhiAccessState::TextureStorageRead | RhiAccessState::TextureStorageWrite => {
            usage.contains(RhiTextureUsage::STORAGE)
        }
        RhiAccessState::RenderAttachment => usage.contains(RhiTextureUsage::RENDER_ATTACHMENT),
        RhiAccessState::CopySrc => usage.contains(RhiTextureUsage::COPY_SRC),
        RhiAccessState::CopyDst => usage.contains(RhiTextureUsage::COPY_DST),
        RhiAccessState::BufferUniform
        | RhiAccessState::BufferStorageRead
        | RhiAccessState::BufferStorageWrite
        | RhiAccessState::BufferVertex
        | RhiAccessState::BufferIndex
        | RhiAccessState::BufferIndirect => false,
    };
    if !valid {
        return Err(RendererError::Validation(
            "RHI texture barrier access is incompatible with texture usage".to_owned(),
        ));
    }
    Ok(())
}

fn validate_texture_usage_format(
    usage: RhiTextureUsage,
    format: TextureFormat,
) -> Result<(), RendererError> {
    if usage.contains(RhiTextureUsage::STORAGE) && !is_storage_texture_format(format) {
        return Err(RendererError::Validation(
            "RHI storage textures require a non-sRGB color format".to_owned(),
        ));
    }
    Ok(())
}

fn is_storage_texture_format(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba16Float | TextureFormat::Rgba32Float
    )
}

fn is_rhi_8bit_color_texture_format(format: TextureFormat) -> bool {
    matches!(
        format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
    )
}

fn validate_buffer_barrier_access(
    usage: RhiBufferUsage,
    access: RhiAccessState,
) -> Result<(), RendererError> {
    let valid = match access {
        RhiAccessState::BufferUniform => usage.contains(RhiBufferUsage::UNIFORM),
        RhiAccessState::BufferStorageRead | RhiAccessState::BufferStorageWrite => {
            usage.contains(RhiBufferUsage::STORAGE)
        }
        RhiAccessState::BufferVertex => usage.contains(RhiBufferUsage::VERTEX),
        RhiAccessState::BufferIndex => usage.contains(RhiBufferUsage::INDEX),
        RhiAccessState::BufferIndirect => usage.contains(RhiBufferUsage::INDIRECT),
        RhiAccessState::CopySrc => usage.contains(RhiBufferUsage::COPY_SRC),
        RhiAccessState::CopyDst => usage.contains(RhiBufferUsage::COPY_DST),
        RhiAccessState::TextureSampled
        | RhiAccessState::TextureStorageRead
        | RhiAccessState::TextureStorageWrite
        | RhiAccessState::RenderAttachment => false,
    };
    if !valid {
        return Err(RendererError::Validation(
            "RHI buffer barrier access is incompatible with buffer usage".to_owned(),
        ));
    }
    Ok(())
}

fn validate_vertex_buffer_binding_slots(
    vertex_buffer_layouts: &[RhiVertexBufferLayout],
    bindings: &[RhiVertexBufferBinding],
) -> Result<(), RendererError> {
    for binding in bindings {
        if binding.slot as usize >= vertex_buffer_layouts.len() {
            return Err(RendererError::Validation(
                "RHI vertex buffer binding slot must be declared by the graphics pipeline"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_vertex_buffer_binding_range(
    buffer_size: u64,
    binding: &RhiVertexBufferBinding,
    layout: &RhiVertexBufferLayout,
    vertex_count: u32,
    instance_count: u32,
) -> Result<(), RendererError> {
    let element_count = match layout.step_mode {
        VertexStepMode::Vertex => u64::from(vertex_count),
        VertexStepMode::Instance => u64::from(instance_count),
    };
    let required = layout
        .stride
        .checked_mul(element_count)
        .and_then(|size| binding.offset.checked_add(size))
        .ok_or_else(|| {
            RendererError::Validation("RHI vertex buffer draw range overflows".to_owned())
        })?;
    if required > buffer_size {
        return Err(RendererError::Validation(
            "RHI vertex buffer binding draw range exceeds buffer bounds".to_owned(),
        ));
    }
    Ok(())
}

fn validate_index_buffer_binding_range(
    buffer_size: u64,
    binding: &RhiIndexBufferBinding,
    index_count: u32,
) -> Result<(), RendererError> {
    let required = index_format_size(binding.format)
        .checked_mul(u64::from(index_count))
        .and_then(|size| binding.offset.checked_add(size))
        .ok_or_else(|| {
            RendererError::Validation("RHI index buffer draw range overflows".to_owned())
        })?;
    if required > buffer_size {
        return Err(RendererError::Validation(
            "RHI index buffer binding draw range exceeds buffer bounds".to_owned(),
        ));
    }
    Ok(())
}

fn validate_pipeline_target_formats(
    pipeline_color_format: Option<TextureFormat>,
    pipeline_depth_format: Option<DepthFormat>,
    color_target_format: Option<TextureFormat>,
    depth_target_format: Option<TextureFormat>,
) -> Result<(), RendererError> {
    match (pipeline_color_format, color_target_format) {
        (Some(expected), Some(actual)) if expected == actual => {}
        (Some(_), Some(_)) => {
            return Err(RendererError::Validation(
                "RHI render pass color target format must match the graphics pipeline".to_owned(),
            ));
        }
        (Some(_), None) => {
            return Err(RendererError::Validation(
                "RHI render pass requires a color target for this graphics pipeline".to_owned(),
            ));
        }
        (None, Some(_)) => {
            return Err(RendererError::Validation(
                "RHI render pass color target requires a pipeline color format".to_owned(),
            ));
        }
        (None, None) => {}
    }

    match (
        pipeline_depth_format.map(depth_pipeline_texture_format),
        depth_target_format,
    ) {
        (Some(Ok(expected)), Some(actual)) if expected == actual => {}
        (Some(Ok(_)), Some(_)) => {
            return Err(RendererError::Validation(
                "RHI render pass depth target format must match the graphics pipeline".to_owned(),
            ));
        }
        (Some(Ok(_)), None) => {
            return Err(RendererError::Validation(
                "RHI render pass requires a depth target for this graphics pipeline".to_owned(),
            ));
        }
        (Some(Err(err)), _) => return Err(err),
        (None, Some(_)) => {
            return Err(RendererError::Validation(
                "RHI render pass depth target requires a pipeline depth format".to_owned(),
            ));
        }
        (None, None) => {}
    }
    Ok(())
}

fn validate_pipeline_target_samples(
    pipeline_sample_count: u32,
    color_target_samples: Option<u32>,
    depth_target_samples: Option<u32>,
) -> Result<(), RendererError> {
    if color_target_samples.is_some_and(|samples| samples != pipeline_sample_count) {
        return Err(RendererError::Validation(
            "RHI render pass color target sample count must match the graphics pipeline".to_owned(),
        ));
    }
    if depth_target_samples.is_some_and(|samples| samples != pipeline_sample_count) {
        return Err(RendererError::Validation(
            "RHI render pass depth target sample count must match the graphics pipeline".to_owned(),
        ));
    }
    Ok(())
}

fn validate_rgba8_resolve_shape_and_usage(
    source: RhiTexture,
    source_width: u32,
    source_height: u32,
    source_samples: u32,
    source_format: TextureFormat,
    source_usage: RhiTextureUsage,
    target: RhiTexture,
    target_width: u32,
    target_height: u32,
    target_samples: u32,
    target_format: TextureFormat,
    target_usage: RhiTextureUsage,
) -> Result<(), RendererError> {
    if source_samples <= 1 {
        return Err(RendererError::Validation(format!(
            "RHI resolve source texture {} must be multisampled",
            source.0
        )));
    }
    if target_samples != 1 {
        return Err(RendererError::Validation(format!(
            "RHI resolve target texture {} must be single-sampled",
            target.0
        )));
    }
    if source_width != target_width || source_height != target_height {
        return Err(RendererError::Validation(
            "RHI resolve source and target dimensions must match".to_owned(),
        ));
    }
    if source_format != TextureFormat::Rgba8Unorm || target_format != TextureFormat::Rgba8Unorm {
        return Err(RendererError::Validation(
            "RHI explicit RGBA8 resolve requires Rgba8Unorm source and target".to_owned(),
        ));
    }
    if !source_usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        return Err(RendererError::Validation(
            "RHI resolve source requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    if !target_usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        return Err(RendererError::Validation(
            "RHI resolve target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    Ok(())
}

fn depth_pipeline_texture_format(format: DepthFormat) -> Result<TextureFormat, RendererError> {
    match format {
        DepthFormat::D32Float => Ok(TextureFormat::Depth32Float),
        DepthFormat::D16Unorm | DepthFormat::D24Plus | DepthFormat::D24PlusStencil8 => {
            Err(RendererError::Validation(
                "RHI depth pipeline format is not supported by the current texture format set"
                    .to_owned(),
            ))
        }
    }
}

fn validate_debug_group_label(label: &str) -> Result<(), RendererError> {
    if label.trim().is_empty() {
        return Err(RendererError::Validation(
            "RHI debug group label must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_encoder_finish_state(
    open_debug_groups: usize,
    active_pipeline_statistics: usize,
    active_occlusion_queries: usize,
) -> Result<(), RendererError> {
    if open_debug_groups != 0 {
        return Err(RendererError::Validation(
            "RHI command encoder cannot finish with open debug groups".to_owned(),
        ));
    }
    if active_pipeline_statistics != 0 {
        return Err(RendererError::Validation(
            "RHI command encoder cannot finish with active pipeline statistics queries".to_owned(),
        ));
    }
    if active_occlusion_queries != 0 {
        return Err(RendererError::Validation(
            "RHI command encoder cannot finish with active occlusion queries".to_owned(),
        ));
    }
    Ok(())
}

fn validate_rhi_label(label: Option<&str>) -> Result<(), RendererError> {
    if label.is_some_and(|value| value.trim().is_empty()) {
        return Err(RendererError::Validation(
            "RHI labels must not be empty".to_owned(),
        ));
    }
    Ok(())
}

const RGBA8_BYTES_PER_PIXEL: u32 = 4;
#[cfg(feature = "backend-wgpu")]
const RGBA16F_BYTES_PER_PIXEL: u32 = 8;
#[cfg(feature = "backend-wgpu")]
const RGBA32F_BYTES_PER_PIXEL: u32 = 16;
#[cfg(feature = "backend-wgpu")]
const DEPTH32F_BYTES_PER_PIXEL: u32 = 4;
const RHI_DRAW_INDIRECT_BYTES: u64 = 16;
const RHI_INDEXED_DRAW_INDIRECT_BYTES: u64 = 20;

fn validate_texture_region(
    texture_width: u32,
    texture_height: u32,
    region: RhiTextureRegion,
) -> Result<(), RendererError> {
    if region.width == 0 || region.height == 0 {
        return Err(RendererError::Validation(
            "RHI texture regions must have non-zero dimensions".to_owned(),
        ));
    }
    let x_end = region.x.checked_add(region.width).ok_or_else(|| {
        RendererError::Validation("RHI texture region x range overflows u32".to_owned())
    })?;
    let y_end = region.y.checked_add(region.height).ok_or_else(|| {
        RendererError::Validation("RHI texture region y range overflows u32".to_owned())
    })?;
    if x_end > texture_width || y_end > texture_height {
        return Err(RendererError::Validation(
            "RHI texture region exceeds texture bounds".to_owned(),
        ));
    }
    Ok(())
}

fn rgba8_region_len(region: RhiTextureRegion) -> Result<usize, RendererError> {
    let pixel_count = region
        .width
        .checked_mul(region.height)
        .and_then(|count| count.checked_mul(RGBA8_BYTES_PER_PIXEL))
        .ok_or_else(|| RendererError::Validation("RHI RGBA8 region size overflows".to_owned()))?;
    usize::try_from(pixel_count)
        .map_err(|_| RendererError::Validation("RHI RGBA8 region size is too large".to_owned()))
}

fn depth32f_region_len(region: RhiTextureRegion) -> Result<usize, RendererError> {
    let pixel_count = region.width.checked_mul(region.height).ok_or_else(|| {
        RendererError::Validation("RHI depth32f region size overflows".to_owned())
    })?;
    usize::try_from(pixel_count)
        .map_err(|_| RendererError::Validation("RHI depth32f region size is too large".to_owned()))
}

fn rgba16f_region_len(region: RhiTextureRegion) -> Result<usize, RendererError> {
    let channel_count = region
        .width
        .checked_mul(region.height)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| RendererError::Validation("RHI RGBA16F region size overflows".to_owned()))?;
    usize::try_from(channel_count)
        .map_err(|_| RendererError::Validation("RHI RGBA16F region size is too large".to_owned()))
}

fn rgba32f_region_len(region: RhiTextureRegion) -> Result<usize, RendererError> {
    let channel_count = region
        .width
        .checked_mul(region.height)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| RendererError::Validation("RHI RGBA32F region size overflows".to_owned()))?;
    usize::try_from(channel_count)
        .map_err(|_| RendererError::Validation("RHI RGBA32F region size is too large".to_owned()))
}

fn copy_rgba16f_region_to_texture(
    texture: &mut [u16],
    texture_width: u32,
    region: RhiTextureRegion,
    data: &[u16],
) {
    let row_channels = region.width as usize * 4;
    let texture_stride = texture_width as usize * 4;
    for row in 0..region.height as usize {
        let src_start = row * row_channels;
        let dst_start = ((region.y as usize + row) * texture_stride) + region.x as usize * 4;
        texture[dst_start..dst_start + row_channels]
            .copy_from_slice(&data[src_start..src_start + row_channels]);
    }
}

fn copy_rgba32f_region_to_texture(
    texture: &mut [f32],
    texture_width: u32,
    region: RhiTextureRegion,
    data: &[f32],
) {
    let row_channels = region.width as usize * 4;
    let texture_stride = texture_width as usize * 4;
    for row in 0..region.height as usize {
        let src_start = row * row_channels;
        let dst_start = ((region.y as usize + row) * texture_stride) + region.x as usize * 4;
        texture[dst_start..dst_start + row_channels]
            .copy_from_slice(&data[src_start..src_start + row_channels]);
    }
}

fn copy_depth32f_region_to_texture(
    texture: &mut [f32],
    texture_width: u32,
    region: RhiTextureRegion,
    data: &[f32],
) {
    let row_values = region.width as usize;
    let texture_stride = texture_width as usize;
    for row in 0..region.height as usize {
        let src_start = row * row_values;
        let dst_start = ((region.y as usize + row) * texture_stride) + region.x as usize;
        texture[dst_start..dst_start + row_values]
            .copy_from_slice(&data[src_start..src_start + row_values]);
    }
}

fn copy_rgba8_region_to_texture(
    texture: &mut [u8],
    texture_width: u32,
    region: RhiTextureRegion,
    data: &[u8],
) {
    let row_bytes = region.width as usize * RGBA8_BYTES_PER_PIXEL as usize;
    let texture_stride = texture_width as usize * RGBA8_BYTES_PER_PIXEL as usize;
    for row in 0..region.height as usize {
        let src_start = row * row_bytes;
        let dst_start = ((region.y as usize + row) * texture_stride) + region.x as usize * 4;
        texture[dst_start..dst_start + row_bytes]
            .copy_from_slice(&data[src_start..src_start + row_bytes]);
    }
}

fn copy_rgba16f_region_from_texture(
    texture: &[u16],
    texture_width: u32,
    region: RhiTextureRegion,
) -> Vec<u16> {
    let row_channels = region.width as usize * 4;
    let texture_stride = texture_width as usize * 4;
    let mut out = Vec::with_capacity(row_channels * region.height as usize);
    for row in 0..region.height as usize {
        let src_start = ((region.y as usize + row) * texture_stride) + region.x as usize * 4;
        out.extend_from_slice(&texture[src_start..src_start + row_channels]);
    }
    out
}

fn copy_rgba32f_region_from_texture(
    texture: &[f32],
    texture_width: u32,
    region: RhiTextureRegion,
) -> Vec<f32> {
    let row_channels = region.width as usize * 4;
    let texture_stride = texture_width as usize * 4;
    let mut out = Vec::with_capacity(row_channels * region.height as usize);
    for row in 0..region.height as usize {
        let src_start = ((region.y as usize + row) * texture_stride) + region.x as usize * 4;
        out.extend_from_slice(&texture[src_start..src_start + row_channels]);
    }
    out
}

fn copy_depth32f_region_from_texture(
    texture: &[f32],
    texture_width: u32,
    region: RhiTextureRegion,
) -> Vec<f32> {
    let row_values = region.width as usize;
    let texture_stride = texture_width as usize;
    let mut out = Vec::with_capacity(row_values * region.height as usize);
    for row in 0..region.height as usize {
        let src_start = ((region.y as usize + row) * texture_stride) + region.x as usize;
        out.extend_from_slice(&texture[src_start..src_start + row_values]);
    }
    out
}

fn copy_rgba8_region_from_texture(
    texture: &[u8],
    texture_width: u32,
    region: RhiTextureRegion,
) -> Vec<u8> {
    let row_bytes = region.width as usize * RGBA8_BYTES_PER_PIXEL as usize;
    let texture_stride = texture_width as usize * RGBA8_BYTES_PER_PIXEL as usize;
    let mut out = Vec::with_capacity(row_bytes * region.height as usize);
    for row in 0..region.height as usize {
        let src_start = ((region.y as usize + row) * texture_stride) + region.x as usize * 4;
        out.extend_from_slice(&texture[src_start..src_start + row_bytes]);
    }
    out
}

#[cfg(feature = "backend-wgpu")]
fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        value
    } else {
        value.div_ceil(alignment) * alignment
    }
}

#[cfg(feature = "backend-wgpu")]
fn validate_buffer_range(
    buffer_size: u64,
    offset: u64,
    size: u64,
    operation: &str,
) -> Result<(), RendererError> {
    let end = offset.checked_add(size).ok_or_else(|| {
        RendererError::Validation(format!("RHI buffer {operation} range overflows u64"))
    })?;
    if end > buffer_size {
        return Err(RendererError::Validation(format!(
            "RHI buffer {operation} range exceeds buffer size"
        )));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn map_wgpu_buffer_usage(usage: RhiBufferUsage) -> wgpu::BufferUsages {
    let mut mapped = wgpu::BufferUsages::empty();
    if usage.contains(RhiBufferUsage::UNIFORM) {
        mapped |= wgpu::BufferUsages::UNIFORM;
    }
    if usage.contains(RhiBufferUsage::STORAGE) {
        mapped |= wgpu::BufferUsages::STORAGE;
    }
    if usage.contains(RhiBufferUsage::VERTEX) {
        mapped |= wgpu::BufferUsages::VERTEX;
    }
    if usage.contains(RhiBufferUsage::INDEX) {
        mapped |= wgpu::BufferUsages::INDEX;
    }
    if usage.contains(RhiBufferUsage::INDIRECT) {
        mapped |= wgpu::BufferUsages::INDIRECT;
    }
    if usage.contains(RhiBufferUsage::COPY_SRC) {
        mapped |= wgpu::BufferUsages::COPY_SRC;
    }
    if usage.contains(RhiBufferUsage::COPY_DST) {
        mapped |= wgpu::BufferUsages::COPY_DST;
    }
    mapped
}

#[cfg(feature = "backend-wgpu")]
fn map_rhi_texture_format(format: TextureFormat) -> wgpu::TextureFormat {
    match format {
        TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_color_texture_format(format: TextureFormat) -> Result<wgpu::TextureFormat, RendererError> {
    match format {
        TextureFormat::Depth32Float => Err(RendererError::PipelineCompile(
            "depth texture format cannot be used as a color target".to_owned(),
        )),
        format => Ok(map_rhi_texture_format(format)),
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_wgpu_texture_usage(usage: RhiTextureUsage) -> wgpu::TextureUsages {
    let mut mapped = wgpu::TextureUsages::empty();
    if usage.contains(RhiTextureUsage::SAMPLED) {
        mapped |= wgpu::TextureUsages::TEXTURE_BINDING;
    }
    if usage.contains(RhiTextureUsage::STORAGE) {
        mapped |= wgpu::TextureUsages::STORAGE_BINDING;
    }
    if usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        mapped |= wgpu::TextureUsages::RENDER_ATTACHMENT;
    }
    if usage.contains(RhiTextureUsage::COPY_SRC) {
        mapped |= wgpu::TextureUsages::COPY_SRC;
    }
    if usage.contains(RhiTextureUsage::COPY_DST) {
        mapped |= wgpu::TextureUsages::COPY_DST;
    }
    mapped
}

#[cfg(feature = "backend-wgpu")]
fn map_depth_format(format: DepthFormat) -> wgpu::TextureFormat {
    match format {
        DepthFormat::D16Unorm => wgpu::TextureFormat::Depth16Unorm,
        DepthFormat::D24Plus => wgpu::TextureFormat::Depth24Plus,
        DepthFormat::D24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
        DepthFormat::D32Float => wgpu::TextureFormat::Depth32Float,
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_vertex_format(format: VertexFormat) -> Result<wgpu::VertexFormat, RendererError> {
    match format {
        VertexFormat::Uint8x2 => Ok(wgpu::VertexFormat::Uint8x2),
        VertexFormat::Uint8x4 => Ok(wgpu::VertexFormat::Uint8x4),
        VertexFormat::Sint8x2 => Ok(wgpu::VertexFormat::Sint8x2),
        VertexFormat::Sint8x4 => Ok(wgpu::VertexFormat::Sint8x4),
        VertexFormat::Unorm8x2 => Ok(wgpu::VertexFormat::Unorm8x2),
        VertexFormat::Unorm8x4 => Ok(wgpu::VertexFormat::Unorm8x4),
        VertexFormat::Snorm8x2 => Ok(wgpu::VertexFormat::Snorm8x2),
        VertexFormat::Snorm8x4 => Ok(wgpu::VertexFormat::Snorm8x4),
        VertexFormat::Uint16x2 => Ok(wgpu::VertexFormat::Uint16x2),
        VertexFormat::Uint16x4 => Ok(wgpu::VertexFormat::Uint16x4),
        VertexFormat::Sint16x2 => Ok(wgpu::VertexFormat::Sint16x2),
        VertexFormat::Sint16x4 => Ok(wgpu::VertexFormat::Sint16x4),
        VertexFormat::Unorm16x2 => Ok(wgpu::VertexFormat::Unorm16x2),
        VertexFormat::Unorm16x4 => Ok(wgpu::VertexFormat::Unorm16x4),
        VertexFormat::Snorm16x2 => Ok(wgpu::VertexFormat::Snorm16x2),
        VertexFormat::Snorm16x4 => Ok(wgpu::VertexFormat::Snorm16x4),
        VertexFormat::Float16x2 => Ok(wgpu::VertexFormat::Float16x2),
        VertexFormat::Float16x4 => Ok(wgpu::VertexFormat::Float16x4),
        VertexFormat::Float64 => Ok(wgpu::VertexFormat::Float64),
        VertexFormat::Float64x2 => Ok(wgpu::VertexFormat::Float64x2),
        VertexFormat::Float64x3 => Ok(wgpu::VertexFormat::Float64x3),
        VertexFormat::Float64x4 => Ok(wgpu::VertexFormat::Float64x4),
        VertexFormat::Float32 => Ok(wgpu::VertexFormat::Float32),
        VertexFormat::Float32x2 => Ok(wgpu::VertexFormat::Float32x2),
        VertexFormat::Float32x3 => Ok(wgpu::VertexFormat::Float32x3),
        VertexFormat::Float32x4 => Ok(wgpu::VertexFormat::Float32x4),
        VertexFormat::Uint32 => Ok(wgpu::VertexFormat::Uint32),
        VertexFormat::Uint32x2 => Ok(wgpu::VertexFormat::Uint32x2),
        VertexFormat::Uint32x3 => Ok(wgpu::VertexFormat::Uint32x3),
        VertexFormat::Uint32x4 => Ok(wgpu::VertexFormat::Uint32x4),
        VertexFormat::Sint32 => Ok(wgpu::VertexFormat::Sint32),
        VertexFormat::Sint32x2 => Ok(wgpu::VertexFormat::Sint32x2),
        VertexFormat::Sint32x3 => Ok(wgpu::VertexFormat::Sint32x3),
        VertexFormat::Sint32x4 => Ok(wgpu::VertexFormat::Sint32x4),
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_vertex_step_mode(mode: VertexStepMode) -> wgpu::VertexStepMode {
    match mode {
        VertexStepMode::Vertex => wgpu::VertexStepMode::Vertex,
        VertexStepMode::Instance => wgpu::VertexStepMode::Instance,
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_index_format(format: RhiIndexFormat) -> wgpu::IndexFormat {
    match format {
        RhiIndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
        RhiIndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_primitive_topology(topology: RhiPrimitiveTopology) -> wgpu::PrimitiveTopology {
    match topology {
        RhiPrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
        RhiPrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
        RhiPrimitiveTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
        RhiPrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        RhiPrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_face(face: RhiFace) -> wgpu::Face {
    match face {
        RhiFace::Front => wgpu::Face::Front,
        RhiFace::Back => wgpu::Face::Back,
    }
}

#[cfg(feature = "backend-wgpu")]
fn map_compare_function(compare: RhiCompareFunction) -> wgpu::CompareFunction {
    match compare {
        RhiCompareFunction::Never => wgpu::CompareFunction::Never,
        RhiCompareFunction::Less => wgpu::CompareFunction::Less,
        RhiCompareFunction::Equal => wgpu::CompareFunction::Equal,
        RhiCompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
        RhiCompareFunction::Greater => wgpu::CompareFunction::Greater,
        RhiCompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
        RhiCompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        RhiCompareFunction::Always => wgpu::CompareFunction::Always,
    }
}

fn record_compute_statistics(
    active: &mut HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    desc: &RhiComputePassDesc,
) {
    let invocations = u64::from(desc.workgroups[0])
        .saturating_mul(u64::from(desc.workgroups[1]))
        .saturating_mul(u64::from(desc.workgroups[2]));
    for statistics in active.values_mut() {
        statistics.compute_shader_invocations = statistics
            .compute_shader_invocations
            .saturating_add(invocations);
        statistics.dispatch_calls = statistics.dispatch_calls.saturating_add(1);
    }
}

fn record_render_statistics(
    active: &mut HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    desc: &RhiRenderPassDesc,
) {
    let submitted_vertices = desc.index_count.unwrap_or(desc.vertex_count);
    let vertices = u64::from(submitted_vertices).saturating_mul(u64::from(desc.instance_count));
    let primitives =
        u64::from(submitted_vertices / 3).saturating_mul(u64::from(desc.instance_count));
    for statistics in active.values_mut() {
        statistics.input_assembly_vertices =
            statistics.input_assembly_vertices.saturating_add(vertices);
        statistics.input_assembly_primitives = statistics
            .input_assembly_primitives
            .saturating_add(primitives);
        statistics.vertex_shader_invocations = statistics
            .vertex_shader_invocations
            .saturating_add(vertices);
        statistics.clipping_invocations =
            statistics.clipping_invocations.saturating_add(primitives);
        statistics.clipping_primitives = statistics.clipping_primitives.saturating_add(primitives);
        statistics.fragment_shader_invocations = statistics
            .fragment_shader_invocations
            .saturating_add(primitives);
        statistics.draw_calls = statistics.draw_calls.saturating_add(1);
    }
}

fn record_occlusion_samples(active: &mut HashMap<RhiOcclusionQuery, u64>, samples: u64) {
    for value in active.values_mut() {
        *value = value.saturating_add(samples);
    }
}

fn record_indirect_render_statistics(
    active: &mut HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    desc: &RhiIndirectRenderPassDesc,
) {
    for statistics in active.values_mut() {
        statistics.draw_calls = statistics
            .draw_calls
            .saturating_add(u64::from(desc.draw_count));
    }
}

#[derive(Clone, Debug)]
struct HeadlessRhiCommandEncoder {
    state: Arc<Mutex<HeadlessRhiState>>,
    encoded_barriers: usize,
    encoded_compute_dispatches: usize,
    encoded_render_draws: usize,
    encoded_indirect_draws: usize,
    encoded_debug_groups: usize,
    open_debug_groups: usize,
    timestamp_writes: Vec<RhiTimestampQuery>,
    active_pipeline_statistics: HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    pipeline_statistics_writes: Vec<(RhiPipelineStatisticsQuery, RhiPipelineStatistics)>,
    active_occlusion_queries: HashMap<RhiOcclusionQuery, u64>,
    occlusion_writes: Vec<(RhiOcclusionQuery, u64)>,
}

impl RhiCommandEncoder for HeadlessRhiCommandEncoder {
    fn encode_resource_barrier(
        &mut self,
        desc: &RhiResourceBarrierDesc,
    ) -> Result<(), RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        validate_headless_resource_barrier(&state, desc)?;
        drop(state);
        self.encoded_barriers += 1;
        Ok(())
    }

    fn encode_compute_pass(&mut self, desc: &RhiComputePassDesc) -> Result<(), RendererError> {
        validate_rhi_label(desc.label.as_deref())?;
        if desc.workgroups.contains(&0) {
            return Err(RendererError::Validation(
                "RHI compute dispatch workgroups must be non-zero".to_owned(),
            ));
        }
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.compute_pipelines.contains(&desc.pipeline) {
            return Err(RendererError::Validation(format!(
                "unknown RHI compute pipeline: {}",
                desc.pipeline.0
            )));
        }
        validate_headless_compute_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        drop(state);
        record_compute_statistics(&mut self.active_pipeline_statistics, desc);
        self.encoded_compute_dispatches += 1;
        Ok(())
    }

    fn encode_render_pass(&mut self, desc: &RhiRenderPassDesc) -> Result<(), RendererError> {
        validate_render_pass_desc(desc)?;
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI graphics pipeline: {}",
                desc.pipeline.0
            )));
        };
        validate_headless_render_targets_for_pipeline(&state, pipeline, desc)?;
        if let Some(color_target) = desc.color_target {
            validate_headless_color_target(&state, color_target)?;
        }
        if let Some(depth_target) = desc.depth_target {
            validate_headless_depth_target(&state, depth_target)?;
        }
        validate_headless_vertex_buffer_bindings(
            &state,
            pipeline,
            &desc.vertex_buffers,
            desc.vertex_count,
            desc.instance_count,
        )?;
        if let Some(index_buffer) = &desc.index_buffer {
            let index_count = desc
                .index_count
                .expect("index_count is validated when index buffer is set");
            validate_headless_index_buffer_binding(&state, index_buffer, index_count)?;
        }
        validate_headless_render_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        drop(state);
        record_render_statistics(&mut self.active_pipeline_statistics, desc);
        record_occlusion_samples(&mut self.active_occlusion_queries, 1);
        self.encoded_render_draws += 1;
        Ok(())
    }

    fn encode_indirect_render_pass(
        &mut self,
        desc: &RhiIndirectRenderPassDesc,
    ) -> Result<(), RendererError> {
        validate_indirect_render_pass_desc(desc)?;
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI graphics pipeline: {}",
                desc.pipeline.0
            )));
        };
        validate_headless_indirect_render_targets_for_pipeline(&state, pipeline, desc)?;
        if let Some(color_target) = desc.color_target {
            validate_headless_color_target(&state, color_target)?;
        }
        if let Some(depth_target) = desc.depth_target {
            validate_headless_depth_target(&state, depth_target)?;
        }
        validate_headless_vertex_buffer_bindings(&state, pipeline, &desc.vertex_buffers, 1, 1)?;
        validate_headless_render_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        let Some(indirect_buffer) = state.buffers.get(&desc.indirect_buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI indirect buffer: {}",
                desc.indirect_buffer.0
            )));
        };
        if !indirect_buffer.usage.contains(RhiBufferUsage::INDIRECT) {
            return Err(RendererError::Validation(
                "RHI indirect draw requires INDIRECT buffer usage".to_owned(),
            ));
        }
        validate_indirect_buffer_range(indirect_buffer.bytes.len() as u64, desc)?;
        drop(state);
        record_indirect_render_statistics(&mut self.active_pipeline_statistics, desc);
        record_occlusion_samples(
            &mut self.active_occlusion_queries,
            u64::from(desc.draw_count),
        );
        self.encoded_indirect_draws += desc.draw_count as usize;
        Ok(())
    }

    fn encode_indexed_indirect_render_pass(
        &mut self,
        desc: &RhiIndexedIndirectRenderPassDesc,
    ) -> Result<(), RendererError> {
        validate_indexed_indirect_render_pass_desc(desc)?;
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI graphics pipeline: {}",
                desc.pipeline.0
            )));
        };
        validate_headless_indexed_indirect_render_targets_for_pipeline(&state, pipeline, desc)?;
        if let Some(color_target) = desc.color_target {
            validate_headless_color_target(&state, color_target)?;
        }
        if let Some(depth_target) = desc.depth_target {
            validate_headless_depth_target(&state, depth_target)?;
        }
        validate_headless_vertex_buffer_bindings(&state, pipeline, &desc.vertex_buffers, 1, 1)?;
        validate_headless_index_buffer_binding(&state, &desc.index_buffer, 1)?;
        validate_headless_render_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        let Some(indirect_buffer) = state.buffers.get(&desc.indirect_buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI indirect buffer: {}",
                desc.indirect_buffer.0
            )));
        };
        if !indirect_buffer.usage.contains(RhiBufferUsage::INDIRECT) {
            return Err(RendererError::Validation(
                "RHI indexed indirect draw requires INDIRECT buffer usage".to_owned(),
            ));
        }
        validate_indexed_indirect_buffer_range(indirect_buffer.bytes.len() as u64, desc)?;
        drop(state);
        for statistics in self.active_pipeline_statistics.values_mut() {
            statistics.draw_calls = statistics
                .draw_calls
                .saturating_add(u64::from(desc.draw_count));
        }
        record_occlusion_samples(
            &mut self.active_occlusion_queries,
            u64::from(desc.draw_count),
        );
        self.encoded_indirect_draws += desc.draw_count as usize;
        Ok(())
    }

    fn begin_pipeline_statistics(
        &mut self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<(), RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.pipeline_statistics_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI pipeline statistics query: {}",
                query.0
            )));
        }
        drop(state);
        if self.active_pipeline_statistics.contains_key(&query) {
            return Err(RendererError::Validation(
                "RHI pipeline statistics query is already active".to_owned(),
            ));
        }
        self.active_pipeline_statistics
            .insert(query, RhiPipelineStatistics::default());
        Ok(())
    }

    fn end_pipeline_statistics(
        &mut self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<(), RendererError> {
        let Some(statistics) = self.active_pipeline_statistics.remove(&query) else {
            return Err(RendererError::Validation(
                "RHI pipeline statistics query is not active".to_owned(),
            ));
        };
        self.pipeline_statistics_writes.push((query, statistics));
        Ok(())
    }

    fn begin_occlusion_query(&mut self, query: RhiOcclusionQuery) -> Result<(), RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.occlusion_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI occlusion query: {}",
                query.0
            )));
        }
        drop(state);
        if self.active_occlusion_queries.contains_key(&query) {
            return Err(RendererError::Validation(
                "RHI occlusion query is already active".to_owned(),
            ));
        }
        self.active_occlusion_queries.insert(query, 0);
        Ok(())
    }

    fn end_occlusion_query(&mut self, query: RhiOcclusionQuery) -> Result<(), RendererError> {
        let Some(samples_passed) = self.active_occlusion_queries.remove(&query) else {
            return Err(RendererError::Validation(
                "RHI occlusion query is not active".to_owned(),
            ));
        };
        self.occlusion_writes.push((query, samples_passed));
        Ok(())
    }

    fn write_timestamp(&mut self, query: RhiTimestampQuery) -> Result<(), RendererError> {
        let state = self.state.lock().expect("headless RHI mutex poisoned");
        if !state.timestamp_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI timestamp query: {}",
                query.0
            )));
        }
        drop(state);
        self.timestamp_writes.push(query);
        Ok(())
    }

    fn push_debug_group(&mut self, label: &str) -> Result<(), RendererError> {
        validate_debug_group_label(label)?;
        self.encoded_debug_groups += 1;
        self.open_debug_groups += 1;
        Ok(())
    }

    fn pop_debug_group(&mut self) -> Result<(), RendererError> {
        if self.open_debug_groups == 0 {
            return Err(RendererError::Validation(
                "RHI debug group stack is empty".to_owned(),
            ));
        }
        self.open_debug_groups -= 1;
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<RhiCommandBuffer, RendererError> {
        validate_encoder_finish_state(
            self.open_debug_groups,
            self.active_pipeline_statistics.len(),
            self.active_occlusion_queries.len(),
        )?;
        let mut state = self.state.lock().expect("headless RHI mutex poisoned");
        let command = RhiCommandBuffer(state.allocate());
        state.finished_command_buffers.insert(command);
        state.encoded_barriers += self.encoded_barriers;
        state.encoded_compute_dispatches += self.encoded_compute_dispatches;
        state.encoded_render_draws += self.encoded_render_draws;
        state.encoded_indirect_draws += self.encoded_indirect_draws;
        state.encoded_debug_groups += self.encoded_debug_groups;
        for query in &self.timestamp_writes {
            let timestamp = state.next_timestamp_ns;
            state.next_timestamp_ns += 1_000;
            state.timestamp_results.insert(*query, timestamp);
        }
        for (query, statistics) in self.pipeline_statistics_writes {
            state.pipeline_statistics_results.insert(query, statistics);
        }
        for (query, samples_passed) in self.occlusion_writes {
            state.occlusion_results.insert(query, samples_passed);
        }
        state.encoded_timestamp_writes += self.timestamp_writes.len();
        Ok(command)
    }
}

#[cfg(feature = "backend-wgpu")]
struct WgpuRhiCommandEncoder {
    state: Arc<Mutex<WgpuRhiState>>,
    encoder: Option<wgpu::CommandEncoder>,
    encoded_barriers: usize,
    encoded_compute_dispatches: usize,
    encoded_render_draws: usize,
    encoded_indirect_draws: usize,
    encoded_debug_groups: usize,
    open_debug_groups: usize,
    timestamp_writes: Vec<RhiTimestampQuery>,
    active_pipeline_statistics: HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    pipeline_statistics_writes: Vec<(RhiPipelineStatisticsQuery, RhiPipelineStatistics)>,
    active_occlusion_queries: HashMap<RhiOcclusionQuery, u64>,
    occlusion_gpu_writes: HashSet<RhiOcclusionQuery>,
    occlusion_writes: Vec<(RhiOcclusionQuery, u64)>,
}

#[cfg(feature = "backend-wgpu")]
impl RhiCommandEncoder for WgpuRhiCommandEncoder {
    fn encode_resource_barrier(
        &mut self,
        desc: &RhiResourceBarrierDesc,
    ) -> Result<(), RendererError> {
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        validate_wgpu_resource_barrier(&state, desc)?;
        drop(state);
        self.encoded_barriers += 1;
        Ok(())
    }

    fn encode_compute_pass(&mut self, desc: &RhiComputePassDesc) -> Result<(), RendererError> {
        validate_rhi_label(desc.label.as_deref())?;
        if desc.workgroups.contains(&0) {
            return Err(RendererError::Validation(
                "RHI compute dispatch workgroups must be non-zero".to_owned(),
            ));
        }
        let Some(encoder) = self.encoder.as_mut() else {
            return Err(RendererError::Validation(
                "RHI command encoder has already been finished".to_owned(),
            ));
        };
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let Some(pipeline) = state.compute_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI compute pipeline: {}",
                desc.pipeline.0
            )));
        };
        let bind_groups =
            validate_wgpu_compute_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: desc.label.as_deref(),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            for (index, bind_group) in &bind_groups {
                pass.set_bind_group(*index, *bind_group, &[]);
            }
            pass.dispatch_workgroups(desc.workgroups[0], desc.workgroups[1], desc.workgroups[2]);
        }
        record_compute_statistics(&mut self.active_pipeline_statistics, desc);
        self.encoded_compute_dispatches += 1;
        Ok(())
    }

    fn encode_render_pass(&mut self, desc: &RhiRenderPassDesc) -> Result<(), RendererError> {
        validate_render_pass_desc(desc)?;
        let Some(encoder) = self.encoder.as_mut() else {
            return Err(RendererError::Validation(
                "RHI command encoder has already been finished".to_owned(),
            ));
        };
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI graphics pipeline: {}",
                desc.pipeline.0
            )));
        };
        validate_wgpu_render_targets_for_pipeline(&state, pipeline, desc)?;
        let color_view = desc
            .color_target
            .map(|target| {
                let texture = validate_wgpu_color_target(&state, target)?;
                Ok(texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let depth_view = desc
            .depth_target
            .map(|target| {
                let texture = validate_wgpu_depth_target(&state, target)?;
                Ok(texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let color_attachment = color_view.as_ref().map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        let color_attachments = color_attachment.into_iter().collect::<Vec<_>>();
        let depth_stencil_attachment =
            depth_view
                .as_ref()
                .map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                });
        let occlusion_query = self
            .active_occlusion_queries
            .keys()
            .find(|query| !self.occlusion_gpu_writes.contains(query))
            .copied();
        let occlusion_query_set = occlusion_query
            .and_then(|query| state.occlusion_queries.get(&query))
            .map(|resource| Arc::clone(&resource.query_set));
        let vertex_buffers = validate_wgpu_vertex_buffer_bindings(
            &state,
            pipeline,
            &desc.vertex_buffers,
            desc.vertex_count,
            desc.instance_count,
        )?;
        let index_buffer = desc
            .index_buffer
            .as_ref()
            .map(|binding| {
                let index_count = desc
                    .index_count
                    .expect("index_count is validated when index buffer is set");
                validate_wgpu_index_buffer_binding(&state, binding, index_count)
            })
            .transpose()?;
        let bind_groups =
            validate_wgpu_render_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: desc.label.as_deref(),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                occlusion_query_set: occlusion_query_set.as_deref(),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            for (index, bind_group) in &bind_groups {
                pass.set_bind_group(*index, *bind_group, &[]);
            }
            for (slot, buffer, offset) in &vertex_buffers {
                pass.set_vertex_buffer(*slot, buffer.slice(*offset..));
            }
            if let Some((buffer, offset, format)) = &index_buffer {
                pass.set_index_buffer(buffer.slice(*offset..), *format);
            }
            if occlusion_query_set.is_some() {
                pass.begin_occlusion_query(0);
            }
            if let Some(index_count) = desc.index_count {
                pass.draw_indexed(0..index_count, 0, 0..desc.instance_count);
            } else {
                pass.draw(0..desc.vertex_count, 0..desc.instance_count);
            }
            if occlusion_query_set.is_some() {
                pass.end_occlusion_query();
            }
        }
        if let Some(query) = occlusion_query {
            self.occlusion_gpu_writes.insert(query);
        }
        record_render_statistics(&mut self.active_pipeline_statistics, desc);
        record_occlusion_samples(&mut self.active_occlusion_queries, 1);
        self.encoded_render_draws += 1;
        Ok(())
    }

    fn encode_indirect_render_pass(
        &mut self,
        desc: &RhiIndirectRenderPassDesc,
    ) -> Result<(), RendererError> {
        validate_indirect_render_pass_desc(desc)?;
        let Some(encoder) = self.encoder.as_mut() else {
            return Err(RendererError::Validation(
                "RHI command encoder has already been finished".to_owned(),
            ));
        };
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI graphics pipeline: {}",
                desc.pipeline.0
            )));
        };
        validate_wgpu_indirect_render_targets_for_pipeline(&state, pipeline, desc)?;
        let vertex_buffers =
            validate_wgpu_vertex_buffer_bindings(&state, pipeline, &desc.vertex_buffers, 1, 1)?;
        let bind_groups =
            validate_wgpu_render_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        let Some(indirect_buffer) = state.buffers.get(&desc.indirect_buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI indirect buffer: {}",
                desc.indirect_buffer.0
            )));
        };
        if !indirect_buffer.usage.contains(RhiBufferUsage::INDIRECT) {
            return Err(RendererError::Validation(
                "RHI indirect draw requires INDIRECT buffer usage".to_owned(),
            ));
        }
        validate_indirect_buffer_range(indirect_buffer.buffer.size(), desc)?;
        let color_view = desc
            .color_target
            .map(|target| {
                let texture = validate_wgpu_color_target(&state, target)?;
                Ok(texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let depth_view = desc
            .depth_target
            .map(|target| {
                let texture = validate_wgpu_depth_target(&state, target)?;
                Ok(texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let color_attachment = color_view.as_ref().map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        let color_attachments = color_attachment.into_iter().collect::<Vec<_>>();
        let depth_stencil_attachment =
            depth_view
                .as_ref()
                .map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                });
        let occlusion_query = self
            .active_occlusion_queries
            .keys()
            .find(|query| !self.occlusion_gpu_writes.contains(query))
            .copied();
        let occlusion_query_set = occlusion_query
            .and_then(|query| state.occlusion_queries.get(&query))
            .map(|resource| Arc::clone(&resource.query_set));
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: desc.label.as_deref(),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                occlusion_query_set: occlusion_query_set.as_deref(),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            for (index, bind_group) in &bind_groups {
                pass.set_bind_group(*index, *bind_group, &[]);
            }
            for (slot, buffer, offset) in &vertex_buffers {
                pass.set_vertex_buffer(*slot, buffer.slice(*offset..));
            }
            if occlusion_query_set.is_some() {
                pass.begin_occlusion_query(0);
            }
            for draw_index in 0..desc.draw_count {
                pass.draw_indirect(
                    &indirect_buffer.buffer,
                    desc.indirect_offset + u64::from(draw_index) * desc.draw_stride,
                );
            }
            if occlusion_query_set.is_some() {
                pass.end_occlusion_query();
            }
        }
        if let Some(query) = occlusion_query {
            self.occlusion_gpu_writes.insert(query);
        }
        record_indirect_render_statistics(&mut self.active_pipeline_statistics, desc);
        record_occlusion_samples(
            &mut self.active_occlusion_queries,
            u64::from(desc.draw_count),
        );
        self.encoded_indirect_draws += desc.draw_count as usize;
        Ok(())
    }

    fn encode_indexed_indirect_render_pass(
        &mut self,
        desc: &RhiIndexedIndirectRenderPassDesc,
    ) -> Result<(), RendererError> {
        validate_indexed_indirect_render_pass_desc(desc)?;
        let Some(encoder) = self.encoder.as_mut() else {
            return Err(RendererError::Validation(
                "RHI command encoder has already been finished".to_owned(),
            ));
        };
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let Some(pipeline) = state.graphics_pipelines.get(&desc.pipeline) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI graphics pipeline: {}",
                desc.pipeline.0
            )));
        };
        validate_wgpu_indexed_indirect_render_targets_for_pipeline(&state, pipeline, desc)?;
        let vertex_buffers =
            validate_wgpu_vertex_buffer_bindings(&state, pipeline, &desc.vertex_buffers, 1, 1)?;
        let (index_buffer, index_offset, index_format) =
            validate_wgpu_index_buffer_binding(&state, &desc.index_buffer, 1)?;
        let bind_groups =
            validate_wgpu_render_bind_groups(&state, desc.pipeline, &desc.bind_groups)?;
        let Some(indirect_buffer) = state.buffers.get(&desc.indirect_buffer) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI indirect buffer: {}",
                desc.indirect_buffer.0
            )));
        };
        if !indirect_buffer.usage.contains(RhiBufferUsage::INDIRECT) {
            return Err(RendererError::Validation(
                "RHI indexed indirect draw requires INDIRECT buffer usage".to_owned(),
            ));
        }
        validate_indexed_indirect_buffer_range(indirect_buffer.buffer.size(), desc)?;
        let color_view = desc
            .color_target
            .map(|target| {
                let texture = validate_wgpu_color_target(&state, target)?;
                Ok(texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let depth_view = desc
            .depth_target
            .map(|target| {
                let texture = validate_wgpu_depth_target(&state, target)?;
                Ok(texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let color_attachment = color_view.as_ref().map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        let color_attachments = color_attachment.into_iter().collect::<Vec<_>>();
        let depth_stencil_attachment =
            depth_view
                .as_ref()
                .map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                });
        let occlusion_query = self
            .active_occlusion_queries
            .keys()
            .find(|query| !self.occlusion_gpu_writes.contains(query))
            .copied();
        let occlusion_query_set = occlusion_query
            .and_then(|query| state.occlusion_queries.get(&query))
            .map(|resource| Arc::clone(&resource.query_set));
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: desc.label.as_deref(),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                occlusion_query_set: occlusion_query_set.as_deref(),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline.pipeline);
            for (index, bind_group) in &bind_groups {
                pass.set_bind_group(*index, *bind_group, &[]);
            }
            for (slot, buffer, offset) in &vertex_buffers {
                pass.set_vertex_buffer(*slot, buffer.slice(*offset..));
            }
            pass.set_index_buffer(index_buffer.slice(index_offset..), index_format);
            if occlusion_query_set.is_some() {
                pass.begin_occlusion_query(0);
            }
            for draw_index in 0..desc.draw_count {
                pass.draw_indexed_indirect(
                    &indirect_buffer.buffer,
                    desc.indirect_offset + u64::from(draw_index) * desc.draw_stride,
                );
            }
            if occlusion_query_set.is_some() {
                pass.end_occlusion_query();
            }
        }
        if let Some(query) = occlusion_query {
            self.occlusion_gpu_writes.insert(query);
        }
        for statistics in self.active_pipeline_statistics.values_mut() {
            statistics.draw_calls = statistics
                .draw_calls
                .saturating_add(u64::from(desc.draw_count));
        }
        record_occlusion_samples(
            &mut self.active_occlusion_queries,
            u64::from(desc.draw_count),
        );
        self.encoded_indirect_draws += desc.draw_count as usize;
        Ok(())
    }

    fn begin_pipeline_statistics(
        &mut self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<(), RendererError> {
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        if !state.pipeline_statistics_queries.contains(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI pipeline statistics query: {}",
                query.0
            )));
        }
        drop(state);
        if self.active_pipeline_statistics.contains_key(&query) {
            return Err(RendererError::Validation(
                "RHI pipeline statistics query is already active".to_owned(),
            ));
        }
        self.active_pipeline_statistics
            .insert(query, RhiPipelineStatistics::default());
        Ok(())
    }

    fn end_pipeline_statistics(
        &mut self,
        query: RhiPipelineStatisticsQuery,
    ) -> Result<(), RendererError> {
        let Some(statistics) = self.active_pipeline_statistics.remove(&query) else {
            return Err(RendererError::Validation(
                "RHI pipeline statistics query is not active".to_owned(),
            ));
        };
        self.pipeline_statistics_writes.push((query, statistics));
        Ok(())
    }

    fn begin_occlusion_query(&mut self, query: RhiOcclusionQuery) -> Result<(), RendererError> {
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        if !state.occlusion_queries.contains_key(&query) {
            return Err(RendererError::Validation(format!(
                "unknown RHI occlusion query: {}",
                query.0
            )));
        }
        drop(state);
        if self.active_occlusion_queries.contains_key(&query) {
            return Err(RendererError::Validation(
                "RHI occlusion query is already active".to_owned(),
            ));
        }
        self.active_occlusion_queries.insert(query, 0);
        Ok(())
    }

    fn end_occlusion_query(&mut self, query: RhiOcclusionQuery) -> Result<(), RendererError> {
        let Some(samples_passed) = self.active_occlusion_queries.remove(&query) else {
            return Err(RendererError::Validation(
                "RHI occlusion query is not active".to_owned(),
            ));
        };
        self.occlusion_writes.push((query, samples_passed));
        Ok(())
    }

    fn write_timestamp(&mut self, query: RhiTimestampQuery) -> Result<(), RendererError> {
        let state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let Some(resource) = state.timestamp_queries.get(&query) else {
            return Err(RendererError::Validation(format!(
                "unknown RHI timestamp query: {}",
                query.0
            )));
        };
        if let (Some(query_set), Some(_)) = (&resource.query_set, &resource.resolve_buffer) {
            let Some(encoder) = self.encoder.as_mut() else {
                return Err(RendererError::Validation(
                    "RHI command encoder has already been finished".to_owned(),
                ));
            };
            encoder.write_timestamp(query_set, 0);
        }
        drop(state);
        self.timestamp_writes.push(query);
        Ok(())
    }

    fn push_debug_group(&mut self, label: &str) -> Result<(), RendererError> {
        validate_debug_group_label(label)?;
        let Some(encoder) = self.encoder.as_mut() else {
            return Err(RendererError::Validation(
                "RHI command encoder has already been finished".to_owned(),
            ));
        };
        encoder.push_debug_group(label);
        self.encoded_debug_groups += 1;
        self.open_debug_groups += 1;
        Ok(())
    }

    fn pop_debug_group(&mut self) -> Result<(), RendererError> {
        if self.open_debug_groups == 0 {
            return Err(RendererError::Validation(
                "RHI debug group stack is empty".to_owned(),
            ));
        }
        let Some(encoder) = self.encoder.as_mut() else {
            return Err(RendererError::Validation(
                "RHI command encoder has already been finished".to_owned(),
            ));
        };
        encoder.pop_debug_group();
        self.open_debug_groups -= 1;
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<RhiCommandBuffer, RendererError> {
        validate_encoder_finish_state(
            self.open_debug_groups,
            self.active_pipeline_statistics.len(),
            self.active_occlusion_queries.len(),
        )?;
        let mut state = self.state.lock().expect("wgpu RHI mutex poisoned");
        let command = RhiCommandBuffer(state.allocate());
        let mut encoder = self
            .encoder
            .take()
            .expect("wgpu RHI command encoder is finished once");
        for query in &self.timestamp_writes {
            if let Some(resource) = state.timestamp_queries.get(query) {
                if let (Some(query_set), Some(resolve_buffer), Some(readback_buffer)) = (
                    &resource.query_set,
                    &resource.resolve_buffer,
                    &resource.readback_buffer,
                ) {
                    encoder.resolve_query_set(query_set, 0..1, resolve_buffer, 0);
                    encoder.copy_buffer_to_buffer(
                        resolve_buffer,
                        0,
                        readback_buffer,
                        0,
                        std::mem::size_of::<u64>() as u64,
                    );
                }
            }
        }
        for (query, _) in &self.occlusion_writes {
            if let Some(resource) = state.occlusion_queries.get(query) {
                encoder.resolve_query_set(&resource.query_set, 0..1, &resource.resolve_buffer, 0);
                encoder.copy_buffer_to_buffer(
                    &resource.resolve_buffer,
                    0,
                    &resource.readback_buffer,
                    0,
                    std::mem::size_of::<u64>() as u64,
                );
            }
        }
        state
            .finished_command_buffers
            .insert(command, encoder.finish());
        for (query, statistics) in self.pipeline_statistics_writes {
            state.pipeline_statistics_results.insert(query, statistics);
        }
        for (query, samples_passed) in self.occlusion_writes {
            if !state.occlusion_queries.contains_key(&query) {
                state.occlusion_results.insert(query, samples_passed);
            }
        }
        state.encoded_barriers += self.encoded_barriers;
        state.encoded_compute_dispatches += self.encoded_compute_dispatches;
        state.encoded_render_draws += self.encoded_render_draws;
        state.encoded_indirect_draws += self.encoded_indirect_draws;
        state.encoded_debug_groups += self.encoded_debug_groups;
        for query in &self.timestamp_writes {
            if state
                .timestamp_queries
                .get(query)
                .is_some_and(|resource| resource.query_set.is_none())
            {
                let timestamp = state.next_timestamp_ns;
                state.next_timestamp_ns += 1_000;
                state.timestamp_results.insert(*query, timestamp);
            }
        }
        state.encoded_timestamp_writes += self.timestamp_writes.len();
        Ok(command)
    }
}

#[derive(Clone, Debug)]
struct HeadlessRhiState {
    next_id: u64,
    next_submission: u64,
    buffers: HashMap<RhiBuffer, HeadlessBufferState>,
    textures: HashMap<RhiTexture, HeadlessTextureState>,
    samplers: HashSet<RhiSampler>,
    bind_groups: HashMap<RhiBindGroup, HeadlessBindGroupState>,
    shader_modules: HashSet<RhiShaderModule>,
    graphics_pipelines: HashMap<RhiGraphicsPipeline, HeadlessGraphicsPipelineState>,
    compute_pipelines: HashSet<RhiComputePipeline>,
    timestamp_queries: HashSet<RhiTimestampQuery>,
    pipeline_statistics_queries: HashSet<RhiPipelineStatisticsQuery>,
    occlusion_queries: HashSet<RhiOcclusionQuery>,
    timestamp_results: HashMap<RhiTimestampQuery, u64>,
    pipeline_statistics_results: HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    occlusion_results: HashMap<RhiOcclusionQuery, u64>,
    finished_command_buffers: HashSet<RhiCommandBuffer>,
    submitted_command_buffers: HashSet<RhiCommandBuffer>,
    encoded_barriers: usize,
    encoded_compute_dispatches: usize,
    encoded_render_draws: usize,
    encoded_indirect_draws: usize,
    encoded_timestamp_writes: usize,
    encoded_debug_groups: usize,
    next_timestamp_ns: u64,
    last_poll: Option<PollMode>,
}

#[derive(Clone, Debug)]
struct HeadlessBufferState {
    bytes: Vec<u8>,
    usage: RhiBufferUsage,
}

impl HeadlessBufferState {
    fn new(desc: &RhiBufferDesc) -> Self {
        Self {
            bytes: vec![0; desc.size as usize],
            usage: desc.usage,
        }
    }
}

#[derive(Clone, Debug)]
struct HeadlessTextureState {
    width: u32,
    height: u32,
    samples: u32,
    format: TextureFormat,
    usage: RhiTextureUsage,
    rgba8: Vec<u8>,
    rgba16f: Vec<u16>,
    rgba32f: Vec<f32>,
    depth32f: Vec<f32>,
}

impl HeadlessTextureState {
    fn new(desc: &RhiTextureDesc) -> Self {
        Self {
            width: desc.width,
            height: desc.height,
            samples: desc.samples,
            format: desc.format,
            usage: desc.usage,
            rgba8: vec![
                0;
                desc.width as usize * desc.height as usize * RGBA8_BYTES_PER_PIXEL as usize
            ],
            rgba16f: vec![0; desc.width as usize * desc.height as usize * 4],
            rgba32f: vec![0.0; desc.width as usize * desc.height as usize * 4],
            depth32f: vec![0.0; desc.width as usize * desc.height as usize],
        }
    }
}

#[derive(Clone, Debug)]
struct HeadlessBindGroupState {
    owner: RhiBindGroupOwner,
    group_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RhiBindGroupOwner {
    Graphics(RhiGraphicsPipeline),
    Compute(RhiComputePipeline),
}

impl HeadlessBindGroupState {
    fn new_graphics(desc: &RhiBindGroupDesc) -> Self {
        Self {
            owner: RhiBindGroupOwner::Graphics(desc.pipeline),
            group_index: desc.group_index,
        }
    }

    fn new_compute(desc: &RhiComputeBindGroupDesc) -> Self {
        Self {
            owner: RhiBindGroupOwner::Compute(desc.pipeline),
            group_index: desc.group_index,
        }
    }
}

#[derive(Clone, Debug)]
struct HeadlessGraphicsPipelineState {
    color_format: Option<TextureFormat>,
    depth_format: Option<DepthFormat>,
    sample_count: u32,
    vertex_buffer_layouts: Vec<RhiVertexBufferLayout>,
}

impl HeadlessGraphicsPipelineState {
    fn new(desc: &RhiGraphicsPipelineDesc) -> Self {
        Self {
            color_format: desc.color_format,
            depth_format: desc.depth.map(|depth| depth.format),
            sample_count: desc.sample_count,
            vertex_buffer_layouts: desc.vertex_buffers.clone(),
        }
    }
}

#[cfg(feature = "backend-wgpu")]
struct WgpuRhiState {
    next_id: u64,
    next_submission: u64,
    buffers: HashMap<RhiBuffer, WgpuBufferState>,
    textures: HashMap<RhiTexture, WgpuTextureState>,
    samplers: HashMap<RhiSampler, Arc<wgpu::Sampler>>,
    bind_groups: HashMap<RhiBindGroup, WgpuBindGroupState>,
    shader_modules: HashMap<RhiShaderModule, wgpu::ShaderModule>,
    graphics_pipelines: HashMap<RhiGraphicsPipeline, WgpuGraphicsPipelineState>,
    compute_pipelines: HashMap<RhiComputePipeline, wgpu::ComputePipeline>,
    timestamp_queries: HashMap<RhiTimestampQuery, WgpuTimestampQueryResource>,
    pipeline_statistics_queries: HashSet<RhiPipelineStatisticsQuery>,
    occlusion_queries: HashMap<RhiOcclusionQuery, WgpuOcclusionQueryResource>,
    timestamp_results: HashMap<RhiTimestampQuery, u64>,
    pipeline_statistics_results: HashMap<RhiPipelineStatisticsQuery, RhiPipelineStatistics>,
    occlusion_results: HashMap<RhiOcclusionQuery, u64>,
    finished_command_buffers: HashMap<RhiCommandBuffer, wgpu::CommandBuffer>,
    submitted_command_buffers: HashSet<RhiCommandBuffer>,
    encoded_barriers: usize,
    encoded_compute_dispatches: usize,
    encoded_render_draws: usize,
    encoded_indirect_draws: usize,
    encoded_timestamp_writes: usize,
    encoded_debug_groups: usize,
    next_timestamp_ns: u64,
    last_poll: Option<PollMode>,
}

#[cfg(feature = "backend-wgpu")]
#[derive(Clone)]
struct WgpuBufferState {
    buffer: Arc<wgpu::Buffer>,
    size: u64,
    usage: RhiBufferUsage,
}

#[cfg(feature = "backend-wgpu")]
#[derive(Clone)]
struct WgpuTextureState {
    texture: Arc<wgpu::Texture>,
    width: u32,
    height: u32,
    format: TextureFormat,
    usage: RhiTextureUsage,
    samples: u32,
}

#[cfg(feature = "backend-wgpu")]
struct WgpuBindGroupState {
    bind_group: wgpu::BindGroup,
    owner: RhiBindGroupOwner,
    group_index: u32,
}

#[cfg(feature = "backend-wgpu")]
impl WgpuBindGroupState {
    fn new_graphics(bind_group: wgpu::BindGroup, desc: &RhiBindGroupDesc) -> Self {
        Self {
            bind_group,
            owner: RhiBindGroupOwner::Graphics(desc.pipeline),
            group_index: desc.group_index,
        }
    }

    fn new_compute(bind_group: wgpu::BindGroup, desc: &RhiComputeBindGroupDesc) -> Self {
        Self {
            bind_group,
            owner: RhiBindGroupOwner::Compute(desc.pipeline),
            group_index: desc.group_index,
        }
    }
}

#[cfg(feature = "backend-wgpu")]
struct WgpuGraphicsPipelineState {
    pipeline: wgpu::RenderPipeline,
    color_format: Option<TextureFormat>,
    depth_format: Option<DepthFormat>,
    sample_count: u32,
    vertex_buffer_layouts: Vec<RhiVertexBufferLayout>,
}

#[cfg(feature = "backend-wgpu")]
impl WgpuGraphicsPipelineState {
    fn new(pipeline: wgpu::RenderPipeline, desc: &RhiGraphicsPipelineDesc) -> Self {
        Self {
            pipeline,
            color_format: desc.color_format,
            depth_format: desc.depth.map(|depth| depth.format),
            sample_count: desc.sample_count,
            vertex_buffer_layouts: desc.vertex_buffers.clone(),
        }
    }
}

#[cfg(feature = "backend-wgpu")]
struct WgpuTimestampQueryResource {
    query_set: Option<Arc<wgpu::QuerySet>>,
    resolve_buffer: Option<Arc<wgpu::Buffer>>,
    readback_buffer: Option<Arc<wgpu::Buffer>>,
}

#[cfg(feature = "backend-wgpu")]
#[derive(Clone, Debug)]
struct WgpuOcclusionQueryResource {
    query_set: Arc<wgpu::QuerySet>,
    resolve_buffer: Arc<wgpu::Buffer>,
    readback_buffer: Arc<wgpu::Buffer>,
}

impl Default for HeadlessRhiState {
    fn default() -> Self {
        Self {
            next_id: 1,
            next_submission: 1,
            buffers: HashMap::new(),
            textures: HashMap::new(),
            samplers: HashSet::new(),
            bind_groups: HashMap::new(),
            shader_modules: HashSet::new(),
            graphics_pipelines: HashMap::new(),
            compute_pipelines: HashSet::new(),
            timestamp_queries: HashSet::new(),
            pipeline_statistics_queries: HashSet::new(),
            occlusion_queries: HashSet::new(),
            timestamp_results: HashMap::new(),
            pipeline_statistics_results: HashMap::new(),
            occlusion_results: HashMap::new(),
            finished_command_buffers: HashSet::new(),
            submitted_command_buffers: HashSet::new(),
            encoded_barriers: 0,
            encoded_compute_dispatches: 0,
            encoded_render_draws: 0,
            encoded_indirect_draws: 0,
            encoded_timestamp_writes: 0,
            encoded_debug_groups: 0,
            next_timestamp_ns: 1_000,
            last_poll: None,
        }
    }
}

#[cfg(feature = "backend-wgpu")]
impl Default for WgpuRhiState {
    fn default() -> Self {
        Self {
            next_id: 1,
            next_submission: 1,
            buffers: HashMap::new(),
            textures: HashMap::new(),
            samplers: HashMap::new(),
            bind_groups: HashMap::new(),
            shader_modules: HashMap::new(),
            graphics_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
            timestamp_queries: HashMap::new(),
            pipeline_statistics_queries: HashSet::new(),
            occlusion_queries: HashMap::new(),
            timestamp_results: HashMap::new(),
            pipeline_statistics_results: HashMap::new(),
            occlusion_results: HashMap::new(),
            finished_command_buffers: HashMap::new(),
            submitted_command_buffers: HashSet::new(),
            encoded_barriers: 0,
            encoded_compute_dispatches: 0,
            encoded_render_draws: 0,
            encoded_indirect_draws: 0,
            encoded_timestamp_writes: 0,
            encoded_debug_groups: 0,
            next_timestamp_ns: 1_000,
            last_poll: None,
        }
    }
}

impl HeadlessRhiState {
    fn allocate(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn stats(&self) -> HeadlessRhiStats {
        HeadlessRhiStats {
            buffers: self.buffers.len(),
            textures: self.textures.len(),
            samplers: self.samplers.len(),
            bind_groups: self.bind_groups.len(),
            shader_modules: self.shader_modules.len(),
            graphics_pipelines: self.graphics_pipelines.len(),
            compute_pipelines: self.compute_pipelines.len(),
            timestamp_queries: self.timestamp_queries.len(),
            pipeline_statistics_queries: self.pipeline_statistics_queries.len(),
            occlusion_queries: self.occlusion_queries.len(),
            uniform_buffers: count_headless_buffers_with_usage(
                &self.buffers,
                RhiBufferUsage::UNIFORM,
            ),
            storage_buffers: count_headless_buffers_with_usage(
                &self.buffers,
                RhiBufferUsage::STORAGE,
            ),
            vertex_buffers: count_headless_buffers_with_usage(
                &self.buffers,
                RhiBufferUsage::VERTEX,
            ),
            index_buffers: count_headless_buffers_with_usage(&self.buffers, RhiBufferUsage::INDEX),
            indirect_buffers: count_headless_buffers_with_usage(
                &self.buffers,
                RhiBufferUsage::INDIRECT,
            ),
            copy_src_buffers: count_headless_buffers_with_usage(
                &self.buffers,
                RhiBufferUsage::COPY_SRC,
            ),
            copy_dst_buffers: count_headless_buffers_with_usage(
                &self.buffers,
                RhiBufferUsage::COPY_DST,
            ),
            sampled_textures: count_headless_textures_with_usage(
                &self.textures,
                RhiTextureUsage::SAMPLED,
            ),
            storage_textures: count_headless_textures_with_usage(
                &self.textures,
                RhiTextureUsage::STORAGE,
            ),
            render_attachment_textures: count_headless_textures_with_usage(
                &self.textures,
                RhiTextureUsage::RENDER_ATTACHMENT,
            ),
            copy_src_textures: count_headless_textures_with_usage(
                &self.textures,
                RhiTextureUsage::COPY_SRC,
            ),
            copy_dst_textures: count_headless_textures_with_usage(
                &self.textures,
                RhiTextureUsage::COPY_DST,
            ),
            finished_command_buffers: self.finished_command_buffers.len(),
            submitted_command_buffers: self.submitted_command_buffers.len(),
            submissions: self.next_submission.saturating_sub(1) as usize,
            encoded_compute_dispatches: self.encoded_compute_dispatches,
            encoded_render_draws: self.encoded_render_draws,
            encoded_indirect_draws: self.encoded_indirect_draws,
            encoded_barriers: self.encoded_barriers,
            encoded_timestamp_writes: self.encoded_timestamp_writes,
            encoded_debug_groups: self.encoded_debug_groups,
            last_poll: self.last_poll,
        }
    }
}

fn count_headless_buffers_with_usage(
    buffers: &HashMap<RhiBuffer, HeadlessBufferState>,
    usage: RhiBufferUsage,
) -> usize {
    buffers
        .values()
        .filter(|buffer| buffer.usage.contains(usage))
        .count()
}

fn count_headless_textures_with_usage(
    textures: &HashMap<RhiTexture, HeadlessTextureState>,
    usage: RhiTextureUsage,
) -> usize {
    textures
        .values()
        .filter(|texture| texture.usage.contains(usage))
        .count()
}

fn validate_headless_color_target(
    state: &HeadlessRhiState,
    target: RhiTexture,
) -> Result<(), RendererError> {
    let Some(texture) = state.textures.get(&target) else {
        return Err(RendererError::Validation(format!(
            "unknown RHI color target texture: {}",
            target.0
        )));
    };
    if !texture.usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        return Err(RendererError::Validation(
            "RHI color target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    if texture.format == TextureFormat::Depth32Float {
        return Err(RendererError::Validation(
            "RHI color target must not use a depth format".to_owned(),
        ));
    }
    Ok(())
}

fn validate_headless_depth_target(
    state: &HeadlessRhiState,
    target: RhiTexture,
) -> Result<(), RendererError> {
    let Some(texture) = state.textures.get(&target) else {
        return Err(RendererError::Validation(format!(
            "unknown RHI depth target texture: {}",
            target.0
        )));
    };
    if !texture.usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        return Err(RendererError::Validation(
            "RHI depth target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    if texture.format != TextureFormat::Depth32Float {
        return Err(RendererError::Validation(
            "RHI depth target requires a depth format".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_color_target(
    state: &WgpuRhiState,
    target: RhiTexture,
) -> Result<&WgpuTextureState, RendererError> {
    let Some(texture) = state.textures.get(&target) else {
        return Err(RendererError::Validation(format!(
            "unknown RHI color target texture: {}",
            target.0
        )));
    };
    if !texture.usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        return Err(RendererError::Validation(
            "RHI color target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    if texture.format == TextureFormat::Depth32Float {
        return Err(RendererError::Validation(
            "RHI color target must not use a depth format".to_owned(),
        ));
    }
    Ok(texture)
}

#[cfg(feature = "backend-wgpu")]
fn validate_wgpu_depth_target(
    state: &WgpuRhiState,
    target: RhiTexture,
) -> Result<&WgpuTextureState, RendererError> {
    let Some(texture) = state.textures.get(&target) else {
        return Err(RendererError::Validation(format!(
            "unknown RHI depth target texture: {}",
            target.0
        )));
    };
    if !texture.usage.contains(RhiTextureUsage::RENDER_ATTACHMENT) {
        return Err(RendererError::Validation(
            "RHI depth target requires RENDER_ATTACHMENT usage".to_owned(),
        ));
    }
    if texture.format != TextureFormat::Depth32Float {
        return Err(RendererError::Validation(
            "RHI depth target requires a depth format".to_owned(),
        ));
    }
    Ok(texture)
}

#[cfg(feature = "backend-wgpu")]
impl WgpuRhiState {
    fn allocate(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn stats(&self) -> WgpuRhiStats {
        WgpuRhiStats {
            buffers: self.buffers.len(),
            textures: self.textures.len(),
            samplers: self.samplers.len(),
            bind_groups: self.bind_groups.len(),
            shader_modules: self.shader_modules.len(),
            graphics_pipelines: self.graphics_pipelines.len(),
            compute_pipelines: self.compute_pipelines.len(),
            timestamp_queries: self.timestamp_queries.len(),
            pipeline_statistics_queries: self.pipeline_statistics_queries.len(),
            occlusion_queries: self.occlusion_queries.len(),
            finished_command_buffers: self.finished_command_buffers.len(),
            submitted_command_buffers: self.submitted_command_buffers.len(),
            submissions: self.next_submission.saturating_sub(1) as usize,
            encoded_compute_dispatches: self.encoded_compute_dispatches,
            encoded_render_draws: self.encoded_render_draws,
            encoded_indirect_draws: self.encoded_indirect_draws,
            encoded_barriers: self.encoded_barriers,
            encoded_timestamp_writes: self.encoded_timestamp_writes,
            encoded_debug_groups: self.encoded_debug_groups,
            last_poll: self.last_poll,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhi_access_exposes_plugin_safe_command_submission() {
        let device = HeadlessRhiDevice::new();
        let access = RhiAccess::new(&device);

        assert_eq!(access.caps().backend_name, "headless");
        assert!(access.create_command_encoder(Some("")).is_err());

        let encoder = access.create_command_encoder(Some("plugin")).unwrap();
        let command = encoder.finish().unwrap();
        assert_eq!(access.submit(vec![command]).unwrap(), SubmissionIndex(1));
        access.poll(PollMode::Poll);

        let stats = device.stats();
        assert_eq!(stats.finished_command_buffers, 1);
        assert_eq!(stats.submitted_command_buffers, 1);
        assert_eq!(stats.submissions, 1);
        assert_eq!(stats.last_poll, Some(PollMode::Poll));
        assert_eq!(access.device().caps().backend_name, "headless");
    }

    #[test]
    fn headless_rhi_allocates_resources_and_records_submissions() {
        let device = HeadlessRhiDevice::new();

        let buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("vertices".to_owned()),
                size: 256,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_SRC | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(buffer, 4, &[1, 2, 3, 4, 5, 6, 7, 8])
            .unwrap();
        assert_eq!(device.read_buffer(buffer, 6, 4).unwrap(), vec![3, 4, 5, 6]);
        let depth_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("depth".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        assert_ne!(depth_texture, RhiTexture(0));
        let texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("color".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT
                    | RhiTextureUsage::COPY_SRC
                    | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        assert_eq!(device.texture_samples(texture).unwrap(), 1);
        let region = RhiTextureRegion {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        let pixels = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        device
            .write_texture_rgba8(texture, region, &pixels)
            .unwrap();
        assert_eq!(device.read_texture_rgba8(texture, region).unwrap(), pixels);
        assert_eq!(
            device
                .read_texture_rgba8(
                    texture,
                    RhiTextureRegion {
                        x: 2,
                        y: 2,
                        width: 1,
                        height: 1,
                    },
                )
                .unwrap(),
            vec![13, 14, 15, 16]
        );
        let sampler = device
            .create_sampler(&RhiSamplerDesc {
                label: Some("linear".to_owned()),
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("shader".to_owned()),
                source: "@compute @workgroup_size(1) fn main() {}".to_owned(),
            })
            .unwrap();
        let graphics = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: "main".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("main".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let compute = device
            .create_compute_pipeline(&RhiComputePipelineDesc {
                label: Some("compute".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let timestamp = device
            .create_timestamp_query(&RhiTimestampQueryDesc {
                label: Some("frame_start".to_owned()),
            })
            .unwrap();
        let pipeline_statistics = device
            .create_pipeline_statistics_query(&RhiPipelineStatisticsQueryDesc {
                label: Some("pipeline_statistics".to_owned()),
            })
            .unwrap();
        let occlusion = device
            .create_occlusion_query(&RhiOcclusionQueryDesc {
                label: Some("occlusion".to_owned()),
            })
            .unwrap();
        let indirect = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indirect".to_owned()),
                size: 32,
                usage: RhiBufferUsage::INDIRECT | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(indirect, 0, &[0_u8; RHI_DRAW_INDIRECT_BYTES as usize * 2])
            .unwrap();
        let indices = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indices".to_owned()),
                size: 8,
                usage: RhiBufferUsage::INDEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(indices, 0, &[0, 0, 1, 0, 2, 0])
            .unwrap();
        let indexed_indirect = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indexed_indirect".to_owned()),
                size: RHI_INDEXED_DRAW_INDIRECT_BYTES,
                usage: RhiBufferUsage::INDIRECT | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(
                indexed_indirect,
                0,
                &[0_u8; RHI_INDEXED_DRAW_INDIRECT_BYTES as usize],
            )
            .unwrap();

        assert_ne!(buffer.0, texture.0);
        assert_ne!(sampler.0, shader.0);
        assert_ne!(graphics.0, compute.0);

        let mut encoder = device.create_command_encoder(Some("frame")).unwrap();
        encoder.push_debug_group("frame").unwrap();
        encoder
            .begin_pipeline_statistics(pipeline_statistics)
            .unwrap();
        encoder.begin_occlusion_query(occlusion).unwrap();
        encoder
            .encode_compute_pass(&RhiComputePassDesc {
                label: Some("compute".to_owned()),
                pipeline: compute,
                bind_groups: Vec::new(),
                workgroups: [1, 1, 1],
            })
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("draw".to_owned()),
                pipeline: graphics,
                color_target: Some(texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        encoder
            .encode_indirect_render_pass(&RhiIndirectRenderPassDesc {
                label: Some("draw_indirect".to_owned()),
                pipeline: graphics,
                color_target: Some(texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                bind_groups: Vec::new(),
                indirect_buffer: indirect,
                indirect_offset: 0,
                draw_count: 2,
                draw_stride: RHI_DRAW_INDIRECT_BYTES,
            })
            .unwrap();
        encoder
            .encode_indexed_indirect_render_pass(&RhiIndexedIndirectRenderPassDesc {
                label: Some("draw_indexed_indirect".to_owned()),
                pipeline: graphics,
                color_target: Some(texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: RhiIndexBufferBinding {
                    buffer: indices,
                    offset: 0,
                    format: RhiIndexFormat::Uint16,
                },
                bind_groups: Vec::new(),
                indirect_buffer: indexed_indirect,
                indirect_offset: 0,
                draw_count: 1,
                draw_stride: RHI_INDEXED_DRAW_INDIRECT_BYTES,
            })
            .unwrap();
        encoder
            .end_pipeline_statistics(pipeline_statistics)
            .unwrap();
        encoder.end_occlusion_query(occlusion).unwrap();
        encoder.write_timestamp(timestamp).unwrap();
        encoder.pop_debug_group().unwrap();
        let command = encoder.finish().unwrap();
        assert_eq!(device.submit(vec![command]).unwrap(), SubmissionIndex(1));
        device.poll(PollMode::Wait);
        let timestamp_result = device.timestamp_result(timestamp).unwrap();
        assert!(timestamp_result.available);
        assert!(timestamp_result.timestamp_ns > 0);
        let pipeline_statistics_result = device
            .pipeline_statistics_result(pipeline_statistics)
            .unwrap();
        assert!(pipeline_statistics_result.available);
        assert_eq!(
            pipeline_statistics_result.statistics,
            RhiPipelineStatistics {
                input_assembly_vertices: 3,
                input_assembly_primitives: 1,
                vertex_shader_invocations: 3,
                clipping_invocations: 1,
                clipping_primitives: 1,
                fragment_shader_invocations: 1,
                compute_shader_invocations: 1,
                draw_calls: 4,
                dispatch_calls: 1,
            }
        );
        let occlusion_result = device.occlusion_result(occlusion).unwrap();
        assert!(occlusion_result.available);
        assert_eq!(occlusion_result.samples_passed, 4);
        assert!(occlusion_result.visible);

        assert_eq!(
            device.stats(),
            HeadlessRhiStats {
                buffers: 4,
                textures: 2,
                samplers: 1,
                bind_groups: 0,
                shader_modules: 1,
                graphics_pipelines: 1,
                compute_pipelines: 1,
                timestamp_queries: 1,
                pipeline_statistics_queries: 1,
                occlusion_queries: 1,
                uniform_buffers: 0,
                storage_buffers: 0,
                vertex_buffers: 1,
                index_buffers: 1,
                indirect_buffers: 2,
                copy_src_buffers: 1,
                copy_dst_buffers: 4,
                sampled_textures: 1,
                storage_textures: 0,
                render_attachment_textures: 2,
                copy_src_textures: 1,
                copy_dst_textures: 1,
                finished_command_buffers: 1,
                submitted_command_buffers: 1,
                submissions: 1,
                encoded_compute_dispatches: 1,
                encoded_render_draws: 1,
                encoded_indirect_draws: 3,
                encoded_barriers: 0,
                encoded_timestamp_writes: 1,
                encoded_debug_groups: 1,
                last_poll: Some(PollMode::Wait),
            }
        );
    }

    #[test]
    fn headless_rhi_texture_samples_are_queryable() {
        let device = HeadlessRhiDevice::new();
        let texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("headless_msaa_query".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();

        assert_eq!(device.texture_samples(texture).unwrap(), 4);
        assert!(device.texture_samples(RhiTexture(999)).is_err());
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_texture_samples_are_queryable() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_msaa_query".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();

        assert_eq!(device.texture_samples(texture).unwrap(), 4);
        assert!(device.texture_samples(RhiTexture(999)).is_err());
    }

    #[test]
    fn headless_rhi_graphics_pipeline_sample_count_matches_render_targets() {
        let device = HeadlessRhiDevice::new();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("headless_msaa_pipeline_shader".to_owned()),
                source: "shader".to_owned(),
            })
            .unwrap();
        let msaa_target = device
            .create_texture(&RhiTextureDesc {
                label: Some("headless_msaa_pipeline_target".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let msaa_pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("headless_msaa_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let single_sample_pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("headless_single_sample_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("headless_msaa_pipeline_match"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("headless_msaa_pipeline_match".to_owned()),
                pipeline: msaa_pipeline,
                color_target: Some(msaa_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: Some("headless_msaa_pipeline_mismatch".to_owned()),
                pipeline: single_sample_pipeline,
                color_target: Some(msaa_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn headless_rhi_resolves_rgba8_msaa_texture_explicitly() {
        let device = HeadlessRhiDevice::new();
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("headless_explicit_msaa_resolve_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("headless_explicit_msaa_resolve_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let bytes = vec![
            10_u8, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
        ];
        device
            .write_texture_rgba8(
                source,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                &bytes,
            )
            .unwrap();

        device.resolve_texture_rgba8(source, target).unwrap();

        assert_eq!(
            device
                .read_texture_rgba8(
                    target,
                    RhiTextureRegion {
                        x: 0,
                        y: 0,
                        width: 2,
                        height: 2,
                    },
                )
                .unwrap(),
            bytes
        );
    }

    #[test]
    fn headless_rhi_resolves_rgba8_msaa_texture_with_first_sample_mode() {
        let device = HeadlessRhiDevice::new();
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("headless_first_sample_resolve_source".to_owned()),
                width: 1,
                height: 1,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT
                    | RhiTextureUsage::SAMPLED
                    | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("headless_first_sample_resolve_target".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT
                    | RhiTextureUsage::STORAGE
                    | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let bytes = vec![12_u8, 34, 56, 255];
        device
            .write_texture_rgba8(
                source,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &bytes,
            )
            .unwrap();

        device
            .resolve_texture_rgba8_with_mode(source, target, RhiResolveMode::FirstSample)
            .unwrap();

        assert_eq!(
            device
                .read_texture_rgba8(
                    target,
                    RhiTextureRegion {
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                    },
                )
                .unwrap(),
            bytes
        );
        device
            .resolve_texture_rgba8_with_mode(source, target, RhiResolveMode::Sample(2))
            .unwrap();
        assert!(matches!(
            device.resolve_texture_rgba8_with_mode(source, target, RhiResolveMode::Sample(4)),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn rhi_custom_resolve_support_reports_supported_paths() {
        let headless = RhiCustomResolveSupport::headless();
        assert!(!headless.supports(RhiCustomResolvePath::Rgba8StorageCompute));
        assert!(!headless.supports(RhiCustomResolvePath::Rgba16FloatStorageCompute));
        assert!(!headless.supports(RhiCustomResolvePath::Rgba32FloatStorageCompute));
        assert!(!headless.supports(RhiCustomResolvePath::EightBitColorFragment));
        assert!(!headless.supports(RhiCustomResolvePath::Depth32FloatFragment));
        assert!(headless
            .support_for(RhiCustomResolvePath::Depth32FloatFragment)
            .and_then(|support| support.unsupported_reason.as_deref())
            .is_some());

        let wgpu = RhiCustomResolveSupport::backend_wgpu();
        assert!(wgpu.supports(RhiCustomResolvePath::Rgba8StorageCompute));
        assert!(wgpu.supports(RhiCustomResolvePath::Rgba16FloatStorageCompute));
        assert!(wgpu.supports(RhiCustomResolvePath::Rgba32FloatStorageCompute));
        assert!(wgpu.supports(RhiCustomResolvePath::EightBitColorFragment));
        assert!(wgpu.supports(RhiCustomResolvePath::Depth32FloatFragment));
    }

    #[test]
    fn headless_rhi_rejects_custom_resolve_shader() {
        let device = HeadlessRhiDevice::new();
        let err = device
            .resolve_texture_rgba8_with_shader(
                RhiTexture(1),
                RhiTexture(2),
                &RhiResolveShaderDesc {
                    label: Some("headless_custom_resolve".to_owned()),
                    source: "@compute @workgroup_size(1) fn main() {}".to_owned(),
                    entry_point: "main".to_owned(),
                },
            )
            .unwrap_err();
        assert!(matches!(
            err,
            RendererError::UnsupportedFeature(RendererFeature::BackendWgpu)
        ));
        let err = device
            .resolve_texture_8bit_color_with_shader(
                RhiTexture(1),
                RhiTexture(2),
                &RhiResolveShaderDesc {
                    label: Some("headless_8bit_custom_resolve".to_owned()),
                    source:
                        "@fragment fn main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }"
                            .to_owned(),
                    entry_point: "main".to_owned(),
                },
            )
            .unwrap_err();
        assert!(matches!(
            err,
            RendererError::UnsupportedFeature(RendererFeature::BackendWgpu)
        ));
        let err = device
            .resolve_texture_depth32f_with_shader(
                RhiTexture(1),
                RhiTexture(2),
                &RhiResolveShaderDesc {
                    label: Some("headless_depth_custom_resolve".to_owned()),
                    source: "@fragment fn main() -> @builtin(frag_depth) f32 { return 1.0; }"
                        .to_owned(),
                    entry_point: "main".to_owned(),
                },
            )
            .unwrap_err();
        assert!(matches!(
            err,
            RendererError::UnsupportedFeature(RendererFeature::BackendWgpu)
        ));
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_graphics_pipeline_sample_count_matches_msaa_target() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_msaa_pipeline_target".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_msaa_pipeline_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_msaa_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_msaa_pipeline_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_msaa_pipeline_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_rgba8_msaa_texture_explicitly() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_explicit_msaa_resolve_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_explicit_msaa_resolve_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_explicit_msaa_resolve_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_explicit_msaa_resolve_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_explicit_msaa_source_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_explicit_msaa_source_draw".to_owned()),
                pipeline,
                color_target: Some(source),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();

        device.resolve_texture_rgba8(source, target).unwrap();

        let resolved = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![255, 0, 0, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_rgba8_msaa_texture_with_first_sample_mode() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_first_sample_resolve_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_first_sample_resolve_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT
                    | RhiTextureUsage::STORAGE
                    | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_first_sample_resolve_draw_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_first_sample_resolve_draw_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_first_sample_resolve_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_first_sample_resolve_draw".to_owned()),
                pipeline,
                color_target: Some(source),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();

        device
            .resolve_texture_rgba8_with_mode(source, target, RhiResolveMode::FirstSample)
            .unwrap();

        let resolved = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![0, 255, 0, 255]);
        device
            .resolve_texture_rgba8_with_mode(source, target, RhiResolveMode::Sample(2))
            .unwrap();
        assert!(matches!(
            device.resolve_texture_rgba8_with_mode(source, target, RhiResolveMode::Sample(4)),
            Err(RendererError::Validation(_))
        ));
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_rgba8_msaa_texture_with_custom_shader() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT
                    | RhiTextureUsage::STORAGE
                    | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_custom_resolve_draw_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_custom_resolve_draw_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_custom_resolve_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_custom_resolve_draw".to_owned()),
                pipeline,
                color_target: Some(source),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device
            .resolve_texture_rgba8_with_shader(
                source,
                target,
                &RhiResolveShaderDesc {
                    label: Some("wgpu_custom_resolve_shader".to_owned()),
                    entry_point: "main".to_owned(),
                    source: r#"
                        @group(0) @binding(0)
                        var source_tex: texture_multisampled_2d<f32>;

                        @group(0) @binding(1)
                        var target_tex: texture_storage_2d<rgba8unorm, write>;

                        @compute @workgroup_size(8, 8)
                        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                            let dims = textureDimensions(target_tex);
                            if (id.x >= dims.x || id.y >= dims.y) {
                                return;
                            }
                            let value = textureLoad(source_tex, vec2<i32>(i32(id.x), i32(id.y)), 0);
                            textureStore(
                                target_tex,
                                vec2<i32>(i32(id.x), i32(id.y)),
                                vec4<f32>(0.0, value.r, 1.0, value.a)
                            );
                        }
                    "#
                    .to_owned(),
                },
            )
            .unwrap();

        let resolved = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![0, 255, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_rgba16f_msaa_texture_with_custom_shader() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_rgba16f_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba16Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_rgba16f_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: RhiTextureUsage::STORAGE | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_custom_resolve_rgba16f_draw_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_custom_resolve_rgba16f_draw_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba16Float),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_custom_resolve_rgba16f_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_custom_resolve_rgba16f_draw".to_owned()),
                pipeline,
                color_target: Some(source),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device
            .resolve_texture_rgba16f_with_shader(
                source,
                target,
                &RhiResolveShaderDesc {
                    label: Some("wgpu_custom_resolve_rgba16f_shader".to_owned()),
                    entry_point: "main".to_owned(),
                    source: r#"
                        @group(0) @binding(0)
                        var source_tex: texture_multisampled_2d<f32>;

                        @group(0) @binding(1)
                        var target_tex: texture_storage_2d<rgba16float, write>;

                        @compute @workgroup_size(8, 8)
                        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                            let dims = textureDimensions(target_tex);
                            if (id.x >= dims.x || id.y >= dims.y) {
                                return;
                            }
                            let value = textureLoad(source_tex, vec2<i32>(i32(id.x), i32(id.y)), 0);
                            textureStore(
                                target_tex,
                                vec2<i32>(i32(id.x), i32(id.y)),
                                vec4<f32>(0.0, value.r, 1.0, value.a)
                            );
                        }
                    "#
                    .to_owned(),
                },
            )
            .unwrap();

        let resolved = device
            .read_texture_rgba16f(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![0x0000, 0x3c00, 0x3c00, 0x3c00]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_rgba32f_msaa_texture_with_custom_shader() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let rgba32f_features = wgpu::TextureFormat::Rgba32Float
            .guaranteed_format_features(graphics.device().features());
        if !rgba32f_features.flags.sample_count_supported(4) {
            return;
        }
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_rgba32f_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_rgba32f_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba32Float,
                usage: RhiTextureUsage::STORAGE | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        device
            .resolve_texture_rgba32f_with_shader(
                source,
                target,
                &RhiResolveShaderDesc {
                    label: Some("wgpu_custom_resolve_rgba32f_shader".to_owned()),
                    entry_point: "main".to_owned(),
                    source: r#"
                        @group(0) @binding(0)
                        var source_tex: texture_multisampled_2d<f32>;

                        @group(0) @binding(1)
                        var target_tex: texture_storage_2d<rgba32float, write>;

                        @compute @workgroup_size(8, 8)
                        fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                            let dims = textureDimensions(target_tex);
                            if (id.x >= dims.x || id.y >= dims.y) {
                                return;
                            }
                            textureStore(
                                target_tex,
                                vec2<i32>(i32(id.x), i32(id.y)),
                                vec4<f32>(0.25, 0.5, 0.75, 1.0)
                            );
                        }
                    "#
                    .to_owned(),
                },
            )
            .unwrap();

        let resolved = device
            .read_texture_rgba32f(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![0.25, 0.5, 0.75, 1.0]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_rgba8_srgb_msaa_texture_with_custom_fragment_shader() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let features = wgpu::TextureFormat::Rgba8UnormSrgb
            .guaranteed_format_features(graphics.device().features());
        if !features.flags.sample_count_supported(4) {
            return;
        }
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_rgba8_srgb_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_rgba8_srgb_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let draw_shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_custom_resolve_rgba8_srgb_draw_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let draw_pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_custom_resolve_rgba8_srgb_draw_pipeline".to_owned()),
                vertex_shader: draw_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(draw_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8UnormSrgb),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_custom_resolve_rgba8_srgb_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_custom_resolve_rgba8_srgb_draw".to_owned()),
                pipeline: draw_pipeline,
                color_target: Some(source),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();

        device
            .resolve_texture_8bit_color_with_shader(
                source,
                target,
                &RhiResolveShaderDesc {
                    label: Some("wgpu_custom_resolve_rgba8_srgb_shader".to_owned()),
                    entry_point: "main".to_owned(),
                    source: r#"
                        @group(0) @binding(0)
                        var source_tex: texture_multisampled_2d<f32>;

                        @fragment
                        fn main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
                            let value = textureLoad(
                                source_tex,
                                vec2<i32>(i32(position.x), i32(position.y)),
                                0
                            );
                            return vec4<f32>(value.r, value.g, 1.0, value.a);
                        }
                    "#
                    .to_owned(),
                },
            )
            .unwrap();

        let resolved = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![0, 255, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_bgra8_srgb_msaa_texture_with_custom_fragment_shader() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let features = wgpu::TextureFormat::Bgra8UnormSrgb
            .guaranteed_format_features(graphics.device().features());
        if !features.flags.sample_count_supported(4) {
            return;
        }
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_bgra8_srgb_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_bgra8_srgb_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let draw_shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_custom_resolve_bgra8_srgb_draw_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let draw_pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_custom_resolve_bgra8_srgb_draw_pipeline".to_owned()),
                vertex_shader: draw_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(draw_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Bgra8UnormSrgb),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_custom_resolve_bgra8_srgb_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_custom_resolve_bgra8_srgb_draw".to_owned()),
                pipeline: draw_pipeline,
                color_target: Some(source),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();

        device
            .resolve_texture_8bit_color_with_shader(
                source,
                target,
                &RhiResolveShaderDesc {
                    label: Some("wgpu_custom_resolve_bgra8_srgb_shader".to_owned()),
                    entry_point: "main".to_owned(),
                    source: r#"
                        @group(0) @binding(0)
                        var source_tex: texture_multisampled_2d<f32>;

                        @fragment
                        fn main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
                            let value = textureLoad(
                                source_tex,
                                vec2<i32>(i32(position.x), i32(position.y)),
                                0
                            );
                            return vec4<f32>(value.g, value.g, value.g, value.a);
                        }
                    "#
                    .to_owned(),
                },
            )
            .unwrap();

        let resolved = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(resolved, vec![255, 255, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_resolves_depth32f_msaa_texture_with_custom_shader() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let depth_features = wgpu::TextureFormat::Depth32Float
            .guaranteed_format_features(graphics.device().features());
        if !depth_features.flags.sample_count_supported(4) {
            return;
        }
        let device = WgpuRhiDevice::new(&graphics);
        let source = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_depth_source".to_owned()),
                width: 2,
                height: 2,
                samples: 4,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("wgpu_custom_resolve_depth_target".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let draw_shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("wgpu_custom_resolve_depth_draw_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        let x = f32((vertex_index << 1u) & 2u);
                        let y = f32(vertex_index & 2u);
                        return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.25, 1.0);
                    }

                    @fragment
                    fn fs() -> @builtin(frag_depth) f32 {
                        return 0.25;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let draw_pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("wgpu_custom_resolve_depth_draw_pipeline".to_owned()),
                vertex_shader: draw_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(draw_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: None,
                depth_format: Some(DepthFormat::D32Float),
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: Some(RhiDepthState {
                    format: DepthFormat::D32Float,
                    write_enabled: true,
                    compare: RhiCompareFunction::Always,
                }),
                sample_count: 4,
            })
            .unwrap();
        let mut encoder = device
            .create_command_encoder(Some("wgpu_custom_resolve_depth_draw"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("wgpu_custom_resolve_depth_draw".to_owned()),
                pipeline: draw_pipeline,
                color_target: None,
                depth_target: Some(source),
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();

        device
            .resolve_texture_depth32f_with_shader(
                source,
                target,
                &RhiResolveShaderDesc {
                    label: Some("wgpu_custom_resolve_depth_shader".to_owned()),
                    entry_point: "main".to_owned(),
                    source: r#"
                        @group(0) @binding(0)
                        var source_tex: texture_depth_multisampled_2d;

                        @fragment
                        fn main(@builtin(position) position: vec4<f32>) -> @builtin(frag_depth) f32 {
                            let depth = textureLoad(
                                source_tex,
                                vec2<i32>(i32(position.x), i32(position.y)),
                                0
                            );
                            return depth + 0.125;
                        }
                    "#
                    .to_owned(),
                },
            )
            .unwrap();

        let resolved = device
            .read_texture_depth32f(
                target,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert!((resolved[0] - 0.375).abs() < 0.001);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_compute_pass_binds_storage_buffer() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("compute_storage".to_owned()),
                size: std::mem::size_of::<u32>() as u64,
                usage: RhiBufferUsage::STORAGE | RhiBufferUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("compute_storage_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var<storage, read_write> output: array<u32>;

                    @compute @workgroup_size(1)
                    fn main() {
                        output[0] = 42u;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(&RhiComputePipelineDesc {
                label: Some("compute_storage_pipeline".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let bind_group = device
            .create_compute_bind_group(&RhiComputeBindGroupDesc {
                label: Some("compute_storage_bind_group".to_owned()),
                pipeline,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer { binding: 0, buffer }],
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("compute_storage"))
            .unwrap();
        encoder
            .encode_compute_pass(&RhiComputePassDesc {
                label: Some("compute_storage".to_owned()),
                pipeline,
                bind_groups: vec![RhiComputePassBindGroup {
                    index: 0,
                    bind_group,
                }],
                workgroups: [1, 1, 1],
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let bytes = device.read_buffer(buffer, 0, 4).unwrap();
        assert_eq!(bytes, 42_u32.to_le_bytes());
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_occlusion_query_resolves_readback() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let depth = device
            .create_texture(&RhiTextureDesc {
                label: Some("occlusion_depth".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let color = device
            .create_texture(&RhiTextureDesc {
                label: Some("occlusion_color".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("occlusion_vs".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
                        var pos = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(pos[index], 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("occlusion_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: Some(DepthFormat::D32Float),
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: Some(RhiDepthState {
                    format: DepthFormat::D32Float,
                    write_enabled: false,
                    compare: RhiCompareFunction::Always,
                }),
                sample_count: 1,
            })
            .unwrap();
        let query = device
            .create_occlusion_query(&RhiOcclusionQueryDesc {
                label: Some("occlusion".to_owned()),
            })
            .unwrap();

        let mut encoder = device.create_command_encoder(Some("occlusion")).unwrap();
        encoder.begin_occlusion_query(query).unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("occlusion_draw".to_owned()),
                pipeline,
                color_target: Some(color),
                depth_target: Some(depth),
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        encoder.end_occlusion_query(query).unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let result = device.occlusion_result(query).unwrap();
        assert!(result.available);
        assert!(result.visible);
        assert!(result.samples_passed > 0);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_picking_id_shader_writes_readable_object_id() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("picking_target".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let object: crate::ObjectHandle =
            crate::make_handle(crate::ResourceKind::Object, 0x0003_0201, 1);
        let encoded = crate::encode_gpu_picking_object_index(object);
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("picking_id_shader".to_owned()),
                source: format!(
                    r#"
                    @vertex
                    fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {{
                        var pos = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(pos[index], 0.0, 1.0);
                    }}

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {{
                        return vec4<f32>(
                            {r} / 255.0,
                            {g} / 255.0,
                            {b} / 255.0,
                            {a} / 255.0
                        );
                    }}
                    "#,
                    r = encoded[0],
                    g = encoded[1],
                    b = encoded[2],
                    a = encoded[3],
                ),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("picking_id_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device.create_command_encoder(Some("picking_id")).unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("picking_id_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, encoded);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_render_pass_binds_sampled_texture_and_sampler() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let sampled = device
            .create_texture(&RhiTextureDesc {
                label: Some("sampled_red".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::SAMPLED | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_texture_rgba8(
                sampled,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                &[
                    255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
                ],
            )
            .unwrap();
        let sampler = device
            .create_sampler(&RhiSamplerDesc {
                label: Some("sampled_red_sampler".to_owned()),
            })
            .unwrap();
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("sampled_target".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("texture_sample_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0) var sampled_tex: texture_2d<f32>;
                    @group(0) @binding(1) var sampled_sampler: sampler;

                    @vertex
                    fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
                        var pos = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(pos[index], 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return textureSample(sampled_tex, sampled_sampler, vec2<f32>(0.5, 0.5));
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("texture_sample_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let bind_group = device
            .create_bind_group(&RhiBindGroupDesc {
                label: Some("sampled_red_bind_group".to_owned()),
                pipeline,
                group_index: 0,
                entries: vec![
                    RhiBindGroupEntry::Texture {
                        binding: 0,
                        texture: sampled,
                    },
                    RhiBindGroupEntry::Sampler {
                        binding: 1,
                        sampler,
                    },
                ],
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("texture_sample"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("texture_sample_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: vec![RhiRenderPassBindGroup {
                    index: 0,
                    bind_group,
                }],
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, vec![255, 0, 0, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_render_pass_binds_vertex_buffers() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("vertex_buffer_target".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let vertices = [
            [-1.0_f32, -1.0_f32],
            [3.0_f32, -1.0_f32],
            [-1.0_f32, 3.0_f32],
        ];
        let vertex_bytes = vertices
            .iter()
            .flat_map(|vertex| vertex.iter().flat_map(|value| value.to_le_bytes()))
            .collect::<Vec<_>>();
        let vertex_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("positions".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(vertex_buffer, 0, &vertex_bytes)
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("vertex_buffer_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(position, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(0.0, 1.0, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("vertex_buffer_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![RhiVertexAttribute {
                        location: 0,
                        format: VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("vertex_buffer"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("vertex_buffer_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: vec![RhiVertexBufferBinding {
                    slot: 0,
                    buffer: vertex_buffer,
                    offset: 0,
                }],
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, vec![0, 255, 0, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_render_pass_draws_instanced_vertex_buffers() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("instanced_target".to_owned()),
                width: 8,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let vertices = [
            [-0.9_f32, -0.8_f32],
            [-0.1_f32, -0.8_f32],
            [-0.1_f32, 0.8_f32],
            [-0.9_f32, -0.8_f32],
            [-0.1_f32, 0.8_f32],
            [-0.9_f32, 0.8_f32],
        ];
        let vertex_bytes = vertices
            .iter()
            .flat_map(|vertex| vertex.iter().flat_map(|value| value.to_le_bytes()))
            .collect::<Vec<_>>();
        let vertex_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("instanced_positions".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(vertex_buffer, 0, &vertex_bytes)
            .unwrap();
        let instances = [
            [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0],
            [1.0_f32, 0.0, 0.0, 0.0, 1.0, 1.0],
        ];
        let instance_bytes = instances
            .iter()
            .flat_map(|instance| instance.iter().flat_map(|value| value.to_le_bytes()))
            .collect::<Vec<_>>();
        let instance_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("instanced_offsets_and_colors".to_owned()),
                size: instance_bytes.len() as u64,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(instance_buffer, 0, &instance_bytes)
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("instanced_shader".to_owned()),
                source: r#"
                    struct VsOut {
                        @builtin(position) position: vec4<f32>,
                        @location(0) color: vec4<f32>,
                    };

                    @vertex
                    fn vs(
                        @location(0) position: vec2<f32>,
                        @location(1) offset: vec2<f32>,
                        @location(2) color: vec4<f32>
                    ) -> VsOut {
                        var out: VsOut;
                        out.position = vec4<f32>(position + offset, 0.0, 1.0);
                        out.color = color;
                        return out;
                    }

                    @fragment
                    fn fs(input: VsOut) -> @location(0) vec4<f32> {
                        return input.color;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("instanced_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![
                    RhiVertexBufferLayout {
                        stride: 8,
                        step_mode: VertexStepMode::Vertex,
                        attributes: vec![RhiVertexAttribute {
                            location: 0,
                            format: VertexFormat::Float32x2,
                            offset: 0,
                        }],
                    },
                    RhiVertexBufferLayout {
                        stride: 24,
                        step_mode: VertexStepMode::Instance,
                        attributes: vec![
                            RhiVertexAttribute {
                                location: 1,
                                format: VertexFormat::Float32x2,
                                offset: 0,
                            },
                            RhiVertexAttribute {
                                location: 2,
                                format: VertexFormat::Float32x4,
                                offset: 8,
                            },
                        ],
                    },
                ],
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device.create_command_encoder(Some("instanced")).unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("instanced_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: vec![
                    RhiVertexBufferBinding {
                        slot: 0,
                        buffer: vertex_buffer,
                        offset: 0,
                    },
                    RhiVertexBufferBinding {
                        slot: 1,
                        buffer: instance_buffer,
                        offset: 0,
                    },
                ],
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 6,
                index_count: None,
                instance_count: 2,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let left_pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        let right_pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 5,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(left_pixel, vec![255, 0, 0, 255]);
        assert_eq!(right_pixel, vec![0, 0, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_render_pass_draws_indexed_geometry() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("indexed_target".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let vertices = [
            [-1.0_f32, -1.0_f32],
            [3.0_f32, -1.0_f32],
            [-1.0_f32, 3.0_f32],
        ];
        let vertex_bytes = vertices
            .iter()
            .flat_map(|vertex| vertex.iter().flat_map(|value| value.to_le_bytes()))
            .collect::<Vec<_>>();
        let vertex_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indexed_positions".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(vertex_buffer, 0, &vertex_bytes)
            .unwrap();
        let indices = [0_u16, 1, 2];
        let mut index_bytes = indices
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect::<Vec<_>>();
        index_bytes.resize(8, 0);
        let index_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indices".to_owned()),
                size: index_bytes.len() as u64,
                usage: RhiBufferUsage::INDEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(index_buffer, 0, &index_bytes).unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("indexed_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(position, 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(0.0, 0.0, 1.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("indexed_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![RhiVertexAttribute {
                        location: 0,
                        format: VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device.create_command_encoder(Some("indexed")).unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("indexed_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: vec![RhiVertexBufferBinding {
                    slot: 0,
                    buffer: vertex_buffer,
                    offset: 0,
                }],
                index_buffer: Some(RhiIndexBufferBinding {
                    buffer: index_buffer,
                    offset: 0,
                    format: RhiIndexFormat::Uint16,
                }),
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: Some(3),
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, vec![0, 0, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_indexed_indirect_pass_draws_indexed_geometry() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("indexed_indirect_target".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let indices = [0_u16, 1, 2];
        let mut index_bytes = indices
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect::<Vec<_>>();
        index_bytes.resize(8, 0);
        let index_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indexed_indirect_indices".to_owned()),
                size: index_bytes.len() as u64,
                usage: RhiBufferUsage::INDEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(index_buffer, 0, &index_bytes).unwrap();
        let vertices = [
            [-1.0_f32, -1.0_f32],
            [3.0_f32, -1.0_f32],
            [-1.0_f32, 3.0_f32],
        ];
        let vertex_bytes = vertices
            .iter()
            .flat_map(|vertex| vertex.iter().flat_map(|value| value.to_le_bytes()))
            .collect::<Vec<_>>();
        let vertex_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indexed_indirect_positions".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(vertex_buffer, 0, &vertex_bytes)
            .unwrap();
        let indirect_args = [0_u32, 1, 0, 0, 0, 3, 1, 0, 0, 0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let indirect_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indexed_indirect_args".to_owned()),
                size: indirect_args.len() as u64,
                usage: RhiBufferUsage::INDIRECT | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(indirect_buffer, 0, &indirect_args)
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("indexed_indirect_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(position, 0.0, 1.0);
                    }

                    @group(0) @binding(0)
                    var<uniform> color: vec4<f32>;

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return color;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("indexed_indirect_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![RhiVertexAttribute {
                        location: 0,
                        format: VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let color_bytes = [1.0_f32, 0.0, 1.0, 1.0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let color_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("indexed_indirect_color".to_owned()),
                size: color_bytes.len() as u64,
                usage: RhiBufferUsage::UNIFORM | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(color_buffer, 0, &color_bytes).unwrap();
        let bind_group = device
            .create_bind_group(&RhiBindGroupDesc {
                label: Some("indexed_indirect_bind_group".to_owned()),
                pipeline,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer {
                    binding: 0,
                    buffer: color_buffer,
                }],
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("indexed_indirect"))
            .unwrap();
        encoder
            .encode_indexed_indirect_render_pass(&RhiIndexedIndirectRenderPassDesc {
                label: Some("indexed_indirect_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: vec![RhiVertexBufferBinding {
                    slot: 0,
                    buffer: vertex_buffer,
                    offset: 0,
                }],
                index_buffer: RhiIndexBufferBinding {
                    buffer: index_buffer,
                    offset: 0,
                    format: RhiIndexFormat::Uint16,
                },
                bind_groups: vec![RhiRenderPassBindGroup {
                    index: 0,
                    bind_group,
                }],
                indirect_buffer,
                indirect_offset: 0,
                draw_count: 2,
                draw_stride: RHI_INDEXED_DRAW_INDIRECT_BYTES,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, vec![255, 0, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_indirect_pass_draws_multiple_commands() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("multi_indirect_target".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let vertices = [
            [-1.0_f32, -1.0_f32],
            [3.0_f32, -1.0_f32],
            [-1.0_f32, 3.0_f32],
        ];
        let vertex_bytes = vertices
            .iter()
            .flat_map(|vertex| vertex.iter().flat_map(|value| value.to_le_bytes()))
            .collect::<Vec<_>>();
        let vertex_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("multi_indirect_positions".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: RhiBufferUsage::VERTEX | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(vertex_buffer, 0, &vertex_bytes)
            .unwrap();
        let indirect_args = [0_u32, 1, 0, 0, 3, 1, 0, 0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let indirect_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("multi_indirect_args".to_owned()),
                size: indirect_args.len() as u64,
                usage: RhiBufferUsage::INDIRECT | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device
            .write_buffer(indirect_buffer, 0, &indirect_args)
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("multi_indirect_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
                        return vec4<f32>(position, 0.0, 1.0);
                    }

                    @group(0) @binding(0)
                    var<uniform> color: vec4<f32>;

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return color;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("multi_indirect_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![RhiVertexAttribute {
                        location: 0,
                        format: VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let color_bytes = [0.25_f32, 0.75, 1.0, 1.0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let color_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("multi_indirect_color".to_owned()),
                size: color_bytes.len() as u64,
                usage: RhiBufferUsage::UNIFORM | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(color_buffer, 0, &color_bytes).unwrap();
        let bind_group = device
            .create_bind_group(&RhiBindGroupDesc {
                label: Some("multi_indirect_bind_group".to_owned()),
                pipeline,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer {
                    binding: 0,
                    buffer: color_buffer,
                }],
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("multi_indirect"))
            .unwrap();
        encoder
            .encode_indirect_render_pass(&RhiIndirectRenderPassDesc {
                label: Some("multi_indirect_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: vec![RhiVertexBufferBinding {
                    slot: 0,
                    buffer: vertex_buffer,
                    offset: 0,
                }],
                bind_groups: vec![RhiRenderPassBindGroup {
                    index: 0,
                    bind_group,
                }],
                indirect_buffer,
                indirect_offset: 0,
                draw_count: 2,
                draw_stride: RHI_DRAW_INDIRECT_BYTES,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba8(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, vec![64, 191, 255, 255]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_writes_and_reads_rgba16f_texture() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("rgba16f_upload".to_owned()),
                width: 3,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            })
            .unwrap();

        let pixels = vec![
            0x3c00, 0x3800, 0x0000, 0xbc00, 0x4000, 0x4200, 0x4400, 0x4600,
        ];
        device
            .write_texture_rgba16f(
                texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                &pixels,
            )
            .unwrap();

        let uploaded = device
            .read_texture_rgba16f(
                texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(uploaded, pixels);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_writes_and_reads_rgba32f_texture() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("rgba32f_upload".to_owned()),
                width: 3,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba32Float,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            })
            .unwrap();

        let pixels = vec![1.0, 0.5, -2.0, 4.0, 8.0, 16.0, 32.0, 64.0];
        device
            .write_texture_rgba32f(
                texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                &pixels,
            )
            .unwrap();

        let uploaded = device
            .read_texture_rgba32f(
                texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(uploaded, pixels);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_motion_vector_shader_writes_readable_rgba16f_target() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let target = device
            .create_texture(&RhiTextureDesc {
                label: Some("motion_vectors".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("motion_vector_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
                        var pos = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(pos[index], 0.0, 1.0);
                    }

                    @fragment
                    fn fs() -> @location(0) vec4<f32> {
                        return vec4<f32>(0.25, -0.5, 0.0, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("motion_vector_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba16Float),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device
            .create_command_encoder(Some("motion_vectors"))
            .unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("motion_vector_draw".to_owned()),
                pipeline,
                color_target: Some(target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let pixel = device
            .read_texture_rgba16f(
                target,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(pixel, vec![0x3400, 0xb800, 0x0000, 0x3c00]);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_depth_pass_writes_readable_depth32f_target() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let depth = device
            .create_texture(&RhiTextureDesc {
                label: Some("readable_depth".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT | RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("depth_only_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
                        var pos = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(pos[index], 0.5, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("depth_only_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: None,
                fragment_entry: None,
                color_format: None,
                depth_format: Some(DepthFormat::D32Float),
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: Some(RhiDepthState {
                    format: DepthFormat::D32Float,
                    write_enabled: true,
                    compare: RhiCompareFunction::LessEqual,
                }),
                sample_count: 1,
            })
            .unwrap();

        let mut encoder = device.create_command_encoder(Some("depth_only")).unwrap();
        encoder
            .encode_render_pass(&RhiRenderPassDesc {
                label: Some("depth_only_draw".to_owned()),
                pipeline,
                color_target: None,
                depth_target: Some(depth),
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            })
            .unwrap();
        let command = encoder.finish().unwrap();
        device.submit(vec![command]).unwrap();
        device.poll(PollMode::Wait);

        let depth_values = device
            .read_texture_depth32f(
                depth,
                RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(depth_values.len(), 1);
        assert!((depth_values[0] - 0.5).abs() < 0.001);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn wgpu_rhi_write_texture_depth32f_writes_readable_region() {
        let _wgpu_guard = crate::wgpu_test_serial_guard();
        let Ok(graphics) = WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default()) else {
            return;
        };
        let device = WgpuRhiDevice::new(&graphics);
        let depth = device
            .create_texture(&RhiTextureDesc {
                label: Some("direct_depth_write".to_owned()),
                width: 4,
                height: 4,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        let region = RhiTextureRegion {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        let values = vec![0.125, 0.25, 0.5, 0.75];

        device
            .write_texture_depth32f(depth, region, &values)
            .unwrap();

        let readback = device.read_texture_depth32f(depth, region).unwrap();
        assert_eq!(readback, values);
    }

    #[test]
    fn headless_rhi_rejects_invalid_descriptors() {
        let device = HeadlessRhiDevice::new();

        assert!(matches!(
            device.create_buffer(&RhiBufferDesc {
                label: None,
                size: 0,
                usage: RhiBufferUsage::COPY_DST,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_buffer(&RhiBufferDesc {
                label: Some("empty_usage".to_owned()),
                size: 4,
                usage: RhiBufferUsage::empty(),
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_buffer(RhiBuffer(999), 0, &[1]),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.read_buffer(RhiBuffer(999), 0, 1),
            Err(RendererError::Validation(_))
        ));
        let tiny = device
            .create_buffer(&RhiBufferDesc {
                label: Some("tiny".to_owned()),
                size: 4,
                usage: RhiBufferUsage::COPY_SRC | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        assert!(matches!(
            device.write_buffer(tiny, 2, &[1, 2, 3]),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.read_buffer(tiny, 2, 3),
            Err(RendererError::Validation(_))
        ));
        let copy_src_only = device
            .create_buffer(&RhiBufferDesc {
                label: Some("copy_src_only".to_owned()),
                size: 4,
                usage: RhiBufferUsage::COPY_SRC,
            })
            .unwrap();
        assert!(matches!(
            device.write_buffer(copy_src_only, 0, &[1]),
            Err(RendererError::Validation(_))
        ));
        let copy_dst_only = device
            .create_buffer(&RhiBufferDesc {
                label: Some("copy_dst_only".to_owned()),
                size: 4,
                usage: RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        assert!(matches!(
            device.read_buffer(copy_dst_only, 0, 1),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_texture(&RhiTextureDesc {
                label: None,
                width: 1,
                height: 0,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_texture(&RhiTextureDesc {
                label: Some("empty_usage".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::empty(),
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_texture(&RhiTextureDesc {
                label: Some("srgb_storage".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: RhiTextureUsage::STORAGE,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_texture(&RhiTextureDesc {
                label: Some("depth_storage".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::STORAGE,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba8(
                RhiTexture(999),
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba16f(
                RhiTexture(999),
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba32f(
                RhiTexture(999),
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0.0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        let sampled_only_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("sampled_only".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::SAMPLED,
            })
            .unwrap();
        assert!(matches!(
            device.write_texture_rgba8(
                sampled_only_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba16f(
                sampled_only_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba32f(
                sampled_only_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0.0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.read_texture_rgba8(
                sampled_only_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            ),
            Err(RendererError::Validation(_))
        ));
        let small_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("small".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        assert!(matches!(
            device.write_texture_rgba8(
                small_texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                &[0; 8],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba8(
                small_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 3],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba16f(
                small_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba32f(
                small_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0.0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.read_texture_depth32f(
                small_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            ),
            Err(RendererError::Validation(_))
        ));
        let rgba16f_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("rgba16f_upload".to_owned()),
                width: 3,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        let rgba16f_pixels = vec![
            0x3c00, 0x3800, 0x0000, 0xbc00, 0x4000, 0x4200, 0x4400, 0x4600,
        ];
        device
            .write_texture_rgba16f(
                rgba16f_texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                &rgba16f_pixels,
            )
            .unwrap();
        assert_eq!(
            device
                .read_texture_rgba16f(
                    rgba16f_texture,
                    RhiTextureRegion {
                        x: 1,
                        y: 1,
                        width: 2,
                        height: 1,
                    },
                )
                .unwrap(),
            rgba16f_pixels
        );
        assert!(matches!(
            device.write_texture_rgba16f(
                rgba16f_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 3],
            ),
            Err(RendererError::Validation(_))
        ));
        let rgba16f_without_copy_dst = device
            .create_texture(&RhiTextureDesc {
                label: Some("rgba16f_without_copy_dst".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        assert!(matches!(
            device.write_texture_rgba16f(
                rgba16f_without_copy_dst,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        let rgba32f_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("rgba32f_upload".to_owned()),
                width: 3,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba32Float,
                usage: RhiTextureUsage::COPY_SRC | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        let rgba32f_pixels = vec![1.0, 0.5, -2.0, 4.0, 8.0, 16.0, 32.0, 64.0];
        device
            .write_texture_rgba32f(
                rgba32f_texture,
                RhiTextureRegion {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 1,
                },
                &rgba32f_pixels,
            )
            .unwrap();
        assert_eq!(
            device
                .read_texture_rgba32f(
                    rgba32f_texture,
                    RhiTextureRegion {
                        x: 1,
                        y: 1,
                        width: 2,
                        height: 1,
                    },
                )
                .unwrap(),
            rgba32f_pixels
        );
        assert!(matches!(
            device.write_texture_rgba32f(
                rgba32f_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0.0; 3],
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.write_texture_rgba32f(
                rgba32f_texture,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[f32::NAN, 0.0, 0.0, 1.0],
            ),
            Err(RendererError::Validation(_))
        ));
        let rgba32f_without_copy_dst = device
            .create_texture(&RhiTextureDesc {
                label: Some("rgba32f_without_copy_dst".to_owned()),
                width: 1,
                height: 1,
                samples: 1,
                format: TextureFormat::Rgba32Float,
                usage: RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        assert!(matches!(
            device.write_texture_rgba32f(
                rgba32f_without_copy_dst,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                &[0.0; 4],
            ),
            Err(RendererError::Validation(_))
        ));
        let readable_depth = device
            .create_texture(&RhiTextureDesc {
                label: Some("readable_depth".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        assert_eq!(
            device
                .read_texture_depth32f(
                    readable_depth,
                    RhiTextureRegion {
                        x: 0,
                        y: 0,
                        width: 1,
                        height: 1,
                    },
                )
                .unwrap(),
            vec![0.0]
        );
        let depth_without_copy_src = device
            .create_texture(&RhiTextureDesc {
                label: Some("depth_without_copy_src".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        assert!(matches!(
            device.read_texture_depth32f(
                depth_without_copy_src,
                RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_shader_module(&RhiShaderModuleDesc {
                label: None,
                source: " ".to_owned(),
            }),
            Err(RendererError::ShaderCompile(_))
        ));
        assert!(matches!(
            device.create_sampler(&RhiSamplerDesc {
                label: Some(" ".to_owned()),
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_compute_pipeline(&RhiComputePipelineDesc {
                label: Some("bad_compute".to_owned()),
                shader: RhiShaderModule(999),
                entry_point: "main".to_owned(),
            }),
            Err(RendererError::Validation(_))
        ));

        let shader = device
            .create_shader_module(&RhiShaderModuleDesc {
                label: Some("shader".to_owned()),
                source: "@compute @workgroup_size(1) fn main() {}".to_owned(),
            })
            .unwrap();
        assert!(matches!(
            device.create_compute_pipeline(&RhiComputePipelineDesc {
                label: Some("bad_compute".to_owned()),
                shader,
                entry_point: " ".to_owned(),
            }),
            Err(RendererError::PipelineCompile(_))
        ));
        assert!(matches!(
            device.create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("bad_graphics".to_owned()),
                vertex_shader: RhiShaderModule(999),
                vertex_entry: "main".to_owned(),
                fragment_shader: None,
                fragment_entry: None,
                color_format: None,
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("bad_graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: " ".to_owned(),
                fragment_shader: None,
                fragment_entry: None,
                color_format: None,
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            }),
            Err(RendererError::PipelineCompile(_))
        ));

        let compute = device
            .create_compute_pipeline(&RhiComputePipelineDesc {
                label: Some("compute".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        assert!(matches!(
            device.create_compute_bind_group(&RhiComputeBindGroupDesc {
                label: Some("bad_compute_bind_group".to_owned()),
                pipeline: compute,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer {
                    binding: 0,
                    buffer: tiny,
                }],
            }),
            Err(RendererError::Validation(_))
        ));
        let storage_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("storage_buffer".to_owned()),
                size: 16,
                usage: RhiBufferUsage::STORAGE,
            })
            .unwrap();
        let compute_bind_group = device
            .create_compute_bind_group(&RhiComputeBindGroupDesc {
                label: Some("compute_bind_group".to_owned()),
                pipeline: compute,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer {
                    binding: 0,
                    buffer: storage_buffer,
                }],
            })
            .unwrap();
        let graphics = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: "main".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("main".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        assert!(matches!(
            device.create_bind_group(&RhiBindGroupDesc {
                label: Some("bad_graphics_bind_group".to_owned()),
                pipeline: graphics,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer {
                    binding: 0,
                    buffer: tiny,
                }],
            }),
            Err(RendererError::Validation(_))
        ));
        let mismatched_graphics = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("mismatched_graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: "main".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("main".to_owned()),
                color_format: Some(TextureFormat::Rgba16Float),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let other_graphics = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("other_graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: "main".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("main".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let vertex_graphics = device
            .create_graphics_pipeline(&RhiGraphicsPipelineDesc {
                label: Some("vertex_graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: "main".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("main".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![RhiVertexAttribute {
                        location: 0,
                        format: VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: RhiPrimitiveState::default(),
                depth: None,
                sample_count: 1,
            })
            .unwrap();
        let pipeline_statistics = device
            .create_pipeline_statistics_query(&RhiPipelineStatisticsQueryDesc {
                label: Some("pipeline_statistics".to_owned()),
            })
            .unwrap();
        let occlusion = device
            .create_occlusion_query(&RhiOcclusionQueryDesc {
                label: Some("occlusion".to_owned()),
            })
            .unwrap();
        let mut unclosed_debug_group = device.create_command_encoder(None).unwrap();
        unclosed_debug_group.push_debug_group("open").unwrap();
        assert!(matches!(
            unclosed_debug_group.finish(),
            Err(RendererError::Validation(_))
        ));
        let mut unclosed_pipeline_statistics = device.create_command_encoder(None).unwrap();
        unclosed_pipeline_statistics
            .begin_pipeline_statistics(pipeline_statistics)
            .unwrap();
        assert!(matches!(
            unclosed_pipeline_statistics.finish(),
            Err(RendererError::Validation(_))
        ));
        let mut unclosed_occlusion = device.create_command_encoder(None).unwrap();
        unclosed_occlusion.begin_occlusion_query(occlusion).unwrap();
        assert!(matches!(
            unclosed_occlusion.finish(),
            Err(RendererError::Validation(_))
        ));
        let mut encoder = device.create_command_encoder(None).unwrap();
        assert!(matches!(
            encoder.encode_resource_barrier(&RhiResourceBarrierDesc {
                resource: RhiResource::Texture(RhiTexture(999)),
                before: None,
                after: RhiAccessState::TextureSampled,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_resource_barrier(&RhiResourceBarrierDesc {
                resource: RhiResource::Texture(small_texture),
                before: None,
                after: RhiAccessState::BufferVertex,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_resource_barrier(&RhiResourceBarrierDesc {
                resource: RhiResource::Buffer(tiny),
                before: None,
                after: RhiAccessState::TextureSampled,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_resource_barrier(&RhiResourceBarrierDesc {
                resource: RhiResource::Texture(sampled_only_texture),
                before: Some(RhiAccessState::TextureSampled),
                after: RhiAccessState::CopyDst,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.write_timestamp(RhiTimestampQuery(999)),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.begin_pipeline_statistics(RhiPipelineStatisticsQuery(999)),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.end_pipeline_statistics(RhiPipelineStatisticsQuery(999)),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.begin_occlusion_query(RhiOcclusionQuery(999)),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.end_occlusion_query(RhiOcclusionQuery(999)),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.push_debug_group(" "),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.pop_debug_group(),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_compute_pass(&RhiComputePassDesc {
                label: Some(" ".to_owned()),
                pipeline: compute,
                bind_groups: Vec::new(),
                workgroups: [1, 1, 1],
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: Some(" ".to_owned()),
                pipeline: graphics,
                color_target: Some(small_texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_indirect_render_pass(&RhiIndirectRenderPassDesc {
                label: Some(" ".to_owned()),
                pipeline: graphics,
                color_target: Some(small_texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                bind_groups: Vec::new(),
                indirect_buffer: tiny,
                indirect_offset: 0,
                draw_count: 1,
                draw_stride: RHI_DRAW_INDIRECT_BYTES,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_compute_pass(&RhiComputePassDesc {
                label: None,
                pipeline: compute,
                bind_groups: vec![RhiComputePassBindGroup {
                    index: 1,
                    bind_group: compute_bind_group,
                }],
                workgroups: [1, 1, 1],
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_compute_pass(&RhiComputePassDesc {
                label: None,
                pipeline: compute,
                bind_groups: Vec::new(),
                workgroups: [1, 0, 1],
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: None,
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: Some(sampled_only_texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        let depth_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("depth_attachment".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: Some(depth_texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: None,
                depth_target: Some(small_texture),
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        let color_target = device
            .create_texture(&RhiTextureDesc {
                label: Some("color_attachment".to_owned()),
                width: 2,
                height: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let vertex_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("vertex_buffer".to_owned()),
                size: 16,
                usage: RhiBufferUsage::VERTEX,
            })
            .unwrap();
        let short_index_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("short_index_buffer".to_owned()),
                size: 4,
                usage: RhiBufferUsage::INDEX,
            })
            .unwrap();
        let bind_group = device
            .create_bind_group(&RhiBindGroupDesc {
                label: Some("bind_group".to_owned()),
                pipeline: graphics,
                group_index: 0,
                entries: vec![RhiBindGroupEntry::Buffer {
                    binding: 0,
                    buffer: storage_buffer,
                }],
            })
            .unwrap();
        assert!(matches!(
            encoder.encode_compute_pass(&RhiComputePassDesc {
                label: None,
                pipeline: compute,
                bind_groups: vec![RhiComputePassBindGroup {
                    index: 0,
                    bind_group,
                }],
                workgroups: [1, 1, 1],
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: Some(color_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: vec![RhiRenderPassBindGroup {
                    index: 1,
                    bind_group,
                }],
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: Some(color_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: Some(RhiIndexBufferBinding {
                    buffer: short_index_buffer,
                    offset: 0,
                    format: RhiIndexFormat::Uint16,
                }),
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: Some(3),
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: vertex_graphics,
                color_target: Some(color_target),
                depth_target: None,
                vertex_buffers: vec![RhiVertexBufferBinding {
                    slot: 0,
                    buffer: vertex_buffer,
                    offset: 0,
                }],
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: other_graphics,
                color_target: Some(color_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: vec![RhiRenderPassBindGroup {
                    index: 0,
                    bind_group,
                }],
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_render_pass(&RhiRenderPassDesc {
                label: None,
                pipeline: mismatched_graphics,
                color_target: Some(color_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_indirect_render_pass(&RhiIndirectRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: Some(color_target),
                depth_target: None,
                vertex_buffers: Vec::new(),
                bind_groups: Vec::new(),
                indirect_buffer: tiny,
                indirect_offset: 0,
                draw_count: 1,
                draw_stride: RHI_DRAW_INDIRECT_BYTES,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_indirect_render_pass(&RhiIndirectRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: None,
                depth_target: None,
                vertex_buffers: Vec::new(),
                bind_groups: Vec::new(),
                indirect_buffer: tiny,
                indirect_offset: 0,
                draw_count: 1,
                draw_stride: RHI_DRAW_INDIRECT_BYTES,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            encoder.encode_indirect_render_pass(&RhiIndirectRenderPassDesc {
                label: None,
                pipeline: graphics,
                color_target: Some(small_texture),
                depth_target: None,
                vertex_buffers: Vec::new(),
                bind_groups: Vec::new(),
                indirect_buffer: tiny,
                indirect_offset: 0,
                draw_count: 1,
                draw_stride: RHI_DRAW_INDIRECT_BYTES,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn headless_rhi_rejects_invalid_submissions() {
        let device = HeadlessRhiDevice::new();

        assert!(matches!(
            device.submit(Vec::new()),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            device.submit(vec![RhiCommandBuffer(999)]),
            Err(RendererError::Validation(_))
        ));

        let command = device
            .create_command_encoder(None)
            .unwrap()
            .finish()
            .unwrap();
        device.submit(vec![command]).unwrap();
        assert!(matches!(
            device.submit(vec![command]),
            Err(RendererError::Validation(_))
        ));
    }
}
