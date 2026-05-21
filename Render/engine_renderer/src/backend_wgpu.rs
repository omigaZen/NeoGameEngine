use engine_graphics::{GraphicsError, PresentMode, RenderSurface, SurfaceSize};
use engine_platform::PlatformWindow;
use engine_render::{RenderQueue, RenderQueueStats, RenderScene};
use graphics_wgpu::{
    wgpu, WgpuFrameReadback, WgpuGraphics, WgpuGraphicsOptions, WgpuSurface, WgpuSurfaceOptions,
};
use render_wgpu::{MeshRenderStats, MeshRenderer, WgpuPostProcessOptions, WgpuRenderScene};

use crate::{
    AddressMode, BackendNativePassDrawStats, BackendPreference, BindingClass, BindingType,
    CompareFunc, DepthFormat, DeviceStatus, FilterMode, FormatCaps, FrameStats, MaterialHandle,
    MaterialParameter, MaterialParameterValue, MaterialTemplateHandle, MemoryStats,
    PipelineCacheStats, PipelineKey, RenderGraphStats, RendererCaps, RendererConfig, RendererError,
    RendererFeatures, RendererLimits, ResourceReclaimPolicy, SamplerDesc, SamplerHandle,
    ShaderHandle, ShaderInterfaceDesc, ShaderSource, ShaderStages, StorageTextureAccess,
    TextureDimension, TextureFormat, TextureHandle, VSyncMode, WgpuRhiDevice,
};

#[derive(Clone, Debug)]
pub struct WgpuShaderInterfaceLayoutPlan {
    pub bind_groups: Vec<WgpuShaderBindGroupLayoutPlan>,
    pub push_constants: Vec<wgpu::PushConstantRange>,
}

#[derive(Clone, Debug)]
pub struct WgpuShaderBindGroupLayoutPlan {
    pub group: u32,
    pub entries: Vec<wgpu::BindGroupLayoutEntry>,
}

#[derive(Debug)]
pub struct WgpuShaderInterfaceLayoutObjects {
    pub bind_group_layouts: Vec<WgpuShaderBindGroupLayoutObject>,
    pub pipeline_layout: wgpu::PipelineLayout,
}

