use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use crate::{
    rhi::{
        RhiAccessState, RhiBuffer, RhiBufferDesc, RhiBufferUsage, RhiCommandEncoder,
        RhiComputePassDesc, RhiComputePipeline, RhiDevice, RhiGraphicsPipeline,
        RhiIndexedIndirectRenderPassDesc, RhiIndirectRenderPassDesc, RhiRenderPassDesc, RhiResource,
        RhiResourceBarrierDesc, RhiTexture, RhiTextureDesc, RhiTextureUsage, RhiTimestampQueryDesc,
    },
    BufferDesc, BufferHandle, RenderLayerMask, RenderPath, RendererCaps, RendererError,
    RendererFeature, RendererFeatures, SceneHandle, TextureDesc, TextureFormat, TextureHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PassId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphTexture(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphBuffer(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueType {
    Graphics,
    Compute,
    AsyncCompute,
    Copy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureReadUsage {
    Sampled,
    Storage,
    CopySrc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureWriteUsage {
    Storage,
    CopyDst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphTextureUsage(pub u32);

impl GraphTextureUsage {
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

impl std::ops::BitOr for GraphTextureUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferReadUsage {
    Uniform,
    Storage,
    Vertex,
    Index,
    Indirect,
    CopySrc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferWriteUsage {
    Storage,
    CopyDst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphBufferUsage(pub u32);

impl GraphBufferUsage {
    pub const UNIFORM: Self = Self(1 << 0);
    pub const STORAGE: Self = Self(1 << 1);
    pub const VERTEX: Self = Self(1 << 2);
    pub const INDEX: Self = Self(1 << 3);
    pub const COPY_SRC: Self = Self(1 << 4);
    pub const COPY_DST: Self = Self(1 << 5);
    pub const INDIRECT: Self = Self(1 << 6);

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

impl std::ops::BitOr for GraphBufferUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorAttachmentOps {
    pub load: bool,
    pub store: bool,
}

impl ColorAttachmentOps {
    pub const fn load_store() -> Self {
        Self {
            load: true,
            store: true,
        }
    }

    pub const fn clear_store() -> Self {
        Self {
            load: false,
            store: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepthAttachmentOps {
    pub load: bool,
    pub store: bool,
}

impl DepthAttachmentOps {
    pub const fn load_store() -> Self {
        Self {
            load: true,
            store: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphTextureDesc {
    pub label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphBufferDesc {
    pub label: Option<String>,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportedTexture {
    label: String,
    texture: TextureHandle,
    usage: GraphTextureUsage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportedBuffer {
    label: String,
    buffer: BufferHandle,
    usage: GraphBufferUsage,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RhiResourceImports {
    pub textures: HashMap<TextureHandle, RhiTexture>,
    pub buffers: HashMap<BufferHandle, RhiBuffer>,
}

impl RhiResourceImports {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_texture(mut self, texture: TextureHandle, rhi_texture: RhiTexture) -> Self {
        self.textures.insert(texture, rhi_texture);
        self
    }

    pub fn with_buffer(mut self, buffer: BufferHandle, rhi_buffer: RhiBuffer) -> Self {
        self.buffers.insert(buffer, rhi_buffer);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderGraphStats {
    pub pass_count: u32,
    pub pass_labels: Vec<String>,
    pub transient_textures: u32,
    pub transient_buffers: u32,
    pub aliased_memory_bytes: u64,
    pub barriers: u32,
    pub executed_callbacks: u32,
    pub graphics_queue_passes: u32,
    pub compute_queue_passes: u32,
    pub async_compute_queue_passes: u32,
    pub copy_queue_passes: u32,
    pub render_passes: u32,
    pub compute_passes: u32,
    pub pipeline_binds: u32,
    pub fullscreen_draws: u32,
    pub compute_dispatches: u32,
    pub phase_draws: u32,
    pub debug_groups: u32,
    pub timestamp_queries: u32,
    pub timestamp_writes: u32,
    pub variable_rate_shading_passes: u32,
    pub bindless_texture_table_passes: u32,
    pub mesh_shader_passes: u32,
    pub virtual_texture_feedback_passes: u32,
    pub ray_tracing_passes: u32,
    pub gpu_time_ns: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompiledRenderGraph {
    pub passes: Vec<CompiledPass>,
    pub resource_lifetimes: Vec<ResourceLifetime>,
    pub resource_accesses: Vec<CompiledResourceAccess>,
    pub barriers: Vec<ResourceBarrier>,
    pub alias_allocations: Vec<AliasAllocation>,
    pub stats: RenderGraphStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPass {
    pub id: PassId,
    pub label: String,
    pub queue: QueueType,
    pub dependencies: Vec<PassId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewInfo {
    pub label: Option<String>,
    pub scene: SceneHandle,
    pub render_path: RenderPath,
    pub layers: RenderLayerMask,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceLifetime {
    pub resource: GraphResource,
    pub first_pass: PassId,
    pub last_pass: PassId,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledResourceAccess {
    pub pass: PassId,
    pub resource: GraphResource,
    pub access: GraphAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphAccess {
    TextureRead(TextureReadUsage),
    TextureWrite(TextureWriteUsage),
    ColorAttachment(ColorAttachmentOps),
    DepthAttachment(DepthAttachmentOps),
    BufferRead(BufferReadUsage),
    BufferWrite(BufferWriteUsage),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBarrier {
    pub resource: GraphResource,
    pub from_pass: Option<PassId>,
    pub to_pass: PassId,
    pub before: Option<GraphAccess>,
    pub after: GraphAccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasAllocation {
    pub resource: GraphResource,
    pub slot: u32,
    pub offset: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphResource {
    Texture(GraphTexture),
    Buffer(GraphBuffer),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphPipelineRef {
    label: String,
}

impl GraphPipelineRef {
    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomPostProcessInfo {
    pub pass_label: String,
    pub output_texture_label: String,
}

pub trait RenderGraphExtension: Send + Sync + 'static {
    fn name(&self) -> &str;

    fn build(
        &self,
        ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError>;

    fn custom_post_process_info(&self) -> Option<CustomPostProcessInfo> {
        None
    }
}

pub trait RenderPassNode: Send + Sync + 'static {
    fn setup(&self, builder: &mut PassBuilder<'_, '_>);
    fn execute(&self, ctx: &mut PassContext<'_>) -> Result<(), RendererError>;
}

#[derive(Clone, Debug)]
pub struct RenderGraphExtensionContext {
    main_depth: GraphTexture,
    main_color: GraphTexture,
    caps: RendererCaps,
}

impl RenderGraphExtensionContext {
    pub fn new(main_color: GraphTexture, main_depth: GraphTexture, caps: RendererCaps) -> Self {
        Self {
            main_color,
            main_depth,
            caps,
        }
    }

    pub fn main_depth(&self) -> GraphTexture {
        self.main_depth
    }

    pub fn main_color(&self) -> GraphTexture {
        self.main_color
    }

    pub fn renderer_caps(&self) -> &RendererCaps {
        &self.caps
    }
}

pub struct RenderGraphBuilder<'a> {
    next_pass: u32,
    next_texture: u32,
    next_buffer: u32,
    passes: Vec<PassNode>,
    textures: HashMap<GraphTexture, GraphTextureDesc>,
    buffers: HashMap<GraphBuffer, GraphBufferDesc>,
    imported_textures: HashMap<GraphTexture, ImportedTexture>,
    imported_buffers: HashMap<GraphBuffer, ImportedBuffer>,
    _marker: PhantomData<&'a mut ()>,
}

impl<'a> Default for RenderGraphBuilder<'a> {
    fn default() -> Self {
        Self {
            next_pass: 0,
            next_texture: 0,
            next_buffer: 0,
            passes: Vec::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
            imported_textures: HashMap::new(),
            imported_buffers: HashMap::new(),
            _marker: PhantomData,
        }
    }
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn import_texture(
        &mut self,
        label: impl Into<String>,
        texture: TextureHandle,
        usage: GraphTextureUsage,
    ) -> GraphTexture {
        let id = GraphTexture(self.next_texture);
        self.next_texture += 1;
        self.imported_textures.insert(
            id,
            ImportedTexture {
                label: label.into(),
                texture,
                usage,
            },
        );
        id
    }

    pub fn create_texture(&mut self, desc: GraphTextureDesc) -> GraphTexture {
        let id = GraphTexture(self.next_texture);
        self.next_texture += 1;
        self.textures.insert(id, desc);
        id
    }

    pub fn create_texture_from_desc(
        &mut self,
        label: impl Into<String>,
        desc: TextureDesc<'_>,
    ) -> GraphTexture {
        self.create_texture(GraphTextureDesc {
            label: Some(label.into()),
            width: desc.width,
            height: desc.height,
            format: desc.format,
        })
    }

    pub fn texture_desc(&self, texture: GraphTexture) -> Option<&GraphTextureDesc> {
        self.textures.get(&texture)
    }

    pub fn import_buffer(
        &mut self,
        label: impl Into<String>,
        buffer: BufferHandle,
        usage: GraphBufferUsage,
    ) -> GraphBuffer {
        let id = GraphBuffer(self.next_buffer);
        self.next_buffer += 1;
        self.imported_buffers.insert(
            id,
            ImportedBuffer {
                label: label.into(),
                buffer,
                usage,
            },
        );
        id
    }

    pub fn create_buffer(&mut self, desc: GraphBufferDesc) -> GraphBuffer {
        let id = GraphBuffer(self.next_buffer);
        self.next_buffer += 1;
        self.buffers.insert(id, desc);
        id
    }

    pub fn create_buffer_from_desc(
        &mut self,
        label: impl Into<String>,
        desc: BufferDesc<'_>,
    ) -> GraphBuffer {
        self.create_buffer(GraphBufferDesc {
            label: Some(label.into()),
            size: desc.size,
        })
    }

    pub fn add_pass(&mut self, label: impl Into<String>) -> PassBuilder<'_, 'a> {
        PassBuilder {
            graph: self,
            record: PassRecord {
                label: label.into(),
                queue: QueueType::Graphics,
                accesses: Vec::new(),
                dependencies: Vec::new(),
            },
        }
    }

    pub fn stats(&self) -> RenderGraphStats {
        self.compile()
            .map(|compiled| compiled.stats)
            .unwrap_or_else(|_| RenderGraphStats {
                pass_count: self.passes.len() as u32,
                pass_labels: self
                    .passes
                    .iter()
                    .map(|pass| pass.record.label.clone())
                    .collect(),
                transient_textures: self.textures.len() as u32,
                transient_buffers: self.buffers.len() as u32,
                barriers: self
                    .passes
                    .iter()
                    .map(|pass| pass.record.accesses.len() + pass.record.dependencies.len())
                    .sum::<usize>() as u32,
                graphics_queue_passes: self
                    .passes
                    .iter()
                    .filter(|pass| matches!(pass.record.queue, QueueType::Graphics))
                    .count() as u32,
                compute_queue_passes: self
                    .passes
                    .iter()
                    .filter(|pass| matches!(pass.record.queue, QueueType::Compute))
                    .count() as u32,
                async_compute_queue_passes: self
                    .passes
                    .iter()
                    .filter(|pass| matches!(pass.record.queue, QueueType::AsyncCompute))
                    .count() as u32,
                copy_queue_passes: self
                    .passes
                    .iter()
                    .filter(|pass| matches!(pass.record.queue, QueueType::Copy))
                    .count() as u32,
                variable_rate_shading_passes: self
                    .passes
                    .iter()
                    .filter(|pass| pass.record.label == "vrs_shading_rate")
                    .count() as u32,
                bindless_texture_table_passes: self
                    .passes
                    .iter()
                    .filter(|pass| pass.record.label == "bindless_texture_table")
                    .count() as u32,
                mesh_shader_passes: self
                    .passes
                    .iter()
                    .filter(|pass| pass.record.label == "meshlet_culling")
                    .count() as u32,
                virtual_texture_feedback_passes: self
                    .passes
                    .iter()
                    .filter(|pass| pass.record.label == "virtual_texture_feedback")
                    .count() as u32,
                ray_tracing_passes: self
                    .passes
                    .iter()
                    .filter(|pass| pass.record.label == "ray_tracing_accel_build")
                    .count() as u32,
                ..RenderGraphStats::default()
            })
    }

    pub fn validate(&self) -> Result<(), RendererError> {
        for (texture, desc) in &self.textures {
            if desc.width == 0 || desc.height == 0 {
                return Err(RendererError::RenderGraphValidation(format!(
                    "texture {:?} must have non-zero dimensions",
                    texture
                )));
            }
        }
        for (buffer, desc) in &self.buffers {
            if desc.size == 0 {
                return Err(RendererError::RenderGraphValidation(format!(
                    "buffer {:?} must have a non-zero size",
                    buffer
                )));
            }
        }
        for (index, pass) in self.passes.iter().enumerate() {
            if pass.record.label.trim().is_empty() {
                return Err(RendererError::RenderGraphValidation(
                    "render graph pass label must not be empty".to_owned(),
                ));
            }
            for dependency in &pass.record.dependencies {
                if dependency.0 as usize >= index {
                    return Err(RendererError::RenderGraphValidation(format!(
                        "pass '{}' depends on {:?}, which is not an earlier pass",
                        pass.record.label, dependency
                    )));
                }
            }
            for access in &pass.record.accesses {
                match access.resource {
                    ResourceUse::Texture(texture)
                        if !self.textures.contains_key(&texture)
                            && !self.imported_textures.contains_key(&texture) =>
                    {
                        return Err(RendererError::RenderGraphValidation(format!(
                            "pass '{}' references unknown texture {:?}",
                            pass.record.label, texture
                        )));
                    }
                    ResourceUse::Buffer(buffer)
                        if !self.buffers.contains_key(&buffer)
                            && !self.imported_buffers.contains_key(&buffer) =>
                    {
                        return Err(RendererError::RenderGraphValidation(format!(
                            "pass '{}' references unknown buffer {:?}",
                            pass.record.label, buffer
                        )));
                    }
                    _ => {}
                }
            }
            for access in pass.record.accesses.iter() {
                self.validate_declared_access(&pass.record.label, access)?;
            }
        }
        Ok(())
    }

    pub fn compile(&self) -> Result<CompiledRenderGraph, RendererError> {
        self.compile_with_transient_aliasing(true)
    }

    pub fn compile_with_transient_aliasing(
        &self,
        transient_resource_aliasing: bool,
    ) -> Result<CompiledRenderGraph, RendererError> {
        self.validate()?;
        let mut lifetimes: HashMap<GraphResource, ResourceLifetime> = HashMap::new();
        let mut resource_accesses = Vec::new();
        for (index, pass) in self.passes.iter().enumerate() {
            let pass_id = PassId(index as u32);
            for access in &pass.record.accesses {
                let resource = access.resource.graph_resource();
                lifetimes
                    .entry(resource)
                    .and_modify(|lifetime| lifetime.last_pass = pass_id)
                    .or_insert_with(|| ResourceLifetime {
                        resource,
                        first_pass: pass_id,
                        last_pass: pass_id,
                        bytes: self.resource_bytes(resource),
                    });
                resource_accesses.push(CompiledResourceAccess {
                    pass: pass_id,
                    resource,
                    access: access.access,
                });
            }
        }
        let mut resource_lifetimes = lifetimes.into_values().collect::<Vec<_>>();
        resource_lifetimes.sort_by_key(|lifetime| match lifetime.resource {
            GraphResource::Texture(texture) => (0_u8, texture.0),
            GraphResource::Buffer(buffer) => (1_u8, buffer.0),
        });
        let passes = self
            .passes
            .iter()
            .enumerate()
            .map(|(index, pass)| CompiledPass {
                id: PassId(index as u32),
                label: pass.record.label.clone(),
                queue: pass.record.queue,
                dependencies: pass.record.dependencies.clone(),
            })
            .collect::<Vec<_>>();
        let barriers = compile_resource_barriers(&resource_accesses);
        let alias_allocations = transient_resource_aliasing
            .then(|| compile_alias_allocations(&resource_lifetimes))
            .unwrap_or_default();
        let transient_memory_bytes = resource_lifetimes
            .iter()
            .filter(|lifetime| lifetime.bytes > 0)
            .map(|lifetime| lifetime.bytes)
            .sum::<u64>();
        let peak_memory_bytes = peak_memory_bytes(&resource_lifetimes);
        let aliased_memory_bytes = transient_resource_aliasing
            .then(|| transient_memory_bytes.saturating_sub(peak_memory_bytes))
            .unwrap_or(0);
        let stats = RenderGraphStats {
            pass_count: self.passes.len() as u32,
            pass_labels: passes.iter().map(|pass| pass.label.clone()).collect(),
            transient_textures: self.textures.len() as u32,
            transient_buffers: self.buffers.len() as u32,
            aliased_memory_bytes,
            barriers: barriers.len() as u32,
            graphics_queue_passes: passes
                .iter()
                .filter(|pass| matches!(pass.queue, QueueType::Graphics))
                .count() as u32,
            compute_queue_passes: passes
                .iter()
                .filter(|pass| matches!(pass.queue, QueueType::Compute))
                .count() as u32,
            async_compute_queue_passes: passes
                .iter()
                .filter(|pass| matches!(pass.queue, QueueType::AsyncCompute))
                .count() as u32,
            copy_queue_passes: passes
                .iter()
                .filter(|pass| matches!(pass.queue, QueueType::Copy))
                .count() as u32,
            variable_rate_shading_passes: passes
                .iter()
                .filter(|pass| pass.label == "vrs_shading_rate")
                .count() as u32,
            bindless_texture_table_passes: passes
                .iter()
                .filter(|pass| pass.label == "bindless_texture_table")
                .count() as u32,
            mesh_shader_passes: passes
                .iter()
                .filter(|pass| pass.label == "meshlet_culling")
                .count() as u32,
            virtual_texture_feedback_passes: passes
                .iter()
                .filter(|pass| pass.label == "virtual_texture_feedback")
                .count() as u32,
            ray_tracing_passes: passes
                .iter()
                .filter(|pass| pass.label == "ray_tracing_accel_build")
                .count() as u32,
            ..RenderGraphStats::default()
        };
        Ok(CompiledRenderGraph {
            passes,
            resource_lifetimes,
            resource_accesses,
            barriers,
            alias_allocations,
            stats,
        })
    }

    fn validate_declared_access(
        &self,
        pass_label: &str,
        access: &ResourceAccess,
    ) -> Result<(), RendererError> {
        match (access.resource, access.access) {
            (ResourceUse::Texture(texture), GraphAccess::TextureRead(usage)) => {
                self.validate_texture_usage(pass_label, texture, texture_read_flag(usage))
            }
            (ResourceUse::Texture(texture), GraphAccess::TextureWrite(usage)) => {
                self.validate_texture_usage(pass_label, texture, texture_write_flag(usage))
            }
            (ResourceUse::Texture(texture), GraphAccess::ColorAttachment(_)) => self
                .validate_texture_usage(pass_label, texture, GraphTextureUsage::RENDER_ATTACHMENT),
            (ResourceUse::Texture(texture), GraphAccess::DepthAttachment(_)) => self
                .validate_texture_usage(pass_label, texture, GraphTextureUsage::RENDER_ATTACHMENT),
            (ResourceUse::Buffer(buffer), GraphAccess::BufferRead(usage)) => {
                self.validate_buffer_usage(pass_label, buffer, buffer_read_flag(usage))
            }
            (ResourceUse::Buffer(buffer), GraphAccess::BufferWrite(usage)) => {
                self.validate_buffer_usage(pass_label, buffer, buffer_write_flag(usage))
            }
            (ResourceUse::Texture(_), _) | (ResourceUse::Buffer(_), _) => {
                Err(RendererError::RenderGraphValidation(format!(
                    "pass '{pass_label}' declares incompatible resource access"
                )))
            }
        }
    }

    fn validate_texture_usage(
        &self,
        pass_label: &str,
        texture: GraphTexture,
        required: GraphTextureUsage,
    ) -> Result<(), RendererError> {
        if self.textures.contains_key(&texture) {
            return Ok(());
        }
        let Some(imported) = self.imported_textures.get(&texture) else {
            return Ok(());
        };
        if imported.usage.0 & required.0 == required.0 {
            Ok(())
        } else {
            Err(RendererError::RenderGraphValidation(format!(
                "pass '{pass_label}' uses imported texture '{}' without required usage",
                imported.label
            )))
        }
    }

    fn validate_buffer_usage(
        &self,
        pass_label: &str,
        buffer: GraphBuffer,
        required: GraphBufferUsage,
    ) -> Result<(), RendererError> {
        if self.buffers.contains_key(&buffer) {
            return Ok(());
        }
        let Some(imported) = self.imported_buffers.get(&buffer) else {
            return Ok(());
        };
        if imported.usage.0 & required.0 == required.0 {
            Ok(())
        } else {
            Err(RendererError::RenderGraphValidation(format!(
                "pass '{pass_label}' uses imported buffer '{}' without required usage",
                imported.label
            )))
        }
    }

    fn resource_bytes(&self, resource: GraphResource) -> u64 {
        match resource {
            GraphResource::Texture(texture) => self.textures.get(&texture).map_or(0, |desc| {
                desc.width as u64 * desc.height as u64 * texture_format_bytes(desc.format) as u64
            }),
            GraphResource::Buffer(buffer) => self.buffers.get(&buffer).map_or(0, |desc| desc.size),
        }
    }

    pub fn execute(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
    ) -> Result<RenderGraphStats, RendererError> {
        self.execute_with_view(frame_index, caps, None)
    }

    pub fn execute_with_view(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
        view: Option<ViewInfo>,
    ) -> Result<RenderGraphStats, RendererError> {
        self.execute_with_view_and_transient_aliasing(frame_index, caps, view, true)
    }

    pub fn execute_with_view_and_transient_aliasing(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
        view: Option<ViewInfo>,
        transient_resource_aliasing: bool,
    ) -> Result<RenderGraphStats, RendererError> {
        self.execute_with_view_options(frame_index, caps, view, transient_resource_aliasing, false)
    }

    pub fn execute_with_view_options(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
        view: Option<ViewInfo>,
        transient_resource_aliasing: bool,
        debug_labels: bool,
    ) -> Result<RenderGraphStats, RendererError> {
        self.validate()?;
        let compiled = self.compile_with_transient_aliasing(transient_resource_aliasing)?;
        validate_compiled_graph_caps(&compiled, caps)?;
        let execution = Rc::new(RefCell::new(PassExecutionStats::default()));
        for pass in &mut self.passes {
            if let Some(callback) = pass.callback.take() {
                let mut ctx = PassContext::new_with_view_and_execution(
                    frame_index,
                    caps,
                    view.clone(),
                    declared_pass_resources(&pass.record),
                    Rc::clone(&execution),
                    None,
                    None,
                    None,
                    None,
                );
                ctx.record_callback();
                if debug_labels {
                    ctx.push_debug_group(&pass.record.label);
                }
                let result = callback(&mut ctx);
                if debug_labels {
                    ctx.pop_debug_group();
                }
                result?;
            }
        }
        let mut stats = compiled.stats;
        stats.merge_execution(execution.borrow().clone());
        Ok(stats)
    }

    pub fn execute_on_rhi(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
        view: Option<ViewInfo>,
        device: &dyn RhiDevice,
    ) -> Result<RenderGraphStats, RendererError> {
        self.execute_on_rhi_with_imports(
            frame_index,
            caps,
            view,
            device,
            &RhiResourceImports::default(),
        )
    }

    pub fn execute_on_rhi_with_imports(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
        view: Option<ViewInfo>,
        device: &dyn RhiDevice,
        imports: &RhiResourceImports,
    ) -> Result<RenderGraphStats, RendererError> {
        self.validate()?;
        let compiled = self.compile()?;
        validate_compiled_graph_caps(&compiled, caps)?;
        let mut rhi_textures: HashMap<GraphTexture, RhiTexture> = HashMap::new();
        let mut rhi_buffers: HashMap<GraphBuffer, RhiBuffer> = HashMap::new();
        for lifetime in &compiled.resource_lifetimes {
            match lifetime.resource {
                GraphResource::Texture(texture) => {
                    if let Some(desc) = self.textures.get(&texture) {
                        let rhi_texture = device.create_texture(&RhiTextureDesc {
                            label: desc.label.clone(),
                            width: desc.width,
                            height: desc.height,
                            format: desc.format,
                            usage: map_rhi_texture_usage(graph_texture_usage_from_accesses(
                                &compiled.resource_accesses,
                                texture,
                            )),
                        })?;
                        rhi_textures.insert(texture, rhi_texture);
                    }
                }
                GraphResource::Buffer(buffer) => {
                    if let Some(desc) = self.buffers.get(&buffer) {
                        let rhi_buffer = device.create_buffer(&RhiBufferDesc {
                            label: desc.label.clone(),
                            size: desc.size,
                            usage: map_rhi_buffer_usage(graph_buffer_usage_from_accesses(
                                &compiled.resource_accesses,
                                buffer,
                            )),
                        })?;
                        rhi_buffers.insert(buffer, rhi_buffer);
                    } else if let Some(imported) = self.imported_buffers.get(&buffer) {
                        let Some(rhi_buffer) = imports.buffers.get(&imported.buffer) else {
                            return Err(RendererError::RenderGraphValidation(format!(
                                "imported buffer '{}' is used by the render graph but has no RHI import",
                                imported.label
                            )));
                        };
                        validate_imported_rhi_buffer_usage(
                            imported,
                            device.buffer_usage(*rhi_buffer)?,
                            graph_buffer_usage_from_accesses(&compiled.resource_accesses, buffer),
                        )?;
                        rhi_buffers.insert(buffer, *rhi_buffer);
                    }
                }
            }
        }
        for lifetime in &compiled.resource_lifetimes {
            if let GraphResource::Texture(texture) = lifetime.resource {
                if self.textures.contains_key(&texture) {
                    continue;
                }
                if let Some(imported) = self.imported_textures.get(&texture) {
                    let Some(rhi_texture) = imports.textures.get(&imported.texture) else {
                        return Err(RendererError::RenderGraphValidation(format!(
                            "imported texture '{}' is used by the render graph but has no RHI import",
                            imported.label
                        )));
                    };
                    validate_imported_rhi_texture_usage(
                        imported,
                        device.texture_usage(*rhi_texture)?,
                        graph_texture_usage_from_accesses(&compiled.resource_accesses, texture),
                    )?;
                    rhi_textures.insert(texture, *rhi_texture);
                }
            }
        }

        let execution = Rc::new(RefCell::new(PassExecutionStats::default()));
        let mut commands = Vec::new();
        let mut timestamp_pairs = Vec::new();
        for (compiled_pass, pass) in compiled.passes.iter().zip(&mut self.passes) {
            let mut encoder =
                device.create_command_encoder(non_empty_label(&compiled_pass.label))?;
            let timestamp_start = device.create_timestamp_query(&RhiTimestampQueryDesc {
                label: Some(format!("{}:start", compiled_pass.label)),
            })?;
            let timestamp_end = device.create_timestamp_query(&RhiTimestampQueryDesc {
                label: Some(format!("{}:end", compiled_pass.label)),
            })?;
            encoder.write_timestamp(timestamp_start)?;
            for barrier in compiled
                .barriers
                .iter()
                .filter(|barrier| barrier.to_pass == compiled_pass.id)
            {
                let desc = map_rhi_barrier(barrier, &rhi_textures, &rhi_buffers)?;
                encoder.encode_resource_barrier(&desc)?;
            }
            if let Some(callback) = pass.callback.take() {
                let mut ctx = PassContext::new_with_view_and_execution(
                    frame_index,
                    caps,
                    view.clone(),
                    declared_pass_resources(&pass.record),
                    Rc::clone(&execution),
                    Some(encoder.as_mut()),
                    Some(device),
                    Some(&rhi_textures),
                    Some(&rhi_buffers),
                );
                ctx.set_declared_accesses(
                    pass.record
                        .accesses
                        .iter()
                        .map(|a| (a.resource.graph_resource(), a.access))
                        .collect(),
                );
                ctx.record_callback();
                callback(&mut ctx)?;
            }
            encoder.write_timestamp(timestamp_end)?;
            timestamp_pairs.push((timestamp_start, timestamp_end));
            commands.push(encoder.finish()?);
        }
        if !commands.is_empty() {
            let _ = device.submit(commands)?;
            device.poll(crate::rhi::PollMode::Poll);
        }

        let mut stats = compiled.stats;
        stats.merge_execution(execution.borrow().clone());
        stats.timestamp_queries = (timestamp_pairs.len() * 2) as u32;
        stats.timestamp_writes = stats.timestamp_queries;
        let mut gpu_time_ns = 0_u64;
        for (start, end) in timestamp_pairs {
            let start = device.timestamp_result(start)?;
            let end = device.timestamp_result(end)?;
            if start.available && end.available && end.timestamp_ns >= start.timestamp_ns {
                gpu_time_ns = gpu_time_ns.saturating_add(end.timestamp_ns - start.timestamp_ns);
            }
        }
        stats.gpu_time_ns = (stats.timestamp_writes > 0).then_some(gpu_time_ns);
        Ok(stats)
    }
}

fn non_empty_label(label: &str) -> Option<&str> {
    if label.trim().is_empty() {
        None
    } else {
        Some(label)
    }
}

fn validate_compiled_graph_caps(
    compiled: &CompiledRenderGraph,
    caps: &RendererCaps,
) -> Result<(), RendererError> {
    if compiled
        .passes
        .iter()
        .any(|pass| matches!(pass.queue, QueueType::AsyncCompute))
        && !caps.features.contains(RendererFeatures::ASYNC_COMPUTE)
    {
        return Err(RendererError::UnsupportedFeature(
            RendererFeature::AsyncCompute,
        ));
    }
    Ok(())
}

fn texture_format_bytes(format: TextureFormat) -> u32 {
    match format {
        TextureFormat::Rgba16Float => 8,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb | TextureFormat::Depth32Float => {
            4
        }
        TextureFormat::Rgba32Float => 16,
        _ => 4,
    }
}

fn texture_read_flag(usage: TextureReadUsage) -> GraphTextureUsage {
    match usage {
        TextureReadUsage::Sampled => GraphTextureUsage::SAMPLED,
        TextureReadUsage::Storage => GraphTextureUsage::STORAGE,
        TextureReadUsage::CopySrc => GraphTextureUsage::COPY_SRC,
    }
}

fn texture_write_flag(usage: TextureWriteUsage) -> GraphTextureUsage {
    match usage {
        TextureWriteUsage::Storage => GraphTextureUsage::STORAGE,
        TextureWriteUsage::CopyDst => GraphTextureUsage::COPY_DST,
    }
}

fn map_rhi_texture_usage(usage: GraphTextureUsage) -> RhiTextureUsage {
    let mut mapped = RhiTextureUsage::empty();
    if usage.contains(GraphTextureUsage::SAMPLED) {
        mapped = mapped | RhiTextureUsage::SAMPLED;
    }
    if usage.contains(GraphTextureUsage::STORAGE) {
        mapped = mapped | RhiTextureUsage::STORAGE;
    }
    if usage.contains(GraphTextureUsage::RENDER_ATTACHMENT) {
        mapped = mapped | RhiTextureUsage::RENDER_ATTACHMENT;
    }
    if usage.contains(GraphTextureUsage::COPY_SRC) {
        mapped = mapped | RhiTextureUsage::COPY_SRC;
    }
    if usage.contains(GraphTextureUsage::COPY_DST) {
        mapped = mapped | RhiTextureUsage::COPY_DST;
    }
    mapped
}

fn map_rhi_buffer_usage(usage: GraphBufferUsage) -> RhiBufferUsage {
    let mut mapped = RhiBufferUsage::empty();
    if usage.contains(GraphBufferUsage::UNIFORM) {
        mapped = mapped | RhiBufferUsage::UNIFORM;
    }
    if usage.contains(GraphBufferUsage::STORAGE) {
        mapped = mapped | RhiBufferUsage::STORAGE;
    }
    if usage.contains(GraphBufferUsage::VERTEX) {
        mapped = mapped | RhiBufferUsage::VERTEX;
    }
    if usage.contains(GraphBufferUsage::INDEX) {
        mapped = mapped | RhiBufferUsage::INDEX;
    }
    if usage.contains(GraphBufferUsage::INDIRECT) {
        mapped = mapped | RhiBufferUsage::INDIRECT;
    }
    if usage.contains(GraphBufferUsage::COPY_SRC) {
        mapped = mapped | RhiBufferUsage::COPY_SRC;
    }
    if usage.contains(GraphBufferUsage::COPY_DST) {
        mapped = mapped | RhiBufferUsage::COPY_DST;
    }
    mapped
}

fn validate_imported_rhi_texture_usage(
    imported: &ImportedTexture,
    actual: RhiTextureUsage,
    required: GraphTextureUsage,
) -> Result<(), RendererError> {
    let required = map_rhi_texture_usage(required);
    if !actual.contains(required) {
        return Err(RendererError::RenderGraphValidation(format!(
            "imported texture '{}' RHI usage does not satisfy render graph usage",
            imported.label
        )));
    }
    Ok(())
}

fn validate_imported_rhi_buffer_usage(
    imported: &ImportedBuffer,
    actual: RhiBufferUsage,
    required: GraphBufferUsage,
) -> Result<(), RendererError> {
    let required = map_rhi_buffer_usage(required);
    if !actual.contains(required) {
        return Err(RendererError::RenderGraphValidation(format!(
            "imported buffer '{}' RHI usage does not satisfy render graph usage",
            imported.label
        )));
    }
    Ok(())
}

fn graph_texture_usage_from_accesses(
    accesses: &[CompiledResourceAccess],
    texture: GraphTexture,
) -> GraphTextureUsage {
    let mut usage = GraphTextureUsage::empty();
    for access in accesses {
        if access.resource != GraphResource::Texture(texture) {
            continue;
        }
        usage = usage
            | match access.access {
                GraphAccess::TextureRead(read) => texture_read_flag(read),
                GraphAccess::TextureWrite(write) => texture_write_flag(write),
                GraphAccess::ColorAttachment(_) | GraphAccess::DepthAttachment(_) => {
                    GraphTextureUsage::RENDER_ATTACHMENT
                }
                GraphAccess::BufferRead(_) | GraphAccess::BufferWrite(_) => {
                    GraphTextureUsage::empty()
                }
            };
    }
    usage
}

fn graph_buffer_usage_from_accesses(
    accesses: &[CompiledResourceAccess],
    buffer: GraphBuffer,
) -> GraphBufferUsage {
    let mut usage = GraphBufferUsage::empty();
    for access in accesses {
        if access.resource != GraphResource::Buffer(buffer) {
            continue;
        }
        usage = usage
            | match access.access {
                GraphAccess::BufferRead(read) => buffer_read_flag(read),
                GraphAccess::BufferWrite(write) => buffer_write_flag(write),
                GraphAccess::TextureRead(_)
                | GraphAccess::TextureWrite(_)
                | GraphAccess::ColorAttachment(_)
                | GraphAccess::DepthAttachment(_) => GraphBufferUsage::empty(),
            };
    }
    usage
}

fn buffer_read_flag(usage: BufferReadUsage) -> GraphBufferUsage {
    match usage {
        BufferReadUsage::Uniform => GraphBufferUsage::UNIFORM,
        BufferReadUsage::Storage => GraphBufferUsage::STORAGE,
        BufferReadUsage::Vertex => GraphBufferUsage::VERTEX,
        BufferReadUsage::Index => GraphBufferUsage::INDEX,
        BufferReadUsage::Indirect => GraphBufferUsage::INDIRECT,
        BufferReadUsage::CopySrc => GraphBufferUsage::COPY_SRC,
    }
}

fn buffer_write_flag(usage: BufferWriteUsage) -> GraphBufferUsage {
    match usage {
        BufferWriteUsage::Storage => GraphBufferUsage::STORAGE,
        BufferWriteUsage::CopyDst => GraphBufferUsage::COPY_DST,
    }
}

fn compile_resource_barriers(accesses: &[CompiledResourceAccess]) -> Vec<ResourceBarrier> {
    let mut previous: HashMap<GraphResource, (PassId, GraphAccess)> = HashMap::new();
    let mut barriers = Vec::new();
    for access in accesses {
        let prior = previous.insert(access.resource, (access.pass, access.access));
        barriers.push(ResourceBarrier {
            resource: access.resource,
            from_pass: prior.map(|(pass, _)| pass),
            to_pass: access.pass,
            before: prior.map(|(_, access)| access),
            after: access.access,
        });
    }
    barriers
}

fn compile_alias_allocations(lifetimes: &[ResourceLifetime]) -> Vec<AliasAllocation> {
    #[derive(Clone, Copy)]
    struct Slot {
        last_pass: PassId,
        bytes: u64,
    }

    let mut ordered = lifetimes
        .iter()
        .filter(|lifetime| lifetime.bytes > 0)
        .collect::<Vec<_>>();
    ordered.sort_by_key(|lifetime| (lifetime.first_pass.0, lifetime.last_pass.0));

    let mut slots: Vec<Slot> = Vec::new();
    let mut allocations = Vec::new();
    for lifetime in ordered {
        let mut slot_index = None;
        for (index, slot) in slots.iter_mut().enumerate() {
            if slot.last_pass.0 < lifetime.first_pass.0 {
                slot.last_pass = lifetime.last_pass;
                slot.bytes = slot.bytes.max(lifetime.bytes);
                slot_index = Some(index as u32);
                break;
            }
        }
        let slot = slot_index.unwrap_or_else(|| {
            let index = slots.len() as u32;
            slots.push(Slot {
                last_pass: lifetime.last_pass,
                bytes: lifetime.bytes,
            });
            index
        });
        allocations.push(AliasAllocation {
            resource: lifetime.resource,
            slot,
            offset: 0,
            bytes: lifetime.bytes,
        });
    }
    allocations.sort_by_key(|allocation| match allocation.resource {
        GraphResource::Texture(texture) => (0_u8, texture.0),
        GraphResource::Buffer(buffer) => (1_u8, buffer.0),
    });
    allocations
}

fn map_rhi_barrier(
    barrier: &ResourceBarrier,
    textures: &HashMap<GraphTexture, RhiTexture>,
    buffers: &HashMap<GraphBuffer, RhiBuffer>,
) -> Result<RhiResourceBarrierDesc, RendererError> {
    let resource = match barrier.resource {
        GraphResource::Texture(texture) => {
            let Some(texture) = textures.get(&texture) else {
                return Err(RendererError::RenderGraphValidation(format!(
                    "texture {:?} has no RHI resource mapping",
                    texture
                )));
            };
            RhiResource::Texture(*texture)
        }
        GraphResource::Buffer(buffer) => {
            let Some(buffer) = buffers.get(&buffer) else {
                return Err(RendererError::RenderGraphValidation(format!(
                    "buffer {:?} has no RHI resource mapping",
                    buffer
                )));
            };
            RhiResource::Buffer(*buffer)
        }
    };
    Ok(RhiResourceBarrierDesc {
        resource,
        before: barrier.before.map(map_rhi_access_state).transpose()?,
        after: map_rhi_access_state(barrier.after)?,
    })
}

fn map_rhi_access_state(access: GraphAccess) -> Result<RhiAccessState, RendererError> {
    match access {
        GraphAccess::TextureRead(TextureReadUsage::Sampled) => Ok(RhiAccessState::TextureSampled),
        GraphAccess::TextureRead(TextureReadUsage::Storage) => {
            Ok(RhiAccessState::TextureStorageRead)
        }
        GraphAccess::TextureRead(TextureReadUsage::CopySrc) => Ok(RhiAccessState::CopySrc),
        GraphAccess::TextureWrite(TextureWriteUsage::Storage) => {
            Ok(RhiAccessState::TextureStorageWrite)
        }
        GraphAccess::TextureWrite(TextureWriteUsage::CopyDst) => Ok(RhiAccessState::CopyDst),
        GraphAccess::ColorAttachment(_) | GraphAccess::DepthAttachment(_) => {
            Ok(RhiAccessState::RenderAttachment)
        }
        GraphAccess::BufferRead(BufferReadUsage::Uniform) => Ok(RhiAccessState::BufferUniform),
        GraphAccess::BufferRead(BufferReadUsage::Storage) => Ok(RhiAccessState::BufferStorageRead),
        GraphAccess::BufferRead(BufferReadUsage::Vertex) => Ok(RhiAccessState::BufferVertex),
        GraphAccess::BufferRead(BufferReadUsage::Index) => Ok(RhiAccessState::BufferIndex),
        GraphAccess::BufferRead(BufferReadUsage::Indirect) => Ok(RhiAccessState::BufferIndirect),
        GraphAccess::BufferRead(BufferReadUsage::CopySrc) => Ok(RhiAccessState::CopySrc),
        GraphAccess::BufferWrite(BufferWriteUsage::Storage) => {
            Ok(RhiAccessState::BufferStorageWrite)
        }
        GraphAccess::BufferWrite(BufferWriteUsage::CopyDst) => Ok(RhiAccessState::CopyDst),
    }
}

fn peak_memory_bytes(lifetimes: &[ResourceLifetime]) -> u64 {
    let max_pass = lifetimes
        .iter()
        .map(|lifetime| lifetime.last_pass.0)
        .max()
        .unwrap_or(0);
    (0..=max_pass)
        .map(|pass| {
            lifetimes
                .iter()
                .filter(|lifetime| lifetime.first_pass.0 <= pass && lifetime.last_pass.0 >= pass)
                .map(|lifetime| lifetime.bytes)
                .sum()
        })
        .max()
        .unwrap_or(0)
}

impl RenderGraphStats {
    fn merge_execution(&mut self, execution: PassExecutionStats) {
        self.executed_callbacks = execution.executed_callbacks;
        self.render_passes = execution.render_passes;
        self.compute_passes = execution.compute_passes;
        self.pipeline_binds = execution.pipeline_binds;
        self.fullscreen_draws = execution.fullscreen_draws;
        self.compute_dispatches = execution.compute_dispatches;
        self.phase_draws = execution.phase_draws;
        self.debug_groups = execution.debug_groups;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PassRecord {
    label: String,
    queue: QueueType,
    accesses: Vec<ResourceAccess>,
    dependencies: Vec<PassId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResourceAccess {
    resource: ResourceUse,
    access: GraphAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResourceUse {
    Texture(GraphTexture),
    Buffer(GraphBuffer),
}

impl ResourceUse {
    fn graph_resource(self) -> GraphResource {
        match self {
            Self::Texture(texture) => GraphResource::Texture(texture),
            Self::Buffer(buffer) => GraphResource::Buffer(buffer),
        }
    }
}

fn declared_pass_resources(record: &PassRecord) -> Vec<GraphResource> {
    let mut resources = Vec::new();
    for access in &record.accesses {
        let resource = access.resource.graph_resource();
        if !resources.contains(&resource) {
            resources.push(resource);
        }
    }
    resources
}

type PassCallback =
    Box<dyn for<'ctx> FnOnce(&mut PassContext<'ctx>) -> Result<(), RendererError> + Send + 'static>;

struct PassNode {
    record: PassRecord,
    callback: Option<PassCallback>,
}

pub struct PassBuilder<'b, 'a> {
    graph: &'b mut RenderGraphBuilder<'a>,
    record: PassRecord,
}

impl<'b, 'a> PassBuilder<'b, 'a> {
    pub fn set_queue(&mut self, queue: QueueType) -> &mut Self {
        self.record.queue = queue;
        self
    }

    pub fn queue(mut self, queue: QueueType) -> Self {
        self.record.queue = queue;
        self
    }

    pub fn declare_read_texture(
        &mut self,
        texture: GraphTexture,
        usage: TextureReadUsage,
    ) -> &mut Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::TextureRead(usage),
        });
        self
    }

    pub fn read_texture(mut self, texture: GraphTexture, usage: TextureReadUsage) -> Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::TextureRead(usage),
        });
        self
    }

    pub fn declare_write_texture(
        &mut self,
        texture: GraphTexture,
        usage: TextureWriteUsage,
    ) -> &mut Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::TextureWrite(usage),
        });
        self
    }

    pub fn write_texture(mut self, texture: GraphTexture, usage: TextureWriteUsage) -> Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::TextureWrite(usage),
        });
        self
    }

    pub fn declare_read_buffer(
        &mut self,
        buffer: GraphBuffer,
        usage: BufferReadUsage,
    ) -> &mut Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Buffer(buffer),
            access: GraphAccess::BufferRead(usage),
        });
        self
    }

    pub fn read_buffer(mut self, buffer: GraphBuffer, usage: BufferReadUsage) -> Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Buffer(buffer),
            access: GraphAccess::BufferRead(usage),
        });
        self
    }

    pub fn declare_write_buffer(
        &mut self,
        buffer: GraphBuffer,
        usage: BufferWriteUsage,
    ) -> &mut Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Buffer(buffer),
            access: GraphAccess::BufferWrite(usage),
        });
        self
    }

    pub fn write_buffer(mut self, buffer: GraphBuffer, usage: BufferWriteUsage) -> Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Buffer(buffer),
            access: GraphAccess::BufferWrite(usage),
        });
        self
    }

    pub fn declare_color_attachment(
        &mut self,
        texture: GraphTexture,
        ops: ColorAttachmentOps,
    ) -> &mut Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::ColorAttachment(ops),
        });
        self
    }

    pub fn color_attachment(mut self, texture: GraphTexture, ops: ColorAttachmentOps) -> Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::ColorAttachment(ops),
        });
        self
    }

    pub fn declare_depth_attachment(
        &mut self,
        texture: GraphTexture,
        ops: DepthAttachmentOps,
    ) -> &mut Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::DepthAttachment(ops),
        });
        self
    }

    pub fn depth_attachment(mut self, texture: GraphTexture, ops: DepthAttachmentOps) -> Self {
        self.record.accesses.push(ResourceAccess {
            resource: ResourceUse::Texture(texture),
            access: GraphAccess::DepthAttachment(ops),
        });
        self
    }

    pub fn declare_dependency(&mut self, pass: PassId) -> &mut Self {
        self.record.dependencies.push(pass);
        self
    }

    pub fn depends_on(mut self, pass: PassId) -> Self {
        self.record.dependencies.push(pass);
        self
    }

    pub fn execute(
        self,
        callback: impl for<'ctx> FnOnce(&mut PassContext<'ctx>) -> Result<(), RendererError>
            + Send
            + 'static,
    ) -> PassId {
        let id = PassId(self.graph.next_pass);
        self.graph.next_pass += 1;
        self.graph.passes.push(PassNode {
            record: self.record,
            callback: Some(Box::new(callback)),
        });
        id
    }

    pub fn execute_node(mut self, node: impl RenderPassNode) -> PassId {
        node.setup(&mut self);
        self.execute(move |ctx| node.execute(ctx))
    }
}

pub struct PassContext<'a> {
    frame_index: u64,
    caps: &'a RendererCaps,
    view: Option<ViewInfo>,
    declared_resources: Vec<GraphResource>,
    declared_accesses: Vec<(GraphResource, GraphAccess)>,
    execution: Rc<RefCell<PassExecutionStats>>,
    rhi_encoder: Option<&'a mut dyn RhiCommandEncoder>,
    rhi_device: Option<&'a dyn RhiDevice>,
    rhi_textures: Option<&'a HashMap<GraphTexture, RhiTexture>>,
    rhi_buffers: Option<&'a HashMap<GraphBuffer, RhiBuffer>>,
    rhi_graphics_pipelines: Option<&'a HashMap<String, RhiGraphicsPipeline>>,
    rhi_compute_pipelines: Option<&'a HashMap<String, RhiComputePipeline>>,
}

impl<'a> PassContext<'a> {
    pub fn new(frame_index: u64, caps: &'a RendererCaps) -> Self {
        Self::new_with_view(frame_index, caps, None)
    }

    pub fn new_with_view(frame_index: u64, caps: &'a RendererCaps, view: Option<ViewInfo>) -> Self {
        Self::new_with_view_and_execution(
            frame_index,
            caps,
            view,
            Vec::new(),
            Rc::new(RefCell::new(PassExecutionStats::default())),
            None,
            None,
            None,
            None,
        )
    }

    fn new_with_view_and_execution(
        frame_index: u64,
        caps: &'a RendererCaps,
        view: Option<ViewInfo>,
        declared_resources: Vec<GraphResource>,
        execution: Rc<RefCell<PassExecutionStats>>,
        rhi_encoder: Option<&'a mut dyn RhiCommandEncoder>,
        rhi_device: Option<&'a dyn RhiDevice>,
        rhi_textures: Option<&'a HashMap<GraphTexture, RhiTexture>>,
        rhi_buffers: Option<&'a HashMap<GraphBuffer, RhiBuffer>>,
    ) -> Self {
        Self {
            frame_index,
            caps,
            view,
            declared_resources,
            declared_accesses: Vec::new(),
            execution,
            rhi_encoder,
            rhi_device,
            rhi_textures,
            rhi_buffers,
            rhi_graphics_pipelines: None,
            rhi_compute_pipelines: None,
        }
    }