#[derive(Debug)]
pub struct WgpuShaderBindGroupLayoutObject {
    pub group: u32,
    pub entry_count: usize,
    pub layout: wgpu::BindGroupLayout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuMaterialBindGroupResourcePlan {
    pub groups: Vec<WgpuMaterialBindGroupResourceGroupPlan>,
}

#[derive(Debug)]
pub struct WgpuMaterialBindGroupObject {
    pub group: u32,
    pub entry_count: usize,
    pub owned_buffers: Vec<WgpuMaterialOwnedBuffer>,
    pub bind_group: std::sync::Arc<wgpu::BindGroup>,
}

#[derive(Default)]
pub struct WgpuMaterialExternalResourceRegistry {
    textures: std::collections::HashMap<TextureHandle, WgpuMaterialTextureBinding>,
    samplers: std::collections::HashMap<SamplerHandle, WgpuMaterialSamplerBinding>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WgpuMaterialExternalResourceStats {
    pub texture_bindings: usize,
    pub sampler_bindings: usize,
    pub total_bindings: usize,
}

pub struct WgpuMaterialTextureBinding {
    _texture: Option<wgpu::Texture>,
    pub view: wgpu::TextureView,
    pub generated_mips: u32,
}

pub struct WgpuMaterialSamplerBinding {
    pub sampler: wgpu::Sampler,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuMaterialTextureUploadDesc {
    pub label: Option<String>,
    pub dimension: TextureDimension,
    pub width: u32,
    pub height: u32,
    pub depth_or_layers: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub format: TextureFormat,
    pub sampled_binding: bool,
    pub storage_binding: bool,
    pub generate_mips_from_base: bool,
    pub uploads: Vec<WgpuMaterialTextureUpload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuMaterialTextureUpload {
    pub mip_level: u32,
    pub origin: [u32; 3],
    pub extent: [u32; 3],
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct WgpuMaterialOwnedBuffer {
    pub binding: u32,
    pub size: u64,
    pub buffer: wgpu::Buffer,
}

pub struct WgpuRenderPipelineDesc<'a> {
    pub label: Option<&'a str>,
    pub shader: &'a wgpu::ShaderModule,
    pub vertex_entry: &'a str,
    pub fragment_entry: Option<&'a str>,
    pub vertex_buffers: &'a [wgpu::VertexBufferLayout<'a>],
    pub color_format: Option<TextureFormat>,
    pub depth_format: Option<DepthFormat>,
    pub sample_count: u32,
    pub depth_write: bool,
    pub blend: Option<wgpu::BlendState>,
}

pub struct WgpuNativeRenderPipelineBuildDesc<'a> {
    pub label: Option<&'a str>,
    pub key: PipelineKey,
    pub shader_interface_layout_hash: u64,
    pub shader_source: ShaderSource<'a>,
    pub interface: &'a ShaderInterfaceDesc,
    pub material_resource_plan: Option<&'a WgpuMaterialBindGroupResourcePlan>,
    pub vertex_entry: &'a str,
    pub fragment_entry: Option<&'a str>,
    pub vertex_buffers: &'a [wgpu::VertexBufferLayout<'a>],
    pub color_format: Option<TextureFormat>,
    pub depth_format: Option<DepthFormat>,
    pub sample_count: u32,
    pub depth_write: bool,
    pub blend: Option<wgpu::BlendState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuMaterialBindGroupResourceGroupPlan {
    pub group: u32,
    pub entries: Vec<WgpuMaterialBindGroupResourceEntryPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuMaterialBindGroupResourceEntryPlan {
    pub name: String,
    pub binding: u32,
    pub binding_class: BindingClass,
    pub binding_type: BindingType,
    pub resource: WgpuMaterialBindingResource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WgpuMaterialBindingResource {
    Texture(TextureHandle),
    Sampler(SamplerHandle),
    BufferBytes { bytes: Vec<u8> },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WgpuNativePipelineCacheMetadata {
    entries: std::collections::HashMap<PipelineKey, WgpuNativePipelineCacheEntryMetadata>,
    invalidated_this_frame: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WgpuShaderVariantModuleKey {
    shader: ShaderHandle,
    flags: Vec<String>,
}

#[derive(Debug)]
pub struct WgpuNativePipelineObjects {
    pub shader_module: wgpu::ShaderModule,
    pub layout_objects: WgpuShaderInterfaceLayoutObjects,
    pub material_bind_groups: Vec<WgpuMaterialBindGroupObject>,
    pub render_pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    pub render_pipeline_key: PipelineKey,
    pub material: Option<MaterialHandle>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WgpuTombstoneSubmissionIndexCoverage {
    #[default]
    NotApplicable,
    None,
    Partial,
    All,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WgpuBackendResourceRetirementStats {
    pub tombstones: usize,
    pub last_poll_queue_empty: bool,
    pub retired_after_queue_empty_poll: bool,
    pub last_poll_completed_submission_index_recorded: bool,
    pub retired_after_completed_submission_index_poll: bool,
    pub nonblocking_submission_index_poll_supported: bool,
    pub queue_empty_poll_fallback: bool,
    pub last_poll_used_queue_empty_fallback: bool,
    pub tombstones_with_submission_index: usize,
    pub tombstones_without_submission_index: usize,
    pub tombstones_waiting_for_submission_index: usize,
    pub tombstones_waiting_for_queue_empty: usize,
    pub tombstone_submission_index_coverage: WgpuTombstoneSubmissionIndexCoverage,
    pub all_tombstones_have_submission_index: bool,
    pub partial_tombstone_submission_index_coverage: bool,
    pub no_tombstones_have_submission_index: bool,
    pub native_pipeline_entries: usize,
    pub render_pipeline_refs: usize,
    pub shader_modules: usize,
    pub shader_variant_modules: usize,
    pub material_textures: usize,
    pub material_samplers: usize,
    pub fence_objects: usize,
    pub fence_submission_indices: usize,
    pub fence_objects_without_submission_index: usize,
    pub post_pass_vertex_buffers: usize,
    pub post_pass_index_buffers: usize,
    pub bind_groups: usize,
    pub owned_buffers: usize,
    pub retired_tombstones_this_poll: usize,
    pub retired_tombstones_with_submission_index_this_poll: usize,
    pub retired_tombstones_without_submission_index_this_poll: usize,
    pub retired_tombstone_submission_index_coverage_this_poll: WgpuTombstoneSubmissionIndexCoverage,
    pub retired_all_tombstones_had_submission_index_this_poll: bool,
    pub retired_partial_tombstone_submission_index_coverage_this_poll: bool,
    pub retired_no_tombstones_had_submission_index_this_poll: bool,
    pub retired_native_pipeline_entries_this_poll: usize,
    pub retired_render_pipeline_refs_this_poll: usize,
    pub retired_shader_modules_this_poll: usize,
    pub retired_shader_variant_modules_this_poll: usize,
    pub retired_material_textures_this_poll: usize,
    pub retired_material_samplers_this_poll: usize,
    pub retired_fence_objects_this_poll: usize,
    pub retired_fence_submission_indices_this_poll: usize,
    pub retired_fence_objects_without_submission_index_this_poll: usize,
    pub retired_post_pass_vertex_buffers_this_poll: usize,
    pub retired_post_pass_index_buffers_this_poll: usize,
    pub retired_bind_groups_this_poll: usize,
    pub retired_owned_buffers_this_poll: usize,
}

#[derive(Clone, Debug)]
struct WgpuBackendFence {
    submission_index: Option<wgpu::SubmissionIndex>,
    submission_order: Option<u64>,
}

impl WgpuBackendFence {
    fn has_submission_index(&self) -> bool {
        self.submission_index.is_some()
    }

    fn is_completed_by(&self, completed_submission_order: Option<u64>, queue_empty: bool) -> bool {
        match &self.submission_index {
            Some(_) => self.submission_order.is_some_and(|order| {
                completed_submission_order.is_some_and(|completed| order <= completed)
            }),
            None => queue_empty,
        }
    }
}

#[derive(Default)]
struct WgpuBackendResourceTombstone {
    fence: Option<WgpuBackendFence>,
    native_pipeline_objects: Vec<WgpuNativePipelineObjects>,
    shader_variant_modules: Vec<wgpu::ShaderModule>,
    material_textures: Vec<WgpuMaterialTextureBinding>,
    material_samplers: Vec<WgpuMaterialSamplerBinding>,
    post_pass_buffers: Vec<WgpuBackendPostPassBufferTombstone>,
}

impl WgpuBackendResourceTombstone {
    fn is_completed_by(&self, completed_submission_order: Option<u64>, queue_empty: bool) -> bool {
        self.fence.as_ref().map_or(queue_empty, |fence| {
            fence.is_completed_by(completed_submission_order, queue_empty)
        })
    }
}

#[derive(Debug)]
struct WgpuBackendSubmissionCompletion {
    max_tombstone_submission_order: u64,
    completion_receiver: std::sync::mpsc::Receiver<()>,
}

struct WgpuBackendPostPassBufferTombstone {
    vertex_buffers: Vec<WgpuNativePipelinePostPassVertexBuffer>,
    index_buffer: Option<WgpuNativePipelinePostPassIndexBuffer>,
}

struct WgpuNativePipelinePostPassSubmission {
    render_pipeline: std::sync::Arc<wgpu::RenderPipeline>,
    bind_groups: Vec<(u32, std::sync::Arc<wgpu::BindGroup>)>,
    vertices: std::ops::Range<u32>,
    indices: Option<std::ops::Range<u32>>,
    instances: std::ops::Range<u32>,
    vertex_buffers: Vec<WgpuNativePipelinePostPassVertexBuffer>,
    index_buffer: Option<WgpuNativePipelinePostPassIndexBuffer>,
}

struct WgpuNativePipelinePostPassVertexBuffer {
    slot: u32,
    buffer: std::sync::Arc<wgpu::Buffer>,
}

struct WgpuNativePipelinePostPassIndexBuffer {
    format: wgpu::IndexFormat,
    buffer: std::sync::Arc<wgpu::Buffer>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WgpuNativePipelineSubmissionInfo {
    pub bind_group_count: usize,
    pub vertex_count: u32,
    pub instance_count: u32,
}

pub struct WgpuNativePipelineDrawDesc<'a> {
    pub label: Option<&'a str>,
    pub key: PipelineKey,
    pub color_view: &'a wgpu::TextureView,
    pub clear_color: wgpu::Color,
    pub vertices: std::ops::Range<u32>,
    pub instances: std::ops::Range<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuQueuedNativePipelineDraw {
    pub key: PipelineKey,
    pub vertices: std::ops::Range<u32>,
    pub indices: Option<std::ops::Range<u32>>,
    pub instances: std::ops::Range<u32>,
    pub vertex_buffers: Vec<WgpuQueuedVertexBuffer>,
    pub index_buffer: Option<WgpuQueuedIndexBuffer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuQueuedVertexBuffer {
    pub slot: u32,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuQueuedIndexBuffer {
    pub format: WgpuQueuedIndexFormat,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WgpuQueuedIndexFormat {
    Uint16,
    Uint32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuNativePipelineCacheEntryMetadata {
    pub key: PipelineKey,
    pub shader_interface_layout_hash: u64,
    pub last_used_frame: Option<u64>,
    pub used_this_frame: bool,
}

pub struct WgpuRendererRuntime {
    config: RendererConfig,
    graphics: WgpuGraphics,
    rhi_device: WgpuRhiDevice,
    surface: Option<WgpuSurface>,
    renderer: Option<MeshRenderer>,
    scene: Option<WgpuRenderScene>,
    native_pipeline_cache: WgpuNativePipelineCacheMetadata,
    shader_variant_modules:
        std::collections::HashMap<WgpuShaderVariantModuleKey, wgpu::ShaderModule>,
    native_pipeline_objects: std::collections::HashMap<PipelineKey, WgpuNativePipelineObjects>,
    native_render_pipeline_objects:
        std::collections::HashMap<PipelineKey, std::sync::Arc<wgpu::RenderPipeline>>,
    backend_resource_tombstones: Vec<WgpuBackendResourceTombstone>,
    backend_resource_retirement_stats: WgpuBackendResourceRetirementStats,
    backend_submission_completions: Vec<WgpuBackendSubmissionCompletion>,
    material_external_resources: WgpuMaterialExternalResourceRegistry,
    queued_native_pipeline_draws: Vec<WgpuQueuedNativePipelineDraw>,
    last_stats: Option<FrameStats>,
    last_submission_index: Option<wgpu::SubmissionIndex>,
    last_submission_fence_index: Option<wgpu::SubmissionIndex>,
    last_submission_fence_order: Option<u64>,
    next_backend_submission_order: u64,
    latest_completed_submission_order: Option<u64>,
    device_status: DeviceStatus,
    frame_index: u64,
}

impl WgpuRendererRuntime {
    pub fn new(config: RendererConfig) -> Result<Self, RendererError> {
        let graphics =
            WgpuGraphics::new(wgpu_options(config.backend)).map_err(map_backend_error)?;
        let rhi_device = WgpuRhiDevice::new(&graphics);
        Ok(Self {
            config,
            graphics,
            rhi_device,
            surface: None,
            renderer: None,
            scene: None,
            native_pipeline_cache: WgpuNativePipelineCacheMetadata::default(),
            shader_variant_modules: std::collections::HashMap::new(),
            native_pipeline_objects: std::collections::HashMap::new(),
            native_render_pipeline_objects: std::collections::HashMap::new(),
            backend_resource_tombstones: Vec::new(),
            backend_resource_retirement_stats: WgpuBackendResourceRetirementStats::default(),
            backend_submission_completions: Vec::new(),
            material_external_resources: WgpuMaterialExternalResourceRegistry::default(),
            queued_native_pipeline_draws: Vec::new(),
            last_stats: None,
            last_submission_index: None,
            last_submission_fence_index: None,
            last_submission_fence_order: None,
            next_backend_submission_order: 1,
            latest_completed_submission_order: None,
            device_status: DeviceStatus::Ok,
            frame_index: 0,
        })
    }

    pub fn with_surface(
        config: RendererConfig,
        window: &dyn PlatformWindow,
    ) -> Result<Self, RendererError> {
        let graphics =
            WgpuGraphics::new(wgpu_options(config.backend)).map_err(map_backend_error)?;
        let rhi_device = WgpuRhiDevice::new(&graphics);
        let size = window.inner_size();
        let mut surface = graphics
            .create_surface_with_options(
                &*window,
                SurfaceSize::new(size.width, size.height),
                surface_options(&config),
            )
            .map_err(map_backend_error)?;
        surface
            .set_present_mode(present_mode_for_vsync(config.vsync))
            .map_err(map_backend_error)?;
        surface
            .set_frame_latency(config.frame_latency)
            .map_err(map_backend_error)?;
        if config.msaa_samples > 1 {
            surface
                .set_sample_count(config.msaa_samples)
                .map_err(map_backend_error)?;
        }
        let runtime_caps = wgpu_renderer_caps(&config, &graphics);
        validate_surface_runtime_formats(
            &config,
            surface.format(),
            surface.depth_format(),
            &runtime_caps,
        )?;
        let renderer = MeshRenderer::new_with_sample_count(
            &graphics,
            surface.format(),
            surface.depth_format(),
            surface.sample_count(),
        )
        .map_err(map_backend_error)?;

        Ok(Self {
            config,
            graphics,
            rhi_device,
            surface: Some(surface),
            renderer: Some(renderer),
            scene: None,
            native_pipeline_cache: WgpuNativePipelineCacheMetadata::default(),
            shader_variant_modules: std::collections::HashMap::new(),
            native_pipeline_objects: std::collections::HashMap::new(),
            native_render_pipeline_objects: std::collections::HashMap::new(),
            backend_resource_tombstones: Vec::new(),
            backend_resource_retirement_stats: WgpuBackendResourceRetirementStats::default(),
            backend_submission_completions: Vec::new(),
            material_external_resources: WgpuMaterialExternalResourceRegistry::default(),
            queued_native_pipeline_draws: Vec::new(),
            last_stats: None,
            last_submission_index: None,
            last_submission_fence_index: None,
            last_submission_fence_order: None,
            next_backend_submission_order: 1,
            latest_completed_submission_order: None,
            device_status: DeviceStatus::Ok,
            frame_index: 0,
        })
    }

fn validate_surface_runtime_formats(
    config: &RendererConfig,
    surface_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    caps: &RendererCaps,
) -> Result<(), RendererError> {
    if let Some(requested_surface_format) = config.surface_format {
        let requested = wgpu_surface_format(requested_surface_format);
        if requested != surface_format {
            return Err(RendererError::Validation(format!(
                "renderer surface_format {:?} does not match runtime surface format {:?}",
                requested_surface_format, surface_format
            )));
        }
        if !caps.formats.color.contains(&requested_surface_format) {
            return Err(RendererError::Validation(
                "renderer surface_format is not supported by runtime capabilities".to_owned(),
            ));
        }
    } else if !caps.formats.color.is_empty() {
        let actual_surface_format = texture_format_from_wgpu_surface(surface_format)
            .ok_or_else(|| RendererError::Validation("runtime surface format is unsupported".to_owned()))?;
        if !caps.formats.color.contains(&actual_surface_format) {
            return Err(RendererError::Validation(
                "runtime surface format is not supported by runtime capabilities".to_owned(),
            ));
        }
    }

    let requested_depth_format = wgpu_depth_format(config.depth_format);
    if depth_format != Some(requested_depth_format) {
        return Err(RendererError::Validation(format!(
            "renderer depth_format {:?} does not match runtime depth format {:?}",
            config.depth_format, depth_format
        )));
    }
    let Some(runtime_depth_format) = depth_format.and_then(depth_format_from_wgpu) else {
        return Err(RendererError::Validation(
            "runtime surface depth format is unsupported".to_owned(),
        ));
    };
    if !caps.formats.depth.contains(&runtime_depth_format) {
        return Err(RendererError::Validation(
            "renderer depth_format is not supported by runtime capabilities".to_owned(),
        ));
    }
    Ok(())
}

    pub fn graphics(&self) -> &WgpuGraphics {
        &self.graphics
    }

    pub fn rhi_device(&self) -> WgpuRhiDevice {
        self.rhi_device.clone()
    }

    pub fn renderer_caps(&self) -> RendererCaps {
        let mut caps = wgpu_renderer_caps(&self.config, &self.graphics);
        if self.surface.is_some() {
            caps.features = caps.features | RendererFeatures::SURFACE;
        }
        caps
    }

    pub fn surface(&self) -> Option<&WgpuSurface> {
        self.surface.as_ref()
    }

    pub fn surface_mut(&mut self) -> Option<&mut WgpuSurface> {
        self.surface.as_mut()
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        let Some(surface) = &mut self.surface else {
            return Err(RendererError::Validation(
                "wgpu renderer was created without a surface".to_owned(),
            ));
        };
        surface
            .resize(SurfaceSize::new(width, height))
            .map_err(map_backend_error)
    }

    pub fn set_vsync(&mut self, mode: VSyncMode) -> Result<(), RendererError> {
        if let Some(surface) = &mut self.surface {
            surface
                .set_present_mode(present_mode_for_vsync(mode))
                .map_err(map_backend_error)?;
        }
        self.config.vsync = mode;
        Ok(())
    }

    pub fn render_scene(&mut self, scene: &RenderScene) -> Result<FrameStats, RendererError> {
        self.render_scene_with_post_process_options(scene, WgpuPostProcessOptions::default())
    }

    pub fn render_scene_with_post_process_options(
        &mut self,
        scene: &RenderScene,
        post_process_options: WgpuPostProcessOptions,
    ) -> Result<FrameStats, RendererError> {
        let Some(surface) = &mut self.surface else {
            return Err(RendererError::Validation(
                "wgpu renderer was created without a surface".to_owned(),
            ));
        };
        let Some(renderer) = &mut self.renderer else {
            return Err(RendererError::Validation(
                "wgpu mesh renderer is not initialized".to_owned(),
            ));
        };
        self.native_pipeline_cache.begin_frame();

        let queue = RenderQueue::from_scene(scene);
        let aspect_ratio = aspect_ratio(surface.size());
        match &mut self.scene {
            Some(gpu_scene) => {
                if let Err(error) =
                    gpu_scene.sync(&self.graphics, renderer, scene, &queue, aspect_ratio)
                {
                    return Err(record_backend_error(&mut self.device_status, error));
                }
            }
            None => {
                match WgpuRenderScene::prepare(
                    &self.graphics,
                    renderer,
                    scene,
                    &queue,
                    aspect_ratio,
                ) {
                    Ok(scene) => self.scene = Some(scene),
                    Err(error) => return Err(record_backend_error(&mut self.device_status, error)),
                }
            }
        }

        let gpu_scene = self
            .scene
            .as_ref()
            .expect("gpu scene was prepared before render");
        renderer.set_gpu_profiling_enabled(self.config.gpu_profiling);
        renderer.set_post_process_options(post_process_options);
        let queued_native_draws = std::mem::take(&mut self.queued_native_pipeline_draws);
        for draw in &queued_native_draws {
            if !self.native_pipeline_objects.contains_key(&draw.key) {
                return Err(RendererError::Validation(
                    "wgpu native pipeline objects do not exist".to_owned(),
                ));
            }
            self.native_pipeline_cache
                .mark_used(draw.key, self.frame_index)?;
        }
        let native_post_pass_submissions = queued_native_draws
            .iter()
            .map(|draw| {
                let objects = self.native_pipeline_objects.get(&draw.key).ok_or_else(|| {
                    RendererError::Validation(
                        "wgpu native pipeline objects do not exist".to_owned(),
                    )
                })?;
                let vertex_buffers = draw
                    .vertex_buffers
                    .iter()
                    .map(|vertex_buffer| {
                        Ok(WgpuNativePipelinePostPassVertexBuffer {
                            slot: vertex_buffer.slot,
                            buffer: std::sync::Arc::new(create_wgpu_queued_native_buffer(
                                self.graphics.device(),
                                Some("Neo queued native vertex buffer"),
                                &vertex_buffer.bytes,
                                wgpu::BufferUsages::VERTEX,
                            )?),
                        })
                    })
                    .collect::<Result<Vec<_>, RendererError>>()?;
                let index_buffer = draw
                    .index_buffer
                    .as_ref()
                    .map(|index_buffer| {
                        Ok(WgpuNativePipelinePostPassIndexBuffer {
                            format: wgpu_queued_index_format(index_buffer.format),
                            buffer: std::sync::Arc::new(create_wgpu_queued_native_buffer(
                                self.graphics.device(),
                                Some("Neo queued native index buffer"),
                                &index_buffer.bytes,
                                wgpu::BufferUsages::INDEX,
                            )?),
                        })
                    })
                    .transpose()?;
                Ok(WgpuNativePipelinePostPassSubmission {
                    render_pipeline: std::sync::Arc::clone(&objects.render_pipeline),
                    bind_groups: objects
                        .material_bind_groups
                        .iter()
                        .map(|bind_group| {
                            (
                                bind_group.group,
                                std::sync::Arc::clone(&bind_group.bind_group),
                            )
                        })
                        .collect(),
                    vertices: draw.vertices.clone(),
                    indices: draw.indices.clone(),
                    instances: draw.instances.clone(),
                    vertex_buffers,
                    index_buffer,
                })
            })
            .collect::<Result<Vec<_>, RendererError>>()?;
        let render_result = if queued_native_draws.is_empty() {
            gpu_scene.render(renderer, surface, &queue)
        } else {
            gpu_scene.render_with_post_pass(renderer, surface, &queue, &[], |pass| {
                for submission in &native_post_pass_submissions {
                    bind_wgpu_native_pipeline_post_pass_submission(pass, submission);
                    if let (Some(indices), Some(_)) = (
                        submission.indices.as_ref(),
                        submission.index_buffer.as_ref(),
                    ) {
                        pass.draw_indexed(indices.clone(), 0, submission.instances.clone());
                    } else {
                        pass.draw(submission.vertices.clone(), submission.instances.clone());
                    }
                }
            })
        };
        if let Err(error) = render_result {
            return Err(record_backend_error(&mut self.device_status, error));
        }
        self.last_submission_index = surface.last_submission_index();

        let queue_stats = queue.stats();
        let gpu_stats = renderer.last_stats();
        let backend_pipeline_objects = renderer.render_pipeline_count();
        let backend_pipeline_layouts = renderer.render_pipeline_layout_count();
        let rhi_executed_pass_labels =
            native_pass_labels_from_wgpu_metrics(scene, queue_stats.item_count, &gpu_stats);
        let native_draw_call_count = queued_native_draws.len() as u32;
        let mut stats = frame_stats_from_wgpu_metrics(
            self.frame_index,
            rhi_executed_pass_labels,
            queue_stats,
            gpu_stats.clone(),
            backend_pipeline_objects,
            backend_pipeline_layouts,
            ResourceReclaimPolicy::FrameLatency {
                frames: self.config.frame_latency.max(1),
            },
            self.config.gpu_profiling,
        );
        record_native_post_pass_draws(&mut stats, native_draw_call_count);
        merge_wgpu_pipeline_cache_stats(
            &mut stats.pipeline_cache,
            self.native_pipeline_cache.stats(),
        );
        self.queue_post_pass_buffer_tombstones(native_post_pass_submissions);
        self.frame_index += 1;
        self.last_stats = Some(stats.clone());
        Ok(stats)
    }

    pub fn last_frame_stats(&self) -> Option<&FrameStats> {
        self.last_stats.as_ref()
    }

    pub fn device_status(&self) -> DeviceStatus {
        self.device_status
    }

    pub fn wait_for_gpu(&self) {
        if let Some(index) = self.last_submission_index.clone() {
            let _ = self
                .graphics
                .device()
                .poll(wgpu::Maintain::WaitForSubmissionIndex(index));
        } else {
            let _ = self.graphics.device().poll(wgpu::Maintain::Wait);
        }
    }

    pub fn poll_submissions(&self) -> bool {
        self.graphics
            .device()
            .poll(wgpu::Maintain::Poll)
            .is_queue_empty()
    }

    pub fn last_submission_index(&self) -> Option<wgpu::SubmissionIndex> {
        self.last_submission_index.clone()
    }

    pub fn set_gpu_profiling_enabled(&mut self, enabled: bool) {
        self.config.gpu_profiling = enabled;
    }

    pub fn config(&self) -> &RendererConfig {
        &self.config
    }

    pub fn surface_color_format(&self) -> Option<TextureFormat> {
        self.surface
            .as_ref()
            .and_then(|surface| texture_format_from_wgpu_surface(surface.format()))
    }

    pub fn surface_frame_readback_supported(&self) -> bool {
        self.surface
            .as_ref()
            .is_some_and(WgpuSurface::surface_readback_supported)
    }

    pub fn surface_frame_readback_enabled(&self) -> bool {
        self.surface
            .as_ref()
            .is_some_and(WgpuSurface::frame_readback_enabled)
    }

    pub fn set_surface_frame_readback_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<(), RendererError> {
        let Some(surface) = &mut self.surface else {
            return Err(RendererError::Validation(
                "wgpu renderer was created without a surface".to_owned(),
            ));
        };
        surface
            .set_frame_readback_enabled(enabled)
            .map_err(map_backend_error)
    }

    pub fn last_surface_frame_readback(&self) -> Option<&WgpuFrameReadback> {
        self.surface
            .as_ref()
            .and_then(WgpuSurface::last_frame_readback)
    }

    pub fn has_pending_surface_frame_readback(&self) -> bool {
        self.surface
            .as_ref()
            .is_some_and(WgpuSurface::has_pending_frame_readback)
    }

    pub fn resolve_pending_surface_frame_readback(&mut self) -> Result<bool, RendererError> {
        let Some(surface) = &mut self.surface else {
            return Ok(false);
        };
        surface
            .resolve_pending_frame_readback()
            .map_err(map_backend_error)
    }

    pub fn try_resolve_pending_surface_frame_readback(&mut self) -> Result<bool, RendererError> {
        let Some(surface) = &mut self.surface else {
            return Ok(false);
        };
        surface
            .try_resolve_pending_frame_readback()
            .map_err(map_backend_error)
    }

    pub fn native_pipeline_cache_stats(&self) -> PipelineCacheStats {
        let mut stats = self.native_pipeline_cache.stats();
        stats.backend_objects = self.native_render_pipeline_objects.len();
        stats
    }

    pub fn backend_resource_retirement_stats(&self) -> WgpuBackendResourceRetirementStats {
        self.backend_resource_retirement_stats
    }

    pub fn poll_backend_resource_retirements(&mut self) -> WgpuBackendResourceRetirementStats {
        self.clear_backend_retired_this_poll();
        let queue_empty = self.poll_submissions();
        let mut completed_submission_order = self.latest_completed_submission_order;
        let (completed_from_trackers, has_active_completion_trackers) =
            self.poll_submission_completions();
        if let Some(order) = completed_from_trackers {
            completed_submission_order = Some(completed_submission_order.unwrap_or(0).max(order));
        }
        if queue_empty {
            if let Some(latest_submission_order) = self.latest_backend_tombstone_submission_order() {
                completed_submission_order = Some(
                    completed_submission_order
                        .unwrap_or(0)
                        .max(latest_submission_order),
                );
            }
        }
        self.latest_completed_submission_order = completed_submission_order;
        self.backend_resource_retirement_stats
            .nonblocking_submission_index_poll_supported = has_active_completion_trackers;
        self.backend_resource_retirement_stats
            .queue_empty_poll_fallback = true;
        self.backend_resource_retirement_stats
            .last_poll_used_queue_empty_fallback = queue_empty;
        self.backend_resource_retirement_stats.last_poll_queue_empty = queue_empty;
        self.backend_resource_retirement_stats
            .last_poll_completed_submission_index_recorded = completed_submission_order.is_some()
            || (queue_empty && self.last_submission_index.is_some());
        if queue_empty || completed_submission_order.is_some() {
            self.retire_backend_resource_tombstones(completed_submission_order, queue_empty);
        }
        self.refresh_backend_resource_tombstone_stats();
        self.backend_resource_retirement_stats
    }

    pub fn record_native_pipeline_ready(
        &mut self,
        key: PipelineKey,
        shader_interface_layout_hash: u64,
    ) {
        self.native_pipeline_cache
            .record_ready_pipeline(key, shader_interface_layout_hash);
    }

    pub fn insert_native_pipeline_objects(
        &mut self,
        key: PipelineKey,
        shader_interface_layout_hash: u64,
        objects: WgpuNativePipelineObjects,
    ) {
        self.native_pipeline_cache
            .record_ready_pipeline(key, shader_interface_layout_hash);
        if let Some(previous) = self.native_pipeline_objects.insert(key, objects) {
            let previous_render_pipeline_key = previous.render_pipeline_key;
            let still_referenced = self
                .native_pipeline_objects
                .values()
                .any(|other| other.render_pipeline_key == previous_render_pipeline_key);
            if !still_referenced {
                self.native_render_pipeline_objects
                    .remove(&previous_render_pipeline_key);
            }
            self.queue_backend_resource_tombstone(previous);
        }
    }

    pub fn native_pipeline_objects(&self, key: PipelineKey) -> Option<&WgpuNativePipelineObjects> {
        self.native_pipeline_objects.get(&key)
    }

    pub fn tag_native_pipeline_material(
        &mut self,
        key: PipelineKey,
        material: MaterialHandle,
    ) -> Result<(), RendererError> {
        let Some(objects) = self.native_pipeline_objects.get_mut(&key) else {
            return Err(RendererError::Validation(
                "wgpu native pipeline objects do not exist".to_owned(),
            ));
        };
        objects.material = Some(material);
        Ok(())
    }

    pub fn native_pipeline_objects_for_submission(
        &mut self,
        key: PipelineKey,
    ) -> Result<&WgpuNativePipelineObjects, RendererError> {
        if !self.native_pipeline_objects.contains_key(&key) {
            return Err(RendererError::Validation(
                "wgpu native pipeline objects do not exist".to_owned(),
            ));
        }
        self.native_pipeline_cache
            .mark_used(key, self.frame_index)?;
        self.native_pipeline_objects.get(&key).ok_or_else(|| {
            RendererError::Validation("wgpu native pipeline objects do not exist".to_owned())
        })
    }

    pub fn mark_native_pipeline_used(&mut self, key: PipelineKey) -> Result<(), RendererError> {
        self.native_pipeline_cache.mark_used(key, self.frame_index)
    }

    pub fn invalidate_native_pipeline(&mut self, key: PipelineKey) -> bool {
        if let Some(objects) = self.native_pipeline_objects.remove(&key) {
            let still_referenced = self
                .native_pipeline_objects
                .values()
                .any(|other| other.render_pipeline_key == objects.render_pipeline_key);
            if !still_referenced {
                self.native_render_pipeline_objects
                    .remove(&objects.render_pipeline_key);
            }
            self.queue_backend_resource_tombstone(objects);
        }
        self.native_pipeline_cache.invalidate(key)
    }

    pub fn invalidate_native_pipelines_for_shader(&mut self, shader: ShaderHandle) -> usize {
        let keys = self
            .native_pipeline_objects
            .keys()
            .copied()
            .filter(|key| key.shader == shader)
            .collect::<Vec<_>>();
        self.invalidate_native_pipeline_keys(keys)
    }

    pub fn invalidate_native_pipelines_for_material_template(
        &mut self,
        template: MaterialTemplateHandle,
    ) -> usize {
        let keys = self
            .native_pipeline_objects
            .keys()
            .copied()
            .filter(|key| key.material_template == template)
            .collect::<Vec<_>>();
        self.invalidate_native_pipeline_keys(keys)
    }

    pub fn invalidate_native_pipelines_for_material(&mut self, material: MaterialHandle) -> usize {
        let keys = self
            .native_pipeline_objects
            .iter()
            .filter_map(|(key, objects)| (objects.material == Some(material)).then_some(*key))
            .collect::<Vec<_>>();
        self.invalidate_native_pipeline_keys(keys)
    }

    fn invalidate_native_pipeline_keys(&mut self, keys: Vec<PipelineKey>) -> usize {
        keys.into_iter()
            .filter(|key| self.invalidate_native_pipeline(*key))
            .count()
    }

    fn queue_backend_resource_tombstone(&mut self, objects: WgpuNativePipelineObjects) {
        let fence = self.current_backend_fence();
        self.invalidate_backend_retirement_poll_gate();
        self.backend_resource_tombstones
            .push(WgpuBackendResourceTombstone {
                fence: Some(fence),
                native_pipeline_objects: vec![objects],
                shader_variant_modules: Vec::new(),
                material_textures: Vec::new(),
                material_samplers: Vec::new(),
                post_pass_buffers: Vec::new(),
            });
        self.refresh_backend_resource_tombstone_stats();
    }

    fn queue_shader_variant_module_tombstone(&mut self, modules: Vec<wgpu::ShaderModule>) {
        let fence = self.current_backend_fence();
        self.invalidate_backend_retirement_poll_gate();
        self.backend_resource_tombstones
            .push(WgpuBackendResourceTombstone {
                fence: Some(fence),
                native_pipeline_objects: Vec::new(),
                shader_variant_modules: modules,
                material_textures: Vec::new(),
                material_samplers: Vec::new(),
                post_pass_buffers: Vec::new(),
            });
        self.refresh_backend_resource_tombstone_stats();
    }

    fn queue_material_texture_tombstone(&mut self, binding: WgpuMaterialTextureBinding) {
        let fence = self.current_backend_fence();
        self.invalidate_backend_retirement_poll_gate();
        self.backend_resource_tombstones
            .push(WgpuBackendResourceTombstone {
                fence: Some(fence),
                native_pipeline_objects: Vec::new(),
                shader_variant_modules: Vec::new(),
                material_textures: vec![binding],
                material_samplers: Vec::new(),
                post_pass_buffers: Vec::new(),
            });
        self.refresh_backend_resource_tombstone_stats();
    }

    fn queue_material_sampler_tombstone(&mut self, binding: WgpuMaterialSamplerBinding) {
        let fence = self.current_backend_fence();
        self.invalidate_backend_retirement_poll_gate();
        self.backend_resource_tombstones
            .push(WgpuBackendResourceTombstone {
                fence: Some(fence),
                native_pipeline_objects: Vec::new(),
                shader_variant_modules: Vec::new(),
                material_textures: Vec::new(),
                material_samplers: vec![binding],
                post_pass_buffers: Vec::new(),
            });
        self.refresh_backend_resource_tombstone_stats();
    }

    fn queue_post_pass_buffer_tombstones(
        &mut self,
        submissions: Vec<WgpuNativePipelinePostPassSubmission>,
    ) {
        let post_pass_buffers = submissions
            .into_iter()
            .filter_map(|submission| {
                (!submission.vertex_buffers.is_empty() || submission.index_buffer.is_some())
                    .then_some(WgpuBackendPostPassBufferTombstone {
                        vertex_buffers: submission.vertex_buffers,
                        index_buffer: submission.index_buffer,
                    })
            })
            .collect::<Vec<_>>();
        self.queue_post_pass_buffer_tombstone(post_pass_buffers);
    }

    fn queue_post_pass_buffer_tombstone(
        &mut self,
        post_pass_buffers: Vec<WgpuBackendPostPassBufferTombstone>,
    ) {
        if post_pass_buffers.is_empty() {
            return;
        }
        let fence = self.current_backend_fence();
        self.invalidate_backend_retirement_poll_gate();
        self.backend_resource_tombstones
            .push(WgpuBackendResourceTombstone {
                fence: Some(fence),
                native_pipeline_objects: Vec::new(),
                shader_variant_modules: Vec::new(),
                material_textures: Vec::new(),
                material_samplers: Vec::new(),
                post_pass_buffers,
            });
        self.refresh_backend_resource_tombstone_stats();
    }

    fn invalidate_backend_retirement_poll_gate(&mut self) {
        self.backend_resource_retirement_stats.last_poll_queue_empty = false;
        self.backend_resource_retirement_stats
            .retired_after_queue_empty_poll = false;
        self.backend_resource_retirement_stats
            .last_poll_completed_submission_index_recorded = false;
        self.backend_resource_retirement_stats
            .retired_after_completed_submission_index_poll = false;
        self.backend_resource_retirement_stats
            .last_poll_used_queue_empty_fallback = false;
    }

    fn poll_submission_completions(&mut self) -> (Option<u64>, bool) {
        let mut completed_submission_order = None;
        let mut has_active_completion_trackers = false;
        let mut pending_completions = Vec::new();
        for completion in std::mem::take(&mut self.backend_submission_completions) {
            match completion.completion_receiver.try_recv() {
                Ok(()) => {
                    completed_submission_order = Some(
                        completed_submission_order
                            .unwrap_or(0)
                            .max(completion.max_tombstone_submission_order),
                    );
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    has_active_completion_trackers = true;
                    pending_completions.push(completion);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    completed_submission_order = Some(
                        completed_submission_order
                            .unwrap_or(0)
                            .max(completion.max_tombstone_submission_order),
                    );
                }
            }
        }
        self.backend_submission_completions = pending_completions;
        (completed_submission_order, has_active_completion_trackers)
    }

    fn register_submission_completion_tracker(
        &mut self,
        submission_index: wgpu::SubmissionIndex,
        submission_order: u64,
    ) {
        let device = self.graphics.device_handle();
        let (completion_sender, completion_receiver) = std::sync::mpsc::channel();
        let wait_index = submission_index.clone();
        std::thread::spawn(move || {
            let _ = device.poll(wgpu::Maintain::WaitForSubmissionIndex(wait_index));
            let _ = completion_sender.send(());
        });
        self.backend_submission_completions.push(WgpuBackendSubmissionCompletion {
            max_tombstone_submission_order: submission_order,
            completion_receiver,
        });
        self.backend_resource_retirement_stats.nonblocking_submission_index_poll_supported = true;
    }

    fn ensure_submission_fence_order_for_index(&mut self, submission_index: &wgpu::SubmissionIndex) -> u64 {
        if let Some(last_fence_index) = self.last_submission_fence_index.as_ref() {
            if last_fence_index == submission_index {
                if let Some(last_fence_order) = self.last_submission_fence_order {
                    return last_fence_order;
                }
            }
        }
        let order = self.next_backend_submission_order;
        self.next_backend_submission_order = self.next_backend_submission_order.saturating_add(1);
        self.register_submission_completion_tracker(submission_index.clone(), order);
        self.last_submission_fence_index = Some(submission_index.clone());
        self.last_submission_fence_order = Some(order);
        order
    }

    fn current_backend_fence(&mut self) -> WgpuBackendFence {
        let submission_index = self.last_submission_index.clone();
        let submission_order = submission_index
            .as_ref()
            .map(|submission_index| self.ensure_submission_fence_order_for_index(submission_index));
        WgpuBackendFence {
            submission_index,
            submission_order,
        }
    }

    fn latest_backend_tombstone_submission_order(&self) -> Option<u64> {
        self.backend_resource_tombstones
            .iter()
            .filter_map(|tombstone| {
                tombstone
                    .fence
                    .as_ref()
                    .and_then(|fence| fence.submission_order)
            })
            .max()
    }

    fn retire_backend_resource_tombstones(
        &mut self,
        completed_submission_order: Option<u64>,
        queue_empty: bool,
    ) {
        let tombstones = std::mem::take(&mut self.backend_resource_tombstones);
        self.backend_resource_retirement_stats
            .last_poll_used_queue_empty_fallback = queue_empty;
        let mut retired = Vec::new();
        let mut pending = Vec::new();
        for tombstone in tombstones {
            if tombstone.is_completed_by(completed_submission_order, queue_empty) {
                retired.push(tombstone);
            } else {
                pending.push(tombstone);
            }
        }
        self.backend_resource_tombstones = pending;
        let mut retired_stats = WgpuBackendResourceRetirementStats {
            retired_tombstones_this_poll: retired.len(),
            ..WgpuBackendResourceRetirementStats::default()
        };
        for tombstone in &retired {
            accumulate_backend_tombstone_stats(tombstone, &mut retired_stats, true);
        }
        self.backend_resource_retirement_stats
            .retired_tombstones_this_poll = retired_stats.retired_tombstones_this_poll;
        self.backend_resource_retirement_stats
            .retired_tombstones_with_submission_index_this_poll =
            retired_stats.retired_tombstones_with_submission_index_this_poll;
        self.backend_resource_retirement_stats
            .retired_tombstones_without_submission_index_this_poll =
            retired_stats.retired_tombstones_without_submission_index_this_poll;
        self.backend_resource_retirement_stats
            .retired_tombstone_submission_index_coverage_this_poll =
            tombstone_submission_index_coverage(
                retired_stats.retired_tombstones_this_poll,
                retired_stats.retired_tombstones_with_submission_index_this_poll,
                retired_stats.retired_tombstones_without_submission_index_this_poll,
            );
        self.backend_resource_retirement_stats
            .retired_all_tombstones_had_submission_index_this_poll =
            retired_stats.retired_tombstones_this_poll > 0
                && retired_stats.retired_tombstones_without_submission_index_this_poll == 0;
        self.backend_resource_retirement_stats
            .retired_partial_tombstone_submission_index_coverage_this_poll =
            retired_stats.retired_tombstones_with_submission_index_this_poll > 0
                && retired_stats.retired_tombstones_without_submission_index_this_poll > 0;
        self.backend_resource_retirement_stats
            .retired_no_tombstones_had_submission_index_this_poll =
            retired_stats.retired_tombstones_this_poll > 0
                && retired_stats.retired_tombstones_with_submission_index_this_poll == 0;
        self.backend_resource_retirement_stats
            .retired_after_queue_empty_poll =
            queue_empty && retired_stats.retired_tombstones_this_poll > 0;
        self.backend_resource_retirement_stats
            .retired_after_completed_submission_index_poll =
            retired_stats.retired_tombstones_this_poll > 0
                && retired_stats.retired_fence_submission_indices_this_poll > 0
                && completed_submission_order.is_some()
                && self
                    .backend_resource_retirement_stats
                    .last_poll_completed_submission_index_recorded;
        self.backend_resource_retirement_stats
            .retired_native_pipeline_entries_this_poll =
            retired_stats.retired_native_pipeline_entries_this_poll;
        self.backend_resource_retirement_stats
            .retired_render_pipeline_refs_this_poll =
            retired_stats.retired_render_pipeline_refs_this_poll;
        self.backend_resource_retirement_stats
            .retired_shader_modules_this_poll = retired_stats.retired_shader_modules_this_poll;
        self.backend_resource_retirement_stats
            .retired_shader_variant_modules_this_poll =
            retired_stats.retired_shader_variant_modules_this_poll;
        self.backend_resource_retirement_stats
            .retired_material_textures_this_poll =
            retired_stats.retired_material_textures_this_poll;
        self.backend_resource_retirement_stats
            .retired_material_samplers_this_poll =
            retired_stats.retired_material_samplers_this_poll;
        self.backend_resource_retirement_stats
            .retired_fence_objects_this_poll = retired_stats.retired_fence_objects_this_poll;
        self.backend_resource_retirement_stats
            .retired_fence_submission_indices_this_poll =
            retired_stats.retired_fence_submission_indices_this_poll;
        self.backend_resource_retirement_stats
            .retired_fence_objects_without_submission_index_this_poll =
            retired_stats.retired_fence_objects_without_submission_index_this_poll;
        self.backend_resource_retirement_stats
            .retired_post_pass_vertex_buffers_this_poll =
            retired_stats.retired_post_pass_vertex_buffers_this_poll;
        self.backend_resource_retirement_stats
            .retired_post_pass_index_buffers_this_poll =
            retired_stats.retired_post_pass_index_buffers_this_poll;
        self.backend_resource_retirement_stats
            .retired_bind_groups_this_poll = retired_stats.retired_bind_groups_this_poll;
        self.backend_resource_retirement_stats
            .retired_owned_buffers_this_poll = retired_stats.retired_owned_buffers_this_poll;
    }

    fn clear_backend_retired_this_poll(&mut self) {
        self.backend_resource_retirement_stats
            .retired_after_queue_empty_poll = false;
        self.backend_resource_retirement_stats
            .retired_after_completed_submission_index_poll = false;
        self.backend_resource_retirement_stats
            .retired_tombstones_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_tombstones_with_submission_index_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_tombstones_without_submission_index_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_tombstone_submission_index_coverage_this_poll =
            WgpuTombstoneSubmissionIndexCoverage::NotApplicable;
        self.backend_resource_retirement_stats
            .retired_all_tombstones_had_submission_index_this_poll = false;
        self.backend_resource_retirement_stats
            .retired_partial_tombstone_submission_index_coverage_this_poll = false;
        self.backend_resource_retirement_stats
            .retired_no_tombstones_had_submission_index_this_poll = false;
        self.backend_resource_retirement_stats
            .retired_native_pipeline_entries_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_render_pipeline_refs_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_shader_modules_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_shader_variant_modules_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_material_textures_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_material_samplers_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_fence_objects_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_fence_submission_indices_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_fence_objects_without_submission_index_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_post_pass_vertex_buffers_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_post_pass_index_buffers_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_bind_groups_this_poll = 0;
        self.backend_resource_retirement_stats
            .retired_owned_buffers_this_poll = 0;
    }

    fn refresh_backend_resource_tombstone_stats(&mut self) {
        let retired = WgpuBackendResourceRetirementStats {
            last_poll_queue_empty: self.backend_resource_retirement_stats.last_poll_queue_empty,
            retired_after_queue_empty_poll: self
                .backend_resource_retirement_stats
                .retired_after_queue_empty_poll,
            last_poll_completed_submission_index_recorded: self
                .backend_resource_retirement_stats
                .last_poll_completed_submission_index_recorded,
            retired_after_completed_submission_index_poll: self
                .backend_resource_retirement_stats
                .retired_after_completed_submission_index_poll,
            last_poll_used_queue_empty_fallback: self
                .backend_resource_retirement_stats
                .last_poll_used_queue_empty_fallback,
            retired_tombstones_this_poll: self
                .backend_resource_retirement_stats
                .retired_tombstones_this_poll,
            retired_tombstones_with_submission_index_this_poll: self
                .backend_resource_retirement_stats
                .retired_tombstones_with_submission_index_this_poll,
            retired_tombstones_without_submission_index_this_poll: self
                .backend_resource_retirement_stats
                .retired_tombstones_without_submission_index_this_poll,
            retired_tombstone_submission_index_coverage_this_poll: self
                .backend_resource_retirement_stats
                .retired_tombstone_submission_index_coverage_this_poll,
            retired_all_tombstones_had_submission_index_this_poll: self
                .backend_resource_retirement_stats
                .retired_all_tombstones_had_submission_index_this_poll,
            retired_partial_tombstone_submission_index_coverage_this_poll: self
                .backend_resource_retirement_stats
                .retired_partial_tombstone_submission_index_coverage_this_poll,
            retired_no_tombstones_had_submission_index_this_poll: self
                .backend_resource_retirement_stats
                .retired_no_tombstones_had_submission_index_this_poll,
            retired_native_pipeline_entries_this_poll: self
                .backend_resource_retirement_stats
                .retired_native_pipeline_entries_this_poll,
            retired_render_pipeline_refs_this_poll: self
                .backend_resource_retirement_stats
                .retired_render_pipeline_refs_this_poll,
            retired_shader_modules_this_poll: self
                .backend_resource_retirement_stats
                .retired_shader_modules_this_poll,
            retired_shader_variant_modules_this_poll: self
                .backend_resource_retirement_stats
                .retired_shader_variant_modules_this_poll,
            retired_material_textures_this_poll: self
                .backend_resource_retirement_stats
                .retired_material_textures_this_poll,
            retired_material_samplers_this_poll: self
                .backend_resource_retirement_stats
                .retired_material_samplers_this_poll,
            retired_fence_objects_this_poll: self
                .backend_resource_retirement_stats
                .retired_fence_objects_this_poll,
            retired_fence_submission_indices_this_poll: self
                .backend_resource_retirement_stats
                .retired_fence_submission_indices_this_poll,
            retired_fence_objects_without_submission_index_this_poll: self
                .backend_resource_retirement_stats
                .retired_fence_objects_without_submission_index_this_poll,
            retired_post_pass_vertex_buffers_this_poll: self
                .backend_resource_retirement_stats
                .retired_post_pass_vertex_buffers_this_poll,
            retired_post_pass_index_buffers_this_poll: self
                .backend_resource_retirement_stats
                .retired_post_pass_index_buffers_this_poll,
            retired_bind_groups_this_poll: self
                .backend_resource_retirement_stats
                .retired_bind_groups_this_poll,
            retired_owned_buffers_this_poll: self
                .backend_resource_retirement_stats
                .retired_owned_buffers_this_poll,
            ..WgpuBackendResourceRetirementStats::default()
        };
        let mut stats = WgpuBackendResourceRetirementStats {
            tombstones: self.backend_resource_tombstones.len(),
            ..retired
        };
        stats.nonblocking_submission_index_poll_supported =
            self.backend_resource_retirement_stats.nonblocking_submission_index_poll_supported;
        stats.queue_empty_poll_fallback = true;
        for tombstone in &self.backend_resource_tombstones {
            accumulate_backend_tombstone_stats(tombstone, &mut stats, false);
        }
        stats.tombstone_submission_index_coverage = tombstone_submission_index_coverage(
            stats.tombstones,
            stats.tombstones_with_submission_index,
            stats.tombstones_without_submission_index,
        );
        stats.all_tombstones_have_submission_index =
            stats.tombstones > 0 && stats.tombstones_without_submission_index == 0;
        stats.partial_tombstone_submission_index_coverage = stats.tombstones_with_submission_index
            > 0
            && stats.tombstones_without_submission_index > 0;
        stats.no_tombstones_have_submission_index =
            stats.tombstones > 0 && stats.tombstones_with_submission_index == 0;
        self.backend_resource_retirement_stats = stats;
    }

    pub fn queue_native_pipeline_draw(
        &mut self,
        draw: WgpuQueuedNativePipelineDraw,
    ) -> Result<(), RendererError> {
        if !self.native_pipeline_objects.contains_key(&draw.key) {
            return Err(RendererError::Validation(
                "wgpu native pipeline objects do not exist".to_owned(),
            ));
        }
        if draw.indices.is_some() != draw.index_buffer.is_some() {
            return Err(RendererError::Validation(
                "wgpu queued native pipeline indexed draws require both an index range and index buffer".to_owned(),
            ));
        }
        if draw
            .vertex_buffers
            .iter()
            .any(|vertex_buffer| vertex_buffer.bytes.is_empty())
            || draw
                .index_buffer
                .as_ref()
                .is_some_and(|index_buffer| index_buffer.bytes.is_empty())
        {
            return Err(RendererError::Validation(
                "wgpu queued native pipeline draw buffers must not be empty".to_owned(),
            ));
        }
        self.queued_native_pipeline_draws.push(draw);
        Ok(())
    }

    pub fn queued_native_pipeline_draw_count(&self) -> usize {
        self.queued_native_pipeline_draws.len()
    }

    pub fn register_material_texture_binding(
        &mut self,
        handle: TextureHandle,
        view: wgpu::TextureView,
    ) {
        if let Some(binding) = self.material_external_resources.take_texture(handle) {
            self.queue_material_texture_tombstone(binding);
        }
        self.material_external_resources
            .register_texture(handle, view);
    }

    pub fn register_material_sampler_binding(
        &mut self,
        handle: SamplerHandle,
        sampler: wgpu::Sampler,
    ) {
        if let Some(binding) = self.material_external_resources.take_sampler(handle) {
            self.queue_material_sampler_tombstone(binding);
        }
        self.material_external_resources
            .register_sampler(handle, sampler);
    }

    pub fn create_and_register_material_texture_binding(
        &mut self,
        handle: TextureHandle,
        desc: &WgpuMaterialTextureUploadDesc,
    ) -> Result<(), RendererError> {
        let binding = create_wgpu_material_texture_binding(
            self.graphics.device(),
            self.graphics.queue(),
            desc,
        )?;
        if let Some(previous) = self.material_external_resources.take_texture(handle) {
            self.queue_material_texture_tombstone(previous);
        }
        self.material_external_resources
            .register_texture_binding(handle, binding);
        Ok(())
    }

    pub fn create_and_register_material_sampler_binding(
        &mut self,
        handle: SamplerHandle,
        desc: &SamplerDesc,
    ) {
        let sampler = self
            .graphics
            .device()
            .create_sampler(&wgpu_sampler_desc(desc));
        if let Some(binding) = self.material_external_resources.take_sampler(handle) {
            self.queue_material_sampler_tombstone(binding);
        }
        self.material_external_resources
            .register_sampler(handle, sampler);
    }

    pub fn unregister_material_texture_binding(&mut self, handle: TextureHandle) -> bool {
        let Some(binding) = self.material_external_resources.take_texture(handle) else {
            return false;
        };
        self.queue_material_texture_tombstone(binding);
        true
    }

    pub fn unregister_material_sampler_binding(&mut self, handle: SamplerHandle) -> bool {
        let Some(binding) = self.material_external_resources.take_sampler(handle) else {
            return false;
        };
        self.queue_material_sampler_tombstone(binding);
        true
    }

    pub fn material_external_resources(&self) -> &WgpuMaterialExternalResourceRegistry {
        &self.material_external_resources
    }

    pub fn material_external_resource_stats(&self) -> WgpuMaterialExternalResourceStats {
        self.material_external_resources.stats()
    }

    pub fn create_shader_interface_layout_objects(
        &self,
        interface: &ShaderInterfaceDesc,
        label: Option<&str>,
    ) -> Result<WgpuShaderInterfaceLayoutObjects, RendererError> {
        create_wgpu_shader_interface_layout_objects(self.graphics.device(), interface, label)
    }

    pub fn create_render_pipeline(
        &self,
        layout: &wgpu::PipelineLayout,
        desc: WgpuRenderPipelineDesc<'_>,
    ) -> Result<wgpu::RenderPipeline, RendererError> {
        create_wgpu_render_pipeline(self.graphics.device(), layout, desc)
    }

    pub fn create_shader_module(
        &self,
        source: &ShaderSource<'_>,
        label: Option<&str>,
    ) -> Result<wgpu::ShaderModule, RendererError> {
        create_wgpu_shader_module(self.graphics.device(), source, label)
    }

    pub fn compile_and_cache_shader_variant_module(
        &mut self,
        shader: ShaderHandle,
        flags: &[String],
        source: &ShaderSource<'_>,
        label: Option<&str>,
    ) -> Result<bool, RendererError> {
        let key = WgpuShaderVariantModuleKey {
            shader,
            flags: flags.to_vec(),
        };
        if self.shader_variant_modules.contains_key(&key) {
            return Ok(true);
        }
        let module = create_wgpu_shader_module(self.graphics.device(), source, label)?;
        self.shader_variant_modules.insert(key, module);
        Ok(true)
    }

    pub fn shader_variant_module_count(&self) -> usize {
        self.shader_variant_modules.len()
    }

    pub fn invalidate_shader_variant_modules_for_shader(&mut self, shader: ShaderHandle) -> usize {
        let keys = self
            .shader_variant_modules
            .keys()
            .filter(|key| key.shader == shader)
            .cloned()
            .collect::<Vec<_>>();
        let mut modules = Vec::new();
        for key in keys {
            if let Some(module) = self.shader_variant_modules.remove(&key) {
                modules.push(module);
            }
        }
        let removed = modules.len();
        if !modules.is_empty() {
            self.queue_shader_variant_module_tombstone(modules);
        }
        removed
    }

    pub fn create_and_cache_native_render_pipeline(
        &mut self,
        desc: WgpuNativeRenderPipelineBuildDesc<'_>,
    ) -> Result<(), RendererError> {
        self.create_and_cache_native_render_pipeline_with_resource_resolver(desc, |_| {
            Err(RendererError::Validation(
                "material resource resolver is required for texture and sampler bindings"
                    .to_owned(),
            ))
        })
    }

    pub fn create_and_cache_native_render_pipeline_with_render_key(
        &mut self,
        desc: WgpuNativeRenderPipelineBuildDesc<'_>,
        render_pipeline_key: PipelineKey,
    ) -> Result<(), RendererError> {
        self.create_and_cache_native_render_pipeline_with_resource_resolver_and_render_key(
            desc,
            render_pipeline_key,
            |_| {
                Err(RendererError::Validation(
                    "material resource resolver is required for texture and sampler bindings"
                        .to_owned(),
                ))
            },
        )
    }

    pub fn create_and_cache_native_render_pipeline_with_resource_resolver<'a, F>(
        &mut self,
        desc: WgpuNativeRenderPipelineBuildDesc<'a>,
        resolve_external: F,
    ) -> Result<(), RendererError>
    where
        F: FnMut(
            &WgpuMaterialBindGroupResourceEntryPlan,
        ) -> Result<wgpu::BindingResource<'a>, RendererError>,
    {
        let render_pipeline_key = desc.key;
        self.create_and_cache_native_render_pipeline_with_resource_resolver_and_render_key(
            desc,
            render_pipeline_key,
            resolve_external,
        )
    }

    pub fn create_and_cache_native_render_pipeline_with_resource_resolver_and_render_key<'a, F>(
        &mut self,
        desc: WgpuNativeRenderPipelineBuildDesc<'a>,
        render_pipeline_key: PipelineKey,
        mut resolve_external: F,
    ) -> Result<(), RendererError>
    where
        F: FnMut(
            &WgpuMaterialBindGroupResourceEntryPlan,
        ) -> Result<wgpu::BindingResource<'a>, RendererError>,
    {
        let shader_module =
            create_wgpu_shader_module(self.graphics.device(), &desc.shader_source, desc.label)?;
        let layout_objects = create_wgpu_shader_interface_layout_objects(
            self.graphics.device(),
            desc.interface,
            desc.label,
        )?;
        let material_bind_groups = desc
            .material_resource_plan
            .map(|plan| {
                create_wgpu_material_bind_groups_with_owned_buffers_from_plan(
                    self.graphics.device(),
                    &layout_objects.bind_group_layouts,
                    plan,
                    desc.label,
                    &mut resolve_external,
                )
            })
            .transpose()?
            .unwrap_or_default();
        let render_pipeline = if let Some(render_pipeline) = self
            .native_render_pipeline_objects
            .get(&render_pipeline_key)
        {
            std::sync::Arc::clone(render_pipeline)
        } else {
            let render_pipeline = std::sync::Arc::new(create_wgpu_render_pipeline(
                self.graphics.device(),
                &layout_objects.pipeline_layout,
                WgpuRenderPipelineDesc {
                    label: desc.label,
                    shader: &shader_module,
                    vertex_entry: desc.vertex_entry,
                    fragment_entry: desc.fragment_entry,
                    vertex_buffers: desc.vertex_buffers,
                    color_format: desc.color_format,
                    depth_format: desc.depth_format,
                    sample_count: desc.sample_count,
                    depth_write: desc.depth_write,
                    blend: desc.blend,
                },
            )?);
            self.native_render_pipeline_objects
                .insert(render_pipeline_key, std::sync::Arc::clone(&render_pipeline));
            render_pipeline
        };
        self.insert_native_pipeline_objects(
            desc.key,
            desc.shader_interface_layout_hash,
            WgpuNativePipelineObjects {
                shader_module,
                layout_objects,
                material_bind_groups,
                render_pipeline,
                render_pipeline_key,
                material: None,
            },
        );
        Ok(())
    }

    pub fn create_and_cache_native_render_pipeline_with_registered_resources(
        &mut self,
        desc: WgpuNativeRenderPipelineBuildDesc<'_>,
    ) -> Result<(), RendererError> {
        let render_pipeline_key = desc.key;
        self.create_and_cache_native_render_pipeline_with_registered_resources_and_render_key(
            desc,
            render_pipeline_key,
        )
    }

    pub fn create_and_cache_native_render_pipeline_with_registered_resources_and_render_key(
        &mut self,
        desc: WgpuNativeRenderPipelineBuildDesc<'_>,
        render_pipeline_key: PipelineKey,
    ) -> Result<(), RendererError> {
        let shader_module =
            create_wgpu_shader_module(self.graphics.device(), &desc.shader_source, desc.label)?;
        let layout_objects = create_wgpu_shader_interface_layout_objects(
            self.graphics.device(),
            desc.interface,
            desc.label,
        )?;
        let material_bind_groups = desc
            .material_resource_plan
            .map(|plan| {
                create_wgpu_material_bind_groups_with_owned_buffers_from_plan(
                    self.graphics.device(),
                    &layout_objects.bind_group_layouts,
                    plan,
                    desc.label,
                    |entry| self.material_external_resources.resolve(entry),
                )
            })
            .transpose()?
            .unwrap_or_default();
        let render_pipeline = if let Some(render_pipeline) = self
            .native_render_pipeline_objects
            .get(&render_pipeline_key)
        {
            std::sync::Arc::clone(render_pipeline)
        } else {
            let render_pipeline = std::sync::Arc::new(create_wgpu_render_pipeline(
                self.graphics.device(),
                &layout_objects.pipeline_layout,
                WgpuRenderPipelineDesc {
                    label: desc.label,
                    shader: &shader_module,
                    vertex_entry: desc.vertex_entry,
                    fragment_entry: desc.fragment_entry,
                    vertex_buffers: desc.vertex_buffers,
                    color_format: desc.color_format,
                    depth_format: desc.depth_format,
                    sample_count: desc.sample_count,
                    depth_write: desc.depth_write,
                    blend: desc.blend,
                },
            )?);
            self.native_render_pipeline_objects
                .insert(render_pipeline_key, std::sync::Arc::clone(&render_pipeline));
            render_pipeline
        };
        self.insert_native_pipeline_objects(
            desc.key,
            desc.shader_interface_layout_hash,
            WgpuNativePipelineObjects {
                shader_module,
                layout_objects,
                material_bind_groups,
                render_pipeline,
                render_pipeline_key,
                material: None,
            },
        );
        Ok(())
    }

    pub fn submit_native_pipeline_draw_to_view(
        &mut self,
        desc: WgpuNativePipelineDrawDesc<'_>,
    ) -> Result<WgpuNativePipelineSubmissionInfo, RendererError> {
        self.native_pipeline_cache
            .mark_used(desc.key, self.frame_index)?;
        let objects = self.native_pipeline_objects.get(&desc.key).ok_or_else(|| {
            RendererError::Validation("wgpu native pipeline objects do not exist".to_owned())
        })?;
        let mut encoder = self
            .graphics
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: desc.label });
        let info = {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: desc.label,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: desc.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(desc.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let mut info = bind_wgpu_native_pipeline_for_render_pass(&mut pass, objects);
            info.vertex_count = desc.vertices.end.saturating_sub(desc.vertices.start);
            info.instance_count = desc.instances.end.saturating_sub(desc.instances.start);
            pass.draw(desc.vertices, desc.instances);
            info
        };
        self.last_submission_index = Some(self.graphics.queue().submit(Some(encoder.finish())));
        Ok(info)
    }
}

impl WgpuNativePipelineCacheMetadata {
    pub fn record_ready_pipeline(&mut self, key: PipelineKey, shader_interface_layout_hash: u64) {
        self.entries
            .entry(key)
            .or_insert(WgpuNativePipelineCacheEntryMetadata {
                key,
                shader_interface_layout_hash,
                last_used_frame: None,
                used_this_frame: false,
            })
            .shader_interface_layout_hash = shader_interface_layout_hash;
    }

    pub fn mark_used(&mut self, key: PipelineKey, frame_index: u64) -> Result<(), RendererError> {
        let Some(entry) = self.entries.get_mut(&key) else {
            return Err(RendererError::Validation(
                "wgpu native pipeline cache entry does not exist".to_owned(),
            ));
        };
        entry.last_used_frame = Some(frame_index);
        entry.used_this_frame = true;
        Ok(())
    }

    pub fn begin_frame(&mut self) {
        self.invalidated_this_frame = 0;
        for entry in self.entries.values_mut() {
            entry.used_this_frame = false;
        }
    }

    pub fn invalidate(&mut self, key: PipelineKey) -> bool {
        let removed = self.entries.remove(&key).is_some();
        if removed {
            self.invalidated_this_frame = self.invalidated_this_frame.saturating_add(1);
        }
        removed
    }

    pub fn clear(&mut self) {
        let removed = self.entries.len() as u32;
        self.entries.clear();
        self.invalidated_this_frame = self.invalidated_this_frame.saturating_add(removed);
    }

    pub fn entry(&self, key: PipelineKey) -> Option<&WgpuNativePipelineCacheEntryMetadata> {
        self.entries.get(&key)
    }

    pub fn entries(&self) -> Vec<WgpuNativePipelineCacheEntryMetadata> {
        let mut entries = self.entries.values().cloned().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.key.shader_interface_sort_key());
        entries
    }

    pub fn stats(&self) -> PipelineCacheStats {
        let mut shader_interface_layouts = std::collections::HashSet::new();
        for entry in self.entries.values() {
            if entry.shader_interface_layout_hash != 0 {
                shader_interface_layouts.insert(entry.shader_interface_layout_hash);
            }
        }
        PipelineCacheStats {
            total: self.entries.len(),
            ready: self.entries.len(),
            backend_objects: self.entries.len(),
            shader_interface_layouts: shader_interface_layouts.len(),
            entries_used_this_frame: self
                .entries
                .values()
                .filter(|entry| entry.used_this_frame)
                .count(),
            ready_unused_entries: self
                .entries
                .values()
                .filter(|entry| !entry.used_this_frame)
                .count(),
            invalidated_this_frame: self.invalidated_this_frame,
            ..PipelineCacheStats::default()
        }
    }
}

fn accumulate_backend_tombstone_stats(
    tombstone: &WgpuBackendResourceTombstone,
    stats: &mut WgpuBackendResourceRetirementStats,
    retired: bool,
) {
    if let Some(fence) = &tombstone.fence {
        let has_submission_index = fence.has_submission_index();
        if retired {
            stats.retired_fence_objects_this_poll =
                stats.retired_fence_objects_this_poll.saturating_add(1);
            if has_submission_index {
                stats.retired_tombstones_with_submission_index_this_poll = stats
                    .retired_tombstones_with_submission_index_this_poll
                    .saturating_add(1);
                stats.retired_fence_submission_indices_this_poll = stats
                    .retired_fence_submission_indices_this_poll
                    .saturating_add(1);
            } else {
                stats.retired_tombstones_without_submission_index_this_poll = stats
                    .retired_tombstones_without_submission_index_this_poll
                    .saturating_add(1);
                stats.retired_fence_objects_without_submission_index_this_poll = stats
                    .retired_fence_objects_without_submission_index_this_poll
                    .saturating_add(1);
            }
        } else {
            stats.fence_objects = stats.fence_objects.saturating_add(1);
            if has_submission_index {
                stats.tombstones_with_submission_index =
                    stats.tombstones_with_submission_index.saturating_add(1);
                stats.tombstones_waiting_for_submission_index = stats
                    .tombstones_waiting_for_submission_index
                    .saturating_add(1);
                stats.fence_submission_indices = stats.fence_submission_indices.saturating_add(1);
            } else {
                stats.tombstones_without_submission_index =
                    stats.tombstones_without_submission_index.saturating_add(1);
                stats.tombstones_waiting_for_queue_empty =
                    stats.tombstones_waiting_for_queue_empty.saturating_add(1);
                stats.fence_objects_without_submission_index = stats
                    .fence_objects_without_submission_index
                    .saturating_add(1);
            }
        }
    }
    if retired {
        stats.retired_shader_variant_modules_this_poll = stats
            .retired_shader_variant_modules_this_poll
            .saturating_add(tombstone.shader_variant_modules.len());
        stats.retired_material_textures_this_poll = stats
            .retired_material_textures_this_poll
            .saturating_add(tombstone.material_textures.len());
        stats.retired_material_samplers_this_poll = stats
            .retired_material_samplers_this_poll
            .saturating_add(tombstone.material_samplers.len());
        for buffers in &tombstone.post_pass_buffers {
            stats.retired_post_pass_vertex_buffers_this_poll = stats
                .retired_post_pass_vertex_buffers_this_poll
                .saturating_add(buffers.vertex_buffers.len());
            if buffers.index_buffer.is_some() {
                stats.retired_post_pass_index_buffers_this_poll = stats
                    .retired_post_pass_index_buffers_this_poll
                    .saturating_add(1);
            }
        }
    } else {
        stats.shader_variant_modules = stats
            .shader_variant_modules
            .saturating_add(tombstone.shader_variant_modules.len());
        stats.material_textures = stats
            .material_textures
            .saturating_add(tombstone.material_textures.len());
        stats.material_samplers = stats
            .material_samplers
            .saturating_add(tombstone.material_samplers.len());
        for buffers in &tombstone.post_pass_buffers {
            stats.post_pass_vertex_buffers = stats
                .post_pass_vertex_buffers
                .saturating_add(buffers.vertex_buffers.len());
            if buffers.index_buffer.is_some() {
                stats.post_pass_index_buffers = stats.post_pass_index_buffers.saturating_add(1);
            }
        }
    }
    for objects in &tombstone.native_pipeline_objects {
        let bind_groups = objects.material_bind_groups.len();
        let owned_buffers = objects
            .material_bind_groups
            .iter()
            .map(|bind_group| bind_group.owned_buffers.len())
            .sum::<usize>();
        if retired {
            stats.retired_native_pipeline_entries_this_poll = stats
                .retired_native_pipeline_entries_this_poll
                .saturating_add(1);
            stats.retired_render_pipeline_refs_this_poll = stats
                .retired_render_pipeline_refs_this_poll
                .saturating_add(1);
            stats.retired_shader_modules_this_poll =
                stats.retired_shader_modules_this_poll.saturating_add(1);
            stats.retired_bind_groups_this_poll = stats
                .retired_bind_groups_this_poll
                .saturating_add(bind_groups);
            stats.retired_owned_buffers_this_poll = stats
                .retired_owned_buffers_this_poll
                .saturating_add(owned_buffers);
        } else {
            stats.native_pipeline_entries = stats.native_pipeline_entries.saturating_add(1);
            stats.render_pipeline_refs = stats.render_pipeline_refs.saturating_add(1);
            stats.shader_modules = stats.shader_modules.saturating_add(1);
            stats.bind_groups = stats.bind_groups.saturating_add(bind_groups);
            stats.owned_buffers = stats.owned_buffers.saturating_add(owned_buffers);
        }
    }
}

impl WgpuMaterialExternalResourceRegistry {
    pub fn register_texture(&mut self, handle: TextureHandle, view: wgpu::TextureView) {
        self.textures.insert(
            handle,
            WgpuMaterialTextureBinding {
                _texture: None,
                view,
                generated_mips: 0,
            },
        );
    }

    pub fn register_texture_binding(
        &mut self,
        handle: TextureHandle,
        binding: WgpuMaterialTextureBinding,
    ) {
        self.textures.insert(handle, binding);
    }

    pub fn register_sampler(&mut self, handle: SamplerHandle, sampler: wgpu::Sampler) {
        self.samplers
            .insert(handle, WgpuMaterialSamplerBinding { sampler });
    }

    pub fn take_texture(&mut self, handle: TextureHandle) -> Option<WgpuMaterialTextureBinding> {
        self.textures.remove(&handle)
    }

    pub fn unregister_texture(&mut self, handle: TextureHandle) -> bool {
        self.take_texture(handle).is_some()
    }

    pub fn take_sampler(&mut self, handle: SamplerHandle) -> Option<WgpuMaterialSamplerBinding> {
        self.samplers.remove(&handle)
    }

    pub fn unregister_sampler(&mut self, handle: SamplerHandle) -> bool {
        self.take_sampler(handle).is_some()
    }

    pub fn stats(&self) -> WgpuMaterialExternalResourceStats {
        WgpuMaterialExternalResourceStats {
            texture_bindings: self.textures.len(),
            sampler_bindings: self.samplers.len(),
            total_bindings: self.textures.len().saturating_add(self.samplers.len()),
        }
    }

    pub fn resolve<'a>(
        &'a self,
        entry: &WgpuMaterialBindGroupResourceEntryPlan,
    ) -> Result<wgpu::BindingResource<'a>, RendererError> {
        match entry.resource {
            WgpuMaterialBindingResource::Texture(handle) => self
                .textures
                .get(&handle)
                .map(|binding| wgpu::BindingResource::TextureView(&binding.view))
                .ok_or_else(|| RendererError::InvalidHandle {
                    kind: crate::ResourceKind::Texture,
                    raw: handle.raw().get(),
                }),
            WgpuMaterialBindingResource::Sampler(handle) => self
                .samplers
                .get(&handle)
                .map(|binding| wgpu::BindingResource::Sampler(&binding.sampler))
                .ok_or_else(|| RendererError::InvalidHandle {
                    kind: crate::ResourceKind::Sampler,
                    raw: handle.raw().get(),
                }),
            WgpuMaterialBindingResource::BufferBytes { .. } => Err(RendererError::Validation(
                "buffer-backed material resources are owned by the bind group creator".to_owned(),
            )),
        }
    }
}

trait WgpuPipelineKeySort {
    fn shader_interface_sort_key(self) -> (u64, u64, u64, u64);
}

impl WgpuPipelineKeySort for PipelineKey {
    fn shader_interface_sort_key(self) -> (u64, u64, u64, u64) {
        (
            self.shader.raw().get(),
            self.material_template.raw().get(),
            self.vertex_layout_hash,
            self.render_state_hash,
        )
    }
}

pub fn wgpu_shader_interface_layout_plan(
    interface: &ShaderInterfaceDesc,
) -> Result<WgpuShaderInterfaceLayoutPlan, RendererError> {
    let mut bind_groups = std::collections::BTreeMap::<u32, Vec<wgpu::BindGroupLayoutEntry>>::new();
    for resource in &interface.resources {
        bind_groups
            .entry(resource.group)
            .or_default()
            .push(wgpu::BindGroupLayoutEntry {
                binding: resource.binding,
                visibility: wgpu_shader_stages(resource.visibility)?,
                ty: wgpu_binding_type(resource.binding_class, &resource.ty)?,
                count: None,
            });
    }

    let bind_groups = bind_groups
        .into_iter()
        .map(|(group, mut entries)| {
            entries.sort_by_key(|entry| entry.binding);
            WgpuShaderBindGroupLayoutPlan { group, entries }
        })
        .collect();

    let push_constants = interface
        .push_constants
        .iter()
        .map(|push| {
            Ok(wgpu::PushConstantRange {
                stages: wgpu_shader_stages(push.stages)?,
                range: push.range.clone(),
            })
        })
        .collect::<Result<Vec<_>, RendererError>>()?;

    Ok(WgpuShaderInterfaceLayoutPlan {
        bind_groups,
        push_constants,
    })
}

pub fn create_wgpu_shader_interface_layout_objects(
    device: &wgpu::Device,
    interface: &ShaderInterfaceDesc,
    label: Option<&str>,
) -> Result<WgpuShaderInterfaceLayoutObjects, RendererError> {
    let plan = wgpu_shader_interface_layout_plan(interface)?;
    create_wgpu_shader_interface_layout_objects_from_plan(device, &plan, label)
}

pub fn create_wgpu_shader_interface_layout_objects_from_plan(
    device: &wgpu::Device,
    plan: &WgpuShaderInterfaceLayoutPlan,
    label: Option<&str>,
) -> Result<WgpuShaderInterfaceLayoutObjects, RendererError> {
    let label_prefix = label.unwrap_or("Neo Reflected Shader Interface");
    let bind_group_layouts = plan
        .bind_groups
        .iter()
        .map(|group| {
            let layout_label = format!("{label_prefix} Bind Group {} Layout", group.group);
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&layout_label),
                entries: &group.entries,
            });
            WgpuShaderBindGroupLayoutObject {
                group: group.group,
                entry_count: group.entries.len(),
                layout,
            }
        })
        .collect::<Vec<_>>();

    let pipeline_layout_label = format!("{label_prefix} Pipeline Layout");
    let bind_group_layout_refs = bind_group_layouts
        .iter()
        .map(|layout| &layout.layout)
        .collect::<Vec<_>>();
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&pipeline_layout_label),
        bind_group_layouts: &bind_group_layout_refs,
        push_constant_ranges: &plan.push_constants,
    });

    Ok(WgpuShaderInterfaceLayoutObjects {
        bind_group_layouts,
        pipeline_layout,
    })
}

pub fn wgpu_material_bind_group_resource_plan(
    interface: &ShaderInterfaceDesc,
    parameters: &[MaterialParameter],
) -> Result<WgpuMaterialBindGroupResourcePlan, RendererError> {
    let mut parameter_names = std::collections::HashSet::new();
    let mut bind_groups =
        std::collections::BTreeMap::<u32, Vec<WgpuMaterialBindGroupResourceEntryPlan>>::new();

    for parameter in parameters {
        if !parameter_names.insert(parameter.name.as_str()) {
            return Err(RendererError::MaterialParameterMismatch(format!(
                "duplicate material parameter '{}'",
                parameter.name
            )));
        }
        let resource_binding = interface
            .resources
            .iter()
            .find(|binding| binding.name == parameter.name)
            .ok_or_else(|| {
                RendererError::MaterialParameterMismatch(format!(
                    "material parameter '{}' does not match any reflected shader binding",
                    parameter.name
                ))
            })?;
        let resource = wgpu_material_binding_resource(
            &parameter.value,
            resource_binding.binding_class,
            &resource_binding.ty,
        )
        .ok_or_else(|| {
            RendererError::MaterialParameterMismatch(format!(
                "material parameter '{}' does not match shader binding class {:?}",
                parameter.name, resource_binding.binding_class
            ))
        })?;
        bind_groups.entry(resource_binding.group).or_default().push(
            WgpuMaterialBindGroupResourceEntryPlan {
                name: parameter.name.clone(),
                binding: resource_binding.binding,
                binding_class: resource_binding.binding_class,
                binding_type: resource_binding.ty.clone(),
                resource,
            },
        );
    }

    let groups = bind_groups
        .into_iter()
        .map(|(group, mut entries)| {
            entries.sort_by_key(|entry| entry.binding);
            WgpuMaterialBindGroupResourceGroupPlan { group, entries }
        })
        .collect();

    Ok(WgpuMaterialBindGroupResourcePlan { groups })
}

pub fn create_wgpu_material_bind_groups_from_plan<'a, F>(
    device: &wgpu::Device,
    layout_objects: &[WgpuShaderBindGroupLayoutObject],
    plan: &WgpuMaterialBindGroupResourcePlan,
    label: Option<&str>,
    mut resolve: F,
) -> Result<Vec<WgpuMaterialBindGroupObject>, RendererError>
where
    F: FnMut(
        &WgpuMaterialBindGroupResourceEntryPlan,
    ) -> Result<wgpu::BindingResource<'a>, RendererError>,
{
    let label_prefix = label.unwrap_or("Neo Reflected Material");
    let mut bind_groups = Vec::with_capacity(plan.groups.len());
    for group in &plan.groups {
        let layout = layout_objects
            .iter()
            .find(|layout| layout.group == group.group)
            .ok_or_else(|| {
                RendererError::Validation(format!(
                    "material bind group {} has no matching shader bind group layout",
                    group.group
                ))
            })?;
        if group.entries.len() != layout.entry_count {
            return Err(RendererError::Validation(format!(
                "material bind group {} has {} resources, but shader layout requires {}",
                group.group,
                group.entries.len(),
                layout.entry_count
            )));
        }

        let entries = group
            .entries
            .iter()
            .map(|entry| {
                Ok(wgpu::BindGroupEntry {
                    binding: entry.binding,
                    resource: resolve(entry)?,
                })
            })
            .collect::<Result<Vec<_>, RendererError>>()?;
        let bind_group_label = format!("{label_prefix} Bind Group {}", group.group);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&bind_group_label),
            layout: &layout.layout,
            entries: &entries,
        });
        bind_groups.push(WgpuMaterialBindGroupObject {
            group: group.group,
            entry_count: entries.len(),
            owned_buffers: Vec::new(),
            bind_group: std::sync::Arc::new(bind_group),
        });
    }
    Ok(bind_groups)
}