    pub(crate) fn set_declared_accesses(
        &mut self,
        accesses: Vec<(GraphResource, GraphAccess)>,
    ) {
        self.declared_accesses = accesses;
    }

    pub(crate) fn set_rhi_graphics_pipelines(
        &mut self,
        pipelines: &'a HashMap<String, RhiGraphicsPipeline>,
    ) {
        self.rhi_graphics_pipelines = Some(pipelines);
    }

    pub(crate) fn set_rhi_compute_pipelines(
        &mut self,
        pipelines: &'a HashMap<String, RhiComputePipeline>,
    ) {
        self.rhi_compute_pipelines = Some(pipelines);
    }

    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }

    pub fn renderer_caps(&self) -> &RendererCaps {
        self.caps
    }

    pub fn view(&self) -> Option<ViewInfo> {
        self.view.clone()
    }

    pub fn rhi_device(&self) -> Result<&dyn RhiDevice, RendererError> {
        self.rhi_device.ok_or_else(|| {
            RendererError::Validation("PassContext has no RHI device access".to_owned())
        })
    }

    pub fn texture(&self, texture: GraphTexture) -> Result<TextureViewRef<'_>, RendererError> {
        let resource = GraphResource::Texture(texture);
        if !self.declared_resources.contains(&resource) {
            return Err(RendererError::RenderGraphValidation(format!(
                "pass attempted to access undeclared texture {:?}",
                texture
            )));
        }
        Ok(TextureViewRef {
            id: texture,
            _marker: PhantomData,
        })
    }

    pub fn try_texture(&self, texture: GraphTexture) -> Result<TextureViewRef<'_>, RendererError> {
        self.texture(texture)
    }

    pub fn rhi_texture(&self, texture: GraphTexture) -> Result<RhiTexture, RendererError> {
        self.texture(texture)?;
        let Some(textures) = self.rhi_textures else {
            return Err(RendererError::Validation(
                "PassContext has no RHI texture mappings".to_owned(),
            ));
        };
        textures.get(&texture).copied().ok_or_else(|| {
            RendererError::RenderGraphValidation(format!(
                "texture {:?} has no RHI resource mapping",
                texture
            ))
        })
    }

    pub fn buffer(&self, buffer: GraphBuffer) -> Result<BufferRef<'_>, RendererError> {
        let resource = GraphResource::Buffer(buffer);
        if !self.declared_resources.contains(&resource) {
            return Err(RendererError::RenderGraphValidation(format!(
                "pass attempted to access undeclared buffer {:?}",
                buffer
            )));
        }
        Ok(BufferRef {
            id: buffer,
            _marker: PhantomData,
        })
    }

    pub fn try_buffer(&self, buffer: GraphBuffer) -> Result<BufferRef<'_>, RendererError> {
        self.buffer(buffer)
    }

    pub fn rhi_buffer(&self, buffer: GraphBuffer) -> Result<RhiBuffer, RendererError> {
        self.buffer(buffer)?;
        let Some(buffers) = self.rhi_buffers else {
            return Err(RendererError::Validation(
                "PassContext has no RHI buffer mappings".to_owned(),
            ));
        };
        buffers.get(&buffer).copied().ok_or_else(|| {
            RendererError::RenderGraphValidation(format!(
                "buffer {:?} has no RHI resource mapping",
                buffer
            ))
        })
    }

    pub fn draw_render_phase(&mut self, _phase: crate::RenderPhaseId) -> Result<(), RendererError> {
        self.execution.borrow_mut().phase_draws += 1;
        Ok(())
    }

    pub fn pipeline(&self, label: impl Into<String>) -> Result<GraphPipelineRef, RendererError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(RendererError::Validation(
                "graph pipeline label must not be empty".to_owned(),
            ));
        }
        Ok(GraphPipelineRef { label })
    }

    pub fn rhi_graphics_pipeline(
        &self,
        name: &str,
    ) -> Result<RhiGraphicsPipeline, RendererError> {
        let Some(pipelines) = self.rhi_graphics_pipelines else {
            return Err(RendererError::Validation(
                "PassContext has no RHI graphics pipeline registry".to_owned(),
            ));
        };
        pipelines.get(name).copied().ok_or_else(|| {
            RendererError::RenderGraphValidation(format!(
                "no RHI graphics pipeline registered for '{}'",
                name
            ))
        })
    }

    pub fn rhi_compute_pipeline(
        &self,
        name: &str,
    ) -> Result<RhiComputePipeline, RendererError> {
        let Some(pipelines) = self.rhi_compute_pipelines else {
            return Err(RendererError::Validation(
                "PassContext has no RHI compute pipeline registry".to_owned(),
            ));
        };
        pipelines.get(name).copied().ok_or_else(|| {
            RendererError::RenderGraphValidation(format!(
                "no RHI compute pipeline registered for '{}'",
                name
            ))
        })
    }

    fn color_attachment_graph_texture(&self) -> Option<GraphTexture> {
        for (resource, access) in &self.declared_accesses {
            if let (GraphResource::Texture(tex), GraphAccess::ColorAttachment(_)) = (resource, access)
            {
                return Some(*tex);
            }
        }
        None
    }

    fn depth_attachment_graph_texture(&self) -> Option<GraphTexture> {
        for (resource, access) in &self.declared_accesses {
            if let (GraphResource::Texture(tex), GraphAccess::DepthAttachment(_)) = (resource, access)
            {
                return Some(*tex);
            }
        }
        None
    }

    fn resolve_color_target(&self) -> Option<RhiTexture> {
        let tex = self.color_attachment_graph_texture()?;
        self.rhi_textures.and_then(|map| map.get(&tex).copied())
    }

    fn resolve_depth_target(&self) -> Option<RhiTexture> {
        let tex = self.depth_attachment_graph_texture()?;
        self.rhi_textures.and_then(|map| map.get(&tex).copied())
    }

    pub fn begin_render_pass<'enc>(&'enc mut self, desc: RenderPassDesc) -> RenderPassEncoder<'enc>
    where
        'a: 'enc,
    {
        self.execution.borrow_mut().render_passes += 1;
        let color_target = self.resolve_color_target();
        let depth_target = self.resolve_depth_target();
        RenderPassEncoder {
            label: desc.label,
            execution: Rc::clone(&self.execution),
            rhi_encoder: self.rhi_encoder.as_deref_mut().map(|encoder| {
                encoder as *mut (dyn RhiCommandEncoder + 'enc)
            }),
            rhi_graphics_pipelines: self
                .rhi_graphics_pipelines
                .map(|pipelines| pipelines as *const _),
            color_target,
            depth_target,
            _current_pipeline_label: None,
            _marker: PhantomData,
        }
    }

    pub fn begin_compute_pass<'enc>(&'enc mut self, desc: ComputePassDesc) -> ComputePassEncoder<'enc>
    where
        'a: 'enc,
    {
        self.execution.borrow_mut().compute_passes += 1;
        ComputePassEncoder {
            label: desc.label,
            execution: Rc::clone(&self.execution),
            rhi_encoder: self.rhi_encoder.as_deref_mut().map(|encoder| {
                encoder as *mut (dyn RhiCommandEncoder + 'enc)
            }),
            rhi_compute_pipelines: self
                .rhi_compute_pipelines
                .map(|pipelines| pipelines as *const _),
            _current_pipeline_label: None,
            _marker: PhantomData,
        }
    }

    pub fn encode_rhi_compute_pass(
        &mut self,
        desc: &RhiComputePassDesc,
    ) -> Result<(), RendererError> {
        let Some(encoder) = self.rhi_encoder.as_deref_mut() else {
            return Err(RendererError::Validation(
                "PassContext has no RHI command encoder".to_owned(),
            ));
        };
        encoder.encode_compute_pass(desc)?;
        let mut execution = self.execution.borrow_mut();
        execution.compute_passes += 1;
        execution.compute_dispatches += 1;
        Ok(())
    }

    pub fn encode_rhi_render_pass(
        &mut self,
        desc: &RhiRenderPassDesc,
    ) -> Result<(), RendererError> {
        let Some(encoder) = self.rhi_encoder.as_deref_mut() else {
            return Err(RendererError::Validation(
                "PassContext has no RHI command encoder".to_owned(),
            ));
        };
        encoder.encode_render_pass(desc)?;
        self.execution.borrow_mut().render_passes += 1;
        Ok(())
    }

    pub fn encode_rhi_indirect_render_pass(
        &mut self,
        desc: &RhiIndirectRenderPassDesc,
    ) -> Result<(), RendererError> {
        let Some(encoder) = self.rhi_encoder.as_deref_mut() else {
            return Err(RendererError::Validation(
                "PassContext has no RHI command encoder".to_owned(),
            ));
        };
        encoder.encode_indirect_render_pass(desc)?;
        self.execution.borrow_mut().render_passes += 1;
        Ok(())
    }

    pub fn encode_rhi_indexed_indirect_render_pass(
        &mut self,
        desc: &RhiIndexedIndirectRenderPassDesc,
    ) -> Result<(), RendererError> {
        let Some(encoder) = self.rhi_encoder.as_deref_mut() else {
            return Err(RendererError::Validation(
                "PassContext has no RHI command encoder".to_owned(),
            ));
        };
        encoder.encode_indexed_indirect_render_pass(desc)?;
        self.execution.borrow_mut().render_passes += 1;
        Ok(())
    }

    pub fn push_debug_group(&mut self, _label: &str) {
        if let Some(encoder) = self.rhi_encoder.as_deref_mut() {
            let _ = encoder.push_debug_group(_label);
        }
        self.execution.borrow_mut().debug_groups += 1;
    }

    pub fn pop_debug_group(&mut self) {
        if let Some(encoder) = self.rhi_encoder.as_deref_mut() {
            let _ = encoder.pop_debug_group();
        }
    }

    fn record_callback(&mut self) {
        self.execution.borrow_mut().executed_callbacks += 1;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PassExecutionStats {
    executed_callbacks: u32,
    render_passes: u32,
    compute_passes: u32,
    pipeline_binds: u32,
    fullscreen_draws: u32,
    compute_dispatches: u32,
    phase_draws: u32,
    debug_groups: u32,
}

pub struct TextureViewRef<'a> {
    pub id: GraphTexture,
    _marker: PhantomData<&'a ()>,
}

pub struct BufferRef<'a> {
    pub id: GraphBuffer,
    _marker: PhantomData<&'a ()>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderPassDesc {
    pub label: Option<String>,
}