pub fn create_wgpu_material_bind_groups_with_owned_buffers_from_plan<'a, F>(
    device: &wgpu::Device,
    layout_objects: &[WgpuShaderBindGroupLayoutObject],
    plan: &WgpuMaterialBindGroupResourcePlan,
    label: Option<&str>,
    mut resolve_external: F,
) -> Result<Vec<WgpuMaterialBindGroupObject>, RendererError>
where
    F: FnMut(
        &WgpuMaterialBindGroupResourceEntryPlan,
    ) -> Result<wgpu::BindingResource<'a>, RendererError>,
{
    let label_prefix = label.unwrap_or("Neo Reflected Material");
    let mut bind_groups = Vec::with_capacity(plan.groups.len());
    for group in &plan.groups {
        let layout = layout_objects
            .iter()
            .find(|layout| layout.group == group.group)
            .ok_or_else(|| {
                RendererError::Validation(format!(
                    "material bind group {} has no matching shader bind group layout",
                    group.group
                ))
            })?;
        if group.entries.len() != layout.entry_count {
            return Err(RendererError::Validation(format!(
                "material bind group {} has {} resources, but shader layout requires {}",
                group.group,
                group.entries.len(),
                layout.entry_count
            )));
        }

        let mut owned_buffers = Vec::new();
        for entry in &group.entries {
            if let WgpuMaterialBindingResource::BufferBytes { bytes } = &entry.resource {
                owned_buffers.push(create_wgpu_material_parameter_buffer(
                    device,
                    label_prefix,
                    group.group,
                    entry.binding,
                    entry.binding_class,
                    bytes,
                )?);
            }
        }

        let bind_group = {
            let mut buffer_index = 0usize;
            let entries = group
                .entries
                .iter()
                .map(|entry| {
                    let resource = match &entry.resource {
                        WgpuMaterialBindingResource::BufferBytes { .. } => {
                            let buffer = owned_buffers.get(buffer_index).ok_or_else(|| {
                                RendererError::Validation(
                                    "material buffer resource planning is inconsistent".to_owned(),
                                )
                            })?;
                            buffer_index += 1;
                            buffer.buffer.as_entire_binding()
                        }
                        WgpuMaterialBindingResource::Texture(_)
                        | WgpuMaterialBindingResource::Sampler(_) => resolve_external(entry)?,
                    };
                    Ok(wgpu::BindGroupEntry {
                        binding: entry.binding,
                        resource,
                    })
                })
                .collect::<Result<Vec<_>, RendererError>>()?;
            let bind_group_label = format!("{label_prefix} Bind Group {}", group.group);
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&bind_group_label),
                layout: &layout.layout,
                entries: &entries,
            })
        };

        bind_groups.push(WgpuMaterialBindGroupObject {
            group: group.group,
            entry_count: group.entries.len(),
            owned_buffers,
            bind_group: std::sync::Arc::new(bind_group),
        });
    }
    Ok(bind_groups)
}

pub fn create_wgpu_material_texture_binding(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    desc: &WgpuMaterialTextureUploadDesc,
) -> Result<WgpuMaterialTextureBinding, RendererError> {
    if desc.width == 0 || desc.height == 0 || desc.depth_or_layers == 0 {
        return Err(RendererError::Validation(
            "wgpu material texture extent must be non-zero".to_owned(),
        ));
    }
    if desc.mip_level_count == 0 || desc.sample_count == 0 {
        return Err(RendererError::Validation(
            "wgpu material texture mip level count and sample count must be non-zero".to_owned(),
        ));
    }
    if !desc.sampled_binding && !desc.storage_binding {
        return Err(RendererError::Validation(
            "wgpu material texture must be used as sampled or storage binding".to_owned(),
        ));
    }
    if desc.generate_mips_from_base {
        validate_wgpu_material_texture_gpu_mip_generation_desc(desc)?;
    }
    let mut usage = wgpu::TextureUsages::COPY_DST;
    if desc.sampled_binding {
        usage |= wgpu::TextureUsages::TEXTURE_BINDING;
    }
    if desc.storage_binding {
        usage |= wgpu::TextureUsages::STORAGE_BINDING;
        wgpu_storage_texture_format(desc.format)?;
    }
    if desc.generate_mips_from_base {
        usage |= wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: desc.label.as_deref(),
        size: wgpu::Extent3d {
            width: desc.width,
            height: desc.height,
            depth_or_array_layers: desc.depth_or_layers,
        },
        mip_level_count: desc.mip_level_count,
        sample_count: desc.sample_count,
        dimension: wgpu_texture_dimension(desc.dimension),
        format: wgpu_surface_format(desc.format),
        usage,
        view_formats: &[],
    });
    for upload in &desc.uploads {
        validate_wgpu_material_texture_upload(desc, upload)?;
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: upload.mip_level,
                origin: wgpu::Origin3d {
                    x: upload.origin[0],
                    y: upload.origin[1],
                    z: upload.origin[2],
                },
                aspect: wgpu::TextureAspect::All,
            },
            &upload.bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(upload.bytes_per_row),
                rows_per_image: Some(upload.rows_per_image),
            },
            wgpu::Extent3d {
                width: upload.extent[0],
                height: upload.extent[1],
                depth_or_array_layers: upload.extent[2],
            },
        );
    }
    let generated_mips = if desc.generate_mips_from_base {
        generate_wgpu_material_texture_mips(device, queue, &texture, desc)?
    } else {
        0
    };
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: desc
            .label
            .as_ref()
            .map(|label| format!("{label} View"))
            .as_deref(),
        format: Some(wgpu_surface_format(desc.format)),
        dimension: Some(wgpu_texture_view_dimension(desc.dimension)),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(desc.mip_level_count),
        base_array_layer: 0,
        array_layer_count: Some(desc.depth_or_layers),
    });
    Ok(WgpuMaterialTextureBinding {
        _texture: Some(texture),
        view,
        generated_mips,
    })
}

fn validate_wgpu_material_texture_gpu_mip_generation_desc(
    desc: &WgpuMaterialTextureUploadDesc,
) -> Result<(), RendererError> {
    if desc.sample_count != 1 {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation requires single-sampled textures".to_owned(),
        ));
    }
    if desc.mip_level_count <= 1 {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation requires more than one mip level".to_owned(),
        ));
    }
    if desc.mip_level_count > wgpu_material_texture_full_mip_level_count(desc.width, desc.height, 1)
    {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation mip count exceeds texture extent".to_owned(),
        ));
    }
    if !matches!(
        desc.dimension,
        TextureDimension::D2
            | TextureDimension::D2Array
            | TextureDimension::Cube
            | TextureDimension::CubeArray
    ) {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation currently supports only 2D layer-based textures"
                .to_owned(),
        ));
    }
    match desc.dimension {
        TextureDimension::D2 if desc.depth_or_layers != 1 => {
            return Err(RendererError::Validation(
                "wgpu material GPU mip generation D2 textures require exactly one layer".to_owned(),
            ));
        }
        TextureDimension::Cube => {
            if desc.width != desc.height || desc.depth_or_layers != 6 {
                return Err(RendererError::Validation(
                    "wgpu material GPU mip generation cube textures require square 6-face textures"
                        .to_owned(),
                ));
            }
        }
        TextureDimension::CubeArray => {
            if desc.width != desc.height
                || desc.depth_or_layers < 6
                || desc.depth_or_layers % 6 != 0
            {
                return Err(RendererError::Validation(
                    "wgpu material GPU mip generation cube-array textures require square face counts in multiples of 6"
                        .to_owned(),
                ));
            }
        }
        _ => {}
    }
    if !matches!(
        desc.format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Float
    ) {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation currently supports only filterable 8-bit and float color formats"
                .to_owned(),
        ));
    }
    if desc.uploads.len() != 1 {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation requires exactly one base-level upload".to_owned(),
        ));
    }
    let upload = &desc.uploads[0];
    if upload.mip_level != 0 || upload.origin != [0, 0, 0] {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation requires a base-level origin-zero upload".to_owned(),
        ));
    }
    if upload.extent != [desc.width, desc.height, desc.depth_or_layers] {
        return Err(RendererError::Validation(
            "wgpu material GPU mip generation requires a complete base-level upload".to_owned(),
        ));
    }
    Ok(())
}

fn validate_wgpu_material_texture_upload(
    desc: &WgpuMaterialTextureUploadDesc,
    upload: &WgpuMaterialTextureUpload,
) -> Result<(), RendererError> {
    if upload.mip_level >= desc.mip_level_count {
        return Err(RendererError::Validation(
            "wgpu material texture upload mip level exceeds texture mip levels".to_owned(),
        ));
    }
    if upload.extent.contains(&0) {
        return Err(RendererError::Validation(
            "wgpu material texture upload extent must be non-zero".to_owned(),
        ));
    }
    let mip_width = wgpu_material_texture_mip_extent(desc.width, upload.mip_level);
    let mip_height = wgpu_material_texture_mip_extent(desc.height, upload.mip_level);
    let mip_depth_or_layers = match desc.dimension {
        TextureDimension::D3 => {
            wgpu_material_texture_mip_extent(desc.depth_or_layers, upload.mip_level)
        }
        TextureDimension::D1
        | TextureDimension::D2
        | TextureDimension::D2Array
        | TextureDimension::Cube
        | TextureDimension::CubeArray => desc.depth_or_layers,
    };
    let end_x = upload.origin[0]
        .checked_add(upload.extent[0])
        .ok_or_else(|| {
            RendererError::Validation("wgpu material texture upload x range overflows".to_owned())
        })?;
    let end_y = upload.origin[1]
        .checked_add(upload.extent[1])
        .ok_or_else(|| {
            RendererError::Validation("wgpu material texture upload y range overflows".to_owned())
        })?;
    let end_z = upload.origin[2]
        .checked_add(upload.extent[2])
        .ok_or_else(|| {
            RendererError::Validation("wgpu material texture upload z range overflows".to_owned())
        })?;
    if end_x > mip_width || end_y > mip_height || end_z > mip_depth_or_layers {
        return Err(RendererError::Validation(
            "wgpu material texture upload region exceeds mip extent".to_owned(),
        ));
    }
    let bytes_per_texel = u64::from(wgpu_material_texture_format_bytes_per_pixel(desc.format));
    let min_row_bytes = u64::from(upload.extent[0])
        .checked_mul(bytes_per_texel)
        .ok_or_else(|| {
            RendererError::Validation(
                "wgpu material texture upload row byte size overflows".to_owned(),
            )
        })?;
    if u64::from(upload.bytes_per_row) < min_row_bytes {
        return Err(RendererError::Validation(
            "wgpu material texture upload bytes_per_row is smaller than row size".to_owned(),
        ));
    }
    if upload.rows_per_image < upload.extent[1] {
        return Err(RendererError::Validation(
            "wgpu material texture upload rows_per_image is smaller than upload height".to_owned(),
        ));
    }
    let layer_stride = u64::from(upload.rows_per_image)
        .checked_mul(u64::from(upload.bytes_per_row))
        .ok_or_else(|| {
            RendererError::Validation(
                "wgpu material texture upload layer byte size overflows".to_owned(),
            )
        })?;
    let min_len = u64::from(upload.extent[2] - 1)
        .checked_mul(layer_stride)
        .and_then(|bytes| {
            bytes.checked_add(u64::from(upload.extent[1] - 1) * u64::from(upload.bytes_per_row))
        })
        .and_then(|bytes| bytes.checked_add(min_row_bytes))
        .ok_or_else(|| {
            RendererError::Validation(
                "wgpu material texture upload byte length overflows".to_owned(),
            )
        })?;
    if upload.bytes.len() as u64 != min_len {
        return Err(RendererError::Validation(
            "wgpu material texture upload byte length does not match layout".to_owned(),
        ));
    }
    Ok(())
}

fn wgpu_material_texture_full_mip_level_count(width: u32, height: u32, depth: u32) -> u32 {
    let mut max_extent = width.max(height).max(depth).max(1);
    let mut levels = 1;
    while max_extent > 1 {
        max_extent >>= 1;
        levels += 1;
    }
    levels
}

fn wgpu_material_texture_mip_extent(base: u32, mip_level: u32) -> u32 {
    (base >> mip_level).max(1)
}

fn wgpu_material_texture_format_bytes_per_pixel(format: TextureFormat) -> u32 {
    match format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Bgra8UnormSrgb
        | TextureFormat::Depth32Float => 4,
        TextureFormat::Rgba16Float => 8,
        TextureFormat::Rgba32Float => 16,
    }
}

fn generate_wgpu_material_texture_mips(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    desc: &WgpuMaterialTextureUploadDesc,
) -> Result<u32, RendererError> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Neo Material Texture Mip Generator Shader"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );
    var out: VertexOut;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(source_texture, source_sampler, input.uv, 0.0);
}
"#
            .into(),
        ),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Neo Material Texture Mip Generator Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Neo Material Texture Mip Generator Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let color_targets = [Some(wgpu::ColorTargetState {
        format: wgpu_surface_format(desc.format),
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Neo Material Texture Mip Generator Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &color_targets,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Neo Material Texture Mip Generator Sampler"),
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..wgpu::SamplerDescriptor::default()
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Neo Material Texture Mip Generator Encoder"),
    });
    for mip_level in 1..desc.mip_level_count {
        for array_layer in 0..desc.depth_or_layers {
            let source_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu_surface_format(desc.format)),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: mip_level - 1,
                mip_level_count: Some(1),
                base_array_layer: array_layer,
                array_layer_count: Some(1),
            });
            let target_view = texture.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu_surface_format(desc.format)),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: mip_level,
                mip_level_count: Some(1),
                base_array_layer: array_layer,
                array_layer_count: Some(1),
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Neo Material Texture Mip Generator Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Neo Material Texture Mip Generator Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
    }
    queue.submit([encoder.finish()]);
    Ok((desc.mip_level_count - 1).saturating_mul(desc.depth_or_layers))
}

fn create_wgpu_material_parameter_buffer(
    device: &wgpu::Device,
    label_prefix: &str,
    group: u32,
    binding: u32,
    binding_class: BindingClass,
    bytes: &[u8],
) -> Result<WgpuMaterialOwnedBuffer, RendererError> {
    if bytes.is_empty() {
        return Err(RendererError::MaterialParameterMismatch(format!(
            "material buffer binding group {group} binding {binding} must not be empty"
        )));
    }
    let usage = match binding_class {
        BindingClass::Uniform => wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        BindingClass::Storage => wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        BindingClass::Texture | BindingClass::Sampler => {
            return Err(RendererError::MaterialParameterMismatch(format!(
                "material buffer binding group {group} binding {binding} has non-buffer class {:?}",
                binding_class
            )));
        }
    };
    let size = bytes.len() as u64;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!(
            "{label_prefix} Buffer group {group} binding {binding}"
        )),
        size,
        usage,
        mapped_at_creation: true,
    });
    {
        let mut range = buffer.slice(..).get_mapped_range_mut();
        range[..bytes.len()].copy_from_slice(bytes);
    }
    buffer.unmap();
    Ok(WgpuMaterialOwnedBuffer {
        binding,
        size,
        buffer,
    })
}

fn create_wgpu_queued_native_buffer(
    device: &wgpu::Device,
    label: Option<&str>,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> Result<wgpu::Buffer, RendererError> {
    if bytes.is_empty() {
        return Err(RendererError::Validation(
            "wgpu queued native pipeline buffers must not be empty".to_owned(),
        ));
    }
    let size = align_to(bytes.len() as u64, wgpu::COPY_BUFFER_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label,
        size,
        usage,
        mapped_at_creation: true,
    });
    {
        let mut range = buffer.slice(..).get_mapped_range_mut();
        range[..bytes.len()].copy_from_slice(bytes);
    }
    buffer.unmap();
    Ok(buffer)
}

fn align_to(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

fn wgpu_queued_index_format(format: WgpuQueuedIndexFormat) -> wgpu::IndexFormat {
    match format {
        WgpuQueuedIndexFormat::Uint16 => wgpu::IndexFormat::Uint16,
        WgpuQueuedIndexFormat::Uint32 => wgpu::IndexFormat::Uint32,
    }
}

pub fn create_wgpu_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    desc: WgpuRenderPipelineDesc<'_>,
) -> Result<wgpu::RenderPipeline, RendererError> {
    if desc.vertex_entry.trim().is_empty() {
        return Err(RendererError::Validation(
            "wgpu render pipeline vertex entry point must not be empty".to_owned(),
        ));
    }
    if desc.sample_count == 0 {
        return Err(RendererError::Validation(
            "wgpu render pipeline sample_count must be non-zero".to_owned(),
        ));
    }
    if desc.fragment_entry.is_some() && desc.color_format.is_none() {
        return Err(RendererError::Validation(
            "wgpu render pipeline fragment entry requires a color format".to_owned(),
        ));
    }
    let color_targets = desc
        .color_format
        .map(|format| {
            vec![Some(wgpu::ColorTargetState {
                format: wgpu_surface_format(format),
                blend: desc.blend,
                write_mask: wgpu::ColorWrites::ALL,
            })]
        })
        .unwrap_or_default();
    let fragment = desc.fragment_entry.map(|entry_point| wgpu::FragmentState {
        module: desc.shader,
        entry_point,
        targets: &color_targets,
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    });
    let depth_stencil = desc.depth_format.map(|format| wgpu::DepthStencilState {
        format: wgpu_depth_format(format),
        depth_write_enabled: desc.depth_write,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: desc.label,
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: desc.shader,
                entry_point: desc.vertex_entry,
                buffers: desc.vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil,
            multisample: wgpu::MultisampleState {
                count: desc.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment,
            multiview: None,
        }),
    )
}

pub fn bind_wgpu_native_pipeline_for_render_pass<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    objects: &'pass WgpuNativePipelineObjects,
) -> WgpuNativePipelineSubmissionInfo {
    pass.set_pipeline(objects.render_pipeline.as_ref());
    for bind_group in &objects.material_bind_groups {
        pass.set_bind_group(bind_group.group, bind_group.bind_group.as_ref(), &[]);
    }
    WgpuNativePipelineSubmissionInfo {
        bind_group_count: objects.material_bind_groups.len(),
        vertex_count: 0,
        instance_count: 0,
    }
}

fn bind_wgpu_native_pipeline_post_pass_submission<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    submission: &WgpuNativePipelinePostPassSubmission,
) {
    let pipeline: &'pass wgpu::RenderPipeline = unsafe {
        // SAFETY: `submission` owns an `Arc` to the pipeline and is held by the
        // render_scene stack frame until after the render pass is dropped. The
        // reference is used only for this immediate `RenderPass` binding call.
        &*std::sync::Arc::as_ptr(&submission.render_pipeline)
    };
    pass.set_pipeline(pipeline);
    for (group, bind_group) in &submission.bind_groups {
        let bind_group: &'pass wgpu::BindGroup = unsafe {
            // SAFETY: same lifetime argument as above; the Arc-backed bind group
            // outlives this render pass and is not moved or destroyed during it.
            &*std::sync::Arc::as_ptr(bind_group)
        };
        pass.set_bind_group(*group, bind_group, &[]);
    }
    for vertex_buffer in &submission.vertex_buffers {
        let buffer: &'pass wgpu::Buffer = unsafe {
            // SAFETY: the Arc-backed buffer is owned by the submission vector in
            // the render_scene stack frame and outlives this immediate pass bind.
            &*std::sync::Arc::as_ptr(&vertex_buffer.buffer)
        };
        pass.set_vertex_buffer(vertex_buffer.slot, buffer.slice(..));
    }
    if let Some(index_buffer) = &submission.index_buffer {
        let buffer: &'pass wgpu::Buffer = unsafe {
            // SAFETY: same lifetime argument as the vertex buffers above.
            &*std::sync::Arc::as_ptr(&index_buffer.buffer)
        };
        pass.set_index_buffer(buffer.slice(..), index_buffer.format);
    }
}

pub fn create_wgpu_shader_module(
    device: &wgpu::Device,
    source: &ShaderSource<'_>,
    label: Option<&str>,
) -> Result<wgpu::ShaderModule, RendererError> {
    let source = match source {
        ShaderSource::Wgsl(source) => wgpu::ShaderSource::Wgsl((*source).into()),
        ShaderSource::File(path) => {
            if path.extension().and_then(|extension| extension.to_str()) != Some("wgsl") {
                return Err(RendererError::ShaderCompile(format!(
                    "wgpu shader module creation only supports .wgsl file sources: {path:?}"
                )));
            }
            let source = std::fs::read_to_string(path).map_err(|err| {
                RendererError::ShaderCompile(format!(
                    "failed to read WGSL shader source {path:?}: {err}"
                ))
            })?;
            wgpu::ShaderSource::Wgsl(source.into())
        }
        ShaderSource::SpirV(_)
        | ShaderSource::Msl(_)
        | ShaderSource::Hlsl(_)
        | ShaderSource::Slang(_) => {
            return Err(RendererError::ShaderCompile(
                "wgpu shader module creation currently supports WGSL sources only".to_owned(),
            ));
        }
    };
    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor { label, source }))
}

fn wgpu_material_binding_resource(
    value: &MaterialParameterValue,
    binding_class: BindingClass,
    binding_type: &BindingType,
) -> Option<WgpuMaterialBindingResource> {
    match (binding_class, binding_type, value) {
        (
            BindingClass::Texture,
            BindingType::Texture(_),
            MaterialParameterValue::Texture(texture),
        )
        | (
            BindingClass::Storage,
            BindingType::StorageTexture { .. },
            MaterialParameterValue::Texture(texture),
        ) => Some(WgpuMaterialBindingResource::Texture(*texture)),
        (BindingClass::Sampler, BindingType::Sampler, MaterialParameterValue::Sampler(sampler)) => {
            Some(WgpuMaterialBindingResource::Sampler(*sampler))
        }
        (
            BindingClass::Uniform | BindingClass::Storage,
            BindingType::Buffer,
            MaterialParameterValue::Bytes(bytes),
        ) => Some(WgpuMaterialBindingResource::BufferBytes {
            bytes: bytes.clone(),
        }),
        _ => None,
    }
}

fn wgpu_shader_stages(stages: ShaderStages) -> Result<wgpu::ShaderStages, RendererError> {
    let supported = ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::COMPUTE;
    if stages.0 & !supported.0 != 0 {
        return Err(RendererError::Validation(
            "wgpu backend layout planning only supports vertex, fragment, and compute shader stages"
                .to_owned(),
        ));
    }
    let mut wgpu_stages = wgpu::ShaderStages::empty();
    if stages.contains(ShaderStages::VERTEX) {
        wgpu_stages |= wgpu::ShaderStages::VERTEX;
    }
    if stages.contains(ShaderStages::FRAGMENT) {
        wgpu_stages |= wgpu::ShaderStages::FRAGMENT;
    }
    if stages.contains(ShaderStages::COMPUTE) {
        wgpu_stages |= wgpu::ShaderStages::COMPUTE;
    }
    if wgpu_stages.is_empty() {
        return Err(RendererError::Validation(
            "wgpu backend layout planning requires non-empty shader stage visibility".to_owned(),
        ));
    }
    Ok(wgpu_stages)
}