impl RenderPassDesc {
    pub fn label(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputePassDesc {
    pub label: Option<String>,
}

impl ComputePassDesc {
    pub fn label(label: impl Into<String>) -> Self {
        Self {
            label: Some(label.into()),
        }
    }
}

pub struct RenderPassEncoder<'enc> {
    pub label: Option<String>,
    execution: Rc<RefCell<PassExecutionStats>>,
    rhi_encoder: Option<*mut (dyn RhiCommandEncoder + 'enc)>,
    rhi_graphics_pipelines: Option<*const HashMap<String, RhiGraphicsPipeline>>,
    color_target: Option<RhiTexture>,
    depth_target: Option<RhiTexture>,
    _current_pipeline_label: Option<String>,
    _marker: PhantomData<&'enc mut dyn RhiCommandEncoder>,
}

impl<'enc> RenderPassEncoder<'enc> {
    pub fn set_pipeline(&mut self, pipeline: GraphPipelineRef) {
        self.execution.borrow_mut().pipeline_binds += 1;
        self._current_pipeline_label = Some(pipeline.label);
    }

    pub fn draw_fullscreen_triangle(&mut self) {
        self.execution.borrow_mut().fullscreen_draws += 1;
        let pipeline = self
            ._current_pipeline_label
            .as_ref()
            .and_then(|label| unsafe {
                self.rhi_graphics_pipelines
                    .and_then(|map| (*map).get(label.as_str()).copied())
            });
        let color_target = self.color_target;
        let Some(encoder) = self.rhi_encoder else {
            return;
        };
        if let (Some(pipeline), Some(color_target)) = (pipeline, color_target) {
            let encoder = unsafe { &mut *encoder };
            let _ = encoder.encode_render_pass(&RhiRenderPassDesc {
                label: self.label.clone(),
                pipeline,
                color_target: Some(color_target),
                depth_target: self.depth_target,
                vertex_buffers: Vec::new(),
                index_buffer: None,
                bind_groups: Vec::new(),
                vertex_count: 3,
                index_count: None,
                instance_count: 1,
            });
        }
    }
}

pub struct ComputePassEncoder<'enc> {
    pub label: Option<String>,
    execution: Rc<RefCell<PassExecutionStats>>,
    rhi_encoder: Option<*mut (dyn RhiCommandEncoder + 'enc)>,
    rhi_compute_pipelines: Option<*const HashMap<String, RhiComputePipeline>>,
    _current_pipeline_label: Option<String>,
    _marker: PhantomData<&'enc mut dyn RhiCommandEncoder>,
}

impl<'enc> ComputePassEncoder<'enc> {
    pub fn set_pipeline(&mut self, pipeline: GraphPipelineRef) {
        self.execution.borrow_mut().pipeline_binds += 1;
        self._current_pipeline_label = Some(pipeline.label);
    }

    pub fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) {
        self.execution.borrow_mut().compute_dispatches += 1;
        let pipeline = self
            ._current_pipeline_label
            .as_ref()
            .and_then(|label| unsafe {
                self.rhi_compute_pipelines
                    .and_then(|map| (*map).get(label.as_str()).copied())
            });
        let Some(encoder) = self.rhi_encoder else {
            return;
        };
        if let Some(pipeline) = pipeline {
            let encoder = unsafe { &mut *encoder };
            let _ = encoder.encode_compute_pass(&RhiComputePassDesc {
                label: self.label.clone(),
                pipeline,
                bind_groups: Vec::new(),
                workgroups: [x, y, z],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU64;

    use crate::{
        rhi::HeadlessRhiDevice, BufferDesc, BufferTag, BufferUsage, Handle, RendererCaps,
        TextureDesc, TextureDimension, TextureFormat, TextureTag, TextureUsage,
    };

    #[test]
    fn builder_tracks_declared_pass_resources() {
        let mut graph = RenderGraphBuilder::default();
        let color = graph.create_texture(GraphTextureDesc {
            label: Some("color".to_owned()),
            width: 1280,
            height: 720,
            format: TextureFormat::Rgba16Float,
        });
        graph
            .add_pass("tonemap")
            .queue(QueueType::Graphics)
            .read_texture(color, TextureReadUsage::Sampled)
            .color_attachment(color, ColorAttachmentOps::load_store())
            .execute(|_| Ok(()));

        let stats = graph.stats();
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.pass_labels, vec!["tonemap".to_owned()]);
        assert_eq!(stats.transient_textures, 1);
        assert_eq!(stats.barriers, 2);

        let ctx = RenderGraphExtensionContext::new(color, color, RendererCaps::default());
        assert_eq!(ctx.main_color(), color);
    }

    #[test]
    fn builder_can_create_transients_from_renderer_resource_descs() {
        let mut graph = RenderGraphBuilder::default();
        let color = graph.create_texture_from_desc(
            "hdr_color",
            TextureDesc {
                label: Some("ignored_texture_label"),
                dimension: TextureDimension::D2,
                width: 1920,
                height: 1080,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsage::RENDER_TARGET | TextureUsage::SAMPLED,
                initial_data: None,
            },
        );
        let constants = graph.create_buffer_from_desc(
            "view_constants",
            BufferDesc {
                label: Some("ignored_buffer_label"),
                size: 256,
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                initial_data: None,
            },
        );

        graph
            .add_pass("use_transients")
            .read_texture(color, TextureReadUsage::Sampled)
            .color_attachment(color, ColorAttachmentOps::load_store())
            .read_buffer(constants, BufferReadUsage::Uniform)
            .execute(|_| Ok(()));

        let compiled = graph.compile().unwrap();
        assert_eq!(compiled.stats.transient_textures, 1);
        assert_eq!(compiled.stats.transient_buffers, 1);
        assert_eq!(compiled.resource_lifetimes.len(), 2);
        assert_eq!(compiled.resource_accesses.len(), 3);
    }

    #[test]
    fn pass_context_validates_declared_resource_access() {
        let mut graph = RenderGraphBuilder::default();
        let declared_texture = graph.create_texture(GraphTextureDesc {
            label: Some("declared_texture".to_owned()),
            width: 16,
            height: 16,
            format: TextureFormat::Rgba16Float,
        });
        let undeclared_texture = graph.create_texture(GraphTextureDesc {
            label: Some("undeclared_texture".to_owned()),
            width: 16,
            height: 16,
            format: TextureFormat::Rgba16Float,
        });
        let declared_buffer = graph.create_buffer(GraphBufferDesc {
            label: Some("declared_buffer".to_owned()),
            size: 64,
        });
        let undeclared_buffer = graph.create_buffer(GraphBufferDesc {
            label: Some("undeclared_buffer".to_owned()),
            size: 64,
        });

        graph
            .add_pass("validate_declared_access")
            .read_texture(declared_texture, TextureReadUsage::Sampled)
            .read_buffer(declared_buffer, BufferReadUsage::Uniform)
            .execute(move |ctx| {
                assert_eq!(ctx.texture(declared_texture)?.id, declared_texture);
                assert_eq!(ctx.buffer(declared_buffer)?.id, declared_buffer);
                assert_eq!(ctx.try_texture(declared_texture)?.id, declared_texture);
                assert_eq!(ctx.try_buffer(declared_buffer)?.id, declared_buffer);
                assert!(matches!(
                    ctx.texture(undeclared_texture),
                    Err(RendererError::RenderGraphValidation(_))
                ));
                assert!(matches!(
                    ctx.buffer(undeclared_buffer),
                    Err(RendererError::RenderGraphValidation(_))
                ));
                assert!(matches!(
                    ctx.rhi_texture(declared_texture),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    ctx.rhi_buffer(declared_buffer),
                    Err(RendererError::Validation(_))
                ));
                Ok(())
            });

        let stats = graph.execute(0, &RendererCaps::default()).unwrap();
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[test]
    fn render_pass_node_can_setup_resources_and_execute() {
        struct FullscreenNode {
            source: GraphTexture,
            output: GraphTexture,
            constants: GraphBuffer,
        }

        impl RenderPassNode for FullscreenNode {
            fn setup(&self, builder: &mut PassBuilder<'_, '_>) {
                builder
                    .set_queue(QueueType::Graphics)
                    .declare_read_texture(self.source, TextureReadUsage::Sampled)
                    .declare_color_attachment(self.output, ColorAttachmentOps::load_store())
                    .declare_read_buffer(self.constants, BufferReadUsage::Uniform);
            }

            fn execute(&self, ctx: &mut PassContext<'_>) -> Result<(), RendererError> {
                let pipeline = ctx.pipeline("fullscreen_node")?;
                let mut pass = ctx.begin_render_pass(RenderPassDesc::label("fullscreen_node"));
                pass.set_pipeline(pipeline);
                pass.draw_fullscreen_triangle();
                Ok(())
            }
        }

        let mut graph = RenderGraphBuilder::default();
        let source = graph.create_texture(GraphTextureDesc {
            label: Some("source".to_owned()),
            width: 320,
            height: 180,
            format: TextureFormat::Rgba16Float,
        });
        let output = graph.create_texture(GraphTextureDesc {
            label: Some("output".to_owned()),
            width: 320,
            height: 180,
            format: TextureFormat::Rgba16Float,
        });
        let constants = graph.create_buffer(GraphBufferDesc {
            label: Some("constants".to_owned()),
            size: 64,
        });

        graph
            .add_pass("fullscreen_node")
            .execute_node(FullscreenNode {
                source,
                output,
                constants,
            });

        let stats = graph.execute(1, &RendererCaps::default()).unwrap();
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.executed_callbacks, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.pipeline_binds, 1);
        assert_eq!(stats.fullscreen_draws, 1);
        assert_eq!(stats.barriers, 3);
    }

    #[test]
    fn graph_resource_usage_flags_support_bitflag_queries() {
        assert!(GraphTextureUsage::empty().is_empty());
        assert!(GraphTextureUsage::empty().contains(GraphTextureUsage::empty()));
        assert!((GraphTextureUsage::SAMPLED | GraphTextureUsage::COPY_DST)
            .contains(GraphTextureUsage::SAMPLED));
        assert!(!(GraphTextureUsage::SAMPLED | GraphTextureUsage::COPY_DST)
            .contains(GraphTextureUsage::RENDER_ATTACHMENT));

        assert!(GraphBufferUsage::empty().is_empty());
        assert!(GraphBufferUsage::empty().contains(GraphBufferUsage::empty()));
        assert!((GraphBufferUsage::UNIFORM | GraphBufferUsage::COPY_DST)
            .contains(GraphBufferUsage::COPY_DST));
        assert!(!(GraphBufferUsage::UNIFORM | GraphBufferUsage::COPY_DST)
            .contains(GraphBufferUsage::VERTEX));
        assert!((GraphBufferUsage::STORAGE | GraphBufferUsage::INDIRECT)
            .contains(GraphBufferUsage::INDIRECT));
    }

    #[test]
    fn builder_imports_external_texture_and_buffer_handles() {
        let mut graph = RenderGraphBuilder::default();
        let texture = Handle::<TextureTag>::from_raw(NonZeroU64::new(1).unwrap());
        let buffer = Handle::<BufferTag>::from_raw(NonZeroU64::new(2).unwrap());
        let imported_texture = graph.import_texture(
            "history",
            texture,
            GraphTextureUsage::SAMPLED | GraphTextureUsage::COPY_DST,
        );
        let imported_buffer = graph.import_buffer(
            "camera",
            buffer,
            GraphBufferUsage::UNIFORM | GraphBufferUsage::COPY_DST,
        );

        graph
            .add_pass("reuse_imports")
            .read_texture(imported_texture, TextureReadUsage::Sampled)
            .write_texture(imported_texture, TextureWriteUsage::CopyDst)
            .read_buffer(imported_buffer, BufferReadUsage::Uniform)
            .write_buffer(imported_buffer, BufferWriteUsage::CopyDst)
            .execute(|_| Ok(()));

        let compiled = graph.compile().unwrap();
        assert_eq!(compiled.stats.transient_textures, 0);
        assert_eq!(compiled.stats.transient_buffers, 0);
        assert_eq!(compiled.stats.barriers, 4);
        assert_eq!(compiled.resource_lifetimes.len(), 2);
        assert_eq!(compiled.resource_accesses.len(), 4);
        assert_eq!(compiled.barriers.len(), 4);

        let mut invalid = RenderGraphBuilder::default();
        let sampled_only =
            invalid.import_texture("sampled_only", texture, GraphTextureUsage::SAMPLED);
        invalid
            .add_pass("bad_usage")
            .write_texture(sampled_only, TextureWriteUsage::CopyDst)
            .execute(|_| Ok(()));
        assert!(matches!(
            invalid.compile(),
            Err(RendererError::RenderGraphValidation(_))
        ));
    }

    #[test]
    fn graph_execute_on_rhi_maps_imported_resources() {
        let device = HeadlessRhiDevice::new();
        let texture = Handle::<TextureTag>::from_raw(NonZeroU64::new(10).unwrap());
        let buffer = Handle::<BufferTag>::from_raw(NonZeroU64::new(11).unwrap());
        let rhi_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("history".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::SAMPLED | RhiTextureUsage::COPY_DST,
            })
            .unwrap();
        let rhi_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("camera".to_owned()),
                size: 64,
                usage: RhiBufferUsage::UNIFORM | RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        let imports = RhiResourceImports::new()
            .with_texture(texture, rhi_texture)
            .with_buffer(buffer, rhi_buffer);

        let mut graph = RenderGraphBuilder::default();
        let imported_texture = graph.import_texture(
            "history",
            texture,
            GraphTextureUsage::SAMPLED | GraphTextureUsage::COPY_DST,
        );
        let imported_buffer = graph.import_buffer(
            "camera",
            buffer,
            GraphBufferUsage::UNIFORM | GraphBufferUsage::COPY_DST,
        );
        graph
            .add_pass("reuse_imports")
            .read_texture(imported_texture, TextureReadUsage::Sampled)
            .write_texture(imported_texture, TextureWriteUsage::CopyDst)
            .read_buffer(imported_buffer, BufferReadUsage::Uniform)
            .write_buffer(imported_buffer, BufferWriteUsage::CopyDst)
            .execute(|_| Ok(()));

        let stats = graph
            .execute_on_rhi_with_imports(5, &RendererCaps::default(), None, &device, &imports)
            .unwrap();
        let rhi_stats = device.stats();

        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.executed_callbacks, 1);
        assert_eq!(stats.barriers, 4);
        assert_eq!(rhi_stats.encoded_barriers, 4);
        assert_eq!(rhi_stats.finished_command_buffers, 1);
        assert_eq!(rhi_stats.submitted_command_buffers, 1);

        let mut missing_import = RenderGraphBuilder::default();
        let missing_texture = missing_import.import_texture(
            "missing",
            texture,
            GraphTextureUsage::SAMPLED | GraphTextureUsage::COPY_DST,
        );
        missing_import
            .add_pass("missing_import")
            .read_texture(missing_texture, TextureReadUsage::Sampled)
            .execute(|_| Ok(()));
        assert!(matches!(
            missing_import.execute_on_rhi_with_imports(
                5,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::default()
            ),
            Err(RendererError::RenderGraphValidation(_))
        ));

        let mismatched_texture = device
            .create_texture(&RhiTextureDesc {
                label: Some("mismatched_history".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: RhiTextureUsage::RENDER_ATTACHMENT,
            })
            .unwrap();
        let mut mismatched_texture_graph = RenderGraphBuilder::default();
        let imported_texture = mismatched_texture_graph.import_texture(
            "mismatched_history",
            texture,
            GraphTextureUsage::SAMPLED | GraphTextureUsage::COPY_DST,
        );
        mismatched_texture_graph
            .add_pass("mismatched_texture")
            .read_texture(imported_texture, TextureReadUsage::Sampled)
            .write_texture(imported_texture, TextureWriteUsage::CopyDst)
            .execute(|_| Ok(()));
        assert!(matches!(
            mismatched_texture_graph.execute_on_rhi_with_imports(
                5,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(texture, mismatched_texture)
            ),
            Err(RendererError::RenderGraphValidation(_))
        ));

        let mismatched_buffer = device
            .create_buffer(&RhiBufferDesc {
                label: Some("mismatched_camera".to_owned()),
                size: 64,
                usage: RhiBufferUsage::COPY_SRC,
            })
            .unwrap();
        let mut mismatched_buffer_graph = RenderGraphBuilder::default();
        let imported_buffer = mismatched_buffer_graph.import_buffer(
            "mismatched_camera",
            buffer,
            GraphBufferUsage::UNIFORM | GraphBufferUsage::COPY_DST,
        );
        mismatched_buffer_graph
            .add_pass("mismatched_buffer")
            .read_buffer(imported_buffer, BufferReadUsage::Uniform)
            .write_buffer(imported_buffer, BufferWriteUsage::CopyDst)
            .execute(|_| Ok(()));
        assert!(matches!(
            mismatched_buffer_graph.execute_on_rhi_with_imports(
                5,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_buffer(buffer, mismatched_buffer)
            ),
            Err(RendererError::RenderGraphValidation(_))
        ));
    }

    #[test]
    fn compile_derives_indirect_buffer_usage_for_draw_arguments() {
        let mut graph = RenderGraphBuilder::default();
        let indirect_args = graph.create_buffer(GraphBufferDesc {
            label: Some("gpu_indirect_args".to_owned()),
            size: 16,
        });
        graph
            .add_pass("build_indirect_args")
            .write_buffer(indirect_args, BufferWriteUsage::Storage)
            .execute(|_| Ok(()));
        graph
            .add_pass("draw_indirect")
            .read_buffer(indirect_args, BufferReadUsage::Indirect)
            .execute(|_| Ok(()));

        let device = HeadlessRhiDevice::new();
        let stats = graph
            .execute_on_rhi(7, &RendererCaps::default(), None, &device)
            .unwrap();
        let rhi_stats = device.stats();

        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.barriers, 2);
        assert_eq!(rhi_stats.indirect_buffers, 1);
        assert_eq!(rhi_stats.storage_buffers, 1);
        assert_eq!(rhi_stats.encoded_barriers, 2);
    }

    #[test]
    fn pass_context_exposes_optional_view_info() {
        let mut graph = RenderGraphBuilder::default();
        graph.add_pass("inspect_view").execute(|ctx| {
            let view = ctx.view().expect("view info is attached");
            assert_eq!(view.label.as_deref(), Some("main"));
            assert_eq!(view.render_path, crate::RenderPath::ForwardPlus);
            assert_eq!(ctx.frame_index(), 42);
            Ok(())
        });
        let view = ViewInfo {
            label: Some("main".to_owned()),
            scene: Handle::<crate::SceneTag>::from_raw(NonZeroU64::new(3).unwrap()),
            render_path: crate::RenderPath::ForwardPlus,
            layers: crate::RenderLayerMask::single(crate::RenderLayer(2)),
        };
        assert_eq!(
            graph
                .execute_with_view(42, &RendererCaps::default(), Some(view))
                .unwrap()
                .executed_callbacks,
            1
        );
    }

    #[test]
    fn execute_options_can_wrap_passes_in_debug_groups() {
        let mut graph = RenderGraphBuilder::default();
        graph.add_pass("first").execute(|_| Ok(()));
        graph.add_pass("second").execute(|_| Ok(()));
        let stats = graph
            .execute_with_view_options(1, &RendererCaps::default(), None, true, true)
            .unwrap();
        assert_eq!(stats.executed_callbacks, 2);
        assert_eq!(stats.debug_groups, 2);

        let mut graph = RenderGraphBuilder::default();
        graph.add_pass("first").execute(|ctx| {
            ctx.push_debug_group("manual");
            ctx.pop_debug_group();
            Ok(())
        });
        let stats = graph
            .execute_with_view_options(1, &RendererCaps::default(), None, true, false)
            .unwrap();
        assert_eq!(stats.debug_groups, 1);
    }

    #[test]
    fn compile_derives_resource_lifetimes_and_aliasing() {
        let mut graph = RenderGraphBuilder::default();
        let a = graph.create_texture(GraphTextureDesc {
            label: Some("a".to_owned()),
            width: 4,
            height: 4,
            format: TextureFormat::Rgba8Unorm,
        });
        let b = graph.create_texture(GraphTextureDesc {
            label: Some("b".to_owned()),
            width: 4,
            height: 4,
            format: TextureFormat::Rgba8Unorm,
        });
        graph
            .add_pass("write_a")
            .color_attachment(a, ColorAttachmentOps::clear_store())
            .execute(|_| Ok(()));
        let second = graph
            .add_pass("read_a")
            .read_texture(a, TextureReadUsage::Sampled)
            .execute(|_| Ok(()));
        graph
            .add_pass("write_b")
            .depends_on(second)
            .color_attachment(b, ColorAttachmentOps::clear_store())
            .execute(|_| Ok(()));

        let compiled = graph.compile().unwrap();

        assert_eq!(compiled.passes.len(), 3);
        assert_eq!(
            compiled.stats.pass_labels,
            vec![
                "write_a".to_owned(),
                "read_a".to_owned(),
                "write_b".to_owned()
            ]
        );
        assert_eq!(compiled.resource_lifetimes.len(), 2);
        assert_eq!(compiled.resource_lifetimes[0].first_pass, PassId(0));
        assert_eq!(compiled.resource_lifetimes[0].last_pass, PassId(1));
        assert_eq!(compiled.resource_lifetimes[1].first_pass, PassId(2));
        assert_eq!(compiled.resource_lifetimes[1].last_pass, PassId(2));
        assert_eq!(compiled.stats.aliased_memory_bytes, 64);
        assert_eq!(compiled.resource_accesses.len(), 3);
        assert_eq!(compiled.barriers.len(), 3);
        assert_eq!(compiled.barriers[0].resource, GraphResource::Texture(a));
        assert_eq!(compiled.barriers[0].from_pass, None);
        assert_eq!(
            compiled.barriers[0].after,
            GraphAccess::ColorAttachment(ColorAttachmentOps::clear_store())
        );
        assert_eq!(compiled.barriers[1].from_pass, Some(PassId(0)));
        assert_eq!(
            compiled.barriers[1].before,
            Some(GraphAccess::ColorAttachment(
                ColorAttachmentOps::clear_store()
            ))
        );
        assert_eq!(
            compiled.barriers[1].after,
            GraphAccess::TextureRead(TextureReadUsage::Sampled)
        );
        assert_eq!(compiled.alias_allocations.len(), 2);
        assert_eq!(
            compiled.alias_allocations[0].slot,
            compiled.alias_allocations[1].slot
        );
        assert_eq!(compiled.alias_allocations[0].bytes, 64);

        let unaliased = graph.compile_with_transient_aliasing(false).unwrap();
        assert_eq!(unaliased.stats.aliased_memory_bytes, 0);
        assert!(unaliased.alias_allocations.is_empty());
        assert_eq!(unaliased.resource_lifetimes, compiled.resource_lifetimes);
        assert_eq!(unaliased.barriers, compiled.barriers);
    }

    #[test]
    fn graph_rejects_invalid_transient_resource_descriptors() {
        let mut graph = RenderGraphBuilder::default();
        let texture = graph.create_texture(GraphTextureDesc {
            label: Some("bad".to_owned()),
            width: 0,
            height: 4,
            format: TextureFormat::Rgba8Unorm,
        });
        graph
            .add_pass("bad_texture")
            .read_texture(texture, TextureReadUsage::Sampled)
            .execute(|_| Ok(()));
        assert!(matches!(
            graph.compile(),
            Err(RendererError::RenderGraphValidation(_))
        ));

        let mut graph = RenderGraphBuilder::default();
        let buffer = graph.create_buffer(GraphBufferDesc {
            label: Some("bad".to_owned()),
            size: 0,
        });
        graph
            .add_pass("bad_buffer")
            .read_buffer(buffer, BufferReadUsage::Uniform)
            .execute(|_| Ok(()));
        assert!(matches!(
            graph.compile(),
            Err(RendererError::RenderGraphValidation(_))
        ));
    }

    #[test]
    fn graph_rejects_empty_pass_labels() {
        let mut graph = RenderGraphBuilder::default();
        graph.add_pass("  ").execute(|_| Ok(()));
        assert!(matches!(
            graph.compile(),
            Err(RendererError::RenderGraphValidation(_))
        ));
    }

    #[test]
    fn graph_executes_pass_callbacks_in_order() {
        let mut graph = RenderGraphBuilder::default();
        graph.add_pass("first").execute(|ctx| {
            assert_eq!(ctx.frame_index(), 7);
            ctx.push_debug_group("first");
            ctx.pop_debug_group();
            Ok(())
        });
        let first = PassId(0);
        graph.add_pass("second").depends_on(first).execute(|ctx| {
            let mut pass = ctx.begin_render_pass(RenderPassDesc::label("second"));
            assert_eq!(pass.label.as_deref(), Some("second"));
            pass.draw_fullscreen_triangle();
            ctx.draw_render_phase(crate::RenderPhaseKind::ForwardOpaque)?;
            Ok(())
        });
        graph
            .add_pass("compute")
            .queue(QueueType::Compute)
            .depends_on(PassId(1))
            .execute(|ctx| {
                let mut pass = ctx.begin_compute_pass(ComputePassDesc::label("compute"));
                assert_eq!(pass.label.as_deref(), Some("compute"));
                pass.dispatch_workgroups(1, 1, 1);
                Ok(())
            });

        let stats = graph.execute(7, &RendererCaps::default()).unwrap();

        assert_eq!(stats.pass_count, 3);
        assert_eq!(stats.graphics_queue_passes, 2);
        assert_eq!(stats.compute_queue_passes, 1);
        assert_eq!(stats.async_compute_queue_passes, 0);
        assert_eq!(stats.copy_queue_passes, 0);
        assert_eq!(stats.executed_callbacks, 3);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.fullscreen_draws, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.phase_draws, 1);
        assert_eq!(stats.debug_groups, 1);
    }

    #[test]
    fn graph_tracks_and_validates_async_compute_queue_passes() {
        fn async_compute_graph() -> RenderGraphBuilder<'static> {
            let mut graph = RenderGraphBuilder::default();
            graph
                .add_pass("async_cull")
                .queue(QueueType::AsyncCompute)
                .execute(|ctx| {
                    let mut pass = ctx.begin_compute_pass(ComputePassDesc::label("async_cull"));
                    pass.dispatch_workgroups(1, 1, 1);
                    Ok(())
                });
            graph.add_pass("present").execute(|_| Ok(()));
            graph
        }

        let mut graph = async_compute_graph();
        let stats = graph.stats();
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.graphics_queue_passes, 1);
        assert_eq!(stats.async_compute_queue_passes, 1);
        assert_eq!(stats.compute_queue_passes, 0);