fn wgpu_binding_type(
    binding_class: BindingClass,
    ty: &BindingType,
) -> Result<wgpu::BindingType, RendererError> {
    match (binding_class, ty) {
        (BindingClass::Uniform, BindingType::Buffer) => Ok(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }),
        (BindingClass::Storage, BindingType::Buffer) => Ok(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        }),
        (BindingClass::Texture, BindingType::Texture(dimension)) => {
            Ok(wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu_texture_view_dimension(*dimension),
                multisampled: false,
            })
        }
        (
            BindingClass::Storage,
            BindingType::StorageTexture {
                dimension,
                format,
                access,
            },
        ) => Ok(wgpu::BindingType::StorageTexture {
            access: wgpu_storage_texture_access(*access),
            format: wgpu_storage_texture_format(*format)?,
            view_dimension: wgpu_texture_view_dimension(*dimension),
        }),
        (BindingClass::Sampler, BindingType::Sampler) => Ok(wgpu::BindingType::Sampler(
            wgpu::SamplerBindingType::Filtering,
        )),
        _ => Err(RendererError::Validation(format!(
            "wgpu backend layout planning cannot map binding class {:?} with type {:?}",
            binding_class, ty
        ))),
    }
}

fn wgpu_storage_texture_access(access: StorageTextureAccess) -> wgpu::StorageTextureAccess {
    match access {
        StorageTextureAccess::ReadOnly => wgpu::StorageTextureAccess::ReadOnly,
        StorageTextureAccess::WriteOnly => wgpu::StorageTextureAccess::WriteOnly,
        StorageTextureAccess::ReadWrite => wgpu::StorageTextureAccess::ReadWrite,
    }
}

fn wgpu_storage_texture_format(
    format: TextureFormat,
) -> Result<wgpu::TextureFormat, RendererError> {
    match format {
        TextureFormat::Rgba8Unorm => Ok(wgpu::TextureFormat::Rgba8Unorm),
        TextureFormat::Rgba16Float => Ok(wgpu::TextureFormat::Rgba16Float),
        TextureFormat::Rgba32Float => Ok(wgpu::TextureFormat::Rgba32Float),
        TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Bgra8UnormSrgb
        | TextureFormat::Depth32Float => Err(RendererError::Validation(format!(
            "texture format {:?} is not supported as a wgpu storage texture binding",
            format
        ))),
    }
}

fn wgpu_texture_view_dimension(dimension: TextureDimension) -> wgpu::TextureViewDimension {
    match dimension {
        TextureDimension::D1 => wgpu::TextureViewDimension::D1,
        TextureDimension::D2 => wgpu::TextureViewDimension::D2,
        TextureDimension::D3 => wgpu::TextureViewDimension::D3,
        TextureDimension::Cube => wgpu::TextureViewDimension::Cube,
        TextureDimension::D2Array => wgpu::TextureViewDimension::D2Array,
        TextureDimension::CubeArray => wgpu::TextureViewDimension::CubeArray,
    }
}

fn wgpu_texture_dimension(dimension: TextureDimension) -> wgpu::TextureDimension {
    match dimension {
        TextureDimension::D1 => wgpu::TextureDimension::D1,
        TextureDimension::D2
        | TextureDimension::D2Array
        | TextureDimension::Cube
        | TextureDimension::CubeArray => wgpu::TextureDimension::D2,
        TextureDimension::D3 => wgpu::TextureDimension::D3,
    }
}

fn wgpu_sampler_desc(desc: &SamplerDesc) -> wgpu::SamplerDescriptor<'_> {
    wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu_address_mode(desc.address_u),
        address_mode_v: wgpu_address_mode(desc.address_v),
        address_mode_w: wgpu_address_mode(desc.address_w),
        mag_filter: wgpu_filter_mode(desc.mag_filter),
        min_filter: wgpu_filter_mode(desc.min_filter),
        mipmap_filter: wgpu_filter_mode(desc.mip_filter),
        lod_min_clamp: desc.lod_min.get(),
        lod_max_clamp: desc.lod_max.get(),
        compare: desc.compare.map(wgpu_compare_function),
        anisotropy_clamp: desc.anisotropy as u16,
        border_color: None,
    }
}

fn wgpu_address_mode(mode: AddressMode) -> wgpu::AddressMode {
    match mode {
        AddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        AddressMode::Repeat => wgpu::AddressMode::Repeat,
        AddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
    }
}

fn wgpu_filter_mode(mode: FilterMode) -> wgpu::FilterMode {
    match mode {
        FilterMode::Nearest => wgpu::FilterMode::Nearest,
        FilterMode::Linear => wgpu::FilterMode::Linear,
    }
}

fn wgpu_compare_function(compare: CompareFunc) -> wgpu::CompareFunction {
    match compare {
        CompareFunc::Less => wgpu::CompareFunction::Less,
        CompareFunc::LessEqual => wgpu::CompareFunction::LessEqual,
        CompareFunc::Greater => wgpu::CompareFunction::Greater,
        CompareFunc::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
    }
}

fn frame_stats_from_wgpu_metrics(
    frame_index: u64,
    rhi_executed_pass_labels: Vec<String>,
    queue_stats: RenderQueueStats,
    gpu_stats: MeshRenderStats,
    backend_pipeline_objects: usize,
    backend_pipeline_layouts: usize,
    reclaim_policy: ResourceReclaimPolicy,
    gpu_profiling_enabled: bool,
) -> FrameStats {
    let gpu_time_ns = if gpu_profiling_enabled {
        gpu_stats.gpu_time_ns
    } else {
        None
    };
    let timestamp_writes = if gpu_profiling_enabled {
        gpu_stats.timestamp_writes
    } else {
        0
    };
    let pass_count = (rhi_executed_pass_labels.len() as u32)
        .saturating_add(gpu_stats.native_pass_labels_dropped as u32);
    let backend_directional_shadow_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Directional Shadow Pass");
    let backend_spot_shadow_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Spot Shadow Pass");
    let backend_point_shadow_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Point Shadow Pass");
    let backend_depth_prepass_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Depth Prepass");
    let backend_gbuffer_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo GBuffer Pass");
    let backend_deferred_lighting_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Deferred Lighting Pass");
    let backend_forward_opaque_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Forward Opaque Pass");
    let backend_transparent_passes =
        count_native_pass_instances(&rhi_executed_pass_labels, "Neo Transparent Pass");
    let backend_post_process_passes = rhi_executed_pass_labels
        .iter()
        .filter(|label| is_backend_post_process_pass_label(label))
        .count() as u32;
    let backend_native_pass_draws =
        backend_native_pass_draw_stats(&rhi_executed_pass_labels, &gpu_stats);
    FrameStats {
        frame_index,
        gpu_time_ms: gpu_time_ns.map(|time_ns| time_ns as f32 / 1_000_000.0),
        gpu_profiler_enabled: gpu_profiling_enabled,
        draw_calls: gpu_stats.draw_call_count as u32,
        backend_mesh_pass_draw_calls: gpu_stats.mesh_pass_draw_call_count as u32,
        backend_skybox_draw_calls: gpu_stats.skybox_draw_call_count as u32,
        backend_gbuffer_draw_calls: gpu_stats.gbuffer_draw_call_count as u32,
        backend_deferred_lighting_draw_calls: gpu_stats.deferred_lighting_draw_call_count as u32,
        backend_depth_prepass_draw_calls: gpu_stats.depth_prepass_draw_call_count as u32,
        backend_shadow_draw_calls: gpu_stats.shadow_draw_call_count as u32,
        backend_directional_shadow_draw_calls: gpu_stats.directional_shadow_draw_call_count as u32,
        backend_spot_shadow_draw_calls: gpu_stats.spot_shadow_draw_call_count as u32,
        backend_point_shadow_draw_calls: gpu_stats.point_shadow_draw_call_count as u32,
        backend_opaque_draw_calls: gpu_stats.opaque_draw_call_count as u32,
        backend_transparent_draw_calls: gpu_stats.transparent_draw_call_count as u32,
        backend_directional_shadow_passes,
        backend_spot_shadow_passes,
        backend_point_shadow_passes,
        backend_gbuffer_passes,
        backend_deferred_lighting_passes,
        backend_depth_prepass_passes,
        backend_forward_opaque_passes,
        backend_transparent_passes,
        backend_post_process_passes,
        backend_post_process_draw_calls: gpu_stats.post_process_draw_call_count as u32,
        backend_native_pass_label_capacity: MeshRenderStats::native_pass_label_capacity() as u32,
        backend_native_pass_labels_dropped: gpu_stats.native_pass_labels_dropped as u32,
        backend_post_pass_draw_calls: 0,
        backend_native_pass_draws,
        visible_objects: queue_stats.item_count as u32,
        culled_objects: queue_stats.culled_item_count as u32,
        memory: MemoryStats {
            resident_bytes: gpu_stats.instance_buffer_bytes() as u64,
            resident_resources: 0,
            evicted_resources: 0,
            streamable_resources: 0,
            resident_streamable_resources: 0,
            evicted_streamable_resources: 0,
            streamable_texture_mips: 0,
            resident_streamable_texture_mips: 0,
            evicted_streamable_texture_mips: 0,
            streamable_mesh_bytes: 0,
            resident_streamable_mesh_bytes: 0,
            evicted_streamable_mesh_bytes: 0,
            reclaim_policy,
            background_retirement_active: false,
            delayed_destroy_count: 0,
            delayed_destroy_bytes: 0,
            reclaimed_this_frame: 0,
            reclaimed_bytes_this_frame: 0,
            backend_retirement: Default::default(),
        },
        graph: RenderGraphStats {
            pass_count,
            rhi_executed_passes: pass_count,
            transient_textures: 0,
            transient_buffers: 1,
            aliased_memory_bytes: 0,
            barriers: 0,
            fullscreen_draws: gpu_stats
                .post_process_draw_call_count
                .saturating_add(gpu_stats.deferred_lighting_draw_call_count)
                as u32,
            timestamp_queries: if timestamp_writes > 0 { 1 } else { 0 },
            timestamp_writes: timestamp_writes as u32,
            gpu_time_ns,
            rhi_executed_pass_labels,
            ..RenderGraphStats::default()
        },
        pipeline_cache: PipelineCacheStats {
            backend_objects: backend_pipeline_objects,
            shader_interface_layouts: backend_pipeline_layouts,
            ..PipelineCacheStats::default()
        },
        ..FrameStats::default()
    }
}

fn count_native_pass_instances(rhi_executed_pass_labels: &[String], pass_label: &str) -> u32 {
    rhi_executed_pass_labels
        .iter()
        .filter(|label| label.as_str() == pass_label)
        .count() as u32
}

fn native_pass_labels_from_wgpu_metrics(
    scene: &RenderScene,
    visible_items: usize,
    gpu_stats: &MeshRenderStats,
) -> Vec<String> {
    let actual_labels = gpu_stats.native_pass_label_strings();
    if actual_labels.is_empty() {
        default_wgpu_pass_labels(scene, visible_items)
    } else {
        actual_labels
    }
}

fn backend_native_pass_draw_stats(
    rhi_executed_pass_labels: &[String],
    gpu_stats: &MeshRenderStats,
) -> Vec<BackendNativePassDrawStats> {
    let mut stats = Vec::new();
    let mut push_if_present = |pass_label: &str, draw_calls: usize| {
        let pass_instances = rhi_executed_pass_labels
            .iter()
            .filter(|label| label.as_str() == pass_label)
            .count();
        if pass_instances > 0 {
            stats.push(BackendNativePassDrawStats {
                pass_label: pass_label.to_owned(),
                pass_instances: pass_instances as u32,
                draw_calls: draw_calls as u32,
            });
        }
    };
    push_if_present(
        "Neo Directional Shadow Pass",
        gpu_stats.directional_shadow_draw_call_count,
    );
    push_if_present(
        "Neo Spot Shadow Pass",
        gpu_stats.spot_shadow_draw_call_count,
    );
    push_if_present(
        "Neo Point Shadow Pass",
        gpu_stats.point_shadow_draw_call_count,
    );
    push_if_present("Neo Depth Prepass", gpu_stats.depth_prepass_draw_call_count);
    push_if_present("Neo GBuffer Pass", gpu_stats.gbuffer_draw_call_count);
    push_if_present(
        "Neo Deferred Lighting Pass",
        gpu_stats.deferred_lighting_draw_call_count,
    );
    push_if_present(
        "Neo Forward Opaque Pass",
        gpu_stats
            .opaque_draw_call_count
            .saturating_add(gpu_stats.skybox_draw_call_count),
    );
    push_if_present(
        "Neo Transparent Pass",
        gpu_stats.transparent_draw_call_count,
    );
    let mut post_process_labels = Vec::new();
    for label in rhi_executed_pass_labels {
        if is_backend_post_process_pass_label(label)
            && !post_process_labels
                .iter()
                .any(|existing| *existing == label.as_str())
        {
            post_process_labels.push(label.as_str());
        }
    }
    if post_process_labels.is_empty() {
        push_if_present(
            "Neo Post Process Pass",
            gpu_stats.post_process_draw_call_count,
        );
    } else {
        for label in post_process_labels {
            push_if_present(label, gpu_stats.post_process_draw_call_count);
        }
    }
    stats
}

fn is_backend_post_process_pass_label(label: &str) -> bool {
    label == "Neo Post Process Pass"
        || (label.starts_with("Neo ")
            && label.ends_with(" Post Process Pass")
            && label.contains("Tonemap"))
}

fn record_native_post_pass_draws(stats: &mut FrameStats, draw_call_count: u32) {
    if draw_call_count == 0 {
        return;
    }

    stats.draw_calls = stats.draw_calls.saturating_add(draw_call_count);
    stats.backend_post_pass_draw_calls = stats
        .backend_post_pass_draw_calls
        .saturating_add(draw_call_count);
    let pass_instances = stats
        .graph
        .rhi_executed_pass_labels
        .iter()
        .filter(|label| is_backend_post_process_pass_label(label))
        .count()
        .max(1) as u32;
    let post_process_pass_label = stats
        .graph
        .rhi_executed_pass_labels
        .iter()
        .find(|label| is_backend_post_process_pass_label(label))
        .map(String::as_str)
        .unwrap_or("Neo Post Process Pass");
    if let Some(pass_stats) = stats
        .backend_native_pass_draws
        .iter_mut()
        .find(|pass| pass.pass_label == post_process_pass_label)
    {
        pass_stats.pass_instances = pass_stats.pass_instances.max(pass_instances);
        pass_stats.draw_calls = pass_stats.draw_calls.saturating_add(draw_call_count);
    } else {
        stats
            .backend_native_pass_draws
            .push(BackendNativePassDrawStats {
                pass_label: post_process_pass_label.to_owned(),
                pass_instances,
                draw_calls: draw_call_count,
            });
    }
}

fn merge_wgpu_pipeline_cache_stats(
    stats: &mut PipelineCacheStats,
    native_stats: PipelineCacheStats,
) {
    stats.total = stats.total.saturating_add(native_stats.total);
    stats.ready = stats.ready.saturating_add(native_stats.ready);
    stats.compiling = stats.compiling.saturating_add(native_stats.compiling);
    stats.failed = stats.failed.saturating_add(native_stats.failed);
    stats.backend_objects = stats
        .backend_objects
        .saturating_add(native_stats.backend_objects);
    stats.shader_interface_layouts = stats
        .shader_interface_layouts
        .saturating_add(native_stats.shader_interface_layouts);
    stats.entries_used_this_frame = stats
        .entries_used_this_frame
        .saturating_add(native_stats.entries_used_this_frame);
    stats.ready_unused_entries = stats
        .ready_unused_entries
        .saturating_add(native_stats.ready_unused_entries);
    stats.ready_entries_without_backend_object = stats
        .ready_entries_without_backend_object
        .saturating_add(native_stats.ready_entries_without_backend_object);
    stats.used_entries_without_backend_object = stats
        .used_entries_without_backend_object
        .saturating_add(native_stats.used_entries_without_backend_object);
    stats.cache_hits_this_frame = stats
        .cache_hits_this_frame
        .saturating_add(native_stats.cache_hits_this_frame);
    stats.cache_misses_this_frame = stats
        .cache_misses_this_frame
        .saturating_add(native_stats.cache_misses_this_frame);
    stats.invalidated_this_frame = stats
        .invalidated_this_frame
        .saturating_add(native_stats.invalidated_this_frame);
}

fn wgpu_options(backend: BackendPreference) -> WgpuGraphicsOptions {
    WgpuGraphicsOptions {
        power_preference: match backend {
            BackendPreference::Headless => wgpu::PowerPreference::LowPower,
            _ => wgpu::PowerPreference::HighPerformance,
        },
        force_fallback_adapter: matches!(backend, BackendPreference::Headless),
    }
}

fn wgpu_renderer_caps(config: &RendererConfig, graphics: &WgpuGraphics) -> RendererCaps {
    let mut caps = RendererCaps::for_backend(config, "wgpu", &graphics.adapter().get_info().name);
    let limits = graphics.device().limits();
    caps.limits = renderer_limits_from_wgpu(&limits);
    caps.formats = wgpu_format_caps(graphics.adapter());
    let device_features = graphics.device().features();
    if device_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        caps.features = caps.features | RendererFeatures::TIMESTAMP_QUERY;
    } else {
        caps.features = RendererFeatures(caps.features.0 & !RendererFeatures::TIMESTAMP_QUERY.0);
    }
    if device_features.contains(wgpu::Features::VERTEX_ATTRIBUTE_64BIT) {
        caps.features = caps.features | RendererFeatures::VERTEX_ATTRIBUTE_64BIT;
    } else {
        caps.features =
            RendererFeatures(caps.features.0 & !RendererFeatures::VERTEX_ATTRIBUTE_64BIT.0);
    }
    caps
}

fn wgpu_format_caps(adapter: &wgpu::Adapter) -> FormatCaps {
    let candidates = FormatCaps::default();
    FormatCaps {
        color: candidates
            .color
            .into_iter()
            .filter(|format| {
                format_supports_render_attachment(
                    adapter.get_texture_format_features(wgpu_surface_format(*format)),
                )
            })
            .collect(),
        depth: candidates
            .depth
            .into_iter()
            .filter(|format| {
                format_supports_render_attachment(
                    adapter.get_texture_format_features(wgpu_depth_format(*format)),
                )
            })
            .collect(),
    }
}

fn format_supports_render_attachment(features: wgpu::TextureFormatFeatures) -> bool {
    features
        .allowed_usages
        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
}

fn renderer_limits_from_wgpu(limits: &wgpu::Limits) -> RendererLimits {
    RendererLimits {
        max_texture_dimension_2d: limits.max_texture_dimension_2d,
        max_texture_array_layers: limits.max_texture_array_layers,
        max_bind_groups: limits.max_bind_groups,
        max_vertex_buffers: limits.max_vertex_buffers,
    }
}

fn present_mode_for_vsync(mode: VSyncMode) -> PresentMode {
    match mode {
        VSyncMode::Off => PresentMode::Immediate,
        VSyncMode::On => PresentMode::Fifo,
        VSyncMode::Adaptive => PresentMode::AutoVsync,
    }
}

fn surface_options(config: &RendererConfig) -> WgpuSurfaceOptions {
    WgpuSurfaceOptions {
        preferred_format: config.surface_format.map(wgpu_surface_format),
        depth_format: wgpu_depth_format(config.depth_format),
    }
}

fn wgpu_surface_format(format: TextureFormat) -> wgpu::TextureFormat {
    match format {
        TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
    }
}

pub(crate) fn texture_format_from_wgpu_surface(
    format: wgpu::TextureFormat,
) -> Option<TextureFormat> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm => Some(TextureFormat::Rgba8Unorm),
        wgpu::TextureFormat::Rgba8UnormSrgb => Some(TextureFormat::Rgba8UnormSrgb),
        wgpu::TextureFormat::Bgra8UnormSrgb => Some(TextureFormat::Bgra8UnormSrgb),
        wgpu::TextureFormat::Rgba16Float => Some(TextureFormat::Rgba16Float),
        wgpu::TextureFormat::Rgba32Float => Some(TextureFormat::Rgba32Float),
        _ => None,
    }
}

fn wgpu_depth_format(format: DepthFormat) -> wgpu::TextureFormat {
    match format {
        DepthFormat::D16Unorm => wgpu::TextureFormat::Depth16Unorm,
        DepthFormat::D24Plus => wgpu::TextureFormat::Depth24Plus,
        DepthFormat::D24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
        DepthFormat::D32Float => wgpu::TextureFormat::Depth32Float,
    }
}

fn depth_format_from_wgpu(format: wgpu::TextureFormat) -> Option<DepthFormat> {
    match format {
        wgpu::TextureFormat::Depth16Unorm => Some(DepthFormat::D16Unorm),
        wgpu::TextureFormat::Depth24Plus => Some(DepthFormat::D24Plus),
        wgpu::TextureFormat::Depth24PlusStencil8 => Some(DepthFormat::D24PlusStencil8),
        wgpu::TextureFormat::Depth32Float => Some(DepthFormat::D32Float),
        _ => None,
    }
}

fn aspect_ratio(size: SurfaceSize) -> f32 {
    if size.height == 0 {
        1.0
    } else {
        size.width as f32 / size.height as f32
    }
}

fn default_wgpu_pass_labels(scene: &RenderScene, visible_items: usize) -> Vec<String> {
    let lighting = scene.lighting();
    let mut labels = Vec::new();
    if visible_items > 0 {
        for _ in 0..directional_shadow_pass_count(scene) {
            labels.push("Neo Directional Shadow Pass".to_owned());
        }
        labels.extend(
            lighting
                .spot_lights()
                .iter()
                .filter(|light| {
                    light.shadow.enabled && light.shadow.strength > 0.0 && light.intensity > 0.0
                })
                .map(|_| "Neo Spot Shadow Pass".to_owned()),
        );
        for _ in lighting.point_lights().iter().filter(|light| {
            light.shadow.enabled && light.shadow.strength > 0.0 && light.intensity > 0.0
        }) {
            labels.extend((0..6).map(|_| "Neo Point Shadow Pass".to_owned()));
        }
        labels.push("Neo Depth Prepass".to_owned());
        labels.push("Neo GBuffer Pass".to_owned());
        labels.push("Neo Deferred Lighting Pass".to_owned());
    }
    labels.push("Neo Forward Opaque Pass".to_owned());
    labels.push("Neo Transparent Pass".to_owned());
    if visible_items > 0 {
        labels.push("Neo Tonemap Post Process Pass".to_owned());
    } else {
        labels.push("Neo Post Process Pass".to_owned());
    }
    labels
}

fn directional_shadow_pass_count(scene: &RenderScene) -> usize {
    let lighting = scene.lighting();
    let shadow = lighting.directional_shadow;
    if !shadow.enabled || shadow.strength <= 0.0 {
        return 0;
    }
    let requested = shadow
        .cascade_count
        .clamp(1, engine_render::MAX_DIRECTIONAL_SHADOW_CASCADES);
    if requested == 1 {
        return 1;
    }
    let engine_render::Camera::Perspective(camera) = scene.camera() else {
        return 1;
    };
    let near = camera.near.max(0.0001);
    let far = camera
        .far
        .max(near + 0.0001)
        .min(shadow.cascade_max_distance.max(near + 0.0001));
    if far <= near + 0.0001 {
        return 0;
    }
    requested
}

fn map_backend_error(error: GraphicsError) -> RendererError {
    match error {
        GraphicsError::SurfaceOutOfMemory => {
            RendererError::OutOfMemory("surface is out of memory".to_owned())
        }
        GraphicsError::Backend(message)
            if message.contains("device was lost") || message.contains("surface was lost") =>
        {
            RendererError::DeviceLost { reason: message }
        }
        error => RendererError::Backend(error.to_string()),
    }
}

fn record_backend_error(device_status: &mut DeviceStatus, error: GraphicsError) -> RendererError {
    let error = map_backend_error(error);
    if matches!(error, RendererError::DeviceLost { .. }) {
        *device_status = DeviceStatus::Lost;
    }
    error
}