        let default_caps_result = graph.execute(0, &RendererCaps::default());
        if cfg!(feature = "async-compute") {
            assert!(default_caps_result.is_ok());
        } else {
            assert!(matches!(
                default_caps_result,
                Err(RendererError::UnsupportedFeature(
                    RendererFeature::AsyncCompute
                ))
            ));
        }

        let mut caps = RendererCaps::default();
        caps.features = caps.features | RendererFeatures::ASYNC_COMPUTE;
        let mut graph = async_compute_graph();
        let stats = graph.execute(0, &caps).unwrap();
        assert_eq!(stats.async_compute_queue_passes, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
    }

    #[test]
    fn graph_execute_on_rhi_allocates_transients_and_submits_pass_commands() {
        let device = HeadlessRhiDevice::new();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("shader".to_owned()),
                source: "@compute @workgroup_size(1) fn main() {}".to_owned(),
            })
            .unwrap();
        let graphics_pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graphics".to_owned()),
                vertex_shader: shader,
                vertex_entry: "main".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("main".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();
        let compute_pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("compute".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let indirect = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("indirect".to_owned()),
                size: 16,
                usage: crate::rhi::RhiBufferUsage::INDIRECT | crate::rhi::RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(indirect, 0, &[0; 16]).unwrap();
        let mut graph = RenderGraphBuilder::default();
        let color = graph.create_texture(GraphTextureDesc {
            label: Some("color".to_owned()),
            width: 4,
            height: 4,
            format: TextureFormat::Rgba8Unorm,
        });
        let constants = graph.create_buffer(GraphBufferDesc {
            label: Some("constants".to_owned()),
            size: 64,
        });
        let compute_output = graph.create_buffer(GraphBufferDesc {
            label: Some("compute_output".to_owned()),
            size: 16,
        });
        graph
            .add_pass("draw")
            .read_buffer(constants, BufferReadUsage::Uniform)
            .color_attachment(color, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                ctx.push_debug_group("draw");
                let color_target = ctx.rhi_texture(color)?;
                let _constants = ctx.rhi_buffer(constants)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("draw".to_owned()),
                    pipeline: graphics_pipeline,
                    color_target: Some(color_target),
                    depth_target: None,
                    vertex_buffers: Vec::new(),
                    index_buffer: None,
                    bind_groups: Vec::new(),
                    vertex_count: 3,
                    index_count: None,
                    instance_count: 1,
                })?;
                ctx.encode_rhi_indirect_render_pass(&crate::rhi::RhiIndirectRenderPassDesc {
                    label: Some("draw_indirect".to_owned()),
                    pipeline: graphics_pipeline,
                    color_target: Some(color_target),
                    depth_target: None,
                    vertex_buffers: Vec::new(),
                    bind_groups: Vec::new(),
                    indirect_buffer: indirect,
                    indirect_offset: 0,
                    draw_count: 1,
                    draw_stride: 16,
                })?;
                ctx.pop_debug_group();
                Ok(())
            });
        graph
            .add_pass("post")
            .read_texture(color, TextureReadUsage::Sampled)
            .write_buffer(compute_output, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                let compute_output = ctx.rhi_buffer(compute_output)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("post_compute_bind_group".to_owned()),
                        pipeline: compute_pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                            binding: 0,
                            buffer: compute_output,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("post".to_owned()),
                    pipeline: compute_pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });

        let stats = graph
            .execute_on_rhi(11, &RendererCaps::default(), None, &device)
            .unwrap();
        let rhi_stats = device.stats();

        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.executed_callbacks, 2);
        assert_eq!(stats.render_passes, 2);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.fullscreen_draws, 0);
        assert_eq!(stats.debug_groups, 1);
        assert_eq!(rhi_stats.textures, 1);
        assert_eq!(rhi_stats.sampled_textures, 1);
        assert_eq!(rhi_stats.render_attachment_textures, 1);
        assert_eq!(rhi_stats.buffers, 3);
        assert_eq!(rhi_stats.bind_groups, 1);
        assert_eq!(rhi_stats.encoded_render_draws, 1);
        assert_eq!(rhi_stats.encoded_indirect_draws, 1);
        assert_eq!(rhi_stats.encoded_compute_dispatches, 1);
        assert_eq!(rhi_stats.encoded_barriers, 4);
        assert_eq!(rhi_stats.encoded_debug_groups, 1);
        assert_eq!(rhi_stats.timestamp_queries, 4);
        assert_eq!(rhi_stats.encoded_timestamp_writes, 4);
        assert_eq!(rhi_stats.finished_command_buffers, 2);
        assert_eq!(rhi_stats.submitted_command_buffers, 2);
        assert_eq!(rhi_stats.submissions, 1);
        assert_eq!(stats.timestamp_queries, 4);
        assert_eq!(stats.timestamp_writes, 4);
        assert_eq!(stats.gpu_time_ns, Some(2_000));
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_bind_imported_storage_buffer_in_compute_pass() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let output = Handle::<BufferTag>::from_raw(NonZeroU64::new(41).unwrap());
        let rhi_output = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("graph_compute_output".to_owned()),
                size: std::mem::size_of::<u32>() as u64,
                usage: crate::rhi::RhiBufferUsage::STORAGE | crate::rhi::RhiBufferUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_compute_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var<storage, read_write> output: array<u32>;

                    @compute @workgroup_size(1)
                    fn main() {
                        output[0] = 7u;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_compute_pipeline".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_output =
            graph.import_buffer("graph_compute_output", output, GraphBufferUsage::STORAGE);
        graph
            .add_pass("graph_compute_write")
            .write_buffer(imported_output, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                let output = ctx.rhi_buffer(imported_output)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_compute_bind_group".to_owned()),
                        pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                            binding: 0,
                            buffer: output,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_compute_write".to_owned()),
                    pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                17,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_buffer(output, rhi_output),
            )
            .unwrap();
        let bytes = device.read_buffer(rhi_output, 0, 4).unwrap();

        assert_eq!(bytes, 7_u32.to_le_bytes());
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_draw_to_imported_color_attachment() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let output = Handle::<TextureTag>::from_raw(NonZeroU64::new(42).unwrap());
        let rhi_output = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_render_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_render_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        var positions = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(positions[vertex_index], 0.0, 1.0);
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
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_render_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_output = graph.import_texture(
            "graph_render_target",
            output,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_render_draw")
            .color_attachment(imported_output, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let output = ctx.rhi_texture(imported_output)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_render_draw".to_owned()),
                    pipeline,
                    color_target: Some(output),
                    depth_target: None,
                    vertex_buffers: Vec::new(),
                    index_buffer: None,
                    bind_groups: Vec::new(),
                    vertex_count: 3,
                    index_count: None,
                    instance_count: 1,
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                19,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(output, rhi_output),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_output,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![0, 255, 0, 255]);
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_write_readable_picking_id_target() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let target = Handle::<TextureTag>::from_raw(NonZeroU64::new(43).unwrap());
        let rhi_target = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_picking_id_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let object: crate::ObjectHandle =
            crate::make_handle(crate::ResourceKind::Object, 0x0003_0201, 1);
        let encoded = crate::encode_gpu_picking_object_index(object);
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_picking_id_shader".to_owned()),
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
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_picking_id_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_target = graph.import_texture(
            "graph_picking_id_target",
            target,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_picking_id")
            .color_attachment(imported_target, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let target = ctx.rhi_texture(imported_target)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_picking_id".to_owned()),
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
            });
        graph
            .add_pass("graph_picking_id_readback")
            .read_texture(imported_target, TextureReadUsage::CopySrc)
            .execute(|_| Ok(()));

        let stats = graph
            .execute_on_rhi_with_imports(
                20,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(target, rhi_target),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_target,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, encoded);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.executed_callbacks, 2);
        assert!(stats.barriers >= 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_draw_with_imported_depth_attachment() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let color = Handle::<TextureTag>::from_raw(NonZeroU64::new(44).unwrap());
        let depth = Handle::<TextureTag>::from_raw(NonZeroU64::new(45).unwrap());
        let rhi_color = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_depth_color".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let rhi_depth = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_depth_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Depth32Float,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_depth_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        var positions = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(positions[vertex_index], 0.5, 1.0);
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
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_depth_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: Some(crate::DepthFormat::D32Float),
                vertex_buffers: Vec::new(),
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: Some(crate::rhi::RhiDepthState {
                    format: crate::DepthFormat::D32Float,
                    write_enabled: true,
                    compare: crate::rhi::RhiCompareFunction::LessEqual,
                }),
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_color = graph.import_texture(
            "graph_depth_color",
            color,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        let imported_depth = graph.import_texture(
            "graph_depth_target",
            depth,
            GraphTextureUsage::RENDER_ATTACHMENT,
        );
        graph
            .add_pass("graph_depth_draw")
            .color_attachment(imported_color, ColorAttachmentOps::clear_store())
            .depth_attachment(imported_depth, DepthAttachmentOps::load_store())
            .execute(move |ctx| {
                let color = ctx.rhi_texture(imported_color)?;
                let depth = ctx.rhi_texture(imported_depth)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_depth_draw".to_owned()),
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
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                20,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new()
                    .with_texture(color, rhi_color)
                    .with_texture(depth, rhi_depth),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_color,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![0, 255, 0, 255]);
        let depth_values = device
            .read_texture_depth32f(
                rhi_depth,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();
        assert_eq!(depth_values.len(), 1);
        assert!((depth_values[0] - 0.5).abs() < 0.001);
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_run_depth_only_pass() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let depth = Handle::<TextureTag>::from_raw(NonZeroU64::new(46).unwrap());
        let rhi_depth = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_depth_only_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Depth32Float,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_depth_only_shader".to_owned()),
                source: r#"
                    @vertex
                    fn vs(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                        var positions = array<vec2<f32>, 3>(
                            vec2<f32>(-1.0, -1.0),
                            vec2<f32>( 3.0, -1.0),
                            vec2<f32>(-1.0,  3.0)
                        );
                        return vec4<f32>(positions[vertex_index], 0.25, 1.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_depth_only_pipeline".to_owned()),
                vertex_shader: shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: None,
                fragment_entry: None,
                color_format: None,
                depth_format: Some(crate::DepthFormat::D32Float),
                vertex_buffers: Vec::new(),
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: Some(crate::rhi::RhiDepthState {
                    format: crate::DepthFormat::D32Float,
                    write_enabled: true,
                    compare: crate::rhi::RhiCompareFunction::LessEqual,
                }),
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_depth = graph.import_texture(
            "graph_depth_only_target",
            depth,
            GraphTextureUsage::RENDER_ATTACHMENT,
        );
        graph
            .add_pass("graph_depth_only")
            .depth_attachment(imported_depth, DepthAttachmentOps::load_store())
            .execute(move |ctx| {
                let depth = ctx.rhi_texture(imported_depth)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_depth_only".to_owned()),
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
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                21,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(depth, rhi_depth),
            )
            .unwrap();
        let depth_values = device
            .read_texture_depth32f(
                rhi_depth,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(depth_values.len(), 1);
        assert!((depth_values[0] - 0.25).abs() < 0.001);
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_write_imported_storage_texture_in_compute_pass() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let output = Handle::<TextureTag>::from_raw(NonZeroU64::new(43).unwrap());
        let rhi_output = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_compute_texture".to_owned()),
                width: 1,
                height: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::STORAGE | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_compute_texture_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var output: texture_storage_2d<rgba8unorm, write>;

                    @compute @workgroup_size(1)
                    fn main() {
                        textureStore(output, vec2<i32>(0, 0), vec4<f32>(1.0, 0.0, 0.0, 1.0));
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_compute_texture_pipeline".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_output = graph.import_texture(
            "graph_compute_texture",
            output,
            GraphTextureUsage::STORAGE | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_compute_texture_write")
            .write_texture(imported_output, TextureWriteUsage::Storage)
            .execute(move |ctx| {
                let output = ctx.rhi_texture(imported_output)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_compute_texture_bind_group".to_owned()),
                        pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Texture {
                            binding: 0,
                            texture: output,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_compute_texture_write".to_owned()),
                    pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                18,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(output, rhi_output),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_output,
                crate::rhi::RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![255, 0, 0, 255]);
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_write_imported_rgba32f_storage_texture_in_compute_pass() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let output = Handle::<TextureTag>::from_raw(NonZeroU64::new(44).unwrap());
        let rhi_output = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_compute_rgba32f_texture".to_owned()),
                width: 1,
                height: 1,
                format: TextureFormat::Rgba32Float,
                usage: crate::rhi::RhiTextureUsage::STORAGE | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_compute_rgba32f_texture_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var output: texture_storage_2d<rgba32float, write>;

                    @compute @workgroup_size(1)
                    fn main() {
                        textureStore(output, vec2<i32>(0, 0), vec4<f32>(1.25, 0.5, 8.0, 16.0));
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_compute_rgba32f_texture_pipeline".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let imported_output = graph.import_texture(
            "graph_compute_rgba32f_texture",
            output,
            GraphTextureUsage::STORAGE | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_compute_rgba32f_texture_write")
            .write_texture(imported_output, TextureWriteUsage::Storage)
            .execute(move |ctx| {
                let output = ctx.rhi_texture(imported_output)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_compute_rgba32f_texture_bind_group".to_owned()),
                        pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Texture {
                            binding: 0,
                            texture: output,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_compute_rgba32f_texture_write".to_owned()),
                    pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                19,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(output, rhi_output),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba32f(
                rhi_output,
                crate::rhi::RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![1.25, 0.5, 8.0, 16.0]);
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_write_transient_rgba32f_storage_texture_in_compute_pass() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_transient_rgba32f_texture_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var output: texture_storage_2d<rgba32float, write>;

                    @compute @workgroup_size(1)
                    fn main() {
                        textureStore(output, vec2<i32>(0, 0), vec4<f32>(2.0, 4.0, 6.0, 8.0));
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_transient_rgba32f_texture_pipeline".to_owned()),
                shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();

        let captured_texture = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured_for_write = std::sync::Arc::clone(&captured_texture);
        let mut graph = RenderGraphBuilder::default();
        let output = graph.create_texture(GraphTextureDesc {
            label: Some("graph_transient_rgba32f_texture".to_owned()),
            width: 1,
            height: 1,
            format: TextureFormat::Rgba32Float,
        });
        graph
            .add_pass("graph_transient_rgba32f_texture_write")
            .write_texture(output, TextureWriteUsage::Storage)
            .execute(move |ctx| {
                let output = ctx.rhi_texture(output)?;
                *captured_for_write
                    .lock()
                    .expect("captured texture poisoned") = Some(output);
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_transient_rgba32f_texture_bind_group".to_owned()),
                        pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Texture {
                            binding: 0,
                            texture: output,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_transient_rgba32f_texture_write".to_owned()),
                    pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });
        graph
            .add_pass("graph_transient_rgba32f_texture_readback_marker")
            .read_texture(output, TextureReadUsage::CopySrc)
            .execute(move |ctx| {
                ctx.rhi_texture(output)?;
                Ok(())
            });

        let stats = graph
            .execute_on_rhi(20, &RendererCaps::default(), None, &device)
            .unwrap();
        let rhi_output = captured_texture
            .lock()
            .expect("captured texture poisoned")
            .expect("transient texture was captured");
        let pixel = device
            .read_texture_rgba32f(
                rhi_output,
                crate::rhi::RhiTextureRegion {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![2.0, 4.0, 6.0, 8.0]);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.transient_textures, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 2);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_sample_texture_after_storage_write() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let target = Handle::<TextureTag>::from_raw(NonZeroU64::new(45).unwrap());
        let rhi_target = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_storage_then_sample_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let compute_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_sample_compute_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var output: texture_storage_2d<rgba8unorm, write>;

                    @compute @workgroup_size(1)
                    fn main() {
                        textureStore(output, vec2<i32>(0, 0), vec4<f32>(1.0, 0.0, 0.0, 1.0));
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let compute_pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_storage_then_sample_compute_pipeline".to_owned()),
                shader: compute_shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let render_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_sample_render_shader".to_owned()),
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
        let render_pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_storage_then_sample_render_pipeline".to_owned()),
                vertex_shader: render_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(render_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: Vec::new(),
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();
        let sampler = device
            .create_sampler(&crate::rhi::RhiSamplerDesc {
                label: Some("graph_storage_then_sample_sampler".to_owned()),
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let storage_texture = graph.create_texture(GraphTextureDesc {
            label: Some("graph_storage_then_sample_texture".to_owned()),
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Unorm,
        });
        let imported_target = graph.import_texture(
            "graph_storage_then_sample_target",
            target,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_storage_then_sample_write")
            .write_texture(storage_texture, TextureWriteUsage::Storage)
            .execute(move |ctx| {
                let output = ctx.rhi_texture(storage_texture)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_storage_then_sample_compute_bind_group".to_owned()),
                        pipeline: compute_pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Texture {
                            binding: 0,
                            texture: output,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_storage_then_sample_write".to_owned()),
                    pipeline: compute_pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });
        graph
            .add_pass("graph_storage_then_sample_draw")
            .read_texture(storage_texture, TextureReadUsage::Sampled)
            .color_attachment(imported_target, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let sampled = ctx.rhi_texture(storage_texture)?;
                let target = ctx.rhi_texture(imported_target)?;
                let bind_group =
                    ctx.rhi_device()?
                        .create_bind_group(&crate::rhi::RhiBindGroupDesc {
                            label: Some("graph_storage_then_sample_render_bind_group".to_owned()),
                            pipeline: render_pipeline,
                            group_index: 0,
                            entries: vec![
                                crate::rhi::RhiBindGroupEntry::Texture {
                                    binding: 0,
                                    texture: sampled,
                                },
                                crate::rhi::RhiBindGroupEntry::Sampler {
                                    binding: 1,
                                    sampler,
                                },
                            ],
                        })?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_storage_then_sample_draw".to_owned()),
                    pipeline: render_pipeline,
                    color_target: Some(target),
                    depth_target: None,
                    vertex_buffers: Vec::new(),
                    index_buffer: None,
                    bind_groups: vec![crate::rhi::RhiRenderPassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    vertex_count: 3,
                    index_count: None,
                    instance_count: 1,
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                21,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(target, rhi_target),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_target,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![255, 0, 0, 255]);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.transient_textures, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 2);
        assert!(stats.barriers >= 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_draw_vertex_buffer_after_storage_write() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let target = Handle::<TextureTag>::from_raw(NonZeroU64::new(46).unwrap());
        let rhi_target = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_storage_then_vertex_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
            })
            .unwrap();
        let compute_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_vertex_compute_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var<storage, read_write> positions: array<vec2<f32>>;

                    @compute @workgroup_size(1)
                    fn main() {
                        positions[0] = vec2<f32>(-1.0, -1.0);
                        positions[1] = vec2<f32>( 3.0, -1.0);
                        positions[2] = vec2<f32>(-1.0,  3.0);
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let compute_pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_storage_then_vertex_compute_pipeline".to_owned()),
                shader: compute_shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let render_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_vertex_render_shader".to_owned()),
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
        let render_pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_storage_then_vertex_render_pipeline".to_owned()),
                vertex_shader: render_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(render_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![crate::rhi::RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: crate::VertexStepMode::Vertex,
                    attributes: vec![crate::rhi::RhiVertexAttribute {
                        location: 0,
                        format: crate::VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let positions = graph.create_buffer(GraphBufferDesc {
            label: Some("graph_storage_then_vertex_positions".to_owned()),
            size: 24,
        });
        let imported_target = graph.import_texture(
            "graph_storage_then_vertex_target",
            target,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_storage_then_vertex_write")
            .write_buffer(positions, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                let positions = ctx.rhi_buffer(positions)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_storage_then_vertex_compute_bind_group".to_owned()),
                        pipeline: compute_pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                            binding: 0,
                            buffer: positions,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_storage_then_vertex_write".to_owned()),
                    pipeline: compute_pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });
        graph
            .add_pass("graph_storage_then_vertex_draw")
            .read_buffer(positions, BufferReadUsage::Vertex)
            .color_attachment(imported_target, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let positions = ctx.rhi_buffer(positions)?;
                let target = ctx.rhi_texture(imported_target)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_storage_then_vertex_draw".to_owned()),
                    pipeline: render_pipeline,
                    color_target: Some(target),
                    depth_target: None,
                    vertex_buffers: vec![crate::rhi::RhiVertexBufferBinding {
                        slot: 0,
                        buffer: positions,
                        offset: 0,
                    }],
                    index_buffer: None,
                    bind_groups: Vec::new(),
                    vertex_count: 3,
                    index_count: None,
                    instance_count: 1,
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                22,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new().with_texture(target, rhi_target),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_target,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![0, 255, 0, 255]);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.transient_buffers, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 2);
        assert!(stats.barriers >= 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_draw_index_buffer_after_storage_write() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let target = Handle::<TextureTag>::from_raw(NonZeroU64::new(47).unwrap());
        let vertex_source = Handle::<BufferTag>::from_raw(NonZeroU64::new(48).unwrap());
        let rhi_target = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_storage_then_index_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
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
        let rhi_vertices = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("graph_storage_then_index_vertices".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: crate::rhi::RhiBufferUsage::VERTEX | crate::rhi::RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(rhi_vertices, 0, &vertex_bytes).unwrap();
        let compute_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_index_compute_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var<storage, read_write> indices: array<u32>;

                    @compute @workgroup_size(1)
                    fn main() {
                        indices[0] = 0u;
                        indices[1] = 1u;
                        indices[2] = 2u;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let compute_pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_storage_then_index_compute_pipeline".to_owned()),
                shader: compute_shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let render_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_index_render_shader".to_owned()),
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
        let render_pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_storage_then_index_render_pipeline".to_owned()),
                vertex_shader: render_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(render_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![crate::rhi::RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: crate::VertexStepMode::Vertex,
                    attributes: vec![crate::rhi::RhiVertexAttribute {
                        location: 0,
                        format: crate::VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let indices = graph.create_buffer(GraphBufferDesc {
            label: Some("graph_storage_then_index_indices".to_owned()),
            size: 12,
        });
        let imported_vertices = graph.import_buffer(
            "graph_storage_then_index_vertices",
            vertex_source,
            GraphBufferUsage::VERTEX,
        );
        let imported_target = graph.import_texture(
            "graph_storage_then_index_target",
            target,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_storage_then_index_write")
            .write_buffer(indices, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                let indices = ctx.rhi_buffer(indices)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_storage_then_index_compute_bind_group".to_owned()),
                        pipeline: compute_pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                            binding: 0,
                            buffer: indices,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_storage_then_index_write".to_owned()),
                    pipeline: compute_pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });
        graph
            .add_pass("graph_storage_then_index_draw")
            .read_buffer(imported_vertices, BufferReadUsage::Vertex)
            .read_buffer(indices, BufferReadUsage::Index)
            .color_attachment(imported_target, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let vertices = ctx.rhi_buffer(imported_vertices)?;
                let indices = ctx.rhi_buffer(indices)?;
                let target = ctx.rhi_texture(imported_target)?;
                ctx.encode_rhi_render_pass(&crate::rhi::RhiRenderPassDesc {
                    label: Some("graph_storage_then_index_draw".to_owned()),
                    pipeline: render_pipeline,
                    color_target: Some(target),
                    depth_target: None,
                    vertex_buffers: vec![crate::rhi::RhiVertexBufferBinding {
                        slot: 0,
                        buffer: vertices,
                        offset: 0,
                    }],
                    index_buffer: Some(crate::rhi::RhiIndexBufferBinding {
                        buffer: indices,
                        offset: 0,
                        format: crate::rhi::RhiIndexFormat::Uint32,
                    }),
                    bind_groups: Vec::new(),
                    vertex_count: 3,
                    index_count: Some(3),
                    instance_count: 1,
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                23,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new()
                    .with_texture(target, rhi_target)
                    .with_buffer(vertex_source, rhi_vertices),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_target,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![0, 0, 255, 255]);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.transient_buffers, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 2);
        assert!(stats.barriers >= 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_draw_indirect_after_storage_write() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let target = Handle::<TextureTag>::from_raw(NonZeroU64::new(49).unwrap());
        let vertex_source = Handle::<BufferTag>::from_raw(NonZeroU64::new(51).unwrap());
        let color_source = Handle::<BufferTag>::from_raw(NonZeroU64::new(52).unwrap());
        let rhi_target = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_storage_then_indirect_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
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
        let rhi_vertices = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("graph_storage_then_indirect_vertices".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: crate::rhi::RhiBufferUsage::VERTEX | crate::rhi::RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(rhi_vertices, 0, &vertex_bytes).unwrap();
        let color_bytes = [1.0_f32, 1.0, 0.0, 1.0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let rhi_color = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("graph_storage_then_indirect_color".to_owned()),
                size: color_bytes.len() as u64,
                usage: crate::rhi::RhiBufferUsage::UNIFORM | crate::rhi::RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(rhi_color, 0, &color_bytes).unwrap();
        let compute_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_indirect_compute_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var<storage, read_write> args: array<u32>;

                    @compute @workgroup_size(1)
                    fn main() {
                        args[0] = 0u;
                        args[1] = 1u;
                        args[2] = 0u;
                        args[3] = 0u;
                        args[4] = 3u;
                        args[5] = 1u;
                        args[6] = 0u;
                        args[7] = 0u;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let compute_pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_storage_then_indirect_compute_pipeline".to_owned()),
                shader: compute_shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let render_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_indirect_render_shader".to_owned()),
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
        let render_pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_storage_then_indirect_render_pipeline".to_owned()),
                vertex_shader: render_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(render_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![crate::rhi::RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: crate::VertexStepMode::Vertex,
                    attributes: vec![crate::rhi::RhiVertexAttribute {
                        location: 0,
                        format: crate::VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let indirect_args = graph.create_buffer(GraphBufferDesc {
            label: Some("graph_storage_then_indirect_args".to_owned()),
            size: 32,
        });
        let imported_vertices = graph.import_buffer(
            "graph_storage_then_indirect_vertices",
            vertex_source,
            GraphBufferUsage::VERTEX,
        );
        let imported_color = graph.import_buffer(
            "graph_storage_then_indirect_color",
            color_source,
            GraphBufferUsage::UNIFORM,
        );
        let imported_target = graph.import_texture(
            "graph_storage_then_indirect_target",
            target,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_storage_then_indirect_write")
            .write_buffer(indirect_args, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                let indirect_args = ctx.rhi_buffer(indirect_args)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some("graph_storage_then_indirect_compute_bind_group".to_owned()),
                        pipeline: compute_pipeline,
                        group_index: 0,
                        entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                            binding: 0,
                            buffer: indirect_args,
                        }],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_storage_then_indirect_write".to_owned()),
                    pipeline: compute_pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });
        graph
            .add_pass("graph_storage_then_indirect_draw")
            .read_buffer(imported_vertices, BufferReadUsage::Vertex)
            .read_buffer(imported_color, BufferReadUsage::Uniform)
            .read_buffer(indirect_args, BufferReadUsage::Indirect)
            .color_attachment(imported_target, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let vertices = ctx.rhi_buffer(imported_vertices)?;
                let color = ctx.rhi_buffer(imported_color)?;
                let indirect_args = ctx.rhi_buffer(indirect_args)?;
                let target = ctx.rhi_texture(imported_target)?;
                let bind_group =
                    ctx.rhi_device()?
                        .create_bind_group(&crate::rhi::RhiBindGroupDesc {
                            label: Some("graph_storage_then_indirect_bind_group".to_owned()),
                            pipeline: render_pipeline,
                            group_index: 0,
                            entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                                binding: 0,
                                buffer: color,
                            }],
                        })?;
                ctx.encode_rhi_indirect_render_pass(&crate::rhi::RhiIndirectRenderPassDesc {
                    label: Some("graph_storage_then_indirect_draw".to_owned()),
                    pipeline: render_pipeline,
                    color_target: Some(target),
                    depth_target: None,
                    vertex_buffers: vec![crate::rhi::RhiVertexBufferBinding {
                        slot: 0,
                        buffer: vertices,
                        offset: 0,
                    }],
                    bind_groups: vec![crate::rhi::RhiRenderPassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    indirect_buffer: indirect_args,
                    indirect_offset: 0,
                    draw_count: 2,
                    draw_stride: 16,
                })
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                24,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new()
                    .with_texture(target, rhi_target)
                    .with_buffer(vertex_source, rhi_vertices)
                    .with_buffer(color_source, rhi_color),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_target,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![255, 255, 0, 255]);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.transient_buffers, 1);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 2);
        assert!(stats.barriers >= 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn graph_execute_on_wgpu_can_draw_indexed_indirect_after_storage_write() {
        let Ok(graphics) =
            graphics_wgpu::WgpuGraphics::new(graphics_wgpu::WgpuGraphicsOptions::default())
        else {
            return;
        };
        let device = crate::rhi::WgpuRhiDevice::new(&graphics);
        let target = Handle::<TextureTag>::from_raw(NonZeroU64::new(50).unwrap());
        let vertex_source = Handle::<BufferTag>::from_raw(NonZeroU64::new(53).unwrap());
        let color_source = Handle::<BufferTag>::from_raw(NonZeroU64::new(54).unwrap());
        let rhi_target = device
            .create_texture(&crate::rhi::RhiTextureDesc {
                label: Some("graph_storage_then_indexed_indirect_target".to_owned()),
                width: 4,
                height: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: crate::rhi::RhiTextureUsage::RENDER_ATTACHMENT
                    | crate::rhi::RhiTextureUsage::COPY_SRC,
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
        let rhi_vertices = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("graph_storage_then_indexed_indirect_vertices".to_owned()),
                size: vertex_bytes.len() as u64,
                usage: crate::rhi::RhiBufferUsage::VERTEX | crate::rhi::RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(rhi_vertices, 0, &vertex_bytes).unwrap();
        let color_bytes = [0.0_f32, 1.0, 1.0, 1.0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let rhi_color = device
            .create_buffer(&crate::rhi::RhiBufferDesc {
                label: Some("graph_storage_then_indexed_indirect_color".to_owned()),
                size: color_bytes.len() as u64,
                usage: crate::rhi::RhiBufferUsage::UNIFORM | crate::rhi::RhiBufferUsage::COPY_DST,
            })
            .unwrap();
        device.write_buffer(rhi_color, 0, &color_bytes).unwrap();
        let compute_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_indexed_indirect_compute_shader".to_owned()),
                source: r#"
                    @group(0) @binding(0)
                    var<storage, read_write> indices: array<u32>;
                    @group(0) @binding(1)
                    var<storage, read_write> args: array<u32>;

                    @compute @workgroup_size(1)
                    fn main() {
                        indices[0] = 0u;
                        indices[1] = 1u;
                        indices[2] = 2u;
                        args[0] = 0u;
                        args[1] = 1u;
                        args[2] = 0u;
                        args[3] = 0u;
                        args[4] = 0u;
                        args[5] = 3u;
                        args[6] = 1u;
                        args[7] = 0u;
                        args[8] = 0u;
                        args[9] = 0u;
                    }
                "#
                .to_owned(),
            })
            .unwrap();
        let compute_pipeline = device
            .create_compute_pipeline(&crate::rhi::RhiComputePipelineDesc {
                label: Some("graph_storage_then_indexed_indirect_compute_pipeline".to_owned()),
                shader: compute_shader,
                entry_point: "main".to_owned(),
            })
            .unwrap();
        let render_shader = device
            .create_shader_module(&crate::rhi::RhiShaderModuleDesc {
                label: Some("graph_storage_then_indexed_indirect_render_shader".to_owned()),
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
        let render_pipeline = device
            .create_graphics_pipeline(&crate::rhi::RhiGraphicsPipelineDesc {
                label: Some("graph_storage_then_indexed_indirect_render_pipeline".to_owned()),
                vertex_shader: render_shader,
                vertex_entry: "vs".to_owned(),
                fragment_shader: Some(render_shader),
                fragment_entry: Some("fs".to_owned()),
                color_format: Some(TextureFormat::Rgba8Unorm),
                depth_format: None,
                vertex_buffers: vec![crate::rhi::RhiVertexBufferLayout {
                    stride: 8,
                    step_mode: crate::VertexStepMode::Vertex,
                    attributes: vec![crate::rhi::RhiVertexAttribute {
                        location: 0,
                        format: crate::VertexFormat::Float32x2,
                        offset: 0,
                    }],
                }],
                primitive: crate::rhi::RhiPrimitiveState::default(),
                depth: None,
            })
            .unwrap();

        let mut graph = RenderGraphBuilder::default();
        let indices = graph.create_buffer(GraphBufferDesc {
            label: Some("graph_storage_then_indexed_indirect_indices".to_owned()),
            size: 12,
        });
        let indirect_args = graph.create_buffer(GraphBufferDesc {
            label: Some("graph_storage_then_indexed_indirect_args".to_owned()),
            size: 40,
        });
        let imported_vertices = graph.import_buffer(
            "graph_storage_then_indexed_indirect_vertices",
            vertex_source,
            GraphBufferUsage::VERTEX,
        );
        let imported_color = graph.import_buffer(
            "graph_storage_then_indexed_indirect_color",
            color_source,
            GraphBufferUsage::UNIFORM,
        );
        let imported_target = graph.import_texture(
            "graph_storage_then_indexed_indirect_target",
            target,
            GraphTextureUsage::RENDER_ATTACHMENT | GraphTextureUsage::COPY_SRC,
        );
        graph
            .add_pass("graph_storage_then_indexed_indirect_write")
            .write_buffer(indices, BufferWriteUsage::Storage)
            .write_buffer(indirect_args, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                let indices = ctx.rhi_buffer(indices)?;
                let indirect_args = ctx.rhi_buffer(indirect_args)?;
                let bind_group = ctx.rhi_device()?.create_compute_bind_group(
                    &crate::rhi::RhiComputeBindGroupDesc {
                        label: Some(
                            "graph_storage_then_indexed_indirect_compute_bind_group".to_owned(),
                        ),
                        pipeline: compute_pipeline,
                        group_index: 0,
                        entries: vec![
                            crate::rhi::RhiBindGroupEntry::Buffer {
                                binding: 0,
                                buffer: indices,
                            },
                            crate::rhi::RhiBindGroupEntry::Buffer {
                                binding: 1,
                                buffer: indirect_args,
                            },
                        ],
                    },
                )?;
                ctx.encode_rhi_compute_pass(&crate::rhi::RhiComputePassDesc {
                    label: Some("graph_storage_then_indexed_indirect_write".to_owned()),
                    pipeline: compute_pipeline,
                    bind_groups: vec![crate::rhi::RhiComputePassBindGroup {
                        index: 0,
                        bind_group,
                    }],
                    workgroups: [1, 1, 1],
                })
            });
        graph
            .add_pass("graph_storage_then_indexed_indirect_draw")
            .read_buffer(imported_vertices, BufferReadUsage::Vertex)
            .read_buffer(imported_color, BufferReadUsage::Uniform)
            .read_buffer(indices, BufferReadUsage::Index)
            .read_buffer(indirect_args, BufferReadUsage::Indirect)
            .color_attachment(imported_target, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let vertices = ctx.rhi_buffer(imported_vertices)?;
                let color = ctx.rhi_buffer(imported_color)?;
                let indices = ctx.rhi_buffer(indices)?;
                let indirect_args = ctx.rhi_buffer(indirect_args)?;
                let target = ctx.rhi_texture(imported_target)?;
                let bind_group =
                    ctx.rhi_device()?
                        .create_bind_group(&crate::rhi::RhiBindGroupDesc {
                            label: Some(
                                "graph_storage_then_indexed_indirect_bind_group".to_owned(),
                            ),
                            pipeline: render_pipeline,
                            group_index: 0,
                            entries: vec![crate::rhi::RhiBindGroupEntry::Buffer {
                                binding: 0,
                                buffer: color,
                            }],
                        })?;
                ctx.encode_rhi_indexed_indirect_render_pass(
                    &crate::rhi::RhiIndexedIndirectRenderPassDesc {
                        label: Some("graph_storage_then_indexed_indirect_draw".to_owned()),
                        pipeline: render_pipeline,
                        color_target: Some(target),
                        depth_target: None,
                        vertex_buffers: vec![crate::rhi::RhiVertexBufferBinding {
                            slot: 0,
                            buffer: vertices,
                            offset: 0,
                        }],
                        index_buffer: crate::rhi::RhiIndexBufferBinding {
                            buffer: indices,
                            offset: 0,
                            format: crate::rhi::RhiIndexFormat::Uint32,
                        },
                        bind_groups: vec![crate::rhi::RhiRenderPassBindGroup {
                            index: 0,
                            bind_group,
                        }],
                        indirect_buffer: indirect_args,
                        indirect_offset: 0,
                        draw_count: 2,
                        draw_stride: 20,
                    },
                )
            });

        let stats = graph
            .execute_on_rhi_with_imports(
                25,
                &RendererCaps::default(),
                None,
                &device,
                &RhiResourceImports::new()
                    .with_texture(target, rhi_target)
                    .with_buffer(vertex_source, rhi_vertices)
                    .with_buffer(color_source, rhi_color),
            )
            .unwrap();
        let pixel = device
            .read_texture_rgba8(
                rhi_target,
                crate::rhi::RhiTextureRegion {
                    x: 2,
                    y: 2,
                    width: 1,
                    height: 1,
                },
            )
            .unwrap();

        assert_eq!(pixel, vec![0, 255, 255, 255]);
        assert_eq!(stats.pass_count, 2);
        assert_eq!(stats.transient_buffers, 2);
        assert_eq!(stats.compute_passes, 1);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.compute_dispatches, 1);
        assert_eq!(stats.executed_callbacks, 2);
        assert!(stats.barriers >= 2);
    }

    #[test]
    fn graph_rejects_forward_dependencies() {
        let mut graph = RenderGraphBuilder::default();
        graph
            .add_pass("bad")
            .depends_on(PassId(0))
            .execute(|_| Ok(()));

        assert!(matches!(
            graph.validate(),
            Err(RendererError::RenderGraphValidation(_))
        ));
    }

    #[test]
    fn graph_rejects_unknown_resources() {
        let mut graph = RenderGraphBuilder::default();
        graph
            .add_pass("bad_resource")
            .read_texture(GraphTexture(99), TextureReadUsage::Sampled)
            .execute(|_| Ok(()));

        assert!(matches!(
            graph.validate(),
            Err(RendererError::RenderGraphValidation(_))
        ));
    }
}