fn tombstone_submission_index_coverage(
    tombstones: usize,
    with_submission_index: usize,
    without_submission_index: usize,
) -> WgpuTombstoneSubmissionIndexCoverage {
    if tombstones == 0 {
        WgpuTombstoneSubmissionIndexCoverage::NotApplicable
    } else if with_submission_index == 0 {
        WgpuTombstoneSubmissionIndexCoverage::None
    } else if without_submission_index == 0 {
        WgpuTombstoneSubmissionIndexCoverage::All
    } else {
        WgpuTombstoneSubmissionIndexCoverage::Partial
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        MaterialTemplateHandle, PushConstantRange, RenderPhaseKind, ShaderHandle,
        ShaderResourceBinding,
    };

    use super::*;

    #[test]
    fn wgpu_shader_interface_layout_plan_maps_reflected_groups_and_bindings() {
        let interface = ShaderInterfaceDesc {
            resources: vec![
                ShaderResourceBinding {
                    name: "material".to_owned(),
                    group: 1,
                    binding: 5,
                    binding_class: BindingClass::Texture,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture(TextureDimension::D2),
                },
                ShaderResourceBinding {
                    name: "camera".to_owned(),
                    group: 0,
                    binding: 0,
                    binding_class: BindingClass::Uniform,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer,
                },
                ShaderResourceBinding {
                    name: "material_sampler".to_owned(),
                    group: 1,
                    binding: 1,
                    binding_class: BindingClass::Sampler,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler,
                },
                ShaderResourceBinding {
                    name: "particles".to_owned(),
                    group: 2,
                    binding: 0,
                    binding_class: BindingClass::Storage,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer,
                },
                ShaderResourceBinding {
                    name: "storage_image".to_owned(),
                    group: 2,
                    binding: 2,
                    binding_class: BindingClass::Storage,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::StorageTexture {
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8Unorm,
                        access: StorageTextureAccess::WriteOnly,
                    },
                },
            ],
            push_constants: vec![PushConstantRange {
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                range: 0..16,
            }],
            vertex_inputs: Vec::new(),
        };

        let plan = wgpu_shader_interface_layout_plan(&interface).unwrap();

        assert_eq!(plan.bind_groups.len(), 3);
        assert_eq!(plan.bind_groups[0].group, 0);
        assert_eq!(plan.bind_groups[0].entries[0].binding, 0);
        assert_eq!(
            plan.bind_groups[0].entries[0].visibility,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT
        );
        assert!(matches!(
            &plan.bind_groups[0].entries[0].ty,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                ..
            }
        ));

        assert_eq!(plan.bind_groups[1].group, 1);
        assert_eq!(plan.bind_groups[1].entries[0].binding, 1);
        assert!(matches!(
            &plan.bind_groups[1].entries[0].ty,
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
        ));
        assert_eq!(plan.bind_groups[1].entries[1].binding, 5);
        assert!(matches!(
            &plan.bind_groups[1].entries[1].ty,
            wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            }
        ));

        assert_eq!(plan.bind_groups[2].group, 2);
        assert_eq!(plan.bind_groups[2].entries[0].binding, 0);
        assert!(matches!(
            &plan.bind_groups[2].entries[0].ty,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                ..
            }
        ));
        assert_eq!(plan.bind_groups[2].entries[1].binding, 2);
        assert!(matches!(
            &plan.bind_groups[2].entries[1].ty,
            wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: wgpu::TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D2,
            }
        ));
        assert_eq!(plan.push_constants.len(), 1);
        assert_eq!(
            plan.push_constants[0].stages,
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT
        );
        assert_eq!(plan.push_constants[0].range, 0..16);
    }

    #[test]
    fn wgpu_shader_interface_layout_plan_rejects_unmapped_backend_bindings() {
        let unsupported_storage_texture = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "storage_image".to_owned(),
                group: 0,
                binding: 0,
                binding_class: BindingClass::Storage,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::StorageTexture {
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    access: StorageTextureAccess::WriteOnly,
                },
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        assert!(matches!(
            wgpu_shader_interface_layout_plan(&unsupported_storage_texture),
            Err(RendererError::Validation(_))
        ));

        let unsupported_stage = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "mesh_only".to_owned(),
                group: 0,
                binding: 0,
                binding_class: BindingClass::Uniform,
                visibility: ShaderStages::MESH,
                ty: BindingType::Buffer,
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        assert!(matches!(
            wgpu_shader_interface_layout_plan(&unsupported_stage),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn wgpu_material_bind_group_resource_plan_maps_parameters_to_reflected_slots() {
        let texture = TextureHandle::from_raw(std::num::NonZeroU64::new(101).unwrap());
        let storage_texture = TextureHandle::from_raw(std::num::NonZeroU64::new(102).unwrap());
        let sampler = SamplerHandle::from_raw(std::num::NonZeroU64::new(201).unwrap());
        let storage_binding_type = BindingType::StorageTexture {
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            access: StorageTextureAccess::WriteOnly,
        };
        let interface = ShaderInterfaceDesc {
            resources: vec![
                ShaderResourceBinding {
                    name: "base_color".to_owned(),
                    group: 1,
                    binding: 5,
                    binding_class: BindingClass::Texture,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture(TextureDimension::D2),
                },
                ShaderResourceBinding {
                    name: "base_sampler".to_owned(),
                    group: 1,
                    binding: 1,
                    binding_class: BindingClass::Sampler,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler,
                },
                ShaderResourceBinding {
                    name: "material_uniforms".to_owned(),
                    group: 1,
                    binding: 0,
                    binding_class: BindingClass::Uniform,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer,
                },
                ShaderResourceBinding {
                    name: "storage_image".to_owned(),
                    group: 2,
                    binding: 0,
                    binding_class: BindingClass::Storage,
                    visibility: ShaderStages::FRAGMENT,
                    ty: storage_binding_type.clone(),
                },
            ],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        let parameters = vec![
            MaterialParameter {
                name: "base_color".to_owned(),
                value: MaterialParameterValue::Texture(texture),
            },
            MaterialParameter {
                name: "storage_image".to_owned(),
                value: MaterialParameterValue::Texture(storage_texture),
            },
            MaterialParameter {
                name: "material_uniforms".to_owned(),
                value: MaterialParameterValue::Bytes(vec![1, 2, 3, 4]),
            },
            MaterialParameter {
                name: "base_sampler".to_owned(),
                value: MaterialParameterValue::Sampler(sampler),
            },
        ];

        let plan = wgpu_material_bind_group_resource_plan(&interface, &parameters).unwrap();

        assert_eq!(plan.groups.len(), 2);
        assert_eq!(plan.groups[0].group, 1);
        assert_eq!(plan.groups[0].entries.len(), 3);
        assert_eq!(plan.groups[0].entries[0].name, "material_uniforms");
        assert_eq!(plan.groups[0].entries[0].binding, 0);
        assert_eq!(plan.groups[0].entries[0].binding_type, BindingType::Buffer);
        assert_eq!(
            plan.groups[0].entries[0].resource,
            WgpuMaterialBindingResource::BufferBytes {
                bytes: vec![1, 2, 3, 4]
            }
        );
        assert_eq!(plan.groups[0].entries[1].name, "base_sampler");
        assert_eq!(plan.groups[0].entries[1].binding, 1);
        assert_eq!(
            plan.groups[0].entries[1].resource,
            WgpuMaterialBindingResource::Sampler(sampler)
        );
        assert_eq!(plan.groups[0].entries[2].name, "base_color");
        assert_eq!(plan.groups[0].entries[2].binding, 5);
        assert_eq!(
            plan.groups[0].entries[2].resource,
            WgpuMaterialBindingResource::Texture(texture)
        );

        assert_eq!(plan.groups[1].group, 2);
        assert_eq!(plan.groups[1].entries[0].binding, 0);
        assert_eq!(plan.groups[1].entries[0].binding_type, storage_binding_type);
        assert_eq!(
            plan.groups[1].entries[0].resource,
            WgpuMaterialBindingResource::Texture(storage_texture)
        );
    }

    #[test]
    fn wgpu_material_bind_group_resource_plan_rejects_unbound_or_mismatched_parameters() {
        let texture = TextureHandle::from_raw(std::num::NonZeroU64::new(101).unwrap());
        let interface = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "base_sampler".to_owned(),
                group: 0,
                binding: 0,
                binding_class: BindingClass::Sampler,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler,
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };

        assert!(matches!(
            wgpu_material_bind_group_resource_plan(
                &interface,
                &[MaterialParameter {
                    name: "missing".to_owned(),
                    value: MaterialParameterValue::Texture(texture),
                }]
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            wgpu_material_bind_group_resource_plan(
                &interface,
                &[MaterialParameter {
                    name: "base_sampler".to_owned(),
                    value: MaterialParameterValue::Texture(texture),
                }]
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            wgpu_material_bind_group_resource_plan(
                &interface,
                &[
                    MaterialParameter {
                        name: "base_sampler".to_owned(),
                        value: MaterialParameterValue::Sampler(SamplerHandle::from_raw(
                            std::num::NonZeroU64::new(201).unwrap()
                        )),
                    },
                    MaterialParameter {
                        name: "base_sampler".to_owned(),
                        value: MaterialParameterValue::Sampler(SamplerHandle::from_raw(
                            std::num::NonZeroU64::new(202).unwrap()
                        )),
                    },
                ]
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
    }

    #[test]
    fn wgpu_material_external_resource_registry_reports_missing_handles() {
        let registry = WgpuMaterialExternalResourceRegistry::default();
        let texture = TextureHandle::from_raw(std::num::NonZeroU64::new(101).unwrap());
        let sampler = SamplerHandle::from_raw(std::num::NonZeroU64::new(201).unwrap());

        assert!(matches!(
            registry.resolve(&WgpuMaterialBindGroupResourceEntryPlan {
                name: "base_color".to_owned(),
                binding: 0,
                binding_class: BindingClass::Texture,
                binding_type: BindingType::Texture(TextureDimension::D2),
                resource: WgpuMaterialBindingResource::Texture(texture),
            }),
            Err(RendererError::InvalidHandle {
                kind: crate::ResourceKind::Texture,
                ..
            })
        ));
        assert!(matches!(
            registry.resolve(&WgpuMaterialBindGroupResourceEntryPlan {
                name: "base_sampler".to_owned(),
                binding: 1,
                binding_class: BindingClass::Sampler,
                binding_type: BindingType::Sampler,
                resource: WgpuMaterialBindingResource::Sampler(sampler),
            }),
            Err(RendererError::InvalidHandle {
                kind: crate::ResourceKind::Sampler,
                ..
            })
        ));
    }

    #[test]
    fn wgpu_shader_variant_module_cache_compiles_reuses_and_invalidates() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu shader variant module cache test: {error}");
                return;
            }
        };
        let shader = ShaderHandle::from_raw(std::num::NonZeroU64::new(301).unwrap());
        let source = ShaderSource::Wgsl(
            r#"
            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(0.0, 1.0, 0.0, 1.0);
            }
            "#,
        );
        let fog = vec!["fog".to_owned()];
        let skinning = vec!["skinning".to_owned()];

        assert!(runtime
            .compile_and_cache_shader_variant_module(shader, &fog, &source, Some("variant"))
            .unwrap());
        assert_eq!(runtime.shader_variant_module_count(), 1);
        assert!(runtime
            .compile_and_cache_shader_variant_module(shader, &fog, &source, Some("variant"))
            .unwrap());
        assert_eq!(runtime.shader_variant_module_count(), 1);
        assert!(runtime
            .compile_and_cache_shader_variant_module(shader, &skinning, &source, Some("variant"))
            .unwrap());
        assert_eq!(runtime.shader_variant_module_count(), 2);

        assert_eq!(
            runtime.invalidate_shader_variant_modules_for_shader(shader),
            2
        );
        assert_eq!(runtime.shader_variant_module_count(), 0);
        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 1);
        assert_eq!(retirement.fence_objects, 1);
        assert_eq!(retirement.shader_variant_modules, 2);
        assert_eq!(retirement.shader_modules, 0);
        assert_eq!(retirement.native_pipeline_entries, 0);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 1);
        assert_eq!(retired.retired_fence_objects_this_poll, 1);
        assert_eq!(retired.retired_shader_variant_modules_this_poll, 2);
        assert_eq!(retired.retired_shader_modules_this_poll, 0);

        let idle = runtime.poll_backend_resource_retirements();
        assert_eq!(idle.retired_shader_variant_modules_this_poll, 0);
    }

    #[test]
    fn wgpu_material_external_resources_unregister_into_backend_tombstones() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material external resource tombstone test: {error}");
                return;
            }
        };
        let texture_handle = TextureHandle::from_raw(std::num::NonZeroU64::new(701).unwrap());
        let sampler_handle = SamplerHandle::from_raw(std::num::NonZeroU64::new(702).unwrap());
        let texture = runtime
            .graphics()
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Neo material external resource tombstone texture"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
        runtime.register_material_texture_binding(
            texture_handle,
            texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        runtime.register_material_sampler_binding(
            sampler_handle,
            runtime
                .graphics()
                .device()
                .create_sampler(&wgpu::SamplerDescriptor::default()),
        );

        assert!(runtime.unregister_material_texture_binding(texture_handle));
        assert!(runtime.unregister_material_sampler_binding(sampler_handle));
        assert!(!runtime.unregister_material_texture_binding(texture_handle));
        assert!(!runtime.unregister_material_sampler_binding(sampler_handle));

        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 2);
        assert_eq!(retirement.fence_objects, 2);
        assert_eq!(retirement.material_textures, 1);
        assert_eq!(retirement.material_samplers, 1);
        assert_eq!(retirement.shader_variant_modules, 0);
        assert_eq!(retirement.native_pipeline_entries, 0);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 2);
        assert_eq!(retired.retired_fence_objects_this_poll, 2);
        assert_eq!(retired.retired_material_textures_this_poll, 1);
        assert_eq!(retired.retired_material_samplers_this_poll, 1);
        assert_eq!(retired.retired_shader_variant_modules_this_poll, 0);

        let idle = runtime.poll_backend_resource_retirements();
        assert_eq!(idle.retired_material_textures_this_poll, 0);
        assert_eq!(idle.retired_material_samplers_this_poll, 0);
    }

    #[test]
    fn wgpu_material_external_resources_replace_into_backend_tombstones() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material external replacement tombstone test: {error}");
                return;
            }
        };
        let texture_handle = TextureHandle::from_raw(std::num::NonZeroU64::new(703).unwrap());
        let sampler_handle = SamplerHandle::from_raw(std::num::NonZeroU64::new(704).unwrap());
        let create_view = |runtime: &WgpuRendererRuntime, label| {
            runtime
                .graphics()
                .device()
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };

        runtime.register_material_texture_binding(
            texture_handle,
            create_view(&runtime, "Neo material external replacement old texture"),
        );
        runtime.register_material_sampler_binding(
            sampler_handle,
            runtime
                .graphics()
                .device()
                .create_sampler(&wgpu::SamplerDescriptor::default()),
        );
        assert_eq!(runtime.backend_resource_retirement_stats().tombstones, 0);

        runtime.register_material_texture_binding(
            texture_handle,
            create_view(&runtime, "Neo material external replacement new texture"),
        );
        runtime.register_material_sampler_binding(
            sampler_handle,
            runtime
                .graphics()
                .device()
                .create_sampler(&wgpu::SamplerDescriptor::default()),
        );

        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 2);
        assert_eq!(retirement.fence_objects, 2);
        assert_eq!(retirement.material_textures, 1);
        assert_eq!(retirement.material_samplers, 1);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 2);
        assert_eq!(retired.retired_material_textures_this_poll, 1);
        assert_eq!(retired.retired_material_samplers_this_poll, 1);
        assert_eq!(retired.retired_fence_objects_this_poll, 2);
    }

    #[test]
    fn wgpu_material_sampler_create_and_register_replaces_into_backend_tombstone() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material sampler replacement tombstone test: {error}");
                return;
            }
        };
        let sampler_handle = SamplerHandle::from_raw(std::num::NonZeroU64::new(705).unwrap());
        let desc = SamplerDesc::default();

        runtime.create_and_register_material_sampler_binding(sampler_handle, &desc);
        assert_eq!(runtime.backend_resource_retirement_stats().tombstones, 0);

        runtime.create_and_register_material_sampler_binding(sampler_handle, &desc);

        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 1);
        assert_eq!(retirement.fence_objects, 1);
        assert_eq!(retirement.material_samplers, 1);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 1);
        assert_eq!(retired.retired_material_samplers_this_poll, 1);
        assert_eq!(retired.retired_fence_objects_this_poll, 1);
    }

    #[test]
    fn wgpu_material_texture_create_and_register_replaces_into_backend_tombstone() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material texture replacement tombstone test: {error}");
                return;
            }
        };
        let texture_handle = TextureHandle::from_raw(std::num::NonZeroU64::new(706).unwrap());
        let desc = WgpuMaterialTextureUploadDesc {
            label: Some("Neo material texture replacement old".to_owned()),
            dimension: TextureDimension::D2,
            width: 4,
            height: 4,
            depth_or_layers: 1,
            mip_level_count: 1,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            sampled_binding: true,
            storage_binding: false,
            generate_mips_from_base: false,
            uploads: vec![WgpuMaterialTextureUpload {
                mip_level: 0,
                origin: [0, 0, 0],
                extent: [4, 4, 1],
                bytes_per_row: 16,
                rows_per_image: 4,
                bytes: vec![255; 64],
            }],
        };

        runtime
            .create_and_register_material_texture_binding(texture_handle, &desc)
            .unwrap();
        assert_eq!(runtime.backend_resource_retirement_stats().tombstones, 0);

        let replacement = WgpuMaterialTextureUploadDesc {
            label: Some("Neo material texture replacement new".to_owned()),
            ..desc
        };
        runtime
            .create_and_register_material_texture_binding(texture_handle, &replacement)
            .unwrap();

        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 1);
        assert_eq!(retirement.fence_objects, 1);
        assert_eq!(retirement.material_textures, 1);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 1);
        assert_eq!(retired.retired_material_textures_this_poll, 1);
        assert_eq!(retired.retired_fence_objects_this_poll, 1);
    }

    #[test]
    fn wgpu_post_pass_buffers_enter_backend_tombstones_until_poll_retirement() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu post-pass buffer tombstone test: {error}");
                return;
            }
        };

        let vertex_buffer = create_wgpu_queued_native_buffer(
            runtime.graphics().device(),
            Some("post-pass tombstone vertex"),
            &[0, 0, 0, 0],
            wgpu::BufferUsages::VERTEX,
        )
        .unwrap();
        let index_buffer = create_wgpu_queued_native_buffer(
            runtime.graphics().device(),
            Some("post-pass tombstone index"),
            &[0, 0, 0, 0],
            wgpu::BufferUsages::INDEX,
        )
        .unwrap();

        runtime.queue_post_pass_buffer_tombstone(vec![WgpuBackendPostPassBufferTombstone {
            vertex_buffers: vec![WgpuNativePipelinePostPassVertexBuffer {
                slot: 0,
                buffer: std::sync::Arc::new(vertex_buffer),
            }],
            index_buffer: Some(WgpuNativePipelinePostPassIndexBuffer {
                format: wgpu::IndexFormat::Uint16,
                buffer: std::sync::Arc::new(index_buffer),
            }),
        }]);

        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 1);
        assert_eq!(retirement.fence_objects, 1);
        assert_eq!(retirement.post_pass_vertex_buffers, 1);
        assert_eq!(retirement.post_pass_index_buffers, 1);
        assert_eq!(retirement.retired_post_pass_vertex_buffers_this_poll, 0);
        assert_eq!(retirement.retired_post_pass_index_buffers_this_poll, 0);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.post_pass_vertex_buffers, 0);
        assert_eq!(retired.post_pass_index_buffers, 0);
        assert_eq!(retired.retired_post_pass_vertex_buffers_this_poll, 1);
        assert_eq!(retired.retired_post_pass_index_buffers_this_poll, 1);
        assert_eq!(retired.retired_fence_objects_this_poll, 1);
    }

    #[test]
    fn wgpu_backend_tombstone_stats_distinguish_submission_indexed_fences() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu backend fence index tombstone test: {error}");
                return;
            }
        };
        let create_module = |runtime: &WgpuRendererRuntime, label| {
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                })
        };

        let unindexed = create_module(&runtime, "Neo unindexed tombstone module");
        runtime.queue_shader_variant_module_tombstone(vec![unindexed]);
        let live_unindexed = runtime.backend_resource_retirement_stats();
        assert_eq!(live_unindexed.fence_objects, 1);
        assert_eq!(live_unindexed.tombstones_with_submission_index, 0);
        assert_eq!(live_unindexed.tombstones_without_submission_index, 1);
        assert_eq!(
            live_unindexed.tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::None
        );
        assert!(!live_unindexed.all_tombstones_have_submission_index);
        assert!(!live_unindexed.partial_tombstone_submission_index_coverage);
        assert!(live_unindexed.no_tombstones_have_submission_index);
        assert_eq!(live_unindexed.fence_submission_indices, 0);
        assert_eq!(live_unindexed.fence_objects_without_submission_index, 1);

        let retired_unindexed = runtime.poll_backend_resource_retirements();
        assert!(retired_unindexed.last_poll_queue_empty);
        assert!(retired_unindexed.retired_after_queue_empty_poll);
        assert!(!retired_unindexed.last_poll_completed_submission_index_recorded);
        assert!(!retired_unindexed.retired_after_completed_submission_index_poll);
        assert_eq!(
            retired_unindexed.retired_tombstones_with_submission_index_this_poll,
            0
        );
        assert_eq!(
            retired_unindexed.retired_tombstones_without_submission_index_this_poll,
            1
        );
        assert_eq!(
            retired_unindexed.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::None
        );
        assert!(!retired_unindexed.retired_all_tombstones_had_submission_index_this_poll);
        assert!(!retired_unindexed.retired_partial_tombstone_submission_index_coverage_this_poll);
        assert!(retired_unindexed.retired_no_tombstones_had_submission_index_this_poll);
        assert_eq!(retired_unindexed.retired_fence_objects_this_poll, 1);
        assert_eq!(
            retired_unindexed.retired_fence_submission_indices_this_poll,
            0
        );
        assert_eq!(
            retired_unindexed.retired_fence_objects_without_submission_index_this_poll,
            1
        );

        let encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo indexed tombstone fence encoder"),
                });
        runtime.last_submission_index =
            Some(runtime.graphics().queue().submit(Some(encoder.finish())));
        let indexed = create_module(&runtime, "Neo indexed tombstone module");
        runtime.queue_shader_variant_module_tombstone(vec![indexed]);
        let live_indexed = runtime.backend_resource_retirement_stats();
        assert_eq!(live_indexed.fence_objects, 1);
        assert_eq!(live_indexed.tombstones_with_submission_index, 1);
        assert_eq!(live_indexed.tombstones_without_submission_index, 0);
        assert_eq!(
            live_indexed.tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::All
        );
        assert!(live_indexed.all_tombstones_have_submission_index);
        assert!(!live_indexed.partial_tombstone_submission_index_coverage);
        assert!(!live_indexed.no_tombstones_have_submission_index);
        assert_eq!(live_indexed.fence_submission_indices, 1);
        assert_eq!(live_indexed.fence_objects_without_submission_index, 0);

        runtime.wait_for_gpu();
        let retired_indexed = runtime.poll_backend_resource_retirements();
        assert!(retired_indexed.last_poll_queue_empty);
        assert!(retired_indexed.retired_after_queue_empty_poll);
        assert!(retired_indexed.last_poll_completed_submission_index_recorded);
        assert!(retired_indexed.retired_after_completed_submission_index_poll);
        assert_eq!(
            retired_indexed.retired_tombstones_with_submission_index_this_poll,
            1
        );
        assert_eq!(
            retired_indexed.retired_tombstones_without_submission_index_this_poll,
            0
        );
        assert_eq!(
            retired_indexed.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::All
        );
        assert!(retired_indexed.retired_all_tombstones_had_submission_index_this_poll);
        assert!(!retired_indexed.retired_partial_tombstone_submission_index_coverage_this_poll);
        assert!(!retired_indexed.retired_no_tombstones_had_submission_index_this_poll);
        assert_eq!(retired_indexed.retired_fence_objects_this_poll, 1);
        assert_eq!(
            retired_indexed.retired_fence_submission_indices_this_poll,
            1
        );
        assert_eq!(
            retired_indexed.retired_fence_objects_without_submission_index_this_poll,
            0
        );
    }

    #[test]
    fn wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu backend tombstone gate invalidation test: {error}");
                return;
            }
        };
        let idle = runtime.poll_backend_resource_retirements();
        assert!(idle.last_poll_queue_empty);
        assert!(!idle.retired_after_queue_empty_poll);
        assert!(!idle.last_poll_completed_submission_index_recorded);
        assert!(!idle.retired_after_completed_submission_index_poll);
        assert_eq!(
            idle.tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::NotApplicable
        );
        assert_eq!(
            idle.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::NotApplicable
        );

        let module =
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Neo tombstone gate invalidation module"),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                });
        runtime.queue_shader_variant_module_tombstone(vec![module]);

        let pending = runtime.backend_resource_retirement_stats();
        assert_eq!(pending.tombstones, 1);
        assert_eq!(
            pending.tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::None
        );
        assert!(!pending.last_poll_queue_empty);
        assert!(!pending.retired_after_queue_empty_poll);
        assert!(!pending.last_poll_completed_submission_index_recorded);
        assert!(!pending.retired_after_completed_submission_index_poll);

        let retired = runtime.poll_backend_resource_retirements();
        assert!(retired.last_poll_queue_empty);
        assert!(retired.retired_after_queue_empty_poll);
        assert_eq!(retired.retired_tombstones_this_poll, 1);

        let idle_again = runtime.poll_backend_resource_retirements();
        assert!(idle_again.last_poll_queue_empty);
        assert!(!idle_again.retired_after_queue_empty_poll);
        assert_eq!(idle_again.retired_tombstones_this_poll, 0);
        assert_eq!(
            idle_again.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::NotApplicable
        );
    }

    #[test]
    fn wgpu_backend_tombstone_enqueue_invalidates_previous_completed_submission_gate() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!(
                    "skipping wgpu backend tombstone completed submission gate invalidation test: {error}"
                );
                return;
            }
        };
        let create_module = |runtime: &WgpuRendererRuntime, label| {
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                })
        };
        let encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo completed submission gate invalidation encoder"),
                });
        runtime.last_submission_index =
            Some(runtime.graphics().queue().submit(Some(encoder.finish())));

        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo completed submission gate old module",
        )]);
        runtime.wait_for_gpu();
        let retired = runtime.poll_backend_resource_retirements();
        assert!(retired.last_poll_queue_empty);
        assert!(retired.retired_after_queue_empty_poll);
        assert!(retired.last_poll_completed_submission_index_recorded);
        assert!(retired.retired_after_completed_submission_index_poll);

        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo completed submission gate new module",
        )]);
        let pending = runtime.backend_resource_retirement_stats();
        assert_eq!(pending.tombstones, 1);
        assert!(!pending.last_poll_queue_empty);
        assert!(!pending.retired_after_queue_empty_poll);
        assert!(!pending.last_poll_completed_submission_index_recorded);
        assert!(!pending.retired_after_completed_submission_index_poll);
    }

    #[test]
    fn wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!(
                    "skipping wgpu backend unindexed tombstone completed submission gate test: {error}"
                );
                return;
            }
        };
        let module =
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Neo unindexed tombstone before later submission"),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                });
        runtime.queue_shader_variant_module_tombstone(vec![module]);
        assert_eq!(
            runtime
                .backend_resource_retirement_stats()
                .fence_objects_without_submission_index,
            1
        );
        assert_eq!(
            runtime
                .backend_resource_retirement_stats()
                .tombstones_waiting_for_queue_empty,
            1
        );
        assert_eq!(
            runtime
                .backend_resource_retirement_stats()
                .tombstones_waiting_for_submission_index,
            0
        );

        let encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo later unrelated submission"),
                });
        runtime.last_submission_index =
            Some(runtime.graphics().queue().submit(Some(encoder.finish())));
        runtime.wait_for_gpu();

        let retired = runtime.poll_backend_resource_retirements();
        assert!(retired.last_poll_queue_empty);
        assert!(retired.last_poll_completed_submission_index_recorded);
        assert!(retired.retired_after_queue_empty_poll);
        assert!(!retired.retired_after_completed_submission_index_poll);
        assert_eq!(
            retired.retired_tombstones_with_submission_index_this_poll,
            0
        );
        assert_eq!(
            retired.retired_tombstones_without_submission_index_this_poll,
            1
        );
        assert_eq!(
            retired.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::None
        );
        assert!(!retired.retired_all_tombstones_had_submission_index_this_poll);
        assert!(!retired.retired_partial_tombstone_submission_index_coverage_this_poll);
        assert!(retired.retired_no_tombstones_had_submission_index_this_poll);
        assert_eq!(retired.retired_fence_submission_indices_this_poll, 0);
        assert_eq!(
            retired.retired_fence_objects_without_submission_index_this_poll,
            1
        );
    }

    #[test]
    fn wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!(
                    "skipping wgpu backend mixed tombstone submission coverage test: {error}"
                );
                return;
            }
        };
        let create_module = |runtime: &WgpuRendererRuntime, label| {
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                })
        };

        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo mixed tombstone unindexed module",
        )]);
        let encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo mixed tombstone indexed submission"),
                });
        runtime.last_submission_index =
            Some(runtime.graphics().queue().submit(Some(encoder.finish())));
        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo mixed tombstone indexed module",
        )]);

        let pending = runtime.backend_resource_retirement_stats();
        assert_eq!(pending.tombstones, 2);
        assert_eq!(pending.tombstones_with_submission_index, 1);
        assert_eq!(pending.tombstones_without_submission_index, 1);
        assert_eq!(pending.tombstones_waiting_for_submission_index, 1);
        assert_eq!(pending.tombstones_waiting_for_queue_empty, 1);
        assert_eq!(
            pending.tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::Partial
        );
        assert!(!pending.all_tombstones_have_submission_index);
        assert!(pending.partial_tombstone_submission_index_coverage);
        assert!(!pending.no_tombstones_have_submission_index);

        runtime.wait_for_gpu();
        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.retired_tombstones_this_poll, 2);
        assert_eq!(
            retired.retired_tombstones_with_submission_index_this_poll,
            1
        );
        assert_eq!(
            retired.retired_tombstones_without_submission_index_this_poll,
            1
        );
        assert_eq!(
            retired.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::Partial
        );
        assert!(retired.last_poll_completed_submission_index_recorded);
        assert!(retired.retired_after_completed_submission_index_poll);
        assert!(!retired.retired_all_tombstones_had_submission_index_this_poll);
        assert!(retired.retired_partial_tombstone_submission_index_coverage_this_poll);
        assert!(!retired.retired_no_tombstones_had_submission_index_this_poll);
    }

    #[test]
    fn wgpu_backend_retirement_filters_tombstones_by_completed_submission_index() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu backend per-fence retirement filter test: {error}");
                return;
            }
        };
        let create_module = |runtime: &WgpuRendererRuntime, label| {
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                })
        };

        let first_encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo per-fence first submission"),
                });
        let first_submission = runtime
            .graphics()
            .queue()
            .submit(Some(first_encoder.finish()));
        runtime.last_submission_index = Some(first_submission.clone());
        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo per-fence first tombstone",
        )]);
        let first_order = runtime.backend_resource_tombstones[0]
            .fence
            .as_ref()
            .and_then(|fence| fence.submission_order)
            .expect("first tombstone should capture a backend submission order");

        let second_encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo per-fence second submission"),
                });
        let second_submission = runtime
            .graphics()
            .queue()
            .submit(Some(second_encoder.finish()));
        runtime.last_submission_index = Some(second_submission.clone());
        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo per-fence second tombstone",
        )]);
        let second_order = runtime.backend_resource_tombstones[1]
            .fence
            .as_ref()
            .and_then(|fence| fence.submission_order)
            .expect("second tombstone should capture a backend submission order");

        let pending = runtime.backend_resource_retirement_stats();
        assert_eq!(pending.tombstones, 2);
        assert!(pending.nonblocking_submission_index_poll_supported);
        assert!(pending.queue_empty_poll_fallback);
        assert!(!pending.last_poll_used_queue_empty_fallback);
        assert_eq!(pending.tombstones_with_submission_index, 2);
        assert_eq!(pending.tombstones_waiting_for_submission_index, 2);
        assert_eq!(pending.tombstones_waiting_for_queue_empty, 0);
        assert_eq!(
            pending.tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::All
        );

        runtime.clear_backend_retired_this_poll();
        runtime
            .backend_resource_retirement_stats
            .last_poll_completed_submission_index_recorded = true;
        runtime.retire_backend_resource_tombstones(Some(first_order), false);
        runtime.refresh_backend_resource_tombstone_stats();

        let first_retire = runtime.backend_resource_retirement_stats();
        assert_eq!(first_retire.tombstones, 1);
        assert_eq!(first_retire.tombstones_with_submission_index, 1);
        assert_eq!(first_retire.tombstones_waiting_for_submission_index, 1);
        assert_eq!(first_retire.tombstones_waiting_for_queue_empty, 0);
        assert_eq!(first_retire.retired_tombstones_this_poll, 1);
        assert!(!first_retire.last_poll_used_queue_empty_fallback);
        assert_eq!(
            first_retire.retired_tombstone_submission_index_coverage_this_poll,
            WgpuTombstoneSubmissionIndexCoverage::All
        );
        assert!(!first_retire.retired_after_queue_empty_poll);
        assert!(first_retire.retired_after_completed_submission_index_poll);
        assert_eq!(first_retire.retired_fence_submission_indices_this_poll, 1);

        runtime.clear_backend_retired_this_poll();
        runtime
            .backend_resource_retirement_stats
            .last_poll_completed_submission_index_recorded = true;
        runtime.retire_backend_resource_tombstones(Some(second_order), false);
        runtime.refresh_backend_resource_tombstone_stats();

        let second_retire = runtime.backend_resource_retirement_stats();
        assert_eq!(second_retire.tombstones, 0);
        assert_eq!(second_retire.retired_tombstones_this_poll, 1);
        assert!(second_retire.retired_after_completed_submission_index_poll);
        assert_eq!(second_retire.retired_fence_submission_indices_this_poll, 1);

        runtime.wait_for_gpu();
    }

    #[test]
    fn wgpu_submission_fence_reuses_tracker_for_repeated_same_submission_index() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!(
                    "skipping wgpu repeated-submission-index tracker reuse test: {error}"
                );
                return;
            }
        };
        let create_module = |runtime: &WgpuRendererRuntime, label| {
            runtime
                .graphics()
                .device()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(
                        "@compute @workgroup_size(1) fn cs_main() {}".into(),
                    ),
                })
        };

        let encoder =
            runtime
                .graphics()
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo reused tracker same submission encoder"),
                });
        runtime.last_submission_index =
            Some(runtime.graphics().queue().submit(Some(encoder.finish())));

        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo repeated-submission index tracker module 1",
        )]);
        let first_pending = runtime.backend_submission_completions.len();
        let first_tombstone_order = runtime.backend_resource_tombstones[0]
            .fence
            .as_ref()
            .and_then(|fence| fence.submission_order)
            .expect("first tombstone should capture submission order");

        runtime.queue_shader_variant_module_tombstone(vec![create_module(
            &runtime,
            "Neo repeated-submission index tracker module 2",
        )]);
        let second_pending = runtime.backend_submission_completions.len();
        let second_tombstone_order = runtime.backend_resource_tombstones[1]
            .fence
            .as_ref()
            .and_then(|fence| fence.submission_order)
            .expect("second tombstone should capture submission order");

        assert_eq!(runtime.backend_resource_retirement_stats().tombstones, 2);
        assert_eq!(
            runtime.backend_resource_retirement_stats().tombstone_submission_index_coverage,
            WgpuTombstoneSubmissionIndexCoverage::All
        );
        assert_eq!(first_pending, 1);
        assert_eq!(second_pending, 1);
        assert_eq!(first_tombstone_order, second_tombstone_order);

        runtime.wait_for_gpu();
        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.retired_tombstones_this_poll, 2);
        assert!(!runtime
            .backend_resource_retirement_stats
            .nonblocking_submission_index_poll_supported);
        assert_eq!(runtime.backend_submission_completions.len(), 0);
    }

    #[test]
    fn wgpu_tombstone_submission_index_coverage_enum_covers_all_states() {
        assert_eq!(
            tombstone_submission_index_coverage(0, 0, 0),
            WgpuTombstoneSubmissionIndexCoverage::NotApplicable
        );
        assert_eq!(
            tombstone_submission_index_coverage(2, 0, 2),
            WgpuTombstoneSubmissionIndexCoverage::None
        );
        assert_eq!(
            tombstone_submission_index_coverage(2, 1, 1),
            WgpuTombstoneSubmissionIndexCoverage::Partial
        );
        assert_eq!(
            tombstone_submission_index_coverage(2, 2, 0),
            WgpuTombstoneSubmissionIndexCoverage::All
        );
    }

    #[test]
    fn wgpu_material_texture_binding_generates_mips_on_gpu() {
        let runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material texture GPU mip generation test: {error}");
                return;
            }
        };
        let desc = WgpuMaterialTextureUploadDesc {
            label: Some("Neo GPU Generated Mip Texture".to_owned()),
            dimension: TextureDimension::D2,
            width: 4,
            height: 4,
            depth_or_layers: 1,
            mip_level_count: 3,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            sampled_binding: true,
            storage_binding: false,
            generate_mips_from_base: true,
            uploads: vec![WgpuMaterialTextureUpload {
                mip_level: 0,
                origin: [0, 0, 0],
                extent: [4, 4, 1],
                bytes_per_row: 16,
                rows_per_image: 4,
                bytes: vec![255; 64],
            }],
        };

        let binding = create_wgpu_material_texture_binding(
            runtime.graphics.device(),
            runtime.graphics.queue(),
            &desc,
        )
        .unwrap();
        assert_eq!(binding.generated_mips, 2);
        runtime.wait_for_gpu();
    }

    #[test]
    fn wgpu_material_texture_binding_generates_float_mips_on_gpu() {
        let runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!(
                    "skipping wgpu material float texture GPU mip generation test: {error}"
                );
                return;
            }
        };
        for format in [TextureFormat::Rgba16Float, TextureFormat::Rgba32Float] {
            let bytes_per_pixel = wgpu_material_texture_format_bytes_per_pixel(format);
            let desc = WgpuMaterialTextureUploadDesc {
                label: Some(format!("Neo GPU Generated Float Mip Texture ({format:?})").to_owned()),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_level_count: 3,
                sample_count: 1,
                format,
                sampled_binding: true,
                storage_binding: false,
                generate_mips_from_base: true,
                uploads: vec![WgpuMaterialTextureUpload {
                    mip_level: 0,
                    origin: [0, 0, 0],
                    extent: [4, 4, 1],
                    bytes_per_row: 4 * bytes_per_pixel,
                    rows_per_image: 4,
                    bytes: vec![0xFF; 4 * 4 * 1 * bytes_per_pixel as usize],
                }],
            };

            let binding = create_wgpu_material_texture_binding(
                runtime.graphics.device(),
                runtime.graphics.queue(),
                &desc,
            )
            .unwrap();
            assert_eq!(binding.generated_mips, 2);
        }
        runtime.wait_for_gpu();
    }

    #[test]
    fn wgpu_material_array_texture_binding_generates_layer_mips_on_gpu() {
        let runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material array texture GPU mip generation test: {error}");
                return;
            }
        };
        let desc = WgpuMaterialTextureUploadDesc {
            label: Some("Neo GPU Generated Array Mip Texture".to_owned()),
            dimension: TextureDimension::D2Array,
            width: 4,
            height: 4,
            depth_or_layers: 2,
            mip_level_count: 3,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            sampled_binding: true,
            storage_binding: false,
            generate_mips_from_base: true,
            uploads: vec![WgpuMaterialTextureUpload {
                mip_level: 0,
                origin: [0, 0, 0],
                extent: [4, 4, 2],
                bytes_per_row: 16,
                rows_per_image: 4,
                bytes: vec![255; 128],
            }],
        };

        let binding = create_wgpu_material_texture_binding(
            runtime.graphics.device(),
            runtime.graphics.queue(),
            &desc,
        )
        .unwrap();
        assert_eq!(binding.generated_mips, 4);
        runtime.wait_for_gpu();
    }

    #[test]
    fn wgpu_material_cube_texture_binding_generates_face_mips_on_gpu() {
        let runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material cube texture GPU mip generation test: {error}");
                return;
            }
        };
        let desc = WgpuMaterialTextureUploadDesc {
            label: Some("Neo GPU Generated Cube Mip Texture".to_owned()),
            dimension: TextureDimension::Cube,
            width: 4,
            height: 4,
            depth_or_layers: 6,
            mip_level_count: 3,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            sampled_binding: true,
            storage_binding: false,
            generate_mips_from_base: true,
            uploads: vec![WgpuMaterialTextureUpload {
                mip_level: 0,
                origin: [0, 0, 0],
                extent: [4, 4, 6],
                bytes_per_row: 16,
                rows_per_image: 4,
                bytes: vec![255; 384],
            }],
        };

        let binding = create_wgpu_material_texture_binding(
            runtime.graphics.device(),
            runtime.graphics.queue(),
            &desc,
        )
        .unwrap();
        assert_eq!(binding.generated_mips, 12);
        runtime.wait_for_gpu();
    }

    #[test]
    fn wgpu_material_texture_gpu_mip_generation_rejects_invalid_descs() {
        let runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu material texture GPU mip validation test: {error}");
                return;
            }
        };
        let valid = WgpuMaterialTextureUploadDesc {
            label: Some("Neo Invalid GPU Mip Texture".to_owned()),
            dimension: TextureDimension::D2,
            width: 4,
            height: 4,
            depth_or_layers: 1,
            mip_level_count: 3,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            sampled_binding: true,
            storage_binding: false,
            generate_mips_from_base: true,
            uploads: vec![WgpuMaterialTextureUpload {
                mip_level: 0,
                origin: [0, 0, 0],
                extent: [4, 4, 1],
                bytes_per_row: 16,
                rows_per_image: 4,
                bytes: vec![255; 64],
            }],
        };
        let assert_invalid = |desc: WgpuMaterialTextureUploadDesc| {
            assert!(matches!(
                create_wgpu_material_texture_binding(
                    runtime.graphics.device(),
                    runtime.graphics.queue(),
                    &desc,
                ),
                Err(RendererError::Validation(_))
            ));
        };

        let mut invalid = valid.clone();
        invalid.sample_count = 2;
        assert_invalid(invalid);

        let mut invalid = valid.clone();
        invalid.mip_level_count = 5;
        assert_invalid(invalid);

        let mut invalid = valid.clone();
        invalid.dimension = TextureDimension::D3;
        assert_invalid(invalid);

        let mut invalid = valid.clone();
        invalid.depth_or_layers = 2;
        invalid.uploads[0].extent = [4, 4, 2];
        invalid.uploads[0].bytes = vec![255; 128];
        assert_invalid(invalid);

        let mut invalid = valid.clone();
        invalid.dimension = TextureDimension::Cube;
        invalid.depth_or_layers = 5;
        invalid.uploads[0].extent = [4, 4, 5];
        invalid.uploads[0].bytes = vec![255; 320];
        assert_invalid(invalid);

        let mut invalid = valid.clone();
        invalid.uploads.clear();
        assert_invalid(invalid);

        let mut invalid = valid.clone();
        invalid.uploads[0].extent = [2, 4, 1];
        invalid.uploads[0].bytes = vec![255; 32];
        assert_invalid(invalid);

        let mut invalid = valid;
        invalid.uploads[0].bytes.pop();
        assert_invalid(invalid);
    }

    #[test]
    fn wgpu_native_pipeline_cache_metadata_reports_backend_stats() {
        let mut cache = WgpuNativePipelineCacheMetadata::default();
        let key_a = wgpu_test_pipeline_key(11, 21, 31);
        let key_b = wgpu_test_pipeline_key(12, 22, 32);

        cache.record_ready_pipeline(key_a, 101);
        cache.record_ready_pipeline(key_b, 202);
        cache.mark_used(key_b, 7).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.ready, 2);
        assert_eq!(stats.backend_objects, 2);
        assert_eq!(stats.shader_interface_layouts, 2);
        assert_eq!(stats.entries_used_this_frame, 1);
        assert_eq!(stats.ready_unused_entries, 1);
        assert_eq!(cache.entry(key_b).unwrap().last_used_frame, Some(7));
        assert!(cache.entry(key_b).unwrap().used_this_frame);

        cache.begin_frame();
        let stats = cache.stats();
        assert_eq!(stats.entries_used_this_frame, 0);
        assert_eq!(stats.ready_unused_entries, 2);
        assert_eq!(stats.invalidated_this_frame, 0);

        assert!(cache.invalidate(key_a));
        let stats = cache.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.invalidated_this_frame, 1);

        cache.clear();
        let stats = cache.stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.invalidated_this_frame, 2);
    }

    #[test]
    fn wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory() {
        let mut stats = frame_stats_from_wgpu_metrics(
            4,
            vec![
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Post Process Pass".to_owned(),
            ],
            RenderQueueStats::default(),
            MeshRenderStats::default(),
            MeshRenderer::STATIC_RENDER_PIPELINE_COUNT,
            3,
            ResourceReclaimPolicy::FrameLatency { frames: 2 },
            false,
        );
        let native_stats = PipelineCacheStats {
            total: 2,
            ready: 2,
            backend_objects: 2,
            shader_interface_layouts: 2,
            entries_used_this_frame: 1,
            ready_unused_entries: 1,
            invalidated_this_frame: 1,
            ..PipelineCacheStats::default()
        };
        let native_backend_objects = native_stats.backend_objects;

        merge_wgpu_pipeline_cache_stats(&mut stats.pipeline_cache, native_stats);

        assert_eq!(stats.pipeline_cache.total, 2);
        assert_eq!(stats.pipeline_cache.ready, 2);
        assert_eq!(
            stats.pipeline_cache.backend_objects,
            MeshRenderer::STATIC_RENDER_PIPELINE_COUNT + native_backend_objects
        );
        assert_eq!(stats.pipeline_cache.shader_interface_layouts, 5);
        assert_eq!(stats.pipeline_cache.entries_used_this_frame, 1);
        assert_eq!(stats.pipeline_cache.ready_unused_entries, 1);
        assert_eq!(stats.pipeline_cache.invalidated_this_frame, 1);
        assert!(stats
            .pipeline_cache
            .has_complete_facade_backend_object_coverage());

        let mut stats_with_gap = PipelineCacheStats {
            total: 1,
            ready: 1,
            ready_entries_without_backend_object: 1,
            used_entries_without_backend_object: 1,
            ..PipelineCacheStats::default()
        };
        merge_wgpu_pipeline_cache_stats(
            &mut stats_with_gap,
            PipelineCacheStats {
                total: 1,
                ready: 1,
                backend_objects: 1,
                ..PipelineCacheStats::default()
            },
        );
        assert_eq!(stats_with_gap.backend_objects, 1);
        assert_eq!(stats_with_gap.ready_backend_object_gap(), 1);
        assert_eq!(stats_with_gap.used_backend_object_gap(), 1);
        assert!(!stats_with_gap.has_complete_facade_backend_object_coverage());
    }

    #[test]
    fn wgpu_reflected_pipeline_smoke_creates_and_binds_native_objects() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu reflected pipeline smoke test: {error}");
                return;
            }
        };
        let shader_source = r#"
            @group(0) @binding(0) var<uniform> tint: vec4<f32>;

            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                let x = select(-1.0, 3.0, vertex_index == 2u);
                let y = select(-1.0, 3.0, vertex_index == 1u);
                return vec4<f32>(x, y, 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return tint;
            }
        "#;
        let interface = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "tint".to_owned(),
                group: 0,
                binding: 0,
                binding_class: BindingClass::Uniform,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer,
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        let material_plan = wgpu_material_bind_group_resource_plan(
            &interface,
            &[MaterialParameter {
                name: "tint".to_owned(),
                value: MaterialParameterValue::Bytes(vec![
                    0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63,
                ]),
            }],
        )
        .unwrap();
        let key = wgpu_test_pipeline_key(31, 41, 51);

        runtime
            .create_and_cache_native_render_pipeline_with_registered_resources(
                WgpuNativeRenderPipelineBuildDesc {
                    label: Some("Neo Reflected Pipeline Smoke"),
                    key,
                    shader_interface_layout_hash: 303,
                    shader_source: ShaderSource::Wgsl(shader_source),
                    interface: &interface,
                    material_resource_plan: Some(&material_plan),
                    vertex_entry: "vs_main",
                    fragment_entry: Some("fs_main"),
                    vertex_buffers: &[],
                    color_format: Some(TextureFormat::Rgba8Unorm),
                    depth_format: None,
                    sample_count: 1,
                    depth_write: false,
                    blend: None,
                },
            )
            .unwrap();

        let device = runtime.graphics.device();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Neo Reflected Pipeline Smoke Target"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let submission_info = runtime
            .submit_native_pipeline_draw_to_view(WgpuNativePipelineDrawDesc {
                label: Some("Neo Reflected Pipeline Smoke Pass"),
                key,
                color_view: &target_view,
                clear_color: wgpu::Color::BLACK,
                vertices: 0..3,
                instances: 0..1,
            })
            .unwrap();
        assert_eq!(submission_info.bind_group_count, 1);
        assert_eq!(submission_info.vertex_count, 3);
        assert_eq!(submission_info.instance_count, 1);
        assert!(runtime.last_submission_index().is_some());
        runtime.wait_for_gpu();

        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.backend_objects, 1);
        assert_eq!(stats.entries_used_this_frame, 1);
        assert_eq!(stats.shader_interface_layouts, 1);
    }

    #[test]
    fn wgpu_float64_vertex_attribute_pipeline_smoke_is_cap_gated() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu float64 vertex attribute pipeline smoke test: {error}");
                return;
            }
        };
        if !runtime
            .graphics
            .device()
            .features()
            .contains(wgpu::Features::VERTEX_ATTRIBUTE_64BIT)
        {
            assert!(!runtime
                .renderer_caps()
                .features
                .contains(RendererFeatures::VERTEX_ATTRIBUTE_64BIT));
            eprintln!("skipping wgpu float64 vertex attribute pipeline smoke test: device feature unavailable");
            return;
        }
        assert!(runtime
            .renderer_caps()
            .features
            .contains(RendererFeatures::VERTEX_ATTRIBUTE_64BIT));

        let shader_source = r#"
            @vertex
            fn vs_main(@location(0) value: f32) -> @builtin(position) vec4<f32> {
                return vec4<f32>(value, 0.0, 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 1.0, 1.0, 1.0);
            }
        "#;
        let interface = ShaderInterfaceDesc {
            resources: Vec::new(),
            push_constants: Vec::new(),
            vertex_inputs: vec![crate::VertexInputRequirement {
                semantic: crate::VertexSemantic::Custom(0),
                format: crate::VertexFormat::Float64,
            }],
        };
        let vertex_attributes = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float64,
            offset: 0,
            shader_location: 0,
        }];
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attributes,
        }];
        let key = wgpu_test_pipeline_key(91, 92, 93);

        runtime
            .create_and_cache_native_render_pipeline_with_registered_resources(
                WgpuNativeRenderPipelineBuildDesc {
                    label: Some("Neo Float64 Vertex Attribute Pipeline Smoke"),
                    key,
                    shader_interface_layout_hash: 9193,
                    shader_source: ShaderSource::Wgsl(shader_source),
                    interface: &interface,
                    material_resource_plan: None,
                    vertex_entry: "vs_main",
                    fragment_entry: Some("fs_main"),
                    vertex_buffers: &vertex_buffers,
                    color_format: Some(TextureFormat::Rgba8Unorm),
                    depth_format: None,
                    sample_count: 1,
                    depth_write: false,
                    blend: None,
                },
            )
            .unwrap();

        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.backend_objects, 1);
    }

    #[test]
    fn wgpu_native_pipeline_replacement_enters_backend_tombstone() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu native pipeline replacement tombstone test: {error}");
                return;
            }
        };
        let shader_source = r#"
            @group(0) @binding(0) var<uniform> tint: vec4<f32>;

            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                let x = select(-1.0, 3.0, vertex_index == 2u);
                let y = select(-1.0, 3.0, vertex_index == 1u);
                return vec4<f32>(x, y, 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return tint;
            }
        "#;
        let interface = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "tint".to_owned(),
                group: 0,
                binding: 0,
                binding_class: BindingClass::Uniform,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer,
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        let key = wgpu_test_pipeline_key(172, 184, 192);

        for tint in [
            vec![0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63],
            vec![0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 128, 63],
        ] {
            let material_plan = wgpu_material_bind_group_resource_plan(
                &interface,
                &[MaterialParameter {
                    name: "tint".to_owned(),
                    value: MaterialParameterValue::Bytes(tint),
                }],
            )
            .unwrap();
            runtime
                .create_and_cache_native_render_pipeline_with_registered_resources(
                    WgpuNativeRenderPipelineBuildDesc {
                        label: Some("Neo Reflected Pipeline Replacement"),
                        key,
                        shader_interface_layout_hash: 1909,
                        shader_source: ShaderSource::Wgsl(shader_source),
                        interface: &interface,
                        material_resource_plan: Some(&material_plan),
                        vertex_entry: "vs_main",
                        fragment_entry: Some("fs_main"),
                        vertex_buffers: &[],
                        color_format: Some(TextureFormat::Rgba8Unorm),
                        depth_format: None,
                        sample_count: 1,
                        depth_write: false,
                        blend: None,
                    },
                )
                .unwrap();
        }

        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.backend_objects, 1);
        assert_eq!(stats.invalidated_this_frame, 0);
        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 1);
        assert_eq!(retirement.fence_objects, 1);
        assert_eq!(retirement.native_pipeline_entries, 1);
        assert_eq!(retirement.render_pipeline_refs, 1);
        assert_eq!(retirement.shader_modules, 1);
        assert_eq!(retirement.bind_groups, 1);
        assert_eq!(retirement.owned_buffers, 1);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 1);
        assert_eq!(retired.retired_fence_objects_this_poll, 1);
        assert_eq!(retired.retired_native_pipeline_entries_this_poll, 1);
        assert_eq!(retired.retired_render_pipeline_refs_this_poll, 1);
        assert_eq!(retired.retired_shader_modules_this_poll, 1);
        assert_eq!(retired.retired_bind_groups_this_poll, 1);
        assert_eq!(retired.retired_owned_buffers_this_poll, 1);
    }

    #[test]
    fn wgpu_native_cache_reuses_render_pipeline_across_material_bind_groups() {
        let mut runtime = match WgpuRendererRuntime::new(RendererConfig::default()) {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("skipping wgpu native cache reuse test: {error}");
                return;
            }
        };
        let shader_source = r#"
            @group(0) @binding(0) var<uniform> tint: vec4<f32>;

            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                let x = select(-1.0, 3.0, vertex_index == 2u);
                let y = select(-1.0, 3.0, vertex_index == 1u);
                return vec4<f32>(x, y, 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return tint;
            }
        "#;
        let interface = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "tint".to_owned(),
                group: 0,
                binding: 0,
                binding_class: BindingClass::Uniform,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer,
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        let render_pipeline_key = wgpu_test_pipeline_key(71, 81, 91);
        for (material, tint) in [
            (
                82,
                vec![0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63],
            ),
            (
                83,
                vec![0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 128, 63],
            ),
        ] {
            let material_plan = wgpu_material_bind_group_resource_plan(
                &interface,
                &[MaterialParameter {
                    name: "tint".to_owned(),
                    value: MaterialParameterValue::Bytes(tint),
                }],
            )
            .unwrap();
            runtime
                .create_and_cache_native_render_pipeline_with_registered_resources_and_render_key(
                    WgpuNativeRenderPipelineBuildDesc {
                        label: Some("Neo Reflected Pipeline Reuse"),
                        key: wgpu_test_pipeline_key(71, material, 91),
                        shader_interface_layout_hash: 909,
                        shader_source: ShaderSource::Wgsl(shader_source),
                        interface: &interface,
                        material_resource_plan: Some(&material_plan),
                        vertex_entry: "vs_main",
                        fragment_entry: Some("fs_main"),
                        vertex_buffers: &[],
                        color_format: Some(TextureFormat::Rgba8Unorm),
                        depth_format: None,
                        sample_count: 1,
                        depth_write: false,
                        blend: None,
                    },
                    render_pipeline_key,
                )
                .unwrap();
        }

        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.backend_objects, 1);
        assert_eq!(stats.ready_unused_entries, 2);
        assert_eq!(stats.shader_interface_layouts, 1);

        let removed = runtime.invalidate_native_pipelines_for_shader(render_pipeline_key.shader);
        assert_eq!(removed, 2);
        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.backend_objects, 0);
        assert_eq!(stats.invalidated_this_frame, 2);
        let retirement = runtime.backend_resource_retirement_stats();
        assert_eq!(retirement.tombstones, 2);
        assert_eq!(retirement.fence_objects, 2);
        assert_eq!(retirement.native_pipeline_entries, 2);
        assert_eq!(retirement.render_pipeline_refs, 2);
        assert_eq!(retirement.shader_modules, 2);
        assert_eq!(retirement.bind_groups, 2);
        assert_eq!(retirement.owned_buffers, 2);

        let retired = runtime.poll_backend_resource_retirements();
        assert_eq!(retired.tombstones, 0);
        assert_eq!(retired.retired_tombstones_this_poll, 2);
        assert_eq!(retired.retired_fence_objects_this_poll, 2);
        assert_eq!(retired.retired_native_pipeline_entries_this_poll, 2);
        assert_eq!(retired.retired_render_pipeline_refs_this_poll, 2);
        assert_eq!(retired.retired_shader_modules_this_poll, 2);
        assert_eq!(retired.retired_bind_groups_this_poll, 2);
        assert_eq!(retired.retired_owned_buffers_this_poll, 2);

        let idle_retired = runtime.poll_backend_resource_retirements();
        assert_eq!(idle_retired.retired_tombstones_this_poll, 0);

        let first_key = wgpu_test_pipeline_key(72, 84, 92);
        let second_key = wgpu_test_pipeline_key(73, 84, 93);
        let material_handle = MaterialHandle::from_raw(std::num::NonZeroU64::new(501).unwrap());
        for key in [first_key, second_key] {
            let material_plan = wgpu_material_bind_group_resource_plan(
                &interface,
                &[MaterialParameter {
                    name: "tint".to_owned(),
                    value: MaterialParameterValue::Bytes(vec![
                        0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63,
                    ]),
                }],
            )
            .unwrap();
            runtime
                .create_and_cache_native_render_pipeline_with_registered_resources_and_render_key(
                    WgpuNativeRenderPipelineBuildDesc {
                        label: Some("Neo Reflected Pipeline Template Invalidation"),
                        key,
                        shader_interface_layout_hash: 910,
                        shader_source: ShaderSource::Wgsl(shader_source),
                        interface: &interface,
                        material_resource_plan: Some(&material_plan),
                        vertex_entry: "vs_main",
                        fragment_entry: Some("fs_main"),
                        vertex_buffers: &[],
                        color_format: Some(TextureFormat::Rgba8Unorm),
                        depth_format: None,
                        sample_count: 1,
                        depth_write: false,
                        blend: None,
                    },
                    key,
                )
                .unwrap();
            runtime
                .tag_native_pipeline_material(key, material_handle)
                .unwrap();
        }
        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.backend_objects, 2);

        let removed = runtime.invalidate_native_pipelines_for_material(material_handle);
        assert_eq!(removed, 2);
        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.backend_objects, 0);
        assert_eq!(stats.invalidated_this_frame, 4);

        for key in [first_key, second_key] {
            let material_plan = wgpu_material_bind_group_resource_plan(
                &interface,
                &[MaterialParameter {
                    name: "tint".to_owned(),
                    value: MaterialParameterValue::Bytes(vec![
                        0, 0, 128, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63,
                    ]),
                }],
            )
            .unwrap();
            runtime
                .create_and_cache_native_render_pipeline_with_registered_resources_and_render_key(
                    WgpuNativeRenderPipelineBuildDesc {
                        label: Some("Neo Reflected Pipeline Template Invalidation"),
                        key,
                        shader_interface_layout_hash: 910,
                        shader_source: ShaderSource::Wgsl(shader_source),
                        interface: &interface,
                        material_resource_plan: Some(&material_plan),
                        vertex_entry: "vs_main",
                        fragment_entry: Some("fs_main"),
                        vertex_buffers: &[],
                        color_format: Some(TextureFormat::Rgba8Unorm),
                        depth_format: None,
                        sample_count: 1,
                        depth_write: false,
                        blend: None,
                    },
                    key,
                )
                .unwrap();
        }
        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.backend_objects, 2);

        let removed =
            runtime.invalidate_native_pipelines_for_material_template(first_key.material_template);
        assert_eq!(removed, 2);
        let stats = runtime.native_pipeline_cache_stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.backend_objects, 0);
        assert_eq!(stats.invalidated_this_frame, 6);
    }

    fn wgpu_test_pipeline_key(shader: u64, material: u64, vertex_layout_hash: u64) -> PipelineKey {
        PipelineKey {
            shader: ShaderHandle::from_raw(std::num::NonZeroU64::new(shader).unwrap()),
            material_template: MaterialTemplateHandle::from_raw(
                std::num::NonZeroU64::new(material).unwrap(),
            ),
            vertex_layout_hash,
            render_state_hash: 41,
            pass: RenderPhaseKind::ForwardOpaque,
            sample_count: 1,
            depth_format: DepthFormat::D32Float,
            color_format: TextureFormat::Rgba8Unorm,
            feature_bits: 1,
        }
    }

    #[test]
    fn vsync_modes_map_to_wgpu_present_modes() {
        assert_eq!(
            present_mode_for_vsync(VSyncMode::Off),
            PresentMode::Immediate
        );
        assert_eq!(present_mode_for_vsync(VSyncMode::On), PresentMode::Fifo);
        assert_eq!(
            present_mode_for_vsync(VSyncMode::Adaptive),
            PresentMode::AutoVsync
        );
    }

    #[test]
    fn renderer_config_maps_to_wgpu_surface_options() {
        let options = surface_options(&RendererConfig {
            surface_format: Some(TextureFormat::Bgra8UnormSrgb),
            depth_format: DepthFormat::D24Plus,
            ..RendererConfig::default()
        });

        assert_eq!(
            options.preferred_format,
            Some(wgpu::TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(options.depth_format, wgpu::TextureFormat::Depth24Plus);
    }

    #[test]
    fn validate_surface_runtime_formats_rejects_configured_color_format_mismatch() {
        let mut caps = RendererCaps::for_backend(&RendererConfig::default(), "validation", "validation");
        caps.formats = FormatCaps {
            color: vec![TextureFormat::Rgba8Unorm],
            depth: vec![DepthFormat::D32Float],
        };
        let config = RendererConfig {
            surface_format: Some(TextureFormat::Rgba8Unorm),
            depth_format: DepthFormat::D32Float,
            ..RendererConfig::default()
        };

        assert!(matches!(
            validate_surface_runtime_formats(
                &config,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                Some(wgpu::TextureFormat::Depth32Float),
                &caps
            ),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn validate_surface_runtime_formats_rejects_configured_depth_format_mismatch() {
        let mut caps = RendererCaps::for_backend(&RendererConfig::default(), "validation", "validation");
        caps.formats = FormatCaps {
            color: vec![TextureFormat::Rgba8Unorm],
            depth: vec![DepthFormat::D24Plus],
        };
        let config = RendererConfig {
            surface_format: Some(TextureFormat::Rgba8Unorm),
            depth_format: DepthFormat::D32Float,
            ..RendererConfig::default()
        };

        assert!(matches!(
            validate_surface_runtime_formats(
                &config,
                wgpu::TextureFormat::Rgba8Unorm,
                Some(wgpu::TextureFormat::Depth24Plus),
                &caps
            ),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn wgpu_metrics_map_gpu_timestamps_to_frame_stats() {
        let queue_stats = RenderQueueStats {
            item_count: 5,
            culled_item_count: 2,
            ..RenderQueueStats::default()
        };
        let gpu_stats = MeshRenderStats {
            draw_call_count: 12,
            mesh_pass_draw_call_count: 3,
            skybox_draw_call_count: 1,
            gbuffer_draw_call_count: 2,
            deferred_lighting_draw_call_count: 1,
            depth_prepass_draw_call_count: 1,
            shadow_draw_call_count: 3,
            directional_shadow_draw_call_count: 1,
            spot_shadow_draw_call_count: 1,
            point_shadow_draw_call_count: 1,
            opaque_draw_call_count: 2,
            transparent_draw_call_count: 1,
            post_process_draw_call_count: 1,
            instance_buffer_capacity: 8,
            timestamp_writes: 2,
            gpu_time_ns: Some(3_250_000),
            ..MeshRenderStats::default()
        };

        let stats = frame_stats_from_wgpu_metrics(
            12,
            vec![
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned(),
            ],
            queue_stats,
            gpu_stats.clone(),
            33,
            3,
            ResourceReclaimPolicy::FrameLatency { frames: 2 },
            true,
        );

        assert_eq!(stats.frame_index, 12);
        assert!(stats.gpu_profiler_enabled);
        assert_eq!(stats.gpu_time_ms, Some(3.25));
        assert_eq!(stats.graph.gpu_time_ns, Some(3_250_000));
        assert_eq!(stats.graph.timestamp_queries, 1);
        assert_eq!(stats.graph.timestamp_writes, 2);
        assert_eq!(stats.graph.fullscreen_draws, 2);
        assert_eq!(stats.graph.pass_count, 7);
        assert_eq!(stats.graph.rhi_executed_passes, 7);
        assert_eq!(
            stats.graph.rhi_executed_pass_labels,
            vec![
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned()
            ]
        );
        assert_eq!(stats.draw_calls, 12);
        assert_eq!(stats.backend_mesh_pass_draw_calls, 3);
        assert_eq!(stats.backend_skybox_draw_calls, 1);
        assert_eq!(stats.backend_gbuffer_draw_calls, 2);
        assert_eq!(stats.backend_deferred_lighting_draw_calls, 1);
        assert_eq!(stats.backend_depth_prepass_draw_calls, 1);
        assert_eq!(stats.backend_shadow_draw_calls, 3);
        assert_eq!(stats.backend_directional_shadow_draw_calls, 1);
        assert_eq!(stats.backend_spot_shadow_draw_calls, 1);
        assert_eq!(stats.backend_point_shadow_draw_calls, 1);
        assert_eq!(stats.backend_opaque_draw_calls, 2);
        assert_eq!(stats.backend_transparent_draw_calls, 1);
        assert_eq!(stats.backend_post_process_draw_calls, 1);
        assert_eq!(stats.backend_directional_shadow_passes, 1);
        assert_eq!(stats.backend_spot_shadow_passes, 0);
        assert_eq!(stats.backend_point_shadow_passes, 0);
        assert_eq!(stats.backend_gbuffer_passes, 1);
        assert_eq!(stats.backend_deferred_lighting_passes, 1);
        assert_eq!(stats.backend_depth_prepass_passes, 1);
        assert_eq!(stats.backend_forward_opaque_passes, 1);
        assert_eq!(stats.backend_transparent_passes, 1);
        assert_eq!(stats.backend_post_process_passes, 1);
        assert_eq!(
            stats.backend_native_pass_label_capacity,
            MeshRenderStats::native_pass_label_capacity() as u32
        );
        assert_eq!(stats.backend_native_pass_labels_dropped, 0);
        assert_eq!(
            stats.backend_native_pass_draws,
            vec![
                BackendNativePassDrawStats {
                    pass_label: "Neo Directional Shadow Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Depth Prepass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo GBuffer Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 2,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Deferred Lighting Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Forward Opaque Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 3,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Transparent Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Tonemap Post Process Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
            ]
        );
        assert_eq!(stats.visible_objects, 5);
        assert_eq!(stats.culled_objects, 2);
        assert_eq!(stats.pipeline_cache.backend_objects, 33);
        assert_eq!(stats.pipeline_cache.shader_interface_layouts, 3);
        assert_eq!(
            stats.memory.resident_bytes,
            gpu_stats.instance_buffer_bytes() as u64
        );
        assert_eq!(
            stats.memory.reclaim_policy,
            ResourceReclaimPolicy::FrameLatency { frames: 2 }
        );
    }

    #[test]
    fn backend_native_pass_draw_stats_counts_repeated_pass_instances() {
        let stats = backend_native_pass_draw_stats(
            &[
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Hdr Bloom Ssao Taa Fxaa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass".to_owned(),
            ],
            &MeshRenderStats {
                directional_shadow_draw_call_count: 4,
                point_shadow_draw_call_count: 12,
                depth_prepass_draw_call_count: 2,
                gbuffer_draw_call_count: 2,
                deferred_lighting_draw_call_count: 1,
                opaque_draw_call_count: 2,
                transparent_draw_call_count: 1,
                skybox_draw_call_count: 1,
                post_process_draw_call_count: 1,
                ..MeshRenderStats::default()
            },
        );

        assert_eq!(
            stats,
            vec![
                BackendNativePassDrawStats {
                    pass_label: "Neo Directional Shadow Pass".to_owned(),
                    pass_instances: 2,
                    draw_calls: 4,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Point Shadow Pass".to_owned(),
                    pass_instances: 6,
                    draw_calls: 12,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Depth Prepass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 2,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo GBuffer Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 2,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Deferred Lighting Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Forward Opaque Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 3,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Transparent Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label:
                        "Neo Hdr Bloom Ssao Taa Fxaa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass"
                            .to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
            ]
        );
    }

    #[test]
    fn wgpu_metrics_count_native_pass_instances_separately_from_draw_calls() {
        let stats = frame_stats_from_wgpu_metrics(
            31,
            vec![
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Spot Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned(),
            ],
            RenderQueueStats::default(),
            MeshRenderStats {
                draw_call_count: 29,
                directional_shadow_draw_call_count: 4,
                spot_shadow_draw_call_count: 2,
                point_shadow_draw_call_count: 12,
                depth_prepass_draw_call_count: 2,
                gbuffer_draw_call_count: 2,
                deferred_lighting_draw_call_count: 1,
                opaque_draw_call_count: 3,
                transparent_draw_call_count: 1,
                skybox_draw_call_count: 1,
                post_process_draw_call_count: 1,
                native_pass_labels_dropped: 5,
                ..MeshRenderStats::default()
            },
            33,
            3,
            ResourceReclaimPolicy::FrameLatency { frames: 2 },
            false,
        );

        assert_eq!(stats.graph.rhi_executed_pass_labels.len(), 15);
        assert_eq!(stats.graph.rhi_executed_passes, 20);
        assert_eq!(stats.graph.pass_count, 20);
        assert_eq!(stats.graph.fullscreen_draws, 2);
        assert_eq!(stats.backend_directional_shadow_passes, 2);
        assert_eq!(stats.backend_spot_shadow_passes, 1);
        assert_eq!(stats.backend_point_shadow_passes, 6);
        assert_eq!(stats.backend_gbuffer_passes, 1);
        assert_eq!(stats.backend_deferred_lighting_passes, 1);
        assert_eq!(stats.backend_depth_prepass_passes, 1);
        assert_eq!(stats.backend_forward_opaque_passes, 1);
        assert_eq!(stats.backend_transparent_passes, 1);
        assert_eq!(stats.backend_post_process_passes, 1);
        assert_eq!(
            stats.backend_native_pass_label_capacity,
            MeshRenderStats::native_pass_label_capacity() as u32
        );
        assert_eq!(stats.backend_native_pass_labels_dropped, 5);
        assert_eq!(stats.backend_directional_shadow_draw_calls, 4);
        assert_eq!(stats.backend_spot_shadow_draw_calls, 2);
        assert_eq!(stats.backend_point_shadow_draw_calls, 12);
        assert_eq!(stats.backend_depth_prepass_draw_calls, 2);
        assert_eq!(stats.backend_gbuffer_draw_calls, 2);
        assert_eq!(stats.backend_deferred_lighting_draw_calls, 1);
        assert_eq!(stats.backend_opaque_draw_calls, 3);
        assert_eq!(stats.backend_transparent_draw_calls, 1);
        assert_eq!(stats.backend_post_process_draw_calls, 1);
        assert_eq!(stats.backend_post_process_draw_calls, 1);
        assert_eq!(stats.backend_post_pass_draw_calls, 0);
    }

    #[test]
    fn wgpu_metrics_prefer_actual_native_pass_labels_over_default_estimate() {
        let scene = RenderScene::new(engine_render::PerspectiveCamera::default());
        let mut gpu_stats = MeshRenderStats::default();
        gpu_stats.record_native_pass_label("Neo Forward Opaque Pass");
        gpu_stats.record_native_pass_label("Neo Transparent Pass");
        gpu_stats.record_native_pass_label("Neo Post Process Pass");
        let labels = native_pass_labels_from_wgpu_metrics(&scene, 1, &gpu_stats);

        assert_eq!(
            labels,
            vec![
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Post Process Pass".to_owned(),
            ]
        );

        let fallback = native_pass_labels_from_wgpu_metrics(&scene, 1, &MeshRenderStats::default());
        assert_eq!(
            fallback,
            vec![
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned(),
            ]
        );
    }

    #[test]
    fn native_post_pass_draws_are_counted_in_post_process_native_pass_stats() {
        let mut stats = frame_stats_from_wgpu_metrics(
            7,
            vec![
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Post Process Pass".to_owned(),
            ],
            RenderQueueStats::default(),
            MeshRenderStats {
                draw_call_count: 3,
                opaque_draw_call_count: 1,
                transparent_draw_call_count: 1,
                post_process_draw_call_count: 1,
                ..MeshRenderStats::default()
            },
            33,
            3,
            ResourceReclaimPolicy::FrameLatency { frames: 2 },
            false,
        );

        record_native_post_pass_draws(&mut stats, 3);

        assert_eq!(stats.draw_calls, 6);
        assert_eq!(stats.backend_forward_opaque_passes, 1);
        assert_eq!(stats.backend_transparent_passes, 1);
        assert_eq!(stats.backend_post_process_passes, 1);
        assert_eq!(
            stats.backend_native_pass_label_capacity,
            MeshRenderStats::native_pass_label_capacity() as u32
        );
        assert_eq!(stats.backend_native_pass_labels_dropped, 0);
        assert_eq!(stats.backend_post_process_draw_calls, 1);
        assert_eq!(stats.backend_post_pass_draw_calls, 3);
        assert_eq!(
            stats.backend_native_pass_draws,
            vec![
                BackendNativePassDrawStats {
                    pass_label: "Neo Forward Opaque Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Transparent Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 1,
                },
                BackendNativePassDrawStats {
                    pass_label: "Neo Post Process Pass".to_owned(),
                    pass_instances: 1,
                    draw_calls: 4,
                },
            ]
        );
    }

    #[test]
    fn wgpu_metrics_hide_gpu_timestamps_when_profiler_is_disabled() {
        let gpu_stats = MeshRenderStats {
            timestamp_writes: 2,
            gpu_time_ns: Some(3_250_000),
            ..MeshRenderStats::default()
        };

        let stats = frame_stats_from_wgpu_metrics(
            12,
            vec![
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Post Process Pass".to_owned(),
            ],
            RenderQueueStats::default(),
            gpu_stats,
            33,
            3,
            ResourceReclaimPolicy::FrameLatency { frames: 2 },
            false,
        );

        assert!(!stats.gpu_profiler_enabled);
        assert_eq!(stats.gpu_time_ms, None);
        assert_eq!(stats.graph.gpu_time_ns, None);
        assert_eq!(stats.graph.timestamp_queries, 0);
        assert_eq!(stats.graph.timestamp_writes, 0);
        assert_eq!(stats.graph.pass_count, 3);
        assert_eq!(stats.graph.rhi_executed_passes, 3);
        assert_eq!(
            stats.graph.rhi_executed_pass_labels,
            vec![
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Post Process Pass".to_owned(),
            ]
        );
        assert_eq!(stats.pipeline_cache.backend_objects, 33);
        assert_eq!(stats.pipeline_cache.shader_interface_layouts, 3);
    }

    #[test]
    fn wgpu_frame_debug_report_preserves_native_backend_stats() {
        let stats = frame_stats_from_wgpu_metrics(
            21,
            vec![
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned(),
            ],
            RenderQueueStats {
                item_count: 3,
                culled_item_count: 1,
                ..RenderQueueStats::default()
            },
            MeshRenderStats {
                draw_call_count: 10,
                mesh_pass_draw_call_count: 1,
                skybox_draw_call_count: 1,
                gbuffer_draw_call_count: 2,
                deferred_lighting_draw_call_count: 1,
                depth_prepass_draw_call_count: 1,
                shadow_draw_call_count: 3,
                directional_shadow_draw_call_count: 1,
                spot_shadow_draw_call_count: 1,
                point_shadow_draw_call_count: 1,
                opaque_draw_call_count: 1,
                transparent_draw_call_count: 0,
                post_process_draw_call_count: 1,
                instance_buffer_capacity: 4,
                timestamp_writes: 2,
                gpu_time_ns: Some(1_500_000),
                ..MeshRenderStats::default()
            },
            33,
            3,
            ResourceReclaimPolicy::FrameLatency { frames: 2 },
            true,
        );

        let report = crate::FrameDebugReport::from_stats(&stats);

        assert_eq!(report.frame_index, 21);
        assert_eq!(report.graph_passes, 7);
        assert_eq!(report.rhi_executed_graph_passes, 7);
        assert_eq!(
            report.rhi_executed_pass_labels,
            vec![
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned()
            ]
        );
        assert!(report.gpu_profiler_enabled);
        assert_eq!(report.gpu_time_ms, Some(1.5));
        assert_eq!(report.draw_calls, 10);
        assert_eq!(report.backend_mesh_pass_draw_calls, 1);
        assert_eq!(report.backend_skybox_draw_calls, 1);
        assert_eq!(report.backend_gbuffer_draw_calls, 2);
        assert_eq!(report.backend_deferred_lighting_draw_calls, 1);
        assert_eq!(report.backend_depth_prepass_draw_calls, 1);
        assert_eq!(report.backend_shadow_draw_calls, 3);
        assert_eq!(report.backend_directional_shadow_draw_calls, 1);
        assert_eq!(report.backend_spot_shadow_draw_calls, 1);
        assert_eq!(report.backend_point_shadow_draw_calls, 1);
        assert_eq!(report.backend_opaque_draw_calls, 1);
        assert_eq!(report.backend_transparent_draw_calls, 0);
        assert_eq!(report.backend_directional_shadow_passes, 1);
        assert_eq!(report.backend_spot_shadow_passes, 0);
        assert_eq!(report.backend_point_shadow_passes, 0);
        assert_eq!(report.backend_gbuffer_passes, 1);
        assert_eq!(report.backend_deferred_lighting_passes, 1);
        assert_eq!(report.backend_depth_prepass_passes, 1);
        assert_eq!(report.backend_forward_opaque_passes, 1);
        assert_eq!(report.backend_transparent_passes, 1);
        assert_eq!(report.backend_post_process_passes, 1);
        assert_eq!(
            report.backend_native_pass_label_capacity,
            MeshRenderStats::native_pass_label_capacity() as u32
        );
        assert_eq!(report.backend_native_pass_labels_dropped, 0);
        assert_eq!(report.backend_post_process_draw_calls, 1);
        assert_eq!(report.backend_post_pass_draw_calls, 0);
        assert_eq!(
            report.backend_native_pass_draws,
            stats.backend_native_pass_draws
        );
        assert_eq!(report.visible_objects, 3);
        assert_eq!(report.culled_objects, 1);
        assert_eq!(report.pipeline_cache.backend_objects, 33);
        assert_eq!(report.pipeline_shader_interface_layouts, 3);
        assert_eq!(
            report.memory.reclaim_policy,
            ResourceReclaimPolicy::FrameLatency { frames: 2 }
        );
    }

    #[test]
    fn default_wgpu_pass_labels_match_native_render_pass_order() {
        let mut scene = RenderScene::new(engine_render::PerspectiveCamera::default());
        scene.set_lighting(
            engine_render::RenderLighting::default()
                .with_directional_shadow(
                    engine_render::DirectionalShadow::enabled(1024, 8.0, -8.0, 8.0, 0.7, 0.002)
                        .with_cascades(2, 20.0, 0.5),
                )
                .with_spot_lights(&[engine_render::SpotLight::new(
                    [0.0, 4.0, 0.0],
                    [0.0, -1.0, 0.0],
                    [1.0, 1.0, 1.0],
                    1.0,
                    10.0,
                    0.25,
                    0.75,
                )
                .with_shadow(engine_render::SpotShadow::enabled(
                    512, 0.05, 10.0, 0.5, 0.001,
                ))])
                .with_point_lights(&[engine_render::PointLight::new(
                    [1.0, 2.0, 3.0],
                    [1.0, 0.9, 0.8],
                    1.0,
                    8.0,
                )
                .with_shadow(engine_render::PointShadow::enabled(
                    256, 0.05, 8.0, 0.5, 0.001,
                ))]),
        );

        assert_eq!(
            default_wgpu_pass_labels(&scene, 1),
            vec![
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Directional Shadow Pass".to_owned(),
                "Neo Spot Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Point Shadow Pass".to_owned(),
                "Neo Depth Prepass".to_owned(),
                "Neo GBuffer Pass".to_owned(),
                "Neo Deferred Lighting Pass".to_owned(),
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Tonemap Post Process Pass".to_owned(),
            ]
        );
        assert_eq!(
            default_wgpu_pass_labels(&scene, 0),
            vec![
                "Neo Forward Opaque Pass".to_owned(),
                "Neo Transparent Pass".to_owned(),
                "Neo Post Process Pass".to_owned(),
            ]
        );
    }

    #[test]
    fn backend_errors_map_to_renderer_status_errors() {
        assert!(matches!(
            map_backend_error(GraphicsError::SurfaceOutOfMemory),
            RendererError::OutOfMemory(_)
        ));
        assert!(matches!(
            map_backend_error(GraphicsError::Backend("surface was lost".to_owned())),
            RendererError::DeviceLost { .. }
        ));

        let mut status = DeviceStatus::Ok;
        let error = record_backend_error(
            &mut status,
            GraphicsError::Backend("device was lost".to_owned()),
        );
        assert!(matches!(error, RendererError::DeviceLost { .. }));
        assert_eq!(status, DeviceStatus::Lost);
    }

    #[test]
    fn wgpu_limits_map_to_renderer_limits() {
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 4096,
            max_texture_array_layers: 128,
            max_bind_groups: 6,
            max_vertex_buffers: 12,
            ..wgpu::Limits::downlevel_defaults()
        };

        assert_eq!(
            renderer_limits_from_wgpu(&limits),
            RendererLimits {
                max_texture_dimension_2d: 4096,
                max_texture_array_layers: 128,
                max_bind_groups: 6,
                max_vertex_buffers: 12,
            }
        );
    }

    #[test]
    fn wgpu_format_support_requires_render_attachment_usage() {
        assert!(format_supports_render_attachment(
            wgpu::TextureFormatFeatures {
                allowed_usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
                flags: wgpu::TextureFormatFeatureFlags::empty(),
            }
        ));
        assert!(!format_supports_render_attachment(
            wgpu::TextureFormatFeatures {
                allowed_usages: wgpu::TextureUsages::TEXTURE_BINDING,
                flags: wgpu::TextureFormatFeatureFlags::empty(),
            }
        ));
    }
}
