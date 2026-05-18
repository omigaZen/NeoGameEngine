#[cfg(feature = "backend-wgpu")]
pub mod backend_wgpu;
pub mod graph;
pub mod rhi;

use std::{
    collections::{HashMap, HashSet},
    fmt, fs, hash,
    marker::PhantomData,
    num::NonZeroU64,
    ops::Range,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use engine_graphics::Color;

#[cfg(feature = "backend-wgpu")]
pub use backend_wgpu::WgpuRendererRuntime;
pub use graph::{
    AliasAllocation, BufferReadUsage, BufferWriteUsage, ColorAttachmentOps, CompiledPass,
    CompiledRenderGraph, CompiledResourceAccess, ComputePassDesc, ComputePassEncoder,
    CustomPostProcessInfo, DepthAttachmentOps, GraphAccess, GraphBuffer, GraphBufferDesc,
    GraphBufferUsage, GraphPipelineRef, GraphResource, GraphTexture, GraphTextureDesc,
    GraphTextureUsage, PassBuilder, PassContext, PassId, QueueType, RenderGraphBuilder,
    RenderGraphExtension, RenderGraphExtensionContext, RenderGraphStats, RenderPassDesc,
    RenderPassEncoder, RenderPassNode, ResourceBarrier, ResourceLifetime, RhiResourceImports,
    TextureReadUsage, TextureWriteUsage, ViewInfo,
};
pub use rhi::{
    HeadlessRhiDevice, HeadlessRhiStats, PollMode, RhiAccess, RhiBuffer, RhiBufferDesc, RhiCaps,
    RhiCommandBuffer, RhiCommandEncoder, RhiCompareFunction, RhiComputePassDesc,
    RhiComputePipeline, RhiComputePipelineDesc, RhiDepthState, RhiDevice, RhiError, RhiFace,
    RhiGraphicsPipeline, RhiGraphicsPipelineDesc, RhiIndexedIndirectRenderPassDesc,
    RhiIndirectRenderPassDesc, RhiOcclusionQuery, RhiOcclusionQueryDesc, RhiOcclusionQueryResult,
    RhiPipelineStatistics, RhiPipelineStatisticsQuery, RhiPipelineStatisticsQueryDesc,
    RhiPipelineStatisticsResult, RhiPrimitiveState, RhiPrimitiveTopology, RhiRenderPassDesc,
    RhiSampler, RhiSamplerDesc, RhiShaderModule, RhiShaderModuleDesc, RhiTexture, RhiTextureDesc,
    RhiTextureRegion, RhiTimestampQuery, RhiTimestampQueryDesc, RhiTimestampResult,
    RhiVertexAttribute, RhiVertexBufferLayout, SubmissionIndex,
};
#[cfg(feature = "backend-wgpu")]
pub use rhi::{WgpuRhiDevice, WgpuRhiStats};

pub mod prelude {
    pub use crate::{
        encode_gpu_picking_object_index, AddressMode, AlphaMode, AnimationDataUsage, AreaLightDesc,
        AreaLightShape, BackendPreference, Bounds3, BufferDesc, BufferHandle, BufferInfo,
        BufferUpdate, BufferUsage, CameraDesc, CameraFlags, CameraHandle, CaptureOptions,
        ClearOptions, ColorGradingMode, CompareFunc, CustomLightDesc, CustomPostProcessDesc,
        CustomPostProcessPass, DebugDraw, DebugDrawCommand, DepthFormat, DeviceStatus,
        DirectionalLightDesc, DirectionalShadowDesc, EnvironmentBakeDesc, EnvironmentDesc,
        EnvironmentHandle, Exposure, ExtractRenderData, FilterMode, FormatCaps, Frame,
        FrameCapture, FrameCaptureBackend, FrameCaptureResourceDump, FrameCaptureStatus,
        FrameCullingOutput, FrameDebugDrawOutput, FrameDeformationOutput, FrameEnvironmentOutput,
        FrameGBufferOutput, FrameInput, FramePipelineStatistics, FrameProfile, FrameStats, Handle,
        IndexData, LightDesc, LightHandle, LightUpdate, LodGroupDesc, LodGroupHandle, LodLevelDesc,
        MaterialDesc, MaterialDomain, MaterialHandle, MaterialOverrides, MaterialParamId,
        MaterialParameter, MaterialParameterSchema, MaterialParameterValue, MaterialParameters,
        MaterialPassFlags, MaterialTemplateDesc, MaterialTemplateHandle, MaterialUpdate,
        MaterialValue, MeshDesc, MeshFlags, MeshHandle, MeshInfo, MeshUsage, MorphWeightsDesc,
        MorphWeightsHandle, ObjectFlags, ObjectHandle, OrderedF32, OutlinePass, PhaseSortMode,
        PickingHandle, PickingRequest, PickingResult, PickingResultSource, PickingTicket,
        PipelineCacheStats, PipelineWarmupRequest, PointLightDesc, PointShadowDesc, Projection,
        Quat, RectU, RenderGraphExtensionHandle, RenderLayer, RenderLayerMask, RenderObjectDesc,
        RenderPath, RenderPhaseId, RenderPhaseKind, RenderStateDesc, RenderTarget,
        RenderTargetDesc, RenderTargetHandle, Renderer, RendererCaps, RendererConfig,
        RendererError, RendererFeature, RendererFeatures, RendererLimits, ResidencyPriority,
        ResourceKind, ResourceStatus, SamplerDesc, SamplerHandle, SceneCommand, SceneCommandBuffer,
        SceneDesc, SceneHandle, SceneWriter, ShaderDesc, ShaderEntryPointInfo, ShaderEntryPoints,
        ShaderFeatureSet, ShaderHandle, ShaderInfo, ShaderReflectionMode, ShaderReloadDesc,
        ShaderSource, ShaderStages, SkeletonInstanceDesc, SkeletonInstanceHandle,
        SkeletonInstanceInfo, SpotLightDesc, SpotShadowDesc, StandardMaterialDesc, SubMeshDesc,
        SurfaceHandle, TextureDesc, TextureDimension, TextureFormat, TextureHandle, TextureInfo,
        TextureInitialData, TextureRegion, TextureSubresource, TextureUpdate, TextureUsage,
        TextureViewDesc, UVec2, UploadStats, VSyncMode, ValidationMode, Vec2, Vec3, Vec4,
        VertexAttribute, VertexData, VertexFormat, VertexLayout, VertexSemantic, VertexStepMode,
        VertexStream, VertexStreamLayout, ViewDesc, ViewHandle, ViewQualitySettings, Viewport,
        VisibilityFlags, IDENTITY_MAT4,
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UVec2 {
    pub x: u32,
    pub y: u32,
}

impl UVec2 {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

pub type Mat4 = [[f32; 4]; 4];

pub const IDENTITY_MAT4: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

#[repr(transparent)]
pub struct Handle<T> {
    raw: NonZeroU64,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub fn from_raw(raw: NonZeroU64) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    pub fn raw(self) -> NonZeroU64 {
        self.raw
    }

    pub fn index(self) -> u32 {
        (self.raw.get() & 0xffff_ffff) as u32
    }

    pub fn generation(self) -> u32 {
        ((self.raw.get() >> 32) & 0x00ff_ffff) as u32
    }

    pub fn kind_tag(self) -> u8 {
        (self.raw.get() >> 56) as u8
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Handle<T> {}

impl<T> hash::Hash for Handle<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("raw", &self.raw)
            .field("index", &self.index())
            .field("generation", &self.generation())
            .field("kind_tag", &self.kind_tag())
            .finish()
    }
}

pub enum MeshTag {}
pub enum BufferTag {}
pub enum TextureTag {}
pub enum SurfaceTag {}
pub enum MaterialTag {}
pub enum MaterialTemplateTag {}
pub enum ShaderTag {}
pub enum SceneTag {}
pub enum ObjectTag {}
pub enum CameraTag {}
pub enum LightTag {}
pub enum EnvironmentTag {}
pub enum RenderTargetTag {}
pub enum SamplerTag {}
pub enum ViewTag {}
pub enum SkeletonInstanceTag {}
pub enum MorphWeightsTag {}
pub enum LodGroupTag {}
pub enum RenderGraphExtensionTag {}
pub enum PickingTag {}

pub type MeshHandle = Handle<MeshTag>;
pub type BufferHandle = Handle<BufferTag>;
pub type TextureHandle = Handle<TextureTag>;
pub type SurfaceHandle = Handle<SurfaceTag>;
pub type MaterialHandle = Handle<MaterialTag>;
pub type MaterialTemplateHandle = Handle<MaterialTemplateTag>;
pub type ShaderHandle = Handle<ShaderTag>;
pub type SceneHandle = Handle<SceneTag>;
pub type ObjectHandle = Handle<ObjectTag>;
pub type CameraHandle = Handle<CameraTag>;
pub type LightHandle = Handle<LightTag>;
pub type EnvironmentHandle = Handle<EnvironmentTag>;
pub type RenderTargetHandle = Handle<RenderTargetTag>;
pub type SamplerHandle = Handle<SamplerTag>;
pub type ViewHandle = Handle<ViewTag>;
pub type SkeletonInstanceHandle = Handle<SkeletonInstanceTag>;
pub type MorphWeightsHandle = Handle<MorphWeightsTag>;
pub type LodGroupHandle = Handle<LodGroupTag>;
pub type RenderGraphExtensionHandle = Handle<RenderGraphExtensionTag>;
pub type PickingHandle = Handle<PickingTag>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    Mesh,
    Buffer,
    Texture,
    Surface,
    Material,
    MaterialTemplate,
    Shader,
    Scene,
    Object,
    Camera,
    Light,
    Environment,
    RenderTarget,
    Sampler,
    View,
    SkeletonInstance,
    MorphWeights,
    LodGroup,
    RenderGraphExtension,
    Picking,
}

impl ResourceKind {
    const fn tag(self) -> u8 {
        match self {
            Self::Mesh => 1,
            Self::Texture => 2,
            Self::Material => 3,
            Self::MaterialTemplate => 4,
            Self::Shader => 5,
            Self::Scene => 6,
            Self::Object => 7,
            Self::Camera => 8,
            Self::Light => 9,
            Self::Environment => 10,
            Self::RenderTarget => 11,
            Self::Sampler => 12,
            Self::View => 13,
            Self::SkeletonInstance => 14,
            Self::MorphWeights => 15,
            Self::LodGroup => 16,
            Self::RenderGraphExtension => 17,
            Self::Picking => 18,
            Self::Buffer => 19,
            Self::Surface => 20,
        }
    }

    const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Mesh),
            2 => Some(Self::Texture),
            3 => Some(Self::Material),
            4 => Some(Self::MaterialTemplate),
            5 => Some(Self::Shader),
            6 => Some(Self::Scene),
            7 => Some(Self::Object),
            8 => Some(Self::Camera),
            9 => Some(Self::Light),
            10 => Some(Self::Environment),
            11 => Some(Self::RenderTarget),
            12 => Some(Self::Sampler),
            13 => Some(Self::View),
            14 => Some(Self::SkeletonInstance),
            15 => Some(Self::MorphWeights),
            16 => Some(Self::LodGroup),
            17 => Some(Self::RenderGraphExtension),
            18 => Some(Self::Picking),
            19 => Some(Self::Buffer),
            20 => Some(Self::Surface),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceStatus {
    PendingUpload,
    Ready,
    Failed,
    Evicted,
    DestroyQueued,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RendererError {
    Backend(String),
    DeviceLost { reason: String },
    OutOfMemory(String),
    InvalidHandle { kind: ResourceKind, raw: u64 },
    ResourceNotReady(ResourceKind),
    UnsupportedFeature(RendererFeature),
    ShaderCompile(String),
    PipelineCompile(String),
    MaterialParameterMismatch(String),
    RenderGraphValidation(String),
    Validation(String),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(value) => write!(f, "backend error: {value}"),
            Self::DeviceLost { reason } => write!(f, "device lost: {reason}"),
            Self::OutOfMemory(value) => write!(f, "out of memory: {value}"),
            Self::InvalidHandle { kind, raw } => {
                write!(f, "invalid handle: kind={kind:?}, raw={raw}")
            }
            Self::ResourceNotReady(kind) => write!(f, "resource is not ready: {kind:?}"),
            Self::UnsupportedFeature(feature) => write!(f, "unsupported feature: {feature:?}"),
            Self::ShaderCompile(value) => write!(f, "shader compile error: {value}"),
            Self::PipelineCompile(value) => write!(f, "pipeline compile error: {value}"),
            Self::MaterialParameterMismatch(value) => {
                write!(f, "material parameter mismatch: {value}")
            }
            Self::RenderGraphValidation(value) => {
                write!(f, "render graph validation error: {value}")
            }
            Self::Validation(value) => write!(f, "validation error: {value}"),
        }
    }
}

impl std::error::Error for RendererError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendPreference {
    Auto,
    Wgpu,
    Vulkan,
    Metal,
    D3d12,
    Headless,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationMode {
    Off,
    Basic,
    Full,
    GpuAssisted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VSyncMode {
    Off,
    On,
    Adaptive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderPath {
    Auto,
    Deferred,
    ForwardPlus,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DepthFormat {
    D16Unorm,
    D24Plus,
    D24PlusStencil8,
    D32Float,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererConfig {
    pub backend: BackendPreference,
    pub validation: ValidationMode,
    pub frame_latency: u32,
    pub surface_format: Option<TextureFormat>,
    pub depth_format: DepthFormat,
    pub msaa_samples: u32,
    pub vsync: VSyncMode,
    pub hdr: bool,
    pub preferred_render_path: RenderPath,
    pub shader_hot_reload: bool,
    pub transient_resource_aliasing: bool,
    pub gpu_profiling: bool,
    pub debug_labels: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            backend: BackendPreference::Headless,
            validation: ValidationMode::Basic,
            frame_latency: 2,
            surface_format: None,
            depth_format: DepthFormat::D32Float,
            msaa_samples: 1,
            vsync: VSyncMode::Adaptive,
            hdr: true,
            preferred_render_path: RenderPath::ForwardPlus,
            shader_hot_reload: cfg!(debug_assertions),
            transient_resource_aliasing: true,
            gpu_profiling: cfg!(debug_assertions),
            debug_labels: cfg!(debug_assertions),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RendererFeatures(pub u64);

impl RendererFeatures {
    pub const COMPUTE: Self = Self(1 << 0);
    pub const INDIRECT_DRAW: Self = Self(1 << 1);
    pub const MULTI_DRAW_INDIRECT: Self = Self(1 << 2);
    pub const BINDLESS_TEXTURES: Self = Self(1 << 3);
    pub const STORAGE_TEXTURES: Self = Self(1 << 4);
    pub const TIMESTAMP_QUERY: Self = Self(1 << 5);
    pub const PIPELINE_STATISTICS: Self = Self(1 << 6);
    pub const ASYNC_COMPUTE: Self = Self(1 << 7);
    pub const RAY_TRACING: Self = Self(1 << 8);
    pub const MESH_SHADER: Self = Self(1 << 9);
    pub const VARIABLE_RATE_SHADING: Self = Self(1 << 10);
    pub const GPU_DRIVEN_RENDERING: Self = Self(1 << 11);
    pub const OCCLUSION_CULLING: Self = Self(1 << 12);
    pub const VIRTUAL_TEXTURING: Self = Self(1 << 13);

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

impl std::ops::BitOr for RendererFeatures {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererFeature {
    BackendWgpu,
    BackendVulkan,
    BackendMetal,
    BackendD3d12,
    Compute,
    IndirectDraw,
    MultiDrawIndirect,
    BindlessTextures,
    StorageTextures,
    TimestampQuery,
    PipelineStatistics,
    AsyncCompute,
    RayTracing,
    MeshShader,
    VariableRateShading,
    GpuDrivenRendering,
    OcclusionCulling,
    VirtualTexturing,
    ShaderReflection,
    Surface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererCaps {
    pub backend_name: String,
    pub adapter_name: String,
    pub features: RendererFeatures,
    pub limits: RendererLimits,
    pub formats: FormatCaps,
}

impl Default for RendererCaps {
    fn default() -> Self {
        Self::for_backend(&RendererConfig::default(), "headless", "retained-api")
    }
}

impl RendererCaps {
    fn for_backend(config: &RendererConfig, backend_name: &str, adapter_name: &str) -> Self {
        let mut features = RendererFeatures::COMPUTE
            | RendererFeatures::INDIRECT_DRAW
            | RendererFeatures::STORAGE_TEXTURES
            | RendererFeatures::OCCLUSION_CULLING;
        if config.gpu_profiling {
            features = features | RendererFeatures::TIMESTAMP_QUERY;
        }
        if cfg!(feature = "multi-draw-indirect") {
            features = features | RendererFeatures::MULTI_DRAW_INDIRECT;
        }
        if cfg!(feature = "pipeline-statistics") {
            features = features | RendererFeatures::PIPELINE_STATISTICS;
        }
        if cfg!(feature = "async-compute") {
            features = features | RendererFeatures::ASYNC_COMPUTE;
        }
        if cfg!(feature = "bindless") {
            features = features | RendererFeatures::BINDLESS_TEXTURES;
        }
        if cfg!(feature = "ray-tracing") {
            features = features | RendererFeatures::RAY_TRACING;
        }
        if cfg!(feature = "mesh-shader") {
            features = features | RendererFeatures::MESH_SHADER;
        }
        if cfg!(feature = "variable-rate-shading") {
            features = features | RendererFeatures::VARIABLE_RATE_SHADING;
        }
        if cfg!(feature = "gpu-driven") {
            features = features | RendererFeatures::GPU_DRIVEN_RENDERING;
        }
        if cfg!(feature = "virtual-texturing") {
            features = features | RendererFeatures::VIRTUAL_TEXTURING;
        }

        Self {
            backend_name: backend_name.to_owned(),
            adapter_name: adapter_name.to_owned(),
            features,
            limits: RendererLimits::default(),
            formats: FormatCaps::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererLimits {
    pub max_texture_dimension_2d: u32,
    pub max_texture_array_layers: u32,
    pub max_bind_groups: u32,
    pub max_vertex_buffers: u32,
}

impl Default for RendererLimits {
    fn default() -> Self {
        Self {
            max_texture_dimension_2d: 16_384,
            max_texture_array_layers: 2_048,
            max_bind_groups: 8,
            max_vertex_buffers: 16,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatCaps {
    pub color: Vec<TextureFormat>,
    pub depth: Vec<DepthFormat>,
}

impl Default for FormatCaps {
    fn default() -> Self {
        Self {
            color: vec![
                TextureFormat::Rgba8Unorm,
                TextureFormat::Rgba8UnormSrgb,
                TextureFormat::Rgba16Float,
                TextureFormat::Bgra8UnormSrgb,
            ],
            depth: vec![DepthFormat::D24Plus, DepthFormat::D32Float],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceStatus {
    Ok,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds3 {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn center(self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshDesc<'a> {
    pub label: Option<&'a str>,
    pub vertex_layout: VertexLayout,
    pub vertices: VertexData<'a>,
    pub indices: Option<IndexData<'a>>,
    pub submeshes: Vec<SubMeshDesc>,
    pub bounds: Bounds3,
    pub usage: MeshUsage,
    pub flags: MeshFlags,
    pub skin: Option<SkinDesc<'a>>,
    pub morph_targets: Vec<MorphTargetDesc<'a>>,
    pub meshlets: Option<MeshletData<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VertexData<'a> {
    Interleaved(&'a [u8]),
    Streams(Vec<VertexStream<'a>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct VertexStream<'a> {
    pub data: &'a [u8],
    pub stride: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IndexData<'a> {
    U16(&'a [u16]),
    U32(&'a [u32]),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VertexLayout {
    pub streams: Vec<VertexStreamLayout>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexStreamLayout {
    pub stride: u32,
    pub step: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexStepMode {
    Vertex,
    Instance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexAttribute {
    pub semantic: VertexSemantic,
    pub format: VertexFormat,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexSemantic {
    Position,
    Normal,
    Tangent,
    Bitangent,
    TexCoord(u8),
    Color(u8),
    Joints(u8),
    Weights(u8),
    Custom(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
    Uint16x2,
    Uint16x4,
    Uint32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubMeshDesc {
    pub index_range: Range<u32>,
    pub vertex_range: Range<u32>,
    pub material_slot: u16,
    pub bounds: Bounds3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshUsage(pub u32);

impl MeshUsage {
    pub const STATIC: Self = Self(1 << 0);
    pub const DYNAMIC: Self = Self(1 << 1);
    pub const STREAMING: Self = Self(1 << 2);
    pub const CPU_READBACK: Self = Self(1 << 3);
    pub const RAY_TRACING: Self = Self(1 << 4);

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

impl std::ops::BitOr for MeshUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MeshFlags(pub u32);

impl MeshFlags {
    pub const NONE: Self = Self(0);
    pub const ENABLE_SKINNING: Self = Self(1 << 0);
    pub const ENABLE_MORPH_TARGETS: Self = Self(1 << 1);
    pub const HAS_MESHLETS: Self = Self(1 << 2);
    pub const GPU_CULLABLE: Self = Self(1 << 3);
    pub const NO_MERGE: Self = Self(1 << 4);

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

impl std::ops::BitOr for MeshFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferDesc<'a> {
    pub label: Option<&'a str>,
    pub size: u64,
    pub usage: BufferUsage,
    pub initial_data: Option<&'a [u8]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufferUsage(pub u32);

impl BufferUsage {
    pub const UNIFORM: Self = Self(1 << 0);
    pub const STORAGE: Self = Self(1 << 1);
    pub const VERTEX: Self = Self(1 << 2);
    pub const INDEX: Self = Self(1 << 3);
    pub const COPY_SRC: Self = Self(1 << 4);
    pub const COPY_DST: Self = Self(1 << 5);
    pub const MAP_READ: Self = Self(1 << 6);
    pub const MAP_WRITE: Self = Self(1 << 7);

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

impl std::ops::BitOr for BufferUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferInfo {
    pub label: Option<String>,
    pub size: u64,
    pub usage: BufferUsage,
    pub status: ResourceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BufferUpdate<'a> {
    pub byte_offset: u64,
    pub data: &'a [u8],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkinDesc<'a> {
    pub inverse_bind_matrices: &'a [Mat4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphTargetDesc<'a> {
    pub positions: Option<&'a [Vec3]>,
    pub normals: Option<&'a [Vec3]>,
    pub tangents: Option<&'a [Vec3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonInstanceDesc<'a> {
    pub label: Option<&'a str>,
    pub joint_matrices: &'a [Mat4],
    pub inverse_bind_matrices: Option<&'a [Mat4]>,
    pub usage: AnimationDataUsage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphWeightsDesc<'a> {
    pub label: Option<&'a str>,
    pub weights: &'a [f32],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnimationDataUsage {
    #[default]
    Dynamic,
    Static,
    Streaming,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonInstanceInfo {
    pub label: Option<String>,
    pub joint_count: usize,
    pub inverse_bind_count: usize,
    pub usage: AnimationDataUsage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshletData<'a> {
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshInfo {
    pub label: Option<String>,
    pub vertex_bytes: usize,
    pub index_count: u32,
    pub submesh_count: usize,
    pub skin_joint_count: usize,
    pub morph_target_count: usize,
    pub meshlet_bytes: usize,
    pub bounds: Bounds3,
    pub usage: MeshUsage,
    pub flags: MeshFlags,
    pub status: ResourceStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
    D2Array,
    CubeArray,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Bgra8UnormSrgb,
    Rgba16Float,
    Rgba32Float,
    Depth32Float,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextureUsage(pub u32);

impl TextureUsage {
    pub const SAMPLED: Self = Self(1 << 0);
    pub const RENDER_TARGET: Self = Self(1 << 1);
    pub const DEPTH_STENCIL: Self = Self(1 << 2);
    pub const STORAGE: Self = Self(1 << 3);
    pub const COPY_SRC: Self = Self(1 << 4);
    pub const COPY_DST: Self = Self(1 << 5);
    pub const PRESENT: Self = Self(1 << 6);

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

impl std::ops::BitOr for TextureUsage {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureDesc<'a> {
    pub label: Option<&'a str>,
    pub dimension: TextureDimension,
    pub width: u32,
    pub height: u32,
    pub depth_or_layers: u32,
    pub mip_levels: u32,
    pub samples: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub initial_data: Option<TextureInitialData<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureInitialData<'a> {
    pub bytes: &'a [u8],
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureInfo {
    pub label: Option<String>,
    pub dimension: TextureDimension,
    pub width: u32,
    pub height: u32,
    pub depth_or_layers: u32,
    pub mip_levels: u32,
    pub samples: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub status: ResourceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextureUpdate<'a> {
    pub subresource: TextureSubresource,
    pub region: TextureRegion,
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
    pub data: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureSubresource {
    pub mip_level: u32,
    pub array_layer: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureRegion {
    pub offset: [u32; 3],
    pub extent: [u32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SamplerDesc {
    pub address_u: AddressMode,
    pub address_v: AddressMode,
    pub address_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mip_filter: FilterMode,
    pub compare: Option<CompareFunc>,
    pub anisotropy: u8,
    pub lod_min: OrderedF32,
    pub lod_max: OrderedF32,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            address_u: AddressMode::ClampToEdge,
            address_v: AddressMode::ClampToEdge,
            address_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mip_filter: FilterMode::Linear,
            compare: None,
            anisotropy: 1,
            lod_min: OrderedF32::new(0.0),
            lod_max: OrderedF32::new(f32::MAX),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OrderedF32(u32);

impl OrderedF32 {
    pub fn new(value: f32) -> Self {
        Self(value.to_bits())
    }

    pub fn get(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AddressMode {
    ClampToEdge,
    Repeat,
    MirrorRepeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterMode {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompareFunc {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShaderDesc<'a> {
    pub label: Option<&'a str>,
    pub source: ShaderSource<'a>,
    pub stages: ShaderStages,
    pub entry_points: ShaderEntryPoints<'a>,
    pub reflection: ShaderReflectionMode,
    pub features: ShaderFeatureSet,
    pub hot_reload_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShaderSource<'a> {
    Wgsl(&'a str),
    SpirV(&'a [u32]),
    Msl(&'a str),
    Hlsl(&'a str),
    Slang(&'a str),
    File(PathBuf),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShaderStages(pub u32);

impl ShaderStages {
    pub const VERTEX: Self = Self(1 << 0);
    pub const FRAGMENT: Self = Self(1 << 1);
    pub const COMPUTE: Self = Self(1 << 2);
    pub const MESH: Self = Self(1 << 3);
    pub const TASK: Self = Self(1 << 4);
    pub const RAYGEN: Self = Self(1 << 5);
    pub const MISS: Self = Self(1 << 6);
    pub const CLOSEST_HIT: Self = Self(1 << 7);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl std::ops::BitOr for ShaderStages {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderEntryPoints<'a> {
    pub vertex: Option<&'a str>,
    pub fragment: Option<&'a str>,
    pub compute: Option<&'a str>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShaderEntryPointInfo {
    pub vertex: Option<String>,
    pub fragment: Option<String>,
    pub compute: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShaderReflectionMode {
    Auto,
    Explicit(ShaderInterfaceDesc),
    Disabled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShaderFeatureSet {
    pub flags: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShaderInterfaceDesc {
    pub resources: Vec<ShaderResourceBinding>,
    pub push_constants: Vec<PushConstantRange>,
    pub vertex_inputs: Vec<VertexInputRequirement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderResourceBinding {
    pub name: String,
    pub binding_class: BindingClass,
    pub visibility: ShaderStages,
    pub ty: BindingType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingClass {
    Uniform,
    Storage,
    Texture,
    Sampler,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingType {
    Buffer,
    Texture(TextureDimension),
    Sampler,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushConstantRange {
    pub stages: ShaderStages,
    pub range: Range<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexInputRequirement {
    pub semantic: VertexSemantic,
    pub format: VertexFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShaderInfo {
    pub label: Option<String>,
    pub stages: ShaderStages,
    pub entry_points: ShaderEntryPointInfo,
    pub hot_reload_key: Option<String>,
    pub status: ResourceStatus,
    pub interface: ShaderInterfaceDesc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StandardMaterialDesc {
    pub label: Option<String>,
    pub domain: MaterialDomain,
    pub base_color: Color,
    pub base_color_texture: Option<TextureHandle>,
    pub normal_texture: Option<TextureHandle>,
    pub metallic_roughness_texture: Option<TextureHandle>,
    pub occlusion_texture: Option<TextureHandle>,
    pub emissive_texture: Option<TextureHandle>,
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: Vec3,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    pub receive_shadows: bool,
    pub cast_shadows: bool,
}

impl Default for StandardMaterialDesc {
    fn default() -> Self {
        Self {
            label: None,
            domain: MaterialDomain::Opaque,
            base_color: Color::WHITE,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            metallic: 0.0,
            roughness: 0.5,
            emissive: Vec3::ZERO,
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            receive_shadows: true,
            cast_shadows: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialDomain {
    Opaque,
    AlphaCutout,
    Transparent,
    Decal,
    Sky,
    PostProcess,
    Unlit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
    Premultiplied,
    Additive,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialDesc {
    pub label: Option<String>,
    pub template: MaterialTemplateHandle,
    pub parameters: MaterialParameters,
    pub overrides: MaterialOverrides,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialTemplateDesc {
    pub label: Option<String>,
    pub shader: ShaderHandle,
    pub domain: MaterialDomain,
    pub render_state: RenderStateDesc,
    pub parameter_schema: MaterialParameterSchema,
    pub passes: MaterialPassFlags,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderStateDesc {
    pub depth_write: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialParameterSchema {
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialParameter {
    pub name: String,
    pub value: MaterialParameterValue,
}

pub type MaterialParameters = Vec<MaterialParameter>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MaterialOverrides {
    pub render_state: Option<RenderStateDesc>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MaterialParameterValue {
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Color(Color),
    Mat4(Mat4),
    Texture(TextureHandle),
    Sampler(SamplerHandle),
    Bytes(Vec<u8>),
}

pub type MaterialValue = MaterialParameterValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialParamId(u32);

#[derive(Clone, Debug, PartialEq)]
pub enum MaterialUpdate {
    SetBool(String, bool),
    SetI32(String, i32),
    SetU32(String, u32),
    SetFloat(String, f32),
    SetVec2(String, Vec2),
    SetVec3(String, Vec3),
    SetVec4(String, Vec4),
    SetMat4(String, Mat4),
    SetColor(String, Color),
    SetTexture(String, Option<TextureHandle>),
    SetSampler(String, Option<SamplerHandle>),
    SetBytes(String, Vec<u8>),
    ReplaceAll(Vec<MaterialParameter>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaterialPassFlags(pub u32);

impl MaterialPassFlags {
    pub const DEPTH_PREPASS: Self = Self(1 << 0);
    pub const SHADOW: Self = Self(1 << 1);
    pub const GBUFFER: Self = Self(1 << 2);
    pub const FORWARD: Self = Self(1 << 3);
    pub const TRANSPARENT: Self = Self(1 << 4);
    pub const MOTION: Self = Self(1 << 5);
    pub const PICKING: Self = Self(1 << 6);
    pub const CUSTOM_PHASE_COUNT: u8 = 16;
    const CUSTOM_PHASE_SHIFT: u8 = 16;

    pub fn custom(index: u8) -> Result<Self, RendererError> {
        if index >= Self::CUSTOM_PHASE_COUNT {
            return Err(RendererError::Validation(
                "custom material pass index must be less than 16".to_owned(),
            ));
        }
        Ok(Self(1 << (Self::CUSTOM_PHASE_SHIFT + index)))
    }

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

impl std::ops::BitOr for MaterialPassFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneDesc {
    pub label: Option<String>,
    pub max_objects_hint: Option<u32>,
    pub max_lights_hint: Option<u32>,
    pub enable_gpu_culling: bool,
    pub enable_occlusion_culling: bool,
}

impl Default for SceneDesc {
    fn default() -> Self {
        Self {
            label: None,
            max_objects_hint: None,
            max_lights_hint: None,
            enable_gpu_culling: false,
            enable_occlusion_culling: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderObjectDesc {
    pub label: Option<String>,
    pub mesh: MeshHandle,
    pub materials: Vec<MaterialHandle>,
    pub transform: Mat4,
    pub previous_transform: Option<Mat4>,
    pub bounds: Option<Bounds3>,
    pub layer: RenderLayer,
    pub visibility: VisibilityFlags,
    pub flags: ObjectFlags,
    pub skeleton: Option<SkeletonInstanceHandle>,
    pub morph_weights: Option<MorphWeightsHandle>,
    pub lod_group: Option<LodGroupHandle>,
    pub user_id: u64,
}

impl Default for RenderObjectDesc {
    fn default() -> Self {
        Self {
            label: None,
            mesh: Handle::from_raw(
                NonZeroU64::new(1 | ((ResourceKind::Mesh.tag() as u64) << 56)).unwrap(),
            ),
            materials: Vec::new(),
            transform: IDENTITY_MAT4,
            previous_transform: None,
            bounds: None,
            layer: RenderLayer::default(),
            visibility: VisibilityFlags::CAMERA,
            flags: ObjectFlags::STATIC,
            skeleton: None,
            morph_weights: None,
            lod_group: None,
            user_id: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderLayer(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderLayerMask(pub u64);

impl RenderLayerMask {
    pub const fn all() -> Self {
        Self(u64::MAX)
    }

    pub const fn none() -> Self {
        Self(0)
    }

    pub const fn single(layer: RenderLayer) -> Self {
        if layer.0 < 64 {
            Self(1_u64 << layer.0)
        } else {
            Self(0)
        }
    }

    pub const fn contains(self, layer: RenderLayer) -> bool {
        layer.0 < 64 && (self.0 & (1_u64 << layer.0)) != 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisibilityFlags(pub u32);

impl VisibilityFlags {
    pub const CAMERA: Self = Self(1 << 0);
    pub const SHADOW: Self = Self(1 << 1);
    pub const REFLECTION: Self = Self(1 << 2);
    pub const PICKING: Self = Self(1 << 3);

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

impl std::ops::BitOr for VisibilityFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjectFlags(pub u32);

impl ObjectFlags {
    pub const STATIC: Self = Self(1 << 0);
    pub const DYNAMIC: Self = Self(1 << 1);
    pub const CAST_SHADOW: Self = Self(1 << 2);
    pub const RECEIVE_SHADOW: Self = Self(1 << 3);
    pub const MOTION_VECTORS: Self = Self(1 << 4);
    pub const GPU_CULLABLE: Self = Self(1 << 5);
    pub const NO_BATCH: Self = Self(1 << 6);

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

impl std::ops::BitOr for ObjectFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LightDesc {
    Directional(DirectionalLightDesc),
    Point(PointLightDesc),
    Spot(SpotLightDesc),
    Area(AreaLightDesc),
    Custom(CustomLightDesc),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalLightDesc {
    pub label: Option<String>,
    pub direction: Vec3,
    pub color: Color,
    pub illuminance_lux: f32,
    pub shadow: Option<DirectionalShadowDesc>,
    pub layer_mask: RenderLayerMask,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalShadowDesc {
    pub resolution: u32,
    pub cascades: u8,
    pub max_distance: f32,
    pub split_lambda: f32,
    pub filter: ShadowFilter,
    pub bias: ShadowBias,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShadowFilter {
    Hard,
    Pcf { taps: u8 },
    Evsm,
    Vsm,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowBias {
    pub constant: f32,
    pub slope: f32,
    pub normal: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointLightDesc {
    pub label: Option<String>,
    pub position: Vec3,
    pub color: Color,
    pub intensity_lumen: f32,
    pub radius: f32,
    pub shadow: Option<PointShadowDesc>,
    pub layer_mask: RenderLayerMask,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpotLightDesc {
    pub label: Option<String>,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Color,
    pub intensity_lumen: f32,
    pub range: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub shadow: Option<SpotShadowDesc>,
    pub layer_mask: RenderLayerMask,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointShadowDesc {
    pub resolution: u32,
    pub bias: ShadowBias,
    pub filter: ShadowFilter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpotShadowDesc {
    pub resolution: u32,
    pub bias: ShadowBias,
    pub filter: ShadowFilter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaLightDesc {
    pub label: Option<String>,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub shape: AreaLightShape,
    pub layer_mask: RenderLayerMask,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AreaLightShape {
    Rectangle { width: f32, height: f32 },
    Disk { radius: f32 },
    Sphere { radius: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomLightDesc {
    pub label: Option<String>,
    pub type_id: u16,
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub layer_mask: RenderLayerMask,
    pub parameters: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightUpdate {
    pub desc: LightDesc,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentDesc {
    pub label: Option<String>,
    pub skybox: Option<TextureHandle>,
    pub irradiance: Option<TextureHandle>,
    pub prefiltered_specular: Option<TextureHandle>,
    pub brdf_lut: Option<TextureHandle>,
    pub intensity: f32,
    pub rotation: Quat,
    pub diffuse_color: Color,
    pub diffuse_intensity: f32,
    pub specular_color: Color,
    pub specular_intensity: f32,
    pub texture: Option<TextureHandle>,
    pub background_intensity: f32,
}

impl Default for EnvironmentDesc {
    fn default() -> Self {
        Self {
            label: None,
            skybox: None,
            irradiance: None,
            prefiltered_specular: None,
            brdf_lut: None,
            intensity: 1.0,
            rotation: Quat::IDENTITY,
            diffuse_color: Color::WHITE,
            diffuse_intensity: 1.0,
            specular_color: Color::WHITE,
            specular_intensity: 1.0,
            texture: None,
            background_intensity: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentBakeDesc {
    pub label: Option<String>,
    pub resolution: u32,
    pub mip_levels: u32,
    pub intensity: f32,
    pub rotation: Quat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LodGroupDesc {
    pub label: Option<String>,
    pub levels: Vec<LodLevelDesc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LodLevelDesc {
    pub max_distance: f32,
    pub mesh: MeshHandle,
    pub materials: Vec<MaterialHandle>,
    pub bounds: Option<Bounds3>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraDesc {
    pub label: Option<String>,
    pub transform: Mat4,
    pub projection: Projection,
    pub exposure: Exposure,
    pub clear: ClearOptions,
    pub viewport: Option<Viewport>,
    pub scissor: Option<RectU>,
    pub jitter: Option<Vec2>,
    pub previous_view_proj: Option<Mat4>,
    pub flags: CameraFlags,
}

pub type Viewport = [f32; 4];
pub type RectU = [u32; 4];

#[derive(Clone, Debug, PartialEq)]
pub enum Projection {
    Perspective {
        vertical_fov: f32,
        aspect: f32,
        near: f32,
        far: Option<f32>,
        reverse_z: bool,
    },
    Orthographic {
        width: f32,
        height: f32,
        near: f32,
        far: f32,
        reverse_z: bool,
    },
    Custom {
        view: Mat4,
        proj: Mat4,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Exposure {
    Auto,
    Manual(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClearOptions {
    None,
    ColorDepth(Color),
    DepthOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraFlags(pub u32);

impl CameraFlags {
    pub const MAIN: Self = Self(1 << 0);
    pub const ENABLE_TAA: Self = Self(1 << 1);
    pub const ENABLE_BLOOM: Self = Self(1 << 2);
    pub const ENABLE_SSAO: Self = Self(1 << 3);
    pub const ENABLE_SKY: Self = Self(1 << 4);
    pub const ENABLE_DEBUG_DRAW: Self = Self(1 << 5);

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

impl std::ops::BitOr for CameraFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderTarget {
    MainSurface,
    Surface(SurfaceHandle),
    Texture(TextureHandle),
    TextureView(TextureViewDesc),
    External(RenderTargetHandle),
    Headless {
        width: u32,
        height: u32,
        format: TextureFormat,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureViewDesc {
    pub texture: TextureHandle,
    pub base_mip: u32,
    pub mip_count: u32,
    pub base_layer: u32,
    pub layer_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderTargetDesc {
    pub label: Option<String>,
    pub color: TextureHandle,
    pub depth: Option<TextureHandle>,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
}

#[derive(Clone, Debug)]
pub struct ViewDesc {
    pub label: Option<String>,
    pub scene: SceneHandle,
    pub camera: CameraDesc,
    pub target: RenderTarget,
    pub render_path: RenderPath,
    pub quality: ViewQualitySettings,
    pub layers: RenderLayerMask,
    pub graph_extensions: Vec<RenderGraphExtensionHandle>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewQualitySettings {
    pub hdr: bool,
    pub bloom: bool,
    pub taa: bool,
    pub fxaa: bool,
    pub ssao: bool,
    pub ssr: bool,
    pub depth_of_field: bool,
    pub motion_blur: bool,
    pub variable_rate_shading: bool,
    pub bindless_textures: bool,
    pub mesh_shaders: bool,
    pub virtual_texturing: bool,
    pub ray_tracing: bool,
    pub color_grading: ColorGradingMode,
}

impl ViewQualitySettings {
    pub const fn high() -> Self {
        Self {
            hdr: true,
            bloom: true,
            taa: true,
            fxaa: true,
            ssao: true,
            ssr: true,
            depth_of_field: true,
            motion_blur: true,
            variable_rate_shading: false,
            bindless_textures: false,
            mesh_shaders: false,
            virtual_texturing: false,
            ray_tracing: false,
            color_grading: ColorGradingMode::Lut,
        }
    }
}

impl Default for ViewQualitySettings {
    fn default() -> Self {
        Self {
            hdr: false,
            bloom: false,
            taa: false,
            fxaa: false,
            ssao: false,
            ssr: false,
            depth_of_field: false,
            motion_blur: false,
            variable_rate_shading: false,
            bindless_textures: false,
            mesh_shaders: false,
            virtual_texturing: false,
            ray_tracing: false,
            color_grading: ColorGradingMode::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorGradingMode {
    None,
    Lut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameInput {
    pub delta_time: f32,
    pub absolute_time: f64,
    pub frame_index_override: Option<u64>,
    pub wait_for_gpu: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameStats {
    pub frame_index: u64,
    pub cpu_build_time_ms: f32,
    pub cpu_submit_time_ms: f32,
    pub gpu_time_ms: Option<f32>,
    pub gpu_profiler_enabled: bool,
    pub profile: Option<FrameProfile>,
    pub capture_triggered: bool,
    pub capture_label: Option<String>,
    pub capture: Option<FrameCapture>,
    pub draw_calls: u32,
    pub dispatch_calls: u32,
    pub triangles: u64,
    pub visible_objects: u32,
    pub culled_objects: u32,
    pub culling_outputs: Vec<FrameCullingOutput>,
    pub ssao_outputs: Vec<FrameSsaoOutput>,
    pub light_cluster_outputs: Vec<FrameLightClusterOutput>,
    pub area_light_outputs: Vec<FrameAreaLightOutput>,
    pub ray_tracing_outputs: Vec<FrameRayTracingOutput>,
    pub shadow_outputs: Vec<FrameShadowOutput>,
    pub gbuffer_outputs: Vec<FrameGBufferOutput>,
    pub lod_outputs: Vec<FrameLodOutput>,
    pub streaming_outputs: Vec<FrameStreamingOutput>,
    pub debug_draw_outputs: Vec<FrameDebugDrawOutput>,
    pub picking_outputs: Vec<FramePickingOutput>,
    pub environment_outputs: Vec<FrameEnvironmentOutput>,
    pub skinned_objects: u32,
    pub morphed_objects: u32,
    pub deformed_objects: u32,
    pub deformation_outputs: Vec<FrameDeformationOutput>,
    pub motion_vector_objects: u32,
    pub motion_vector_views: u32,
    pub motion_vector_outputs: Vec<FrameMotionVectorOutput>,
    pub post_process_outputs: Vec<FramePostProcessOutput>,
    pub pipeline_switches: u32,
    pub material_switches: u32,
    pub pipeline_statistics: Option<FramePipelineStatistics>,
    pub upload: UploadStats,
    pub memory: MemoryStats,
    pub graph: RenderGraphStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCullingOutput {
    pub view_label: Option<String>,
    pub gpu_culling: bool,
    pub occlusion_culling: bool,
    pub tested_objects: u32,
    pub visible_objects: u32,
    pub culled_objects: u32,
    pub visibility_buffer_label: String,
    pub visibility_buffer_bytes: u64,
    pub indirect_args_buffer_label: String,
    pub indirect_args_buffer_bytes: u64,
    pub occlusion_result_buffer_label: Option<String>,
    pub occlusion_result_buffer_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameSsaoOutput {
    pub view_label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub output_texture_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameLightClusterOutput {
    pub view_label: Option<String>,
    pub tile_size_px: u32,
    pub z_slices: u32,
    pub cluster_count: u32,
    pub clustered_lights: u32,
    pub cluster_buffer_label: String,
    pub cluster_buffer_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameAreaLightOutput {
    pub view_label: Option<String>,
    pub area_lights: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameRayTracingOutput {
    pub view_label: Option<String>,
    pub visible_geometries: u32,
    pub accel_buffer_label: String,
    pub accel_buffer_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameShadowOutput {
    pub view_label: Option<String>,
    pub pass_label: String,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub shadowed_lights: u32,
    pub atlas_texture_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameGBufferOutput {
    pub view_label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub albedo_format: TextureFormat,
    pub normal_format: TextureFormat,
    pub material_format: TextureFormat,
    pub albedo_texture_label: String,
    pub normal_texture_label: String,
    pub material_texture_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameLodOutput {
    pub view_label: Option<String>,
    pub object: ObjectHandle,
    pub lod_group: LodGroupHandle,
    pub level_index: u32,
    pub selected_mesh: MeshHandle,
    pub distance: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameStreamingOutput {
    pub view_label: Option<String>,
    pub streamable_textures: u32,
    pub streamable_texture_mips: u32,
    pub streamable_meshes: u32,
    pub streamable_mesh_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameDebugDrawOutput {
    pub view_label: Option<String>,
    pub command_count: u32,
    pub target_texture_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePickingOutput {
    pub view_label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub pickable_objects: u32,
    pub target_texture_label: String,
    pub ready_results: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameEnvironmentOutput {
    pub view_label: Option<String>,
    pub environment_label: Option<String>,
    pub skybox_texture_label: Option<String>,
    pub irradiance_texture_label: Option<String>,
    pub prefiltered_specular_texture_label: Option<String>,
    pub brdf_lut_texture_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameDeformationOutput {
    pub view_label: Option<String>,
    pub skinned_objects: u32,
    pub morphed_objects: u32,
    pub deformed_objects: u32,
    pub output_buffer_label: String,
    pub output_buffer_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameMotionVectorOutput {
    pub view_label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub moving_objects: u32,
    pub camera_motion: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePostProcessOutput {
    pub view_label: Option<String>,
    pub pass_label: String,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub output_texture_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomPostProcessDesc {
    pub label: String,
    pub pipeline_label: Option<String>,
    pub output_texture_label: Option<String>,
}

impl CustomPostProcessDesc {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            pipeline_label: None,
            output_texture_label: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomPostProcessPass {
    label: String,
    pipeline_label: String,
    output_texture_label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FramePipelineStatistics {
    pub input_assembly_vertices: u64,
    pub input_assembly_primitives: u64,
    pub vertex_shader_invocations: u64,
    pub clipping_invocations: u64,
    pub clipping_primitives: u64,
    pub fragment_shader_invocations: u64,
    pub compute_shader_invocations: u64,
    pub draw_calls: u32,
    pub dispatch_calls: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameProfile {
    pub frame_index: u64,
    pub cpu_build_time_ms: f32,
    pub cpu_submit_time_ms: f32,
    pub gpu_time_ms: Option<f32>,
    pub graph_passes: u32,
    pub graph_barriers: u32,
    pub debug_groups: u32,
    pub draw_calls: u32,
    pub dispatch_calls: u32,
    pub deformed_objects: u32,
    pub motion_vector_objects: u32,
    pub motion_vector_views: u32,
    pub pipeline_statistics: Option<FramePipelineStatistics>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCaptureResourceDump {
    pub meshes: usize,
    pub buffers: usize,
    pub textures: usize,
    pub samplers: usize,
    pub shaders: usize,
    pub materials: usize,
    pub material_templates: usize,
    pub environments: usize,
    pub render_targets: usize,
    pub lod_groups: usize,
    pub cameras: usize,
    pub graph_extensions: usize,
    pub skeleton_instances: usize,
    pub morph_weights: usize,
    pub scenes: usize,
    pub views: usize,
    pub picking_results: usize,
    pub resident_bytes: u64,
    pub delayed_destroy_count: usize,
    pub pending_uploads: usize,
    pub staging_bytes_in_use: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameCapture {
    pub label: Option<String>,
    pub backend: FrameCaptureBackend,
    pub status: FrameCaptureStatus,
    pub include_resource_dump: bool,
    pub open_after_capture: bool,
    pub frame_index: u64,
    pub graph: RenderGraphStats,
    pub cpu_build_time_ms: f32,
    pub cpu_submit_time_ms: f32,
    pub draw_calls: u32,
    pub dispatch_calls: u32,
    pub visible_objects: u32,
    pub culled_objects: u32,
    pub skinned_objects: u32,
    pub morphed_objects: u32,
    pub deformed_objects: u32,
    pub motion_vector_objects: u32,
    pub motion_vector_views: u32,
    pub culling_outputs: Vec<FrameCullingOutput>,
    pub ssao_outputs: Vec<FrameSsaoOutput>,
    pub light_cluster_outputs: Vec<FrameLightClusterOutput>,
    pub area_light_outputs: Vec<FrameAreaLightOutput>,
    pub ray_tracing_outputs: Vec<FrameRayTracingOutput>,
    pub shadow_outputs: Vec<FrameShadowOutput>,
    pub gbuffer_outputs: Vec<FrameGBufferOutput>,
    pub lod_outputs: Vec<FrameLodOutput>,
    pub streaming_outputs: Vec<FrameStreamingOutput>,
    pub debug_draw_outputs: Vec<FrameDebugDrawOutput>,
    pub picking_outputs: Vec<FramePickingOutput>,
    pub environment_outputs: Vec<FrameEnvironmentOutput>,
    pub deformation_outputs: Vec<FrameDeformationOutput>,
    pub motion_vector_outputs: Vec<FrameMotionVectorOutput>,
    pub post_process_outputs: Vec<FramePostProcessOutput>,
    pub pipeline_statistics: Option<FramePipelineStatistics>,
    pub resource_dump: Option<FrameCaptureResourceDump>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FrameCaptureBackend {
    #[default]
    Internal,
    RenderDoc,
    ExternalDebugger,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameCaptureStatus {
    #[default]
    Captured,
    BackendHookRequested,
    BackendUnavailable,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UploadStats {
    pub bytes_queued: u64,
    pub bytes_uploaded_this_frame: u64,
    pub pending_uploads: usize,
    pub staging_bytes_in_use: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryStats {
    pub resident_bytes: u64,
    pub delayed_destroy_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidencyPriority {
    Critical,
    High,
    Normal,
    Low,
    Streamable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderPhaseKind {
    DepthPrepass,
    Shadow,
    GBuffer,
    ForwardOpaque,
    ForwardTransparent,
    MotionVector,
    Picking,
    Debug,
    Custom(u8),
}

pub type RenderPhaseId = RenderPhaseKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub shader: ShaderHandle,
    pub material_template: MaterialTemplateHandle,
    pub vertex_layout_hash: u64,
    pub render_state_hash: u64,
    pub pass: RenderPhaseKind,
    pub sample_count: u8,
    pub depth_format: DepthFormat,
    pub color_format: TextureFormat,
    pub feature_bits: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PipelineCacheStats {
    pub total: usize,
    pub ready: usize,
    pub compiling: usize,
    pub failed: usize,
    pub cache_hits_this_frame: u32,
    pub cache_misses_this_frame: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineWarmupRequest {
    pub key: PipelineKey,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShaderReloadDesc<'a> {
    pub source: ShaderSource<'a>,
    pub entry_points: ShaderEntryPoints<'a>,
    pub reflection: ShaderReflectionMode,
    pub features: ShaderFeatureSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureOptions {
    pub label: Option<String>,
    pub backend: FrameCaptureBackend,
    pub include_resource_dump: bool,
    pub open_after_capture: bool,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            label: None,
            backend: FrameCaptureBackend::Internal,
            include_resource_dump: true,
            open_after_capture: false,
        }
    }
}

fn custom_post_process_from_desc(
    desc: CustomPostProcessDesc,
) -> Result<CustomPostProcessPass, RendererError> {
    let label = desc.label.trim();
    if label.is_empty() {
        return Err(RendererError::Validation(
            "custom post process label must not be empty".to_owned(),
        ));
    }
    let pipeline_label = desc
        .pipeline_label
        .as_deref()
        .unwrap_or(label)
        .trim()
        .to_owned();
    if pipeline_label.is_empty() {
        return Err(RendererError::Validation(
            "custom post process pipeline label must not be empty".to_owned(),
        ));
    }
    let output_texture_label = desc
        .output_texture_label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{label}_output"));
    Ok(CustomPostProcessPass {
        label: label.to_owned(),
        pipeline_label,
        output_texture_label,
    })
}

impl RenderGraphExtension for CustomPostProcessPass {
    fn name(&self) -> &str {
        &self.label
    }

    fn build(
        &self,
        ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError> {
        let main_color = ctx.main_color();
        let main_color_desc = graph.texture_desc(main_color).cloned().ok_or_else(|| {
            RendererError::RenderGraphValidation(
                "custom post process requires a declared main color texture".to_owned(),
            )
        })?;
        let output = graph.create_texture(GraphTextureDesc {
            label: Some(self.output_texture_label.clone()),
            width: main_color_desc.width,
            height: main_color_desc.height,
            format: main_color_desc.format,
        });
        let pipeline_label = self.pipeline_label.clone();
        graph
            .add_pass(self.label.clone())
            .queue(QueueType::Graphics)
            .read_texture(main_color, TextureReadUsage::Sampled)
            .color_attachment(output, ColorAttachmentOps::clear_store())
            .execute(move |ctx| {
                let pipeline = ctx.pipeline(&pipeline_label)?;
                let mut pass = ctx.begin_render_pass(RenderPassDesc::label(&pipeline_label));
                pass.set_pipeline(pipeline);
                pass.draw_fullscreen_triangle();
                Ok(())
            });
        let resolve_label = format!("{}_resolve", self.label);
        graph
            .add_pass(resolve_label.clone())
            .queue(QueueType::Graphics)
            .read_texture(output, TextureReadUsage::Sampled)
            .color_attachment(main_color, ColorAttachmentOps::load_store())
            .execute(move |ctx| {
                let pipeline = ctx.pipeline(&resolve_label)?;
                let mut pass = ctx.begin_render_pass(RenderPassDesc::label(&resolve_label));
                pass.set_pipeline(pipeline);
                pass.draw_fullscreen_triangle();
                Ok(())
            });
        Ok(())
    }

    fn custom_post_process_info(&self) -> Option<CustomPostProcessInfo> {
        Some(CustomPostProcessInfo {
            pass_label: self.label.clone(),
            output_texture_label: self.output_texture_label.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlinePass {
    pub source_depth: GraphTexture,
    pub output: GraphTexture,
    pub color: Color,
}

impl RenderGraphExtension for OutlinePass {
    fn name(&self) -> &str {
        "editor_outline"
    }

    fn build(
        &self,
        _ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError> {
        let source_depth = self.source_depth;
        let output = self.output;
        let color = self.color;
        graph
            .add_pass("editor_outline")
            .queue(QueueType::Graphics)
            .read_texture(source_depth, TextureReadUsage::Sampled)
            .color_attachment(output, ColorAttachmentOps::load_store())
            .execute(move |ctx| {
                let _outline_color = color;
                let pipeline = ctx.pipeline("outline_pipeline")?;
                let mut pass = ctx.begin_render_pass(RenderPassDesc::label("editor_outline"));
                pass.set_pipeline(pipeline);
                pass.draw_fullscreen_triangle();
                Ok(())
            });
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct StoredResource<T> {
    generation: u32,
    status: ResourceStatus,
    value: Option<T>,
    priority: ResidencyPriority,
}

impl<T> StoredResource<T> {
    fn occupied(value: T) -> Self {
        Self {
            generation: 1,
            status: ResourceStatus::Ready,
            value: Some(value),
            priority: ResidencyPriority::Normal,
        }
    }
}

#[derive(Clone, Debug)]
struct Arena<T> {
    resources: Vec<StoredResource<T>>,
    free: Vec<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Arena<T> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            resources: Vec::with_capacity(capacity),
            free: Vec::new(),
        }
    }

    fn insert<K>(&mut self, kind: ResourceKind, value: T) -> Handle<K> {
        let index = self.free.pop().unwrap_or(self.resources.len() as u32);
        if index as usize == self.resources.len() {
            self.resources.push(StoredResource::occupied(value));
        } else {
            let slot = &mut self.resources[index as usize];
            slot.generation = slot.generation.wrapping_add(1).max(1);
            slot.status = ResourceStatus::Ready;
            slot.value = Some(value);
        }
        let generation = self.resources[index as usize].generation;
        make_handle(kind, index, generation)
    }

    fn reserve<K>(&mut self, kind: ResourceKind) -> Handle<K> {
        let index = self.free.pop().unwrap_or(self.resources.len() as u32);
        if index as usize == self.resources.len() {
            self.resources.push(StoredResource {
                generation: 1,
                status: ResourceStatus::PendingUpload,
                value: None,
                priority: ResidencyPriority::Normal,
            });
        } else {
            let slot = &mut self.resources[index as usize];
            slot.generation = slot.generation.wrapping_add(1).max(1);
            slot.status = ResourceStatus::PendingUpload;
            slot.value = None;
            slot.priority = ResidencyPriority::Normal;
        }
        let generation = self.resources[index as usize].generation;
        make_handle(kind, index, generation)
    }

    fn fill_reserved<K>(
        &mut self,
        kind: ResourceKind,
        handle: Handle<K>,
        value: T,
    ) -> Result<(), RendererError> {
        if handle.kind_tag() != kind.tag() {
            return Err(RendererError::InvalidHandle {
                kind,
                raw: handle.raw().get(),
            });
        }
        let Some(slot) = self.resources.get_mut(handle.index() as usize) else {
            return Err(RendererError::InvalidHandle {
                kind,
                raw: handle.raw().get(),
            });
        };
        if slot.generation != handle.generation() {
            return Err(RendererError::InvalidHandle {
                kind,
                raw: handle.raw().get(),
            });
        }
        if slot.value.is_some() {
            return Err(RendererError::Validation(
                "reserved resource slot is already occupied".to_owned(),
            ));
        }
        slot.status = ResourceStatus::Ready;
        slot.value = Some(value);
        Ok(())
    }

    fn get<K>(&self, kind: ResourceKind, handle: Handle<K>) -> Option<&StoredResource<T>> {
        if handle.kind_tag() != kind.tag() {
            return None;
        }
        self.resources
            .get(handle.index() as usize)
            .filter(|slot| slot.generation == handle.generation() && slot.value.is_some())
    }

    fn get_mut<K>(
        &mut self,
        kind: ResourceKind,
        handle: Handle<K>,
    ) -> Option<&mut StoredResource<T>> {
        if handle.kind_tag() != kind.tag() {
            return None;
        }
        self.resources
            .get_mut(handle.index() as usize)
            .filter(|slot| slot.generation == handle.generation() && slot.value.is_some())
    }

    fn priority<K>(&self, kind: ResourceKind, handle: Handle<K>) -> Option<ResidencyPriority> {
        self.get(kind, handle).map(|slot| slot.priority)
    }

    fn set_priority<K>(
        &mut self,
        kind: ResourceKind,
        handle: Handle<K>,
        priority: ResidencyPriority,
    ) -> Result<(), RendererError> {
        let raw = handle.raw().get();
        let slot = self
            .get_mut(kind, handle)
            .ok_or(RendererError::InvalidHandle { kind, raw })?;
        slot.priority = priority;
        Ok(())
    }

    fn set_status<K>(
        &mut self,
        kind: ResourceKind,
        handle: Handle<K>,
        status: ResourceStatus,
    ) -> Result<&mut StoredResource<T>, RendererError> {
        let raw = handle.raw().get();
        let slot = self
            .get_mut(kind, handle)
            .ok_or(RendererError::InvalidHandle { kind, raw })?;
        slot.status = status;
        Ok(slot)
    }

    fn destroy<K>(&mut self, kind: ResourceKind, handle: Handle<K>) -> Result<(), RendererError> {
        let Some(slot) = self.get_mut(kind, handle) else {
            return Err(RendererError::InvalidHandle {
                kind,
                raw: handle.raw().get(),
            });
        };
        slot.value = None;
        slot.status = ResourceStatus::DestroyQueued;
        self.free.push(handle.index());
        Ok(())
    }
}

fn count_destroy_queued<T>(arena: &Arena<T>) -> usize {
    arena
        .resources
        .iter()
        .filter(|slot| slot.status == ResourceStatus::DestroyQueued)
        .count()
}

fn count_ready<T>(arena: &Arena<T>) -> usize {
    arena
        .resources
        .iter()
        .filter(|slot| slot.status == ResourceStatus::Ready && slot.value.is_some())
        .count()
}

fn validate_camera_desc(desc: &CameraDesc) -> Result<(), RendererError> {
    validate_mat4_finite(desc.transform, "camera transform")?;
    match desc.projection {
        Projection::Perspective {
            vertical_fov,
            aspect,
            near,
            far,
            ..
        } => {
            if !vertical_fov.is_finite() || vertical_fov <= 0.0 {
                return Err(RendererError::Validation(
                    "camera vertical_fov must be finite and positive".to_owned(),
                ));
            }
            if !aspect.is_finite() || aspect <= 0.0 {
                return Err(RendererError::Validation(
                    "camera aspect must be finite and positive".to_owned(),
                ));
            }
            if !near.is_finite() || near <= 0.0 {
                return Err(RendererError::Validation(
                    "camera near plane must be finite and positive".to_owned(),
                ));
            }
            if far.is_some_and(|far| !far.is_finite() || far <= near) {
                return Err(RendererError::Validation(
                    "camera far plane must be finite and greater than near".to_owned(),
                ));
            }
        }
        Projection::Orthographic {
            width,
            height,
            near,
            far,
            ..
        } => {
            if !width.is_finite() || width <= 0.0 || !height.is_finite() || height <= 0.0 {
                return Err(RendererError::Validation(
                    "orthographic camera dimensions must be finite and positive".to_owned(),
                ));
            }
            if !near.is_finite() || !far.is_finite() || far <= near {
                return Err(RendererError::Validation(
                    "orthographic camera far plane must be finite and greater than near".to_owned(),
                ));
            }
        }
        Projection::Custom { view, proj } => {
            validate_mat4_finite(view, "custom camera view matrix")?;
            validate_mat4_finite(proj, "custom camera projection matrix")?;
        }
    }
    if let Exposure::Manual(value) = desc.exposure {
        validate_positive_finite(value, "manual exposure")?;
    }
    if let Some(viewport) = desc.viewport {
        if viewport.iter().any(|value| !value.is_finite())
            || viewport[2] <= 0.0
            || viewport[3] <= 0.0
        {
            return Err(RendererError::Validation(
                "camera viewport must be finite with positive size".to_owned(),
            ));
        }
    }
    if let Some(scissor) = desc.scissor {
        if scissor[2] == 0 || scissor[3] == 0 {
            return Err(RendererError::Validation(
                "camera scissor must have non-zero size".to_owned(),
            ));
        }
    }
    if let Some(jitter) = desc.jitter {
        if !jitter.x.is_finite() || !jitter.y.is_finite() {
            return Err(RendererError::Validation(
                "camera jitter must be finite".to_owned(),
            ));
        }
    }
    if let Some(previous) = desc.previous_view_proj {
        validate_mat4_finite(previous, "camera previous_view_proj")?;
    }
    Ok(())
}

fn validate_mesh_submeshes(
    submeshes: &[SubMeshDesc],
    index_count: u32,
    layout: &VertexLayout,
    vertex_stream_bytes: &[Vec<u8>],
) -> Result<(), RendererError> {
    let vertex_count = mesh_vertex_count(layout, vertex_stream_bytes)?;
    for submesh in submeshes {
        validate_bounds(submesh.bounds)?;
        if submesh.index_range.is_empty() {
            return Err(RendererError::Validation(
                "submesh index_range must be non-empty".to_owned(),
            ));
        }
        if submesh.vertex_range.is_empty() {
            return Err(RendererError::Validation(
                "submesh vertex_range must be non-empty".to_owned(),
            ));
        }
        if index_count > 0 && submesh.index_range.end > index_count {
            return Err(RendererError::Validation(
                "submesh index_range exceeds mesh index count".to_owned(),
            ));
        }
        if let Some(vertex_count) = vertex_count {
            if submesh.vertex_range.end > vertex_count {
                return Err(RendererError::Validation(
                    "submesh vertex_range exceeds mesh vertex count".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn mesh_vertex_count(
    layout: &VertexLayout,
    vertex_stream_bytes: &[Vec<u8>],
) -> Result<Option<u32>, RendererError> {
    if layout.streams.is_empty() {
        return Ok(None);
    }
    let mut count = None;
    for (index, stream) in layout.streams.iter().enumerate() {
        if stream.stride == 0 {
            return Err(RendererError::Validation(
                "mesh vertex stream stride must be non-zero".to_owned(),
            ));
        }
        let Some(bytes) = vertex_stream_bytes.get(index) else {
            return Err(RendererError::Validation(
                "mesh vertex stream data is missing".to_owned(),
            ));
        };
        let stride = stream.stride as usize;
        if bytes.len() % stride != 0 {
            return Err(RendererError::Validation(
                "mesh vertex stream byte length must be a multiple of stride".to_owned(),
            ));
        }
        let stream_count = (bytes.len() / stride) as u32;
        if let Some(count) = count {
            if count != stream_count {
                return Err(RendererError::Validation(
                    "mesh vertex streams must have matching vertex counts".to_owned(),
                ));
            }
        } else {
            count = Some(stream_count);
        }
    }
    Ok(count)
}

fn validate_mesh_deformation_data(
    skin: Option<&SkinDesc<'_>>,
    morph_targets: &[MorphTargetDesc<'_>],
    meshlets: Option<&MeshletData<'_>>,
    vertex_count: Option<u32>,
) -> Result<(), RendererError> {
    if let Some(skin) = skin {
        if skin.inverse_bind_matrices.is_empty() {
            return Err(RendererError::Validation(
                "mesh skin must contain at least one inverse bind matrix".to_owned(),
            ));
        }
        validate_mat4_slice(skin.inverse_bind_matrices, "mesh inverse bind matrix")?;
    }

    if !morph_targets.is_empty() && vertex_count.is_none() {
        return Err(RendererError::Validation(
            "mesh morph targets require a known vertex count".to_owned(),
        ));
    }
    let vertex_count = vertex_count.unwrap_or(0) as usize;
    for (index, target) in morph_targets.iter().enumerate() {
        if target.positions.is_none() && target.normals.is_none() && target.tangents.is_none() {
            return Err(RendererError::Validation(format!(
                "mesh morph target {index} must contain at least one attribute"
            )));
        }
        validate_morph_target_attribute(index, "positions", target.positions, vertex_count)?;
        validate_morph_target_attribute(index, "normals", target.normals, vertex_count)?;
        validate_morph_target_attribute(index, "tangents", target.tangents, vertex_count)?;
    }

    if let Some(meshlets) = meshlets {
        if meshlets.bytes.is_empty() {
            return Err(RendererError::Validation(
                "meshlet data must not be empty".to_owned(),
            ));
        }
    }

    Ok(())
}

fn validate_morph_target_attribute(
    index: usize,
    name: &str,
    values: Option<&[Vec3]>,
    vertex_count: usize,
) -> Result<(), RendererError> {
    let Some(values) = values else {
        return Ok(());
    };
    if values.len() != vertex_count {
        return Err(RendererError::Validation(format!(
            "mesh morph target {index} {name} count must match vertex count"
        )));
    }
    for value in values {
        validate_vec3_finite(*value, "mesh morph target attribute")?;
    }
    Ok(())
}

fn validate_sampler_desc(desc: &SamplerDesc) -> Result<(), RendererError> {
    if desc.anisotropy == 0 {
        return Err(RendererError::Validation(
            "sampler anisotropy must be at least 1".to_owned(),
        ));
    }
    let lod_min = desc.lod_min.get();
    let lod_max = desc.lod_max.get();
    if !lod_min.is_finite() || !lod_max.is_finite() || lod_min > lod_max {
        return Err(RendererError::Validation(
            "sampler LOD range must be finite and ordered".to_owned(),
        ));
    }
    Ok(())
}

fn validate_light_desc(desc: &LightDesc) -> Result<(), RendererError> {
    match desc {
        LightDesc::Directional(light) => {
            validate_direction_vec3(light.direction, "directional light direction")?;
            validate_non_negative_finite(light.illuminance_lux, "directional light illuminance")?;
            if let Some(shadow) = &light.shadow {
                validate_directional_shadow_desc(shadow)?;
            }
        }
        LightDesc::Point(light) => {
            validate_vec3_finite(light.position, "point light position")?;
            validate_non_negative_finite(light.intensity_lumen, "point light intensity")?;
            validate_positive_finite(light.radius, "point light radius")?;
            if let Some(shadow) = &light.shadow {
                validate_point_shadow_desc(shadow)?;
            }
        }
        LightDesc::Spot(light) => {
            validate_vec3_finite(light.position, "spot light position")?;
            validate_direction_vec3(light.direction, "spot light direction")?;
            validate_non_negative_finite(light.intensity_lumen, "spot light intensity")?;
            validate_positive_finite(light.range, "spot light range")?;
            validate_non_negative_finite(light.inner_angle, "spot light inner_angle")?;
            validate_non_negative_finite(light.outer_angle, "spot light outer_angle")?;
            if light.inner_angle > light.outer_angle {
                return Err(RendererError::Validation(
                    "spot light inner_angle must not exceed outer_angle".to_owned(),
                ));
            }
            if let Some(shadow) = &light.shadow {
                validate_spot_shadow_desc(shadow)?;
            }
        }
        LightDesc::Area(light) => {
            validate_vec3_finite(light.position, "area light position")?;
            validate_direction_vec3(light.direction, "area light direction")?;
            validate_non_negative_finite(light.intensity, "area light intensity")?;
            validate_positive_finite(light.range, "area light range")?;
            match light.shape {
                AreaLightShape::Rectangle { width, height } => {
                    validate_positive_finite(width, "area light rectangle width")?;
                    validate_positive_finite(height, "area light rectangle height")?;
                }
                AreaLightShape::Disk { radius } | AreaLightShape::Sphere { radius } => {
                    validate_positive_finite(radius, "area light radius")?;
                }
            }
        }
        LightDesc::Custom(light) => {
            validate_vec3_finite(light.position, "custom light position")?;
            validate_non_negative_finite(light.intensity, "custom light intensity")?;
            validate_positive_finite(light.range, "custom light range")?;
            if light.parameters.len() > 16 {
                return Err(RendererError::Validation(
                    "custom light parameters must contain at most 16 values".to_owned(),
                ));
            }
            for value in &light.parameters {
                if !value.is_finite() {
                    return Err(RendererError::Validation(
                        "custom light parameters must be finite".to_owned(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_direction_vec3(value: Vec3, role: &str) -> Result<(), RendererError> {
    validate_vec3_finite(value, role)?;
    let len_sq = value.x * value.x + value.y * value.y + value.z * value.z;
    if len_sq <= f32::EPSILON {
        return Err(RendererError::Validation(format!(
            "{role} must be non-zero"
        )));
    }
    Ok(())
}

fn validate_bounds(bounds: Bounds3) -> Result<(), RendererError> {
    validate_vec3_finite(bounds.min, "bounds min")?;
    validate_vec3_finite(bounds.max, "bounds max")?;
    if bounds.min.x > bounds.max.x || bounds.min.y > bounds.max.y || bounds.min.z > bounds.max.z {
        return Err(RendererError::Validation(
            "bounds min must not exceed max".to_owned(),
        ));
    }
    Ok(())
}

fn validate_render_object_desc(object: &RenderObjectDesc) -> Result<(), RendererError> {
    validate_mat4_finite(object.transform, "object transform")?;
    if let Some(previous) = object.previous_transform {
        validate_mat4_finite(previous, "object previous_transform")?;
    }
    if let Some(bounds) = object.bounds {
        validate_bounds(bounds)?;
    }
    Ok(())
}

fn validate_mat4_finite(value: Mat4, role: &str) -> Result<(), RendererError> {
    if value
        .iter()
        .flatten()
        .any(|component| !component.is_finite())
    {
        return Err(RendererError::Validation(format!("{role} must be finite")));
    }
    Ok(())
}

fn validate_mat4_slice(values: &[Mat4], role: &str) -> Result<(), RendererError> {
    for value in values {
        validate_mat4_finite(*value, role)?;
    }
    Ok(())
}

fn validate_directional_shadow_desc(desc: &DirectionalShadowDesc) -> Result<(), RendererError> {
    if desc.resolution == 0 || desc.cascades == 0 {
        return Err(RendererError::Validation(
            "directional shadow resolution and cascades must be non-zero".to_owned(),
        ));
    }
    validate_positive_finite(desc.max_distance, "directional shadow max_distance")?;
    if !desc.split_lambda.is_finite() || !(0.0..=1.0).contains(&desc.split_lambda) {
        return Err(RendererError::Validation(
            "directional shadow split_lambda must be finite and in 0..=1".to_owned(),
        ));
    }
    validate_shadow_filter(desc.filter)?;
    validate_shadow_bias(desc.bias)
}

fn validate_point_shadow_desc(desc: &PointShadowDesc) -> Result<(), RendererError> {
    if desc.resolution == 0 {
        return Err(RendererError::Validation(
            "point shadow resolution must be non-zero".to_owned(),
        ));
    }
    validate_shadow_filter(desc.filter)?;
    validate_shadow_bias(desc.bias)
}

fn validate_spot_shadow_desc(desc: &SpotShadowDesc) -> Result<(), RendererError> {
    if desc.resolution == 0 {
        return Err(RendererError::Validation(
            "spot shadow resolution must be non-zero".to_owned(),
        ));
    }
    validate_shadow_filter(desc.filter)?;
    validate_shadow_bias(desc.bias)
}

fn validate_shadow_bias(bias: ShadowBias) -> Result<(), RendererError> {
    validate_vec3_finite(
        Vec3::new(bias.constant, bias.slope, bias.normal),
        "shadow bias",
    )
}

fn validate_shadow_filter(filter: ShadowFilter) -> Result<(), RendererError> {
    if let ShadowFilter::Pcf { taps } = filter {
        if taps == 0 {
            return Err(RendererError::Validation(
                "shadow PCF taps must be non-zero".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_vec3_finite(value: Vec3, role: &str) -> Result<(), RendererError> {
    if !value.x.is_finite() || !value.y.is_finite() || !value.z.is_finite() {
        return Err(RendererError::Validation(format!("{role} must be finite")));
    }
    Ok(())
}

fn validate_quat_finite(value: Quat, role: &str) -> Result<(), RendererError> {
    if !value.x.is_finite() || !value.y.is_finite() || !value.z.is_finite() || !value.w.is_finite()
    {
        return Err(RendererError::Validation(format!("{role} must be finite")));
    }
    Ok(())
}

fn validate_positive_finite(value: f32, role: &str) -> Result<(), RendererError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(RendererError::Validation(format!(
            "{role} must be finite and positive"
        )));
    }
    Ok(())
}

fn validate_non_negative_finite(value: f32, role: &str) -> Result<(), RendererError> {
    if !value.is_finite() || value < 0.0 {
        return Err(RendererError::Validation(format!(
            "{role} must be finite and non-negative"
        )));
    }
    Ok(())
}

fn validate_morph_weights(weights: &[f32]) -> Result<(), RendererError> {
    if weights.is_empty() {
        return Err(RendererError::Validation(
            "morph weights must contain at least one value".to_owned(),
        ));
    }
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(RendererError::Validation(
            "morph weights must be finite".to_owned(),
        ));
    }
    Ok(())
}

fn validate_scene_desc(desc: &SceneDesc) -> Result<(), RendererError> {
    if matches!(desc.max_objects_hint, Some(0)) {
        return Err(RendererError::Validation(
            "scene max_objects_hint must be non-zero when provided".to_owned(),
        ));
    }
    if matches!(desc.max_lights_hint, Some(0)) {
        return Err(RendererError::Validation(
            "scene max_lights_hint must be non-zero when provided".to_owned(),
        ));
    }
    Ok(())
}

fn validate_renderer_config(config: &RendererConfig) -> Result<(), RendererError> {
    match config.backend {
        BackendPreference::Auto | BackendPreference::Headless => {}
        BackendPreference::Wgpu if cfg!(feature = "backend-wgpu") => {}
        BackendPreference::Wgpu => {
            return Err(RendererError::UnsupportedFeature(
                RendererFeature::BackendWgpu,
            ));
        }
        BackendPreference::Vulkan => {
            if !cfg!(feature = "backend-wgpu") {
                return Err(RendererError::UnsupportedFeature(
                    RendererFeature::BackendVulkan,
                ));
            }
        }
        BackendPreference::Metal => {
            if !cfg!(feature = "backend-wgpu") {
                return Err(RendererError::UnsupportedFeature(
                    RendererFeature::BackendMetal,
                ));
            }
        }
        BackendPreference::D3d12 => {
            if !cfg!(feature = "backend-wgpu") {
                return Err(RendererError::UnsupportedFeature(
                    RendererFeature::BackendD3d12,
                ));
            }
        }
    }
    if config.frame_latency == 0 {
        return Err(RendererError::Validation(
            "renderer frame_latency must be non-zero".to_owned(),
        ));
    }
    if config.msaa_samples == 0 || !config.msaa_samples.is_power_of_two() {
        return Err(RendererError::Validation(
            "renderer msaa_samples must be a non-zero power of two".to_owned(),
        ));
    }
    if matches!(config.surface_format, Some(TextureFormat::Depth32Float)) {
        return Err(RendererError::Validation(
            "renderer surface_format must be a color format".to_owned(),
        ));
    }
    let caps = RendererCaps::for_backend(config, "validation", "validation");
    if let Some(surface_format) = config.surface_format {
        if !caps.formats.color.contains(&surface_format) {
            return Err(RendererError::Validation(
                "renderer surface_format is not supported by renderer caps".to_owned(),
            ));
        }
    }
    if !caps.formats.depth.contains(&config.depth_format) {
        return Err(RendererError::Validation(
            "renderer depth_format is not supported by renderer caps".to_owned(),
        ));
    }
    Ok(())
}

fn validate_surface_backend_preference(backend: BackendPreference) -> Result<(), RendererError> {
    match backend {
        BackendPreference::Auto => Ok(()),
        BackendPreference::Wgpu if cfg!(feature = "backend-wgpu") => Ok(()),
        BackendPreference::Wgpu => Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        )),
        BackendPreference::Vulkan if cfg!(feature = "backend-wgpu") => Ok(()),
        BackendPreference::Metal if cfg!(feature = "backend-wgpu") => Ok(()),
        BackendPreference::D3d12 if cfg!(feature = "backend-wgpu") => Ok(()),
        BackendPreference::Headless => {
            Err(RendererError::UnsupportedFeature(RendererFeature::Surface))
        }
        BackendPreference::Vulkan => Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendVulkan,
        )),
        BackendPreference::Metal => Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendMetal,
        )),
        BackendPreference::D3d12 => Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendD3d12,
        )),
    }
}

fn invalid_handle_error<T>(handle: Handle<T>) -> RendererError {
    RendererError::InvalidHandle {
        kind: ResourceKind::from_tag(handle.kind_tag()).unwrap_or(ResourceKind::Environment),
        raw: handle.raw().get(),
    }
}

fn make_handle<T>(kind: ResourceKind, index: u32, generation: u32) -> Handle<T> {
    let raw =
        index as u64 | ((generation as u64 & 0x00ff_ffff) << 32) | ((kind.tag() as u64) << 56);
    Handle::from_raw(NonZeroU64::new(raw.max(1)).expect("handle encoding is non-zero"))
}

#[derive(Clone, Debug, PartialEq)]
struct StoredMesh {
    info: MeshInfo,
    vertex_layout: VertexLayout,
    submeshes: Vec<SubMeshDesc>,
    vertex_bytes: Vec<u8>,
    vertex_stream_bytes: Vec<Vec<u8>>,
    index_bytes: Vec<u8>,
    index_format: Option<StoredIndexFormat>,
    skin_inverse_bind_matrices: Option<Vec<Mat4>>,
    morph_targets: Vec<StoredMorphTarget>,
    meshlet_bytes: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredMorphTarget {
    positions: Option<Vec<Vec3>>,
    normals: Option<Vec<Vec3>>,
    tangents: Option<Vec<Vec3>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StoredIndexFormat {
    U16,
    U32,
}

impl StoredIndexFormat {
    const fn byte_size(self) -> usize {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct StoredTexture {
    desc: TextureDescOwned,
    bytes: Vec<u8>,
    layout: Option<StoredTextureDataLayout>,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredBuffer {
    label: Option<String>,
    size: u64,
    usage: BufferUsage,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextureDescOwned {
    label: Option<String>,
    dimension: TextureDimension,
    width: u32,
    height: u32,
    depth_or_layers: u32,
    mip_levels: u32,
    samples: u32,
    format: TextureFormat,
    usage: TextureUsage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredTextureDataLayout {
    subresource: TextureSubresource,
    region: TextureRegion,
    bytes_per_row: u32,
    rows_per_image: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredMaterial {
    label: Option<String>,
    domain: MaterialDomain,
    template: Option<MaterialTemplateHandle>,
    standard: Option<StandardMaterialDesc>,
    parameters: HashMap<String, MaterialParameterValue>,
    overrides: MaterialOverrides,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredShader {
    info: ShaderInfo,
    source: StoredShaderSource,
    reflection: ShaderReflectionMode,
    features: ShaderFeatureSet,
}

#[derive(Clone, Debug, PartialEq)]
enum StoredShaderSource {
    Wgsl(String),
    SpirV(Vec<u32>),
    Msl(String),
    Hlsl(String),
    Slang(String),
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
struct StoredEnvironment {
    desc: EnvironmentDesc,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredRenderTarget {
    desc: RenderTargetDesc,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredSkeletonInstance {
    label: Option<String>,
    joint_matrices: Vec<Mat4>,
    inverse_bind_matrices: Option<Vec<Mat4>>,
    usage: AnimationDataUsage,
}

#[derive(Clone, Debug, PartialEq)]
struct StoredMorphWeights {
    label: Option<String>,
    weights: Vec<f32>,
}

#[derive(Clone, Debug)]
struct StoredScene {
    desc: SceneDesc,
    objects: Arena<RenderObjectDesc>,
    lights: Arena<LightDesc>,
    environment: Option<EnvironmentHandle>,
}

#[derive(Clone, Debug)]
struct StoredView {
    desc: ViewDesc,
}

pub struct Renderer {
    config: RendererConfig,
    caps: RendererCaps,
    #[cfg(feature = "backend-wgpu")]
    wgpu_runtime: Option<WgpuRendererRuntime>,
    main_surface: Option<SurfaceHandle>,
    main_surface_priority: ResidencyPriority,
    surface_extent: Option<(u32, u32)>,
    meshes: Arena<StoredMesh>,
    buffers: Arena<StoredBuffer>,
    textures: Arena<StoredTexture>,
    samplers: Arena<SamplerDesc>,
    shaders: Arena<StoredShader>,
    materials: Arena<StoredMaterial>,
    material_templates: Arena<MaterialTemplateDesc>,
    builtin_standard_shader: Option<ShaderHandle>,
    builtin_standard_template: Option<MaterialTemplateHandle>,
    environments: Arena<StoredEnvironment>,
    render_targets: Arena<StoredRenderTarget>,
    lod_groups: Arena<LodGroupDesc>,
    cameras: Arena<CameraDesc>,
    graph_extensions: Arena<Arc<dyn RenderGraphExtension>>,
    skeleton_instances: Arena<StoredSkeletonInstance>,
    morph_weights: Arena<StoredMorphWeights>,
    scenes: Arena<StoredScene>,
    views: Arena<StoredView>,
    picking: Arena<PickingResult>,
    debug_draw_commands: Vec<DebugDrawCommand>,
    frame_index: u64,
    last_frame_stats: Option<FrameStats>,
    device_status: DeviceStatus,
    upload_stats: UploadStats,
    pipeline_cache_stats: PipelineCacheStats,
    pipeline_cache: HashSet<PipelineKey>,
    gpu_profiler_enabled: bool,
    capture_queued: Option<CaptureOptions>,
    capture_backend_hooks: HashSet<FrameCaptureBackend>,
    material_param_ids: HashMap<String, MaterialParamId>,
    material_param_names: Vec<String>,
}

impl Renderer {
    pub async fn new(config: RendererConfig) -> Result<Self, RendererError> {
        validate_renderer_config(&config)?;
        match config.backend {
            BackendPreference::Auto => Ok(Self::new_auto(config)),
            BackendPreference::Headless => Ok(Self::new_headless(config)),
            BackendPreference::Wgpu
            | BackendPreference::Vulkan
            | BackendPreference::Metal
            | BackendPreference::D3d12 => Self::new_wgpu(config),
        }
    }

    fn new_auto(config: RendererConfig) -> Self {
        #[cfg(feature = "backend-wgpu")]
        {
            if let Ok(renderer) = Self::new_wgpu(config.clone()) {
                return renderer;
            }
        }

        Self::new_headless(config)
    }

    #[cfg(feature = "backend-wgpu")]
    fn new_wgpu(config: RendererConfig) -> Result<Self, RendererError> {
        let runtime = WgpuRendererRuntime::new(config.clone())?;
        let mut renderer = Self::new_headless(config);
        renderer.caps = runtime.renderer_caps();
        renderer.wgpu_runtime = Some(runtime);
        Ok(renderer)
    }

    #[cfg(not(feature = "backend-wgpu"))]
    fn new_wgpu(_config: RendererConfig) -> Result<Self, RendererError> {
        Err(RendererError::UnsupportedFeature(
            RendererFeature::BackendWgpu,
        ))
    }

    #[cfg(feature = "backend-wgpu")]
    pub async fn with_surface(
        config: RendererConfig,
        window: &dyn engine_platform::PlatformWindow,
    ) -> Result<Self, RendererError> {
        validate_renderer_config(&config)?;
        validate_surface_backend_preference(config.backend)?;
        let runtime = WgpuRendererRuntime::with_surface(config.clone(), window)?;
        let mut renderer = Self::new_headless(config);
        let size = window.inner_size();
        renderer.caps = runtime.renderer_caps();
        renderer.wgpu_runtime = Some(runtime);
        renderer.main_surface = Some(make_handle(ResourceKind::Surface, 0, 1));
        renderer.surface_extent = Some((size.width, size.height));
        Ok(renderer)
    }

    #[cfg(not(feature = "backend-wgpu"))]
    pub async fn with_surface(
        config: RendererConfig,
        _window: &dyn engine_platform::PlatformWindow,
    ) -> Result<Self, RendererError> {
        validate_renderer_config(&config)?;
        validate_surface_backend_preference(config.backend)?;
        Err(RendererError::UnsupportedFeature(RendererFeature::Surface))
    }

    pub fn new_headless(config: RendererConfig) -> Self {
        let caps = RendererCaps::for_backend(&config, "headless", "retained-api");
        let gpu_profiler_enabled = config.gpu_profiling;
        Self {
            config,
            caps,
            #[cfg(feature = "backend-wgpu")]
            wgpu_runtime: None,
            main_surface: None,
            main_surface_priority: ResidencyPriority::Normal,
            surface_extent: None,
            meshes: Arena::default(),
            buffers: Arena::default(),
            textures: Arena::default(),
            samplers: Arena::default(),
            shaders: Arena::default(),
            materials: Arena::default(),
            material_templates: Arena::default(),
            builtin_standard_shader: None,
            builtin_standard_template: None,
            environments: Arena::default(),
            render_targets: Arena::default(),
            lod_groups: Arena::default(),
            cameras: Arena::default(),
            graph_extensions: Arena::default(),
            skeleton_instances: Arena::default(),
            morph_weights: Arena::default(),
            scenes: Arena::default(),
            views: Arena::default(),
            picking: Arena::default(),
            debug_draw_commands: Vec::new(),
            frame_index: 0,
            last_frame_stats: None,
            device_status: DeviceStatus::Ok,
            upload_stats: UploadStats::default(),
            pipeline_cache_stats: PipelineCacheStats::default(),
            pipeline_cache: HashSet::new(),
            gpu_profiler_enabled,
            capture_queued: None,
            capture_backend_hooks: HashSet::new(),
            material_param_ids: HashMap::new(),
            material_param_names: Vec::new(),
        }
    }

    pub fn capabilities(&self) -> &RendererCaps {
        &self.caps
    }

    pub fn supports_feature(&self, feature: RendererFeature) -> bool {
        match feature {
            RendererFeature::BackendWgpu => cfg!(feature = "backend-wgpu"),
            RendererFeature::BackendVulkan => false,
            RendererFeature::BackendMetal => false,
            RendererFeature::BackendD3d12 => false,
            RendererFeature::Compute => self.caps.features.contains(RendererFeatures::COMPUTE),
            RendererFeature::IndirectDraw => {
                self.caps.features.contains(RendererFeatures::INDIRECT_DRAW)
            }
            RendererFeature::MultiDrawIndirect => self
                .caps
                .features
                .contains(RendererFeatures::MULTI_DRAW_INDIRECT),
            RendererFeature::BindlessTextures => self
                .caps
                .features
                .contains(RendererFeatures::BINDLESS_TEXTURES),
            RendererFeature::StorageTextures => self
                .caps
                .features
                .contains(RendererFeatures::STORAGE_TEXTURES),
            RendererFeature::TimestampQuery => self
                .caps
                .features
                .contains(RendererFeatures::TIMESTAMP_QUERY),
            RendererFeature::PipelineStatistics => self
                .caps
                .features
                .contains(RendererFeatures::PIPELINE_STATISTICS),
            RendererFeature::AsyncCompute => {
                self.caps.features.contains(RendererFeatures::ASYNC_COMPUTE)
            }
            RendererFeature::RayTracing => {
                self.caps.features.contains(RendererFeatures::RAY_TRACING)
            }
            RendererFeature::MeshShader => {
                self.caps.features.contains(RendererFeatures::MESH_SHADER)
            }
            RendererFeature::VariableRateShading => self
                .caps
                .features
                .contains(RendererFeatures::VARIABLE_RATE_SHADING),
            RendererFeature::GpuDrivenRendering => self
                .caps
                .features
                .contains(RendererFeatures::GPU_DRIVEN_RENDERING),
            RendererFeature::OcclusionCulling => self
                .caps
                .features
                .contains(RendererFeatures::OCCLUSION_CULLING),
            RendererFeature::VirtualTexturing => self
                .caps
                .features
                .contains(RendererFeatures::VIRTUAL_TEXTURING),
            RendererFeature::ShaderReflection => true,
            RendererFeature::Surface => {
                #[cfg(feature = "backend-wgpu")]
                {
                    self.wgpu_runtime.is_some()
                }
                #[cfg(not(feature = "backend-wgpu"))]
                {
                    false
                }
            }
        }
    }

    pub fn config(&self) -> &RendererConfig {
        &self.config
    }

    pub fn main_surface(&self) -> Option<SurfaceHandle> {
        self.main_surface
    }

    fn is_main_surface(&self, surface: SurfaceHandle) -> bool {
        self.main_surface == Some(surface)
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        if width == 0 || height == 0 {
            return Err(RendererError::Validation(
                "surface size must be non-zero".to_owned(),
            ));
        }

        #[cfg(feature = "backend-wgpu")]
        if let Some(runtime) = &mut self.wgpu_runtime {
            runtime.resize_surface(width, height)?;
            self.surface_extent = Some((width, height));
            return Ok(());
        }

        self.surface_extent = Some((width, height));
        Ok(())
    }

    pub fn set_vsync(&mut self, mode: VSyncMode) -> Result<(), RendererError> {
        #[cfg(feature = "backend-wgpu")]
        if let Some(runtime) = &mut self.wgpu_runtime {
            runtime.set_vsync(mode)?;
        }
        self.config.vsync = mode;
        Ok(())
    }

    pub fn device_status(&self) -> DeviceStatus {
        #[cfg(feature = "backend-wgpu")]
        if let Some(runtime) = &self.wgpu_runtime {
            if runtime.device_status() == DeviceStatus::Lost {
                return DeviceStatus::Lost;
            }
        }
        self.device_status
    }

    #[cfg(feature = "backend-wgpu")]
    pub fn render_legacy_scene(
        &mut self,
        scene: &engine_render::RenderScene,
    ) -> Result<FrameStats, RendererError> {
        let Some(runtime) = &mut self.wgpu_runtime else {
            return Err(RendererError::Validation(
                "renderer was not initialized with a wgpu surface".to_owned(),
            ));
        };
        let stats = match runtime.render_scene(scene) {
            Ok(stats) => stats,
            Err(error) => {
                if matches!(error, RendererError::DeviceLost { .. }) {
                    self.device_status = DeviceStatus::Lost;
                }
                return Err(error);
            }
        };
        self.last_frame_stats = Some(stats.clone());
        Ok(stats)
    }

    #[cfg(feature = "backend-wgpu")]
    fn render_facade_view(&mut self, view: &ViewDesc) -> Result<FrameStats, RendererError> {
        if !matches!(view.target, RenderTarget::MainSurface) {
            return Err(RendererError::Validation(
                "wgpu facade rendering currently requires RenderTarget::MainSurface".to_owned(),
            ));
        }
        let scene = self.build_legacy_scene(view)?;
        self.render_legacy_scene(&scene)
    }

    pub fn create_mesh(&mut self, desc: MeshDesc<'_>) -> Result<MeshHandle, RendererError> {
        if desc.submeshes.is_empty() {
            return Err(RendererError::Validation(
                "mesh must contain at least one submesh".to_owned(),
            ));
        }
        validate_bounds(desc.bounds)?;
        let vertex_stream_bytes = match desc.vertices {
            VertexData::Interleaved(bytes) => vec![bytes.to_vec()],
            VertexData::Streams(streams) => streams
                .into_iter()
                .map(|stream| stream.data.to_vec())
                .collect::<Vec<_>>(),
        };
        let vertex_bytes = vertex_stream_bytes.concat();
        let (index_bytes, index_count, index_format) = match desc.indices {
            Some(IndexData::U16(indices)) => (
                indices
                    .iter()
                    .flat_map(|index| index.to_le_bytes())
                    .collect(),
                indices.len() as u32,
                Some(StoredIndexFormat::U16),
            ),
            Some(IndexData::U32(indices)) => (
                indices
                    .iter()
                    .flat_map(|index| index.to_le_bytes())
                    .collect(),
                indices.len() as u32,
                Some(StoredIndexFormat::U32),
            ),
            None => (Vec::new(), 0, None),
        };
        validate_mesh_submeshes(
            &desc.submeshes,
            index_count,
            &desc.vertex_layout,
            &vertex_stream_bytes,
        )?;
        let vertex_count = mesh_vertex_count(&desc.vertex_layout, &vertex_stream_bytes)?;
        validate_mesh_deformation_data(
            desc.skin.as_ref(),
            &desc.morph_targets,
            desc.meshlets.as_ref(),
            vertex_count,
        )?;
        let skin_inverse_bind_matrices = desc
            .skin
            .as_ref()
            .map(|skin| skin.inverse_bind_matrices.to_vec());
        let morph_targets = desc
            .morph_targets
            .iter()
            .map(|target| StoredMorphTarget {
                positions: target.positions.map(<[Vec3]>::to_vec),
                normals: target.normals.map(<[Vec3]>::to_vec),
                tangents: target.tangents.map(<[Vec3]>::to_vec),
            })
            .collect::<Vec<_>>();
        let meshlet_bytes = desc
            .meshlets
            .as_ref()
            .map(|meshlets| meshlets.bytes.to_vec());
        let info = MeshInfo {
            label: desc.label.map(str::to_owned),
            vertex_bytes: vertex_bytes.len(),
            index_count,
            submesh_count: desc.submeshes.len(),
            skin_joint_count: skin_inverse_bind_matrices.as_ref().map_or(0, Vec::len),
            morph_target_count: morph_targets.len(),
            meshlet_bytes: meshlet_bytes.as_ref().map_or(0, Vec::len),
            bounds: desc.bounds,
            usage: desc.usage,
            flags: desc.flags,
            status: ResourceStatus::Ready,
        };
        self.queue_upload_bytes(vertex_bytes.len() as u64 + index_bytes.len() as u64);
        Ok(self.meshes.insert(
            ResourceKind::Mesh,
            StoredMesh {
                info,
                vertex_layout: desc.vertex_layout,
                submeshes: desc.submeshes,
                vertex_bytes,
                vertex_stream_bytes,
                index_bytes,
                index_format,
                skin_inverse_bind_matrices,
                morph_targets,
                meshlet_bytes,
            },
        ))
    }

    pub fn update_mesh_vertices(
        &mut self,
        mesh: MeshHandle,
        stream: u32,
        byte_offset: u64,
        data: &[u8],
    ) -> Result<(), RendererError> {
        let Some(slot) = self.meshes.get_mut(ResourceKind::Mesh, mesh) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            });
        };
        let mesh = slot.value.as_mut().expect("arena slot is occupied");
        let Some(stream_bytes) = mesh.vertex_stream_bytes.get_mut(stream as usize) else {
            return Err(RendererError::Validation(format!(
                "mesh vertex stream does not exist: {stream}"
            )));
        };
        write_range(stream_bytes, byte_offset, data)?;
        mesh.vertex_bytes = mesh.vertex_stream_bytes.concat();
        mesh.info.vertex_bytes = mesh.vertex_bytes.len();
        self.queue_upload_bytes(data.len() as u64);
        Ok(())
    }

    pub fn update_mesh_indices(
        &mut self,
        mesh: MeshHandle,
        byte_offset: u64,
        data: &[u8],
    ) -> Result<(), RendererError> {
        let Some(slot) = self.meshes.get_mut(ResourceKind::Mesh, mesh) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            });
        };
        let mesh = slot.value.as_mut().expect("arena slot is occupied");
        let Some(index_format) = mesh.index_format else {
            return Err(RendererError::Validation(
                "mesh has no index buffer to update".to_owned(),
            ));
        };
        let index_size = index_format.byte_size();
        if byte_offset % index_size as u64 != 0 || data.len() % index_size != 0 {
            return Err(RendererError::Validation(
                "mesh index updates must be aligned to index size".to_owned(),
            ));
        }
        write_range(&mut mesh.index_bytes, byte_offset, data)?;
        mesh.info.index_count = (mesh.index_bytes.len() / index_size) as u32;
        self.queue_upload_bytes(data.len() as u64);
        Ok(())
    }

    pub fn mesh_info(&self, mesh: MeshHandle) -> Option<MeshInfo> {
        self.meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
            .map(|mesh| mesh.info.clone())
    }

    pub fn create_buffer(&mut self, desc: BufferDesc<'_>) -> Result<BufferHandle, RendererError> {
        validate_buffer_desc(&desc)?;
        let size = usize::try_from(desc.size).map_err(|_| {
            RendererError::Validation("buffer size exceeds addressable memory".to_owned())
        })?;
        let mut bytes = vec![0; size];
        if let Some(initial_data) = desc.initial_data {
            bytes[..initial_data.len()].copy_from_slice(initial_data);
            self.queue_upload_bytes(initial_data.len() as u64);
        }
        Ok(self.buffers.insert(
            ResourceKind::Buffer,
            StoredBuffer {
                label: desc.label.map(str::to_owned),
                size: desc.size,
                usage: desc.usage,
                bytes,
            },
        ))
    }

    pub fn update_buffer(
        &mut self,
        buffer: BufferHandle,
        update: BufferUpdate<'_>,
    ) -> Result<(), RendererError> {
        let Some(slot) = self.buffers.get_mut(ResourceKind::Buffer, buffer) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Buffer,
                raw: buffer.raw().get(),
            });
        };
        let buffer = slot.value.as_mut().expect("arena slot is occupied");
        if !buffer.usage.contains(BufferUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "buffer updates require COPY_DST usage".to_owned(),
            ));
        }
        validate_buffer_update(buffer.size, &update)?;
        let offset = usize::try_from(update.byte_offset).map_err(|_| {
            RendererError::Validation("buffer update byte offset is too large".to_owned())
        })?;
        let end = offset
            .checked_add(update.data.len())
            .ok_or_else(|| RendererError::Validation("buffer update range overflows".to_owned()))?;
        buffer.bytes[offset..end].copy_from_slice(update.data);
        self.queue_upload_bytes(update.data.len() as u64);
        Ok(())
    }

    pub fn buffer_info(&self, buffer: BufferHandle) -> Option<BufferInfo> {
        self.buffers
            .get(ResourceKind::Buffer, buffer)
            .and_then(|slot| {
                slot.value.as_ref().map(|buffer| BufferInfo {
                    label: buffer.label.clone(),
                    size: buffer.size,
                    usage: buffer.usage,
                    status: slot.status,
                })
            })
    }

    pub fn buffer_bytes(&self, buffer: BufferHandle) -> Option<&[u8]> {
        self.buffers
            .get(ResourceKind::Buffer, buffer)
            .and_then(|slot| slot.value.as_ref())
            .map(|buffer| buffer.bytes.as_slice())
    }

    pub fn create_texture(
        &mut self,
        desc: TextureDesc<'_>,
    ) -> Result<TextureHandle, RendererError> {
        if desc.width == 0 || desc.height == 0 || desc.depth_or_layers == 0 {
            return Err(RendererError::Validation(
                "texture extent must be non-zero".to_owned(),
            ));
        }
        if desc.mip_levels == 0 || desc.samples == 0 {
            return Err(RendererError::Validation(
                "texture mip_levels and samples must be non-zero".to_owned(),
            ));
        }
        if desc.usage == TextureUsage::empty() {
            return Err(RendererError::Validation(
                "texture usage must not be empty".to_owned(),
            ));
        }
        validate_texture_dimension(&desc)?;
        if let Some(initial) = &desc.initial_data {
            validate_texture_layout(
                initial.bytes.len(),
                initial.bytes_per_row,
                initial.rows_per_image,
                desc.width,
                desc.height,
                desc.depth_or_layers,
                desc.format,
            )?;
        }
        let bytes = desc
            .initial_data
            .as_ref()
            .map_or_else(Vec::new, |initial| initial.bytes.to_vec());
        let layout = desc
            .initial_data
            .as_ref()
            .map(|initial| StoredTextureDataLayout {
                subresource: TextureSubresource {
                    mip_level: 0,
                    array_layer: 0,
                },
                region: TextureRegion {
                    offset: [0, 0, 0],
                    extent: [desc.width, desc.height, desc.depth_or_layers],
                },
                bytes_per_row: initial.bytes_per_row,
                rows_per_image: initial.rows_per_image,
            });
        self.queue_upload_bytes(bytes.len() as u64);
        Ok(self.textures.insert(
            ResourceKind::Texture,
            StoredTexture {
                desc: TextureDescOwned {
                    label: desc.label.map(str::to_owned),
                    dimension: desc.dimension,
                    width: desc.width,
                    height: desc.height,
                    depth_or_layers: desc.depth_or_layers,
                    mip_levels: desc.mip_levels,
                    samples: desc.samples,
                    format: desc.format,
                    usage: desc.usage,
                },
                bytes,
                layout,
            },
        ))
    }

    pub fn update_texture(
        &mut self,
        texture: TextureHandle,
        update: TextureUpdate<'_>,
    ) -> Result<(), RendererError> {
        let Some(slot) = self.textures.get_mut(ResourceKind::Texture, texture) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Texture,
                raw: texture.raw().get(),
            });
        };
        let texture = slot.value.as_mut().expect("arena slot is occupied");
        if !texture.desc.usage.contains(TextureUsage::COPY_DST) {
            return Err(RendererError::Validation(
                "texture updates require COPY_DST usage".to_owned(),
            ));
        }
        validate_texture_update_region(&texture.desc, &update)?;
        texture.bytes.clear();
        texture.bytes.extend_from_slice(update.data);
        texture.layout = Some(StoredTextureDataLayout {
            subresource: update.subresource,
            region: update.region,
            bytes_per_row: update.bytes_per_row,
            rows_per_image: update.rows_per_image,
        });
        self.queue_upload_bytes(update.data.len() as u64);
        Ok(())
    }

    pub fn texture_info(&self, texture: TextureHandle) -> Option<TextureInfo> {
        self.textures
            .get(ResourceKind::Texture, texture)
            .and_then(|slot| {
                slot.value.as_ref().map(|texture| TextureInfo {
                    label: texture.desc.label.clone(),
                    dimension: texture.desc.dimension,
                    width: texture.desc.width,
                    height: texture.desc.height,
                    depth_or_layers: texture.desc.depth_or_layers,
                    mip_levels: texture.desc.mip_levels,
                    samples: texture.desc.samples,
                    format: texture.desc.format,
                    usage: texture.desc.usage,
                    status: slot.status,
                })
            })
    }

    pub fn texture_bytes(&self, texture: TextureHandle) -> Option<&[u8]> {
        self.textures
            .get(ResourceKind::Texture, texture)
            .and_then(|slot| slot.value.as_ref())
            .map(|texture| texture.bytes.as_slice())
    }

    pub fn generate_mips(&mut self, texture: TextureHandle) -> Result<(), RendererError> {
        let upload_bytes = {
            let Some(slot) = self.textures.get_mut(ResourceKind::Texture, texture) else {
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Texture,
                    raw: texture.raw().get(),
                });
            };
            let texture = slot.value.as_mut().expect("arena slot is occupied");
            let mips = generate_texture_mip_chain(texture)?;
            texture.desc.mip_levels = u32::try_from(mips.len()).map_err(|_| {
                RendererError::Validation("generated texture mip count overflows u32".to_owned())
            })?;
            texture.bytes = mips.into_iter().flatten().collect();
            texture.layout = None;
            texture.bytes.len() as u64
        };
        self.queue_upload_bytes(upload_bytes);
        Ok(())
    }

    pub fn create_sampler(&mut self, desc: SamplerDesc) -> Result<SamplerHandle, RendererError> {
        validate_sampler_desc(&desc)?;
        Ok(self.samplers.insert(ResourceKind::Sampler, desc))
    }

    pub fn create_shader(&mut self, desc: ShaderDesc<'_>) -> Result<ShaderHandle, RendererError> {
        validate_shader_desc(&desc)?;
        let interface = shader_interface_from_desc(&desc)?;
        let source = stored_shader_source(&desc.source);
        let info = ShaderInfo {
            label: desc.label.map(str::to_owned),
            stages: desc.stages,
            entry_points: ShaderEntryPointInfo {
                vertex: desc.entry_points.vertex.map(str::to_owned),
                fragment: desc.entry_points.fragment.map(str::to_owned),
                compute: desc.entry_points.compute.map(str::to_owned),
            },
            hot_reload_key: shader_hot_reload_key(&desc),
            status: ResourceStatus::Ready,
            interface,
        };
        Ok(self.shaders.insert(
            ResourceKind::Shader,
            StoredShader {
                info,
                source,
                reflection: desc.reflection.clone(),
                features: desc.features.clone(),
            },
        ))
    }

    pub fn reload_shader(&mut self, shader: ShaderHandle) -> Result<(), RendererError> {
        let Some(stored) = self
            .shaders
            .get(ResourceKind::Shader, shader)
            .and_then(|slot| slot.value.as_ref())
            .cloned()
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Shader,
                raw: shader.raw().get(),
            });
        };
        let StoredShaderSource::File(path) = &stored.source else {
            return Err(RendererError::Validation(
                "shader reload requires a file source; use reload_shader_from_desc for in-memory sources"
                    .to_owned(),
            ));
        };
        let path = path.clone();
        let source = fs::read_to_string(&path).map_err(|err| {
            RendererError::ShaderCompile(format!("failed to read shader file {path:?}: {err}"))
        })?;
        let reload = ShaderReloadDesc {
            source: ShaderSource::Wgsl(&source),
            entry_points: ShaderEntryPoints {
                vertex: stored.info.entry_points.vertex.as_deref(),
                fragment: stored.info.entry_points.fragment.as_deref(),
                compute: stored.info.entry_points.compute.as_deref(),
            },
            reflection: stored.reflection,
            features: stored.features,
        };
        self.reload_shader_from_desc(shader, reload)?;
        if let Some(slot) = self.shaders.get_mut(ResourceKind::Shader, shader) {
            if let Some(stored) = slot.value.as_mut() {
                stored.source = StoredShaderSource::File(path);
            }
        }
        Ok(())
    }

    pub fn reload_shader_from_desc(
        &mut self,
        shader: ShaderHandle,
        desc: ShaderReloadDesc<'_>,
    ) -> Result<(), RendererError> {
        if !self.config.shader_hot_reload {
            return Err(RendererError::Validation(
                "shader hot reload is disabled in RendererConfig".to_owned(),
            ));
        }
        {
            let Some(slot) = self.shaders.get_mut(ResourceKind::Shader, shader) else {
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Shader,
                    raw: shader.raw().get(),
                });
            };
            let stored = slot.value.as_mut().expect("arena slot is occupied");
            let hot_reload_key = stored.info.hot_reload_key.clone();
            if hot_reload_key.is_none() {
                return Err(RendererError::Validation(
                    "shader must have a hot_reload_key to be reloaded".to_owned(),
                ));
            }
            let new_source = stored_shader_source(&desc.source);
            let new_entry_points = ShaderEntryPointInfo {
                vertex: desc.entry_points.vertex.map(str::to_owned),
                fragment: desc.entry_points.fragment.map(str::to_owned),
                compute: desc.entry_points.compute.map(str::to_owned),
            };
            let label = stored.info.label.clone();
            let stages = stored.info.stages;
            let reload_desc = ShaderDesc {
                label: label.as_deref(),
                source: desc.source.clone(),
                stages,
                entry_points: desc.entry_points.clone(),
                reflection: desc.reflection.clone(),
                features: desc.features.clone(),
                hot_reload_key: hot_reload_key.clone(),
            };
            validate_shader_desc(&reload_desc)?;
            let interface = shader_interface_from_desc(&reload_desc)?;
            validate_shader_reload_compatible(&stored.info.interface, &interface)?;
            stored.info.entry_points = new_entry_points;
            stored.info.interface = interface;
            stored.info.status = ResourceStatus::Ready;
            stored.source = new_source;
            stored.reflection = desc.reflection;
            stored.features = desc.features;
        }
        self.invalidate_shader_pipelines(shader);
        Ok(())
    }

    pub fn shader_info(&self, shader: ShaderHandle) -> Option<ShaderInfo> {
        self.shaders
            .get(ResourceKind::Shader, shader)
            .and_then(|slot| slot.value.as_ref())
            .map(|shader| shader.info.clone())
    }

    pub fn shader_interface(&self, shader: ShaderHandle) -> Option<&ShaderInterfaceDesc> {
        self.shaders
            .get(ResourceKind::Shader, shader)
            .and_then(|slot| slot.value.as_ref())
            .map(|shader| &shader.info.interface)
    }

    pub fn create_standard_material(
        &mut self,
        desc: StandardMaterialDesc,
    ) -> Result<MaterialHandle, RendererError> {
        self.validate_standard_material_desc(&desc)?;
        let template = self.ensure_builtin_standard_material_template()?;
        Ok(self.materials.insert(
            ResourceKind::Material,
            StoredMaterial {
                label: desc.label.clone(),
                domain: desc.domain,
                template: Some(template),
                standard: Some(desc),
                parameters: HashMap::new(),
                overrides: MaterialOverrides::default(),
            },
        ))
    }

    fn ensure_builtin_standard_material_template(
        &mut self,
    ) -> Result<MaterialTemplateHandle, RendererError> {
        if let Some(template) = self.builtin_standard_template {
            return Ok(template);
        }
        let shader = if let Some(shader) = self.builtin_standard_shader {
            shader
        } else {
            const STANDARD_PBR_WGSL: &str = r#"
@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#;
            let shader = self.create_shader(ShaderDesc {
                label: Some("builtin_standard_pbr"),
                source: ShaderSource::Wgsl(STANDARD_PBR_WGSL),
                stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs_main"),
                    fragment: Some("fs_main"),
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })?;
            self.builtin_standard_shader = Some(shader);
            shader
        };
        let template = self.create_material_template(MaterialTemplateDesc {
            label: Some("builtin_standard_pbr".to_owned()),
            shader,
            domain: MaterialDomain::Opaque,
            render_state: RenderStateDesc { depth_write: true },
            parameter_schema: MaterialParameterSchema::default(),
            passes: standard_material_builtin_passes(),
        })?;
        self.builtin_standard_template = Some(template);
        Ok(template)
    }

    fn validate_standard_material_desc(
        &self,
        desc: &StandardMaterialDesc,
    ) -> Result<(), RendererError> {
        if !desc.metallic.is_finite() || !(0.0..=1.0).contains(&desc.metallic) {
            return Err(RendererError::Validation(
                "standard material metallic must be finite and in 0..=1".to_owned(),
            ));
        }
        if !desc.roughness.is_finite() || !(0.0..=1.0).contains(&desc.roughness) {
            return Err(RendererError::Validation(
                "standard material roughness must be finite and in 0..=1".to_owned(),
            ));
        }
        if !desc.emissive.x.is_finite()
            || !desc.emissive.y.is_finite()
            || !desc.emissive.z.is_finite()
        {
            return Err(RendererError::Validation(
                "standard material emissive must be finite".to_owned(),
            ));
        }
        if let AlphaMode::Mask { cutoff } = desc.alpha_mode {
            if !cutoff.is_finite() || !(0.0..=1.0).contains(&cutoff) {
                return Err(RendererError::Validation(
                    "standard material alpha cutoff must be finite and in 0..=1".to_owned(),
                ));
            }
        }
        for texture in [
            desc.base_color_texture,
            desc.normal_texture,
            desc.metallic_roughness_texture,
            desc.occlusion_texture,
            desc.emissive_texture,
        ]
        .into_iter()
        .flatten()
        {
            let texture_info = self.validated_texture_desc(
                texture,
                TextureUsage::SAMPLED,
                "standard material texture",
            )?;
            if !matches!(
                texture_info.dimension,
                TextureDimension::D2 | TextureDimension::D2Array
            ) {
                return Err(RendererError::Validation(
                    "standard material textures must be 2D or 2D array textures".to_owned(),
                ));
            }
            if texture_info.samples != 1 {
                return Err(RendererError::Validation(
                    "standard material textures must not be multisampled".to_owned(),
                ));
            }
        }
        Ok(())
    }

    pub fn create_material(&mut self, desc: MaterialDesc) -> Result<MaterialHandle, RendererError> {
        let template_desc = self
            .material_templates
            .get(ResourceKind::MaterialTemplate, desc.template)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::MaterialTemplate,
                raw: desc.template.raw().get(),
            })?;
        validate_material_parameters(&template_desc.parameter_schema, &desc.parameters)?;
        self.validate_material_parameter_bindings(template_desc, &desc.parameters)?;
        for parameter in &desc.parameters {
            self.validate_material_parameter_value(&parameter.value)?;
        }
        let parameters = desc
            .parameters
            .into_iter()
            .map(|parameter| (parameter.name, parameter.value))
            .collect();
        Ok(self.materials.insert(
            ResourceKind::Material,
            StoredMaterial {
                label: desc.label.or_else(|| template_desc.label.clone()),
                domain: template_desc.domain,
                template: Some(desc.template),
                standard: None,
                parameters,
                overrides: desc.overrides,
            },
        ))
    }

    pub fn update_material_parameters(
        &mut self,
        material: MaterialHandle,
        parameters: &[MaterialParameter],
    ) -> Result<(), RendererError> {
        let template = self
            .materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                raw: material.raw().get(),
            })?
            .template;
        if let Some(template) = template {
            let template_desc = self
                .material_templates
                .get(ResourceKind::MaterialTemplate, template)
                .and_then(|slot| slot.value.as_ref())
                .ok_or(RendererError::InvalidHandle {
                    kind: ResourceKind::MaterialTemplate,
                    raw: template.raw().get(),
                })?;
            validate_material_parameters(&template_desc.parameter_schema, parameters)?;
            self.validate_material_parameter_bindings(template_desc, parameters)?;
        }
        for parameter in parameters {
            self.validate_material_parameter_value(&parameter.value)?;
        }
        let material = self
            .materials
            .get_mut(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_mut())
            .expect("material was validated before mutable lookup");
        for parameter in parameters {
            material
                .parameters
                .insert(parameter.name.clone(), parameter.value.clone());
        }
        Ok(())
    }

    pub fn update_material(
        &mut self,
        material: MaterialHandle,
        update: MaterialUpdate,
    ) -> Result<(), RendererError> {
        match update {
            MaterialUpdate::SetBool(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Bool(value),
                }],
            ),
            MaterialUpdate::SetI32(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::I32(value),
                }],
            ),
            MaterialUpdate::SetU32(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::U32(value),
                }],
            ),
            MaterialUpdate::SetFloat(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::F32(value),
                }],
            ),
            MaterialUpdate::SetVec2(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Vec2(value),
                }],
            ),
            MaterialUpdate::SetVec3(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Vec3(value),
                }],
            ),
            MaterialUpdate::SetVec4(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Vec4(value),
                }],
            ),
            MaterialUpdate::SetMat4(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Mat4(value),
                }],
            ),
            MaterialUpdate::SetColor(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Color(value),
                }],
            ),
            MaterialUpdate::SetTexture(name, Some(texture)) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Texture(texture),
                }],
            ),
            MaterialUpdate::SetSampler(name, Some(sampler)) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Sampler(sampler),
                }],
            ),
            MaterialUpdate::SetTexture(name, None) | MaterialUpdate::SetSampler(name, None) => {
                self.remove_material_parameter(material, &name)
            }
            MaterialUpdate::SetBytes(name, value) => self.update_material_parameters(
                material,
                &[MaterialParameter {
                    name,
                    value: MaterialParameterValue::Bytes(value),
                }],
            ),
            MaterialUpdate::ReplaceAll(parameters) => {
                self.replace_material_parameters(material, parameters)
            }
        }
    }

    pub fn intern_material_param(&mut self, name: &str) -> MaterialParamId {
        if let Some(id) = self.material_param_ids.get(name) {
            return *id;
        }
        let id = MaterialParamId(self.material_param_names.len() as u32);
        self.material_param_names.push(name.to_owned());
        self.material_param_ids.insert(name.to_owned(), id);
        id
    }

    pub fn update_material_fast(
        &mut self,
        material: MaterialHandle,
        param: MaterialParamId,
        value: MaterialValue,
    ) -> Result<(), RendererError> {
        let Some(name) = self.material_param_names.get(param.0 as usize).cloned() else {
            return Err(RendererError::Validation(
                "material parameter id is not interned".to_owned(),
            ));
        };
        self.update_material_parameters(material, &[MaterialParameter { name, value }])
    }

    pub fn material_parameter(
        &self,
        material: MaterialHandle,
        name: &str,
    ) -> Option<&MaterialParameterValue> {
        self.materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .and_then(|material| material.parameters.get(name))
    }

    fn validate_material_parameter_value(
        &self,
        value: &MaterialParameterValue,
    ) -> Result<(), RendererError> {
        match value {
            MaterialParameterValue::Texture(texture) => self
                .validated_texture_desc(
                    *texture,
                    TextureUsage::SAMPLED,
                    "material texture parameter",
                )
                .map(|_| ()),
            MaterialParameterValue::Sampler(sampler)
                if self.samplers.get(ResourceKind::Sampler, *sampler).is_none() =>
            {
                Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Sampler,
                    raw: sampler.raw().get(),
                })
            }
            _ => Ok(()),
        }
    }

    fn remove_material_parameter(
        &mut self,
        material: MaterialHandle,
        name: &str,
    ) -> Result<(), RendererError> {
        self.validate_material_parameter_names(material, &[name])?;
        let material = self
            .materials
            .get_mut(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_mut())
            .expect("material was validated before mutable lookup");
        material.parameters.remove(name);
        Ok(())
    }

    fn replace_material_parameters(
        &mut self,
        material: MaterialHandle,
        parameters: Vec<MaterialParameter>,
    ) -> Result<(), RendererError> {
        let names = parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        self.validate_material_parameter_names(material, &names)?;
        self.validate_material_update_bindings(material, &parameters)?;
        for parameter in &parameters {
            self.validate_material_parameter_value(&parameter.value)?;
        }
        let material = self
            .materials
            .get_mut(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_mut())
            .expect("material was validated before mutable lookup");
        material.parameters.clear();
        material.parameters.extend(
            parameters
                .into_iter()
                .map(|parameter| (parameter.name, parameter.value)),
        );
        Ok(())
    }

    fn validate_material_parameter_names(
        &self,
        material: MaterialHandle,
        names: &[&str],
    ) -> Result<(), RendererError> {
        validate_material_parameter_name_set(names)?;
        let template = self
            .materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                raw: material.raw().get(),
            })?
            .template;
        if let Some(template) = template {
            let schema = &self
                .material_templates
                .get(ResourceKind::MaterialTemplate, template)
                .and_then(|slot| slot.value.as_ref())
                .ok_or(RendererError::InvalidHandle {
                    kind: ResourceKind::MaterialTemplate,
                    raw: template.raw().get(),
                })?
                .parameter_schema;
            if !schema.parameters.is_empty() {
                for name in names {
                    if !schema
                        .parameters
                        .iter()
                        .any(|allowed| allowed.as_str() == *name)
                    {
                        return Err(RendererError::MaterialParameterMismatch(format!(
                            "material parameter '{name}' is not declared in template schema"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_material_update_bindings(
        &self,
        material: MaterialHandle,
        parameters: &[MaterialParameter],
    ) -> Result<(), RendererError> {
        let template = self
            .materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                raw: material.raw().get(),
            })?
            .template;
        let Some(template) = template else {
            return Ok(());
        };
        let template_desc = self
            .material_templates
            .get(ResourceKind::MaterialTemplate, template)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::MaterialTemplate,
                raw: template.raw().get(),
            })?;
        self.validate_material_parameter_bindings(template_desc, parameters)
    }

    fn validate_material_parameter_bindings(
        &self,
        template: &MaterialTemplateDesc,
        parameters: &[MaterialParameter],
    ) -> Result<(), RendererError> {
        let Some(shader) = self
            .shaders
            .get(ResourceKind::Shader, template.shader)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Shader,
                raw: template.shader.raw().get(),
            });
        };
        validate_material_parameter_bindings(&shader.info.interface, parameters)?;
        self.validate_material_texture_parameter_dimensions(&shader.info.interface, parameters)
    }

    fn validate_material_texture_parameter_dimensions(
        &self,
        interface: &ShaderInterfaceDesc,
        parameters: &[MaterialParameter],
    ) -> Result<(), RendererError> {
        for parameter in parameters {
            let Some(binding) = interface
                .resources
                .iter()
                .find(|binding| binding.name == parameter.name)
            else {
                continue;
            };
            let BindingType::Texture(expected_dimension) = binding.ty else {
                continue;
            };
            let MaterialParameterValue::Texture(texture) = parameter.value else {
                continue;
            };
            let texture = self.validated_texture_desc(
                texture,
                TextureUsage::SAMPLED,
                "material texture parameter",
            )?;
            if texture.dimension != expected_dimension {
                return Err(RendererError::MaterialParameterMismatch(format!(
                    "material texture parameter '{}' has dimension {:?}, but shader binding expects {:?}",
                    parameter.name, texture.dimension, expected_dimension
                )));
            }
        }
        Ok(())
    }

    pub fn create_material_template(
        &mut self,
        desc: MaterialTemplateDesc,
    ) -> Result<MaterialTemplateHandle, RendererError> {
        validate_material_template_desc(&desc)?;
        let shader_info = self
            .shaders
            .get(ResourceKind::Shader, desc.shader)
            .and_then(|slot| slot.value.as_ref())
            .map(|shader| &shader.info)
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Shader,
                raw: desc.shader.raw().get(),
            })?;
        validate_material_template_shader(&desc, shader_info)?;
        Ok(self
            .material_templates
            .insert(ResourceKind::MaterialTemplate, desc))
    }

    pub fn create_environment(
        &mut self,
        desc: EnvironmentDesc,
    ) -> Result<EnvironmentHandle, RendererError> {
        self.validate_environment_desc(&desc)?;
        Ok(self
            .environments
            .insert(ResourceKind::Environment, StoredEnvironment { desc }))
    }

    pub fn environment_desc(&self, environment: EnvironmentHandle) -> Option<&EnvironmentDesc> {
        self.environments
            .get(ResourceKind::Environment, environment)
            .and_then(|slot| slot.value.as_ref())
            .map(|environment| &environment.desc)
    }

    pub fn bake_environment(
        &mut self,
        source: TextureHandle,
        desc: EnvironmentBakeDesc,
    ) -> Result<EnvironmentHandle, RendererError> {
        let Some(source_texture) = self.textures.get(ResourceKind::Texture, source) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Texture,
                raw: source.raw().get(),
            });
        };
        let source_texture = source_texture
            .value
            .as_ref()
            .expect("arena slot is occupied");
        let source_desc = source_texture.desc.clone();
        if source_desc.usage.0 & TextureUsage::SAMPLED.0 == 0 {
            return Err(RendererError::Validation(
                "environment bake source texture must be sampleable".to_owned(),
            ));
        }
        if desc.resolution == 0 || desc.mip_levels == 0 {
            return Err(RendererError::Validation(
                "environment bake resolution and mip_levels must be non-zero".to_owned(),
            ));
        }
        if desc.mip_levels > max_mip_levels(desc.resolution, desc.resolution, 1) {
            return Err(RendererError::Validation(
                "environment bake mip_levels exceeds resolution".to_owned(),
            ));
        }
        if !desc.intensity.is_finite() || desc.intensity < 0.0 {
            return Err(RendererError::Validation(
                "environment bake intensity must be finite and non-negative".to_owned(),
            ));
        }
        validate_quat_finite(desc.rotation, "environment bake rotation")?;
        let source_color = average_texture_color(source_texture).unwrap_or([1.0, 1.0, 1.0, 1.0]);
        let irradiance_color = scale_color_rgb(source_color, desc.intensity * 0.318_309_9);
        let prefiltered_color = scale_color_rgb(source_color, desc.intensity);
        let irradiance_bytes = solid_texture_bytes(
            desc.resolution,
            desc.resolution,
            6,
            source_desc.format,
            irradiance_color,
        );
        let prefiltered_specular_bytes = solid_texture_bytes(
            desc.resolution,
            desc.resolution,
            6,
            source_desc.format,
            prefiltered_color,
        );
        let brdf_lut_bytes = generate_brdf_lut_rgba16f(desc.resolution);
        let irradiance = self.create_texture(TextureDesc {
            label: Some("environment_irradiance"),
            dimension: TextureDimension::Cube,
            width: desc.resolution,
            height: desc.resolution,
            depth_or_layers: 6,
            mip_levels: 1,
            samples: 1,
            format: source_desc.format,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
            initial_data: Some(TextureInitialData {
                bytes: &irradiance_bytes,
                bytes_per_row: desc.resolution * texture_format_bytes_per_pixel(source_desc.format),
                rows_per_image: desc.resolution,
            }),
        })?;
        let prefiltered_specular = self.create_texture(TextureDesc {
            label: Some("environment_prefiltered_specular"),
            dimension: TextureDimension::Cube,
            width: desc.resolution,
            height: desc.resolution,
            depth_or_layers: 6,
            mip_levels: desc.mip_levels,
            samples: 1,
            format: source_desc.format,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
            initial_data: Some(TextureInitialData {
                bytes: &prefiltered_specular_bytes,
                bytes_per_row: desc.resolution * texture_format_bytes_per_pixel(source_desc.format),
                rows_per_image: desc.resolution,
            }),
        })?;
        let brdf_lut = self.create_texture(TextureDesc {
            label: Some("environment_brdf_lut"),
            dimension: TextureDimension::D2,
            width: desc.resolution,
            height: desc.resolution,
            depth_or_layers: 1,
            mip_levels: 1,
            samples: 1,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
            initial_data: Some(TextureInitialData {
                bytes: &brdf_lut_bytes,
                bytes_per_row: desc.resolution
                    * texture_format_bytes_per_pixel(TextureFormat::Rgba16Float),
                rows_per_image: desc.resolution,
            }),
        })?;
        self.create_environment(EnvironmentDesc {
            label: desc.label,
            skybox: Some(source),
            irradiance: Some(irradiance),
            prefiltered_specular: Some(prefiltered_specular),
            brdf_lut: Some(brdf_lut),
            intensity: desc.intensity,
            rotation: desc.rotation,
            diffuse_color: Color::WHITE,
            diffuse_intensity: desc.intensity,
            specular_color: Color::WHITE,
            specular_intensity: desc.intensity,
            texture: Some(source),
            background_intensity: desc.intensity,
        })
    }

    fn validate_environment_desc(&self, desc: &EnvironmentDesc) -> Result<(), RendererError> {
        for texture in [
            desc.skybox,
            desc.irradiance,
            desc.prefiltered_specular,
            desc.brdf_lut,
            desc.texture,
        ]
        .into_iter()
        .flatten()
        {
            self.validated_texture_desc(texture, TextureUsage::SAMPLED, "environment texture")?;
        }
        if !desc.intensity.is_finite() || desc.intensity < 0.0 {
            return Err(RendererError::Validation(
                "environment intensity must be finite and non-negative".to_owned(),
            ));
        }
        if !desc.diffuse_intensity.is_finite()
            || desc.diffuse_intensity < 0.0
            || !desc.specular_intensity.is_finite()
            || desc.specular_intensity < 0.0
            || !desc.background_intensity.is_finite()
            || desc.background_intensity < 0.0
        {
            return Err(RendererError::Validation(
                "environment lighting intensities must be finite and non-negative".to_owned(),
            ));
        }
        validate_quat_finite(desc.rotation, "environment rotation")?;
        Ok(())
    }

    pub fn create_render_target(
        &mut self,
        desc: RenderTargetDesc,
    ) -> Result<RenderTargetHandle, RendererError> {
        self.validate_render_target_desc(&desc)?;
        Ok(self
            .render_targets
            .insert(ResourceKind::RenderTarget, StoredRenderTarget { desc }))
    }

    pub fn update_render_target(
        &mut self,
        render_target: RenderTargetHandle,
        desc: RenderTargetDesc,
    ) -> Result<(), RendererError> {
        self.validate_render_target_desc(&desc)?;
        let Some(slot) = self
            .render_targets
            .get_mut(ResourceKind::RenderTarget, render_target)
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::RenderTarget,
                raw: render_target.raw().get(),
            });
        };
        slot.value = Some(StoredRenderTarget { desc });
        slot.status = ResourceStatus::Ready;
        Ok(())
    }

    pub fn render_target_desc(
        &self,
        render_target: RenderTargetHandle,
    ) -> Option<&RenderTargetDesc> {
        self.render_targets
            .get(ResourceKind::RenderTarget, render_target)
            .and_then(|slot| slot.value.as_ref())
            .map(|target| &target.desc)
    }

    pub fn create_camera(&mut self, desc: CameraDesc) -> Result<CameraHandle, RendererError> {
        validate_camera_desc(&desc)?;
        Ok(self.cameras.insert(ResourceKind::Camera, desc))
    }

    pub fn update_camera(
        &mut self,
        camera: CameraHandle,
        desc: CameraDesc,
    ) -> Result<(), RendererError> {
        validate_camera_desc(&desc)?;
        let Some(slot) = self.cameras.get_mut(ResourceKind::Camera, camera) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Camera,
                raw: camera.raw().get(),
            });
        };
        slot.value = Some(desc);
        slot.status = ResourceStatus::Ready;
        Ok(())
    }

    pub fn camera_desc(&self, camera: CameraHandle) -> Option<&CameraDesc> {
        self.cameras
            .get(ResourceKind::Camera, camera)
            .and_then(|slot| slot.value.as_ref())
    }

    pub fn register_graph_extension(
        &mut self,
        extension: impl RenderGraphExtension,
    ) -> Result<RenderGraphExtensionHandle, RendererError> {
        if extension.name().trim().is_empty() {
            return Err(RendererError::Validation(
                "render graph extension name must not be empty".to_owned(),
            ));
        }
        Ok(self
            .graph_extensions
            .insert(ResourceKind::RenderGraphExtension, Arc::new(extension)))
    }

    pub fn register_post_process(
        &mut self,
        desc: CustomPostProcessDesc,
    ) -> Result<RenderGraphExtensionHandle, RendererError> {
        self.register_graph_extension(custom_post_process_from_desc(desc)?)
    }

    pub fn graph_extension_name(&self, extension: RenderGraphExtensionHandle) -> Option<&str> {
        self.graph_extensions
            .get(ResourceKind::RenderGraphExtension, extension)
            .and_then(|slot| slot.value.as_ref())
            .map(|extension| extension.name())
    }

    fn validate_render_target_desc(&self, desc: &RenderTargetDesc) -> Result<(), RendererError> {
        if desc.width == 0 || desc.height == 0 || desc.samples == 0 {
            return Err(RendererError::Validation(
                "render target dimensions and samples must be non-zero".to_owned(),
            ));
        }
        let color = self.validated_texture_desc(
            desc.color,
            TextureUsage::RENDER_TARGET,
            "render target color texture",
        )?;
        validate_direct_render_target_texture_shape(color, "render target color texture")?;
        if color.width != desc.width || color.height != desc.height || color.samples != desc.samples
        {
            return Err(RendererError::Validation(
                "render target color texture must match target dimensions and sample count"
                    .to_owned(),
            ));
        }
        if matches!(color.format, TextureFormat::Depth32Float) {
            return Err(RendererError::Validation(
                "render target color texture must use a color format".to_owned(),
            ));
        }
        if let Some(depth) = desc.depth {
            let depth = self.validated_texture_desc(
                depth,
                TextureUsage::DEPTH_STENCIL,
                "render target depth texture",
            )?;
            validate_direct_render_target_texture_shape(depth, "render target depth texture")?;
            if depth.width != desc.width
                || depth.height != desc.height
                || depth.samples != desc.samples
            {
                return Err(RendererError::Validation(
                    "render target depth texture must match target dimensions and sample count"
                        .to_owned(),
                ));
            }
            if !matches!(depth.format, TextureFormat::Depth32Float) {
                return Err(RendererError::Validation(
                    "render target depth texture must use a depth format".to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn validated_texture_desc(
        &self,
        texture: TextureHandle,
        required_usage: TextureUsage,
        role: &str,
    ) -> Result<&TextureDescOwned, RendererError> {
        let Some(texture_resource) = self.textures.get(ResourceKind::Texture, texture) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Texture,
                raw: texture.raw().get(),
            });
        };
        if texture_resource.status != ResourceStatus::Ready {
            return Err(RendererError::ResourceNotReady(ResourceKind::Texture));
        }
        let texture_resource = texture_resource
            .value
            .as_ref()
            .expect("arena get only returns occupied slots");
        if texture_resource.desc.usage.0 & required_usage.0 == 0 {
            return Err(RendererError::Validation(format!(
                "{role} is missing required usage"
            )));
        }
        Ok(&texture_resource.desc)
    }

    pub fn create_lod_group(
        &mut self,
        desc: LodGroupDesc,
    ) -> Result<LodGroupHandle, RendererError> {
        self.validate_lod_group(&desc)?;
        Ok(self.lod_groups.insert(ResourceKind::LodGroup, desc))
    }

    pub fn update_lod_group(
        &mut self,
        lod_group: LodGroupHandle,
        desc: LodGroupDesc,
    ) -> Result<(), RendererError> {
        self.validate_lod_group(&desc)?;
        let Some(slot) = self.lod_groups.get_mut(ResourceKind::LodGroup, lod_group) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::LodGroup,
                raw: lod_group.raw().get(),
            });
        };
        slot.value = Some(desc);
        slot.status = ResourceStatus::Ready;
        Ok(())
    }

    fn validate_lod_group(&self, desc: &LodGroupDesc) -> Result<(), RendererError> {
        if desc.levels.is_empty() {
            return Err(RendererError::Validation(
                "LOD group must contain at least one level".to_owned(),
            ));
        }
        let mut previous = 0.0;
        for (index, level) in desc.levels.iter().enumerate() {
            if !level.max_distance.is_finite() || level.max_distance <= 0.0 {
                return Err(RendererError::Validation(
                    "LOD level max_distance must be finite and positive".to_owned(),
                ));
            }
            if index > 0 && level.max_distance < previous {
                return Err(RendererError::Validation(
                    "LOD levels must be sorted by increasing max_distance".to_owned(),
                ));
            }
            previous = level.max_distance;
            if let Some(bounds) = level.bounds {
                validate_bounds(bounds)?;
            }
            if self.meshes.get(ResourceKind::Mesh, level.mesh).is_none() {
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Mesh,
                    raw: level.mesh.raw().get(),
                });
            }
            for material in &level.materials {
                if self
                    .materials
                    .get(ResourceKind::Material, *material)
                    .is_none()
                {
                    return Err(RendererError::InvalidHandle {
                        kind: ResourceKind::Material,
                        raw: material.raw().get(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn create_skeleton_instance(
        &mut self,
        desc: SkeletonInstanceDesc<'_>,
    ) -> Result<SkeletonInstanceHandle, RendererError> {
        if desc.joint_matrices.is_empty() {
            return Err(RendererError::Validation(
                "skeleton instance must contain at least one joint matrix".to_owned(),
            ));
        }
        validate_mat4_slice(desc.joint_matrices, "skeleton joint matrix")?;
        if let Some(inverse_bind_matrices) = desc.inverse_bind_matrices {
            if inverse_bind_matrices.len() != desc.joint_matrices.len() {
                return Err(RendererError::Validation(
                    "inverse bind matrix count must match joint matrix count".to_owned(),
                ));
            }
            validate_mat4_slice(inverse_bind_matrices, "skeleton inverse bind matrix")?;
        }
        Ok(self.skeleton_instances.insert(
            ResourceKind::SkeletonInstance,
            StoredSkeletonInstance {
                label: desc.label.map(str::to_owned),
                joint_matrices: desc.joint_matrices.to_vec(),
                inverse_bind_matrices: desc.inverse_bind_matrices.map(<[Mat4]>::to_vec),
                usage: desc.usage,
            },
        ))
    }

    pub fn skeleton_instance_info(
        &self,
        skeleton: SkeletonInstanceHandle,
    ) -> Option<SkeletonInstanceInfo> {
        self.skeleton_instances
            .get(ResourceKind::SkeletonInstance, skeleton)
            .and_then(|slot| slot.value.as_ref())
            .map(|skeleton| SkeletonInstanceInfo {
                label: skeleton.label.clone(),
                joint_count: skeleton.joint_matrices.len(),
                inverse_bind_count: skeleton
                    .inverse_bind_matrices
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or(0),
                usage: skeleton.usage,
            })
    }

    pub fn update_skeleton_instance(
        &mut self,
        skeleton: SkeletonInstanceHandle,
        joint_matrices: &[Mat4],
    ) -> Result<(), RendererError> {
        self.update_skeleton_joints(skeleton, joint_matrices)
    }

    pub fn update_skeleton_joints(
        &mut self,
        skeleton: SkeletonInstanceHandle,
        joint_matrices: &[Mat4],
    ) -> Result<(), RendererError> {
        if joint_matrices.is_empty() {
            return Err(RendererError::Validation(
                "skeleton instance update must contain at least one joint matrix".to_owned(),
            ));
        }
        validate_mat4_slice(joint_matrices, "skeleton joint matrix")?;
        let Some(slot) = self
            .skeleton_instances
            .get_mut(ResourceKind::SkeletonInstance, skeleton)
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::SkeletonInstance,
                raw: skeleton.raw().get(),
            });
        };
        let skeleton = slot.value.as_mut().expect("arena slot is occupied");
        if let Some(inverse_bind_matrices) = &skeleton.inverse_bind_matrices {
            if inverse_bind_matrices.len() != joint_matrices.len() {
                return Err(RendererError::Validation(
                    "joint matrix update count must match inverse bind matrix count".to_owned(),
                ));
            }
        }
        skeleton.joint_matrices = joint_matrices.to_vec();
        Ok(())
    }

    pub fn create_morph_weights(
        &mut self,
        desc: MorphWeightsDesc<'_>,
    ) -> Result<MorphWeightsHandle, RendererError> {
        validate_morph_weights(desc.weights)?;
        Ok(self.morph_weights.insert(
            ResourceKind::MorphWeights,
            StoredMorphWeights {
                label: desc.label.map(str::to_owned),
                weights: desc.weights.to_vec(),
            },
        ))
    }

    pub fn create_morph_weights_from_slice(
        &mut self,
        weights: &[f32],
    ) -> Result<MorphWeightsHandle, RendererError> {
        self.create_morph_weights(MorphWeightsDesc {
            label: None,
            weights,
        })
    }

    pub fn update_morph_weights(
        &mut self,
        morph_weights: MorphWeightsHandle,
        weights: &[f32],
    ) -> Result<(), RendererError> {
        validate_morph_weights(weights)?;
        let Some(slot) = self
            .morph_weights
            .get_mut(ResourceKind::MorphWeights, morph_weights)
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::MorphWeights,
                raw: morph_weights.raw().get(),
            });
        };
        slot.value.as_mut().expect("arena slot is occupied").weights = weights.to_vec();
        Ok(())
    }

    pub fn create_scene(&mut self, desc: SceneDesc) -> Result<SceneHandle, RendererError> {
        validate_scene_desc(&desc)?;
        let object_capacity = desc.max_objects_hint.unwrap_or(0) as usize;
        let light_capacity = desc.max_lights_hint.unwrap_or(0) as usize;
        Ok(self.scenes.insert(
            ResourceKind::Scene,
            StoredScene {
                desc,
                objects: Arena::with_capacity(object_capacity),
                lights: Arena::with_capacity(light_capacity),
                environment: None,
            },
        ))
    }

    pub fn reserve_scene_object(
        &mut self,
        scene: SceneHandle,
    ) -> Result<ObjectHandle, RendererError> {
        let Some(slot) = self.scenes.get_mut(ResourceKind::Scene, scene) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: scene.raw().get(),
            });
        };
        Ok(slot
            .value
            .as_mut()
            .expect("arena slot is occupied")
            .objects
            .reserve(ResourceKind::Object))
    }

    pub fn reserve_scene_light(
        &mut self,
        scene: SceneHandle,
    ) -> Result<LightHandle, RendererError> {
        let Some(slot) = self.scenes.get_mut(ResourceKind::Scene, scene) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: scene.raw().get(),
            });
        };
        Ok(slot
            .value
            .as_mut()
            .expect("arena slot is occupied")
            .lights
            .reserve(ResourceKind::Light))
    }

    pub fn edit_scene<R>(
        &mut self,
        scene: SceneHandle,
        edit: impl FnOnce(&mut SceneWriter<'_>) -> R,
    ) -> Result<R, RendererError> {
        let Some(slot) = self.scenes.get_mut(ResourceKind::Scene, scene) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: scene.raw().get(),
            });
        };
        let scene = slot.value.as_mut().expect("arena slot is occupied");
        let mut editor = SceneEditor { scene };
        Ok(edit(&mut editor))
    }

    pub fn scene_desc(&self, scene: SceneHandle) -> Option<&SceneDesc> {
        self.scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .map(|scene| &scene.desc)
    }

    pub fn apply_scene_commands(
        &mut self,
        commands: SceneCommandBuffer,
    ) -> Result<(), RendererError> {
        self.edit_scene(commands.scene, |scene| -> Result<(), RendererError> {
            for command in commands.commands {
                match command {
                    SceneCommand::SpawnAuto(object) => {
                        validate_render_object_desc(&object)?;
                        scene.spawn(object);
                    }
                    SceneCommand::Spawn(desc, object) => {
                        scene.spawn_reserved(object, desc)?;
                    }
                    SceneCommand::Despawn(object) => {
                        scene.despawn(object)?;
                    }
                    SceneCommand::SetTransform(object, transform) => {
                        scene.set_transform(object, transform)?;
                    }
                    SceneCommand::SetPreviousTransform { object, transform } => {
                        scene.set_previous_transform(object, transform)?;
                    }
                    SceneCommand::ClearPreviousTransform(object) => {
                        scene.clear_previous_transform(object)?;
                    }
                    SceneCommand::SetMesh { object, mesh } => {
                        scene.set_mesh(object, mesh)?;
                    }
                    SceneCommand::SetMaterial(object, slot, material) => {
                        scene.set_material(object, slot, material)?;
                    }
                    SceneCommand::SetVisibility(object, flags) => {
                        scene.set_visibility(object, flags)?;
                    }
                    SceneCommand::SetFlags { object, flags } => {
                        scene.set_flags(object, flags)?;
                    }
                    SceneCommand::SetLayer { object, layer } => {
                        scene.set_layer(object, layer)?;
                    }
                    SceneCommand::SetBounds { object, bounds } => {
                        scene.set_bounds(object, bounds)?;
                    }
                    SceneCommand::SetSkeleton { object, skeleton } => {
                        scene.set_skeleton(object, skeleton)?;
                    }
                    SceneCommand::SetMorphWeights {
                        object,
                        morph_weights,
                    } => {
                        scene.set_morph_weights(object, morph_weights)?;
                    }
                    SceneCommand::SetLodGroup { object, lod_group } => {
                        scene.set_lod_group(object, lod_group)?;
                    }
                    SceneCommand::AddLightAuto(light) => {
                        scene.add_light(light)?;
                    }
                    SceneCommand::AddLight(desc, light) => {
                        scene.add_light_reserved(light, desc)?;
                    }
                    SceneCommand::UpdateLight { light, update } => {
                        scene.update_light(light, update)?;
                    }
                    SceneCommand::RemoveLight(light) => {
                        scene.remove_light(light)?;
                    }
                    SceneCommand::SetEnvironment(environment) => {
                        scene.set_environment(environment)?;
                    }
                }
            }
            Ok(())
        })?
    }

    pub fn debug_draw(&mut self) -> DebugDraw<'_> {
        DebugDraw { renderer: self }
    }

    pub fn debug_draw_commands(&self) -> &[DebugDrawCommand] {
        &self.debug_draw_commands
    }

    pub fn request_picking(
        &mut self,
        request: PickingRequest,
    ) -> Result<PickingTicket, RendererError> {
        let Some(view) = self
            .views
            .get(ResourceKind::View, request.view)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::View,
                raw: request.view.raw().get(),
            });
        };
        let view_desc = view.desc.clone();
        let result = self.gpu_readback_pick_view(request.view, &view_desc, request.pixel)?;
        let handle: PickingHandle = self.picking.insert(ResourceKind::Picking, result);
        Ok(PickingTicket { raw: handle.raw() })
    }

    fn gpu_readback_pick_view(
        &self,
        view_handle: ViewHandle,
        view: &ViewDesc,
        pixel: UVec2,
    ) -> Result<PickingResult, RendererError> {
        let cpu_result = self.cpu_pick_view(view, pixel)?;
        let Some(object) = cpu_result.object else {
            return self
                .decode_gpu_picking_pixel(view_handle, [0, 0, 0, 0], 1.0, Vec3::ZERO)
                .map(|mut result| {
                    result.readback_pixel = Some(pixel);
                    result
                });
        };
        self.decode_gpu_picking_pixel(
            view_handle,
            encode_gpu_picking_object_index(object),
            cpu_result.depth,
            cpu_result.world_position,
        )
        .map(|mut result| {
            result.readback_pixel = Some(pixel);
            result
        })
    }

    fn cpu_pick_view(&self, view: &ViewDesc, pixel: UVec2) -> Result<PickingResult, RendererError> {
        let Some(scene) = self
            .scenes
            .get(ResourceKind::Scene, view.scene)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Ok(PickingResult::miss(PickingResultSource::CpuProjection));
        };
        let viewport = self.pick_viewport(view)?;
        let pixel_center = Vec2::new(pixel.x as f32 + 0.5, pixel.y as f32 + 0.5);
        if pixel_center.x < viewport[0]
            || pixel_center.y < viewport[1]
            || pixel_center.x >= viewport[0] + viewport[2]
            || pixel_center.y >= viewport[1] + viewport[3]
        {
            return Ok(PickingResult::miss(PickingResultSource::CpuProjection));
        }
        let mut best: Option<PickingResult> = None;
        for (index, slot) in scene.objects.resources.iter().enumerate() {
            let Some(object) = &slot.value else {
                continue;
            };
            if !view_object_pickable(self, scene, object, view)? {
                continue;
            }
            let Some(hit) = project_pick_candidate(object, view, viewport, pixel_center) else {
                continue;
            };
            let handle = make_handle(ResourceKind::Object, index as u32, slot.generation);
            let result = PickingResult {
                object: Some(handle),
                user_id: object.user_id,
                depth: hit.depth,
                world_position: hit.world_position,
                source: PickingResultSource::CpuProjection,
                readback_pixel: None,
                encoded_object_id: [0, 0, 0, 0],
            };
            if best
                .as_ref()
                .is_none_or(|current| result.depth < current.depth)
            {
                best = Some(result);
            }
        }
        Ok(best.unwrap_or_else(|| PickingResult::miss(PickingResultSource::CpuProjection)))
    }

    fn pick_viewport(&self, view: &ViewDesc) -> Result<Viewport, RendererError> {
        if let Some(viewport) = view.camera.viewport {
            return Ok(viewport);
        }
        let (width, height) = self.render_target_extent(&view.target)?;
        Ok([0.0, 0.0, width as f32, height as f32])
    }

    fn render_target_extent(&self, target: &RenderTarget) -> Result<(u32, u32), RendererError> {
        match *target {
            RenderTarget::Headless { width, height, .. } => Ok((width, height)),
            RenderTarget::Texture(texture) => {
                let desc = self.validated_texture_desc(
                    texture,
                    TextureUsage::RENDER_TARGET,
                    "pick target",
                )?;
                Ok((desc.width, desc.height))
            }
            RenderTarget::TextureView(view) => {
                let desc = self.validated_texture_desc(
                    view.texture,
                    TextureUsage::RENDER_TARGET,
                    "pick target texture view",
                )?;
                let divisor = 1u32.checked_shl(view.base_mip).unwrap_or(u32::MAX).max(1);
                Ok((
                    (desc.width / divisor).max(1),
                    (desc.height / divisor).max(1),
                ))
            }
            RenderTarget::External(render_target) => {
                let Some(desc) = self.render_target_desc(render_target) else {
                    return Err(RendererError::InvalidHandle {
                        kind: ResourceKind::RenderTarget,
                        raw: render_target.raw().get(),
                    });
                };
                Ok((desc.width, desc.height))
            }
            RenderTarget::MainSurface | RenderTarget::Surface(_) => Err(RendererError::Validation(
                "picking on a surface target requires camera.viewport to define pixel bounds"
                    .to_owned(),
            )),
        }
    }

    pub fn decode_gpu_picking_pixel(
        &self,
        view: ViewHandle,
        encoded: [u8; 4],
        depth: f32,
        world_position: Vec3,
    ) -> Result<PickingResult, RendererError> {
        let Some(view) = self
            .views
            .get(ResourceKind::View, view)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::View,
                raw: view.raw().get(),
            });
        };
        let Some(scene) = self
            .scenes
            .get(ResourceKind::Scene, view.desc.scene)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Ok(PickingResult::miss(PickingResultSource::GpuReadback));
        };
        if encoded == [0, 0, 0, 0] {
            return Ok(PickingResult::miss(PickingResultSource::GpuReadback));
        }
        validate_gpu_picking_payload(depth, world_position)?;
        let object_index =
            u32::from(encoded[0]) | (u32::from(encoded[1]) << 8) | (u32::from(encoded[2]) << 16);
        let Some(slot) = scene.objects.resources.get(object_index as usize) else {
            return Ok(PickingResult::miss(PickingResultSource::GpuReadback));
        };
        let Some(object) = &slot.value else {
            return Ok(PickingResult::miss(PickingResultSource::GpuReadback));
        };
        let handle = make_handle(ResourceKind::Object, object_index, slot.generation);
        if encoded[3] != gpu_picking_generation_byte(handle) {
            return Ok(PickingResult::miss(PickingResultSource::GpuReadback));
        }
        Ok(PickingResult {
            object: Some(handle),
            user_id: object.user_id,
            depth,
            world_position,
            source: PickingResultSource::GpuReadback,
            readback_pixel: None,
            encoded_object_id: encoded,
        })
    }

    pub fn poll_picking(&mut self, ticket: PickingTicket) -> Option<PickingResult> {
        let handle = PickingHandle::from_raw(ticket.raw);
        self.picking
            .get(ResourceKind::Picking, handle)
            .and_then(|slot| slot.value.clone())
    }

    pub fn begin_frame(&mut self, input: FrameInput) -> Result<Frame<'_>, RendererError> {
        if self.device_status() == DeviceStatus::Lost {
            return Err(RendererError::DeviceLost {
                reason: "renderer device is lost".to_owned(),
            });
        }
        if !input.delta_time.is_finite() || input.delta_time < 0.0 {
            return Err(RendererError::Validation(
                "frame delta_time must be finite and non-negative".to_owned(),
            ));
        }
        if !input.absolute_time.is_finite() {
            return Err(RendererError::Validation(
                "frame absolute_time must be finite".to_owned(),
            ));
        }
        let frame_index = input.frame_index_override.unwrap_or(self.frame_index);
        self.reset_pipeline_cache_frame_stats();
        Ok(Frame {
            renderer: self,
            frame_index,
            started_at: Instant::now(),
            wait_for_gpu: input.wait_for_gpu,
            views: Vec::new(),
            graph_extensions: Vec::new(),
        })
    }

    pub fn resource_status<T>(&self, handle: Handle<T>) -> Option<ResourceStatus> {
        match handle.kind_tag() {
            tag if tag == ResourceKind::Mesh.tag() => self
                .meshes
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Buffer.tag() => self
                .buffers
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Texture.tag() => self
                .textures
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Surface.tag() => self
                .is_main_surface(Handle::from_raw(handle.raw()))
                .then_some(ResourceStatus::Ready),
            tag if tag == ResourceKind::Material.tag() => self
                .materials
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::MaterialTemplate.tag() => self
                .material_templates
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Shader.tag() => self
                .shaders
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Environment.tag() => self
                .environments
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::RenderTarget.tag() => self
                .render_targets
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Camera.tag() => self
                .cameras
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::RenderGraphExtension.tag() => self
                .graph_extensions
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::SkeletonInstance.tag() => self
                .skeleton_instances
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::MorphWeights.tag() => self
                .morph_weights
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::LodGroup.tag() => self
                .lod_groups
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Scene.tag() => self
                .scenes
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Sampler.tag() => self
                .samplers
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::View.tag() => self
                .views
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            tag if tag == ResourceKind::Picking.tag() => self
                .picking
                .resources
                .get(handle.index() as usize)
                .filter(|slot| slot.generation == handle.generation())
                .map(|slot| slot.status),
            _ => None,
        }
    }

    pub fn destroy<T>(&mut self, handle: Handle<T>) -> Result<(), RendererError> {
        match handle.kind_tag() {
            tag if tag == ResourceKind::Mesh.tag() => {
                self.meshes.destroy(ResourceKind::Mesh, handle)
            }
            tag if tag == ResourceKind::Buffer.tag() => {
                self.buffers.destroy(ResourceKind::Buffer, handle)
            }
            tag if tag == ResourceKind::Texture.tag() => {
                self.textures.destroy(ResourceKind::Texture, handle)
            }
            tag if tag == ResourceKind::Surface.tag() => {
                let surface = Handle::from_raw(handle.raw());
                if self.is_main_surface(surface) {
                    return Err(RendererError::Validation(
                        "main surface is owned by the renderer and cannot be destroyed separately"
                            .to_owned(),
                    ));
                }
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Surface,
                    raw: handle.raw().get(),
                });
            }
            tag if tag == ResourceKind::Material.tag() => {
                self.materials.destroy(ResourceKind::Material, handle)
            }
            tag if tag == ResourceKind::Scene.tag() => {
                self.scenes.destroy(ResourceKind::Scene, handle)
            }
            tag if tag == ResourceKind::MaterialTemplate.tag() => self
                .material_templates
                .destroy(ResourceKind::MaterialTemplate, handle),
            tag if tag == ResourceKind::Shader.tag() => {
                self.shaders.destroy(ResourceKind::Shader, handle)
            }
            tag if tag == ResourceKind::Sampler.tag() => {
                self.samplers.destroy(ResourceKind::Sampler, handle)
            }
            tag if tag == ResourceKind::Environment.tag() => {
                self.environments.destroy(ResourceKind::Environment, handle)
            }
            tag if tag == ResourceKind::RenderTarget.tag() => self
                .render_targets
                .destroy(ResourceKind::RenderTarget, handle),
            tag if tag == ResourceKind::Camera.tag() => {
                self.cameras.destroy(ResourceKind::Camera, handle)
            }
            tag if tag == ResourceKind::RenderGraphExtension.tag() => self
                .graph_extensions
                .destroy(ResourceKind::RenderGraphExtension, handle),
            tag if tag == ResourceKind::SkeletonInstance.tag() => self
                .skeleton_instances
                .destroy(ResourceKind::SkeletonInstance, handle),
            tag if tag == ResourceKind::MorphWeights.tag() => self
                .morph_weights
                .destroy(ResourceKind::MorphWeights, handle),
            tag if tag == ResourceKind::LodGroup.tag() => {
                self.lod_groups.destroy(ResourceKind::LodGroup, handle)
            }
            tag if tag == ResourceKind::View.tag() => {
                self.views.destroy(ResourceKind::View, handle)
            }
            tag if tag == ResourceKind::Picking.tag() => {
                self.picking.destroy(ResourceKind::Picking, handle)
            }
            _ => Err(invalid_handle_error(handle)),
        }
    }

    pub fn upload_stats(&self) -> UploadStats {
        self.upload_stats.clone()
    }

    fn queue_upload_bytes(&mut self, bytes: u64) {
        if bytes == 0 {
            return;
        }
        self.upload_stats.bytes_queued = self.upload_stats.bytes_queued.saturating_add(bytes);
        self.upload_stats.pending_uploads = self.upload_stats.pending_uploads.saturating_add(1);
        self.upload_stats.staging_bytes_in_use =
            self.upload_stats.staging_bytes_in_use.saturating_add(bytes);
    }

    pub fn memory_stats(&self) -> MemoryStats {
        MemoryStats {
            resident_bytes: self.resident_resource_bytes(),
            delayed_destroy_count: self.delayed_destroy_count(),
        }
    }

    pub fn flush_uploads(&mut self) -> Result<(), RendererError> {
        self.upload_stats.bytes_uploaded_this_frame = self.upload_stats.bytes_queued;
        self.upload_stats.bytes_queued = 0;
        self.upload_stats.pending_uploads = 0;
        self.upload_stats.staging_bytes_in_use = 0;
        Ok(())
    }

    pub fn extract_render_data(
        &mut self,
        scene: SceneHandle,
        source: &impl ExtractRenderData,
    ) -> Result<(), RendererError> {
        if self.scenes.get(ResourceKind::Scene, scene).is_none() {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: scene.raw().get(),
            });
        }
        let mut commands = SceneCommandBuffer::new(scene);
        source.extract(&mut commands);
        self.apply_scene_commands(commands)
    }

    pub fn set_resource_priority<T>(
        &mut self,
        handle: Handle<T>,
        priority: ResidencyPriority,
    ) -> Result<(), RendererError> {
        match handle.kind_tag() {
            tag if tag == ResourceKind::Mesh.tag() => {
                self.meshes
                    .set_priority(ResourceKind::Mesh, handle, priority)?;
            }
            tag if tag == ResourceKind::Buffer.tag() => {
                self.buffers
                    .set_priority(ResourceKind::Buffer, handle, priority)?;
            }
            tag if tag == ResourceKind::Texture.tag() => {
                self.textures
                    .set_priority(ResourceKind::Texture, handle, priority)?;
            }
            tag if tag == ResourceKind::Surface.tag() => {
                let surface = Handle::from_raw(handle.raw());
                if !self.is_main_surface(surface) {
                    return Err(RendererError::InvalidHandle {
                        kind: ResourceKind::Surface,
                        raw: handle.raw().get(),
                    });
                }
                self.main_surface_priority = priority;
            }
            tag if tag == ResourceKind::Material.tag() => {
                self.materials
                    .set_priority(ResourceKind::Material, handle, priority)?;
            }
            tag if tag == ResourceKind::MaterialTemplate.tag() => {
                self.material_templates.set_priority(
                    ResourceKind::MaterialTemplate,
                    handle,
                    priority,
                )?;
            }
            tag if tag == ResourceKind::Shader.tag() => {
                self.shaders
                    .set_priority(ResourceKind::Shader, handle, priority)?;
            }
            tag if tag == ResourceKind::Scene.tag() => {
                self.scenes
                    .set_priority(ResourceKind::Scene, handle, priority)?;
            }
            tag if tag == ResourceKind::Sampler.tag() => {
                self.samplers
                    .set_priority(ResourceKind::Sampler, handle, priority)?;
            }
            tag if tag == ResourceKind::Environment.tag() => {
                self.environments
                    .set_priority(ResourceKind::Environment, handle, priority)?;
            }
            tag if tag == ResourceKind::RenderTarget.tag() => {
                self.render_targets
                    .set_priority(ResourceKind::RenderTarget, handle, priority)?;
            }
            tag if tag == ResourceKind::Camera.tag() => {
                self.cameras
                    .set_priority(ResourceKind::Camera, handle, priority)?;
            }
            tag if tag == ResourceKind::RenderGraphExtension.tag() => {
                self.graph_extensions.set_priority(
                    ResourceKind::RenderGraphExtension,
                    handle,
                    priority,
                )?;
            }
            tag if tag == ResourceKind::SkeletonInstance.tag() => {
                self.skeleton_instances.set_priority(
                    ResourceKind::SkeletonInstance,
                    handle,
                    priority,
                )?;
            }
            tag if tag == ResourceKind::MorphWeights.tag() => {
                self.morph_weights
                    .set_priority(ResourceKind::MorphWeights, handle, priority)?;
            }
            tag if tag == ResourceKind::LodGroup.tag() => {
                self.lod_groups
                    .set_priority(ResourceKind::LodGroup, handle, priority)?;
            }
            tag if tag == ResourceKind::View.tag() => {
                self.views
                    .set_priority(ResourceKind::View, handle, priority)?;
            }
            tag if tag == ResourceKind::Picking.tag() => {
                self.picking
                    .set_priority(ResourceKind::Picking, handle, priority)?;
            }
            _ => {
                return Err(invalid_handle_error(handle));
            }
        }
        Ok(())
    }

    pub fn evict_resource<T>(&mut self, handle: Handle<T>) -> Result<(), RendererError> {
        self.set_streaming_resource_status(handle, ResourceStatus::Evicted)
    }

    pub fn make_resource_resident<T>(&mut self, handle: Handle<T>) -> Result<(), RendererError> {
        self.set_streaming_resource_status(handle, ResourceStatus::Ready)
    }

    fn set_streaming_resource_status<T>(
        &mut self,
        handle: Handle<T>,
        status: ResourceStatus,
    ) -> Result<(), RendererError> {
        match handle.kind_tag() {
            tag if tag == ResourceKind::Mesh.tag() => {
                let slot = self.meshes.set_status(ResourceKind::Mesh, handle, status)?;
                if let Some(mesh) = slot.value.as_mut() {
                    mesh.info.status = status;
                }
            }
            tag if tag == ResourceKind::Texture.tag() => {
                self.textures
                    .set_status(ResourceKind::Texture, handle, status)?;
            }
            tag if tag == ResourceKind::Buffer.tag() => {
                self.buffers
                    .set_status(ResourceKind::Buffer, handle, status)?;
            }
            _ => {
                return Err(invalid_handle_error(handle));
            }
        }
        Ok(())
    }

    pub fn resource_priority<T>(&self, handle: Handle<T>) -> Option<ResidencyPriority> {
        match handle.kind_tag() {
            tag if tag == ResourceKind::Mesh.tag() => {
                self.meshes.priority(ResourceKind::Mesh, handle)
            }
            tag if tag == ResourceKind::Buffer.tag() => {
                self.buffers.priority(ResourceKind::Buffer, handle)
            }
            tag if tag == ResourceKind::Texture.tag() => {
                self.textures.priority(ResourceKind::Texture, handle)
            }
            tag if tag == ResourceKind::Surface.tag() => self
                .is_main_surface(Handle::from_raw(handle.raw()))
                .then_some(self.main_surface_priority),
            tag if tag == ResourceKind::Material.tag() => {
                self.materials.priority(ResourceKind::Material, handle)
            }
            tag if tag == ResourceKind::MaterialTemplate.tag() => self
                .material_templates
                .priority(ResourceKind::MaterialTemplate, handle),
            tag if tag == ResourceKind::Shader.tag() => {
                self.shaders.priority(ResourceKind::Shader, handle)
            }
            tag if tag == ResourceKind::Scene.tag() => {
                self.scenes.priority(ResourceKind::Scene, handle)
            }
            tag if tag == ResourceKind::Sampler.tag() => {
                self.samplers.priority(ResourceKind::Sampler, handle)
            }
            tag if tag == ResourceKind::Environment.tag() => self
                .environments
                .priority(ResourceKind::Environment, handle),
            tag if tag == ResourceKind::RenderTarget.tag() => self
                .render_targets
                .priority(ResourceKind::RenderTarget, handle),
            tag if tag == ResourceKind::Camera.tag() => {
                self.cameras.priority(ResourceKind::Camera, handle)
            }
            tag if tag == ResourceKind::RenderGraphExtension.tag() => self
                .graph_extensions
                .priority(ResourceKind::RenderGraphExtension, handle),
            tag if tag == ResourceKind::SkeletonInstance.tag() => self
                .skeleton_instances
                .priority(ResourceKind::SkeletonInstance, handle),
            tag if tag == ResourceKind::MorphWeights.tag() => self
                .morph_weights
                .priority(ResourceKind::MorphWeights, handle),
            tag if tag == ResourceKind::LodGroup.tag() => {
                self.lod_groups.priority(ResourceKind::LodGroup, handle)
            }
            tag if tag == ResourceKind::View.tag() => {
                self.views.priority(ResourceKind::View, handle)
            }
            tag if tag == ResourceKind::Picking.tag() => {
                self.picking.priority(ResourceKind::Picking, handle)
            }
            _ => None,
        }
    }

    pub fn pipeline_cache_stats(&self) -> PipelineCacheStats {
        self.pipeline_cache_stats.clone()
    }

    pub fn warm_up_pipelines(
        &mut self,
        requests: &[PipelineWarmupRequest],
    ) -> Result<(), RendererError> {
        for request in requests {
            self.validate_pipeline_key(&request.key)?;
        }
        for request in requests {
            if self.pipeline_cache.insert(request.key) {
                self.pipeline_cache_stats.total += 1;
                self.pipeline_cache_stats.ready += 1;
                self.pipeline_cache_stats.cache_misses_this_frame += 1;
            } else {
                self.pipeline_cache_stats.cache_hits_this_frame += 1;
            }
        }
        Ok(())
    }

    fn reset_pipeline_cache_frame_stats(&mut self) {
        self.pipeline_cache_stats.cache_hits_this_frame = 0;
        self.pipeline_cache_stats.cache_misses_this_frame = 0;
    }

    fn record_pipeline_keys(&mut self, keys: &[PipelineKey]) {
        for key in keys {
            if self.pipeline_cache.insert(*key) {
                self.pipeline_cache_stats.total += 1;
                self.pipeline_cache_stats.ready += 1;
                self.pipeline_cache_stats.cache_misses_this_frame += 1;
            } else {
                self.pipeline_cache_stats.cache_hits_this_frame += 1;
            }
        }
    }

    fn invalidate_shader_pipelines(&mut self, shader: ShaderHandle) {
        let before = self.pipeline_cache.len();
        self.pipeline_cache.retain(|key| key.shader != shader);
        let removed = before - self.pipeline_cache.len();
        self.pipeline_cache_stats.total = self.pipeline_cache.len();
        self.pipeline_cache_stats.ready = self.pipeline_cache_stats.ready.saturating_sub(removed);
        self.pipeline_cache_stats.compiling = 0;
    }

    fn validate_pipeline_key(&self, key: &PipelineKey) -> Result<(), RendererError> {
        if key.sample_count == 0 {
            return Err(RendererError::Validation(
                "pipeline sample_count must be non-zero".to_owned(),
            ));
        }
        if self.shaders.get(ResourceKind::Shader, key.shader).is_none() {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Shader,
                raw: key.shader.raw().get(),
            });
        }
        if self
            .material_templates
            .get(ResourceKind::MaterialTemplate, key.material_template)
            .is_none()
        {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::MaterialTemplate,
                raw: key.material_template.raw().get(),
            });
        }
        if !self.caps.formats.color.contains(&key.color_format) {
            return Err(RendererError::Validation(
                "pipeline color format is not supported by renderer caps".to_owned(),
            ));
        }
        if !self.caps.formats.depth.contains(&key.depth_format) {
            return Err(RendererError::Validation(
                "pipeline depth format is not supported by renderer caps".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn last_frame_stats(&self) -> Option<&FrameStats> {
        self.last_frame_stats.as_ref()
    }

    fn resident_resource_bytes(&self) -> u64 {
        let mesh_bytes: usize =
            self.meshes
                .resources
                .iter()
                .filter(|slot| slot.status == ResourceStatus::Ready)
                .filter_map(|slot| slot.value.as_ref())
                .map(|mesh| {
                    mesh.vertex_bytes.len()
                        + mesh.vertex_stream_bytes.iter().map(Vec::len).sum::<usize>()
                        + mesh.index_bytes.len()
                        + mesh
                            .skin_inverse_bind_matrices
                            .as_ref()
                            .map_or(0, |matrices| matrices.len() * std::mem::size_of::<Mat4>())
                        + mesh
                            .morph_targets
                            .iter()
                            .map(|target| {
                                target
                                    .positions
                                    .as_ref()
                                    .map_or(0, |values| values.len() * std::mem::size_of::<Vec3>())
                                    + target.normals.as_ref().map_or(0, |values| {
                                        values.len() * std::mem::size_of::<Vec3>()
                                    })
                                    + target.tangents.as_ref().map_or(0, |values| {
                                        values.len() * std::mem::size_of::<Vec3>()
                                    })
                            })
                            .sum::<usize>()
                        + mesh.meshlet_bytes.as_ref().map_or(0, Vec::len)
                })
                .sum();
        let texture_bytes: usize = self
            .textures
            .resources
            .iter()
            .filter(|slot| slot.status == ResourceStatus::Ready)
            .filter_map(|slot| slot.value.as_ref())
            .map(|texture| texture.bytes.len())
            .sum();
        let buffer_bytes: usize = self
            .buffers
            .resources
            .iter()
            .filter(|slot| slot.status == ResourceStatus::Ready)
            .filter_map(|slot| slot.value.as_ref())
            .map(|buffer| buffer.bytes.len())
            .sum();
        let skeleton_bytes: usize = self
            .skeleton_instances
            .resources
            .iter()
            .filter(|slot| slot.status == ResourceStatus::Ready)
            .filter_map(|slot| slot.value.as_ref())
            .map(|skeleton| {
                let inverse_bind_count = skeleton
                    .inverse_bind_matrices
                    .as_ref()
                    .map(Vec::len)
                    .unwrap_or(0);
                (skeleton.joint_matrices.len() + inverse_bind_count) * std::mem::size_of::<Mat4>()
            })
            .sum();
        let morph_bytes: usize = self
            .morph_weights
            .resources
            .iter()
            .filter(|slot| slot.status == ResourceStatus::Ready)
            .filter_map(|slot| slot.value.as_ref())
            .map(|morph| morph.weights.len() * std::mem::size_of::<f32>())
            .sum();
        (mesh_bytes + buffer_bytes + texture_bytes + skeleton_bytes + morph_bytes) as u64
    }

    fn delayed_destroy_count(&self) -> usize {
        count_destroy_queued(&self.meshes)
            + count_destroy_queued(&self.buffers)
            + count_destroy_queued(&self.textures)
            + count_destroy_queued(&self.samplers)
            + count_destroy_queued(&self.shaders)
            + count_destroy_queued(&self.materials)
            + count_destroy_queued(&self.material_templates)
            + count_destroy_queued(&self.environments)
            + count_destroy_queued(&self.render_targets)
            + count_destroy_queued(&self.lod_groups)
            + count_destroy_queued(&self.cameras)
            + count_destroy_queued(&self.graph_extensions)
            + count_destroy_queued(&self.skeleton_instances)
            + count_destroy_queued(&self.morph_weights)
            + count_destroy_queued(&self.scenes)
            + count_destroy_queued(&self.views)
            + count_destroy_queued(&self.picking)
    }

    pub fn enable_gpu_profiler(&mut self, enabled: bool) -> Result<(), RendererError> {
        self.gpu_profiler_enabled = enabled;
        Ok(())
    }

    pub fn capture_next_frame(&mut self, options: CaptureOptions) -> Result<(), RendererError> {
        if options.open_after_capture && matches!(options.backend, FrameCaptureBackend::Internal) {
            return Err(RendererError::Validation(
                "open_after_capture requires an external capture backend".to_owned(),
            ));
        }
        self.capture_queued = Some(options);
        Ok(())
    }

    pub fn set_frame_capture_backend_available(
        &mut self,
        backend: FrameCaptureBackend,
        available: bool,
    ) -> Result<(), RendererError> {
        if matches!(backend, FrameCaptureBackend::Internal) {
            return Err(RendererError::Validation(
                "internal frame capture is always available".to_owned(),
            ));
        }
        if available {
            self.capture_backend_hooks.insert(backend);
        } else {
            self.capture_backend_hooks.remove(&backend);
        }
        Ok(())
    }

    fn apply_frame_instrumentation(&mut self, stats: &mut FrameStats) {
        if stats.pipeline_statistics.is_none() {
            stats.pipeline_statistics = self.frame_pipeline_statistics(stats);
        }
        if self.gpu_profiler_enabled {
            stats.gpu_profiler_enabled = true;
            if stats.gpu_time_ms.is_none() {
                stats.gpu_time_ms = stats
                    .graph
                    .gpu_time_ns
                    .map(|time_ns| time_ns as f32 / 1_000_000.0)
                    .or(Some(0.0));
            }
            stats.profile = Some(FrameProfile {
                frame_index: stats.frame_index,
                cpu_build_time_ms: stats.cpu_build_time_ms,
                cpu_submit_time_ms: stats.cpu_submit_time_ms,
                gpu_time_ms: stats.gpu_time_ms,
                graph_passes: stats.graph.pass_count,
                graph_barriers: stats.graph.barriers,
                debug_groups: stats.graph.debug_groups,
                draw_calls: stats.draw_calls,
                dispatch_calls: stats.dispatch_calls,
                deformed_objects: stats.deformed_objects,
                motion_vector_objects: stats.motion_vector_objects,
                motion_vector_views: stats.motion_vector_views,
                pipeline_statistics: stats.pipeline_statistics.clone(),
            });
        }
        if let Some(options) = self.capture_queued.take() {
            stats.capture_triggered = true;
            stats.capture_label = options.label.clone();
            let status = self.capture_status(options.backend);
            stats.capture = Some(FrameCapture {
                label: options.label,
                backend: options.backend,
                status,
                include_resource_dump: options.include_resource_dump,
                open_after_capture: options.open_after_capture,
                frame_index: stats.frame_index,
                graph: stats.graph.clone(),
                cpu_build_time_ms: stats.cpu_build_time_ms,
                cpu_submit_time_ms: stats.cpu_submit_time_ms,
                draw_calls: stats.draw_calls,
                dispatch_calls: stats.dispatch_calls,
                visible_objects: stats.visible_objects,
                culled_objects: stats.culled_objects,
                skinned_objects: stats.skinned_objects,
                morphed_objects: stats.morphed_objects,
                deformed_objects: stats.deformed_objects,
                motion_vector_objects: stats.motion_vector_objects,
                motion_vector_views: stats.motion_vector_views,
                culling_outputs: stats.culling_outputs.clone(),
                ssao_outputs: stats.ssao_outputs.clone(),
                light_cluster_outputs: stats.light_cluster_outputs.clone(),
                area_light_outputs: stats.area_light_outputs.clone(),
                ray_tracing_outputs: stats.ray_tracing_outputs.clone(),
                shadow_outputs: stats.shadow_outputs.clone(),
                gbuffer_outputs: stats.gbuffer_outputs.clone(),
                lod_outputs: stats.lod_outputs.clone(),
                streaming_outputs: stats.streaming_outputs.clone(),
                debug_draw_outputs: stats.debug_draw_outputs.clone(),
                picking_outputs: stats.picking_outputs.clone(),
                environment_outputs: stats.environment_outputs.clone(),
                deformation_outputs: stats.deformation_outputs.clone(),
                motion_vector_outputs: stats.motion_vector_outputs.clone(),
                post_process_outputs: stats.post_process_outputs.clone(),
                pipeline_statistics: stats.pipeline_statistics.clone(),
                resource_dump: options
                    .include_resource_dump
                    .then(|| self.frame_capture_resource_dump()),
            });
        }
    }

    fn frame_pipeline_statistics(&self, stats: &FrameStats) -> Option<FramePipelineStatistics> {
        if !self.supports_feature(RendererFeature::PipelineStatistics) {
            return None;
        }

        let vertices = stats.triangles.saturating_mul(3);
        Some(FramePipelineStatistics {
            input_assembly_vertices: vertices,
            input_assembly_primitives: stats.triangles,
            vertex_shader_invocations: vertices,
            clipping_invocations: stats.triangles,
            clipping_primitives: stats.triangles,
            fragment_shader_invocations: stats.triangles,
            compute_shader_invocations: u64::from(stats.graph.compute_dispatches),
            draw_calls: stats.draw_calls,
            dispatch_calls: stats.dispatch_calls,
        })
    }

    fn frame_capture_resource_dump(&self) -> FrameCaptureResourceDump {
        let memory = self.memory_stats();
        FrameCaptureResourceDump {
            meshes: count_ready(&self.meshes),
            buffers: count_ready(&self.buffers),
            textures: count_ready(&self.textures),
            samplers: count_ready(&self.samplers),
            shaders: count_ready(&self.shaders),
            materials: count_ready(&self.materials),
            material_templates: count_ready(&self.material_templates),
            environments: count_ready(&self.environments),
            render_targets: count_ready(&self.render_targets),
            lod_groups: count_ready(&self.lod_groups),
            cameras: count_ready(&self.cameras),
            graph_extensions: count_ready(&self.graph_extensions),
            skeleton_instances: count_ready(&self.skeleton_instances),
            morph_weights: count_ready(&self.morph_weights),
            scenes: count_ready(&self.scenes),
            views: count_ready(&self.views),
            picking_results: count_ready(&self.picking),
            resident_bytes: memory.resident_bytes,
            delayed_destroy_count: memory.delayed_destroy_count,
            pending_uploads: self.upload_stats.pending_uploads,
            staging_bytes_in_use: self.upload_stats.staging_bytes_in_use,
        }
    }

    fn capture_status(&self, backend: FrameCaptureBackend) -> FrameCaptureStatus {
        match backend {
            FrameCaptureBackend::Internal => FrameCaptureStatus::Captured,
            FrameCaptureBackend::RenderDoc | FrameCaptureBackend::ExternalDebugger => {
                if self.capture_backend_hooks.contains(&backend) {
                    FrameCaptureStatus::BackendHookRequested
                } else {
                    FrameCaptureStatus::BackendUnavailable
                }
            }
        }
    }

    fn validate_view_target(&self, target: &RenderTarget) -> Result<(), RendererError> {
        match *target {
            RenderTarget::MainSurface => Ok(()),
            RenderTarget::Surface(surface) if Some(surface) == self.main_surface => Ok(()),
            RenderTarget::Surface(surface) => Err(RendererError::InvalidHandle {
                kind: ResourceKind::Surface,
                raw: surface.raw().get(),
            }),
            RenderTarget::Texture(texture) => {
                let texture = self.validated_texture_desc(
                    texture,
                    TextureUsage::RENDER_TARGET,
                    "view target texture",
                )?;
                validate_direct_render_target_texture_shape(texture, "view target texture")?;
                if matches!(texture.format, TextureFormat::Depth32Float) {
                    return Err(RendererError::Validation(
                        "view target texture must use a color format".to_owned(),
                    ));
                }
                Ok(())
            }
            RenderTarget::TextureView(view) => {
                let texture = self.validated_texture_desc(
                    view.texture,
                    TextureUsage::RENDER_TARGET,
                    "view target texture view",
                )?;
                if matches!(texture.format, TextureFormat::Depth32Float) {
                    return Err(RendererError::Validation(
                        "view target texture view must use a color format".to_owned(),
                    ));
                }
                validate_render_target_texture_view_shape(texture)?;
                if view.mip_count == 0 || view.layer_count == 0 {
                    return Err(RendererError::Validation(
                        "texture view mip_count and layer_count must be non-zero".to_owned(),
                    ));
                }
                if view.mip_count != 1 {
                    return Err(RendererError::Validation(
                        "render target texture views must reference exactly one mip level"
                            .to_owned(),
                    ));
                }
                let mip_end = view.base_mip.checked_add(view.mip_count).ok_or_else(|| {
                    RendererError::Validation("texture view mip range overflows".to_owned())
                })?;
                if mip_end > texture.mip_levels {
                    return Err(RendererError::Validation(
                        "texture view mip range exceeds texture mip levels".to_owned(),
                    ));
                }
                let layer_end = view
                    .base_layer
                    .checked_add(view.layer_count)
                    .ok_or_else(|| {
                        RendererError::Validation(
                            "texture view array layer range overflows".to_owned(),
                        )
                    })?;
                if layer_end > texture.depth_or_layers {
                    return Err(RendererError::Validation(
                        "texture view array layer range exceeds texture layers".to_owned(),
                    ));
                }
                Ok(())
            }
            RenderTarget::External(render_target) => {
                if self
                    .render_targets
                    .get(ResourceKind::RenderTarget, render_target)
                    .and_then(|slot| slot.value.as_ref())
                    .is_none()
                {
                    return Err(RendererError::InvalidHandle {
                        kind: ResourceKind::RenderTarget,
                        raw: render_target.raw().get(),
                    });
                }
                Ok(())
            }
            RenderTarget::Headless {
                width,
                height,
                format,
            } => {
                if width == 0 || height == 0 {
                    return Err(RendererError::Validation(
                        "headless render target dimensions must be non-zero".to_owned(),
                    ));
                }
                if matches!(format, TextureFormat::Depth32Float) {
                    return Err(RendererError::Validation(
                        "headless render target must use a color format".to_owned(),
                    ));
                }
                Ok(())
            }
        }
    }

    fn validate_view_quality(&self, view: &ViewDesc) -> Result<(), RendererError> {
        if view.quality.variable_rate_shading
            && !self.supports_feature(RendererFeature::VariableRateShading)
        {
            return Err(RendererError::UnsupportedFeature(
                RendererFeature::VariableRateShading,
            ));
        }
        if view.quality.bindless_textures
            && !self.supports_feature(RendererFeature::BindlessTextures)
        {
            return Err(RendererError::UnsupportedFeature(
                RendererFeature::BindlessTextures,
            ));
        }
        if view.quality.mesh_shaders && !self.supports_feature(RendererFeature::MeshShader) {
            return Err(RendererError::UnsupportedFeature(
                RendererFeature::MeshShader,
            ));
        }
        if view.quality.virtual_texturing
            && !self.supports_feature(RendererFeature::VirtualTexturing)
        {
            return Err(RendererError::UnsupportedFeature(
                RendererFeature::VirtualTexturing,
            ));
        }
        if view.quality.ray_tracing && !self.supports_feature(RendererFeature::RayTracing) {
            return Err(RendererError::UnsupportedFeature(
                RendererFeature::RayTracing,
            ));
        }
        Ok(())
    }

    fn validate_scene_render_resources(
        &self,
        view: &ViewDesc,
        mode: ValidationMode,
    ) -> Result<(), RendererError> {
        self.validate_graph_extensions(&view.graph_extensions)?;
        let Some(scene) = self
            .scenes
            .get(ResourceKind::Scene, view.scene)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: view.scene.raw().get(),
            });
        };
        if let Some(environment) = scene.environment {
            self.require_resource(ResourceKind::Environment, environment)?;
            if validation_checks_deep_resource_dependencies(mode) {
                self.validate_environment_resource_dependencies(environment)?;
            }
        }
        for slot in &scene.objects.resources {
            let Some(object) = &slot.value else {
                continue;
            };
            let visible =
                view_object_visibility(scene, object, view) == ViewObjectVisibility::Visible;
            if !visible && !validation_checks_all_scene_objects(mode) {
                continue;
            }
            validate_render_object_desc(object)?;
            self.require_resource(ResourceKind::Mesh, object.mesh)?;
            for material in &object.materials {
                self.require_resource(ResourceKind::Material, *material)?;
                if validation_checks_deep_resource_dependencies(mode) {
                    self.validate_material_resource_dependencies(*material)?;
                }
            }
            if let Some(skeleton) = object.skeleton {
                self.require_resource(ResourceKind::SkeletonInstance, skeleton)?;
            }
            if let Some(morph_weights) = object.morph_weights {
                self.require_resource(ResourceKind::MorphWeights, morph_weights)?;
            }
            if let Some(lod_group) = object.lod_group {
                let Some(group) = self
                    .lod_groups
                    .get(ResourceKind::LodGroup, lod_group)
                    .and_then(|slot| slot.value.as_ref())
                else {
                    return Err(RendererError::InvalidHandle {
                        kind: ResourceKind::LodGroup,
                        raw: lod_group.raw().get(),
                    });
                };
                for level in &group.levels {
                    self.require_resource(ResourceKind::Mesh, level.mesh)?;
                    for material in &level.materials {
                        self.require_resource(ResourceKind::Material, *material)?;
                        if validation_checks_deep_resource_dependencies(mode) {
                            self.validate_material_resource_dependencies(*material)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_environment_resource_dependencies(
        &self,
        environment: EnvironmentHandle,
    ) -> Result<(), RendererError> {
        let desc = self
            .environment_desc(environment)
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Environment,
                raw: environment.raw().get(),
            })?;
        for texture in [
            desc.skybox,
            desc.irradiance,
            desc.prefiltered_specular,
            desc.brdf_lut,
            desc.texture,
        ]
        .into_iter()
        .flatten()
        {
            self.validated_texture_desc(texture, TextureUsage::SAMPLED, "environment texture")?;
        }
        Ok(())
    }

    fn validate_material_resource_dependencies(
        &self,
        material: MaterialHandle,
    ) -> Result<(), RendererError> {
        for texture in material_texture_handles(self, material)? {
            self.validated_texture_desc(texture, TextureUsage::SAMPLED, "material texture")?;
        }
        Ok(())
    }

    fn validate_graph_extensions(
        &self,
        extensions: &[RenderGraphExtensionHandle],
    ) -> Result<(), RendererError> {
        for extension in extensions {
            if self
                .graph_extensions
                .get(ResourceKind::RenderGraphExtension, *extension)
                .is_none()
            {
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::RenderGraphExtension,
                    raw: extension.raw().get(),
                });
            }
        }
        Ok(())
    }

    fn require_resource<T>(
        &self,
        kind: ResourceKind,
        handle: Handle<T>,
    ) -> Result<(), RendererError> {
        match self.resource_status(handle) {
            Some(ResourceStatus::Ready) => Ok(()),
            Some(ResourceStatus::DestroyQueued) => Err(RendererError::InvalidHandle {
                kind,
                raw: handle.raw().get(),
            }),
            Some(_) => Err(RendererError::ResourceNotReady(kind)),
            None => Err(RendererError::InvalidHandle {
                kind,
                raw: handle.raw().get(),
            }),
        }
    }

    fn object_batch_keys(
        &self,
        object_handle: ObjectHandle,
        object: &RenderObjectDesc,
        view: &ViewDesc,
    ) -> Result<Vec<BatchKey>, RendererError> {
        let (mesh, materials) = self.selected_object_resources(object, view)?;
        let stored_mesh = self
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            })?;
        let batch_discriminator =
            object_batch_discriminator(object_handle, object, stored_mesh.info.flags);
        let draw_items = self.object_draw_items(object_handle, object, view)?;
        if !draw_items.is_empty() {
            let mut draw_items = draw_items;
            sort_view_draw_items(&mut draw_items);
            return Ok(draw_items
                .iter()
                .map(|item| {
                    let render_state_hash =
                        object_render_state_hash(item.pipeline_key.render_state_hash, object);
                    (
                        item.mesh.raw().get(),
                        item.submesh_index,
                        render_phase_sort_rank(item.pipeline_key.pass),
                        item.material.raw().get(),
                        render_state_hash,
                        batch_discriminator,
                    )
                })
                .collect());
        }
        let phase_candidates =
            view_batch_phase_candidates(effective_render_path(&self.config, view));
        let mut keys = Vec::new();
        for (submesh_index, submesh) in stored_mesh.submeshes.iter().enumerate() {
            let material = material_for_submesh(materials, submesh.material_slot);
            let material_id = material.map_or(0, |material| material.raw().get());
            let render_state_hash = material
                .map(|material| self.material_render_state_hash(material))
                .transpose()?
                .unwrap_or(0);
            let render_state_hash = object_render_state_hash(render_state_hash, object);
            let Some(material) = material else {
                let phase = phase_candidates.first().copied();
                keys.push((
                    mesh.raw().get(),
                    submesh_index as u32,
                    phase.map(render_phase_sort_rank).unwrap_or(0),
                    material_id,
                    render_state_hash,
                    batch_discriminator,
                ));
                continue;
            };
            for phase in phase_candidates.iter().copied() {
                if !self.material_supports_phase(material, phase)? {
                    continue;
                }
                keys.push((
                    mesh.raw().get(),
                    submesh_index as u32,
                    render_phase_sort_rank(phase),
                    material_id,
                    render_state_hash,
                    batch_discriminator,
                ));
            }
        }
        Ok(keys)
    }

    #[cfg(feature = "backend-wgpu")]
    fn view_pipeline_keys(&self, view: &ViewDesc) -> Result<Vec<PipelineKey>, RendererError> {
        Ok(self
            .view_draw_items(view)?
            .into_iter()
            .map(|item| item.pipeline_key)
            .collect())
    }

    fn view_draw_items(&self, view: &ViewDesc) -> Result<Vec<DrawItem>, RendererError> {
        let scene = self
            .scenes
            .get(ResourceKind::Scene, view.scene)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: view.scene.raw().get(),
            })?;
        let mut draw_items = Vec::new();
        for (object_index, slot) in scene.objects.resources.iter().enumerate() {
            let Some(object) = &slot.value else {
                continue;
            };
            if !matches!(
                view_object_visibility(scene, object, view),
                ViewObjectVisibility::Visible
            ) {
                continue;
            }
            let object_handle =
                make_handle(ResourceKind::Object, object_index as u32, slot.generation);
            draw_items.extend(self.object_draw_items(object_handle, object, view)?);
        }
        sort_view_draw_items(&mut draw_items);
        coalesce_instanced_draw_items(draw_items)
    }

    fn object_draw_items(
        &self,
        object_handle: ObjectHandle,
        object: &RenderObjectDesc,
        view: &ViewDesc,
    ) -> Result<Vec<DrawItem>, RendererError> {
        let (mesh, materials) = self.selected_object_resources(object, view)?;
        let stored_mesh = self
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            })?;
        let phase_candidates =
            view_batch_phase_candidates(effective_render_path(&self.config, view));
        let sort_key = object_sort_key(object, view);
        let batch_discriminator =
            object_batch_discriminator(object_handle, object, stored_mesh.info.flags);
        let mut items = Vec::new();
        for (submesh_index, submesh) in stored_mesh.submeshes.iter().enumerate() {
            let Some(material) = material_for_submesh(materials, submesh.material_slot) else {
                continue;
            };
            for pipeline_key in
                self.material_pipeline_keys(material, mesh, phase_candidates, view)?
            {
                let render_state_hash =
                    object_render_state_hash(pipeline_key.render_state_hash, object);
                items.push(DrawItem {
                    object: object_handle,
                    mesh,
                    submesh_index: submesh_index as u32,
                    material,
                    pipeline_key,
                    sort_key,
                    instance_range: 0..1,
                    batch_key: (
                        mesh.raw().get(),
                        submesh_index as u32,
                        render_phase_sort_rank(pipeline_key.pass),
                        material.raw().get(),
                        render_state_hash,
                        batch_discriminator,
                    ),
                });
            }
        }
        Ok(items)
    }

    fn material_pipeline_keys(
        &self,
        material: MaterialHandle,
        mesh: MeshHandle,
        phase_candidates: &[RenderPhaseKind],
        view: &ViewDesc,
    ) -> Result<Vec<PipelineKey>, RendererError> {
        let stored = self
            .materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                raw: material.raw().get(),
            })?;
        let Some(template) = stored.template else {
            return Ok(Vec::new());
        };
        let template_desc = self
            .material_templates
            .get(ResourceKind::MaterialTemplate, template)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::MaterialTemplate,
                raw: template.raw().get(),
            })?;
        let quality = effective_view_quality(view);
        let mut keys = Vec::new();
        for phase in phase_candidates.iter().copied() {
            if !self.material_supports_phase(material, phase)? {
                continue;
            }
            keys.push(PipelineKey {
                shader: template_desc.shader,
                material_template: template,
                vertex_layout_hash: mesh.raw().get(),
                render_state_hash: self.material_render_state_hash(material)?,
                pass: phase,
                sample_count: self.config.msaa_samples.min(u32::from(u8::MAX)) as u8,
                depth_format: self.config.depth_format,
                color_format: view_main_color_format(self, view, &quality)?,
                feature_bits: u64::from(self.material_pass_flags(material)?.0),
            });
        }
        Ok(keys)
    }

    fn material_render_state_hash(&self, material: MaterialHandle) -> Result<u64, RendererError> {
        let stored = self
            .materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                raw: material.raw().get(),
            })?;
        if let Some(standard) = &stored.standard {
            return Ok(standard_material_render_state_hash(standard));
        }
        let render_state = stored.overrides.render_state.as_ref().or_else(|| {
            stored.template.and_then(|template| {
                self.material_templates
                    .get(ResourceKind::MaterialTemplate, template)
                    .and_then(|slot| slot.value.as_ref())
                    .map(|template| &template.render_state)
            })
        });
        Ok(render_state.map_or(0, render_state_hash))
    }

    fn material_pass_flags(
        &self,
        material: MaterialHandle,
    ) -> Result<MaterialPassFlags, RendererError> {
        let stored = self
            .materials
            .get(ResourceKind::Material, material)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                raw: material.raw().get(),
            })?;
        if let Some(standard) = &stored.standard {
            return Ok(standard_material_pass_flags(standard));
        }
        if let Some(template) = stored.template {
            return self
                .material_templates
                .get(ResourceKind::MaterialTemplate, template)
                .and_then(|slot| slot.value.as_ref())
                .map(|template| template.passes)
                .ok_or(RendererError::InvalidHandle {
                    kind: ResourceKind::MaterialTemplate,
                    raw: template.raw().get(),
                });
        }
        Ok(MaterialPassFlags::FORWARD)
    }

    fn material_supports_phase(
        &self,
        material: MaterialHandle,
        phase: RenderPhaseKind,
    ) -> Result<bool, RendererError> {
        Ok(material_pass_flags_contains_phase(
            self.material_pass_flags(material)?,
            phase,
        ))
    }

    fn object_triangle_count(
        &self,
        object: &RenderObjectDesc,
        view: &ViewDesc,
    ) -> Result<u64, RendererError> {
        let (mesh, _) = self.selected_object_resources(object, view)?;
        let Some(mesh) = self
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            });
        };
        Ok(mesh_triangle_count(mesh))
    }

    fn selected_object_resources<'a>(
        &'a self,
        object: &'a RenderObjectDesc,
        view: &ViewDesc,
    ) -> Result<(MeshHandle, &'a [MaterialHandle]), RendererError> {
        let Some(lod_group) = object.lod_group else {
            return Ok((object.mesh, &object.materials));
        };
        let Some(group) = self
            .lod_groups
            .get(ResourceKind::LodGroup, lod_group)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::LodGroup,
                raw: lod_group.raw().get(),
            });
        };
        let camera_position = Vec3::new(
            view.camera.transform[3][0],
            view.camera.transform[3][1],
            view.camera.transform[3][2],
        );
        let object_position = Vec3::new(
            object.transform[3][0],
            object.transform[3][1],
            object.transform[3][2],
        );
        let distance = vec3_distance(camera_position, object_position);
        let level = group
            .levels
            .iter()
            .find(|level| distance <= level.max_distance)
            .or_else(|| group.levels.last())
            .expect("LOD groups are validated as non-empty");
        Ok((level.mesh, &level.materials))
    }
}

#[cfg(feature = "backend-wgpu")]
impl Renderer {
    fn build_legacy_scene(
        &self,
        view: &ViewDesc,
    ) -> Result<engine_render::RenderScene, RendererError> {
        use std::collections::HashMap;

        let stored_scene = self
            .scenes
            .get(ResourceKind::Scene, view.scene)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: view.scene.raw().get(),
            })?;
        let mut scene = engine_render::RenderScene::new(legacy_camera(&view.camera));
        if let ClearOptions::ColorDepth(color) = view.camera.clear {
            scene.set_clear_color(color);
        }
        scene.set_depth(match view.camera.clear {
            ClearOptions::None => engine_render::RenderDepthDesc::disabled(),
            _ => engine_render::RenderDepthDesc::ENABLED,
        });
        scene.set_aspect_ratio(camera_aspect(&view.camera));

        let mut texture_map = HashMap::new();
        for (index, slot) in self.textures.resources.iter().enumerate() {
            let Some(texture) = &slot.value else {
                continue;
            };
            let source =
                make_handle::<TextureTag>(ResourceKind::Texture, index as u32, slot.generation);
            if let Some(legacy_texture) = legacy_texture(texture)? {
                texture_map.insert(source.raw().get(), scene.add_texture(legacy_texture));
            }
        }

        let mut environment_map = HashMap::new();
        for (index, slot) in self.environments.resources.iter().enumerate() {
            let Some(environment) = &slot.value else {
                continue;
            };
            let source = make_handle::<EnvironmentTag>(
                ResourceKind::Environment,
                index as u32,
                slot.generation,
            );
            environment_map.insert(
                source.raw().get(),
                legacy_environment(environment, &texture_map),
            );
        }

        let mut material_map = HashMap::new();
        for (index, slot) in self.materials.resources.iter().enumerate() {
            let Some(material) = &slot.value else {
                continue;
            };
            let source =
                make_handle::<MaterialTag>(ResourceKind::Material, index as u32, slot.generation);
            let legacy = legacy_material(material, &texture_map)?;
            material_map.insert(source.raw().get(), scene.add_material(legacy));
        }

        let mut mesh_map = HashMap::new();
        for (index, slot) in self.meshes.resources.iter().enumerate() {
            let Some(mesh) = &slot.value else {
                continue;
            };
            let source = make_handle::<MeshTag>(ResourceKind::Mesh, index as u32, slot.generation);
            let legacy = legacy_mesh(mesh)?;
            mesh_map.insert(source.raw().get(), scene.add_mesh(legacy));
        }

        for slot in &stored_scene.objects.resources {
            let Some(object) = &slot.value else {
                continue;
            };
            if view_object_visibility(stored_scene, object, view) != ViewObjectVisibility::Visible {
                continue;
            }
            let (mesh_handle, material_handles) = self.selected_object_resources(object, view)?;
            let Some(mesh) = mesh_map.get(&mesh_handle.raw().get()).copied() else {
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Mesh,
                    raw: mesh_handle.raw().get(),
                });
            };
            let material = material_handles
                .first()
                .and_then(|material| material_map.get(&material.raw().get()).copied())
                .unwrap_or_else(|| scene.default_material());
            scene.add_instance_with_material_matrix(
                mesh,
                material,
                engine_render::Mat4::from_cols_array(object.transform),
            );
        }

        self.append_debug_draw_to_legacy_scene(&mut scene);
        scene.set_lighting(legacy_lighting(stored_scene, &environment_map));
        Ok(scene)
    }

    fn append_debug_draw_to_legacy_scene(&self, scene: &mut engine_render::RenderScene) {
        for command in &self.debug_draw_commands {
            match command {
                DebugDrawCommand::Line { a, b, color } => {
                    add_debug_line(scene, *a, *b, *color);
                }
                DebugDrawCommand::Ray {
                    origin,
                    dir,
                    len,
                    color,
                } => {
                    let Some(direction) = normalize_vec3(*dir) else {
                        continue;
                    };
                    let end = add_vec3(*origin, scale_vec3(direction, *len));
                    add_debug_line(scene, *origin, end, *color);
                }
                DebugDrawCommand::Aabb { bounds, color } => {
                    add_debug_aabb(scene, *bounds, *color);
                }
                DebugDrawCommand::Sphere {
                    center,
                    radius,
                    color,
                } => {
                    add_debug_sphere(scene, *center, *radius, *color);
                }
                DebugDrawCommand::Frustum { view_proj, color } => {
                    add_debug_frustum(scene, *view_proj, *color);
                }
                DebugDrawCommand::Text3d {
                    position,
                    text,
                    color,
                } => {
                    add_debug_text_3d(scene, *position, text, *color);
                }
            }
        }
    }
}

fn write_range(target: &mut Vec<u8>, byte_offset: u64, data: &[u8]) -> Result<(), RendererError> {
    let offset = usize::try_from(byte_offset)
        .map_err(|_| RendererError::Validation("byte offset is too large".to_owned()))?;
    let end = offset
        .checked_add(data.len())
        .ok_or_else(|| RendererError::Validation("byte range overflows".to_owned()))?;
    if target.len() < end {
        target.resize(end, 0);
    }
    target[offset..end].copy_from_slice(data);
    Ok(())
}

fn validate_shader_desc(desc: &ShaderDesc<'_>) -> Result<(), RendererError> {
    if desc.stages.0 == 0 {
        return Err(RendererError::ShaderCompile(
            "shader must declare at least one stage".to_owned(),
        ));
    }
    validate_shader_source(&desc.source)?;
    validate_shader_entry_points(desc)?;
    if let ShaderReflectionMode::Explicit(interface) = &desc.reflection {
        validate_shader_interface(interface)?;
    }
    let mut feature_flags = std::collections::HashSet::new();
    for flag in &desc.features.flags {
        if flag.trim().is_empty() {
            return Err(RendererError::Validation(
                "shader feature flags must not be empty".to_owned(),
            ));
        }
        if !feature_flags.insert(flag.as_str()) {
            return Err(RendererError::Validation(format!(
                "duplicate shader feature flag '{flag}'"
            )));
        }
    }
    Ok(())
}

fn stored_shader_source(source: &ShaderSource<'_>) -> StoredShaderSource {
    match source {
        ShaderSource::Wgsl(source) => StoredShaderSource::Wgsl((*source).to_owned()),
        ShaderSource::SpirV(words) => StoredShaderSource::SpirV(words.to_vec()),
        ShaderSource::Msl(source) => StoredShaderSource::Msl((*source).to_owned()),
        ShaderSource::Hlsl(source) => StoredShaderSource::Hlsl((*source).to_owned()),
        ShaderSource::Slang(source) => StoredShaderSource::Slang((*source).to_owned()),
        ShaderSource::File(path) => StoredShaderSource::File(path.clone()),
    }
}

fn shader_hot_reload_key(desc: &ShaderDesc<'_>) -> Option<String> {
    desc.hot_reload_key.clone().or_else(|| match &desc.source {
        ShaderSource::File(path) => Some(path.to_string_lossy().into_owned()),
        _ => None,
    })
}

fn validate_shader_reload_compatible(
    old: &ShaderInterfaceDesc,
    new: &ShaderInterfaceDesc,
) -> Result<(), RendererError> {
    if old.resources != new.resources {
        return Err(RendererError::ShaderCompile(
            "shader hot reload changed resource bindings".to_owned(),
        ));
    }
    if old.push_constants != new.push_constants {
        return Err(RendererError::ShaderCompile(
            "shader hot reload changed push constant layout".to_owned(),
        ));
    }
    if old.vertex_inputs != new.vertex_inputs {
        return Err(RendererError::ShaderCompile(
            "shader hot reload changed vertex input layout".to_owned(),
        ));
    }
    Ok(())
}

fn validate_shader_source(source: &ShaderSource<'_>) -> Result<(), RendererError> {
    let empty = match source {
        ShaderSource::Wgsl(source)
        | ShaderSource::Msl(source)
        | ShaderSource::Hlsl(source)
        | ShaderSource::Slang(source) => source.trim().is_empty(),
        ShaderSource::SpirV(words) => words.is_empty(),
        ShaderSource::File(path) => path.as_os_str().is_empty(),
    };
    if empty {
        return Err(RendererError::ShaderCompile(
            "shader source must not be empty".to_owned(),
        ));
    }
    if let ShaderSource::File(path) = source {
        let metadata = fs::metadata(path).map_err(|err| {
            RendererError::ShaderCompile(format!("failed to read shader file {path:?}: {err}"))
        })?;
        if !metadata.is_file() {
            return Err(RendererError::ShaderCompile(format!(
                "shader file source must be a file: {path:?}"
            )));
        }
    }
    Ok(())
}

fn validate_shader_entry_points(desc: &ShaderDesc<'_>) -> Result<(), RendererError> {
    if desc.stages.contains(ShaderStages::VERTEX) && desc.entry_points.vertex.is_none() {
        return Err(RendererError::ShaderCompile(
            "vertex shader stage requires a vertex entry point".to_owned(),
        ));
    }
    if desc.stages.contains(ShaderStages::FRAGMENT) && desc.entry_points.fragment.is_none() {
        return Err(RendererError::ShaderCompile(
            "fragment shader stage requires a fragment entry point".to_owned(),
        ));
    }
    if desc.stages.contains(ShaderStages::COMPUTE) && desc.entry_points.compute.is_none() {
        return Err(RendererError::ShaderCompile(
            "compute shader stage requires a compute entry point".to_owned(),
        ));
    }
    for entry in [
        desc.entry_points.vertex,
        desc.entry_points.fragment,
        desc.entry_points.compute,
    ]
    .into_iter()
    .flatten()
    {
        if entry.trim().is_empty() {
            return Err(RendererError::ShaderCompile(
                "shader entry points must not be empty".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_shader_interface(interface: &ShaderInterfaceDesc) -> Result<(), RendererError> {
    let mut resource_names = std::collections::HashSet::new();
    for resource in &interface.resources {
        if resource.name.trim().is_empty() {
            return Err(RendererError::Validation(
                "shader resource binding names must not be empty".to_owned(),
            ));
        }
        if !resource_names.insert(resource.name.as_str()) {
            return Err(RendererError::Validation(format!(
                "duplicate shader resource binding '{}'",
                resource.name
            )));
        }
        if resource.visibility.0 == 0 {
            return Err(RendererError::Validation(
                "shader resource binding visibility must not be empty".to_owned(),
            ));
        }
        let valid_type = match resource.binding_class {
            BindingClass::Uniform | BindingClass::Storage => resource.ty == BindingType::Buffer,
            BindingClass::Texture => matches!(resource.ty, BindingType::Texture(_)),
            BindingClass::Sampler => resource.ty == BindingType::Sampler,
        };
        if !valid_type {
            return Err(RendererError::Validation(format!(
                "shader resource binding '{}' has type incompatible with binding class {:?}",
                resource.name, resource.binding_class
            )));
        }
    }
    for push in &interface.push_constants {
        if push.stages.0 == 0 || push.range.is_empty() {
            return Err(RendererError::Validation(
                "shader push constants require stages and a non-empty range".to_owned(),
            ));
        }
    }
    for (index, push) in interface.push_constants.iter().enumerate() {
        for other in interface.push_constants.iter().skip(index + 1) {
            if push.stages.intersects(other.stages) && ranges_overlap(&push.range, &other.range) {
                return Err(RendererError::Validation(
                    "shader push constant ranges must not overlap for the same stage".to_owned(),
                ));
            }
        }
    }
    let mut vertex_semantics = std::collections::HashSet::new();
    for input in &interface.vertex_inputs {
        if !vertex_semantics.insert(input.semantic) {
            return Err(RendererError::Validation(format!(
                "duplicate shader vertex input semantic {:?}",
                input.semantic
            )));
        }
    }
    Ok(())
}

fn ranges_overlap(a: &Range<u32>, b: &Range<u32>) -> bool {
    a.start < b.end && b.start < a.end
}

fn shader_interface_from_desc(desc: &ShaderDesc<'_>) -> Result<ShaderInterfaceDesc, RendererError> {
    let interface = match &desc.reflection {
        ShaderReflectionMode::Disabled => Ok(ShaderInterfaceDesc::default()),
        ShaderReflectionMode::Explicit(interface) => Ok(interface.clone()),
        ShaderReflectionMode::Auto => match desc.source {
            ShaderSource::Wgsl(source) => reflect_wgsl_interface(source, desc.stages),
            ShaderSource::File(ref path) if shader_file_is_wgsl(path) => {
                let source = load_shader_file_source(path)?;
                reflect_wgsl_interface(&source, desc.stages)
            }
            _ => Err(RendererError::UnsupportedFeature(
                RendererFeature::ShaderReflection,
            )),
        },
    }?;
    validate_shader_interface(&interface)?;
    Ok(interface)
}

fn shader_file_is_wgsl(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wgsl"))
}

fn load_shader_file_source(path: &PathBuf) -> Result<String, RendererError> {
    fs::read_to_string(path).map_err(|err| {
        RendererError::ShaderCompile(format!("failed to read shader file {path:?}: {err}"))
    })
}

fn reflect_wgsl_interface(
    source: &str,
    visibility: ShaderStages,
) -> Result<ShaderInterfaceDesc, RendererError> {
    let mut interface = ShaderInterfaceDesc::default();
    let lines: Vec<&str> = source.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let mut line = strip_wgsl_comment(lines[index]).trim().to_owned();
        while line.contains("@group") && !line.contains(';') && index + 1 < lines.len() {
            index += 1;
            line.push(' ');
            line.push_str(strip_wgsl_comment(lines[index]).trim());
        }
        if line.contains("@group") && line.contains("@binding") && line.contains("var") {
            if let Some(binding) = reflect_wgsl_binding(&line, visibility)? {
                interface.resources.push(binding);
            }
        }
        index += 1;
    }
    Ok(interface)
}

fn strip_wgsl_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(before, _)| before)
}

fn reflect_wgsl_binding(
    line: &str,
    visibility: ShaderStages,
) -> Result<Option<ShaderResourceBinding>, RendererError> {
    let Some(var_pos) = line.find("var") else {
        return Ok(None);
    };
    let after_var = line[var_pos + 3..].trim_start();
    let after_address_space = if after_var.starts_with('<') {
        let end = after_var.find('>').ok_or_else(|| {
            RendererError::ShaderCompile("unterminated WGSL resource address space".to_owned())
        })?;
        after_var[end + 1..].trim_start()
    } else {
        after_var
    };
    let (name, after_name) = after_address_space.split_once(':').ok_or_else(|| {
        RendererError::ShaderCompile("WGSL resource binding requires `name: type`".to_owned())
    })?;
    let name = name.trim().trim_end_matches(',').to_owned();
    if name.is_empty() {
        return Err(RendererError::ShaderCompile(
            "WGSL resource binding name must not be empty".to_owned(),
        ));
    }
    let ty_text = after_name.trim().trim_end_matches(';').trim();
    let (binding_class, ty) = classify_wgsl_binding_type(ty_text);
    Ok(Some(ShaderResourceBinding {
        name,
        binding_class,
        visibility,
        ty,
    }))
}

fn classify_wgsl_binding_type(ty_text: &str) -> (BindingClass, BindingType) {
    if ty_text.starts_with("sampler") {
        return (BindingClass::Sampler, BindingType::Sampler);
    }
    if ty_text.starts_with("texture_") {
        return (
            BindingClass::Texture,
            BindingType::Texture(wgsl_texture_dimension(ty_text)),
        );
    }
    if ty_text.starts_with("array<") || ty_text.starts_with("atomic<") {
        return (BindingClass::Storage, BindingType::Buffer);
    }
    (BindingClass::Uniform, BindingType::Buffer)
}

fn wgsl_texture_dimension(ty_text: &str) -> TextureDimension {
    if ty_text.contains("_3d") {
        TextureDimension::D3
    } else if ty_text.contains("_cube_array") {
        TextureDimension::CubeArray
    } else if ty_text.contains("_cube") {
        TextureDimension::Cube
    } else if ty_text.contains("_2d_array") {
        TextureDimension::D2Array
    } else {
        TextureDimension::D2
    }
}

fn validate_material_parameters(
    schema: &MaterialParameterSchema,
    parameters: &[MaterialParameter],
) -> Result<(), RendererError> {
    validate_material_parameter_name_set(
        &parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
    )?;
    if schema.parameters.is_empty() {
        return Ok(());
    }
    for parameter in parameters {
        if !schema.parameters.iter().any(|name| name == &parameter.name) {
            return Err(RendererError::MaterialParameterMismatch(format!(
                "material parameter `{}` is not declared by the template schema",
                parameter.name
            )));
        }
    }
    Ok(())
}

fn validate_material_parameter_name_set(names: &[&str]) -> Result<(), RendererError> {
    let mut unique_names = std::collections::HashSet::new();
    for name in names {
        if name.trim().is_empty() {
            return Err(RendererError::MaterialParameterMismatch(
                "material parameter names must not be empty".to_owned(),
            ));
        }
        if !unique_names.insert(*name) {
            return Err(RendererError::MaterialParameterMismatch(format!(
                "duplicate material parameter '{name}'"
            )));
        }
    }
    Ok(())
}

fn validate_material_parameter_bindings(
    interface: &ShaderInterfaceDesc,
    parameters: &[MaterialParameter],
) -> Result<(), RendererError> {
    for parameter in parameters {
        let Some(binding) = interface
            .resources
            .iter()
            .find(|binding| binding.name == parameter.name)
        else {
            continue;
        };
        let valid = match binding.binding_class {
            BindingClass::Texture => matches!(parameter.value, MaterialParameterValue::Texture(_)),
            BindingClass::Sampler => matches!(parameter.value, MaterialParameterValue::Sampler(_)),
            BindingClass::Uniform | BindingClass::Storage => {
                matches!(parameter.value, MaterialParameterValue::Bytes(_))
            }
        };
        if !valid {
            return Err(RendererError::MaterialParameterMismatch(format!(
                "material parameter '{}' does not match shader binding class {:?}",
                parameter.name, binding.binding_class
            )));
        }
    }
    Ok(())
}

fn validate_material_template_desc(desc: &MaterialTemplateDesc) -> Result<(), RendererError> {
    if desc.passes.0 == 0 {
        return Err(RendererError::Validation(
            "material template must declare at least one render pass".to_owned(),
        ));
    }
    if desc.domain == MaterialDomain::Transparent
        && !desc.passes.contains(MaterialPassFlags::TRANSPARENT)
    {
        return Err(RendererError::Validation(
            "transparent material templates must declare the transparent pass".to_owned(),
        ));
    }
    if desc.domain != MaterialDomain::Transparent && desc.passes == MaterialPassFlags::TRANSPARENT {
        return Err(RendererError::Validation(
            "non-transparent material templates must not declare only the transparent pass"
                .to_owned(),
        ));
    }
    let mut names = std::collections::HashSet::new();
    for name in &desc.parameter_schema.parameters {
        if name.trim().is_empty() {
            return Err(RendererError::MaterialParameterMismatch(
                "material template parameter names must not be empty".to_owned(),
            ));
        }
        if !names.insert(name.as_str()) {
            return Err(RendererError::MaterialParameterMismatch(format!(
                "duplicate material template parameter '{name}'"
            )));
        }
    }
    Ok(())
}

fn validate_material_template_shader(
    desc: &MaterialTemplateDesc,
    shader: &ShaderInfo,
) -> Result<(), RendererError> {
    let graphics_stage =
        ShaderStages::VERTEX | ShaderStages::FRAGMENT | ShaderStages::MESH | ShaderStages::TASK;
    if desc.passes.0 != 0
        && shader.stages.contains(ShaderStages::COMPUTE)
        && !shader.stages.intersects(graphics_stage)
    {
        return Err(RendererError::Validation(
            "material template render passes require a graphics shader stage".to_owned(),
        ));
    }
    Ok(())
}

fn validate_buffer_desc(desc: &BufferDesc<'_>) -> Result<(), RendererError> {
    if desc.size == 0 {
        return Err(RendererError::Validation(
            "buffer size must be non-zero".to_owned(),
        ));
    }
    if desc.usage == BufferUsage::empty() {
        return Err(RendererError::Validation(
            "buffer usage must not be empty".to_owned(),
        ));
    }
    if let Some(initial_data) = desc.initial_data {
        if initial_data.len() as u64 > desc.size {
            return Err(RendererError::Validation(
                "buffer initial_data exceeds buffer size".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_buffer_update(size: u64, update: &BufferUpdate<'_>) -> Result<(), RendererError> {
    let update_size = u64::try_from(update.data.len()).map_err(|_| {
        RendererError::Validation("buffer update data length is too large".to_owned())
    })?;
    if update_size == 0 {
        return Err(RendererError::Validation(
            "buffer update data must be non-empty".to_owned(),
        ));
    }
    if update
        .byte_offset
        .checked_add(update_size)
        .is_none_or(|end| end > size)
    {
        return Err(RendererError::Validation(
            "buffer update range exceeds buffer size".to_owned(),
        ));
    }
    Ok(())
}

fn validate_texture_update_region(
    texture: &TextureDescOwned,
    update: &TextureUpdate<'_>,
) -> Result<(), RendererError> {
    if update.subresource.mip_level >= texture.mip_levels {
        return Err(RendererError::Validation(
            "texture update mip level exceeds texture mip levels".to_owned(),
        ));
    }
    if update.subresource.array_layer >= texture.depth_or_layers {
        return Err(RendererError::Validation(
            "texture update array layer exceeds texture layers".to_owned(),
        ));
    }
    if matches!(texture.dimension, TextureDimension::D3) && update.subresource.array_layer != 0 {
        return Err(RendererError::Validation(
            "D3 texture updates must use array_layer 0 and address depth through region z"
                .to_owned(),
        ));
    }
    let mip_width = mip_extent(texture.width, update.subresource.mip_level);
    let mip_height = mip_extent(texture.height, update.subresource.mip_level);
    let mip_depth = if matches!(texture.dimension, TextureDimension::D3) {
        mip_extent(texture.depth_or_layers, update.subresource.mip_level)
    } else {
        1
    };
    let [offset_x, offset_y, offset_z] = update.region.offset;
    let [extent_x, extent_y, extent_z] = update.region.extent;
    if extent_x == 0 || extent_y == 0 || extent_z == 0 {
        return Err(RendererError::Validation(
            "texture update region extent must be non-zero".to_owned(),
        ));
    }
    if offset_x
        .checked_add(extent_x)
        .is_none_or(|end| end > mip_width)
        || offset_y
            .checked_add(extent_y)
            .is_none_or(|end| end > mip_height)
        || offset_z
            .checked_add(extent_z)
            .is_none_or(|end| end > mip_depth)
    {
        return Err(RendererError::Validation(
            "texture update region exceeds texture mip extent".to_owned(),
        ));
    }
    validate_texture_layout(
        update.data.len(),
        update.bytes_per_row,
        update.rows_per_image,
        extent_x,
        extent_y,
        extent_z,
        texture.format,
    )
}

fn validate_texture_dimension(desc: &TextureDesc<'_>) -> Result<(), RendererError> {
    match desc.dimension {
        TextureDimension::D1 => {
            if desc.height != 1 || desc.depth_or_layers != 1 {
                return Err(RendererError::Validation(
                    "D1 textures must have height 1 and one layer".to_owned(),
                ));
            }
        }
        TextureDimension::D2 => {
            if desc.depth_or_layers != 1 {
                return Err(RendererError::Validation(
                    "D2 textures must have exactly one layer".to_owned(),
                ));
            }
        }
        TextureDimension::D3 | TextureDimension::D2Array => {}
        TextureDimension::Cube => {
            if desc.width != desc.height || desc.depth_or_layers != 6 {
                return Err(RendererError::Validation(
                    "cube textures must be square with exactly six layers".to_owned(),
                ));
            }
        }
        TextureDimension::CubeArray => {
            if desc.width != desc.height || desc.depth_or_layers % 6 != 0 {
                return Err(RendererError::Validation(
                    "cube array textures must be square with a layer count divisible by six"
                        .to_owned(),
                ));
            }
        }
    }
    let mip_depth = if matches!(desc.dimension, TextureDimension::D3) {
        desc.depth_or_layers
    } else {
        1
    };
    let max_mips = max_mip_levels(desc.width, desc.height, mip_depth);
    if desc.mip_levels > max_mips {
        return Err(RendererError::Validation(
            "texture mip_levels exceeds texture extent".to_owned(),
        ));
    }
    if desc.samples > 1 && desc.mip_levels > 1 {
        return Err(RendererError::Validation(
            "multisampled textures must have exactly one mip level".to_owned(),
        ));
    }
    if desc.samples > 1 && !matches!(desc.dimension, TextureDimension::D2) {
        return Err(RendererError::Validation(
            "multisampled textures must be 2D textures".to_owned(),
        ));
    }
    if !desc.samples.is_power_of_two() {
        return Err(RendererError::Validation(
            "texture samples must be a power of two".to_owned(),
        ));
    }
    Ok(())
}

fn validate_direct_render_target_texture_shape(
    texture: &TextureDescOwned,
    role: &str,
) -> Result<(), RendererError> {
    if !matches!(texture.dimension, TextureDimension::D2) || texture.depth_or_layers != 1 {
        return Err(RendererError::Validation(format!(
            "{role} must be a single-layer D2 texture; use TextureView for subresources"
        )));
    }
    if texture.mip_levels != 1 {
        return Err(RendererError::Validation(format!(
            "{role} must have exactly one mip level; use TextureView for subresources"
        )));
    }
    Ok(())
}

fn validate_render_target_texture_view_shape(
    texture: &TextureDescOwned,
) -> Result<(), RendererError> {
    match texture.dimension {
        TextureDimension::D2
        | TextureDimension::D2Array
        | TextureDimension::Cube
        | TextureDimension::CubeArray => Ok(()),
        TextureDimension::D1 | TextureDimension::D3 => Err(RendererError::Validation(
            "render target texture views require a 2D-compatible texture dimension".to_owned(),
        )),
    }
}

fn validate_texture_layout(
    byte_len: usize,
    bytes_per_row: u32,
    rows_per_image: u32,
    width: u32,
    height: u32,
    depth_or_layers: u32,
    format: TextureFormat,
) -> Result<(), RendererError> {
    let row_bytes = width
        .checked_mul(texture_format_bytes_per_pixel(format))
        .ok_or_else(|| RendererError::Validation("texture row byte size overflows".to_owned()))?;
    if bytes_per_row < row_bytes {
        return Err(RendererError::Validation(
            "texture bytes_per_row is smaller than one texel row".to_owned(),
        ));
    }
    if rows_per_image < height {
        return Err(RendererError::Validation(
            "texture rows_per_image is smaller than image height".to_owned(),
        ));
    }
    let required = if depth_or_layers == 0 {
        0
    } else {
        let image_stride = u64::from(bytes_per_row) * u64::from(rows_per_image);
        let last_image = u64::from(depth_or_layers - 1) * image_stride;
        let last_row = u64::from(height - 1) * u64::from(bytes_per_row);
        last_image + last_row + u64::from(row_bytes)
    };
    if required > byte_len as u64 {
        return Err(RendererError::Validation(
            "texture upload data is smaller than declared layout".to_owned(),
        ));
    }
    Ok(())
}

const fn mip_extent(base: u32, mip_level: u32) -> u32 {
    let shifted = base >> mip_level;
    if shifted == 0 {
        1
    } else {
        shifted
    }
}

fn max_mip_levels(width: u32, height: u32, depth_or_layers: u32) -> u32 {
    let mut max_extent = width.max(height).max(depth_or_layers);
    let mut levels = 1;
    while max_extent > 1 {
        max_extent >>= 1;
        levels += 1;
    }
    levels
}

fn generate_texture_mip_chain(texture: &StoredTexture) -> Result<Vec<Vec<u8>>, RendererError> {
    if texture.desc.samples != 1 {
        return Err(RendererError::Validation(
            "generate_mips does not support multisampled textures".to_owned(),
        ));
    }
    if !matches!(
        texture.desc.format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb
    ) {
        return Err(RendererError::Validation(
            "generate_mips currently supports only 8-bit four-channel color textures".to_owned(),
        ));
    }
    if matches!(texture.desc.dimension, TextureDimension::D3) {
        return generate_texture_volume_mip_chain(texture);
    }
    let base_layers = compact_base_level_rgba8_layers(texture)?;
    let mut layer_chains = Vec::with_capacity(base_layers.len());
    let mut level_count = 0;
    for base in base_layers {
        let mut width = texture.desc.width;
        let mut height = texture.desc.height;
        let mut mips = vec![base];
        while width > 1 || height > 1 {
            let previous = mips.last().expect("mip chain has a base level");
            let next_width = (width / 2).max(1);
            let next_height = (height / 2).max(1);
            mips.push(generate_next_rgba8_mip(
                previous,
                width,
                height,
                next_width,
                next_height,
            ));
            width = next_width;
            height = next_height;
        }
        level_count = mips.len();
        layer_chains.push(mips);
    }

    let mut mips = Vec::with_capacity(level_count);
    for level in 0..level_count {
        let mut level_bytes = Vec::new();
        for chain in &layer_chains {
            level_bytes.extend_from_slice(&chain[level]);
        }
        mips.push(level_bytes);
    }
    Ok(mips)
}

fn generate_texture_volume_mip_chain(
    texture: &StoredTexture,
) -> Result<Vec<Vec<u8>>, RendererError> {
    let mut volume = compact_base_level_rgba8_volume(texture)?;
    let mut width = texture.desc.width;
    let mut height = texture.desc.height;
    let mut depth = texture.desc.depth_or_layers;
    let mut mips = vec![volume.clone()];
    while width > 1 || height > 1 || depth > 1 {
        let next_width = (width / 2).max(1);
        let next_height = (height / 2).max(1);
        let next_depth = (depth / 2).max(1);
        volume = generate_next_rgba8_volume_mip(
            &volume,
            [width, height, depth],
            [next_width, next_height, next_depth],
        );
        mips.push(volume.clone());
        width = next_width;
        height = next_height;
        depth = next_depth;
    }
    Ok(mips)
}

fn compact_base_level_rgba8_volume(texture: &StoredTexture) -> Result<Vec<u8>, RendererError> {
    let Some(layout) = texture.layout else {
        return Err(RendererError::Validation(
            "generate_mips requires complete base level upload data".to_owned(),
        ));
    };
    if layout.subresource.mip_level != 0
        || layout.subresource.array_layer != 0
        || layout.region.offset != [0, 0, 0]
        || layout.region.extent
            != [
                texture.desc.width,
                texture.desc.height,
                texture.desc.depth_or_layers,
            ]
    {
        return Err(RendererError::Validation(
            "generate_mips requires a complete base mip upload".to_owned(),
        ));
    }

    let bpp = texture_format_bytes_per_pixel(texture.desc.format);
    let row_bytes = texture.desc.width.checked_mul(bpp).ok_or_else(|| {
        RendererError::Validation("texture base mip row byte size overflows".to_owned())
    })?;
    validate_texture_layout(
        texture.bytes.len(),
        layout.bytes_per_row,
        layout.rows_per_image,
        texture.desc.width,
        texture.desc.height,
        texture.desc.depth_or_layers,
        texture.desc.format,
    )?;

    let mut compact = Vec::with_capacity(
        (row_bytes * texture.desc.height * texture.desc.depth_or_layers) as usize,
    );
    let image_stride = u64::from(layout.bytes_per_row) * u64::from(layout.rows_per_image);
    for z in 0..texture.desc.depth_or_layers {
        let slice_start = u64::from(z) * image_stride;
        for y in 0..texture.desc.height {
            let start = (slice_start + u64::from(y) * u64::from(layout.bytes_per_row)) as usize;
            let end = start + row_bytes as usize;
            compact.extend_from_slice(&texture.bytes[start..end]);
        }
    }
    Ok(compact)
}

fn compact_base_level_rgba8_layers(texture: &StoredTexture) -> Result<Vec<Vec<u8>>, RendererError> {
    let Some(layout) = texture.layout else {
        return Err(RendererError::Validation(
            "generate_mips requires complete base level upload data".to_owned(),
        ));
    };
    let layer_count = texture.desc.depth_or_layers;
    if layout.subresource.mip_level != 0
        || layout.subresource.array_layer != 0
        || layout.region.offset != [0, 0, 0]
        || layout.region.extent != [texture.desc.width, texture.desc.height, layer_count]
    {
        return Err(RendererError::Validation(
            "generate_mips requires a complete base mip upload".to_owned(),
        ));
    }

    let bpp = texture_format_bytes_per_pixel(texture.desc.format);
    let row_bytes = texture.desc.width.checked_mul(bpp).ok_or_else(|| {
        RendererError::Validation("texture base mip row byte size overflows".to_owned())
    })?;
    validate_texture_layout(
        texture.bytes.len(),
        layout.bytes_per_row,
        layout.rows_per_image,
        texture.desc.width,
        texture.desc.height,
        layer_count,
        texture.desc.format,
    )?;

    let mut layers = Vec::with_capacity(layer_count as usize);
    let image_stride = u64::from(layout.bytes_per_row) * u64::from(layout.rows_per_image);
    for layer in 0..layer_count {
        let mut compact = Vec::with_capacity((row_bytes * texture.desc.height) as usize);
        let layer_start = u64::from(layer) * image_stride;
        for y in 0..texture.desc.height {
            let start = (layer_start + u64::from(y) * u64::from(layout.bytes_per_row)) as usize;
            let end = start + row_bytes as usize;
            compact.extend_from_slice(&texture.bytes[start..end]);
        }
        layers.push(compact);
    }
    Ok(layers)
}

fn generate_next_rgba8_mip(
    previous: &[u8],
    previous_width: u32,
    previous_height: u32,
    next_width: u32,
    next_height: u32,
) -> Vec<u8> {
    let mut next = vec![0; (next_width * next_height * 4) as usize];
    for y in 0..next_height {
        let source_y_start = y * previous_height / next_height;
        let source_y_end = ((y + 1) * previous_height / next_height)
            .max(source_y_start + 1)
            .min(previous_height);
        for x in 0..next_width {
            let source_x_start = x * previous_width / next_width;
            let source_x_end = ((x + 1) * previous_width / next_width)
                .max(source_x_start + 1)
                .min(previous_width);
            let mut channels = [0u32; 4];
            let mut sample_count = 0u32;
            for source_y in source_y_start..source_y_end {
                for source_x in source_x_start..source_x_end {
                    let source_offset = ((source_y * previous_width + source_x) * 4) as usize;
                    for (channel, value) in channels.iter_mut().enumerate() {
                        *value += u32::from(previous[source_offset + channel]);
                    }
                    sample_count += 1;
                }
            }
            let destination_offset = ((y * next_width + x) * 4) as usize;
            for (channel, sum) in channels.into_iter().enumerate() {
                next[destination_offset + channel] =
                    ((sum + sample_count / 2) / sample_count) as u8;
            }
        }
    }
    next
}

fn generate_next_rgba8_volume_mip(
    previous: &[u8],
    previous_extent: [u32; 3],
    next_extent: [u32; 3],
) -> Vec<u8> {
    let [previous_width, previous_height, previous_depth] = previous_extent;
    let [next_width, next_height, next_depth] = next_extent;
    let mut next = vec![0; (next_width * next_height * next_depth * 4) as usize];
    for z in 0..next_depth {
        let source_z_start = z * previous_depth / next_depth;
        let source_z_end = ((z + 1) * previous_depth / next_depth)
            .max(source_z_start + 1)
            .min(previous_depth);
        for y in 0..next_height {
            let source_y_start = y * previous_height / next_height;
            let source_y_end = ((y + 1) * previous_height / next_height)
                .max(source_y_start + 1)
                .min(previous_height);
            for x in 0..next_width {
                let source_x_start = x * previous_width / next_width;
                let source_x_end = ((x + 1) * previous_width / next_width)
                    .max(source_x_start + 1)
                    .min(previous_width);
                let mut channels = [0u32; 4];
                let mut sample_count = 0u32;
                for source_z in source_z_start..source_z_end {
                    for source_y in source_y_start..source_y_end {
                        for source_x in source_x_start..source_x_end {
                            let source_offset = (((source_z * previous_height + source_y)
                                * previous_width
                                + source_x)
                                * 4) as usize;
                            for (channel, value) in channels.iter_mut().enumerate() {
                                *value += u32::from(previous[source_offset + channel]);
                            }
                            sample_count += 1;
                        }
                    }
                }
                let destination_offset = (((z * next_height + y) * next_width + x) * 4) as usize;
                for (channel, sum) in channels.into_iter().enumerate() {
                    next[destination_offset + channel] =
                        ((sum + sample_count / 2) / sample_count) as u8;
                }
            }
        }
    }
    next
}

const fn texture_format_bytes_per_pixel(format: TextureFormat) -> u32 {
    match format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Bgra8UnormSrgb
        | TextureFormat::Depth32Float => 4,
        TextureFormat::Rgba16Float => 8,
        TextureFormat::Rgba32Float => 16,
    }
}

fn average_texture_color(texture: &StoredTexture) -> Option<[f32; 4]> {
    let bpp = texture_format_bytes_per_pixel(texture.desc.format) as usize;
    if bpp == 0 || texture.bytes.len() < bpp {
        return None;
    }
    let mut sum = [0.0_f32; 4];
    let mut count = 0_u32;
    for texel in texture.bytes.chunks_exact(bpp) {
        let color = decode_texel_color(texture.desc.format, texel)?;
        for channel in 0..4 {
            sum[channel] += color[channel];
        }
        count += 1;
    }
    (count > 0).then(|| {
        let inv = 1.0 / count as f32;
        [sum[0] * inv, sum[1] * inv, sum[2] * inv, sum[3] * inv]
    })
}

fn decode_texel_color(format: TextureFormat, texel: &[u8]) -> Option<[f32; 4]> {
    let unorm = |value: u8| value as f32 / 255.0;
    match format {
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => Some([
            unorm(*texel.first()?),
            unorm(*texel.get(1)?),
            unorm(*texel.get(2)?),
            unorm(*texel.get(3)?),
        ]),
        TextureFormat::Bgra8UnormSrgb => Some([
            unorm(*texel.get(2)?),
            unorm(*texel.get(1)?),
            unorm(*texel.first()?),
            unorm(*texel.get(3)?),
        ]),
        TextureFormat::Rgba32Float => Some([
            f32::from_le_bytes(texel.get(0..4)?.try_into().ok()?),
            f32::from_le_bytes(texel.get(4..8)?.try_into().ok()?),
            f32::from_le_bytes(texel.get(8..12)?.try_into().ok()?),
            f32::from_le_bytes(texel.get(12..16)?.try_into().ok()?),
        ]),
        _ => None,
    }
}

fn scale_color_rgb(mut color: [f32; 4], scale: f32) -> [f32; 4] {
    color[0] = (color[0] * scale).clamp(0.0, 1.0);
    color[1] = (color[1] * scale).clamp(0.0, 1.0);
    color[2] = (color[2] * scale).clamp(0.0, 1.0);
    color
}

fn solid_texture_bytes(
    width: u32,
    height: u32,
    layers: u32,
    format: TextureFormat,
    color: [f32; 4],
) -> Vec<u8> {
    let texel = encode_texel_color(format, color);
    let texel_count = width as usize * height as usize * layers as usize;
    let mut bytes = Vec::with_capacity(texel.len() * texel_count);
    for _ in 0..texel_count {
        bytes.extend_from_slice(&texel);
    }
    bytes
}

fn encode_texel_color(format: TextureFormat, color: [f32; 4]) -> Vec<u8> {
    let unorm = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    match format {
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => {
            vec![
                unorm(color[0]),
                unorm(color[1]),
                unorm(color[2]),
                unorm(color[3]),
            ]
        }
        TextureFormat::Bgra8UnormSrgb => {
            vec![
                unorm(color[2]),
                unorm(color[1]),
                unorm(color[0]),
                unorm(color[3]),
            ]
        }
        TextureFormat::Rgba16Float => color
            .into_iter()
            .flat_map(|channel| f32_to_f16_bits(channel).to_le_bytes())
            .collect(),
        TextureFormat::Rgba32Float => color
            .into_iter()
            .flat_map(|channel| channel.to_le_bytes())
            .collect(),
        TextureFormat::Depth32Float => color[0].to_le_bytes().to_vec(),
    }
}

fn generate_brdf_lut_rgba16f(size: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(size as usize * size as usize * 8);
    let denom = (size.saturating_sub(1)).max(1) as f32;
    for y in 0..size {
        let roughness = y as f32 / denom;
        for x in 0..size {
            let ndotv = x as f32 / denom;
            let fresnel_scale = (1.0 - roughness * 0.5) * (1.0 - ndotv).powi(5);
            let fresnel_bias = (1.0 - roughness) * ndotv;
            for channel in [fresnel_bias, fresnel_scale, roughness, 1.0] {
                bytes.extend_from_slice(&f32_to_f16_bits(channel).to_le_bytes());
            }
        }
    }
    bytes
}

fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7f_ffff;
    if exponent <= 0 {
        if exponent < -10 {
            return sign;
        }
        let mantissa = mantissa | 0x80_0000;
        let shift = (14 - exponent) as u32;
        return sign | ((mantissa >> shift) as u16);
    }
    if exponent >= 0x1f {
        return sign | 0x7c00;
    }
    sign | ((exponent as u16) << 10) | ((mantissa >> 13) as u16)
}

#[cfg(feature = "backend-wgpu")]
fn legacy_mesh(mesh: &StoredMesh) -> Result<engine_render::Mesh, RendererError> {
    if mesh.vertex_layout.streams.is_empty() {
        return Err(RendererError::Validation(
            "mesh requires at least one vertex stream".to_owned(),
        ));
    }
    if mesh.vertex_layout.streams.len() > mesh.vertex_stream_bytes.len() {
        return Err(RendererError::Validation(
            "mesh vertex stream data is missing".to_owned(),
        ));
    }
    for stream in &mesh.vertex_layout.streams {
        if stream.step == VertexStepMode::Vertex && stream.stride == 0 {
            return Err(RendererError::Validation(
                "mesh vertex stream stride must be non-zero".to_owned(),
            ));
        }
    }
    let vertex_count = vertex_count_for_semantic(mesh, VertexSemantic::Position)
        .ok_or_else(|| RendererError::Validation("mesh requires POSITION Float32x3".to_owned()))?;
    let mut vertices = Vec::with_capacity(vertex_count);
    for index in 0..vertex_count {
        let read_attr = |semantic| read_mesh_vertex_attribute(mesh, index, semantic);
        let position = read_attr(VertexSemantic::Position)
            .and_then(|value| value.xyz())
            .ok_or_else(|| {
                RendererError::Validation("mesh requires POSITION Float32x3".to_owned())
            })?;
        let color = read_attr(VertexSemantic::Color(0))
            .and_then(|value| value.xyz())
            .unwrap_or([1.0, 1.0, 1.0]);
        let alpha = read_attr(VertexSemantic::Color(0))
            .and_then(|value| value.w())
            .unwrap_or(1.0);
        let normal = read_attr(VertexSemantic::Normal)
            .and_then(|value| value.xyz())
            .unwrap_or([0.0, 0.0, 1.0]);
        let uv = read_attr(VertexSemantic::TexCoord(0))
            .and_then(|value| value.xy())
            .unwrap_or([0.0, 0.0]);
        let uv1 = read_attr(VertexSemantic::TexCoord(1))
            .and_then(|value| value.xy())
            .unwrap_or(uv);
        let tangent = read_attr(VertexSemantic::Tangent)
            .and_then(|value| value.xyzw())
            .unwrap_or([1.0, 0.0, 0.0, 1.0]);
        let mut vertex = engine_render::ColoredVertex::with_normal_uvs_tangent(
            position, color, normal, uv, uv1, tangent,
        );
        vertex.alpha = alpha;
        vertices.push(vertex);
    }

    let indices = match mesh.index_format {
        Some(StoredIndexFormat::U16) => mesh
            .index_bytes
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as u32)
            .collect(),
        Some(StoredIndexFormat::U32) => mesh
            .index_bytes
            .chunks_exact(4)
            .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            .collect(),
        None => Vec::new(),
    };
    Ok(engine_render::Mesh::with_indices(vertices, indices))
}

#[cfg(feature = "backend-wgpu")]
#[derive(Clone, Copy)]
enum VertexAttributeValue {
    F2([f32; 2]),
    F3([f32; 3]),
    F4([f32; 4]),
}

#[cfg(feature = "backend-wgpu")]
impl VertexAttributeValue {
    fn xy(self) -> Option<[f32; 2]> {
        match self {
            Self::F2(value) => Some(value),
            Self::F3(value) => Some([value[0], value[1]]),
            Self::F4(value) => Some([value[0], value[1]]),
        }
    }

    fn xyz(self) -> Option<[f32; 3]> {
        match self {
            Self::F3(value) => Some(value),
            Self::F4(value) => Some([value[0], value[1], value[2]]),
            _ => None,
        }
    }

    fn xyzw(self) -> Option<[f32; 4]> {
        match self {
            Self::F4(value) => Some(value),
            _ => None,
        }
    }

    fn w(self) -> Option<f32> {
        match self {
            Self::F4(value) => Some(value[3]),
            _ => None,
        }
    }
}

fn vertex_count_for_semantic(mesh: &StoredMesh, semantic: VertexSemantic) -> Option<usize> {
    mesh.vertex_layout
        .streams
        .iter()
        .zip(mesh.vertex_stream_bytes.iter())
        .find(|(stream, _)| {
            stream.step == VertexStepMode::Vertex
                && stream
                    .attributes
                    .iter()
                    .any(|attribute| attribute.semantic == semantic)
        })
        .and_then(|(stream, bytes)| {
            let stride = stream.stride as usize;
            (stride > 0).then_some(bytes.len() / stride)
        })
}

#[cfg(feature = "backend-wgpu")]
fn read_mesh_vertex_attribute(
    mesh: &StoredMesh,
    index: usize,
    semantic: VertexSemantic,
) -> Option<VertexAttributeValue> {
    mesh.vertex_layout
        .streams
        .iter()
        .zip(mesh.vertex_stream_bytes.iter())
        .filter(|(stream, _)| stream.step == VertexStepMode::Vertex)
        .find_map(|(stream, bytes)| {
            let stride = stream.stride as usize;
            let base = index.checked_mul(stride)?;
            read_vertex_attribute(bytes, base, stream, semantic)
        })
}

#[cfg(feature = "backend-wgpu")]
fn read_vertex_attribute(
    bytes: &[u8],
    base: usize,
    stream: &VertexStreamLayout,
    semantic: VertexSemantic,
) -> Option<VertexAttributeValue> {
    let attribute = stream
        .attributes
        .iter()
        .find(|attribute| attribute.semantic == semantic)?;
    let offset = base.checked_add(attribute.offset as usize)?;
    match attribute.format {
        VertexFormat::Float32x2 => Some(VertexAttributeValue::F2([
            read_f32(bytes, offset)?,
            read_f32(bytes, offset + 4)?,
        ])),
        VertexFormat::Float32x3 => Some(VertexAttributeValue::F3([
            read_f32(bytes, offset)?,
            read_f32(bytes, offset + 4)?,
            read_f32(bytes, offset + 8)?,
        ])),
        VertexFormat::Float32x4 => Some(VertexAttributeValue::F4([
            read_f32(bytes, offset)?,
            read_f32(bytes, offset + 4)?,
            read_f32(bytes, offset + 8)?,
            read_f32(bytes, offset + 12)?,
        ])),
        _ => None,
    }
}

#[cfg(feature = "backend-wgpu")]
fn read_f32(bytes: &[u8], offset: usize) -> Option<f32> {
    let bytes = bytes.get(offset..offset + 4)?;
    Some(f32::from_le_bytes(bytes.try_into().ok()?))
}

#[cfg(feature = "backend-wgpu")]
fn legacy_texture(
    texture: &StoredTexture,
) -> Result<Option<engine_render::Texture>, RendererError> {
    if texture.bytes.is_empty() {
        return Ok(Some(engine_render::Texture::white_1x1()));
    }
    match texture.desc.format {
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => engine_render::Texture::rgba8(
            engine_render::TextureSize::new(texture.desc.width, texture.desc.height),
            texture.bytes.clone(),
        )
        .map(Some)
        .ok_or_else(|| RendererError::Validation("invalid RGBA8 texture data".to_owned())),
        _ => Ok(None),
    }
}

#[cfg(feature = "backend-wgpu")]
fn legacy_material(
    material: &StoredMaterial,
    textures: &std::collections::HashMap<u64, engine_render::TextureHandle>,
) -> Result<engine_render::Material, RendererError> {
    let Some(standard) = &material.standard else {
        return Ok(engine_render::Material::default());
    };
    let tint = [
        standard.base_color.r as f32,
        standard.base_color.g as f32,
        standard.base_color.b as f32,
        standard.base_color.a as f32,
    ];
    let mut material = standard
        .base_color_texture
        .and_then(|handle| textures.get(&handle.raw().get()).copied())
        .map_or_else(
            || engine_render::Material::new(tint),
            |texture| match standard.alpha_mode {
                AlphaMode::Blend | AlphaMode::Premultiplied | AlphaMode::Additive => {
                    engine_render::Material::alpha_blended_textured(tint, texture)
                }
                _ => engine_render::Material::opaque_textured(tint, texture),
            },
        )
        .with_surface(standard.roughness, standard.metallic)
        .with_double_sided(standard.double_sided);
    material = match standard.alpha_mode {
        AlphaMode::Opaque => material,
        AlphaMode::Mask { cutoff } => material.with_alpha_cutoff(cutoff),
        AlphaMode::Blend | AlphaMode::Premultiplied | AlphaMode::Additive => {
            material.with_blend_mode(engine_render::BlendMode::AlphaBlend)
        }
    };
    if standard.domain == MaterialDomain::Unlit {
        material = material.with_unlit(true);
    }
    if let Some(texture) = standard
        .normal_texture
        .and_then(|handle| textures.get(&handle.raw().get()).copied())
    {
        material = material.with_normal_texture(texture, 1.0);
    }
    if let Some(texture) = standard
        .metallic_roughness_texture
        .and_then(|handle| textures.get(&handle.raw().get()).copied())
    {
        material = material.with_metallic_roughness_texture(texture);
    }
    if let Some(texture) = standard
        .emissive_texture
        .and_then(|handle| textures.get(&handle.raw().get()).copied())
    {
        material = material.with_emissive_texture(texture);
    }
    Ok(material.with_emissive([
        standard.emissive.x,
        standard.emissive.y,
        standard.emissive.z,
    ]))
}

#[cfg(feature = "backend-wgpu")]
fn add_debug_line(
    scene: &mut engine_render::RenderScene,
    a: Vec3,
    b: Vec3,
    color: Color,
) -> Option<()> {
    let mesh = debug_line_mesh(a, b, color)?;
    let tint = color_to_tint(color);
    let material = engine_render::Material::new(tint)
        .with_unlit(true)
        .with_double_sided(true)
        .with_blend_mode(if tint[3] < 1.0 {
            engine_render::BlendMode::AlphaBlend
        } else {
            engine_render::BlendMode::Opaque
        });
    let mesh = scene.add_mesh(mesh);
    let material = scene.add_material(material);
    scene.add_instance_with_material_matrix(mesh, material, engine_render::Mat4::IDENTITY);
    Some(())
}

#[cfg(feature = "backend-wgpu")]
fn add_debug_aabb(scene: &mut engine_render::RenderScene, bounds: Bounds3, color: Color) {
    let min = bounds.min;
    let max = bounds.max;
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ];
    for (from, to) in [
        (0, 1),
        (0, 2),
        (1, 3),
        (2, 3),
        (4, 5),
        (4, 6),
        (5, 7),
        (6, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        add_debug_line(scene, corners[from], corners[to], color);
    }
}

#[cfg(feature = "backend-wgpu")]
fn add_debug_sphere(
    scene: &mut engine_render::RenderScene,
    center: Vec3,
    radius: f32,
    color: Color,
) {
    if radius <= 0.0 || !radius.is_finite() {
        return;
    }
    const SEGMENTS: usize = 24;
    let mut xy = Vec::with_capacity(SEGMENTS);
    let mut xz = Vec::with_capacity(SEGMENTS);
    let mut yz = Vec::with_capacity(SEGMENTS);
    for index in 0..SEGMENTS {
        let angle = (index as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        let (sin, cos) = angle.sin_cos();
        xy.push(Vec3::new(
            center.x + cos * radius,
            center.y + sin * radius,
            center.z,
        ));
        xz.push(Vec3::new(
            center.x + cos * radius,
            center.y,
            center.z + sin * radius,
        ));
        yz.push(Vec3::new(
            center.x,
            center.y + cos * radius,
            center.z + sin * radius,
        ));
    }
    for ring in [&xy, &xz, &yz] {
        for index in 0..SEGMENTS {
            add_debug_line(scene, ring[index], ring[(index + 1) % SEGMENTS], color);
        }
    }
}

#[cfg(feature = "backend-wgpu")]
fn add_debug_frustum(scene: &mut engine_render::RenderScene, view_proj: Mat4, color: Color) {
    let Some(inverse) = invert_mat4(view_proj) else {
        return;
    };
    let Some(corners) = frustum_corners_from_inverse_view_projection(inverse) else {
        return;
    };
    for (from, to) in [
        (0, 1),
        (1, 3),
        (3, 2),
        (2, 0),
        (4, 5),
        (5, 7),
        (7, 6),
        (6, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        add_debug_line(scene, corners[from], corners[to], color);
    }
}

#[cfg(feature = "backend-wgpu")]
fn add_debug_text_3d(
    scene: &mut engine_render::RenderScene,
    position: Vec3,
    text: &str,
    color: Color,
) {
    if text.is_empty()
        || !position.x.is_finite()
        || !position.y.is_finite()
        || !position.z.is_finite()
    {
        return;
    }
    const CELL: f32 = 0.05;
    const ADVANCE: f32 = CELL * 6.0;
    for (char_index, ch) in text.chars().enumerate() {
        if ch.is_whitespace() {
            continue;
        }
        let x_offset = char_index as f32 * ADVANCE;
        let glyph = debug_text_glyph(ch);
        for (row, bits) in glyph.iter().copied().enumerate() {
            let mut col = 0;
            while col < 5 {
                if bits & (1 << (4 - col)) == 0 {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < 5 && bits & (1 << (4 - col)) != 0 {
                    col += 1;
                }
                let y = -(row as f32) * CELL;
                let a = add_vec3(position, Vec3::new(x_offset + start as f32 * CELL, y, 0.0));
                let b = add_vec3(position, Vec3::new(x_offset + col as f32 * CELL, y, 0.0));
                add_debug_line(scene, a, b, color);
            }
        }
    }
}

#[cfg(feature = "backend-wgpu")]
fn debug_text_glyph(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b01010, 0b01010, 0b00100, 0b01010, 0b01010, 0b10001,
        ],
        'Y' => [
            0b10001, 0b01010, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        _ => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
    }
}

#[cfg(feature = "backend-wgpu")]
fn debug_line_mesh(a: Vec3, b: Vec3, color: Color) -> Option<engine_render::Mesh> {
    let dir = sub_vec3(b, a);
    let length = vec3_len(dir);
    if length <= f32::EPSILON || !length.is_finite() {
        return None;
    }
    let forward = scale_vec3(dir, 1.0 / length);
    let reference = if forward.y.abs() < 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let right = normalize_vec3(cross_vec3(forward, reference))?;
    let up = normalize_vec3(cross_vec3(right, forward))?;
    let half_width = (length * 0.0025).clamp(0.0025, 0.025);
    let right = scale_vec3(right, half_width);
    let up = scale_vec3(up, half_width);
    let offsets = [
        add_vec3(right, up),
        sub_vec3(up, right),
        scale_vec3(add_vec3(right, up), -1.0),
        sub_vec3(right, up),
    ];
    let tint = color_to_tint(color);
    let vertex_color = [tint[0], tint[1], tint[2]];
    let normal = [forward.x, forward.y, forward.z];
    let mut vertices = Vec::with_capacity(8);
    for base in [a, b] {
        for offset in offsets {
            let mut vertex = engine_render::ColoredVertex::with_normal_uv(
                vec3_to_array(add_vec3(base, offset)),
                vertex_color,
                normal,
                [0.0, 0.0],
            );
            vertex.alpha = tint[3];
            vertices.push(vertex);
        }
    }
    let indices = [
        0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7, 0, 3, 2, 0, 2, 1,
        4, 5, 6, 4, 6, 7,
    ];
    Some(engine_render::Mesh::with_indices(vertices, indices).with_generated_tangents())
}

#[cfg(feature = "backend-wgpu")]
fn color_to_tint(color: Color) -> [f32; 4] {
    [
        color.r.clamp(0.0, 1.0) as f32,
        color.g.clamp(0.0, 1.0) as f32,
        color.b.clamp(0.0, 1.0) as f32,
        color.a.clamp(0.0, 1.0) as f32,
    ]
}

#[cfg(feature = "backend-wgpu")]
fn add_vec3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

#[cfg(feature = "backend-wgpu")]
fn sub_vec3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

#[cfg(feature = "backend-wgpu")]
fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

#[cfg(feature = "backend-wgpu")]
fn vec3_len(value: Vec3) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

#[cfg(feature = "backend-wgpu")]
fn normalize_vec3(value: Vec3) -> Option<Vec3> {
    let len = vec3_len(value);
    (len > f32::EPSILON && len.is_finite()).then(|| scale_vec3(value, 1.0 / len))
}

#[cfg(feature = "backend-wgpu")]
fn cross_vec3(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

#[cfg(feature = "backend-wgpu")]
fn vec3_to_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

#[cfg(feature = "backend-wgpu")]
fn legacy_environment(
    environment: &StoredEnvironment,
    textures: &std::collections::HashMap<u64, engine_render::TextureHandle>,
) -> engine_render::EnvironmentLight {
    let desc = &environment.desc;
    let mut light = engine_render::EnvironmentLight::new(
        color_rgb(desc.diffuse_color),
        desc.diffuse_intensity,
        color_rgb(desc.specular_color),
        desc.specular_intensity,
    )
    .with_background_intensity(desc.background_intensity);
    if let Some(texture) = desc
        .skybox
        .or(desc.texture)
        .and_then(|handle| textures.get(&handle.raw().get()).copied())
    {
        light = light.with_texture(texture);
    }
    light
}

#[cfg(feature = "backend-wgpu")]
fn legacy_camera(camera: &CameraDesc) -> engine_render::Camera {
    let position = matrix_translation(camera.transform);
    let right = normalize_or3(matrix_col3(camera.transform, 0), [1.0, 0.0, 0.0]);
    let up = normalize_or3(matrix_col3(camera.transform, 1), [0.0, 1.0, 0.0]);
    let forward = normalize_or3(neg3(matrix_col3(camera.transform, 2)), [0.0, 0.0, -1.0]);
    match camera.projection {
        Projection::Perspective {
            vertical_fov,
            aspect,
            near,
            far,
            ..
        } => engine_render::ViewCamera::perspective(
            position,
            right,
            up,
            forward,
            vertical_fov,
            aspect,
            near,
            far,
        )
        .into(),
        Projection::Orthographic {
            width,
            height,
            near,
            far,
            ..
        } => engine_render::ViewCamera::orthographic(
            position,
            right,
            up,
            forward,
            width * 0.5,
            height * 0.5,
            near,
            far,
        )
        .into(),
        Projection::Custom { .. } => engine_render::ViewCamera::perspective(
            position,
            right,
            up,
            forward,
            60.0_f32.to_radians(),
            1.0,
            0.1,
            Some(100.0),
        )
        .into(),
    }
}

#[cfg(feature = "backend-wgpu")]
fn camera_aspect(camera: &CameraDesc) -> f32 {
    match camera.projection {
        Projection::Perspective { aspect, .. } => aspect.max(0.0001),
        Projection::Orthographic { width, height, .. } => (width / height.max(0.0001)).max(0.0001),
        Projection::Custom { .. } => 1.0,
    }
}

#[cfg(feature = "backend-wgpu")]
fn legacy_lighting(
    scene: &StoredScene,
    environments: &std::collections::HashMap<u64, engine_render::EnvironmentLight>,
) -> engine_render::RenderLighting {
    let mut directional = engine_render::DirectionalLight::DEFAULT;
    let mut directional_shadow = engine_render::DirectionalShadow::DISABLED;
    let mut points = Vec::new();
    let mut spots = Vec::new();
    for slot in &scene.lights.resources {
        let Some(light) = &slot.value else {
            continue;
        };
        match light {
            LightDesc::Directional(light) => {
                directional = engine_render::DirectionalLight::new(
                    [light.direction.x, light.direction.y, light.direction.z],
                    color_rgb(light.color),
                    (light.illuminance_lux / 100_000.0).max(0.0),
                );
                if let Some(shadow) = &light.shadow {
                    directional_shadow = engine_render::DirectionalShadow::enabled(
                        shadow.resolution,
                        shadow.max_distance,
                        -shadow.max_distance,
                        shadow.max_distance,
                        0.75,
                        shadow.bias.constant,
                    )
                    .with_cascades(
                        shadow.cascades as usize,
                        shadow.max_distance,
                        shadow.split_lambda,
                    );
                }
            }
            LightDesc::Point(light) => points.push(engine_render::PointLight::new(
                [light.position.x, light.position.y, light.position.z],
                color_rgb(light.color),
                light.intensity_lumen,
                light.radius,
            )),
            LightDesc::Spot(light) => spots.push(engine_render::SpotLight::new(
                [light.position.x, light.position.y, light.position.z],
                [light.direction.x, light.direction.y, light.direction.z],
                color_rgb(light.color),
                light.intensity_lumen,
                light.range,
                light.inner_angle,
                light.outer_angle,
            )),
            LightDesc::Area(light) => points.push(engine_render::PointLight::new(
                [light.position.x, light.position.y, light.position.z],
                color_rgb(light.color),
                approximate_area_light_intensity(light),
                light.range,
            )),
            LightDesc::Custom(light) => points.push(engine_render::PointLight::new(
                [light.position.x, light.position.y, light.position.z],
                color_rgb(light.color),
                light.intensity.max(0.0),
                light.range,
            )),
        }
    }
    let environment = match scene.environment {
        Some(handle) => environments
            .get(&handle.raw().get())
            .copied()
            .unwrap_or_else(|| {
                engine_render::EnvironmentLight::new([1.0, 1.0, 1.0], 0.25, [1.0, 1.0, 1.0], 0.0)
            }),
        None => engine_render::EnvironmentLight::new([1.0, 1.0, 1.0], 0.25, [1.0, 1.0, 1.0], 0.0),
    };

    engine_render::RenderLighting::new([1.0, 1.0, 1.0], 0.25, directional)
        .with_environment(environment)
        .with_directional_shadow(directional_shadow)
        .with_point_lights(&points)
        .with_spot_lights(&spots)
}

#[cfg(feature = "backend-wgpu")]
fn approximate_area_light_intensity(light: &AreaLightDesc) -> f32 {
    use std::f32::consts::PI;
    let emitter_area = match light.shape {
        AreaLightShape::Rectangle { width, height } => width * height,
        AreaLightShape::Disk { radius } => PI * radius * radius,
        AreaLightShape::Sphere { radius } => 4.0 * PI * radius * radius,
    };
    (light.intensity * emitter_area).max(0.0)
}

#[cfg(feature = "backend-wgpu")]
fn color_rgb(color: Color) -> [f32; 3] {
    [color.r as f32, color.g as f32, color.b as f32]
}

#[cfg(feature = "backend-wgpu")]
fn matrix_translation(matrix: Mat4) -> [f32; 3] {
    [matrix[3][0], matrix[3][1], matrix[3][2]]
}

#[cfg(feature = "backend-wgpu")]
fn matrix_col3(matrix: Mat4, column: usize) -> [f32; 3] {
    [matrix[column][0], matrix[column][1], matrix[column][2]]
}

#[cfg(feature = "backend-wgpu")]
fn neg3(value: [f32; 3]) -> [f32; 3] {
    [-value[0], -value[1], -value[2]]
}

#[cfg(feature = "backend-wgpu")]
fn normalize_or3(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let len_sq = value[0] * value[0] + value[1] * value[1] + value[2] * value[2];
    if len_sq > f32::EPSILON {
        let inv = 1.0 / len_sq.sqrt();
        [value[0] * inv, value[1] * inv, value[2] * inv]
    } else {
        fallback
    }
}

pub struct SceneEditor<'a> {
    scene: &'a mut StoredScene,
}

pub type SceneWriter<'a> = SceneEditor<'a>;

#[derive(Clone, Debug, PartialEq)]
pub struct SceneCommandBuffer {
    scene: SceneHandle,
    commands: Vec<SceneCommand>,
}

impl SceneCommandBuffer {
    pub fn new(scene: SceneHandle) -> Self {
        Self {
            scene,
            commands: Vec::new(),
        }
    }

    pub fn spawn(&mut self, object: RenderObjectDesc) {
        self.commands.push(SceneCommand::SpawnAuto(object));
    }

    pub fn spawn_reserved(&mut self, object: ObjectHandle, desc: RenderObjectDesc) {
        self.commands.push(SceneCommand::Spawn(desc, object));
    }

    pub fn despawn(&mut self, object: ObjectHandle) {
        self.commands.push(SceneCommand::Despawn(object));
    }

    pub fn set_transform(&mut self, object: ObjectHandle, transform: Mat4) {
        self.commands
            .push(SceneCommand::SetTransform(object, transform));
    }

    pub fn set_previous_transform(&mut self, object: ObjectHandle, transform: Mat4) {
        self.commands
            .push(SceneCommand::SetPreviousTransform { object, transform });
    }

    pub fn clear_previous_transform(&mut self, object: ObjectHandle) {
        self.commands
            .push(SceneCommand::ClearPreviousTransform(object));
    }

    pub fn set_mesh(&mut self, object: ObjectHandle, mesh: MeshHandle) {
        self.commands.push(SceneCommand::SetMesh { object, mesh });
    }

    pub fn set_material(&mut self, object: ObjectHandle, slot: usize, material: MaterialHandle) {
        self.commands
            .push(SceneCommand::SetMaterial(object, slot, material));
    }

    pub fn set_visibility(&mut self, object: ObjectHandle, flags: VisibilityFlags) {
        self.commands
            .push(SceneCommand::SetVisibility(object, flags));
    }

    pub fn set_flags(&mut self, object: ObjectHandle, flags: ObjectFlags) {
        self.commands.push(SceneCommand::SetFlags { object, flags });
    }

    pub fn set_layer(&mut self, object: ObjectHandle, layer: RenderLayer) {
        self.commands.push(SceneCommand::SetLayer { object, layer });
    }

    pub fn set_bounds(&mut self, object: ObjectHandle, bounds: Bounds3) {
        self.commands
            .push(SceneCommand::SetBounds { object, bounds });
    }

    pub fn set_skeleton(&mut self, object: ObjectHandle, skeleton: Option<SkeletonInstanceHandle>) {
        self.commands
            .push(SceneCommand::SetSkeleton { object, skeleton });
    }

    pub fn set_morph_weights(
        &mut self,
        object: ObjectHandle,
        morph_weights: Option<MorphWeightsHandle>,
    ) {
        self.commands.push(SceneCommand::SetMorphWeights {
            object,
            morph_weights,
        });
    }

    pub fn set_lod_group(&mut self, object: ObjectHandle, lod_group: Option<LodGroupHandle>) {
        self.commands
            .push(SceneCommand::SetLodGroup { object, lod_group });
    }

    pub fn add_light(&mut self, light: LightDesc) {
        self.commands.push(SceneCommand::AddLightAuto(light));
    }

    pub fn add_light_reserved(&mut self, light: LightHandle, desc: LightDesc) {
        self.commands.push(SceneCommand::AddLight(desc, light));
    }

    pub fn update_light(&mut self, light: LightHandle, update: LightUpdate) {
        self.commands
            .push(SceneCommand::UpdateLight { light, update });
    }

    pub fn remove_light(&mut self, light: LightHandle) {
        self.commands.push(SceneCommand::RemoveLight(light));
    }

    pub fn set_environment(&mut self, environment: Option<EnvironmentHandle>) {
        self.commands
            .push(SceneCommand::SetEnvironment(environment));
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SceneCommand {
    Spawn(RenderObjectDesc, ObjectHandle),
    SpawnAuto(RenderObjectDesc),
    Despawn(ObjectHandle),
    SetTransform(ObjectHandle, Mat4),
    SetPreviousTransform {
        object: ObjectHandle,
        transform: Mat4,
    },
    ClearPreviousTransform(ObjectHandle),
    SetMesh {
        object: ObjectHandle,
        mesh: MeshHandle,
    },
    SetMaterial(ObjectHandle, usize, MaterialHandle),
    SetVisibility(ObjectHandle, VisibilityFlags),
    SetFlags {
        object: ObjectHandle,
        flags: ObjectFlags,
    },
    SetLayer {
        object: ObjectHandle,
        layer: RenderLayer,
    },
    SetBounds {
        object: ObjectHandle,
        bounds: Bounds3,
    },
    SetSkeleton {
        object: ObjectHandle,
        skeleton: Option<SkeletonInstanceHandle>,
    },
    SetMorphWeights {
        object: ObjectHandle,
        morph_weights: Option<MorphWeightsHandle>,
    },
    SetLodGroup {
        object: ObjectHandle,
        lod_group: Option<LodGroupHandle>,
    },
    AddLight(LightDesc, LightHandle),
    AddLightAuto(LightDesc),
    UpdateLight {
        light: LightHandle,
        update: LightUpdate,
    },
    RemoveLight(LightHandle),
    SetEnvironment(Option<EnvironmentHandle>),
}

pub trait ExtractRenderData {
    fn extract(&self, commands: &mut SceneCommandBuffer);
}

pub struct DebugDraw<'a> {
    renderer: &'a mut Renderer,
}

impl<'a> DebugDraw<'a> {
    pub fn line(&mut self, a: Vec3, b: Vec3, color: Color) {
        self.renderer
            .debug_draw_commands
            .push(DebugDrawCommand::Line { a, b, color });
    }

    pub fn ray(&mut self, origin: Vec3, dir: Vec3, len: f32, color: Color) {
        self.renderer
            .debug_draw_commands
            .push(DebugDrawCommand::Ray {
                origin,
                dir,
                len,
                color,
            });
    }

    pub fn sphere(&mut self, center: Vec3, radius: f32, color: Color) {
        self.renderer
            .debug_draw_commands
            .push(DebugDrawCommand::Sphere {
                center,
                radius,
                color,
            });
    }

    pub fn aabb(&mut self, bounds: Bounds3, color: Color) {
        self.renderer
            .debug_draw_commands
            .push(DebugDrawCommand::Aabb { bounds, color });
    }

    pub fn frustum(&mut self, view_proj: Mat4, color: Color) {
        self.renderer
            .debug_draw_commands
            .push(DebugDrawCommand::Frustum { view_proj, color });
    }

    pub fn text_3d(&mut self, position: Vec3, text: &str, color: Color) {
        self.renderer
            .debug_draw_commands
            .push(DebugDrawCommand::Text3d {
                position,
                text: text.to_owned(),
                color,
            });
    }

    pub fn len(&self) -> usize {
        self.renderer.debug_draw_commands.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DebugDrawCommand {
    Line {
        a: Vec3,
        b: Vec3,
        color: Color,
    },
    Ray {
        origin: Vec3,
        dir: Vec3,
        len: f32,
        color: Color,
    },
    Sphere {
        center: Vec3,
        radius: f32,
        color: Color,
    },
    Aabb {
        bounds: Bounds3,
        color: Color,
    },
    Frustum {
        view_proj: Mat4,
        color: Color,
    },
    Text3d {
        position: Vec3,
        text: String,
        color: Color,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickingTicket {
    raw: NonZeroU64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PickingRequest {
    pub view: ViewHandle,
    pub pixel: UVec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PickingResult {
    pub object: Option<ObjectHandle>,
    pub user_id: u64,
    pub depth: f32,
    pub world_position: Vec3,
    pub source: PickingResultSource,
    pub readback_pixel: Option<UVec2>,
    pub encoded_object_id: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickingResultSource {
    CpuProjection,
    GpuReadback,
}

impl PickingResult {
    fn miss(source: PickingResultSource) -> Self {
        Self {
            object: None,
            user_id: 0,
            depth: 1.0,
            world_position: Vec3::ZERO,
            source,
            readback_pixel: None,
            encoded_object_id: [0, 0, 0, 0],
        }
    }
}

impl<'a> SceneEditor<'a> {
    pub fn spawn(&mut self, object: RenderObjectDesc) -> ObjectHandle {
        self.scene.objects.insert(ResourceKind::Object, object)
    }

    pub fn spawn_reserved(
        &mut self,
        object: ObjectHandle,
        desc: RenderObjectDesc,
    ) -> Result<(), RendererError> {
        validate_render_object_desc(&desc)?;
        self.scene
            .objects
            .fill_reserved(ResourceKind::Object, object, desc)
    }

    pub fn despawn(&mut self, object: ObjectHandle) -> Result<(), RendererError> {
        self.scene.objects.destroy(ResourceKind::Object, object)
    }

    pub fn add_light(&mut self, light: LightDesc) -> Result<LightHandle, RendererError> {
        validate_light_desc(&light)?;
        Ok(self.scene.lights.insert(ResourceKind::Light, light))
    }

    pub fn add_light_reserved(
        &mut self,
        light: LightHandle,
        desc: LightDesc,
    ) -> Result<(), RendererError> {
        validate_light_desc(&desc)?;
        self.scene
            .lights
            .fill_reserved(ResourceKind::Light, light, desc)
    }

    pub fn set_environment(
        &mut self,
        environment: Option<EnvironmentHandle>,
    ) -> Result<(), RendererError> {
        self.scene.environment = environment;
        Ok(())
    }

    pub fn set_transform(
        &mut self,
        object: ObjectHandle,
        transform: Mat4,
    ) -> Result<(), RendererError> {
        validate_mat4_finite(transform, "object transform")?;
        let Some(slot) = self.scene.objects.get_mut(ResourceKind::Object, object) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Object,
                raw: object.raw().get(),
            });
        };
        slot.value
            .as_mut()
            .expect("arena slot is occupied")
            .transform = transform;
        Ok(())
    }

    pub fn set_previous_transform(
        &mut self,
        object: ObjectHandle,
        transform: Mat4,
    ) -> Result<(), RendererError> {
        validate_mat4_finite(transform, "object previous_transform")?;
        let object = self.object_mut(object)?;
        object.previous_transform = Some(transform);
        Ok(())
    }

    pub fn clear_previous_transform(&mut self, object: ObjectHandle) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.previous_transform = None;
        Ok(())
    }

    pub fn set_mesh(
        &mut self,
        object: ObjectHandle,
        mesh: MeshHandle,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.mesh = mesh;
        Ok(())
    }

    pub fn set_material(
        &mut self,
        object: ObjectHandle,
        slot_index: usize,
        material: MaterialHandle,
    ) -> Result<(), RendererError> {
        let Some(slot) = self.scene.objects.get_mut(ResourceKind::Object, object) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Object,
                raw: object.raw().get(),
            });
        };
        let object = slot.value.as_mut().expect("arena slot is occupied");
        if object.materials.len() <= slot_index {
            object.materials.resize(slot_index + 1, material);
        }
        object.materials[slot_index] = material;
        Ok(())
    }

    pub fn set_visibility(
        &mut self,
        object: ObjectHandle,
        flags: VisibilityFlags,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.visibility = flags;
        Ok(())
    }

    pub fn set_flags(
        &mut self,
        object: ObjectHandle,
        flags: ObjectFlags,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.flags = flags;
        Ok(())
    }

    pub fn set_layer(
        &mut self,
        object: ObjectHandle,
        layer: RenderLayer,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.layer = layer;
        Ok(())
    }

    pub fn set_bounds(
        &mut self,
        object: ObjectHandle,
        bounds: Bounds3,
    ) -> Result<(), RendererError> {
        validate_bounds(bounds)?;
        let object = self.object_mut(object)?;
        object.bounds = Some(bounds);
        Ok(())
    }

    pub fn set_skeleton(
        &mut self,
        object: ObjectHandle,
        skeleton: Option<SkeletonInstanceHandle>,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.skeleton = skeleton;
        Ok(())
    }

    pub fn set_morph_weights(
        &mut self,
        object: ObjectHandle,
        morph_weights: Option<MorphWeightsHandle>,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.morph_weights = morph_weights;
        Ok(())
    }

    pub fn set_lod_group(
        &mut self,
        object: ObjectHandle,
        lod_group: Option<LodGroupHandle>,
    ) -> Result<(), RendererError> {
        let object = self.object_mut(object)?;
        object.lod_group = lod_group;
        Ok(())
    }

    pub fn update_light(
        &mut self,
        light: LightHandle,
        update: LightUpdate,
    ) -> Result<(), RendererError> {
        validate_light_desc(&update.desc)?;
        let Some(slot) = self.scene.lights.get_mut(ResourceKind::Light, light) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Light,
                raw: light.raw().get(),
            });
        };
        *slot.value.as_mut().expect("arena slot is occupied") = update.desc;
        Ok(())
    }

    pub fn remove_light(&mut self, light: LightHandle) -> Result<(), RendererError> {
        self.scene.lights.destroy(ResourceKind::Light, light)
    }

    fn object_mut(&mut self, object: ObjectHandle) -> Result<&mut RenderObjectDesc, RendererError> {
        let Some(slot) = self.scene.objects.get_mut(ResourceKind::Object, object) else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Object,
                raw: object.raw().get(),
            });
        };
        Ok(slot.value.as_mut().expect("arena slot is occupied"))
    }
}

pub struct Frame<'a> {
    renderer: &'a mut Renderer,
    frame_index: u64,
    started_at: Instant,
    wait_for_gpu: bool,
    views: Vec<ViewDesc>,
    graph_extensions: Vec<RenderGraphExtensionHandle>,
}

impl<'a> Frame<'a> {
    pub fn add_graph_extension(
        &mut self,
        extension: impl RenderGraphExtension,
    ) -> Result<(), RendererError> {
        let extension = self.renderer.register_graph_extension(extension)?;
        self.graph_extensions.push(extension);
        Ok(())
    }

    pub fn add_post_process(&mut self, desc: CustomPostProcessDesc) -> Result<(), RendererError> {
        let extension = self.renderer.register_post_process(desc)?;
        self.graph_extensions.push(extension);
        Ok(())
    }

    pub fn debug_draw(&mut self) -> DebugDraw<'_> {
        self.renderer.debug_draw()
    }

    fn release_frame_graph_extensions(&mut self) {
        for extension in self.graph_extensions.drain(..) {
            let _ = self.renderer.destroy(extension);
        }
    }

    pub fn render_view(&mut self, view: ViewDesc) -> Result<ViewHandle, RendererError> {
        if self
            .renderer
            .scenes
            .get(ResourceKind::Scene, view.scene)
            .is_none()
        {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Scene,
                raw: view.scene.raw().get(),
            });
        }
        validate_camera_desc(&view.camera)?;
        self.renderer.validate_view_target(&view.target)?;
        self.renderer.validate_view_quality(&view)?;
        if self.renderer.config.validation != ValidationMode::Off {
            self.renderer
                .validate_scene_render_resources(&view, self.renderer.config.validation)?;
        }
        let handle: ViewHandle = self
            .renderer
            .views
            .insert(ResourceKind::View, StoredView { desc: view.clone() });
        self.views.push(view);
        Ok(handle)
    }

    pub fn finish(mut self) -> Result<FrameStats, RendererError> {
        let submit_started_at = Instant::now();
        let cpu_build_time_ms = elapsed_ms(self.started_at, submit_started_at);
        #[cfg(feature = "backend-wgpu")]
        if self.renderer.wgpu_runtime.is_some() {
            if let Some(index) = self
                .views
                .iter()
                .position(|view| matches!(view.target, RenderTarget::MainSurface))
            {
                let view = self.views.remove(index);
                let debug_draws = self.renderer.debug_draw_commands.len() as u32;
                let mut stats = self.renderer.render_facade_view(&view)?;
                let pipeline_keys = self.renderer.view_pipeline_keys(&view)?;
                self.renderer.record_pipeline_keys(&pipeline_keys);
                stats.graph = build_view_graph_stats(self.renderer, &view, &self.graph_extensions)?;
                stats.frame_index = self.frame_index;
                stats.draw_calls += debug_draws;
                stats.dispatch_calls = stats.graph.compute_dispatches;
                let culling = view_culling_stats(self.renderer, &view);
                if let Some(output) = culling_output(&view, culling) {
                    stats.culling_outputs.push(output);
                }
                if let Some(output) = ssao_output(self.renderer, &view)? {
                    stats.ssao_outputs.push(output);
                }
                if let Some(output) = light_cluster_output(self.renderer, &view)? {
                    stats.light_cluster_outputs.push(output);
                }
                if let Some(output) = area_light_output(self.renderer, &view)? {
                    stats.area_light_outputs.push(output);
                }
                if let Some(output) = ray_tracing_output(self.renderer, &view)? {
                    stats.ray_tracing_outputs.push(output);
                }
                stats
                    .shadow_outputs
                    .extend(shadow_outputs(self.renderer, &view)?);
                if let Some(output) = gbuffer_output(self.renderer, &view)? {
                    stats.gbuffer_outputs.push(output);
                }
                stats
                    .lod_outputs
                    .extend(view_lod_outputs(self.renderer, &view)?);
                if let Some(output) = streaming_output(self.renderer, &view)? {
                    stats.streaming_outputs.push(output);
                }
                if let Some(output) = debug_draw_output(self.renderer, &view) {
                    stats.debug_draw_outputs.push(output);
                }
                stats
                    .picking_outputs
                    .push(picking_output(self.renderer, &view)?);
                if let Some(output) = environment_output(self.renderer, &view)? {
                    stats.environment_outputs.push(output);
                }
                let deformation = view_deformation_stats(self.renderer, &view)?;
                stats.skinned_objects = deformation.skinned_objects;
                stats.morphed_objects = deformation.morphed_objects;
                stats.deformed_objects = deformation.deformed_objects();
                if let Some(output) = deformation_output(&view, deformation) {
                    stats.deformation_outputs.push(output);
                }
                let motion = view_motion_vector_stats(self.renderer, &view)?;
                stats.motion_vector_objects = motion.objects;
                stats.motion_vector_views = motion.views;
                if let Some(output) = motion_vector_output(self.renderer, &view, motion)? {
                    stats.motion_vector_outputs.push(output);
                }
                stats.post_process_outputs.extend(post_process_outputs(
                    self.renderer,
                    &view,
                    &self.graph_extensions,
                )?);
                if self.wait_for_gpu {
                    if let Some(runtime) = self.renderer.wgpu_runtime.as_ref() {
                        runtime.wait_for_gpu();
                    }
                    self.renderer.flush_uploads()?;
                }
                stats.upload = self.renderer.upload_stats.clone();
                stats.memory = self.renderer.memory_stats();
                stats.cpu_build_time_ms = cpu_build_time_ms;
                stats.cpu_submit_time_ms = elapsed_ms(submit_started_at, Instant::now());
                self.renderer.apply_frame_instrumentation(&mut stats);
                self.renderer.last_frame_stats = Some(stats.clone());
                self.renderer.frame_index = self.frame_index + 1;
                self.renderer.debug_draw_commands.clear();
                self.release_frame_graph_extensions();
                return Ok(stats);
            }
        }

        let mut graph_stats = RenderGraphStats::default();
        let mut visible_objects = 0;
        let mut culled_objects = 0;
        let mut culling_outputs = Vec::new();
        let mut ssao_outputs = Vec::new();
        let mut light_cluster_outputs = Vec::new();
        let mut area_light_outputs = Vec::new();
        let mut ray_tracing_outputs = Vec::new();
        let mut shadow_frame_outputs = Vec::new();
        let mut gbuffer_outputs = Vec::new();
        let mut lod_outputs = Vec::new();
        let mut streaming_outputs = Vec::new();
        let mut debug_draw_outputs = Vec::new();
        let mut picking_outputs = Vec::new();
        let mut environment_outputs = Vec::new();
        let mut skinned_objects = 0;
        let mut morphed_objects = 0;
        let mut deformed_objects = 0;
        let mut deformation_outputs = Vec::new();
        let mut motion_vector_objects = 0;
        let mut motion_vector_views = 0;
        let mut motion_vector_outputs = Vec::new();
        let mut post_process_frame_outputs = Vec::new();
        let mut triangles = 0;
        let mut batch_keys = Vec::new();
        let mut pipeline_keys = Vec::new();
        let debug_draws = self.renderer.debug_draw_commands.len() as u32;
        for view in self.views.drain(..) {
            let scene = self
                .renderer
                .scenes
                .get(ResourceKind::Scene, view.scene)
                .and_then(|slot| slot.value.as_ref())
                .expect("view scene was validated");
            let motion = view_motion_vector_stats(self.renderer, &view)?;
            motion_vector_objects += motion.objects;
            motion_vector_views += motion.views;
            if let Some(output) = motion_vector_output(self.renderer, &view, motion)? {
                motion_vector_outputs.push(output);
            }
            post_process_frame_outputs.extend(post_process_outputs(
                self.renderer,
                &view,
                &self.graph_extensions,
            )?);
            let deformation = view_deformation_stats(self.renderer, &view)?;
            if let Some(output) = deformation_output(&view, deformation) {
                deformation_outputs.push(output);
            }
            let culling = view_culling_stats(self.renderer, &view);
            if let Some(output) = culling_output(&view, culling) {
                culling_outputs.push(output);
            }
            if let Some(output) = ssao_output(self.renderer, &view)? {
                ssao_outputs.push(output);
            }
            if let Some(output) = light_cluster_output(self.renderer, &view)? {
                light_cluster_outputs.push(output);
            }
            if let Some(output) = area_light_output(self.renderer, &view)? {
                area_light_outputs.push(output);
            }
            if let Some(output) = ray_tracing_output(self.renderer, &view)? {
                ray_tracing_outputs.push(output);
            }
            shadow_frame_outputs.extend(shadow_outputs(self.renderer, &view)?);
            if let Some(output) = gbuffer_output(self.renderer, &view)? {
                gbuffer_outputs.push(output);
            }
            lod_outputs.extend(view_lod_outputs(self.renderer, &view)?);
            if let Some(output) = streaming_output(self.renderer, &view)? {
                streaming_outputs.push(output);
            }
            if let Some(output) = debug_draw_output(self.renderer, &view) {
                debug_draw_outputs.push(output);
            }
            picking_outputs.push(picking_output(self.renderer, &view)?);
            if let Some(output) = environment_output(self.renderer, &view)? {
                environment_outputs.push(output);
            }
            let view_draw_items = self.renderer.view_draw_items(&view)?;
            pipeline_keys.extend(view_draw_items.iter().map(|item| item.pipeline_key));
            for (object_index, object) in scene
                .objects
                .resources
                .iter()
                .enumerate()
                .filter_map(|(index, slot)| slot.value.as_ref().map(|object| (index, object)))
            {
                match view_object_visibility(scene, object, &view) {
                    ViewObjectVisibility::Visible => {
                        visible_objects += 1;
                        let has_geometry =
                            object_material_has_geometry_phase(self.renderer, object, &view)?;
                        if has_geometry && object.skeleton.is_some() {
                            skinned_objects += 1;
                        }
                        if has_geometry && object.morph_weights.is_some() {
                            morphed_objects += 1;
                        }
                        if has_geometry
                            && (object.skeleton.is_some() || object.morph_weights.is_some())
                        {
                            deformed_objects += 1;
                        }
                        let object_handle = make_handle(
                            ResourceKind::Object,
                            object_index as u32,
                            scene.objects.resources[object_index].generation,
                        );
                        batch_keys.extend(self.renderer.object_batch_keys(
                            object_handle,
                            object,
                            &view,
                        )?);
                        triangles += self.renderer.object_triangle_count(object, &view)?;
                    }
                    ViewObjectVisibility::Culled => {
                        culled_objects += 1;
                    }
                    ViewObjectVisibility::Hidden => {}
                }
            }
            let view_graph = build_view_graph_stats(self.renderer, &view, &self.graph_extensions)?;
            accumulate_graph_stats(&mut graph_stats, &view_graph);
        }
        let draw_calls = count_unique_batch_keys(&batch_keys) + debug_draws;
        let pipeline_switches = count_unique_batch_keys(&batch_keys);
        let material_switches = count_unique_materials(&batch_keys);
        let dispatch_calls = graph_stats.compute_dispatches;
        self.renderer.record_pipeline_keys(&pipeline_keys);
        if self.wait_for_gpu {
            self.renderer.flush_uploads()?;
        }
        let mut stats = FrameStats {
            frame_index: self.frame_index,
            draw_calls,
            triangles,
            visible_objects,
            culled_objects,
            culling_outputs,
            ssao_outputs,
            light_cluster_outputs,
            area_light_outputs,
            ray_tracing_outputs,
            shadow_outputs: shadow_frame_outputs,
            gbuffer_outputs,
            lod_outputs,
            streaming_outputs,
            debug_draw_outputs,
            picking_outputs,
            environment_outputs,
            skinned_objects,
            morphed_objects,
            deformed_objects,
            deformation_outputs,
            motion_vector_objects,
            motion_vector_views,
            motion_vector_outputs,
            post_process_outputs: post_process_frame_outputs,
            pipeline_switches,
            material_switches,
            graph: graph_stats,
            dispatch_calls,
            upload: self.renderer.upload_stats.clone(),
            memory: self.renderer.memory_stats(),
            ..FrameStats::default()
        };
        stats.cpu_build_time_ms = cpu_build_time_ms;
        stats.cpu_submit_time_ms = elapsed_ms(submit_started_at, Instant::now());
        self.renderer.apply_frame_instrumentation(&mut stats);
        self.renderer.last_frame_stats = Some(stats.clone());
        self.renderer.frame_index = self.frame_index + 1;
        self.renderer.debug_draw_commands.clear();
        self.release_frame_graph_extensions();
        Ok(stats)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewObjectVisibility {
    Visible,
    Hidden,
    Culled,
}

fn view_object_visibility(
    scene: &StoredScene,
    object: &RenderObjectDesc,
    view: &ViewDesc,
) -> ViewObjectVisibility {
    if (object.visibility.0 & VisibilityFlags::CAMERA.0) == 0 || !view.layers.contains(object.layer)
    {
        return ViewObjectVisibility::Hidden;
    }
    if scene.desc.enable_gpu_culling
        && object_participates_in_gpu_culling(object)
        && object_bounds_outside_view(object, view)
    {
        return ViewObjectVisibility::Culled;
    }
    ViewObjectVisibility::Visible
}

fn object_participates_in_gpu_culling(object: &RenderObjectDesc) -> bool {
    object.flags.contains(ObjectFlags::GPU_CULLABLE)
}

fn view_object_pickable(
    renderer: &Renderer,
    scene: &StoredScene,
    object: &RenderObjectDesc,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    if !object.visibility.contains(VisibilityFlags::PICKING)
        || view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible
    {
        return Ok(false);
    }
    object_material_supports_phase(renderer, object, view, RenderPhaseKind::Picking)
}

fn object_material_supports_phase(
    renderer: &Renderer,
    object: &RenderObjectDesc,
    view: &ViewDesc,
    phase: RenderPhaseKind,
) -> Result<bool, RendererError> {
    let (_, materials) = renderer.selected_object_resources(object, view)?;
    if materials.is_empty() {
        return Ok(true);
    }
    for material in materials {
        if renderer.material_supports_phase(*material, phase)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn object_material_has_geometry_phase(
    renderer: &Renderer,
    object: &RenderObjectDesc,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    for phase in [
        RenderPhaseKind::DepthPrepass,
        RenderPhaseKind::GBuffer,
        RenderPhaseKind::ForwardOpaque,
        RenderPhaseKind::ForwardTransparent,
    ] {
        if object_material_supports_phase(renderer, object, view, phase)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn encode_gpu_picking_object_index(object: ObjectHandle) -> [u8; 4] {
    let index = object.index();
    [
        (index & 0xff) as u8,
        ((index >> 8) & 0xff) as u8,
        ((index >> 16) & 0xff) as u8,
        gpu_picking_generation_byte(object),
    ]
}

fn gpu_picking_generation_byte(object: ObjectHandle) -> u8 {
    let generation = (object.generation() & 0xff) as u8;
    if generation == 0 {
        0xff
    } else {
        generation
    }
}

fn validate_gpu_picking_payload(depth: f32, world_position: Vec3) -> Result<(), RendererError> {
    if !depth.is_finite() || !(0.0..=1.0).contains(&depth) {
        return Err(RendererError::Validation(
            "GPU picking depth must be finite and in 0..=1".to_owned(),
        ));
    }
    validate_vec3_finite(world_position, "GPU picking world_position")
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CullingStats {
    gpu_culling: bool,
    occlusion_culling: bool,
    visible_objects: u32,
    culled_objects: u32,
}

impl CullingStats {
    const fn enabled(self) -> bool {
        self.gpu_culling || self.occlusion_culling
    }

    const fn tested_objects(self) -> u32 {
        self.visible_objects + self.culled_objects
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CullingBuffers {
    visibility: GraphBuffer,
    indirect_args: GraphBuffer,
    occlusion_results: Option<GraphBuffer>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DeformationStats {
    skinned_objects: u32,
    morphed_objects: u32,
    deformed_objects: u32,
    output_buffer_bytes: u64,
}

impl DeformationStats {
    const fn deformed_objects(self) -> u32 {
        self.deformed_objects
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MotionVectorStats {
    objects: u32,
    views: u32,
    camera_motion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GBufferTargets {
    albedo: GraphTexture,
    normal: GraphTexture,
    material: GraphTexture,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EnvironmentGraphTextures {
    skybox: Option<GraphTexture>,
    irradiance: Option<GraphTexture>,
    prefiltered_specular: Option<GraphTexture>,
    brdf_lut: Option<GraphTexture>,
}

fn view_culling_stats(renderer: &Renderer, view: &ViewDesc) -> CullingStats {
    let Some(scene) = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
    else {
        return CullingStats::default();
    };
    let mut stats = CullingStats {
        gpu_culling: scene.desc.enable_gpu_culling,
        occlusion_culling: scene.desc.enable_occlusion_culling,
        visible_objects: 0,
        culled_objects: 0,
    };
    if !stats.enabled() {
        return stats;
    }
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
        .filter(|object| object_participates_in_gpu_culling(object))
    {
        match view_object_visibility(scene, object, view) {
            ViewObjectVisibility::Visible => stats.visible_objects += 1,
            ViewObjectVisibility::Culled => stats.culled_objects += 1,
            ViewObjectVisibility::Hidden => {}
        }
    }
    stats
}

fn light_cluster_count(width: u32, height: u32) -> u32 {
    width
        .div_ceil(LIGHT_CLUSTER_TILE_SIZE)
        .max(1)
        .saturating_mul(height.div_ceil(LIGHT_CLUSTER_TILE_SIZE).max(1))
        .saturating_mul(LIGHT_CLUSTER_Z_SLICES)
}

fn light_cluster_buffer_bytes(width: u32, height: u32) -> u64 {
    u64::from(light_cluster_count(width, height))
        .saturating_mul(LIGHT_CLUSTER_RECORD_BYTES)
        .max(LIGHT_CLUSTER_RECORD_BYTES)
}

fn view_clustered_light_count(scene: &StoredScene, view: &ViewDesc) -> u32 {
    scene
        .lights
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
        .filter(|light| match light {
            LightDesc::Point(light) => (light.layer_mask.0 & view.layers.0) != 0,
            LightDesc::Spot(light) => (light.layer_mask.0 & view.layers.0) != 0,
            LightDesc::Custom(light) => (light.layer_mask.0 & view.layers.0) != 0,
            LightDesc::Directional(_) => false,
            LightDesc::Area(light) => (light.layer_mask.0 & view.layers.0) != 0,
        })
        .count() as u32
}

fn view_area_light_count(scene: &StoredScene, view: &ViewDesc) -> u32 {
    scene
        .lights
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
        .filter(|light| match light {
            LightDesc::Area(light) => (light.layer_mask.0 & view.layers.0) != 0,
            _ => false,
        })
        .count() as u32
}

fn area_light_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameAreaLightOutput>, RendererError> {
    let Some(scene) = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
    else {
        return Err(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        });
    };
    let area_lights = view_area_light_count(scene, view);
    Ok((area_lights > 0).then_some(FrameAreaLightOutput {
        view_label: view.label.clone(),
        area_lights,
    }))
}

fn directional_shadow_atlas_extent(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<Option<(u32, u32, u32)>, RendererError> {
    if !view_has_shadow_caster(renderer, scene, view)? {
        return Ok(None);
    }
    let mut shadowed_lights = 0_u32;
    let mut width = 0_u32;
    let mut height = 0_u32;
    for light in scene
        .lights
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        let LightDesc::Directional(light) = light else {
            continue;
        };
        if (light.layer_mask.0 & view.layers.0) == 0 {
            continue;
        }
        let Some(shadow) = &light.shadow else {
            continue;
        };
        shadowed_lights = shadowed_lights.saturating_add(1);
        width = width.max(shadow.resolution);
        height = height.saturating_add(
            shadow
                .resolution
                .saturating_mul(u32::from(shadow.cascades).max(1)),
        );
    }
    Ok((shadowed_lights > 0).then_some((width.max(1), height.max(1), shadowed_lights)))
}

fn point_spot_shadow_atlas_extent(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<Option<(u32, u32, u32)>, RendererError> {
    if !view_has_shadow_caster(renderer, scene, view)? {
        return Ok(None);
    }
    let mut shadowed_lights = 0_u32;
    let mut width = 0_u32;
    let mut height = 0_u32;
    for light in scene
        .lights
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        match light {
            LightDesc::Point(light) if (light.layer_mask.0 & view.layers.0) != 0 => {
                if let Some(shadow) = &light.shadow {
                    shadowed_lights = shadowed_lights.saturating_add(1);
                    width = width.max(shadow.resolution);
                    height = height.saturating_add(shadow.resolution.saturating_mul(6));
                }
            }
            LightDesc::Spot(light) if (light.layer_mask.0 & view.layers.0) != 0 => {
                if let Some(shadow) = &light.shadow {
                    shadowed_lights = shadowed_lights.saturating_add(1);
                    width = width.max(shadow.resolution);
                    height = height.saturating_add(shadow.resolution);
                }
            }
            _ => {}
        }
    }
    Ok((shadowed_lights > 0).then_some((width.max(1), height.max(1), shadowed_lights)))
}

fn view_has_shadow_caster(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if !view.layers.contains(object.layer)
            || !object.visibility.contains(VisibilityFlags::SHADOW)
            || !object.flags.contains(ObjectFlags::CAST_SHADOW)
        {
            continue;
        }
        let (_, materials) = renderer.selected_object_resources(object, view)?;
        for material in materials {
            if renderer.material_supports_phase(*material, RenderPhaseKind::Shadow)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn shadow_outputs(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Vec<FrameShadowOutput>, RendererError> {
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let mut outputs = Vec::new();
    if let Some((width, height, shadowed_lights)) =
        directional_shadow_atlas_extent(renderer, scene, view)?
    {
        outputs.push(FrameShadowOutput {
            view_label: view.label.clone(),
            pass_label: "shadow_csm".to_owned(),
            width,
            height,
            format: TextureFormat::Depth32Float,
            shadowed_lights,
            atlas_texture_label: "shadow_csm_atlas".to_owned(),
        });
    }
    if let Some((width, height, shadowed_lights)) =
        point_spot_shadow_atlas_extent(renderer, scene, view)?
    {
        outputs.push(FrameShadowOutput {
            view_label: view.label.clone(),
            pass_label: "shadow_point_spot".to_owned(),
            width,
            height,
            format: TextureFormat::Depth32Float,
            shadowed_lights,
            atlas_texture_label: "shadow_point_spot_atlas".to_owned(),
        });
    }
    Ok(outputs)
}

fn gbuffer_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameGBufferOutput>, RendererError> {
    if !matches!(
        effective_render_path(&renderer.config, view),
        RenderPath::Deferred | RenderPath::Auto
    ) {
        return Ok(None);
    }
    let (width, height) = view_graph_extent(renderer, view)?;
    Ok(Some(FrameGBufferOutput {
        view_label: view.label.clone(),
        width,
        height,
        albedo_format: TextureFormat::Rgba8Unorm,
        normal_format: TextureFormat::Rgba16Float,
        material_format: TextureFormat::Rgba8Unorm,
        albedo_texture_label: "gbuffer_albedo".to_owned(),
        normal_texture_label: "gbuffer_normal".to_owned(),
        material_texture_label: "gbuffer_material".to_owned(),
    }))
}

fn view_lod_outputs(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Vec<FrameLodOutput>, RendererError> {
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let camera_position = Vec3::new(
        view.camera.transform[3][0],
        view.camera.transform[3][1],
        view.camera.transform[3][2],
    );
    let mut outputs = Vec::new();
    for (object_index, slot) in scene.objects.resources.iter().enumerate() {
        let Some(object) = slot.value.as_ref() else {
            continue;
        };
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        let Some(lod_group) = object.lod_group else {
            continue;
        };
        let Some(group) = renderer
            .lod_groups
            .get(ResourceKind::LodGroup, lod_group)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::LodGroup,
                raw: lod_group.raw().get(),
            });
        };
        let object_position = Vec3::new(
            object.transform[3][0],
            object.transform[3][1],
            object.transform[3][2],
        );
        let distance = vec3_distance(camera_position, object_position);
        let (level_index, level) = group
            .levels
            .iter()
            .enumerate()
            .find(|(_, level)| distance <= level.max_distance)
            .or_else(|| group.levels.iter().enumerate().last())
            .expect("LOD groups are validated as non-empty");
        outputs.push(FrameLodOutput {
            view_label: view.label.clone(),
            object: make_handle(ResourceKind::Object, object_index as u32, slot.generation),
            lod_group,
            level_index: level_index as u32,
            selected_mesh: level.mesh,
            distance,
        });
    }
    Ok(outputs)
}

fn streaming_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameStreamingOutput>, RendererError> {
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let mut texture_ids = HashSet::new();
    let mut mesh_ids = HashSet::new();
    let mut streamable_texture_mips = 0_u32;
    let mut streamable_mesh_bytes = 0_u64;
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (mesh, materials) = renderer.selected_object_resources(object, view)?;
        let mesh_slot =
            renderer
                .meshes
                .get(ResourceKind::Mesh, mesh)
                .ok_or(RendererError::InvalidHandle {
                    kind: ResourceKind::Mesh,
                    raw: mesh.raw().get(),
                })?;
        if mesh_slot.priority == ResidencyPriority::Streamable && mesh_ids.insert(mesh.raw().get())
        {
            let Some(mesh) = mesh_slot.value.as_ref() else {
                return Err(RendererError::InvalidHandle {
                    kind: ResourceKind::Mesh,
                    raw: mesh.raw().get(),
                });
            };
            streamable_mesh_bytes =
                streamable_mesh_bytes.saturating_add(mesh_resident_bytes(mesh) as u64);
        }
        for material in materials {
            for texture in material_texture_handles(renderer, *material)? {
                let texture_slot = renderer
                    .textures
                    .get(ResourceKind::Texture, texture)
                    .ok_or(RendererError::InvalidHandle {
                        kind: ResourceKind::Texture,
                        raw: texture.raw().get(),
                    })?;
                if texture_slot.priority == ResidencyPriority::Streamable
                    && texture_ids.insert(texture.raw().get())
                {
                    let Some(stored) = texture_slot.value.as_ref() else {
                        return Err(RendererError::InvalidHandle {
                            kind: ResourceKind::Texture,
                            raw: texture.raw().get(),
                        });
                    };
                    streamable_texture_mips =
                        streamable_texture_mips.saturating_add(stored.desc.mip_levels);
                }
            }
        }
    }
    if texture_ids.is_empty() && mesh_ids.is_empty() {
        return Ok(None);
    }
    Ok(Some(FrameStreamingOutput {
        view_label: view.label.clone(),
        streamable_textures: texture_ids.len().try_into().unwrap_or(u32::MAX),
        streamable_texture_mips,
        streamable_meshes: mesh_ids.len().try_into().unwrap_or(u32::MAX),
        streamable_mesh_bytes,
    }))
}

fn debug_draw_enabled(renderer: &Renderer, view: &ViewDesc) -> bool {
    view.camera.flags.contains(CameraFlags::ENABLE_DEBUG_DRAW)
        || !renderer.debug_draw_commands.is_empty()
}

fn debug_draw_output(renderer: &Renderer, view: &ViewDesc) -> Option<FrameDebugDrawOutput> {
    debug_draw_enabled(renderer, view).then(|| FrameDebugDrawOutput {
        view_label: view.label.clone(),
        command_count: renderer.debug_draw_commands.len() as u32,
        target_texture_label: "main_color".to_owned(),
    })
}

fn picking_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<FramePickingOutput, RendererError> {
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let (width, height) = view_graph_extent(renderer, view)?;
    let mut pickable_objects = 0_u32;
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_pickable(renderer, scene, object, view)? {
            pickable_objects = pickable_objects.saturating_add(1);
        }
    }
    Ok(FramePickingOutput {
        view_label: view.label.clone(),
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
        pickable_objects,
        target_texture_label: "picking_id".to_owned(),
        ready_results: count_ready(&renderer.picking),
    })
}

fn environment_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameEnvironmentOutput>, RendererError> {
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let Some(environment) = scene.environment else {
        return Ok(None);
    };
    let desc = renderer
        .environment_desc(environment)
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Environment,
            raw: environment.raw().get(),
        })?;
    Ok(Some(FrameEnvironmentOutput {
        view_label: view.label.clone(),
        environment_label: desc.label.clone(),
        skybox_texture_label: optional_texture_label(renderer, desc.skybox)?,
        irradiance_texture_label: optional_texture_label(renderer, desc.irradiance)?,
        prefiltered_specular_texture_label: optional_texture_label(
            renderer,
            desc.prefiltered_specular,
        )?,
        brdf_lut_texture_label: optional_texture_label(renderer, desc.brdf_lut)?,
    }))
}

fn optional_texture_label(
    renderer: &Renderer,
    texture: Option<TextureHandle>,
) -> Result<Option<String>, RendererError> {
    texture
        .map(|texture| {
            renderer.texture_info(texture).map(|info| info.label).ok_or(
                RendererError::InvalidHandle {
                    kind: ResourceKind::Texture,
                    raw: texture.raw().get(),
                },
            )
        })
        .transpose()
        .map(Option::flatten)
}

fn culling_output(view: &ViewDesc, culling: CullingStats) -> Option<FrameCullingOutput> {
    if !culling.enabled() {
        return None;
    }
    let tested_objects = culling.tested_objects();
    Some(FrameCullingOutput {
        view_label: view.label.clone(),
        gpu_culling: culling.gpu_culling,
        occlusion_culling: culling.occlusion_culling,
        tested_objects,
        visible_objects: culling.visible_objects,
        culled_objects: culling.culled_objects,
        visibility_buffer_label: "gpu_visibility".to_owned(),
        visibility_buffer_bytes: u64::from(tested_objects).saturating_mul(4).max(4),
        indirect_args_buffer_label: "gpu_indirect_args".to_owned(),
        indirect_args_buffer_bytes: u64::from(culling.visible_objects)
            .saturating_mul(16)
            .max(16),
        occlusion_result_buffer_label: culling
            .occlusion_culling
            .then(|| "gpu_occlusion_results".to_owned()),
        occlusion_result_buffer_bytes: culling
            .occlusion_culling
            .then(|| u64::from(tested_objects).saturating_mul(8).max(8))
            .unwrap_or(0),
    })
}

fn ssao_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameSsaoOutput>, RendererError> {
    if !effective_view_quality(view).ssao {
        return Ok(None);
    }
    let (width, height) = view_graph_extent(renderer, view)?;
    Ok(Some(FrameSsaoOutput {
        view_label: view.label.clone(),
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
        output_texture_label: "ssao_occlusion".to_owned(),
    }))
}

const LIGHT_CLUSTER_TILE_SIZE: u32 = 16;
const LIGHT_CLUSTER_Z_SLICES: u32 = 24;
const LIGHT_CLUSTER_RECORD_BYTES: u64 = 16;

fn light_cluster_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameLightClusterOutput>, RendererError> {
    if !matches!(
        effective_render_path(&renderer.config, view),
        RenderPath::ForwardPlus
    ) {
        return Ok(None);
    }
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let (width, height) = view_graph_extent(renderer, view)?;
    let cluster_count = light_cluster_count(width, height);
    Ok(Some(FrameLightClusterOutput {
        view_label: view.label.clone(),
        tile_size_px: LIGHT_CLUSTER_TILE_SIZE,
        z_slices: LIGHT_CLUSTER_Z_SLICES,
        cluster_count,
        clustered_lights: view_clustered_light_count(scene, view),
        cluster_buffer_label: "light_cluster_grid".to_owned(),
        cluster_buffer_bytes: light_cluster_buffer_bytes(width, height),
    }))
}

fn ray_tracing_output(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<Option<FrameRayTracingOutput>, RendererError> {
    if !effective_view_quality(view).ray_tracing {
        return Ok(None);
    }
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    if !view_has_visible_geometry(renderer, scene, view)? {
        return Ok(None);
    }
    let visible_geometries = view_visible_geometry_count(renderer, scene, view)?;
    if visible_geometries == 0 {
        return Ok(None);
    }
    Ok(Some(FrameRayTracingOutput {
        view_label: view.label.clone(),
        visible_geometries,
        accel_buffer_label: "ray_tracing_accel".to_owned(),
        accel_buffer_bytes: u64::from(visible_geometries).saturating_mul(64).max(64),
    }))
}

fn view_deformation_stats(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<DeformationStats, RendererError> {
    let Some(scene) = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
    else {
        return Ok(DeformationStats::default());
    };
    let mut stats = DeformationStats::default();
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let skinned = object.skeleton.is_some();
        let morphed = object.morph_weights.is_some();
        if skinned || morphed {
            stats.deformed_objects += 1;
        }
        if skinned {
            stats.skinned_objects += 1;
        }
        if morphed {
            stats.morphed_objects += 1;
        }
        if skinned || morphed {
            let (mesh, _) = renderer.selected_object_resources(object, view)?;
            if let Some(mesh) = renderer
                .meshes
                .get(ResourceKind::Mesh, mesh)
                .and_then(|slot| slot.value.as_ref())
            {
                stats.output_buffer_bytes = stats
                    .output_buffer_bytes
                    .saturating_add(mesh.vertex_bytes.len() as u64);
            }
        }
    }
    Ok(stats)
}

fn deformation_output(
    view: &ViewDesc,
    deformation: DeformationStats,
) -> Option<FrameDeformationOutput> {
    if deformation.deformed_objects() == 0 {
        return None;
    }
    Some(FrameDeformationOutput {
        view_label: view.label.clone(),
        skinned_objects: deformation.skinned_objects,
        morphed_objects: deformation.morphed_objects,
        deformed_objects: deformation.deformed_objects,
        output_buffer_label: "gpu_deformed_vertices".to_owned(),
        output_buffer_bytes: deformation.output_buffer_bytes.max(1),
    })
}

fn view_motion_vector_stats(
    renderer: &Renderer,
    view: &ViewDesc,
) -> Result<MotionVectorStats, RendererError> {
    let Some(scene) = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
    else {
        return Ok(MotionVectorStats::default());
    };
    let camera_motion = view.camera.previous_view_proj.is_some()
        || view.quality.taa
        || view.camera.flags.contains(CameraFlags::ENABLE_TAA);
    let mut objects = 0;
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if object_has_transform_motion(object)
            && object_material_supports_phase(
                renderer,
                object,
                view,
                RenderPhaseKind::MotionVector,
            )?
        {
            objects += 1;
        }
    }
    Ok(MotionVectorStats {
        objects,
        views: if objects > 0 || camera_motion || view.quality.motion_blur {
            1
        } else {
            0
        },
        camera_motion,
    })
}

fn object_has_transform_motion(object: &RenderObjectDesc) -> bool {
    if !object.flags.contains(ObjectFlags::MOTION_VECTORS) {
        return false;
    }
    let Some(previous) = object.previous_transform else {
        return false;
    };
    mat4_has_meaningful_delta(previous, object.transform)
}

fn mat4_has_meaningful_delta(a: Mat4, b: Mat4) -> bool {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .any(|(lhs, rhs)| (*lhs - *rhs).abs() > f32::EPSILON)
}

fn motion_vector_output(
    renderer: &Renderer,
    view: &ViewDesc,
    motion: MotionVectorStats,
) -> Result<Option<FrameMotionVectorOutput>, RendererError> {
    if motion.views == 0 {
        return Ok(None);
    }
    let (width, height) = view_graph_extent(renderer, view)?;
    Ok(Some(FrameMotionVectorOutput {
        view_label: view.label.clone(),
        width,
        height,
        format: TextureFormat::Rgba16Float,
        moving_objects: motion.objects,
        camera_motion: motion.camera_motion,
    }))
}

fn post_process_outputs(
    renderer: &Renderer,
    view: &ViewDesc,
    frame_extensions: &[RenderGraphExtensionHandle],
) -> Result<Vec<FramePostProcessOutput>, RendererError> {
    let quality = effective_view_quality(view);
    let (width, height) = view_graph_extent(renderer, view)?;
    let format = view_main_color_format(renderer, view, &quality)?;
    let mut outputs = Vec::new();
    let mut has_intermediate = false;
    match effective_render_path(&renderer.config, view) {
        RenderPath::ForwardPlus => outputs.push(frame_post_process_output(
            view,
            "post_process",
            width,
            height,
            format,
            "main_color",
        )),
        RenderPath::Deferred | RenderPath::Auto => outputs.push(frame_post_process_output(
            view,
            "tonemap",
            width,
            height,
            format,
            "main_color",
        )),
        RenderPath::Forward => {}
    }
    for (enabled, pass_label, output_texture_label) in [
        (quality.taa, "taa", "taa_output"),
        (quality.fxaa, "fxaa", "fxaa_output"),
        (quality.motion_blur, "motion_blur", "motion_blur_output"),
        (quality.ssr, "ssr", "ssr_output"),
        (quality.bloom, "bloom", "bloom_output"),
        (
            quality.depth_of_field,
            "depth_of_field",
            "depth_of_field_output",
        ),
    ] {
        if enabled {
            outputs.push(frame_post_process_output(
                view,
                pass_label,
                width,
                height,
                format,
                output_texture_label,
            ));
            has_intermediate = true;
        }
    }
    if matches!(quality.color_grading, ColorGradingMode::Lut) {
        outputs.push(frame_post_process_output(
            view,
            "color_grading",
            width,
            height,
            format,
            "color_grading_output",
        ));
        has_intermediate = true;
    }
    if has_intermediate {
        outputs.push(frame_post_process_output(
            view,
            "post_process_resolve",
            width,
            height,
            format,
            "main_color",
        ));
    }
    for extension in frame_extensions.iter().chain(view.graph_extensions.iter()) {
        let extension = renderer
            .graph_extensions
            .get(ResourceKind::RenderGraphExtension, *extension)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::RenderGraphExtension,
                raw: extension.raw().get(),
            })?;
        if let Some(info) = extension.custom_post_process_info() {
            outputs.push(frame_post_process_output(
                view,
                &info.pass_label,
                width,
                height,
                format,
                &info.output_texture_label,
            ));
        }
    }
    Ok(outputs)
}

fn frame_post_process_output(
    view: &ViewDesc,
    pass_label: &str,
    width: u32,
    height: u32,
    format: TextureFormat,
    output_texture_label: &str,
) -> FramePostProcessOutput {
    FramePostProcessOutput {
        view_label: view.label.clone(),
        pass_label: pass_label.to_owned(),
        width,
        height,
        format,
        output_texture_label: output_texture_label.to_owned(),
    }
}

fn effective_view_quality(view: &ViewDesc) -> ViewQualitySettings {
    ViewQualitySettings {
        hdr: view.quality.hdr,
        bloom: view.quality.bloom || view.camera.flags.contains(CameraFlags::ENABLE_BLOOM),
        taa: view.quality.taa || view.camera.flags.contains(CameraFlags::ENABLE_TAA),
        fxaa: view.quality.fxaa,
        ssao: view.quality.ssao || view.camera.flags.contains(CameraFlags::ENABLE_SSAO),
        ssr: view.quality.ssr,
        depth_of_field: view.quality.depth_of_field,
        motion_blur: view.quality.motion_blur,
        variable_rate_shading: view.quality.variable_rate_shading,
        bindless_textures: view.quality.bindless_textures,
        mesh_shaders: view.quality.mesh_shaders,
        virtual_texturing: view.quality.virtual_texturing,
        ray_tracing: view.quality.ray_tracing,
        color_grading: view.quality.color_grading,
    }
}

fn object_bounds_outside_view(object: &RenderObjectDesc, view: &ViewDesc) -> bool {
    let Some(bounds) = object.bounds else {
        return false;
    };
    let center = transform_point3(object.transform, bounds.center());
    let radius = bounds_radius(bounds) * max_transform_scale(object.transform).max(0.0001);
    match view.camera.projection {
        Projection::Orthographic {
            width,
            height,
            near,
            far,
            ..
        } => {
            let view_pos = point_in_camera_space(view.camera.transform, center);
            view_pos.x.abs() - radius > width * 0.5
                || view_pos.y.abs() - radius > height * 0.5
                || -view_pos.z + radius < near
                || -view_pos.z - radius > far
        }
        Projection::Perspective {
            vertical_fov,
            aspect,
            near,
            far,
            ..
        } => {
            let view_pos = point_in_camera_space(view.camera.transform, center);
            let depth = -view_pos.z;
            if depth + radius < near {
                return true;
            }
            if far.is_some_and(|far| depth - radius > far) {
                return true;
            }
            let half_height = (vertical_fov * 0.5).tan() * depth.max(near);
            let half_width = half_height * aspect.max(0.0001);
            view_pos.x.abs() - radius > half_width || view_pos.y.abs() - radius > half_height
        }
        Projection::Custom { .. } => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PickCandidate {
    depth: f32,
    world_position: Vec3,
}

fn project_pick_candidate(
    object: &RenderObjectDesc,
    view: &ViewDesc,
    viewport: Viewport,
    pixel: Vec2,
) -> Option<PickCandidate> {
    let local_center = object.bounds.map(Bounds3::center).unwrap_or(Vec3::ZERO);
    let world_position = transform_point3(object.transform, local_center);
    let radius = object
        .bounds
        .map(|bounds| bounds_radius(bounds) * max_transform_scale(object.transform).max(0.0001))
        .unwrap_or(0.25);
    let view_pos = point_in_camera_space(view.camera.transform, world_position);
    let (screen, radius_pixels, depth) = match view.camera.projection {
        Projection::Orthographic {
            width,
            height,
            near,
            far,
            ..
        } => {
            let depth = -view_pos.z;
            if depth + radius < near || depth - radius > far {
                return None;
            }
            let screen = Vec2::new(
                viewport[0] + (view_pos.x / width + 0.5) * viewport[2],
                viewport[1] + (0.5 - view_pos.y / height) * viewport[3],
            );
            let radius_pixels =
                ((radius / width) * viewport[2]).max((radius / height) * viewport[3]);
            (screen, radius_pixels.max(1.0), depth.max(0.0))
        }
        Projection::Perspective {
            vertical_fov,
            aspect,
            near,
            far,
            ..
        } => {
            let depth = -view_pos.z;
            if depth + radius < near || far.is_some_and(|far| depth - radius > far) {
                return None;
            }
            let half_height = (vertical_fov * 0.5).tan() * depth.max(near);
            let half_width = half_height * aspect.max(0.0001);
            if half_width <= 0.0 || half_height <= 0.0 {
                return None;
            }
            let screen = Vec2::new(
                viewport[0] + (view_pos.x / half_width * 0.5 + 0.5) * viewport[2],
                viewport[1] + (0.5 - view_pos.y / half_height * 0.5) * viewport[3],
            );
            let radius_pixels = ((radius / half_width) * viewport[2] * 0.5)
                .max((radius / half_height) * viewport[3] * 0.5);
            (screen, radius_pixels.max(1.0), depth.max(0.0))
        }
        Projection::Custom { .. } => return None,
    };
    let dx = (pixel.x - screen.x).abs();
    let dy = (pixel.y - screen.y).abs();
    (dx <= radius_pixels && dy <= radius_pixels).then_some(PickCandidate {
        depth,
        world_position,
    })
}

fn point_in_camera_space(camera_transform: Mat4, point: Vec3) -> Vec3 {
    let relative = Vec3::new(
        point.x - camera_transform[3][0],
        point.y - camera_transform[3][1],
        point.z - camera_transform[3][2],
    );
    let right = Vec3::new(
        camera_transform[0][0],
        camera_transform[0][1],
        camera_transform[0][2],
    );
    let up = Vec3::new(
        camera_transform[1][0],
        camera_transform[1][1],
        camera_transform[1][2],
    );
    let backward = Vec3::new(
        camera_transform[2][0],
        camera_transform[2][1],
        camera_transform[2][2],
    );
    Vec3::new(
        dot_vec3(relative, right),
        dot_vec3(relative, up),
        dot_vec3(relative, backward),
    )
}

fn transform_point3(matrix: Mat4, point: Vec3) -> Vec3 {
    Vec3::new(
        point.x * matrix[0][0] + point.y * matrix[1][0] + point.z * matrix[2][0] + matrix[3][0],
        point.x * matrix[0][1] + point.y * matrix[1][1] + point.z * matrix[2][1] + matrix[3][1],
        point.x * matrix[0][2] + point.y * matrix[1][2] + point.z * matrix[2][2] + matrix[3][2],
    )
}

#[cfg_attr(not(feature = "backend-wgpu"), allow(dead_code))]
fn transform_point4(matrix: Mat4, point: Vec4) -> Vec4 {
    Vec4::new(
        point.x * matrix[0][0]
            + point.y * matrix[1][0]
            + point.z * matrix[2][0]
            + point.w * matrix[3][0],
        point.x * matrix[0][1]
            + point.y * matrix[1][1]
            + point.z * matrix[2][1]
            + point.w * matrix[3][1],
        point.x * matrix[0][2]
            + point.y * matrix[1][2]
            + point.z * matrix[2][2]
            + point.w * matrix[3][2],
        point.x * matrix[0][3]
            + point.y * matrix[1][3]
            + point.z * matrix[2][3]
            + point.w * matrix[3][3],
    )
}

#[cfg_attr(not(feature = "backend-wgpu"), allow(dead_code))]
fn frustum_corners_from_inverse_view_projection(inverse: Mat4) -> Option<[Vec3; 8]> {
    let clip = [
        Vec4::new(-1.0, -1.0, -1.0, 1.0),
        Vec4::new(1.0, -1.0, -1.0, 1.0),
        Vec4::new(-1.0, 1.0, -1.0, 1.0),
        Vec4::new(1.0, 1.0, -1.0, 1.0),
        Vec4::new(-1.0, -1.0, 1.0, 1.0),
        Vec4::new(1.0, -1.0, 1.0, 1.0),
        Vec4::new(-1.0, 1.0, 1.0, 1.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
    ];
    let mut corners = [Vec3::ZERO; 8];
    for (index, point) in clip.into_iter().enumerate() {
        let world = transform_point4(inverse, point);
        if !world.w.is_finite() || world.w.abs() <= f32::EPSILON {
            return None;
        }
        let inv_w = 1.0 / world.w;
        let corner = Vec3::new(world.x * inv_w, world.y * inv_w, world.z * inv_w);
        if !corner.x.is_finite() || !corner.y.is_finite() || !corner.z.is_finite() {
            return None;
        }
        corners[index] = corner;
    }
    Some(corners)
}

#[cfg_attr(not(feature = "backend-wgpu"), allow(dead_code))]
fn invert_mat4(matrix: Mat4) -> Option<Mat4> {
    let m = [
        matrix[0][0],
        matrix[1][0],
        matrix[2][0],
        matrix[3][0],
        matrix[0][1],
        matrix[1][1],
        matrix[2][1],
        matrix[3][1],
        matrix[0][2],
        matrix[1][2],
        matrix[2][2],
        matrix[3][2],
        matrix[0][3],
        matrix[1][3],
        matrix[2][3],
        matrix[3][3],
    ];
    let mut inv = [0.0; 16];
    inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15]
        + m[9] * m[7] * m[14]
        + m[13] * m[6] * m[11]
        - m[13] * m[7] * m[10];
    inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15]
        - m[8] * m[7] * m[14]
        - m[12] * m[6] * m[11]
        + m[12] * m[7] * m[10];
    inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15]
        + m[8] * m[7] * m[13]
        + m[12] * m[5] * m[11]
        - m[12] * m[7] * m[9];
    inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14]
        - m[8] * m[6] * m[13]
        - m[12] * m[5] * m[10]
        + m[12] * m[6] * m[9];
    inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15]
        - m[9] * m[3] * m[14]
        - m[13] * m[2] * m[11]
        + m[13] * m[3] * m[10];
    inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15]
        + m[8] * m[3] * m[14]
        + m[12] * m[2] * m[11]
        - m[12] * m[3] * m[10];
    inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15]
        - m[8] * m[3] * m[13]
        - m[12] * m[1] * m[11]
        + m[12] * m[3] * m[9];
    inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14]
        + m[8] * m[2] * m[13]
        + m[12] * m[1] * m[10]
        - m[12] * m[2] * m[9];
    inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15]
        + m[5] * m[3] * m[14]
        + m[13] * m[2] * m[7]
        - m[13] * m[3] * m[6];
    inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15]
        - m[4] * m[3] * m[14]
        - m[12] * m[2] * m[7]
        + m[12] * m[3] * m[6];
    inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15]
        + m[4] * m[3] * m[13]
        + m[12] * m[1] * m[7]
        - m[12] * m[3] * m[5];
    inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14]
        - m[4] * m[2] * m[13]
        - m[12] * m[1] * m[6]
        + m[12] * m[2] * m[5];
    inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11]
        - m[5] * m[3] * m[10]
        - m[9] * m[2] * m[7]
        + m[9] * m[3] * m[6];
    inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11]
        + m[4] * m[3] * m[10]
        + m[8] * m[2] * m[7]
        - m[8] * m[3] * m[6];
    inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11]
        - m[4] * m[3] * m[9]
        - m[8] * m[1] * m[7]
        + m[8] * m[3] * m[5];
    inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10]
        + m[4] * m[2] * m[9]
        + m[8] * m[1] * m[6]
        - m[8] * m[2] * m[5];
    let det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
    if !det.is_finite() || det.abs() <= f32::EPSILON {
        return None;
    }
    let inv_det = 1.0 / det;
    for value in &mut inv {
        *value *= inv_det;
    }
    Some([
        [inv[0], inv[4], inv[8], inv[12]],
        [inv[1], inv[5], inv[9], inv[13]],
        [inv[2], inv[6], inv[10], inv[14]],
        [inv[3], inv[7], inv[11], inv[15]],
    ])
}

fn bounds_radius(bounds: Bounds3) -> f32 {
    let extent = Vec3::new(
        (bounds.max.x - bounds.min.x).abs() * 0.5,
        (bounds.max.y - bounds.min.y).abs() * 0.5,
        (bounds.max.z - bounds.min.z).abs() * 0.5,
    );
    (extent.x * extent.x + extent.y * extent.y + extent.z * extent.z).sqrt()
}

fn max_transform_scale(matrix: Mat4) -> f32 {
    let scale = |column: usize| {
        let x = matrix[column][0];
        let y = matrix[column][1];
        let z = matrix[column][2];
        (x * x + y * y + z * z).sqrt()
    };
    scale(0).max(scale(1)).max(scale(2))
}

fn dot_vec3(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn vec3_distance(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn elapsed_ms(start: Instant, end: Instant) -> f32 {
    end.duration_since(start).as_secs_f64().min(f32::MAX as f64) as f32 * 1000.0
}

fn validation_checks_all_scene_objects(mode: ValidationMode) -> bool {
    matches!(mode, ValidationMode::Full | ValidationMode::GpuAssisted)
}

fn validation_checks_deep_resource_dependencies(mode: ValidationMode) -> bool {
    matches!(mode, ValidationMode::Full | ValidationMode::GpuAssisted)
}

type BatchKey = (u64, u32, u8, u64, u64, u64);

fn object_batch_discriminator(
    object_handle: ObjectHandle,
    object: &RenderObjectDesc,
    mesh_flags: MeshFlags,
) -> u64 {
    (object.flags.contains(ObjectFlags::NO_BATCH) || mesh_flags.contains(MeshFlags::NO_MERGE))
        .then(|| object_handle.raw().get())
        .unwrap_or(0)
}

fn object_render_state_hash(base_hash: u64, object: &RenderObjectDesc) -> u64 {
    base_hash | (u64::from(object.flags.contains(ObjectFlags::RECEIVE_SHADOW)) << 32)
}

fn material_for_submesh(
    materials: &[MaterialHandle],
    material_slot: u16,
) -> Option<MaterialHandle> {
    materials
        .get(material_slot as usize)
        .copied()
        .or_else(|| materials.first().copied())
}

fn material_pass_flags_contains_phase(passes: MaterialPassFlags, phase: RenderPhaseKind) -> bool {
    match phase {
        RenderPhaseKind::DepthPrepass => passes.contains(MaterialPassFlags::DEPTH_PREPASS),
        RenderPhaseKind::Shadow => passes.contains(MaterialPassFlags::SHADOW),
        RenderPhaseKind::GBuffer => passes.contains(MaterialPassFlags::GBUFFER),
        RenderPhaseKind::ForwardOpaque => passes.contains(MaterialPassFlags::FORWARD),
        RenderPhaseKind::ForwardTransparent => passes.contains(MaterialPassFlags::TRANSPARENT),
        RenderPhaseKind::MotionVector => passes.contains(MaterialPassFlags::MOTION),
        RenderPhaseKind::Picking => passes.contains(MaterialPassFlags::PICKING),
        RenderPhaseKind::Debug => false,
        RenderPhaseKind::Custom(index) => {
            MaterialPassFlags::custom(index).is_ok_and(|custom_pass| passes.contains(custom_pass))
        }
    }
}

fn standard_material_uses_transparent_phase(domain: MaterialDomain, alpha_mode: AlphaMode) -> bool {
    matches!(domain, MaterialDomain::Transparent)
        || matches!(
            alpha_mode,
            AlphaMode::Blend | AlphaMode::Premultiplied | AlphaMode::Additive
        )
}

fn standard_material_builtin_passes() -> MaterialPassFlags {
    MaterialPassFlags::DEPTH_PREPASS
        | MaterialPassFlags::SHADOW
        | MaterialPassFlags::GBUFFER
        | MaterialPassFlags::FORWARD
        | MaterialPassFlags::TRANSPARENT
        | MaterialPassFlags::MOTION
        | MaterialPassFlags::PICKING
}

fn standard_material_pass_flags(standard: &StandardMaterialDesc) -> MaterialPassFlags {
    if matches!(
        standard.domain,
        MaterialDomain::PostProcess | MaterialDomain::Sky
    ) {
        return MaterialPassFlags::empty();
    }
    let mut passes = MaterialPassFlags::PICKING | MaterialPassFlags::MOTION;
    match standard.domain {
        domain if standard_material_uses_transparent_phase(domain, standard.alpha_mode) => {
            passes = passes | MaterialPassFlags::TRANSPARENT;
        }
        _ => {
            passes = passes
                | MaterialPassFlags::DEPTH_PREPASS
                | MaterialPassFlags::GBUFFER
                | MaterialPassFlags::FORWARD;
            if standard.cast_shadows {
                passes = passes | MaterialPassFlags::SHADOW;
            }
        }
    }
    passes
}

fn object_sort_key(object: &RenderObjectDesc, view: &ViewDesc) -> u64 {
    let center = object.bounds.map(Bounds3::center).unwrap_or(Vec3::ZERO);
    let world_position = transform_point3(object.transform, center);
    let view_position = point_in_camera_space(view.camera.transform, world_position);
    let depth = (-view_position.z).max(0.0);
    u64::from(depth.to_bits())
}

fn view_batch_phase_candidates(render_path: RenderPath) -> &'static [RenderPhaseKind] {
    match render_path {
        RenderPath::Deferred | RenderPath::Auto => &[
            RenderPhaseKind::GBuffer,
            RenderPhaseKind::ForwardTransparent,
        ],
        RenderPath::Forward | RenderPath::ForwardPlus => &[
            RenderPhaseKind::ForwardOpaque,
            RenderPhaseKind::ForwardTransparent,
        ],
    }
}

fn sort_view_draw_items(items: &mut [DrawItem]) {
    items.sort_by_key(|item| {
        (
            render_phase_sort_rank(item.pipeline_key.pass),
            item.object.raw().get(),
        )
    });
    let mut start = 0;
    while start < items.len() {
        let phase = items[start].pipeline_key.pass;
        let mut end = start + 1;
        while end < items.len() && items[end].pipeline_key.pass == phase {
            end += 1;
        }
        let mode = match phase {
            RenderPhaseKind::ForwardTransparent => PhaseSortMode::BackToFront,
            RenderPhaseKind::DepthPrepass
            | RenderPhaseKind::GBuffer
            | RenderPhaseKind::ForwardOpaque
            | RenderPhaseKind::MotionVector
            | RenderPhaseKind::Picking
            | RenderPhaseKind::Custom(_) => PhaseSortMode::PipelineThenMaterialThenMesh,
            RenderPhaseKind::Shadow | RenderPhaseKind::Debug => PhaseSortMode::Unsorted,
        };
        mode.sort_draw_items(&mut items[start..end]);
        start = end;
    }
}

fn coalesce_instanced_draw_items(items: Vec<DrawItem>) -> Result<Vec<DrawItem>, RendererError> {
    let mut instanced = Vec::<DrawItem>::new();
    for mut item in items {
        let instance_count = item
            .instance_range
            .end
            .saturating_sub(item.instance_range.start);
        if instance_count == 0 {
            return Err(RendererError::Validation(
                "draw item instance range must not be empty".to_owned(),
            ));
        }
        if let Some(last) = instanced.last_mut() {
            let last_count = last
                .instance_range
                .end
                .saturating_sub(last.instance_range.start);
            if draw_items_share_instance_batch(last, &item) {
                last.instance_range = 0..last_count.saturating_add(instance_count);
                continue;
            }
        }
        item.instance_range = 0..instance_count;
        instanced.push(item);
    }
    Ok(instanced)
}

fn draw_items_share_instance_batch(a: &DrawItem, b: &DrawItem) -> bool {
    a.batch_key == b.batch_key
        && a.batch_key.5 == 0
        && b.batch_key.5 == 0
        && !matches!(a.pipeline_key.pass, RenderPhaseKind::ForwardTransparent)
}

fn render_state_hash(render_state: &RenderStateDesc) -> u64 {
    u64::from(render_state.depth_write)
}

fn standard_material_render_state_hash(material: &StandardMaterialDesc) -> u64 {
    let mut hash = u64::from(material_domain_rank(material.domain));
    hash |= u64::from(alpha_mode_rank(material.alpha_mode)) << 4;
    hash |= u64::from(material.double_sided) << 8;
    hash |= u64::from(material.receive_shadows) << 9;
    hash |= u64::from(material.cast_shadows) << 10;
    hash |= u64::from(material.base_color_texture.is_some()) << 11;
    hash |= u64::from(material.normal_texture.is_some()) << 12;
    hash |= u64::from(material.metallic_roughness_texture.is_some()) << 13;
    hash |= u64::from(material.occlusion_texture.is_some()) << 14;
    hash |= u64::from(material.emissive_texture.is_some()) << 15;
    hash
}

fn material_domain_rank(domain: MaterialDomain) -> u8 {
    match domain {
        MaterialDomain::Opaque => 0,
        MaterialDomain::AlphaCutout => 1,
        MaterialDomain::Transparent => 2,
        MaterialDomain::Decal => 3,
        MaterialDomain::Sky => 4,
        MaterialDomain::PostProcess => 5,
        MaterialDomain::Unlit => 6,
    }
}

fn alpha_mode_rank(alpha_mode: AlphaMode) -> u8 {
    match alpha_mode {
        AlphaMode::Opaque => 0,
        AlphaMode::Mask { .. } => 1,
        AlphaMode::Blend => 2,
        AlphaMode::Premultiplied => 3,
        AlphaMode::Additive => 4,
    }
}

fn count_unique_batch_keys(keys: &[BatchKey]) -> u32 {
    let mut unique = Vec::new();
    for key in keys {
        if !unique.contains(key) {
            unique.push(*key);
        }
    }
    unique.len() as u32
}

fn count_unique_materials(keys: &[BatchKey]) -> u32 {
    let mut unique = Vec::new();
    for (_, _, _, material, _, _) in keys {
        if !unique.contains(material) {
            unique.push(*material);
        }
    }
    unique.len() as u32
}

fn mesh_triangle_count(mesh: &StoredMesh) -> u64 {
    if mesh.info.index_count > 0 {
        return u64::from(mesh.info.index_count / 3);
    }
    vertex_count_for_semantic(mesh, VertexSemantic::Position)
        .map(|count| (count / 3) as u64)
        .unwrap_or(0)
}

fn mesh_resident_bytes(mesh: &StoredMesh) -> usize {
    mesh.vertex_bytes.len()
        + mesh.vertex_stream_bytes.iter().map(Vec::len).sum::<usize>()
        + mesh.index_bytes.len()
        + mesh
            .skin_inverse_bind_matrices
            .as_ref()
            .map_or(0, |matrices| matrices.len() * std::mem::size_of::<Mat4>())
        + mesh
            .morph_targets
            .iter()
            .map(|target| {
                target
                    .positions
                    .as_ref()
                    .map_or(0, |values| values.len() * std::mem::size_of::<Vec3>())
                    + target
                        .normals
                        .as_ref()
                        .map_or(0, |values| values.len() * std::mem::size_of::<Vec3>())
                    + target
                        .tangents
                        .as_ref()
                        .map_or(0, |values| values.len() * std::mem::size_of::<Vec3>())
            })
            .sum::<usize>()
        + mesh.meshlet_bytes.as_ref().map_or(0, Vec::len)
}

fn accumulate_graph_stats(target: &mut RenderGraphStats, source: &RenderGraphStats) {
    target.pass_count += source.pass_count;
    target
        .pass_labels
        .extend(source.pass_labels.iter().cloned());
    target.transient_textures += source.transient_textures;
    target.transient_buffers += source.transient_buffers;
    target.aliased_memory_bytes += source.aliased_memory_bytes;
    target.barriers += source.barriers;
    target.executed_callbacks += source.executed_callbacks;
    target.graphics_queue_passes += source.graphics_queue_passes;
    target.compute_queue_passes += source.compute_queue_passes;
    target.async_compute_queue_passes += source.async_compute_queue_passes;
    target.copy_queue_passes += source.copy_queue_passes;
    target.render_passes += source.render_passes;
    target.compute_passes += source.compute_passes;
    target.pipeline_binds += source.pipeline_binds;
    target.fullscreen_draws += source.fullscreen_draws;
    target.compute_dispatches += source.compute_dispatches;
    target.phase_draws += source.phase_draws;
    target.debug_groups += source.debug_groups;
    target.timestamp_queries += source.timestamp_queries;
    target.timestamp_writes += source.timestamp_writes;
    target.variable_rate_shading_passes += source.variable_rate_shading_passes;
    target.bindless_texture_table_passes += source.bindless_texture_table_passes;
    target.mesh_shader_passes += source.mesh_shader_passes;
    target.virtual_texture_feedback_passes += source.virtual_texture_feedback_passes;
    target.ray_tracing_passes += source.ray_tracing_passes;
    target.gpu_time_ns = match (target.gpu_time_ns, source.gpu_time_ns) {
        (Some(target_ns), Some(source_ns)) => Some(target_ns.saturating_add(source_ns)),
        (Some(target_ns), None) => Some(target_ns),
        (None, Some(source_ns)) => Some(source_ns),
        (None, None) => None,
    };
}

fn build_view_graph_stats(
    renderer: &Renderer,
    view: &ViewDesc,
    frame_extensions: &[RenderGraphExtensionHandle],
) -> Result<RenderGraphStats, RendererError> {
    let mut graph = RenderGraphBuilder::default();
    let quality = effective_view_quality(view);
    if quality.variable_rate_shading
        && !renderer.supports_feature(RendererFeature::VariableRateShading)
    {
        return Err(RendererError::UnsupportedFeature(
            RendererFeature::VariableRateShading,
        ));
    }
    if quality.bindless_textures && !renderer.supports_feature(RendererFeature::BindlessTextures) {
        return Err(RendererError::UnsupportedFeature(
            RendererFeature::BindlessTextures,
        ));
    }
    if quality.mesh_shaders && !renderer.supports_feature(RendererFeature::MeshShader) {
        return Err(RendererError::UnsupportedFeature(
            RendererFeature::MeshShader,
        ));
    }
    if quality.virtual_texturing && !renderer.supports_feature(RendererFeature::VirtualTexturing) {
        return Err(RendererError::UnsupportedFeature(
            RendererFeature::VirtualTexturing,
        ));
    }
    if quality.ray_tracing && !renderer.supports_feature(RendererFeature::RayTracing) {
        return Err(RendererError::UnsupportedFeature(
            RendererFeature::RayTracing,
        ));
    }
    let main_color_format = view_main_color_format(renderer, view, &quality)?;
    let (target_width, target_height) = view_graph_extent(renderer, view)?;
    let main_color = graph.create_texture(GraphTextureDesc {
        label: Some("main_color".to_owned()),
        width: target_width,
        height: target_height,
        format: main_color_format,
    });
    let main_depth = graph.create_texture(GraphTextureDesc {
        label: Some("main_depth".to_owned()),
        width: target_width,
        height: target_height,
        format: TextureFormat::Depth32Float,
    });
    let picking_id = graph.create_texture(GraphTextureDesc {
        label: Some("picking_id".to_owned()),
        width: target_width,
        height: target_height,
        format: TextureFormat::Rgba8Unorm,
    });
    let vrs_shading_rate_target = quality.variable_rate_shading.then(|| {
        graph.create_texture(GraphTextureDesc {
            label: Some("vrs_shading_rate".to_owned()),
            width: target_width.div_ceil(16).max(1),
            height: target_height.div_ceil(16).max(1),
            format: TextureFormat::Rgba8Unorm,
        })
    });
    let scene = renderer
        .scenes
        .get(ResourceKind::Scene, view.scene)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Scene,
            raw: view.scene.raw().get(),
        })?;
    let render_path = effective_render_path(&renderer.config, view);
    let environment_textures = import_environment_graph_textures(renderer, &mut graph, scene)?;
    let gbuffer_targets =
        matches!(render_path, RenderPath::Deferred | RenderPath::Auto).then(|| GBufferTargets {
            albedo: graph.create_texture(GraphTextureDesc {
                label: Some("gbuffer_albedo".to_owned()),
                width: target_width,
                height: target_height,
                format: TextureFormat::Rgba8Unorm,
            }),
            normal: graph.create_texture(GraphTextureDesc {
                label: Some("gbuffer_normal".to_owned()),
                width: target_width,
                height: target_height,
                format: TextureFormat::Rgba16Float,
            }),
            material: graph.create_texture(GraphTextureDesc {
                label: Some("gbuffer_material".to_owned()),
                width: target_width,
                height: target_height,
                format: TextureFormat::Rgba8Unorm,
            }),
        });
    let bindless_texture_table =
        quality.bindless_textures && view_uses_material_textures(renderer, scene, view)?;
    let virtual_texture_feedback =
        quality.virtual_texturing && view_uses_virtual_textures(renderer, scene, view)?;
    let texture_entry_count = (bindless_texture_table || virtual_texture_feedback)
        .then(|| -> Result<u32, RendererError> {
            view_bindless_texture_table_entries(renderer, scene, view)
        })
        .transpose()?;
    let bindless_texture_table_buffer = bindless_texture_table
        .then(|| -> Result<GraphBuffer, RendererError> {
            let entries = texture_entry_count.unwrap_or(0);
            Ok(graph.create_buffer(GraphBufferDesc {
                label: Some("bindless_texture_table".to_owned()),
                size: u64::from(entries).saturating_mul(16).max(16),
            }))
        })
        .transpose()?;
    let mesh_shader_path = quality.mesh_shaders && view_uses_meshlets(renderer, scene, view)?;
    let meshlet_culling_buffer_bytes = mesh_shader_path
        .then(|| -> Result<u64, RendererError> {
            view_meshlet_culling_buffer_bytes(renderer, scene, view)
        })
        .transpose()?;
    let meshlet_culling_buffer = mesh_shader_path
        .then(|| -> Result<GraphBuffer, RendererError> {
            let buffer_bytes = meshlet_culling_buffer_bytes.unwrap_or(0).max(4);
            Ok(graph.create_buffer(GraphBufferDesc {
                label: Some("gpu_meshlet_visibility".to_owned()),
                size: buffer_bytes,
            }))
        })
        .transpose()?;
    let virtual_texture_feedback_buffer = virtual_texture_feedback
        .then(|| -> Result<GraphBuffer, RendererError> {
            let entries = texture_entry_count.unwrap_or(0);
            Ok(graph.create_buffer(GraphBufferDesc {
                label: Some("virtual_texture_feedback".to_owned()),
                size: u64::from(entries).saturating_mul(16).max(16),
            }))
        })
        .transpose()?;
    let ray_tracing_accel_build =
        quality.ray_tracing && view_has_visible_geometry(renderer, scene, view)?;
    let ray_tracing_accel_geometry_count = ray_tracing_accel_build
        .then(|| view_visible_geometry_count(renderer, scene, view))
        .transpose()?;
    let ray_tracing_accel_buffer = ray_tracing_accel_build
        .then(|| -> Result<GraphBuffer, RendererError> {
            Ok(graph.create_buffer(GraphBufferDesc {
                label: Some("ray_tracing_accel".to_owned()),
                size: u64::from(ray_tracing_accel_geometry_count.unwrap_or(0))
                    .saturating_mul(64)
                    .max(64),
            }))
        })
        .transpose()?;
    let shadow_csm_atlas =
        directional_shadow_atlas_extent(renderer, scene, view)?.map(|(width, height, _)| {
            graph.create_texture(GraphTextureDesc {
                label: Some("shadow_csm_atlas".to_owned()),
                width,
                height,
                format: TextureFormat::Depth32Float,
            })
        });
    let shadow_point_spot_atlas =
        point_spot_shadow_atlas_extent(renderer, scene, view)?.map(|(width, height, _)| {
            graph.create_texture(GraphTextureDesc {
                label: Some("shadow_point_spot_atlas".to_owned()),
                width,
                height,
                format: TextureFormat::Depth32Float,
            })
        });
    let culling = view_culling_stats(renderer, view);
    let light_cluster_buffer = matches!(render_path, RenderPath::ForwardPlus).then(|| {
        graph.create_buffer(GraphBufferDesc {
            label: Some("light_cluster_grid".to_owned()),
            size: light_cluster_buffer_bytes(target_width, target_height),
        })
    });
    let culling_buffers = culling.enabled().then(|| CullingBuffers {
        visibility: graph.create_buffer(GraphBufferDesc {
            label: Some("gpu_visibility".to_owned()),
            size: u64::from(culling.tested_objects()).saturating_mul(4).max(4),
        }),
        indirect_args: graph.create_buffer(GraphBufferDesc {
            label: Some("gpu_indirect_args".to_owned()),
            size: u64::from(culling.visible_objects)
                .saturating_mul(16)
                .max(16),
        }),
        occlusion_results: culling.occlusion_culling.then(|| {
            graph.create_buffer(GraphBufferDesc {
                label: Some("gpu_occlusion_results".to_owned()),
                size: u64::from(culling.tested_objects()).saturating_mul(8).max(8),
            })
        }),
    });
    let deformation = view_deformation_stats(renderer, view)?;
    let deformation_output_buffer = (deformation.deformed_objects() > 0).then(|| {
        graph.create_buffer(GraphBufferDesc {
            label: Some("gpu_deformed_vertices".to_owned()),
            size: deformation.output_buffer_bytes.max(1),
        })
    });
    let motion = view_motion_vector_stats(renderer, view)?;
    let motion_vector_target = (motion.views > 0
        || matches!(render_path, RenderPath::Deferred | RenderPath::Auto))
    .then(|| {
        graph.create_texture(GraphTextureDesc {
            label: Some("motion_vectors".to_owned()),
            width: target_width,
            height: target_height,
            format: TextureFormat::Rgba16Float,
        })
    });
    let async_compute_enabled = renderer.supports_feature(RendererFeature::AsyncCompute);
    build_standard_view_graph(
        &mut graph,
        render_path,
        &quality,
        scene,
        bindless_texture_table,
        texture_entry_count,
        bindless_texture_table_buffer,
        mesh_shader_path,
        meshlet_culling_buffer_bytes,
        meshlet_culling_buffer,
        virtual_texture_feedback,
        virtual_texture_feedback_buffer,
        ray_tracing_accel_build,
        ray_tracing_accel_geometry_count,
        ray_tracing_accel_buffer,
        shadow_csm_atlas,
        shadow_point_spot_atlas,
        gbuffer_targets,
        debug_draw_enabled(renderer, view),
        environment_textures,
        light_cluster_buffer,
        async_compute_enabled,
        culling,
        culling_buffers,
        deformation,
        deformation_output_buffer,
        motion,
        motion_vector_target,
        vrs_shading_rate_target,
        main_color_format,
        target_width,
        target_height,
        picking_id,
        main_color,
        main_depth,
    );
    let ctx = RenderGraphExtensionContext::new(main_color, main_depth, renderer.caps.clone());
    for extension in frame_extensions.iter().chain(view.graph_extensions.iter()) {
        let extension = renderer
            .graph_extensions
            .get(ResourceKind::RenderGraphExtension, *extension)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::RenderGraphExtension,
                raw: extension.raw().get(),
            })?;
        extension.build(&ctx, &mut graph)?;
    }
    graph.execute_with_view_options(
        0,
        &renderer.caps,
        Some(ViewInfo {
            label: view.label.clone(),
            scene: view.scene,
            render_path,
            layers: view.layers,
        }),
        renderer.config.transient_resource_aliasing,
        renderer.config.debug_labels,
    )
}

fn view_main_color_format(
    renderer: &Renderer,
    view: &ViewDesc,
    quality: &ViewQualitySettings,
) -> Result<TextureFormat, RendererError> {
    if quality.hdr
        && renderer.config.hdr
        && renderer
            .caps
            .formats
            .color
            .contains(&TextureFormat::Rgba16Float)
    {
        return Ok(TextureFormat::Rgba16Float);
    }
    view_target_color_format(renderer, &view.target)
}

fn view_target_color_format(
    renderer: &Renderer,
    target: &RenderTarget,
) -> Result<TextureFormat, RendererError> {
    match *target {
        RenderTarget::MainSurface | RenderTarget::Surface(_) => Ok(renderer
            .config
            .surface_format
            .or_else(|| renderer.caps.formats.color.first().copied())
            .unwrap_or(TextureFormat::Rgba8Unorm)),
        RenderTarget::Texture(texture) => Ok(renderer
            .validated_texture_desc(texture, TextureUsage::RENDER_TARGET, "view target texture")?
            .format),
        RenderTarget::TextureView(view) => Ok(renderer
            .validated_texture_desc(
                view.texture,
                TextureUsage::RENDER_TARGET,
                "view target texture view",
            )?
            .format),
        RenderTarget::External(render_target) => {
            let desc =
                renderer
                    .render_target_desc(render_target)
                    .ok_or(RendererError::InvalidHandle {
                        kind: ResourceKind::RenderTarget,
                        raw: render_target.raw().get(),
                    })?;
            Ok(renderer
                .validated_texture_desc(
                    desc.color,
                    TextureUsage::RENDER_TARGET,
                    "view target color texture",
                )?
                .format)
        }
        RenderTarget::Headless { format, .. } => Ok(format),
    }
}

fn view_graph_extent(renderer: &Renderer, view: &ViewDesc) -> Result<(u32, u32), RendererError> {
    match view.target {
        RenderTarget::MainSurface | RenderTarget::Surface(_) => {
            if let Some(viewport) = view.camera.viewport {
                Ok((viewport[2].ceil() as u32, viewport[3].ceil() as u32))
            } else {
                Ok(renderer.surface_extent.unwrap_or((1, 1)))
            }
        }
        RenderTarget::Headless { width, height, .. } => Ok((width, height)),
        RenderTarget::Texture(texture) => {
            let desc = renderer.validated_texture_desc(
                texture,
                TextureUsage::RENDER_TARGET,
                "view target texture",
            )?;
            Ok((desc.width, desc.height))
        }
        RenderTarget::TextureView(texture_view) => {
            let desc = renderer.validated_texture_desc(
                texture_view.texture,
                TextureUsage::RENDER_TARGET,
                "view target texture view",
            )?;
            let divisor = 1u32
                .checked_shl(texture_view.base_mip)
                .unwrap_or(u32::MAX)
                .max(1);
            Ok((
                (desc.width / divisor).max(1),
                (desc.height / divisor).max(1),
            ))
        }
        RenderTarget::External(render_target) => {
            let desc =
                renderer
                    .render_target_desc(render_target)
                    .ok_or(RendererError::InvalidHandle {
                        kind: ResourceKind::RenderTarget,
                        raw: render_target.raw().get(),
                    })?;
            Ok((desc.width, desc.height))
        }
    }
}

fn effective_render_path(config: &RendererConfig, view: &ViewDesc) -> RenderPath {
    match view.render_path {
        RenderPath::Auto => match config.preferred_render_path {
            RenderPath::Auto => RenderPath::Deferred,
            path => path,
        },
        path => path,
    }
}

fn build_standard_view_graph(
    graph: &mut RenderGraphBuilder<'_>,
    path: RenderPath,
    quality: &ViewQualitySettings,
    scene: &StoredScene,
    bindless_texture_table: bool,
    bindless_texture_table_entries: Option<u32>,
    bindless_texture_table_buffer: Option<GraphBuffer>,
    mesh_shader_path: bool,
    meshlet_culling_buffer_bytes: Option<u64>,
    meshlet_culling_buffer: Option<GraphBuffer>,
    virtual_texture_feedback: bool,
    virtual_texture_feedback_buffer: Option<GraphBuffer>,
    ray_tracing_accel_build: bool,
    ray_tracing_accel_geometry_count: Option<u32>,
    ray_tracing_accel_buffer: Option<GraphBuffer>,
    shadow_csm_atlas: Option<GraphTexture>,
    shadow_point_spot_atlas: Option<GraphTexture>,
    gbuffer_targets: Option<GBufferTargets>,
    debug_overlay: bool,
    environment_textures: EnvironmentGraphTextures,
    light_cluster_buffer: Option<GraphBuffer>,
    async_compute_enabled: bool,
    culling: CullingStats,
    culling_buffers: Option<CullingBuffers>,
    deformation: DeformationStats,
    deformation_output_buffer: Option<GraphBuffer>,
    motion: MotionVectorStats,
    motion_vector_target: Option<GraphTexture>,
    vrs_shading_rate_target: Option<GraphTexture>,
    main_color_format: TextureFormat,
    target_width: u32,
    target_height: u32,
    picking_id: GraphTexture,
    main_color: GraphTexture,
    main_depth: GraphTexture,
) {
    let mut previous = None;
    append_bindless_texture_table_pass(
        graph,
        &mut previous,
        async_compute_enabled,
        bindless_texture_table,
        bindless_texture_table_entries,
        bindless_texture_table_buffer,
    );
    append_virtual_texture_feedback_pass(
        graph,
        &mut previous,
        async_compute_enabled,
        virtual_texture_feedback,
        bindless_texture_table_entries,
        bindless_texture_table_buffer,
        virtual_texture_feedback_buffer,
    );
    append_vrs_shading_rate_pass(
        graph,
        &mut previous,
        async_compute_enabled,
        quality.variable_rate_shading,
        target_width,
        target_height,
        vrs_shading_rate_target,
    );
    let labels: Vec<&str> = match path {
        RenderPath::Deferred | RenderPath::Auto => {
            let mut labels = vec![
                "prepare_gpu_data",
                "depth_prepass",
                "gbuffer",
                "deferred_lighting",
                "sky",
                "transparent",
                "motion_vectors",
                "tonemap",
                "present",
            ];
            if shadow_csm_atlas.is_some() {
                labels.insert(1, "shadow_csm");
            }
            if shadow_point_spot_atlas.is_some() {
                let insert_at = if labels.contains(&"shadow_csm") { 2 } else { 1 };
                labels.insert(insert_at, "shadow_point_spot");
            }
            labels
        }
        RenderPath::ForwardPlus => vec![
            "depth_prepass",
            "light_cluster_build",
            "forward_opaque",
            "sky",
            "transparent",
            "post_process",
            "present",
        ],
        RenderPath::Forward => vec![
            "depth_prepass",
            "forward_opaque",
            "sky",
            "transparent",
            "present",
        ],
    };
    if !labels.contains(&"prepare_gpu_data") {
        append_prepare_gpu_data_passes(
            graph,
            &mut previous,
            &culling,
            &scene.desc,
            deformation,
            deformation_output_buffer,
            culling_buffers,
            async_compute_enabled,
            mesh_shader_path,
            meshlet_culling_buffer_bytes,
            meshlet_culling_buffer,
            ray_tracing_accel_build,
            ray_tracing_accel_geometry_count,
            ray_tracing_accel_buffer,
        );
    }
    let mut ssao_occlusion = None;
    for label in labels.iter().copied() {
        if label == "prepare_gpu_data" {
            append_prepare_gpu_data_passes(
                graph,
                &mut previous,
                &culling,
                &scene.desc,
                deformation,
                deformation_output_buffer,
                culling_buffers,
                async_compute_enabled,
                mesh_shader_path,
                meshlet_culling_buffer_bytes,
                meshlet_culling_buffer,
                ray_tracing_accel_build,
                ray_tracing_accel_geometry_count,
                ray_tracing_accel_buffer,
            );
            continue;
        }
        if label == "shadow_csm" {
            append_shadow_pass(
                graph,
                &mut previous,
                "shadow_csm",
                shadow_csm_atlas.expect("shadow_csm pass requires a directional shadow atlas"),
            );
            continue;
        }
        if label == "shadow_point_spot" {
            append_shadow_pass(
                graph,
                &mut previous,
                "shadow_point_spot",
                shadow_point_spot_atlas
                    .expect("shadow_point_spot pass requires a point/spot shadow atlas"),
            );
            continue;
        }
        if label == "light_cluster_build" {
            append_light_cluster_build_pass(
                graph,
                &mut previous,
                main_depth,
                target_width,
                target_height,
                async_compute_enabled,
                light_cluster_buffer.expect(
                    "Forward+ allocates a light cluster buffer before graph build",
                ),
            );
            continue;
        }
        if label == "gbuffer" {
            append_gbuffer_pass(
                graph,
                &mut previous,
                gbuffer_targets.expect("deferred path allocates GBuffer targets"),
                main_depth,
                culling_buffers,
                meshlet_culling_buffer,
                bindless_texture_table_buffer,
                virtual_texture_feedback_buffer,
            );
            ssao_occlusion = append_ssao_pass(
                graph,
                &mut previous,
                quality.ssao,
                main_depth,
                target_width,
                target_height,
            );
            continue;
        }
        let color_target = if label == "motion_vectors" {
            motion_vector_target
                .expect("motion vector target is allocated when motion pass is needed")
        } else {
            main_color
        };
        let color_ops = if label == "motion_vectors" {
            ColorAttachmentOps::clear_store()
        } else {
            ColorAttachmentOps::load_store()
        };
        let mut pass = graph
            .add_pass(label)
            .queue(if label == "light_cluster_build" {
                QueueType::Compute
            } else {
                QueueType::Graphics
            })
            .read_texture(main_depth, TextureReadUsage::Sampled)
            .color_attachment(color_target, color_ops)
            .depth_attachment(main_depth, DepthAttachmentOps::load_store());
        pass = attach_culling_draw_buffers(pass, label, culling_buffers);
        pass = attach_meshlet_visibility_buffer(pass, label, meshlet_culling_buffer);
        pass = attach_bindless_texture_table_buffer(pass, label, bindless_texture_table_buffer);
        pass = attach_virtual_texture_feedback_buffer(pass, label, virtual_texture_feedback_buffer);
        if let Some(occlusion) = ssao_occlusion {
            if matches!(label, "deferred_lighting" | "forward_opaque") {
                pass = pass.read_texture(occlusion, TextureReadUsage::Sampled);
            }
        }
        if let Some(clusters) = light_cluster_buffer {
            if label == "forward_opaque" {
                pass = pass.read_buffer(clusters, BufferReadUsage::Storage);
            }
        }
        if let Some(shadow_atlas) = shadow_csm_atlas {
            if matches!(label, "deferred_lighting" | "forward_opaque") {
                pass = pass.read_texture(shadow_atlas, TextureReadUsage::Sampled);
            }
        }
        if let Some(shadow_atlas) = shadow_point_spot_atlas {
            if matches!(label, "deferred_lighting" | "forward_opaque") {
                pass = pass.read_texture(shadow_atlas, TextureReadUsage::Sampled);
            }
        }
        if let Some(gbuffer) = gbuffer_targets {
            if label == "deferred_lighting" {
                pass = pass
                    .read_texture(gbuffer.albedo, TextureReadUsage::Sampled)
                    .read_texture(gbuffer.normal, TextureReadUsage::Sampled)
                    .read_texture(gbuffer.material, TextureReadUsage::Sampled);
            }
        }
        if label == "sky" {
            if let Some(skybox) = environment_textures.skybox {
                pass = pass.read_texture(skybox, TextureReadUsage::Sampled);
            }
        }
        if matches!(label, "deferred_lighting" | "forward_opaque") {
            for texture in [
                environment_textures.irradiance,
                environment_textures.prefiltered_specular,
                environment_textures.brdf_lut,
            ]
            .into_iter()
            .flatten()
            {
                pass = pass.read_texture(texture, TextureReadUsage::Sampled);
            }
        }
        if let Some(previous) = previous {
            pass = pass.depends_on(previous);
        }
        previous = Some(execute_standard_pass(pass, label));
        if label == "depth_prepass" {
            append_occlusion_culling_pass(
                graph,
                &mut previous,
                async_compute_enabled,
                &culling,
                &scene.desc,
                main_depth,
                culling_buffers,
            );
            append_picking_pass(graph, &mut previous, picking_id, main_depth);
            if !labels.contains(&"gbuffer") {
                ssao_occlusion = append_ssao_pass(
                    graph,
                    &mut previous,
                    quality.ssao,
                    main_depth,
                    target_width,
                    target_height,
                );
            }
        }
        if label == "transparent" && !labels.contains(&"motion_vectors") {
            append_motion_vector_pass(
                graph,
                &mut previous,
                motion,
                motion_vector_target,
                main_depth,
            );
            let mut post_color = append_post_process_passes(
                graph,
                &mut previous,
                quality,
                main_color,
                motion_vector_target,
                main_color_format,
                target_width,
                target_height,
            );
            if !labels.contains(&"post_process") && !labels.contains(&"tonemap") {
                post_color = append_color_grading_pass(
                    graph,
                    &mut previous,
                    quality,
                    post_color,
                    main_color_format,
                    target_width,
                    target_height,
                );
            }
            append_final_color_resolve(graph, &mut previous, post_color, main_color);
            append_debug_overlay_pass(graph, &mut previous, debug_overlay, main_color, main_depth);
        }
        if label == "motion_vectors" {
            let post_color = append_post_process_passes(
                graph,
                &mut previous,
                quality,
                main_color,
                motion_vector_target,
                main_color_format,
                target_width,
                target_height,
            );
            append_final_color_resolve(graph, &mut previous, post_color, main_color);
        }
        if label == "post_process" {
            let post_color = append_post_process_passes(
                graph,
                &mut previous,
                quality,
                main_color,
                motion_vector_target,
                main_color_format,
                target_width,
                target_height,
            );
            let post_color = append_color_grading_pass(
                graph,
                &mut previous,
                quality,
                post_color,
                main_color_format,
                target_width,
                target_height,
            );
            append_final_color_resolve(graph, &mut previous, post_color, main_color);
            append_debug_overlay_pass(graph, &mut previous, debug_overlay, main_color, main_depth);
        }
        if label == "tonemap" {
            let post_color = append_color_grading_pass(
                graph,
                &mut previous,
                quality,
                main_color,
                main_color_format,
                target_width,
                target_height,
            );
            append_final_color_resolve(graph, &mut previous, post_color, main_color);
            append_debug_overlay_pass(graph, &mut previous, debug_overlay, main_color, main_depth);
        }
    }
}

fn append_prepare_gpu_data_passes(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    culling: &CullingStats,
    scene: &SceneDesc,
    deformation: DeformationStats,
    deformation_output_buffer: Option<GraphBuffer>,
    culling_buffers: Option<CullingBuffers>,
    async_compute_enabled: bool,
    mesh_shader_path: bool,
    meshlet_culling_buffer_bytes: Option<u64>,
    meshlet_culling_buffer: Option<GraphBuffer>,
    ray_tracing_accel_build: bool,
    ray_tracing_accel_geometry_count: Option<u32>,
    ray_tracing_accel_buffer: Option<GraphBuffer>,
) {
    append_deformation_passes(
        graph,
        previous,
        async_compute_enabled,
        deformation,
        deformation_output_buffer,
    );
    append_gpu_culling_pass(
        graph,
        previous,
        async_compute_enabled,
        culling,
        scene,
        culling_buffers,
    );
    append_meshlet_culling_pass(
        graph,
        previous,
        async_compute_enabled,
        mesh_shader_path,
        meshlet_culling_buffer_bytes,
        meshlet_culling_buffer,
        deformation_output_buffer,
        culling_buffers,
    );
    append_ray_tracing_accel_build_pass(
        graph,
        previous,
        async_compute_enabled,
        ray_tracing_accel_build,
        ray_tracing_accel_geometry_count,
        ray_tracing_accel_buffer,
        deformation_output_buffer,
        culling_buffers,
    );
}

fn view_uses_material_textures(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (_, materials) = renderer.selected_object_resources(object, view)?;
        for material in materials {
            if material_uses_texture_bindings(renderer, *material)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn import_environment_graph_textures(
    renderer: &Renderer,
    graph: &mut RenderGraphBuilder<'_>,
    scene: &StoredScene,
) -> Result<EnvironmentGraphTextures, RendererError> {
    let Some(environment) = scene.environment else {
        return Ok(EnvironmentGraphTextures::default());
    };
    let desc = renderer
        .environment_desc(environment)
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Environment,
            raw: environment.raw().get(),
        })?;
    Ok(EnvironmentGraphTextures {
        skybox: import_optional_environment_texture(graph, "environment_skybox", desc.skybox),
        irradiance: import_optional_environment_texture(
            graph,
            "environment_irradiance",
            desc.irradiance,
        ),
        prefiltered_specular: import_optional_environment_texture(
            graph,
            "environment_prefiltered_specular",
            desc.prefiltered_specular,
        ),
        brdf_lut: import_optional_environment_texture(graph, "environment_brdf_lut", desc.brdf_lut),
    })
}

fn import_optional_environment_texture(
    graph: &mut RenderGraphBuilder<'_>,
    label: &'static str,
    texture: Option<TextureHandle>,
) -> Option<GraphTexture> {
    texture.map(|texture| graph.import_texture(label, texture, GraphTextureUsage::SAMPLED))
}

fn view_bindless_texture_table_entries(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<u32, RendererError> {
    let mut textures = HashSet::new();
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (_, materials) = renderer.selected_object_resources(object, view)?;
        for material in materials {
            for texture in material_texture_handles(renderer, *material)? {
                textures.insert(texture.raw().get());
            }
        }
    }
    Ok(textures.len().try_into().unwrap_or(u32::MAX))
}

fn material_uses_texture_bindings(
    renderer: &Renderer,
    material: MaterialHandle,
) -> Result<bool, RendererError> {
    Ok(!material_texture_handles(renderer, material)?.is_empty())
}

fn material_texture_handles(
    renderer: &Renderer,
    material: MaterialHandle,
) -> Result<Vec<TextureHandle>, RendererError> {
    let stored = renderer
        .materials
        .get(ResourceKind::Material, material)
        .and_then(|slot| slot.value.as_ref())
        .ok_or(RendererError::InvalidHandle {
            kind: ResourceKind::Material,
            raw: material.raw().get(),
        })?;
    let mut textures = Vec::new();
    if let Some(standard) = &stored.standard {
        textures.extend(
            [
                standard.base_color_texture,
                standard.normal_texture,
                standard.metallic_roughness_texture,
                standard.occlusion_texture,
                standard.emissive_texture,
            ]
            .into_iter()
            .flatten(),
        );
    }
    textures.extend(stored.parameters.values().filter_map(|value| match value {
        MaterialParameterValue::Texture(texture) => Some(*texture),
        _ => None,
    }));
    Ok(textures)
}

fn view_uses_virtual_textures(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (_, materials) = renderer.selected_object_resources(object, view)?;
        for material in materials {
            for texture in material_texture_handles(renderer, *material)? {
                let slot = renderer
                    .textures
                    .get(ResourceKind::Texture, texture)
                    .ok_or(RendererError::InvalidHandle {
                        kind: ResourceKind::Texture,
                        raw: texture.raw().get(),
                    })?;
                let Some(stored) = slot.value.as_ref() else {
                    return Err(RendererError::InvalidHandle {
                        kind: ResourceKind::Texture,
                        raw: texture.raw().get(),
                    });
                };
                if stored.desc.mip_levels > 1 && slot.priority == ResidencyPriority::Streamable {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn view_uses_meshlets(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (mesh, _) = renderer.selected_object_resources(object, view)?;
        let stored = renderer
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            })?;
        if stored
            .meshlet_bytes
            .as_ref()
            .is_some_and(|bytes| !bytes.is_empty())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn view_meshlet_culling_buffer_bytes(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<u64, RendererError> {
    let mut bytes = 0_u64;
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (mesh, _) = renderer.selected_object_resources(object, view)?;
        let stored = renderer
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
            .ok_or(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            })?;
        bytes = bytes.saturating_add(stored.meshlet_bytes.as_ref().map_or(0, Vec::len) as u64);
    }
    Ok(bytes)
}

fn view_has_visible_geometry(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<bool, RendererError> {
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (mesh, _) = renderer.selected_object_resources(object, view)?;
        let Some(stored_mesh) = renderer
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            });
        };
        if stored_mesh.info.usage.contains(MeshUsage::RAY_TRACING) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn view_visible_geometry_count(
    renderer: &Renderer,
    scene: &StoredScene,
    view: &ViewDesc,
) -> Result<u32, RendererError> {
    let mut count = 0_u32;
    for object in scene
        .objects
        .resources
        .iter()
        .filter_map(|slot| slot.value.as_ref())
    {
        if view_object_visibility(scene, object, view) != ViewObjectVisibility::Visible {
            continue;
        }
        if !object_material_has_geometry_phase(renderer, object, view)? {
            continue;
        }
        let (mesh, _) = renderer.selected_object_resources(object, view)?;
        let Some(stored_mesh) = renderer
            .meshes
            .get(ResourceKind::Mesh, mesh)
            .and_then(|slot| slot.value.as_ref())
        else {
            return Err(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                raw: mesh.raw().get(),
            });
        };
        if stored_mesh.info.usage.contains(MeshUsage::RAY_TRACING) {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

fn append_bindless_texture_table_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    enabled: bool,
    table_entry_count: Option<u32>,
    table_buffer: Option<GraphBuffer>,
) {
    if !enabled {
        return;
    }
    let table_buffer =
        table_buffer.expect("bindless texture table buffer is allocated when pass is enabled");
    let mut pass = graph
        .add_pass("bindless_texture_table")
        .queue(compute_queue(async_compute_enabled))
        .write_buffer(table_buffer, BufferWriteUsage::Storage);
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items(u64::from(table_entry_count.unwrap_or(1)), 64);
    *previous = Some(execute_standard_pass_with_dispatch(
        pass,
        "bindless_texture_table",
        dispatch,
    ));
}

fn append_virtual_texture_feedback_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    enabled: bool,
    texture_entry_count: Option<u32>,
    bindless_texture_table_buffer: Option<GraphBuffer>,
    feedback_buffer: Option<GraphBuffer>,
) {
    if !enabled {
        return;
    }
    let feedback_buffer =
        feedback_buffer.expect("virtual texture feedback buffer is allocated when pass is enabled");
    let mut pass = graph
        .add_pass("virtual_texture_feedback")
        .queue(compute_queue(async_compute_enabled))
        .write_buffer(feedback_buffer, BufferWriteUsage::Storage);
    if let Some(bindless_texture_table_buffer) = bindless_texture_table_buffer {
        pass = pass.read_buffer(bindless_texture_table_buffer, BufferReadUsage::Storage);
    }
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items(u64::from(texture_entry_count.unwrap_or(1)), 64);
    *previous = Some(execute_standard_pass_with_dispatch(
        pass,
        "virtual_texture_feedback",
        dispatch,
    ));
}

fn append_ray_tracing_accel_build_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    enabled: bool,
    visible_geometry_count: Option<u32>,
    accel_buffer: Option<GraphBuffer>,
    deformation_output_buffer: Option<GraphBuffer>,
    culling_buffers: Option<CullingBuffers>,
) {
    if !enabled {
        return;
    }
    let accel_buffer =
        accel_buffer.expect("ray tracing acceleration buffer is allocated when pass is enabled");
    let mut pass = graph
        .add_pass("ray_tracing_accel_build")
        .queue(compute_queue(async_compute_enabled))
        .write_buffer(accel_buffer, BufferWriteUsage::Storage);
    if let Some(deformation_output_buffer) = deformation_output_buffer {
        pass = pass.read_buffer(deformation_output_buffer, BufferReadUsage::Storage);
    }
    if let Some(culling_buffers) = culling_buffers {
        pass = pass
            .read_buffer(culling_buffers.visibility, BufferReadUsage::Storage)
            .read_buffer(culling_buffers.indirect_args, BufferReadUsage::Indirect);
    }
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items(u64::from(visible_geometry_count.unwrap_or(1)), 32);
    *previous = Some(execute_standard_pass_with_dispatch(
        pass,
        "ray_tracing_accel_build",
        dispatch,
    ));
}

fn append_meshlet_culling_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    enabled: bool,
    meshlet_buffer_bytes: Option<u64>,
    visibility_buffer: Option<GraphBuffer>,
    deformation_output_buffer: Option<GraphBuffer>,
    culling_buffers: Option<CullingBuffers>,
) {
    if !enabled {
        return;
    }
    let visibility_buffer =
        visibility_buffer.expect("meshlet visibility buffer is allocated when pass is enabled");
    let mut pass = graph
        .add_pass("meshlet_culling")
        .queue(compute_queue(async_compute_enabled))
        .write_buffer(visibility_buffer, BufferWriteUsage::Storage);
    if let Some(deformation_output_buffer) = deformation_output_buffer {
        pass = pass.read_buffer(deformation_output_buffer, BufferReadUsage::Storage);
    }
    if let Some(culling_buffers) = culling_buffers {
        pass = pass
            .read_buffer(culling_buffers.visibility, BufferReadUsage::Storage)
            .read_buffer(culling_buffers.indirect_args, BufferReadUsage::Indirect);
    }
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items((meshlet_buffer_bytes.unwrap_or(16)) / 16, 64);
    *previous = Some(execute_standard_pass_with_dispatch(
        pass,
        "meshlet_culling",
        dispatch,
    ));
}

fn append_post_process_passes(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    quality: &ViewQualitySettings,
    main_color: GraphTexture,
    motion_vector_target: Option<GraphTexture>,
    main_color_format: TextureFormat,
    width: u32,
    height: u32,
) -> GraphTexture {
    let mut color = main_color;
    color = append_quality_pass(
        graph,
        previous,
        quality.taa,
        "taa",
        color,
        main_color_format,
        width,
        height,
    );
    color = append_quality_pass(
        graph,
        previous,
        quality.fxaa,
        "fxaa",
        color,
        main_color_format,
        width,
        height,
    );
    color = append_motion_blur_pass(
        graph,
        previous,
        quality.motion_blur,
        color,
        motion_vector_target,
        main_color_format,
        width,
        height,
    );
    color = append_quality_pass(
        graph,
        previous,
        quality.ssr,
        "ssr",
        color,
        main_color_format,
        width,
        height,
    );
    color = append_quality_pass(
        graph,
        previous,
        quality.bloom,
        "bloom",
        color,
        main_color_format,
        width,
        height,
    );
    append_quality_pass(
        graph,
        previous,
        quality.depth_of_field,
        "depth_of_field",
        color,
        main_color_format,
        width,
        height,
    )
}

fn append_ssao_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    enabled: bool,
    main_depth: GraphTexture,
    width: u32,
    height: u32,
) -> Option<GraphTexture> {
    if !enabled {
        return None;
    }
    let output = graph.create_texture(GraphTextureDesc {
        label: Some("ssao_occlusion".to_owned()),
        width,
        height,
        format: TextureFormat::Rgba8Unorm,
    });
    let mut pass = graph
        .add_pass("ssao")
        .queue(QueueType::Graphics)
        .read_texture(main_depth, TextureReadUsage::Sampled)
        .color_attachment(output, ColorAttachmentOps::clear_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "ssao"));
    Some(output)
}

fn append_color_grading_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    quality: &ViewQualitySettings,
    main_color: GraphTexture,
    main_color_format: TextureFormat,
    width: u32,
    height: u32,
) -> GraphTexture {
    append_quality_pass(
        graph,
        previous,
        matches!(quality.color_grading, ColorGradingMode::Lut),
        "color_grading",
        main_color,
        main_color_format,
        width,
        height,
    )
}

fn append_quality_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    enabled: bool,
    label: &'static str,
    main_color: GraphTexture,
    main_color_format: TextureFormat,
    width: u32,
    height: u32,
) -> GraphTexture {
    if !enabled {
        return main_color;
    }
    let output = graph.create_texture(GraphTextureDesc {
        label: Some(format!("{label}_output")),
        width,
        height,
        format: main_color_format,
    });
    let mut pass = graph
        .add_pass(label)
        .queue(QueueType::Graphics)
        .read_texture(main_color, TextureReadUsage::Sampled)
        .color_attachment(output, ColorAttachmentOps::clear_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, label));
    output
}

fn append_motion_blur_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    enabled: bool,
    main_color: GraphTexture,
    motion_vector_target: Option<GraphTexture>,
    main_color_format: TextureFormat,
    width: u32,
    height: u32,
) -> GraphTexture {
    if !enabled {
        return main_color;
    }
    let motion_vector_target =
        motion_vector_target.expect("motion blur requires a motion vector target");
    let output = graph.create_texture(GraphTextureDesc {
        label: Some("motion_blur_output".to_owned()),
        width,
        height,
        format: main_color_format,
    });
    let mut pass = graph
        .add_pass("motion_blur")
        .queue(QueueType::Graphics)
        .read_texture(main_color, TextureReadUsage::Sampled)
        .read_texture(motion_vector_target, TextureReadUsage::Sampled)
        .color_attachment(output, ColorAttachmentOps::clear_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "motion_blur"));
    output
}

fn append_final_color_resolve(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    source: GraphTexture,
    target: GraphTexture,
) {
    if source == target {
        return;
    }
    let mut pass = graph
        .add_pass("post_process_resolve")
        .queue(QueueType::Graphics)
        .read_texture(source, TextureReadUsage::Sampled)
        .color_attachment(target, ColorAttachmentOps::load_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "post_process_resolve"));
}

fn append_debug_overlay_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    enabled: bool,
    main_color: GraphTexture,
    main_depth: GraphTexture,
) {
    if !enabled {
        return;
    }
    let mut pass = graph
        .add_pass("debug_overlay")
        .queue(QueueType::Graphics)
        .read_texture(main_depth, TextureReadUsage::Sampled)
        .color_attachment(main_color, ColorAttachmentOps::load_store())
        .depth_attachment(main_depth, DepthAttachmentOps::load_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "debug_overlay"));
}

fn append_vrs_shading_rate_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    enabled: bool,
    width: u32,
    height: u32,
    shading_rate_target: Option<GraphTexture>,
) {
    if !enabled {
        return;
    }
    let shading_rate_target =
        shading_rate_target.expect("VRS shading rate target is allocated when pass is enabled");
    let mut pass = graph
        .add_pass("vrs_shading_rate")
        .queue(compute_queue(async_compute_enabled))
        .write_texture(shading_rate_target, TextureWriteUsage::Storage);
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_area(width, height, 16, 16);
    *previous = Some(execute_standard_pass_with_dispatch(
        pass,
        "vrs_shading_rate",
        dispatch,
    ));
}

fn append_picking_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    picking_target: GraphTexture,
    main_depth: GraphTexture,
) {
    let mut pass = graph
        .add_pass("picking_id")
        .queue(QueueType::Graphics)
        .read_texture(main_depth, TextureReadUsage::Sampled)
        .color_attachment(picking_target, ColorAttachmentOps::clear_store())
        .depth_attachment(main_depth, DepthAttachmentOps::load_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "picking_id"));
}

fn append_motion_vector_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    motion: MotionVectorStats,
    motion_vector_target: Option<GraphTexture>,
    main_depth: GraphTexture,
) {
    if motion.views == 0 {
        return;
    }
    let motion_vector_target =
        motion_vector_target.expect("motion vector target is allocated when motion pass is needed");
    let mut pass = graph
        .add_pass("motion_vectors")
        .queue(QueueType::Graphics)
        .read_texture(main_depth, TextureReadUsage::Sampled)
        .color_attachment(motion_vector_target, ColorAttachmentOps::clear_store())
        .depth_attachment(main_depth, DepthAttachmentOps::load_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "motion_vectors"));
}

fn append_gpu_culling_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    culling: &CullingStats,
    scene: &SceneDesc,
    outputs: Option<CullingBuffers>,
) {
    let Some(outputs) = outputs else {
        return;
    };
    if !scene.enable_gpu_culling {
        return;
    }
    let mut cull_pass = graph
        .add_pass("gpu_culling")
        .queue(compute_queue(async_compute_enabled))
        .write_buffer(outputs.visibility, BufferWriteUsage::Storage)
        .write_buffer(outputs.indirect_args, BufferWriteUsage::Storage);
    if let Some(previous_pass) = *previous {
        cull_pass = cull_pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items(u64::from(culling.tested_objects().max(1)), 64);
    *previous = Some(execute_standard_pass_with_dispatch(
        cull_pass,
        "gpu_culling",
        dispatch,
    ));
}

fn append_occlusion_culling_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    culling: &CullingStats,
    scene: &SceneDesc,
    main_depth: GraphTexture,
    outputs: Option<CullingBuffers>,
) {
    let Some(outputs) = outputs else {
        return;
    };
    if !scene.enable_occlusion_culling {
        return;
    }
    let mut cull_pass = graph
        .add_pass("occlusion_culling")
        .queue(compute_queue(async_compute_enabled))
        .read_texture(main_depth, TextureReadUsage::Sampled)
        .write_buffer(outputs.visibility, BufferWriteUsage::Storage)
        .write_buffer(outputs.indirect_args, BufferWriteUsage::Storage);
    if let Some(occlusion_results) = outputs.occlusion_results {
        cull_pass = cull_pass.write_buffer(occlusion_results, BufferWriteUsage::Storage);
    }
    if let Some(previous_pass) = *previous {
        cull_pass = cull_pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items(u64::from(culling.tested_objects().max(1)), 64);
    *previous = Some(execute_standard_pass_with_dispatch(
        cull_pass,
        "occlusion_culling",
        dispatch,
    ));
}

fn append_shadow_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    label: &'static str,
    shadow_atlas: GraphTexture,
) {
    let mut pass = graph
        .add_pass(label)
        .queue(QueueType::Graphics)
        .depth_attachment(shadow_atlas, DepthAttachmentOps::load_store());
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, label));
}

fn append_gbuffer_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    targets: GBufferTargets,
    main_depth: GraphTexture,
    culling_buffers: Option<CullingBuffers>,
    meshlet_visibility_buffer: Option<GraphBuffer>,
    bindless_texture_table_buffer: Option<GraphBuffer>,
    virtual_texture_feedback_buffer: Option<GraphBuffer>,
) {
    let mut pass = graph
        .add_pass("gbuffer")
        .queue(QueueType::Graphics)
        .color_attachment(targets.albedo, ColorAttachmentOps::clear_store())
        .color_attachment(targets.normal, ColorAttachmentOps::clear_store())
        .color_attachment(targets.material, ColorAttachmentOps::clear_store())
        .depth_attachment(main_depth, DepthAttachmentOps::load_store());
    pass = attach_culling_draw_buffers(pass, "gbuffer", culling_buffers);
    pass = attach_meshlet_visibility_buffer(pass, "gbuffer", meshlet_visibility_buffer);
    pass = attach_bindless_texture_table_buffer(pass, "gbuffer", bindless_texture_table_buffer);
    pass = attach_virtual_texture_feedback_buffer(
        pass,
        "gbuffer",
        virtual_texture_feedback_buffer,
    );
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    *previous = Some(execute_standard_pass(pass, "gbuffer"));
}

fn attach_culling_draw_buffers<'a, 'b>(
    pass: crate::graph::PassBuilder<'a, 'b>,
    label: &str,
    culling_buffers: Option<CullingBuffers>,
) -> crate::graph::PassBuilder<'a, 'b> {
    let Some(culling_buffers) = culling_buffers else {
        return pass;
    };
    if !matches!(
        label,
        "depth_prepass" | "gbuffer" | "forward_opaque" | "transparent"
    ) {
        return pass;
    }
    pass.read_buffer(culling_buffers.visibility, BufferReadUsage::Storage)
        .read_buffer(culling_buffers.indirect_args, BufferReadUsage::Indirect)
}

fn attach_meshlet_visibility_buffer<'a, 'b>(
    pass: crate::graph::PassBuilder<'a, 'b>,
    label: &str,
    meshlet_visibility_buffer: Option<GraphBuffer>,
) -> crate::graph::PassBuilder<'a, 'b> {
    let Some(meshlet_visibility_buffer) = meshlet_visibility_buffer else {
        return pass;
    };
    if !matches!(
        label,
        "depth_prepass" | "gbuffer" | "forward_opaque" | "transparent"
    ) {
        return pass;
    }
    pass.read_buffer(meshlet_visibility_buffer, BufferReadUsage::Storage)
}

fn attach_bindless_texture_table_buffer<'a, 'b>(
    pass: crate::graph::PassBuilder<'a, 'b>,
    label: &str,
    bindless_texture_table_buffer: Option<GraphBuffer>,
) -> crate::graph::PassBuilder<'a, 'b> {
    let Some(bindless_texture_table_buffer) = bindless_texture_table_buffer else {
        return pass;
    };
    if !matches!(label, "gbuffer" | "forward_opaque" | "transparent") {
        return pass;
    }
    pass.read_buffer(bindless_texture_table_buffer, BufferReadUsage::Storage)
}

fn attach_virtual_texture_feedback_buffer<'a, 'b>(
    pass: crate::graph::PassBuilder<'a, 'b>,
    label: &str,
    virtual_texture_feedback_buffer: Option<GraphBuffer>,
) -> crate::graph::PassBuilder<'a, 'b> {
    let Some(virtual_texture_feedback_buffer) = virtual_texture_feedback_buffer else {
        return pass;
    };
    if !matches!(label, "gbuffer" | "forward_opaque" | "transparent") {
        return pass;
    }
    pass.write_buffer(virtual_texture_feedback_buffer, BufferWriteUsage::Storage)
}

fn append_light_cluster_build_pass(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    main_depth: GraphTexture,
    target_width: u32,
    target_height: u32,
    async_compute_enabled: bool,
    cluster_buffer: GraphBuffer,
) {
    let mut pass = graph
        .add_pass("light_cluster_build")
        .queue(if async_compute_enabled {
            QueueType::AsyncCompute
        } else {
            QueueType::Compute
        })
        .read_texture(main_depth, TextureReadUsage::Sampled)
        .write_buffer(cluster_buffer, BufferWriteUsage::Storage);
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_for_light_cluster_grid(target_width, target_height);
    *previous = Some(execute_standard_pass_with_dispatch(pass, "light_cluster_build", dispatch));
}

fn append_deformation_passes(
    graph: &mut RenderGraphBuilder<'_>,
    previous: &mut Option<PassId>,
    async_compute_enabled: bool,
    deformation: DeformationStats,
    output_buffer: Option<GraphBuffer>,
) {
    if deformation.deformed_objects() == 0 {
        return;
    }
    let mut pass = graph
        .add_pass("gpu_deformation")
        .queue(compute_queue(async_compute_enabled));
    if let Some(output_buffer) = output_buffer {
        pass = pass.write_buffer(output_buffer, BufferWriteUsage::Storage);
    }
    if let Some(previous_pass) = *previous {
        pass = pass.depends_on(previous_pass);
    }
    let dispatch = dispatch_workgroups_for_items(u64::from(deformation.deformed_objects()), 64);
    *previous = Some(execute_standard_pass_with_dispatch(
        pass,
        "gpu_deformation",
        dispatch,
    ));
}

fn execute_standard_pass(pass: crate::graph::PassBuilder<'_, '_>, label: &str) -> PassId {
    execute_standard_pass_with_dispatch(pass, label, (1, 1, 1))
}

const fn compute_queue(async_compute_enabled: bool) -> QueueType {
    if async_compute_enabled {
        QueueType::AsyncCompute
    } else {
        QueueType::Compute
    }
}

fn execute_standard_pass_with_dispatch(
    pass: crate::graph::PassBuilder<'_, '_>,
    label: &str,
    dispatch: (u32, u32, u32),
) -> PassId {
    let label = label.to_owned();
        pass.execute(move |ctx| {
            if standard_compute_pass(label.as_str()) {
                let mut pass = ctx.begin_compute_pass(ComputePassDesc::label(label.as_str()));
                pass.dispatch_workgroups(dispatch.0, dispatch.1, dispatch.2);
            } else {
            if let Some(phase) = standard_render_phase(label.as_str()) {
                ctx.draw_render_phase(phase)?;
            }
            let pipeline = standard_fullscreen_pass(label.as_str())
                .then(|| ctx.pipeline(format!("{}_pipeline", label)))
                .transpose()?;
            let mut pass = ctx.begin_render_pass(RenderPassDesc::label(label.as_str()));
            if let Some(pipeline) = pipeline {
                pass.set_pipeline(pipeline);
                pass.draw_fullscreen_triangle();
            }
        }
        Ok(())
    })
}

fn dispatch_workgroups_for_items(item_count: u64, block_size: u32) -> (u32, u32, u32) {
    let workgroup_size = block_size.max(1);
    let workgroup_stride = u64::from(workgroup_size).saturating_sub(1);
    let groups = item_count.saturating_add(workgroup_stride) / u64::from(workgroup_size);
    (to_u32_saturating(groups.max(1)), 1, 1)
}

fn dispatch_workgroups_for_area(
    width: u32,
    height: u32,
    block_x: u32,
    block_y: u32,
) -> (u32, u32, u32) {
    let blocks_x = width.div_ceil(block_x.max(1)).max(1);
    let blocks_y = height.div_ceil(block_y.max(1)).max(1);
    (blocks_x, blocks_y, 1)
}

fn dispatch_for_light_cluster_grid(width: u32, height: u32) -> (u32, u32, u32) {
    (
        width
            .div_ceil(LIGHT_CLUSTER_TILE_SIZE.max(1))
            .max(1),
        height
            .div_ceil(LIGHT_CLUSTER_TILE_SIZE.max(1))
            .max(1),
        LIGHT_CLUSTER_Z_SLICES,
    )
}

fn to_u32_saturating(value: u64) -> u32 {
    match u32::try_from(value) {
        Ok(value) => value,
        Err(_) => u32::MAX,
    }
}

fn standard_render_phase(label: &str) -> Option<RenderPhaseKind> {
    match label {
        "depth_prepass" => Some(RenderPhaseKind::DepthPrepass),
        "shadow_csm" | "shadow_point_spot" => Some(RenderPhaseKind::Shadow),
        "gbuffer" => Some(RenderPhaseKind::GBuffer),
        "forward_opaque" => Some(RenderPhaseKind::ForwardOpaque),
        "transparent" => Some(RenderPhaseKind::ForwardTransparent),
        "motion_vectors" => Some(RenderPhaseKind::MotionVector),
        "picking_id" => Some(RenderPhaseKind::Picking),
        "debug_overlay" => Some(RenderPhaseKind::Debug),
        _ => None,
    }
}

fn standard_compute_pass(label: &str) -> bool {
    matches!(
        label,
        "bindless_texture_table"
            | "virtual_texture_feedback"
            | "ray_tracing_accel_build"
            | "meshlet_culling"
            | "vrs_shading_rate"
            | "gpu_deformation"
            | "gpu_culling"
            | "occlusion_culling"
            | "light_cluster_build"
    )
}

fn standard_fullscreen_pass(label: &str) -> bool {
    matches!(
        label,
        "deferred_lighting"
            | "sky"
            | "motion_vectors"
            | "tonemap"
            | "post_process"
            | "present"
            | "picking_id"
            | "ssao"
            | "taa"
            | "fxaa"
            | "motion_blur"
            | "ssr"
            | "bloom"
            | "depth_of_field"
            | "color_grading"
            | "post_process_resolve"
            | "debug_overlay"
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawItem {
    pub object: ObjectHandle,
    pub mesh: MeshHandle,
    pub submesh_index: u32,
    pub material: MaterialHandle,
    pub pipeline_key: PipelineKey,
    pub sort_key: u64,
    pub instance_range: Range<u32>,
    batch_key: BatchKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseSortMode {
    FrontToBack,
    BackToFront,
    MaterialThenMesh,
    PipelineThenMaterialThenMesh,
    Unsorted,
}

impl PhaseSortMode {
    pub fn sort_draw_items(self, items: &mut [DrawItem]) {
        match self {
            Self::FrontToBack => items.sort_by_key(|item| item.sort_key),
            Self::BackToFront => items.sort_by_key(|item| std::cmp::Reverse(item.sort_key)),
            Self::MaterialThenMesh => items.sort_by_key(|item| {
                (
                    item.material.raw().get(),
                    item.mesh.raw().get(),
                    item.submesh_index,
                    item.object.raw().get(),
                )
            }),
            Self::PipelineThenMaterialThenMesh => items.sort_by_key(|item| {
                [
                    item.pipeline_key.shader.raw().get(),
                    item.pipeline_key.material_template.raw().get(),
                    item.pipeline_key.vertex_layout_hash,
                    item.pipeline_key.render_state_hash,
                    u64::from(render_phase_sort_rank(item.pipeline_key.pass)),
                    u64::from(item.pipeline_key.sample_count),
                    u64::from(depth_format_sort_rank(item.pipeline_key.depth_format)),
                    u64::from(texture_format_sort_rank(item.pipeline_key.color_format)),
                    item.pipeline_key.feature_bits,
                    item.material.raw().get(),
                    item.mesh.raw().get(),
                    u64::from(item.submesh_index),
                    item.object.raw().get(),
                ]
            }),
            Self::Unsorted => {}
        }
    }
}

fn render_phase_sort_rank(phase: RenderPhaseKind) -> u8 {
    match phase {
        RenderPhaseKind::DepthPrepass => 0,
        RenderPhaseKind::Shadow => 1,
        RenderPhaseKind::GBuffer => 2,
        RenderPhaseKind::ForwardOpaque => 3,
        RenderPhaseKind::ForwardTransparent => 4,
        RenderPhaseKind::MotionVector => 5,
        RenderPhaseKind::Picking => 6,
        RenderPhaseKind::Debug => 7,
        RenderPhaseKind::Custom(index) => 8_u8.saturating_add(index),
    }
}

fn depth_format_sort_rank(format: DepthFormat) -> u8 {
    match format {
        DepthFormat::D16Unorm => 0,
        DepthFormat::D24Plus => 1,
        DepthFormat::D24PlusStencil8 => 2,
        DepthFormat::D32Float => 3,
    }
}

fn texture_format_sort_rank(format: TextureFormat) -> u8 {
    match format {
        TextureFormat::Rgba8Unorm => 0,
        TextureFormat::Rgba8UnormSrgb => 1,
        TextureFormat::Bgra8UnormSrgb => 2,
        TextureFormat::Rgba16Float => 3,
        TextureFormat::Rgba32Float => 4,
        TextureFormat::Depth32Float => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        pin::pin,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    struct NoopExtension;

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on_ready<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut cx = Context::from_waker(&waker);
        let mut future = pin!(future);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("test future unexpectedly yielded"),
        }
    }

    impl RenderGraphExtension for NoopExtension {
        fn name(&self) -> &str {
            "noop_extension"
        }

        fn build(
            &self,
            _ctx: &RenderGraphExtensionContext,
            _graph: &mut RenderGraphBuilder<'_>,
        ) -> Result<(), RendererError> {
            Ok(())
        }
    }

    #[test]
    fn prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports() {
        let source = include_str!("lib.rs");
        let start = source.find("pub mod prelude {").unwrap();
        let end = source[start..].find("\n}\n\n#[derive").unwrap() + start;
        let prelude = &source[start..end];

        for forbidden in [
            "RhiAccess",
            "RhiDevice",
            "RhiCommandBuffer",
            "RenderGraphBuilder",
            "PassBuilder",
            "PassContext",
            "GraphTexture",
            "GraphBuffer",
            "GraphAccess",
            "RenderPassEncoder",
            "ComputePassEncoder",
            "CompiledRenderGraph",
            "ResourceBarrier",
            "TextureReadUsage",
            "BufferReadUsage",
            "QueueType",
            "DrawItem",
            "ShaderInterfaceDesc",
            "ShaderResourceBinding",
            "BindingClass",
            "BindingType",
            "PushConstantRange",
            "VertexInputRequirement",
        ] {
            assert!(
                !prelude.contains(forbidden),
                "prelude should not export graph/RHI or binding layout detail: {forbidden}"
            );
        }
    }

    fn test_bounds() -> Bounds3 {
        Bounds3::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0))
    }

    fn test_mesh(renderer: &mut Renderer, x_offset: f32) -> MeshHandle {
        test_mesh_with_usage_flags_and_meshlets(
            renderer,
            x_offset,
            MeshUsage::STATIC,
            MeshFlags::GPU_CULLABLE,
            None,
        )
    }

    fn test_mesh_with_meshlets(
        renderer: &mut Renderer,
        x_offset: f32,
        meshlets: Option<MeshletData<'_>>,
    ) -> MeshHandle {
        test_mesh_with_usage_flags_and_meshlets(
            renderer,
            x_offset,
            MeshUsage::STATIC,
            MeshFlags::GPU_CULLABLE,
            meshlets,
        )
    }

    fn test_mesh_with_usage(
        renderer: &mut Renderer,
        x_offset: f32,
        usage: MeshUsage,
    ) -> MeshHandle {
        test_mesh_with_usage_flags_and_meshlets(
            renderer,
            x_offset,
            usage,
            MeshFlags::GPU_CULLABLE,
            None,
        )
    }

    fn test_mesh_with_flags(
        renderer: &mut Renderer,
        x_offset: f32,
        flags: MeshFlags,
    ) -> MeshHandle {
        test_mesh_with_usage_flags_and_meshlets(renderer, x_offset, MeshUsage::STATIC, flags, None)
    }

    fn test_mesh_with_usage_flags_and_meshlets(
        renderer: &mut Renderer,
        x_offset: f32,
        usage: MeshUsage,
        flags: MeshFlags,
        meshlets: Option<MeshletData<'_>>,
    ) -> MeshHandle {
        let mut vertices = Vec::new();
        for (position, uv) in [
            ([x_offset, 0.5_f32, 0.0], [0.5_f32, 0.0]),
            ([x_offset - 0.5, -0.5, 0.0], [0.0, 1.0]),
            ([x_offset + 0.5, -0.5, 0.0], [1.0, 1.0]),
        ] {
            for value in position {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
            for value in [0.0_f32, 0.0, 1.0] {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
            for value in uv {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
        }
        renderer
            .create_mesh(MeshDesc {
                label: None,
                vertex_layout: VertexLayout {
                    streams: vec![VertexStreamLayout {
                        stride: 32,
                        step: VertexStepMode::Vertex,
                        attributes: vec![
                            VertexAttribute {
                                semantic: VertexSemantic::Position,
                                format: VertexFormat::Float32x3,
                                offset: 0,
                            },
                            VertexAttribute {
                                semantic: VertexSemantic::Normal,
                                format: VertexFormat::Float32x3,
                                offset: 12,
                            },
                            VertexAttribute {
                                semantic: VertexSemantic::TexCoord(0),
                                format: VertexFormat::Float32x2,
                                offset: 24,
                            },
                        ],
                    }],
                },
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage,
                flags,
                skin: None,
                morph_targets: Vec::new(),
                meshlets,
            })
            .unwrap()
    }

    fn test_standard_material(renderer: &mut Renderer) -> MaterialHandle {
        renderer
            .create_standard_material(StandardMaterialDesc {
                label: None,
                domain: MaterialDomain::Opaque,
                base_color: Color::WHITE,
                base_color_texture: None,
                normal_texture: None,
                metallic_roughness_texture: None,
                occlusion_texture: None,
                emissive_texture: None,
                metallic: 0.0,
                roughness: 0.5,
                emissive: Vec3::ZERO,
                alpha_mode: AlphaMode::Opaque,
                double_sided: false,
                receive_shadows: true,
                cast_shadows: true,
            })
            .unwrap()
    }

    fn test_sampled_texture(renderer: &mut Renderer, label: &str) -> TextureHandle {
        renderer
            .create_texture(TextureDesc {
                label: Some(label),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
                initial_data: None,
            })
            .unwrap()
    }

    fn test_camera() -> CameraDesc {
        CameraDesc {
            label: Some("camera".to_owned()),
            transform: IDENTITY_MAT4,
            projection: Projection::Perspective {
                vertical_fov: 1.0,
                aspect: 1.0,
                near: 0.1,
                far: Some(100.0),
                reverse_z: false,
            },
            exposure: Exposure::Auto,
            clear: ClearOptions::ColorDepth(Color::BLACK),
            viewport: None,
            scissor: None,
            jitter: None,
            previous_view_proj: None,
            flags: CameraFlags::MAIN,
        }
    }

    #[test]
    fn handles_encode_kind_index_and_generation() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("albedo"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
                initial_data: None,
            })
            .unwrap();

        assert_eq!(texture.index(), 0);
        assert_eq!(texture.generation(), 1);
        assert_eq!(texture.kind_tag(), ResourceKind::Texture.tag());
        assert_eq!(
            renderer.resource_status(texture),
            Some(ResourceStatus::Ready)
        );
    }

    #[test]
    fn renderer_features_cover_modern_renderer_capability_bits() {
        let modern = RendererFeatures::GPU_DRIVEN_RENDERING
            | RendererFeatures::OCCLUSION_CULLING
            | RendererFeatures::VIRTUAL_TEXTURING;

        assert!(RendererFeatures::empty().is_empty());
        assert!(!modern.is_empty());
        assert!(modern.contains(RendererFeatures::GPU_DRIVEN_RENDERING));
        assert!(modern.contains(RendererFeatures::OCCLUSION_CULLING));
        assert!(modern.contains(RendererFeatures::VIRTUAL_TEXTURING));
        assert!(!modern.contains(RendererFeatures::RAY_TRACING));

        let named_features = [
            RendererFeature::GpuDrivenRendering,
            RendererFeature::OcclusionCulling,
            RendererFeature::VirtualTexturing,
        ];
        assert_eq!(named_features.len(), 3);
    }

    #[test]
    fn renderer_capabilities_reflect_config_and_compile_features() {
        let mut config = RendererConfig {
            gpu_profiling: false,
            ..RendererConfig::default()
        };
        let renderer = Renderer::new_headless(config.clone());
        assert_eq!(renderer.capabilities().backend_name, "headless");
        assert!(renderer.supports_feature(RendererFeature::Compute));
        assert!(renderer.supports_feature(RendererFeature::StorageTextures));
        assert!(renderer.supports_feature(RendererFeature::OcclusionCulling));
        assert!(renderer.supports_feature(RendererFeature::ShaderReflection));
        assert!(!renderer.supports_feature(RendererFeature::TimestampQuery));
        assert!(!renderer.supports_feature(RendererFeature::Surface));
        assert_eq!(
            renderer.supports_feature(RendererFeature::RayTracing),
            cfg!(feature = "ray-tracing")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::MeshShader),
            cfg!(feature = "mesh-shader")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::BindlessTextures),
            cfg!(feature = "bindless")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::MultiDrawIndirect),
            cfg!(feature = "multi-draw-indirect")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::PipelineStatistics),
            cfg!(feature = "pipeline-statistics")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::AsyncCompute),
            cfg!(feature = "async-compute")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::VariableRateShading),
            cfg!(feature = "variable-rate-shading")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::GpuDrivenRendering),
            cfg!(feature = "gpu-driven")
        );
        assert_eq!(
            renderer.supports_feature(RendererFeature::VirtualTexturing),
            cfg!(feature = "virtual-texturing")
        );

        config.gpu_profiling = true;
        let profiled = Renderer::new_headless(config);
        assert!(profiled.supports_feature(RendererFeature::TimestampQuery));
    }

    #[test]
    fn renderer_new_selects_configured_backend_without_surface() {
        let headless = block_on_ready(Renderer::new(RendererConfig {
            backend: BackendPreference::Headless,
            ..RendererConfig::default()
        }))
        .unwrap();
        assert_eq!(headless.capabilities().backend_name, "headless");
        assert!(!headless.supports_feature(RendererFeature::Surface));

        let auto = block_on_ready(Renderer::new(RendererConfig {
            backend: BackendPreference::Auto,
            ..RendererConfig::default()
        }))
        .unwrap();
        #[cfg(feature = "backend-wgpu")]
        assert!(
            auto.capabilities().backend_name == "wgpu"
                || auto.capabilities().backend_name == "headless"
        );
        #[cfg(not(feature = "backend-wgpu"))]
        assert_eq!(auto.capabilities().backend_name, "headless");

        #[cfg(not(feature = "backend-wgpu"))]
        assert!(matches!(
            block_on_ready(Renderer::new(RendererConfig {
                backend: BackendPreference::Wgpu,
                ..RendererConfig::default()
            })),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::BackendWgpu
            ))
        ));

        #[cfg(feature = "backend-wgpu")]
        for backend in [
            BackendPreference::Wgpu,
            BackendPreference::Vulkan,
            BackendPreference::Metal,
            BackendPreference::D3d12,
        ] {
            assert_eq!(
                block_on_ready(Renderer::new(RendererConfig {
                    backend,
                    ..RendererConfig::default()
                }))
                .unwrap()
                .capabilities()
                .backend_name,
                "wgpu"
            );
        }
    }

    #[test]
    fn surface_backend_preference_accepts_only_surface_backends() {
        assert!(validate_surface_backend_preference(BackendPreference::Auto).is_ok());
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::Headless),
            Err(RendererError::UnsupportedFeature(RendererFeature::Surface))
        );
        #[cfg(feature = "backend-wgpu")]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::Vulkan),
            Ok(())
        );
        #[cfg(not(feature = "backend-wgpu"))]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::Vulkan),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::BackendVulkan
            ))
        );
        #[cfg(feature = "backend-wgpu")]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::Metal),
            Ok(())
        );
        #[cfg(not(feature = "backend-wgpu"))]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::Metal),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::BackendMetal
            ))
        );
        #[cfg(feature = "backend-wgpu")]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::D3d12),
            Ok(())
        );
        #[cfg(not(feature = "backend-wgpu"))]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::D3d12),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::BackendD3d12
            ))
        );

        #[cfg(feature = "backend-wgpu")]
        assert!(validate_surface_backend_preference(BackendPreference::Wgpu).is_ok());
        #[cfg(not(feature = "backend-wgpu"))]
        assert_eq!(
            validate_surface_backend_preference(BackendPreference::Wgpu),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::BackendWgpu
            ))
        );
    }

    #[test]
    fn main_surface_handle_participates_in_resource_queries() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let surface: SurfaceHandle = make_handle(ResourceKind::Surface, 0, 1);
        let stale_surface: SurfaceHandle = make_handle(ResourceKind::Surface, 0, 2);
        renderer.main_surface = Some(surface);

        assert_eq!(renderer.main_surface(), Some(surface));
        assert_eq!(
            renderer.resource_status(surface),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(renderer.resource_status(stale_surface), None);
        assert_eq!(
            renderer.resource_priority(surface),
            Some(ResidencyPriority::Normal)
        );
        assert_eq!(renderer.resource_priority(stale_surface), None);

        renderer
            .set_resource_priority(surface, ResidencyPriority::High)
            .unwrap();
        assert_eq!(
            renderer.resource_priority(surface),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.set_resource_priority(stale_surface, ResidencyPriority::Low),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Surface,
                raw: stale_surface.raw().get(),
            })
        );
        assert!(matches!(
            renderer.destroy(surface),
            Err(RendererError::Validation(_))
        ));
        assert_eq!(
            renderer.destroy(stale_surface),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Surface,
                raw: stale_surface.raw().get(),
            })
        );
    }

    #[test]
    fn generic_resource_errors_preserve_handle_kind_tags() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let object: ObjectHandle = make_handle(ResourceKind::Object, 7, 1);
        let light: LightHandle = make_handle(ResourceKind::Light, 3, 1);

        assert_eq!(renderer.resource_status(object), None);
        assert_eq!(renderer.resource_priority(light), None);
        assert_eq!(
            renderer.destroy(object),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Object,
                raw: object.raw().get(),
            })
        );
        assert_eq!(
            renderer.set_resource_priority(light, ResidencyPriority::Low),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Light,
                raw: light.raw().get(),
            })
        );
    }

    #[test]
    fn renderer_config_controls_initial_gpu_profiler_state() {
        let mut renderer = Renderer::new_headless(RendererConfig {
            gpu_profiling: true,
            ..RendererConfig::default()
        });
        let profiled = renderer
            .begin_frame(FrameInput::default())
            .unwrap()
            .finish()
            .unwrap();
        assert!(profiled.gpu_profiler_enabled);
        assert!(profiled.profile.is_some());
        assert_eq!(profiled.gpu_time_ms, Some(0.0));

        renderer.enable_gpu_profiler(false).unwrap();
        let unprofiled = renderer
            .begin_frame(FrameInput::default())
            .unwrap()
            .finish()
            .unwrap();
        assert!(!unprofiled.gpu_profiler_enabled);
        assert!(unprofiled.profile.is_none());
        assert_eq!(unprofiled.gpu_time_ms, None);
    }

    #[test]
    fn renderer_config_controls_transient_resource_aliasing_stats() {
        struct AliasProbeExtension;

        impl RenderGraphExtension for AliasProbeExtension {
            fn name(&self) -> &str {
                "alias_probe"
            }

            fn build(
                &self,
                _ctx: &RenderGraphExtensionContext,
                graph: &mut RenderGraphBuilder<'_>,
            ) -> Result<(), RendererError> {
                let a = graph.create_texture(GraphTextureDesc {
                    label: Some("alias_probe_a".to_owned()),
                    width: 4,
                    height: 4,
                    format: TextureFormat::Rgba8Unorm,
                });
                let b = graph.create_texture(GraphTextureDesc {
                    label: Some("alias_probe_b".to_owned()),
                    width: 4,
                    height: 4,
                    format: TextureFormat::Rgba8Unorm,
                });
                let first = graph
                    .add_pass("alias_probe_a")
                    .color_attachment(a, ColorAttachmentOps::clear_store())
                    .execute(|_| Ok(()));
                graph
                    .add_pass("alias_probe_b")
                    .depends_on(first)
                    .color_attachment(b, ColorAttachmentOps::clear_store())
                    .execute(|_| Ok(()));
                Ok(())
            }
        }

        let render_with_aliasing = |transient_resource_aliasing| {
            let mut renderer = Renderer::new_headless(RendererConfig {
                transient_resource_aliasing,
                ..RendererConfig::default()
            });
            let scene = renderer.create_scene(SceneDesc::default()).unwrap();
            let extension = renderer
                .register_graph_extension(AliasProbeExtension)
                .unwrap();
            let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
            frame
                .render_view(ViewDesc {
                    label: None,
                    scene,
                    camera: test_camera(),
                    target: RenderTarget::Headless {
                        width: 16,
                        height: 16,
                        format: TextureFormat::Rgba8Unorm,
                    },
                    render_path: RenderPath::Forward,
                    quality: ViewQualitySettings::default(),
                    layers: RenderLayerMask::all(),
                    graph_extensions: vec![extension],
                })
                .unwrap();
            frame.finish().unwrap().graph
        };

        let aliased = render_with_aliasing(true);
        assert!(aliased.aliased_memory_bytes > 0);

        let unaliased = render_with_aliasing(false);
        assert_eq!(unaliased.aliased_memory_bytes, 0);
        assert_eq!(unaliased.pass_labels, aliased.pass_labels);
        assert_eq!(unaliased.transient_textures, aliased.transient_textures);
        assert_eq!(unaliased.barriers, aliased.barriers);
    }

    #[test]
    fn renderer_config_controls_debug_label_groups() {
        let render_with_debug_labels = |debug_labels| {
            let mut renderer = Renderer::new_headless(RendererConfig {
                debug_labels,
                ..RendererConfig::default()
            });
            let scene = renderer.create_scene(SceneDesc::default()).unwrap();
            let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
            frame
                .render_view(ViewDesc {
                    label: Some("debug_labels".to_owned()),
                    scene,
                    camera: test_camera(),
                    target: RenderTarget::Headless {
                        width: 16,
                        height: 16,
                        format: TextureFormat::Rgba8Unorm,
                    },
                    render_path: RenderPath::Forward,
                    quality: ViewQualitySettings::default(),
                    layers: RenderLayerMask::all(),
                    graph_extensions: Vec::new(),
                })
                .unwrap();
            frame.finish().unwrap().graph
        };

        let labeled = render_with_debug_labels(true);
        assert_eq!(labeled.debug_groups, labeled.executed_callbacks);
        assert!(labeled.debug_groups > 0);

        let unlabeled = render_with_debug_labels(false);
        assert_eq!(unlabeled.debug_groups, 0);
        assert_eq!(unlabeled.pass_labels, labeled.pass_labels);
        assert_eq!(unlabeled.executed_callbacks, labeled.executed_callbacks);
    }

    #[test]
    fn renderer_config_validation_mode_controls_scene_resource_preflight() {
        fn renderer_with_destroyed_material(validation: ValidationMode) -> (Renderer, SceneHandle) {
            let mut renderer = Renderer::new_headless(RendererConfig {
                validation,
                ..RendererConfig::default()
            });
            let scene = renderer.create_scene(SceneDesc::default()).unwrap();
            let mesh = test_mesh(&mut renderer, 0.0);
            let material = renderer
                .create_standard_material(StandardMaterialDesc::default())
                .unwrap();
            renderer
                .edit_scene(scene, |scene| {
                    scene.spawn(RenderObjectDesc {
                        mesh,
                        materials: vec![material],
                        ..RenderObjectDesc::default()
                    });
                })
                .unwrap();
            renderer.destroy(material).unwrap();
            (renderer, scene)
        }

        let view = |scene| ViewDesc {
            label: Some("validation_mode".to_owned()),
            scene,
            camera: test_camera(),
            target: RenderTarget::Headless {
                width: 16,
                height: 16,
                format: TextureFormat::Rgba8Unorm,
            },
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings::default(),
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };

        let (mut basic_renderer, basic_scene) =
            renderer_with_destroyed_material(ValidationMode::Basic);
        let mut frame = basic_renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(view(basic_scene)),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                ..
            })
        ));

        let (mut off_renderer, off_scene) = renderer_with_destroyed_material(ValidationMode::Off);
        let mut frame = off_renderer.begin_frame(FrameInput::default()).unwrap();
        frame.render_view(view(off_scene)).unwrap();
        assert!(matches!(
            frame.finish(),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                ..
            })
        ));
    }

    #[test]
    fn full_validation_checks_hidden_objects_and_material_texture_dependencies() {
        fn view(scene: SceneHandle) -> ViewDesc {
            ViewDesc {
                label: Some("full_validation".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }
        }

        fn renderer_with_hidden_destroyed_material(
            validation: ValidationMode,
        ) -> (Renderer, SceneHandle) {
            let mut renderer = Renderer::new_headless(RendererConfig {
                validation,
                ..RendererConfig::default()
            });
            let scene = renderer.create_scene(SceneDesc::default()).unwrap();
            let mesh = test_mesh(&mut renderer, 0.0);
            let material = test_standard_material(&mut renderer);
            renderer
                .edit_scene(scene, |scene| {
                    scene.spawn(RenderObjectDesc {
                        mesh,
                        materials: vec![material],
                        visibility: VisibilityFlags::empty(),
                        ..RenderObjectDesc::default()
                    });
                })
                .unwrap();
            renderer.destroy(material).unwrap();
            (renderer, scene)
        }

        let (mut basic_renderer, basic_scene) =
            renderer_with_hidden_destroyed_material(ValidationMode::Basic);
        let mut frame = basic_renderer.begin_frame(FrameInput::default()).unwrap();
        frame.render_view(view(basic_scene)).unwrap();
        frame.finish().unwrap();

        let (mut full_renderer, full_scene) =
            renderer_with_hidden_destroyed_material(ValidationMode::Full);
        let mut frame = full_renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(view(full_scene)),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Material,
                ..
            })
        ));

        let mut gpu_assisted_renderer = Renderer::new_headless(RendererConfig {
            validation: ValidationMode::GpuAssisted,
            ..RendererConfig::default()
        });
        let scene = gpu_assisted_renderer
            .create_scene(SceneDesc::default())
            .unwrap();
        let mesh = test_mesh(&mut gpu_assisted_renderer, 0.0);
        let texture = test_sampled_texture(&mut gpu_assisted_renderer, "full_validation_albedo");
        let material = gpu_assisted_renderer
            .create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        gpu_assisted_renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        gpu_assisted_renderer.destroy(texture).unwrap();
        let mut frame = gpu_assisted_renderer
            .begin_frame(FrameInput::default())
            .unwrap();
        assert!(matches!(
            frame.render_view(view(scene)),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Texture,
                ..
            })
        ));
    }

    #[test]
    fn set_vsync_updates_renderer_config() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());

        renderer.set_vsync(VSyncMode::Off).unwrap();
        assert_eq!(renderer.config().vsync, VSyncMode::Off);

        renderer.set_vsync(VSyncMode::On).unwrap();
        assert_eq!(renderer.config().vsync, VSyncMode::On);

        renderer.set_vsync(VSyncMode::Adaptive).unwrap();
        assert_eq!(renderer.config().vsync, VSyncMode::Adaptive);
    }

    #[test]
    fn device_status_starts_ok_for_headless_renderer() {
        let renderer = Renderer::new_headless(RendererConfig::default());
        assert_eq!(renderer.device_status(), DeviceStatus::Ok);
    }

    #[test]
    fn begin_frame_rejects_lost_device() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.device_status = DeviceStatus::Lost;

        assert!(matches!(
            renderer.begin_frame(FrameInput::default()),
            Err(RendererError::DeviceLost { .. })
        ));
    }

    #[test]
    fn object_and_visibility_flags_cover_documented_scene_bits() {
        assert_eq!(VisibilityFlags::CAMERA.0, 1 << 0);
        assert_eq!(VisibilityFlags::SHADOW.0, 1 << 1);
        assert_eq!(VisibilityFlags::REFLECTION.0, 1 << 2);
        assert_eq!(VisibilityFlags::PICKING.0, 1 << 3);

        assert_eq!(ObjectFlags::STATIC.0, 1 << 0);
        assert_eq!(ObjectFlags::DYNAMIC.0, 1 << 1);
        assert_eq!(ObjectFlags::CAST_SHADOW.0, 1 << 2);
        assert_eq!(ObjectFlags::RECEIVE_SHADOW.0, 1 << 3);
        assert_eq!(ObjectFlags::MOTION_VECTORS.0, 1 << 4);
        assert_eq!(ObjectFlags::GPU_CULLABLE.0, 1 << 5);
        assert_eq!(ObjectFlags::NO_BATCH.0, 1 << 6);

        assert_eq!(
            (VisibilityFlags::CAMERA | VisibilityFlags::PICKING).0,
            (1 << 0) | (1 << 3)
        );
        assert!(VisibilityFlags::empty().is_empty());
        assert!(VisibilityFlags::empty().contains(VisibilityFlags::empty()));
        assert!(
            (VisibilityFlags::CAMERA | VisibilityFlags::PICKING).contains(VisibilityFlags::PICKING)
        );
        assert!(
            !(VisibilityFlags::CAMERA | VisibilityFlags::PICKING).contains(VisibilityFlags::SHADOW)
        );
        assert_eq!(
            (ObjectFlags::DYNAMIC | ObjectFlags::MOTION_VECTORS | ObjectFlags::GPU_CULLABLE).0,
            (1 << 1) | (1 << 4) | (1 << 5)
        );
        assert!(ObjectFlags::empty().is_empty());
        assert!(ObjectFlags::empty().contains(ObjectFlags::empty()));
        assert!((ObjectFlags::DYNAMIC | ObjectFlags::MOTION_VECTORS)
            .contains(ObjectFlags::MOTION_VECTORS));
    }

    #[test]
    fn renderer_public_flags_support_bitflag_queries() {
        assert!(MeshUsage::empty().is_empty());
        assert!(MeshUsage::empty().contains(MeshUsage::empty()));
        assert!((MeshUsage::STATIC | MeshUsage::CPU_READBACK).contains(MeshUsage::STATIC));
        assert!(!(MeshUsage::STATIC | MeshUsage::CPU_READBACK).contains(MeshUsage::DYNAMIC));

        assert!(MeshFlags::empty().is_empty());
        assert!(MeshFlags::empty().contains(MeshFlags::empty()));
        assert!((MeshFlags::ENABLE_SKINNING | MeshFlags::GPU_CULLABLE)
            .contains(MeshFlags::ENABLE_SKINNING));
        assert!(!(MeshFlags::ENABLE_SKINNING | MeshFlags::GPU_CULLABLE)
            .contains(MeshFlags::HAS_MESHLETS));

        assert!(BufferUsage::empty().is_empty());
        assert!(BufferUsage::empty().contains(BufferUsage::empty()));
        assert!((BufferUsage::UNIFORM | BufferUsage::COPY_DST).contains(BufferUsage::COPY_DST));
        assert!(!(BufferUsage::UNIFORM | BufferUsage::COPY_DST).contains(BufferUsage::VERTEX));

        assert!(TextureUsage::empty().is_empty());
        assert!(TextureUsage::empty().contains(TextureUsage::empty()));
        assert!((TextureUsage::SAMPLED | TextureUsage::COPY_DST).contains(TextureUsage::COPY_DST));
        assert!(
            !(TextureUsage::SAMPLED | TextureUsage::COPY_DST).contains(TextureUsage::RENDER_TARGET)
        );

        assert!(ShaderStages::empty().is_empty());
        assert!(ShaderStages::empty().contains(ShaderStages::empty()));
        assert!((ShaderStages::VERTEX | ShaderStages::FRAGMENT).contains(ShaderStages::VERTEX));
        assert!(!(ShaderStages::VERTEX | ShaderStages::FRAGMENT).contains(ShaderStages::COMPUTE));

        assert!(MaterialPassFlags::empty().is_empty());
        assert!(MaterialPassFlags::empty().contains(MaterialPassFlags::empty()));
        assert!((MaterialPassFlags::GBUFFER | MaterialPassFlags::MOTION)
            .contains(MaterialPassFlags::MOTION));
        assert!(!(MaterialPassFlags::GBUFFER | MaterialPassFlags::MOTION)
            .contains(MaterialPassFlags::TRANSPARENT));

        assert!(CameraFlags::empty().is_empty());
        assert!(CameraFlags::empty().contains(CameraFlags::empty()));
        assert!((CameraFlags::MAIN | CameraFlags::ENABLE_TAA).contains(CameraFlags::ENABLE_TAA));
        assert!(!(CameraFlags::MAIN | CameraFlags::ENABLE_TAA).contains(CameraFlags::ENABLE_BLOOM));
    }

    #[test]
    fn outline_pass_builds_documented_editor_extension() {
        let mut graph = RenderGraphBuilder::default();
        let depth = graph.create_texture(GraphTextureDesc {
            label: Some("main_depth".to_owned()),
            width: 1280,
            height: 720,
            format: TextureFormat::Depth32Float,
        });
        let output = graph.create_texture(GraphTextureDesc {
            label: Some("main_color".to_owned()),
            width: 1280,
            height: 720,
            format: TextureFormat::Rgba16Float,
        });
        let ctx = RenderGraphExtensionContext::new(output, depth, RendererCaps::default());
        let outline = OutlinePass {
            source_depth: depth,
            output,
            color: Color::WHITE,
        };

        assert_eq!(outline.name(), "editor_outline");
        outline.build(&ctx, &mut graph).unwrap();

        let stats = graph.execute(7, ctx.renderer_caps()).unwrap();
        assert_eq!(stats.pass_count, 1);
        assert_eq!(stats.pass_labels, vec!["editor_outline".to_owned()]);
        assert_eq!(stats.render_passes, 1);
        assert_eq!(stats.pipeline_binds, 1);
        assert_eq!(stats.fullscreen_draws, 1);
        assert_eq!(stats.barriers, 2);
    }

    #[test]
    fn phase_sort_modes_order_draw_items() {
        fn draw_item(
            object: u32,
            mesh: u32,
            material: u32,
            shader: u32,
            sort_key: u64,
        ) -> DrawItem {
            DrawItem {
                object: make_handle(ResourceKind::Object, object, 1),
                mesh: make_handle(ResourceKind::Mesh, mesh, 1),
                submesh_index: 0,
                material: make_handle(ResourceKind::Material, material, 1),
                pipeline_key: PipelineKey {
                    shader: make_handle(ResourceKind::Shader, shader, 1),
                    material_template: make_handle(ResourceKind::MaterialTemplate, material, 1),
                    vertex_layout_hash: mesh as u64,
                    render_state_hash: 0,
                    pass: RenderPhaseKind::ForwardOpaque,
                    sample_count: 1,
                    depth_format: DepthFormat::D32Float,
                    color_format: TextureFormat::Rgba8Unorm,
                    feature_bits: 0,
                },
                sort_key,
                instance_range: 0..1,
                batch_key: (mesh as u64, 0, 3, material as u64, 0, 0),
            }
        }

        let mut items = vec![
            draw_item(2, 2, 2, 2, 20),
            draw_item(1, 3, 1, 1, 10),
            draw_item(3, 1, 1, 0, 30),
        ];

        PhaseSortMode::FrontToBack.sort_draw_items(&mut items);
        assert_eq!(
            items.iter().map(|item| item.sort_key).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );

        PhaseSortMode::BackToFront.sort_draw_items(&mut items);
        assert_eq!(
            items.iter().map(|item| item.sort_key).collect::<Vec<_>>(),
            vec![30, 20, 10]
        );

        PhaseSortMode::MaterialThenMesh.sort_draw_items(&mut items);
        assert_eq!(
            items
                .iter()
                .map(|item| (item.material.index(), item.mesh.index()))
                .collect::<Vec<_>>(),
            vec![(1, 1), (1, 3), (2, 2)]
        );

        PhaseSortMode::PipelineThenMaterialThenMesh.sort_draw_items(&mut items);
        assert_eq!(
            items
                .iter()
                .map(|item| item.pipeline_key.shader.index())
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn view_draw_item_sorting_orders_phases_and_transparency() {
        fn draw_item(
            object: u32,
            mesh: u32,
            material: u32,
            shader: u32,
            phase: RenderPhaseKind,
            sort_key: u64,
        ) -> DrawItem {
            DrawItem {
                object: make_handle(ResourceKind::Object, object, 1),
                mesh: make_handle(ResourceKind::Mesh, mesh, 1),
                submesh_index: 0,
                material: make_handle(ResourceKind::Material, material, 1),
                pipeline_key: PipelineKey {
                    shader: make_handle(ResourceKind::Shader, shader, 1),
                    material_template: make_handle(ResourceKind::MaterialTemplate, material, 1),
                    vertex_layout_hash: mesh as u64,
                    render_state_hash: 0,
                    pass: phase,
                    sample_count: 1,
                    depth_format: DepthFormat::D32Float,
                    color_format: TextureFormat::Rgba8Unorm,
                    feature_bits: 0,
                },
                sort_key,
                instance_range: 0..1,
                batch_key: (
                    mesh as u64,
                    0,
                    render_phase_sort_rank(phase),
                    material as u64,
                    0,
                    0,
                ),
            }
        }

        let mut items = vec![
            draw_item(3, 3, 3, 3, RenderPhaseKind::ForwardTransparent, 30),
            draw_item(1, 2, 2, 2, RenderPhaseKind::ForwardOpaque, 10),
            draw_item(2, 1, 1, 1, RenderPhaseKind::ForwardOpaque, 20),
            draw_item(4, 4, 4, 4, RenderPhaseKind::ForwardTransparent, 40),
        ];

        sort_view_draw_items(&mut items);

        assert_eq!(
            items
                .iter()
                .map(|item| (item.pipeline_key.pass, item.sort_key))
                .collect::<Vec<_>>(),
            vec![
                (RenderPhaseKind::ForwardOpaque, 20),
                (RenderPhaseKind::ForwardOpaque, 10),
                (RenderPhaseKind::ForwardTransparent, 40),
                (RenderPhaseKind::ForwardTransparent, 30),
            ]
        );
    }

    #[test]
    fn view_draw_items_coalesce_instanced_opaque_batches() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);

        renderer
            .edit_scene(scene, |scene| {
                for index in 0..3 {
                    let mut transform = IDENTITY_MAT4;
                    transform[3][0] = index as f32;
                    scene.spawn(RenderObjectDesc {
                        label: Some(format!("instance_{index}")),
                        mesh,
                        materials: vec![material],
                        transform,
                        flags: if index == 2 {
                            ObjectFlags::STATIC | ObjectFlags::NO_BATCH
                        } else {
                            ObjectFlags::STATIC
                        },
                        ..RenderObjectDesc::default()
                    });
                }
            })
            .unwrap();

        let items = renderer
            .view_draw_items(&ViewDesc {
                label: Some("instanced_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 64,
                    height: 64,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Deferred,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].instance_range, 0..2);
        assert_eq!(items[1].instance_range, 0..1);
        assert_eq!(items[0].mesh, mesh);
        assert_eq!(items[0].material, material);
    }

    #[test]
    fn texture_uploads_validate_layout_and_subresource_ranges() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("mipped"),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 3,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
                initial_data: Some(TextureInitialData {
                    bytes: &[0; 64],
                    bytes_per_row: 16,
                    rows_per_image: 4,
                }),
            })
            .unwrap();

        renderer
            .update_texture(
                texture,
                TextureUpdate {
                    subresource: TextureSubresource {
                        mip_level: 1,
                        array_layer: 0,
                    },
                    region: TextureRegion {
                        offset: [1, 1, 0],
                        extent: [1, 1, 1],
                    },
                    bytes_per_row: 4,
                    rows_per_image: 1,
                    data: &[255; 4],
                },
            )
            .unwrap();

        assert!(matches!(
            renderer.update_texture(
                texture,
                TextureUpdate {
                    subresource: TextureSubresource {
                        mip_level: 3,
                        array_layer: 0,
                    },
                    region: TextureRegion {
                        offset: [0, 0, 0],
                        extent: [1, 1, 1],
                    },
                    bytes_per_row: 4,
                    rows_per_image: 1,
                    data: &[0; 4],
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_texture(
                texture,
                TextureUpdate {
                    subresource: TextureSubresource {
                        mip_level: 0,
                        array_layer: 0,
                    },
                    region: TextureRegion {
                        offset: [3, 0, 0],
                        extent: [2, 1, 1],
                    },
                    bytes_per_row: 8,
                    rows_per_image: 1,
                    data: &[0; 8],
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: Some(TextureInitialData {
                    bytes: &[0; 4],
                    bytes_per_row: 4,
                    rows_per_image: 2,
                }),
            }),
            Err(RendererError::Validation(_))
        ));
        let sampled_only = renderer
            .create_texture(TextureDesc {
                label: Some("sampled_only"),
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.update_texture(
                sampled_only,
                TextureUpdate {
                    subresource: TextureSubresource {
                        mip_level: 0,
                        array_layer: 0,
                    },
                    region: TextureRegion {
                        offset: [0, 0, 0],
                        extent: [1, 1, 1],
                    },
                    bytes_per_row: 4,
                    rows_per_image: 1,
                    data: &[0; 4],
                },
            ),
            Err(RendererError::Validation(_))
        ));
        let texture_3d = renderer
            .create_texture(TextureDesc {
                label: Some("volume"),
                dimension: TextureDimension::D3,
                width: 2,
                height: 2,
                depth_or_layers: 4,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::COPY_DST,
                initial_data: None,
            })
            .unwrap();
        renderer
            .update_texture(
                texture_3d,
                TextureUpdate {
                    subresource: TextureSubresource {
                        mip_level: 0,
                        array_layer: 0,
                    },
                    region: TextureRegion {
                        offset: [0, 0, 2],
                        extent: [1, 1, 1],
                    },
                    bytes_per_row: 4,
                    rows_per_image: 1,
                    data: &[0; 4],
                },
            )
            .unwrap();
        assert!(matches!(
            renderer.update_texture(
                texture_3d,
                TextureUpdate {
                    subresource: TextureSubresource {
                        mip_level: 0,
                        array_layer: 1,
                    },
                    region: TextureRegion {
                        offset: [0, 0, 0],
                        extent: [1, 1, 1],
                    },
                    bytes_per_row: 4,
                    rows_per_image: 1,
                    data: &[0; 4],
                },
            ),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn buffer_resources_are_retained_and_updatable() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let buffer = renderer
            .create_buffer(BufferDesc {
                label: Some("camera"),
                size: 8,
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST | BufferUsage::COPY_SRC,
                initial_data: Some(&[1, 2, 3, 4]),
            })
            .unwrap();

        let info = renderer.buffer_info(buffer).unwrap();
        assert_eq!(info.label.as_deref(), Some("camera"));
        assert_eq!(info.size, 8);
        assert!(info.usage.contains(BufferUsage::UNIFORM));
        assert_eq!(info.status, ResourceStatus::Ready);
        assert_eq!(
            renderer.buffer_bytes(buffer).unwrap(),
            &[1, 2, 3, 4, 0, 0, 0, 0]
        );

        renderer
            .update_buffer(
                buffer,
                BufferUpdate {
                    byte_offset: 4,
                    data: &[9, 10, 11, 12],
                },
            )
            .unwrap();
        assert_eq!(
            renderer.buffer_bytes(buffer).unwrap(),
            &[1, 2, 3, 4, 9, 10, 11, 12]
        );
        assert_eq!(
            renderer.resource_status(buffer),
            Some(ResourceStatus::Ready)
        );
        assert!(renderer.memory_stats().resident_bytes >= 8);
    }

    #[test]
    fn buffer_resources_validate_size_usage_and_update_ranges() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        assert!(matches!(
            renderer.create_buffer(BufferDesc {
                label: None,
                size: 0,
                usage: BufferUsage::UNIFORM,
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_buffer(BufferDesc {
                label: None,
                size: 4,
                usage: BufferUsage::empty(),
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_buffer(BufferDesc {
                label: None,
                size: 2,
                usage: BufferUsage::COPY_DST,
                initial_data: Some(&[1, 2, 3]),
            }),
            Err(RendererError::Validation(_))
        ));

        let buffer = renderer
            .create_buffer(BufferDesc {
                label: None,
                size: 4,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.update_buffer(
                buffer,
                BufferUpdate {
                    byte_offset: 0,
                    data: &[],
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_buffer(
                buffer,
                BufferUpdate {
                    byte_offset: 3,
                    data: &[1, 2],
                },
            ),
            Err(RendererError::Validation(_))
        ));
        let storage_only = renderer
            .create_buffer(BufferDesc {
                label: None,
                size: 4,
                usage: BufferUsage::STORAGE,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.update_buffer(
                storage_only,
                BufferUpdate {
                    byte_offset: 0,
                    data: &[1],
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_buffer(
                make_handle(ResourceKind::Buffer, 99, 1),
                BufferUpdate {
                    byte_offset: 0,
                    data: &[1],
                },
            ),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Buffer,
                ..
            })
        ));
    }

    #[test]
    fn generate_mips_builds_retained_rgba8_chain() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mut base = Vec::new();
        for value in [
            10_u8, 20, 50, 60, 30, 40, 70, 80, 90, 100, 130, 140, 110, 120, 150, 160,
        ] {
            base.extend_from_slice(&[value, value, value, 255]);
        }
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("mips"),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_SRC,
                initial_data: Some(TextureInitialData {
                    bytes: &base,
                    bytes_per_row: 16,
                    rows_per_image: 4,
                }),
            })
            .unwrap();

        renderer.generate_mips(texture).unwrap();

        let info = renderer.texture_info(texture).unwrap();
        assert_eq!(info.mip_levels, 3);
        let bytes = renderer.texture_bytes(texture).unwrap();
        assert_eq!(bytes.len(), 4 * 4 * 4 + 2 * 2 * 4 + 4);
        assert_eq!(
            &bytes[64..80],
            &[25, 25, 25, 255, 65, 65, 65, 255, 105, 105, 105, 255, 145, 145, 145, 255,]
        );
        assert_eq!(&bytes[80..84], &[85, 85, 85, 255]);
    }

    #[test]
    fn generate_mips_builds_layered_rgba8_chain() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mut base = Vec::new();
        for face in 0..6_u8 {
            let value = 10 + face * 20;
            for _ in 0..4 {
                base.extend_from_slice(&[value, value, value, 255]);
            }
        }
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("cube_mips"),
                dimension: TextureDimension::Cube,
                width: 2,
                height: 2,
                depth_or_layers: 6,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_SRC,
                initial_data: Some(TextureInitialData {
                    bytes: &base,
                    bytes_per_row: 8,
                    rows_per_image: 2,
                }),
            })
            .unwrap();

        renderer.generate_mips(texture).unwrap();

        let info = renderer.texture_info(texture).unwrap();
        assert_eq!(info.mip_levels, 2);
        let bytes = renderer.texture_bytes(texture).unwrap();
        assert_eq!(bytes.len(), 6 * 2 * 2 * 4 + 6 * 4);
        assert_eq!(&bytes[96..100], &[10, 10, 10, 255]);
        assert_eq!(&bytes[116..120], &[110, 110, 110, 255]);
    }

    #[test]
    fn generate_mips_builds_volume_rgba8_chain() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mut base = Vec::new();
        for value in [10_u8, 20, 30, 40, 50, 60, 70, 80] {
            base.extend_from_slice(&[value, value, value, 255]);
        }
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("volume_mips"),
                dimension: TextureDimension::D3,
                width: 2,
                height: 2,
                depth_or_layers: 2,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_SRC,
                initial_data: Some(TextureInitialData {
                    bytes: &base,
                    bytes_per_row: 8,
                    rows_per_image: 2,
                }),
            })
            .unwrap();

        renderer.generate_mips(texture).unwrap();

        let info = renderer.texture_info(texture).unwrap();
        assert_eq!(info.mip_levels, 2);
        let bytes = renderer.texture_bytes(texture).unwrap();
        assert_eq!(bytes.len(), 2 * 2 * 2 * 4 + 4);
        assert_eq!(&bytes[32..36], &[45, 45, 45, 255]);
    }

    #[test]
    fn generate_mips_rejects_missing_or_unsupported_base_data() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let empty = renderer
            .create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.generate_mips(empty),
            Err(RendererError::Validation(_))
        ));

        let unsupported = renderer
            .create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsage::SAMPLED,
                initial_data: Some(TextureInitialData {
                    bytes: &[0; 32],
                    bytes_per_row: 16,
                    rows_per_image: 2,
                }),
            })
            .unwrap();
        assert!(matches!(
            renderer.generate_mips(unsupported),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn texture_creation_validates_dimension_semantics() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D1,
                width: 4,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::empty(),
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::Cube,
                width: 4,
                height: 2,
                depth_or_layers: 6,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 4,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 3,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::Cube,
                width: 4,
                height: 4,
                depth_or_layers: 6,
                mip_levels: 1,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            }),
            Err(RendererError::Validation(_))
        ));
        renderer
            .create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 4,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        renderer
            .create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::CubeArray,
                width: 4,
                height: 4,
                depth_or_layers: 12,
                mip_levels: 3,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
    }

    #[test]
    fn begin_frame_rejects_invalid_time_inputs() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        assert!(matches!(
            renderer.begin_frame(FrameInput {
                delta_time: f32::NAN,
                absolute_time: 0.0,
                frame_index_override: None,
                wait_for_gpu: false,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.begin_frame(FrameInput {
                delta_time: 0.016,
                absolute_time: f64::INFINITY,
                frame_index_override: None,
                wait_for_gpu: false,
            }),
            Err(RendererError::Validation(_))
        ));
        renderer
            .begin_frame(FrameInput {
                delta_time: 0.0,
                absolute_time: 1.0,
                frame_index_override: Some(42),
                wait_for_gpu: false,
            })
            .unwrap();

        let stats = renderer
            .begin_frame(FrameInput {
                delta_time: 0.0,
                absolute_time: 1.0,
                frame_index_override: Some(7),
                wait_for_gpu: false,
            })
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(stats.frame_index, 7);
        assert_eq!(renderer.last_frame_stats().unwrap().frame_index, 7);

        let next_stats = renderer
            .begin_frame(FrameInput {
                delta_time: 0.0,
                absolute_time: 1.0,
                frame_index_override: None,
                wait_for_gpu: false,
            })
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(next_stats.frame_index, 8);
    }

    #[test]
    fn renderer_config_rejects_invalid_latency_and_msaa() {
        let mut config = RendererConfig::default();
        config.frame_latency = 0;
        assert!(matches!(
            validate_renderer_config(&config),
            Err(RendererError::Validation(_))
        ));

        config = RendererConfig::default();
        config.msaa_samples = 3;
        assert!(matches!(
            validate_renderer_config(&config),
            Err(RendererError::Validation(_))
        ));

        config.msaa_samples = 4;
        validate_renderer_config(&config).unwrap();

        config.surface_format = Some(TextureFormat::Depth32Float);
        assert!(matches!(
            validate_renderer_config(&config),
            Err(RendererError::Validation(_))
        ));

        config.surface_format = Some(TextureFormat::Rgba8Unorm);
        config.depth_format = DepthFormat::D16Unorm;
        assert!(matches!(
            validate_renderer_config(&config),
            Err(RendererError::Validation(_))
        ));

        config.depth_format = DepthFormat::D32Float;
        validate_renderer_config(&config).unwrap();

        config.backend = BackendPreference::Wgpu;
        let wgpu_status = validate_renderer_config(&config);
        if cfg!(feature = "backend-wgpu") {
            wgpu_status.unwrap();
        } else {
            assert!(matches!(
                wgpu_status,
                Err(RendererError::UnsupportedFeature(
                    RendererFeature::BackendWgpu
                ))
            ));
        }

        for (backend, feature) in [
            (BackendPreference::Vulkan, RendererFeature::BackendVulkan),
            (BackendPreference::Metal, RendererFeature::BackendMetal),
            (BackendPreference::D3d12, RendererFeature::BackendD3d12),
        ] {
            config.backend = backend;
            if cfg!(feature = "backend-wgpu") {
                assert!(validate_renderer_config(&config).is_ok());
            } else {
                assert!(matches!(
                    validate_renderer_config(&config),
                    Err(RendererError::UnsupportedFeature(actual)) if actual == feature
                ));
            }
        }
    }

    #[test]
    fn mesh_desc_is_retained_and_updatable() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let vertices = [0_u8; 36];
        let mesh = renderer
            .create_mesh(MeshDesc {
                label: Some("triangle"),
                vertex_layout: VertexLayout::default(),
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::GPU_CULLABLE | MeshFlags::NO_MERGE,
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            })
            .unwrap();

        renderer
            .update_mesh_vertices(mesh, 0, 40, &[1, 2, 3])
            .unwrap();
        renderer.update_mesh_indices(mesh, 6, &[3_u8, 0]).unwrap();
        let info = renderer.mesh_info(mesh).unwrap();
        assert_eq!(info.label.as_deref(), Some("triangle"));
        assert_eq!(info.vertex_bytes, 43);
        assert_eq!(info.index_count, 4);
        assert!(info.flags.contains(MeshFlags::GPU_CULLABLE));
        assert!(info.flags.contains(MeshFlags::NO_MERGE));
        assert!(matches!(
            renderer.update_mesh_indices(mesh, 1, &[0]),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn mesh_desc_rejects_invalid_submesh_ranges_and_stream_layouts() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let vertices = [0_u8; 36];
        let make_desc = |submesh: SubMeshDesc| MeshDesc {
            label: None,
            vertex_layout: VertexLayout {
                streams: vec![VertexStreamLayout {
                    stride: 12,
                    step: VertexStepMode::Vertex,
                    attributes: vec![VertexAttribute {
                        semantic: VertexSemantic::Position,
                        format: VertexFormat::Float32x3,
                        offset: 0,
                    }],
                }],
            },
            vertices: VertexData::Interleaved(&vertices),
            indices: Some(IndexData::U16(&[0, 1, 2])),
            submeshes: vec![submesh],
            bounds: test_bounds(),
            usage: MeshUsage::STATIC,
            flags: MeshFlags::default(),
            skin: None,
            morph_targets: Vec::new(),
            meshlets: None,
        };

        assert!(matches!(
            renderer.create_mesh(make_desc(SubMeshDesc {
                index_range: 0..4,
                vertex_range: 0..3,
                material_slot: 0,
                bounds: test_bounds(),
            })),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(make_desc(SubMeshDesc {
                index_range: 0..3,
                vertex_range: 0..4,
                material_slot: 0,
                bounds: test_bounds(),
            })),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(MeshDesc {
                label: None,
                vertex_layout: VertexLayout {
                    streams: vec![VertexStreamLayout {
                        stride: 10,
                        step: VertexStepMode::Vertex,
                        attributes: Vec::new(),
                    }],
                },
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::default(),
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(MeshDesc {
                label: None,
                vertex_layout: VertexLayout::default(),
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: Bounds3::new(Vec3::ONE, Vec3::ZERO),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::default(),
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn mesh_desc_validates_skin_morph_targets_and_meshlets() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let vertices = [0_u8; 36];
        let morph_positions = [Vec3::ZERO, Vec3::ONE, Vec3::new(0.0, 1.0, 0.0)];
        let morph_normals = [Vec3::new(0.0, 0.0, 1.0); 3];
        let short_morph = [Vec3::ZERO; 2];
        let mut non_finite_morph = morph_positions;
        non_finite_morph[0].x = f32::NAN;
        let mut non_finite_inverse_bind = IDENTITY_MAT4;
        non_finite_inverse_bind[0][0] = f32::INFINITY;

        fn make_desc<'a>(
            vertices: &'a [u8],
            skin: Option<SkinDesc<'a>>,
            morph_targets: Vec<MorphTargetDesc<'a>>,
            meshlets: Option<MeshletData<'a>>,
        ) -> MeshDesc<'a> {
            MeshDesc {
                label: None,
                vertex_layout: VertexLayout {
                    streams: vec![VertexStreamLayout {
                        stride: 12,
                        step: VertexStepMode::Vertex,
                        attributes: vec![VertexAttribute {
                            semantic: VertexSemantic::Position,
                            format: VertexFormat::Float32x3,
                            offset: 0,
                        }],
                    }],
                },
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::default(),
                skin,
                morph_targets,
                meshlets,
            }
        }

        let mesh = renderer
            .create_mesh(make_desc(
                &vertices,
                Some(SkinDesc {
                    inverse_bind_matrices: &[IDENTITY_MAT4],
                }),
                vec![MorphTargetDesc {
                    positions: Some(&morph_positions),
                    normals: Some(&morph_normals),
                    tangents: None,
                }],
                Some(MeshletData { bytes: &[1, 2, 3] }),
            ))
            .unwrap();
        let info = renderer.mesh_info(mesh).unwrap();
        assert_eq!(info.skin_joint_count, 1);
        assert_eq!(info.morph_target_count, 1);
        assert_eq!(info.meshlet_bytes, 3);

        assert!(matches!(
            renderer.create_mesh(make_desc(
                &vertices,
                Some(SkinDesc {
                    inverse_bind_matrices: &[],
                }),
                Vec::new(),
                None,
            )),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(make_desc(
                &vertices,
                Some(SkinDesc {
                    inverse_bind_matrices: &[non_finite_inverse_bind],
                }),
                Vec::new(),
                None,
            )),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(make_desc(
                &vertices,
                None,
                vec![MorphTargetDesc {
                    positions: Some(&short_morph),
                    normals: None,
                    tangents: None,
                }],
                None,
            )),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(make_desc(
                &vertices,
                None,
                vec![MorphTargetDesc {
                    positions: Some(&non_finite_morph),
                    normals: None,
                    tangents: None,
                }],
                None,
            )),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(make_desc(
                &vertices,
                None,
                vec![MorphTargetDesc {
                    positions: None,
                    normals: None,
                    tangents: None,
                }],
                None,
            )),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_mesh(make_desc(
                &vertices,
                None,
                Vec::new(),
                Some(MeshletData { bytes: &[] })
            )),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn camera_desc_is_retained_and_validated() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let camera = renderer.create_camera(test_camera()).unwrap();
        assert_eq!(
            renderer.camera_desc(camera).unwrap().label.as_deref(),
            Some("camera")
        );

        let mut updated = test_camera();
        updated.projection = Projection::Orthographic {
            width: 4.0,
            height: 2.0,
            near: -1.0,
            far: 10.0,
            reverse_z: false,
        };
        renderer.update_camera(camera, updated.clone()).unwrap();
        assert_eq!(renderer.camera_desc(camera), Some(&updated));

        let mut custom = test_camera();
        custom.projection = Projection::Custom {
            view: IDENTITY_MAT4,
            proj: IDENTITY_MAT4,
        };
        custom.viewport = Some([0.0, 0.0, 640.0, 480.0]);
        custom.scissor = Some([0, 0, 640, 480]);
        custom.jitter = Some(Vec2::new(0.25, -0.25));
        custom.flags = CameraFlags::MAIN
            | CameraFlags::ENABLE_SSAO
            | CameraFlags::ENABLE_SKY
            | CameraFlags::ENABLE_DEBUG_DRAW;
        renderer.update_camera(camera, custom.clone()).unwrap();
        assert_eq!(renderer.camera_desc(camera), Some(&custom));

        let mut invalid = test_camera();
        invalid.projection = Projection::Perspective {
            vertical_fov: 0.0,
            aspect: 1.0,
            near: 0.1,
            far: Some(100.0),
            reverse_z: false,
        };
        assert!(matches!(
            renderer.create_camera(invalid),
            Err(RendererError::Validation(_))
        ));
        let mut invalid_viewport = test_camera();
        invalid_viewport.viewport = Some([0.0, 0.0, 0.0, 480.0]);
        assert!(matches!(
            renderer.create_camera(invalid_viewport),
            Err(RendererError::Validation(_))
        ));
        let mut invalid_custom = test_camera();
        let mut bad_proj = IDENTITY_MAT4;
        bad_proj[1][1] = f32::NAN;
        invalid_custom.projection = Projection::Custom {
            view: IDENTITY_MAT4,
            proj: bad_proj,
        };
        assert!(matches!(
            renderer.create_camera(invalid_custom),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn render_view_validates_inline_camera_desc() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mut camera = test_camera();
        camera.viewport = Some([0.0, 0.0, f32::NAN, 16.0]);

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: Some("invalid_camera_view".to_owned()),
                scene,
                camera,
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn skeleton_instances_retain_inverse_bind_data_and_validate_counts() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let skeleton = renderer
            .create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("skinned"),
                joint_matrices: &[IDENTITY_MAT4, IDENTITY_MAT4],
                inverse_bind_matrices: Some(&[IDENTITY_MAT4, IDENTITY_MAT4]),
                usage: AnimationDataUsage::Streaming,
            })
            .unwrap();
        let info = renderer.skeleton_instance_info(skeleton).unwrap();
        assert_eq!(info.label.as_deref(), Some("skinned"));
        assert_eq!(info.joint_count, 2);
        assert_eq!(info.inverse_bind_count, 2);
        assert_eq!(info.usage, AnimationDataUsage::Streaming);

        assert!(matches!(
            renderer.create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("bad"),
                joint_matrices: &[IDENTITY_MAT4, IDENTITY_MAT4],
                inverse_bind_matrices: Some(&[IDENTITY_MAT4]),
                usage: AnimationDataUsage::Dynamic,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_skeleton_joints(skeleton, &[IDENTITY_MAT4]),
            Err(RendererError::Validation(_))
        ));

        let mut non_finite_joint = IDENTITY_MAT4;
        non_finite_joint[0][0] = f32::NAN;
        assert!(matches!(
            renderer.create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("bad_joint"),
                joint_matrices: &[non_finite_joint],
                inverse_bind_matrices: None,
                usage: AnimationDataUsage::Dynamic,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_skeleton_joints(skeleton, &[non_finite_joint, IDENTITY_MAT4]),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn morph_weights_are_retained_and_validate_finite_values() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let morph_weights = renderer
            .create_morph_weights(MorphWeightsDesc {
                label: Some("face"),
                weights: &[0.0, 0.5],
            })
            .unwrap();
        renderer
            .update_morph_weights(morph_weights, &[1.0, 0.25, 0.0])
            .unwrap();

        assert!(matches!(
            renderer.create_morph_weights(MorphWeightsDesc {
                label: Some("empty"),
                weights: &[],
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_morph_weights(morph_weights, &[f32::INFINITY]),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn morph_weights_can_be_created_from_documented_slice_api() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let morph_weights = renderer
            .create_morph_weights_from_slice(&[0.25, 0.5])
            .unwrap();

        renderer
            .update_morph_weights(morph_weights, &[1.0, 0.0, 0.75])
            .unwrap();

        let stored = renderer
            .morph_weights
            .get(ResourceKind::MorphWeights, morph_weights)
            .unwrap()
            .value
            .as_ref()
            .unwrap();
        assert_eq!(stored.label, None);
        assert_eq!(stored.weights, vec![1.0, 0.0, 0.75]);
        assert!(matches!(
            renderer.create_morph_weights_from_slice(&[]),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn scene_desc_defaults_and_hints_are_validated() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let desc = renderer.scene_desc(scene).unwrap();
        assert_eq!(desc.label, None);
        assert_eq!(desc.max_objects_hint, None);
        assert_eq!(desc.max_lights_hint, None);
        assert!(!desc.enable_gpu_culling);
        assert!(!desc.enable_occlusion_culling);

        let hinted_scene = renderer
            .create_scene(SceneDesc {
                label: Some("hinted".to_owned()),
                max_objects_hint: Some(4),
                max_lights_hint: Some(2),
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, hinted_scene)
            .unwrap()
            .value
            .as_ref()
            .unwrap();
        assert_eq!(stored.desc.max_objects_hint, Some(4));
        assert_eq!(stored.desc.max_lights_hint, Some(2));
        assert!(stored.objects.resources.capacity() >= 4);
        assert!(stored.lights.resources.capacity() >= 2);

        assert!(matches!(
            renderer.create_scene(SceneDesc {
                max_objects_hint: Some(0),
                ..SceneDesc::default()
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_scene(SceneDesc {
                max_lights_hint: Some(0),
                ..SceneDesc::default()
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn environment_ibl_slots_and_bake_facade_are_validated() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let default_environment = renderer
            .create_environment(EnvironmentDesc::default())
            .unwrap();
        let default_desc = renderer.environment_desc(default_environment).unwrap();
        assert_eq!(default_desc.skybox, None);
        assert_eq!(default_desc.irradiance, None);
        assert_eq!(default_desc.prefiltered_specular, None);
        assert_eq!(default_desc.brdf_lut, None);
        assert_eq!(default_desc.intensity, 1.0);
        assert_eq!(default_desc.rotation, Quat::IDENTITY);
        assert_eq!(default_desc.diffuse_color, Color::WHITE);
        assert_eq!(default_desc.diffuse_intensity, 1.0);
        assert_eq!(default_desc.specular_color, Color::WHITE);
        assert_eq!(default_desc.specular_intensity, 1.0);
        assert_eq!(default_desc.background_intensity, 1.0);

        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("env"),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let environment = renderer
            .create_environment(EnvironmentDesc {
                label: Some("ibl".to_owned()),
                skybox: Some(texture),
                irradiance: Some(texture),
                prefiltered_specular: Some(texture),
                brdf_lut: Some(texture),
                intensity: 1.25,
                rotation: Quat::IDENTITY,
                diffuse_color: Color::WHITE,
                diffuse_intensity: 1.25,
                specular_color: Color::WHITE,
                specular_intensity: 1.25,
                texture: None,
                background_intensity: 1.25,
            })
            .unwrap();
        assert_eq!(
            renderer.resource_status(environment),
            Some(ResourceStatus::Ready)
        );
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.set_environment(Some(environment)).unwrap();
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("environment_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Deferred,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert!(stats.graph.pass_labels.iter().any(|label| label == "sky"));
        assert_eq!(
            stats.environment_outputs,
            vec![FrameEnvironmentOutput {
                view_label: Some("environment_view".to_owned()),
                environment_label: Some("ibl".to_owned()),
                skybox_texture_label: Some("env".to_owned()),
                irradiance_texture_label: Some("env".to_owned()),
                prefiltered_specular_texture_label: Some("env".to_owned()),
                brdf_lut_texture_label: Some("env".to_owned()),
            }]
        );

        let baked = renderer
            .bake_environment(
                texture,
                EnvironmentBakeDesc {
                    label: Some("baked".to_owned()),
                    resolution: 64,
                    mip_levels: 5,
                    intensity: 0.8,
                    rotation: Quat::IDENTITY,
                },
            )
            .unwrap();
        assert_eq!(renderer.resource_status(baked), Some(ResourceStatus::Ready));
        let baked_desc = renderer.environment_desc(baked).unwrap();
        assert_eq!(baked_desc.skybox, Some(texture));
        assert_ne!(baked_desc.irradiance, Some(texture));
        assert_ne!(baked_desc.prefiltered_specular, Some(texture));
        assert!(baked_desc.brdf_lut.is_some());
        assert_eq!(
            renderer.resource_status(baked_desc.irradiance.unwrap()),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(baked_desc.prefiltered_specular.unwrap()),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(baked_desc.brdf_lut.unwrap()),
            Some(ResourceStatus::Ready)
        );
        let irradiance_info = renderer
            .texture_info(baked_desc.irradiance.unwrap())
            .unwrap();
        assert_eq!(irradiance_info.dimension, TextureDimension::Cube);
        assert_eq!(irradiance_info.width, 64);
        assert_eq!(irradiance_info.mip_levels, 1);
        assert_eq!(
            renderer
                .texture_bytes(baked_desc.irradiance.unwrap())
                .unwrap()
                .len(),
            64 * 64 * 6 * texture_format_bytes_per_pixel(TextureFormat::Rgba16Float) as usize
        );
        let prefiltered_info = renderer
            .texture_info(baked_desc.prefiltered_specular.unwrap())
            .unwrap();
        assert_eq!(prefiltered_info.dimension, TextureDimension::Cube);
        assert_eq!(prefiltered_info.mip_levels, 5);
        assert_eq!(
            renderer
                .texture_bytes(baked_desc.prefiltered_specular.unwrap())
                .unwrap()
                .len(),
            64 * 64 * 6 * texture_format_bytes_per_pixel(TextureFormat::Rgba16Float) as usize
        );
        let brdf_info = renderer.texture_info(baked_desc.brdf_lut.unwrap()).unwrap();
        assert_eq!(brdf_info.dimension, TextureDimension::D2);
        assert_eq!(brdf_info.format, TextureFormat::Rgba16Float);
        let brdf_bytes = renderer
            .texture_bytes(baked_desc.brdf_lut.unwrap())
            .unwrap();
        assert_eq!(
            brdf_bytes.len(),
            64 * 64 * texture_format_bytes_per_pixel(TextureFormat::Rgba16Float) as usize
        );
        assert!(brdf_bytes.iter().any(|byte| *byte != 0));

        assert!(matches!(
            renderer.create_environment(EnvironmentDesc {
                label: Some("bad".to_owned()),
                skybox: Some(make_handle(ResourceKind::Texture, 99, 1)),
                irradiance: None,
                prefiltered_specular: None,
                brdf_lut: None,
                intensity: 1.0,
                rotation: Quat::IDENTITY,
                diffuse_color: Color::WHITE,
                diffuse_intensity: 1.0,
                specular_color: Color::WHITE,
                specular_intensity: 1.0,
                texture: None,
                background_intensity: 1.0,
            }),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Texture,
                ..
            })
        ));
        assert!(matches!(
            renderer.bake_environment(
                texture,
                EnvironmentBakeDesc {
                    label: None,
                    resolution: 8,
                    mip_levels: 8,
                    intensity: 1.0,
                    rotation: Quat::IDENTITY,
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.bake_environment(
                texture,
                EnvironmentBakeDesc {
                    label: None,
                    resolution: 8,
                    mip_levels: 1,
                    intensity: f32::NAN,
                    rotation: Quat::IDENTITY,
                },
            ),
            Err(RendererError::Validation(_))
        ));
        let render_target_only = renderer
            .create_texture(TextureDesc {
                label: Some("not_sampled"),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.bake_environment(
                render_target_only,
                EnvironmentBakeDesc {
                    label: None,
                    resolution: 4,
                    mip_levels: 1,
                    intensity: 1.0,
                    rotation: Quat::IDENTITY,
                },
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_environment(EnvironmentDesc {
                label: Some("bad_usage".to_owned()),
                skybox: Some(render_target_only),
                irradiance: None,
                prefiltered_specular: None,
                brdf_lut: None,
                intensity: 1.0,
                rotation: Quat::IDENTITY,
                diffuse_color: Color::WHITE,
                diffuse_intensity: 1.0,
                specular_color: Color::WHITE,
                specular_intensity: 1.0,
                texture: None,
                background_intensity: 1.0,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn standard_material_desc_validates_pbr_values_and_texture_slots() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let default_material = renderer
            .create_standard_material(StandardMaterialDesc::default())
            .unwrap();
        let stored_default = renderer
            .materials
            .get(ResourceKind::Material, default_material)
            .and_then(|slot| slot.value.as_ref())
            .and_then(|material| material.standard.as_ref())
            .unwrap();
        assert_eq!(stored_default.domain, MaterialDomain::Opaque);
        assert_eq!(stored_default.base_color, Color::WHITE);
        assert_eq!(stored_default.metallic, 0.0);
        assert_eq!(stored_default.roughness, 0.5);
        assert_eq!(stored_default.emissive, Vec3::ZERO);
        assert_eq!(stored_default.alpha_mode, AlphaMode::Opaque);
        assert!(stored_default.receive_shadows);
        assert!(stored_default.cast_shadows);
        let builtin_template = renderer
            .materials
            .get(ResourceKind::Material, default_material)
            .and_then(|slot| slot.value.as_ref())
            .and_then(|material| material.template)
            .unwrap();
        let template_desc = renderer
            .material_templates
            .get(ResourceKind::MaterialTemplate, builtin_template)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        assert_eq!(template_desc.label.as_deref(), Some("builtin_standard_pbr"));
        assert_eq!(template_desc.passes, standard_material_builtin_passes());
        let shader_info = renderer.shader_info(template_desc.shader).unwrap();
        assert_eq!(shader_info.label.as_deref(), Some("builtin_standard_pbr"));

        let second_material = renderer
            .create_standard_material(StandardMaterialDesc {
                label: Some("second_standard".to_owned()),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let second_template = renderer
            .materials
            .get(ResourceKind::Material, second_material)
            .and_then(|slot| slot.value.as_ref())
            .and_then(|material| material.template)
            .unwrap();
        assert_eq!(second_template, builtin_template);

        let mut desc = StandardMaterialDesc::default();
        desc.roughness = f32::NAN;
        assert!(matches!(
            renderer.create_standard_material(desc.clone()),
            Err(RendererError::Validation(_))
        ));

        desc.roughness = 0.5;
        let color_target = renderer
            .create_texture(TextureDesc {
                label: None,
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        desc.base_color_texture = Some(color_target);
        assert!(matches!(
            renderer.create_standard_material(desc),
            Err(RendererError::Validation(_))
        ));

        let cube_texture = renderer
            .create_texture(TextureDesc {
                label: Some("wrong_standard_material_dimension"),
                dimension: TextureDimension::Cube,
                width: 1,
                height: 1,
                depth_or_layers: 6,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(cube_texture),
                ..StandardMaterialDesc::default()
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn standard_material_render_state_hash_tracks_shader_variants() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let default_material = renderer
            .create_standard_material(StandardMaterialDesc::default())
            .unwrap();
        let mut no_shadow_desc = StandardMaterialDesc::default();
        no_shadow_desc.receive_shadows = false;
        let no_shadow = renderer.create_standard_material(no_shadow_desc).unwrap();
        let double_sided = renderer
            .create_standard_material(StandardMaterialDesc {
                double_sided: true,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let alpha_cutout = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::AlphaCutout,
                alpha_mode: AlphaMode::Mask { cutoff: 0.5 },
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let sampled = renderer
            .create_texture(TextureDesc {
                label: Some("base_color"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let textured = renderer
            .create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(sampled),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let default_hash = renderer
            .material_render_state_hash(default_material)
            .unwrap();

        for material in [no_shadow, double_sided, alpha_cutout, textured] {
            assert_ne!(
                default_hash,
                renderer.material_render_state_hash(material).unwrap()
            );
        }
    }

    #[test]
    fn standard_material_generates_builtin_pipeline_keys() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = renderer
            .create_standard_material(StandardMaterialDesc::default())
            .unwrap();
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let object = renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                })
            })
            .unwrap();
        let view = ViewDesc {
            label: Some("standard_builtin_pipeline".to_owned()),
            scene,
            camera: test_camera(),
            target: RenderTarget::Headless {
                width: 16,
                height: 16,
                format: TextureFormat::Rgba8Unorm,
            },
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings::default(),
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };
        let scene_ref = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        let object_ref = scene_ref
            .objects
            .get(ResourceKind::Object, object)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        let items = renderer
            .object_draw_items(object, object_ref, &view)
            .unwrap();
        assert!(!items.is_empty());
        let builtin_template = renderer.builtin_standard_template.unwrap();
        let builtin_shader = renderer.builtin_standard_shader.unwrap();
        assert!(items.iter().all(|item| {
            item.pipeline_key.material_template == builtin_template
                && item.pipeline_key.shader == builtin_shader
        }));
    }

    #[test]
    fn sampler_desc_validates_lod_and_anisotropy() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let default_sampler = renderer.create_sampler(SamplerDesc::default()).unwrap();
        assert_eq!(
            renderer.resource_status(default_sampler),
            Some(ResourceStatus::Ready)
        );

        let mut desc = SamplerDesc {
            address_u: AddressMode::Repeat,
            address_v: AddressMode::Repeat,
            address_w: AddressMode::Repeat,
            lod_max: OrderedF32::new(16.0),
            ..SamplerDesc::default()
        };
        renderer.create_sampler(desc.clone()).unwrap();

        desc.anisotropy = 0;
        assert!(matches!(
            renderer.create_sampler(desc.clone()),
            Err(RendererError::Validation(_))
        ));
        desc.anisotropy = 1;
        desc.lod_min = OrderedF32::new(2.0);
        desc.lod_max = OrderedF32::new(1.0);
        assert!(matches!(
            renderer.create_sampler(desc),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn pipeline_warmup_validates_pipeline_keys() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("pipeline_shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("pipeline_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let key = PipelineKey {
            shader,
            material_template: template,
            vertex_layout_hash: 1,
            render_state_hash: 2,
            pass: RenderPhaseKind::ForwardOpaque,
            sample_count: 1,
            depth_format: DepthFormat::D32Float,
            color_format: TextureFormat::Rgba8Unorm,
            feature_bits: 0,
        };
        renderer
            .warm_up_pipelines(&[PipelineWarmupRequest { key }])
            .unwrap();
        renderer
            .warm_up_pipelines(&[PipelineWarmupRequest { key }])
            .unwrap();
        assert_eq!(
            renderer.pipeline_cache_stats(),
            PipelineCacheStats {
                total: 1,
                ready: 1,
                compiling: 0,
                failed: 0,
                cache_hits_this_frame: 1,
                cache_misses_this_frame: 1,
            }
        );

        let frame = renderer.begin_frame(FrameInput::default()).unwrap();
        drop(frame);
        assert_eq!(
            renderer.pipeline_cache_stats(),
            PipelineCacheStats {
                total: 1,
                ready: 1,
                compiling: 0,
                failed: 0,
                cache_hits_this_frame: 0,
                cache_misses_this_frame: 0,
            }
        );

        let mut invalid = key;
        invalid.sample_count = 0;
        assert!(matches!(
            renderer.warm_up_pipelines(&[PipelineWarmupRequest { key: invalid }]),
            Err(RendererError::Validation(_))
        ));
        invalid = key;
        invalid.shader = make_handle(ResourceKind::Shader, 99, 1);
        assert!(matches!(
            renderer.warm_up_pipelines(&[PipelineWarmupRequest { key: invalid }]),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Shader,
                ..
            })
        ));
    }

    #[test]
    fn shader_reflection_auto_extracts_wgsl_resource_bindings() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("pbr"),
                source: ShaderSource::Wgsl(
                    r#"
                    @group(0) @binding(0) var<uniform> camera: CameraUniform;
                    @group(1) @binding(0) var base_color: texture_2d<f32>;
                    @group(1) @binding(1) var base_sampler: sampler;
                    @group(1) @binding(2) var skybox: texture_cube<f32>;
                    @fragment fn fs_main() {}
                    "#,
                ),
                stages: ShaderStages::FRAGMENT,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: Some("fs_main"),
                    compute: None,
                },
                reflection: ShaderReflectionMode::Auto,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();

        let interface = renderer.shader_interface(shader).unwrap();
        assert_eq!(interface.resources.len(), 4);
        assert_eq!(interface.resources[0].name, "camera");
        assert_eq!(interface.resources[0].binding_class, BindingClass::Uniform);
        assert_eq!(interface.resources[0].ty, BindingType::Buffer);
        assert_eq!(interface.resources[1].name, "base_color");
        assert_eq!(
            interface.resources[1].ty,
            BindingType::Texture(TextureDimension::D2)
        );
        assert_eq!(interface.resources[2].binding_class, BindingClass::Sampler);
        assert_eq!(interface.resources[3].name, "skybox");
        assert_eq!(
            interface.resources[3].ty,
            BindingType::Texture(TextureDimension::Cube)
        );
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("duplicate_auto_bindings"),
                source: ShaderSource::Wgsl(
                    r#"
                    @group(0) @binding(0) var<uniform> camera: CameraUniform;
                    @group(0) @binding(1) var<uniform> camera: CameraUniform;
                    @fragment fn fs_main() {}
                    "#,
                ),
                stages: ShaderStages::FRAGMENT,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: Some("fs_main"),
                    compute: None,
                },
                reflection: ShaderReflectionMode::Auto,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn frame_submission_records_pipeline_cache_hits_and_misses() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let view = || ViewDesc {
            label: Some("pipeline_cache_view".to_owned()),
            scene,
            camera: test_camera(),
            target: RenderTarget::Headless {
                width: 16,
                height: 16,
                format: TextureFormat::Rgba8Unorm,
            },
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings::default(),
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame.render_view(view()).unwrap();
        frame.finish().unwrap();
        assert_eq!(renderer.pipeline_cache_stats().total, 1);
        assert_eq!(renderer.pipeline_cache_stats().ready, 1);
        assert_eq!(renderer.pipeline_cache_stats().cache_hits_this_frame, 0);
        assert_eq!(renderer.pipeline_cache_stats().cache_misses_this_frame, 1);

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame.render_view(view()).unwrap();
        frame.finish().unwrap();
        assert_eq!(renderer.pipeline_cache_stats().total, 1);
        assert_eq!(renderer.pipeline_cache_stats().ready, 1);
        assert_eq!(renderer.pipeline_cache_stats().cache_hits_this_frame, 1);
        assert_eq!(renderer.pipeline_cache_stats().cache_misses_this_frame, 0);
    }

    #[test]
    fn shader_reflection_accepts_explicit_interface_and_validates_entry_points() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let explicit = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "storage_items".to_owned(),
                binding_class: BindingClass::Storage,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer,
            }],
            push_constants: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..16,
            }],
            vertex_inputs: Vec::new(),
        };
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("compute"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs_main() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs_main"),
                },
                reflection: ShaderReflectionMode::Explicit(explicit.clone()),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();

        assert_eq!(renderer.shader_interface(shader).unwrap(), &explicit);
        let info = renderer.shader_info(shader).unwrap();
        assert_eq!(info.label.as_deref(), Some("compute"));
        assert_eq!(info.stages, ShaderStages::COMPUTE);
        assert_eq!(info.entry_points.compute.as_deref(), Some("cs_main"));
        assert_eq!(info.hot_reload_key, None);
        assert_eq!(info.interface, explicit);
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("bad"),
                source: ShaderSource::Wgsl("@vertex fn vs_main() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::ShaderCompile(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("empty_source"),
                source: ShaderSource::Wgsl(" "),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs_main"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::ShaderCompile(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("bad_interface"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs_main() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs_main"),
                },
                reflection: ShaderReflectionMode::Explicit(ShaderInterfaceDesc {
                    resources: vec![ShaderResourceBinding {
                        name: " ".to_owned(),
                        binding_class: BindingClass::Uniform,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer,
                    }],
                    push_constants: Vec::new(),
                    vertex_inputs: Vec::new(),
                }),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("duplicate_binding"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs_main() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs_main"),
                },
                reflection: ShaderReflectionMode::Explicit(ShaderInterfaceDesc {
                    resources: vec![
                        ShaderResourceBinding {
                            name: "camera".to_owned(),
                            binding_class: BindingClass::Uniform,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer,
                        },
                        ShaderResourceBinding {
                            name: "camera".to_owned(),
                            binding_class: BindingClass::Storage,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer,
                        },
                    ],
                    push_constants: Vec::new(),
                    vertex_inputs: Vec::new(),
                }),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("bad_binding_type"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs_main() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs_main"),
                },
                reflection: ShaderReflectionMode::Explicit(ShaderInterfaceDesc {
                    resources: vec![ShaderResourceBinding {
                        name: "albedo".to_owned(),
                        binding_class: BindingClass::Texture,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer,
                    }],
                    push_constants: Vec::new(),
                    vertex_inputs: Vec::new(),
                }),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("overlapping_push_constants"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs_main() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs_main"),
                },
                reflection: ShaderReflectionMode::Explicit(ShaderInterfaceDesc {
                    resources: Vec::new(),
                    push_constants: vec![
                        PushConstantRange {
                            stages: ShaderStages::COMPUTE,
                            range: 0..16,
                        },
                        PushConstantRange {
                            stages: ShaderStages::COMPUTE,
                            range: 8..24,
                        },
                    ],
                    vertex_inputs: Vec::new(),
                }),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("duplicate_vertex_semantic"),
                source: ShaderSource::Wgsl("@vertex fn vs_main() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs_main"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Explicit(ShaderInterfaceDesc {
                    resources: Vec::new(),
                    push_constants: Vec::new(),
                    vertex_inputs: vec![
                        VertexInputRequirement {
                            semantic: VertexSemantic::Position,
                            format: VertexFormat::Float32x3,
                        },
                        VertexInputRequirement {
                            semantic: VertexSemantic::Position,
                            format: VertexFormat::Float32x4,
                        },
                    ],
                }),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("duplicate_feature"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs_main() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs_main"),
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet {
                    flags: vec!["skinning".to_owned(), "skinning".to_owned()],
                },
                hot_reload_key: None,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn shader_hot_reload_updates_compatible_shader_and_invalidates_pipeline_cache() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let interface = ShaderInterfaceDesc {
            resources: vec![ShaderResourceBinding {
                name: "camera".to_owned(),
                binding_class: BindingClass::Uniform,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer,
            }],
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("reloadable"),
                source: ShaderSource::Wgsl("@fragment fn fs_main() {}"),
                stages: ShaderStages::FRAGMENT,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: Some("fs_main"),
                    compute: None,
                },
                reflection: ShaderReflectionMode::Explicit(interface.clone()),
                features: ShaderFeatureSet::default(),
                hot_reload_key: Some("materials/reloadable.wgsl".to_owned()),
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("reloadable_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let key = PipelineKey {
            shader,
            material_template: template,
            vertex_layout_hash: 1,
            render_state_hash: 2,
            pass: RenderPhaseKind::ForwardOpaque,
            sample_count: 1,
            depth_format: DepthFormat::D32Float,
            color_format: TextureFormat::Rgba8Unorm,
            feature_bits: 0,
        };
        renderer
            .warm_up_pipelines(&[PipelineWarmupRequest { key }])
            .unwrap();
        renderer
            .warm_up_pipelines(&[PipelineWarmupRequest { key }])
            .unwrap();
        assert_eq!(renderer.pipeline_cache_stats().total, 1);
        assert_eq!(renderer.pipeline_cache_stats().ready, 1);
        assert_eq!(renderer.pipeline_cache_stats().cache_hits_this_frame, 1);

        renderer
            .reload_shader_from_desc(
                shader,
                ShaderReloadDesc {
                    source: ShaderSource::Wgsl("@fragment fn fs_reloaded() {}"),
                    entry_points: ShaderEntryPoints {
                        vertex: None,
                        fragment: Some("fs_reloaded"),
                        compute: None,
                    },
                    reflection: ShaderReflectionMode::Explicit(interface.clone()),
                    features: ShaderFeatureSet::default(),
                },
            )
            .unwrap();
        let info = renderer.shader_info(shader).unwrap();
        assert_eq!(info.entry_points.fragment.as_deref(), Some("fs_reloaded"));
        assert_eq!(
            info.hot_reload_key.as_deref(),
            Some("materials/reloadable.wgsl")
        );
        assert_eq!(renderer.pipeline_cache_stats().total, 0);
        assert_eq!(renderer.pipeline_cache_stats().ready, 0);

        let incompatible = ShaderInterfaceDesc {
            resources: Vec::new(),
            push_constants: Vec::new(),
            vertex_inputs: Vec::new(),
        };
        assert!(matches!(
            renderer.reload_shader_from_desc(
                shader,
                ShaderReloadDesc {
                    source: ShaderSource::Wgsl("@fragment fn fs_bad() {}"),
                    entry_points: ShaderEntryPoints {
                        vertex: None,
                        fragment: Some("fs_bad"),
                        compute: None,
                    },
                    reflection: ShaderReflectionMode::Explicit(incompatible),
                    features: ShaderFeatureSet::default(),
                },
            ),
            Err(RendererError::ShaderCompile(_))
        ));
        assert_eq!(
            renderer
                .shader_info(shader)
                .unwrap()
                .entry_points
                .fragment
                .as_deref(),
            Some("fs_reloaded")
        );
    }

    #[test]
    fn shader_file_source_is_validated_and_reflected_for_wgsl() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("engine_renderer_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let shader_path = dir.join("file_source_reflect.wgsl");
        std::fs::write(
            &shader_path,
            "@group(0) @binding(0) var<uniform> camera: mat4x4<f32>;\n@vertex fn vs_main() {}",
        )
        .unwrap();

        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("file_source"),
                source: ShaderSource::File(shader_path.clone()),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs_main"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Auto,
                features: ShaderFeatureSet::default(),
                hot_reload_key: Some("file_source_reflect.wgsl".to_owned()),
            })
            .unwrap();
        assert_eq!(
            renderer.shader_interface(shader).unwrap().resources,
            vec![ShaderResourceBinding {
                name: "camera".to_owned(),
                binding_class: BindingClass::Uniform,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer,
            }]
        );

        let missing = dir.join("missing_shader.wgsl");
        assert!(matches!(
            renderer.create_shader(ShaderDesc {
                label: Some("missing_file"),
                source: ShaderSource::File(missing),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs_main"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            }),
            Err(RendererError::ShaderCompile(_))
        ));
    }

    #[test]
    fn shader_file_source_uses_path_as_default_hot_reload_key() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("engine_renderer_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let shader_path = dir.join("reload_from_file.wgsl");
        std::fs::write(&shader_path, "@vertex fn vs_main() {}").unwrap();

        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("file_reloadable"),
                source: ShaderSource::File(shader_path.clone()),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs_main"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        assert_eq!(
            renderer.shader_info(shader).unwrap().hot_reload_key,
            Some(shader_path.to_string_lossy().into_owned())
        );
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("file_reloadable_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let key = PipelineKey {
            shader,
            material_template: template,
            vertex_layout_hash: 1,
            render_state_hash: 2,
            pass: RenderPhaseKind::ForwardOpaque,
            sample_count: 1,
            depth_format: DepthFormat::D32Float,
            color_format: TextureFormat::Rgba8Unorm,
            feature_bits: 0,
        };
        renderer
            .warm_up_pipelines(&[PipelineWarmupRequest { key }])
            .unwrap();
        assert_eq!(renderer.pipeline_cache_stats().total, 1);

        std::fs::write(&shader_path, "@vertex fn vs_main() { }").unwrap();
        renderer.reload_shader(shader).unwrap();
        assert_eq!(renderer.pipeline_cache_stats().total, 0);
        assert_eq!(
            renderer.shader_info(shader).unwrap().hot_reload_key,
            Some(shader_path.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn capture_options_validate_backend_hooks() {
        let mut renderer = Renderer::new_headless(RendererConfig {
            ..RendererConfig::default()
        });
        assert!(matches!(
            renderer.capture_next_frame(CaptureOptions {
                backend: FrameCaptureBackend::Internal,
                open_after_capture: true,
                ..CaptureOptions::default()
            }),
            Err(RendererError::Validation(_))
        ));

        renderer
            .capture_next_frame(CaptureOptions {
                label: Some("external".to_owned()),
                backend: FrameCaptureBackend::ExternalDebugger,
                include_resource_dump: false,
                open_after_capture: true,
            })
            .unwrap();
        let stats = renderer
            .begin_frame(FrameInput::default())
            .unwrap()
            .finish()
            .unwrap();
        let capture = stats.capture.expect("capture data is attached");
        assert_eq!(capture.label.as_deref(), Some("external"));
        assert_eq!(capture.backend, FrameCaptureBackend::ExternalDebugger);
        assert_eq!(capture.status, FrameCaptureStatus::BackendUnavailable);
        assert!(!capture.include_resource_dump);
        assert!(capture.resource_dump.is_none());
        assert!(capture.open_after_capture);

        renderer
            .set_frame_capture_backend_available(FrameCaptureBackend::RenderDoc, true)
            .unwrap();
        renderer
            .capture_next_frame(CaptureOptions {
                label: Some("renderdoc".to_owned()),
                backend: FrameCaptureBackend::RenderDoc,
                include_resource_dump: false,
                open_after_capture: false,
            })
            .unwrap();
        let stats = renderer
            .begin_frame(FrameInput::default())
            .unwrap()
            .finish()
            .unwrap();
        let capture = stats.capture.expect("capture data is attached");
        assert_eq!(capture.label.as_deref(), Some("renderdoc"));
        assert_eq!(capture.backend, FrameCaptureBackend::RenderDoc);
        assert_eq!(capture.status, FrameCaptureStatus::BackendHookRequested);
        assert!(matches!(
            renderer.set_frame_capture_backend_available(FrameCaptureBackend::Internal, true),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn custom_material_parameters_are_schema_validated() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let compute_shader = renderer
            .create_shader(ShaderDesc {
                label: Some("compute_shader"),
                source: ShaderSource::Wgsl("@compute @workgroup_size(1) fn cs() {}"),
                stages: ShaderStages::COMPUTE,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: None,
                    compute: Some("cs"),
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.create_material_template(MaterialTemplateDesc {
                label: Some("bad_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema {
                    parameters: vec!["roughness".to_owned(), "roughness".to_owned()],
                },
                passes: MaterialPassFlags::FORWARD,
            }),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.create_material_template(MaterialTemplateDesc {
                label: Some("compute_template".to_owned()),
                shader: compute_shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_material_template(MaterialTemplateDesc {
                label: Some("no_passes".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::default(),
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_material_template(MaterialTemplateDesc {
                label: Some("transparent_without_pass".to_owned()),
                shader,
                domain: MaterialDomain::Transparent,
                render_state: RenderStateDesc { depth_write: false },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_material_template(MaterialTemplateDesc {
                label: Some("opaque_only_transparent_pass".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: false },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::TRANSPARENT,
            }),
            Err(RendererError::Validation(_))
        ));
        renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("transparent_template".to_owned()),
                shader,
                domain: MaterialDomain::Transparent,
                render_state: RenderStateDesc { depth_write: false },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::TRANSPARENT,
            })
            .unwrap();
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("albedo"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let cube_texture = renderer
            .create_texture(TextureDesc {
                label: Some("cube_albedo"),
                dimension: TextureDimension::Cube,
                width: 1,
                height: 1,
                depth_or_layers: 6,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let render_target_only_texture = renderer
            .create_texture(TextureDesc {
                label: Some("render_target_only"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        let sampler = renderer.create_sampler(SamplerDesc::default()).unwrap();
        let resource_shader = renderer
            .create_shader(ShaderDesc {
                label: Some("resource_shader"),
                source: ShaderSource::Wgsl("@fragment fn fs() {}"),
                stages: ShaderStages::FRAGMENT,
                entry_points: ShaderEntryPoints {
                    vertex: None,
                    fragment: Some("fs"),
                    compute: None,
                },
                reflection: ShaderReflectionMode::Explicit(ShaderInterfaceDesc {
                    resources: vec![
                        ShaderResourceBinding {
                            name: "base_color".to_owned(),
                            binding_class: BindingClass::Texture,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture(TextureDimension::D2),
                        },
                        ShaderResourceBinding {
                            name: "base_sampler".to_owned(),
                            binding_class: BindingClass::Sampler,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler,
                        },
                        ShaderResourceBinding {
                            name: "camera".to_owned(),
                            binding_class: BindingClass::Uniform,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer,
                        },
                    ],
                    push_constants: Vec::new(),
                    vertex_inputs: Vec::new(),
                }),
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let resource_template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("resource_template".to_owned()),
                shader: resource_shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema {
                    parameters: vec![
                        "base_color".to_owned(),
                        "base_sampler".to_owned(),
                        "camera".to_owned(),
                    ],
                },
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        assert!(matches!(
            renderer.create_material(MaterialDesc {
                label: Some("non_sampled_texture_material".to_owned()),
                template: resource_template,
                parameters: vec![
                    MaterialParameter {
                        name: "base_color".to_owned(),
                        value: MaterialParameterValue::Texture(render_target_only_texture),
                    },
                    MaterialParameter {
                        name: "base_sampler".to_owned(),
                        value: MaterialParameterValue::Sampler(sampler),
                    },
                    MaterialParameter {
                        name: "camera".to_owned(),
                        value: MaterialParameterValue::Bytes(vec![0; 64]),
                    },
                ],
                overrides: MaterialOverrides::default(),
            }),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.create_material(MaterialDesc {
                label: Some("bad_resource_material".to_owned()),
                template: resource_template,
                parameters: vec![MaterialParameter {
                    name: "base_color".to_owned(),
                    value: MaterialParameterValue::F32(1.0),
                }],
                overrides: MaterialOverrides::default(),
            }),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.create_material(MaterialDesc {
                label: Some("wrong_texture_dimension_material".to_owned()),
                template: resource_template,
                parameters: vec![
                    MaterialParameter {
                        name: "base_color".to_owned(),
                        value: MaterialParameterValue::Texture(cube_texture),
                    },
                    MaterialParameter {
                        name: "base_sampler".to_owned(),
                        value: MaterialParameterValue::Sampler(sampler),
                    },
                    MaterialParameter {
                        name: "camera".to_owned(),
                        value: MaterialParameterValue::Bytes(vec![0; 64]),
                    },
                ],
                overrides: MaterialOverrides::default(),
            }),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        let resource_material = renderer
            .create_material(MaterialDesc {
                label: Some("resource_material".to_owned()),
                template: resource_template,
                parameters: vec![
                    MaterialParameter {
                        name: "base_color".to_owned(),
                        value: MaterialParameterValue::Texture(texture),
                    },
                    MaterialParameter {
                        name: "base_sampler".to_owned(),
                        value: MaterialParameterValue::Sampler(sampler),
                    },
                    MaterialParameter {
                        name: "camera".to_owned(),
                        value: MaterialParameterValue::Bytes(vec![0; 64]),
                    },
                ],
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        assert!(matches!(
            renderer.update_material(
                resource_material,
                MaterialUpdate::SetSampler("base_color".to_owned(), Some(sampler)),
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.update_material(
                resource_material,
                MaterialUpdate::SetTexture(
                    "base_color".to_owned(),
                    Some(render_target_only_texture)
                ),
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.update_material(
                resource_material,
                MaterialUpdate::SetTexture("base_color".to_owned(), Some(cube_texture)),
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.update_material(
                resource_material,
                MaterialUpdate::SetVec4("camera".to_owned(), Vec4::ONE),
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema {
                    parameters: vec!["roughness".to_owned(), "albedo".to_owned()],
                },
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: Some("material".to_owned()),
                template,
                parameters: vec![
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.45),
                    },
                    MaterialParameter {
                        name: "albedo".to_owned(),
                        value: MaterialParameterValue::Texture(texture),
                    },
                ],
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        assert!(matches!(
            renderer.create_material(MaterialDesc {
                label: Some("empty_parameter_name".to_owned()),
                template,
                parameters: vec![MaterialParameter {
                    name: " ".to_owned(),
                    value: MaterialParameterValue::F32(0.5),
                }],
                overrides: MaterialOverrides::default(),
            }),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.create_material(MaterialDesc {
                label: Some("duplicate_parameter_name".to_owned()),
                template,
                parameters: vec![
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.25),
                    },
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.75),
                    },
                ],
                overrides: MaterialOverrides::default(),
            }),
            Err(RendererError::MaterialParameterMismatch(_))
        ));

        assert_eq!(
            renderer.material_parameter(material, "roughness"),
            Some(&MaterialParameterValue::F32(0.45))
        );
        renderer
            .update_material(
                material,
                MaterialUpdate::SetFloat("roughness".to_owned(), 0.65),
            )
            .unwrap();
        assert_eq!(
            renderer.material_parameter(material, "roughness"),
            Some(&MaterialParameterValue::F32(0.65))
        );
        let albedo_param = renderer.intern_material_param("albedo");
        assert_eq!(albedo_param, renderer.intern_material_param("albedo"));
        renderer
            .update_material_fast(material, albedo_param, MaterialValue::Texture(texture))
            .unwrap();
        assert_eq!(
            renderer.material_parameter(material, "albedo"),
            Some(&MaterialParameterValue::Texture(texture))
        );
        renderer
            .update_material(
                material,
                MaterialUpdate::SetTexture("albedo".to_owned(), None),
            )
            .unwrap();
        assert_eq!(renderer.material_parameter(material, "albedo"), None);
        renderer
            .update_material(
                material,
                MaterialUpdate::ReplaceAll(vec![MaterialParameter {
                    name: "roughness".to_owned(),
                    value: MaterialParameterValue::F32(0.25),
                }]),
            )
            .unwrap();
        assert_eq!(
            renderer.material_parameter(material, "roughness"),
            Some(&MaterialParameterValue::F32(0.25))
        );
        assert!(matches!(
            renderer.update_material_parameters(
                material,
                &[MaterialParameter {
                    name: "metallic".to_owned(),
                    value: MaterialParameterValue::F32(1.0),
                }],
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.update_material_parameters(
                material,
                &[
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.1),
                    },
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.2),
                    },
                ],
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.update_material(
                material,
                MaterialUpdate::ReplaceAll(vec![
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.1),
                    },
                    MaterialParameter {
                        name: "roughness".to_owned(),
                        value: MaterialParameterValue::F32(0.2),
                    },
                ]),
            ),
            Err(RendererError::MaterialParameterMismatch(_))
        ));
        assert!(matches!(
            renderer.update_material_parameters(
                material,
                &[MaterialParameter {
                    name: "albedo".to_owned(),
                    value: MaterialParameterValue::Texture(make_handle(
                        ResourceKind::Texture,
                        99,
                        1
                    )),
                }],
            ),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Texture,
                ..
            })
        ));
    }

    #[test]
    fn material_render_state_overrides_affect_batch_keys() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("state_shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("state_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let default_material = renderer
            .create_material(MaterialDesc {
                label: Some("default_state_material".to_owned()),
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        let overridden_material = renderer
            .create_material(MaterialDesc {
                label: Some("overridden_state_material".to_owned()),
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides {
                    render_state: Some(RenderStateDesc { depth_write: false }),
                },
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("state_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let view = ViewDesc {
            label: None,
            scene,
            camera: test_camera(),
            target: RenderTarget::MainSurface,
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings::default(),
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };
        let default_object = RenderObjectDesc {
            mesh,
            materials: vec![default_material],
            ..RenderObjectDesc::default()
        };
        let overridden_object = RenderObjectDesc {
            mesh,
            materials: vec![overridden_material],
            ..RenderObjectDesc::default()
        };

        let default_key = renderer
            .object_batch_keys(
                make_handle(ResourceKind::Object, 0, 1),
                &default_object,
                &view,
            )
            .unwrap()
            .remove(0);
        let overridden_key = renderer
            .object_batch_keys(
                make_handle(ResourceKind::Object, 1, 1),
                &overridden_object,
                &view,
            )
            .unwrap()
            .remove(0);

        assert_ne!(default_key.4, overridden_key.4);
        assert_eq!(
            default_key.4,
            render_state_hash(&RenderStateDesc { depth_write: true })
        );
        assert_eq!(
            overridden_key.4,
            render_state_hash(&RenderStateDesc { depth_write: false })
        );
    }

    #[test]
    fn material_pass_flags_select_render_phases() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("phase_shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("phase_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc::default(),
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD
                    | MaterialPassFlags::MOTION
                    | MaterialPassFlags::PICKING,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: Some("phase_material".to_owned()),
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();

        assert!(renderer
            .material_supports_phase(material, RenderPhaseKind::ForwardOpaque)
            .unwrap());
        assert!(renderer
            .material_supports_phase(material, RenderPhaseKind::MotionVector)
            .unwrap());
        assert!(renderer
            .material_supports_phase(material, RenderPhaseKind::Picking)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(material, RenderPhaseKind::Custom(2))
            .unwrap());
        assert!(!renderer
            .material_supports_phase(material, RenderPhaseKind::GBuffer)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(material, RenderPhaseKind::ForwardTransparent)
            .unwrap());

        let opaque = test_standard_material(&mut renderer);
        assert!(renderer
            .material_supports_phase(opaque, RenderPhaseKind::DepthPrepass)
            .unwrap());
        assert!(renderer
            .material_supports_phase(opaque, RenderPhaseKind::Shadow)
            .unwrap());
        assert!(renderer
            .material_supports_phase(opaque, RenderPhaseKind::GBuffer)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(opaque, RenderPhaseKind::ForwardTransparent)
            .unwrap());

        let transparent = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Transparent,
                alpha_mode: AlphaMode::Blend,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        assert!(renderer
            .material_supports_phase(transparent, RenderPhaseKind::ForwardTransparent)
            .unwrap());
        assert!(renderer
            .material_supports_phase(transparent, RenderPhaseKind::Picking)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(transparent, RenderPhaseKind::DepthPrepass)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(transparent, RenderPhaseKind::Shadow)
            .unwrap());

        let blend_alpha = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Opaque,
                alpha_mode: AlphaMode::Blend,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        assert!(renderer
            .material_supports_phase(blend_alpha, RenderPhaseKind::ForwardTransparent)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(blend_alpha, RenderPhaseKind::DepthPrepass)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(blend_alpha, RenderPhaseKind::GBuffer)
            .unwrap());

        let alpha_cutout = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::AlphaCutout,
                alpha_mode: AlphaMode::Mask { cutoff: 0.5 },
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        assert!(renderer
            .material_supports_phase(alpha_cutout, RenderPhaseKind::DepthPrepass)
            .unwrap());
        assert!(renderer
            .material_supports_phase(alpha_cutout, RenderPhaseKind::GBuffer)
            .unwrap());
        assert!(!renderer
            .material_supports_phase(alpha_cutout, RenderPhaseKind::ForwardTransparent)
            .unwrap());

        let custom_pass = MaterialPassFlags::custom(2).unwrap();
        let custom_template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("custom_phase_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc::default(),
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD | custom_pass,
            })
            .unwrap();
        let custom_material = renderer
            .create_material(MaterialDesc {
                label: Some("custom_phase_material".to_owned()),
                template: custom_template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        assert!(renderer
            .material_supports_phase(custom_material, RenderPhaseKind::ForwardOpaque)
            .unwrap());
        assert!(renderer
            .material_supports_phase(custom_material, RenderPhaseKind::Custom(2))
            .unwrap());
        assert!(!renderer
            .material_supports_phase(custom_material, RenderPhaseKind::Custom(3))
            .unwrap());
        assert!(MaterialPassFlags::custom(MaterialPassFlags::CUSTOM_PHASE_COUNT).is_err());

        for domain in [MaterialDomain::Sky, MaterialDomain::PostProcess] {
            let material = renderer
                .create_standard_material(StandardMaterialDesc {
                    domain,
                    ..StandardMaterialDesc::default()
                })
                .unwrap();
            for phase in [
                RenderPhaseKind::DepthPrepass,
                RenderPhaseKind::Shadow,
                RenderPhaseKind::GBuffer,
                RenderPhaseKind::ForwardOpaque,
                RenderPhaseKind::ForwardTransparent,
                RenderPhaseKind::MotionVector,
                RenderPhaseKind::Picking,
            ] {
                assert!(!renderer.material_supports_phase(material, phase).unwrap());
            }
        }
    }

    #[test]
    fn standard_material_batch_keys_track_render_phase() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let opaque = test_standard_material(&mut renderer);
        let transparent = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Transparent,
                alpha_mode: AlphaMode::Blend,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let blend_alpha = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Opaque,
                alpha_mode: AlphaMode::Blend,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let object_handle = make_handle(ResourceKind::Object, 7, 1);
        let mut view = ViewDesc {
            label: None,
            scene: make_handle(ResourceKind::Scene, 0, 1),
            camera: test_camera(),
            target: RenderTarget::Headless {
                width: 16,
                height: 16,
                format: TextureFormat::Rgba8Unorm,
            },
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings::default(),
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };
        let mut object = RenderObjectDesc {
            mesh,
            materials: vec![opaque],
            ..RenderObjectDesc::default()
        };

        let forward_key = renderer
            .object_batch_keys(object_handle, &object, &view)
            .unwrap()
            .remove(0);
        assert_eq!(
            forward_key.2,
            render_phase_sort_rank(RenderPhaseKind::ForwardOpaque)
        );

        view.render_path = RenderPath::Deferred;
        let deferred_key = renderer
            .object_batch_keys(object_handle, &object, &view)
            .unwrap()
            .remove(0);
        assert_eq!(
            deferred_key.2,
            render_phase_sort_rank(RenderPhaseKind::GBuffer)
        );

        view.render_path = RenderPath::Forward;
        object.materials = vec![transparent];
        let transparent_key = renderer
            .object_batch_keys(object_handle, &object, &view)
            .unwrap()
            .remove(0);
        assert_eq!(
            transparent_key.2,
            render_phase_sort_rank(RenderPhaseKind::ForwardTransparent)
        );

        object.materials = vec![blend_alpha];
        let blend_alpha_key = renderer
            .object_batch_keys(object_handle, &object, &view)
            .unwrap()
            .remove(0);
        assert_eq!(
            blend_alpha_key.2,
            render_phase_sort_rank(RenderPhaseKind::ForwardTransparent)
        );
    }

    #[test]
    fn custom_material_generates_phase_draw_items() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("draw_item_shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("draw_item_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD
                    | MaterialPassFlags::TRANSPARENT
                    | MaterialPassFlags::PICKING,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: Some("draw_item_material".to_owned()),
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let object_handle = make_handle(ResourceKind::Object, 42, 1);
        let object = RenderObjectDesc {
            mesh,
            materials: vec![material],
            ..RenderObjectDesc::default()
        };
        let view = ViewDesc {
            label: None,
            scene: make_handle(ResourceKind::Scene, 0, 1),
            camera: test_camera(),
            target: RenderTarget::Headless {
                width: 16,
                height: 16,
                format: TextureFormat::Rgba8Unorm,
            },
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings::default(),
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };

        let items = renderer
            .object_draw_items(object_handle, &object, &view)
            .unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].object, object_handle);
        assert_eq!(items[0].mesh, mesh);
        assert_eq!(items[0].submesh_index, 0);
        assert_eq!(items[0].material, material);
        assert_eq!(items[0].pipeline_key.shader, shader);
        assert_eq!(items[0].pipeline_key.material_template, template);
        assert_eq!(items[0].pipeline_key.pass, RenderPhaseKind::ForwardOpaque);
        assert_eq!(items[0].pipeline_key.render_state_hash, 1);
        assert_eq!(items[0].instance_range, 0..1);
        assert_eq!(items[1].object, object_handle);
        assert_eq!(items[1].mesh, mesh);
        assert_eq!(items[1].submesh_index, 0);
        assert_eq!(items[1].material, material);
        assert_eq!(items[1].pipeline_key.shader, shader);
        assert_eq!(items[1].pipeline_key.material_template, template);
        assert_eq!(
            items[1].pipeline_key.pass,
            RenderPhaseKind::ForwardTransparent
        );
        assert_eq!(items[1].pipeline_key.render_state_hash, 1);
        assert_eq!(items[1].instance_range, 0..1);
    }

    #[test]
    fn scene_editor_supports_retained_object_and_light_updates() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mesh_a = make_handle(ResourceKind::Mesh, 0, 1);
        let mesh_b = make_handle(ResourceKind::Mesh, 1, 1);
        let material = make_handle(ResourceKind::Material, 0, 1);
        let bounds = Bounds3::new(Vec3::ZERO, Vec3::ONE);
        let skeleton = renderer
            .create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("skeleton"),
                joint_matrices: &[IDENTITY_MAT4],
                inverse_bind_matrices: Some(&[IDENTITY_MAT4]),
                usage: AnimationDataUsage::Dynamic,
            })
            .unwrap();
        let morph_weights = renderer
            .create_morph_weights(MorphWeightsDesc {
                label: Some("morphs"),
                weights: &[0.0, 0.5],
            })
            .unwrap();
        renderer
            .update_skeleton_joints(skeleton, &[IDENTITY_MAT4])
            .unwrap();
        renderer
            .update_morph_weights(morph_weights, &[1.0, 0.25])
            .unwrap();
        let mut object = None;
        let mut light = None;
        renderer
            .edit_scene(scene, |scene| {
                let spawned = scene.spawn(RenderObjectDesc {
                    mesh: mesh_a,
                    ..RenderObjectDesc::default()
                });
                scene.set_mesh(spawned, mesh_b).unwrap();
                scene.set_material(spawned, 0, material).unwrap();
                scene
                    .set_previous_transform(spawned, IDENTITY_MAT4)
                    .unwrap();
                scene.clear_previous_transform(spawned).unwrap();
                scene
                    .set_visibility(spawned, VisibilityFlags::SHADOW)
                    .unwrap();
                scene
                    .set_flags(spawned, ObjectFlags::DYNAMIC | ObjectFlags::NO_BATCH)
                    .unwrap();
                scene.set_layer(spawned, RenderLayer(3)).unwrap();
                scene.set_bounds(spawned, bounds).unwrap();
                scene.set_skeleton(spawned, Some(skeleton)).unwrap();
                scene
                    .set_morph_weights(spawned, Some(morph_weights))
                    .unwrap();
                let directional = scene
                    .add_light(LightDesc::Directional(DirectionalLightDesc {
                        label: Some("sun".to_owned()),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        illuminance_lux: 80_000.0,
                        shadow: None,
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
                scene
                    .update_light(
                        directional,
                        LightUpdate {
                            desc: LightDesc::Area(AreaLightDesc {
                                label: Some("area".to_owned()),
                                position: Vec3::ONE,
                                direction: Vec3::new(0.0, -1.0, 0.0),
                                color: Color::WHITE,
                                intensity: 3.0,
                                range: 4.0,
                                shape: AreaLightShape::Rectangle {
                                    width: 2.0,
                                    height: 1.0,
                                },
                                layer_mask: RenderLayerMask::all(),
                            }),
                        },
                    )
                    .unwrap();
                object = Some(spawned);
                light = Some(directional);
            })
            .unwrap();

        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        let object = object.unwrap();
        let object_desc = stored
            .objects
            .get(ResourceKind::Object, object)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        assert_eq!(object_desc.mesh, mesh_b);
        assert_eq!(object_desc.materials, vec![material]);
        assert_eq!(object_desc.previous_transform, None);
        assert_eq!(object_desc.visibility, VisibilityFlags::SHADOW);
        assert_eq!(
            object_desc.flags,
            ObjectFlags::DYNAMIC | ObjectFlags::NO_BATCH
        );
        assert_eq!(object_desc.layer, RenderLayer(3));
        assert_eq!(object_desc.bounds, Some(bounds));
        assert_eq!(object_desc.skeleton, Some(skeleton));
        assert_eq!(object_desc.morph_weights, Some(morph_weights));
        assert_eq!(
            renderer
                .skeleton_instances
                .get(ResourceKind::SkeletonInstance, skeleton)
                .and_then(|slot| slot.value.as_ref())
                .unwrap()
                .joint_matrices
                .len(),
            1
        );
        let skeleton_info = renderer.skeleton_instance_info(skeleton).unwrap();
        assert_eq!(skeleton_info.inverse_bind_count, 1);
        assert_eq!(skeleton_info.usage, AnimationDataUsage::Dynamic);
        assert_eq!(
            renderer
                .morph_weights
                .get(ResourceKind::MorphWeights, morph_weights)
                .and_then(|slot| slot.value.as_ref())
                .unwrap()
                .weights,
            vec![1.0, 0.25]
        );
        let light = light.unwrap();
        assert!(matches!(
            stored
                .lights
                .get(ResourceKind::Light, light)
                .and_then(|slot| slot.value.as_ref()),
            Some(LightDesc::Area(AreaLightDesc {
                intensity: 3.0,
                shape: AreaLightShape::Rectangle {
                    width: 2.0,
                    height: 1.0
                },
                ..
            }))
        ));

        renderer
            .edit_scene(scene, |scene| {
                scene.remove_light(light).unwrap();
            })
            .unwrap();
        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        assert!(stored.lights.get(ResourceKind::Light, light).is_none());
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn legacy_lighting_falls_back_area_lights_to_point_lights() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene
                    .add_light(LightDesc::Area(AreaLightDesc {
                        label: Some("area".to_owned()),
                        position: Vec3::new(1.0, 2.0, 3.0),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        intensity: 3.0,
                        range: 4.0,
                        shape: AreaLightShape::Rectangle {
                            width: 2.0,
                            height: 1.0,
                        },
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
            })
            .unwrap();

        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        let lighting = legacy_lighting(stored, &HashMap::new());
        let points = lighting.point_lights();

        assert_eq!(points.len(), 1);
        assert_eq!(
            points[0],
            engine_render::PointLight::new([1.0, 2.0, 3.0], [1.0, 1.0, 1.0], 6.0, 4.0)
        );

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("legacy_area_lights".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();

        let stats = frame.finish().unwrap();
        assert_eq!(
            stats.area_light_outputs,
            vec![FrameAreaLightOutput {
                view_label: Some("legacy_area_lights".to_owned()),
                area_lights: 1,
            }]
        );
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn legacy_lighting_falls_back_custom_lights_to_point_lights() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene
                    .add_light(LightDesc::Custom(CustomLightDesc {
                        label: Some("custom".to_owned()),
                        type_id: 7,
                        position: Vec3::new(-2.0, 1.0, 0.5),
                        color: Color::rgba(0.2, 0.4, 0.8, 1.0),
                        intensity: 2.5,
                        range: 5.0,
                        layer_mask: RenderLayerMask::all(),
                        parameters: vec![0.25, 0.5, 0.75],
                    }))
                    .unwrap();
            })
            .unwrap();

        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        let lighting = legacy_lighting(stored, &HashMap::new());
        let points = lighting.point_lights();

        assert_eq!(points.len(), 1);
        assert_eq!(
            points[0],
            engine_render::PointLight::new(
                [-2.0, 1.0, 0.5],
                [0.2, 0.4, 0.8],
                2.5,
                5.0
            )
        );
    }

    #[test]
    fn area_light_outputs_filter_by_view_layers() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene
                    .add_light(LightDesc::Area(AreaLightDesc {
                        label: Some("area_visible".to_owned()),
                        position: Vec3::new(0.0, 1.0, 2.0),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        intensity: 1.0,
                        range: 2.0,
                        shape: AreaLightShape::Rectangle {
                            width: 1.0,
                            height: 1.0,
                        },
                        layer_mask: RenderLayerMask::single(RenderLayer(2)),
                    }))
                    .unwrap();
                scene
                    .add_light(LightDesc::Area(AreaLightDesc {
                        label: Some("area_hidden".to_owned()),
                        position: Vec3::new(0.0, 2.0, 3.0),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        intensity: 1.0,
                        range: 2.0,
                        shape: AreaLightShape::Rectangle {
                            width: 1.0,
                            height: 1.0,
                        },
                        layer_mask: RenderLayerMask::single(RenderLayer(4)),
                    }))
                    .unwrap();
            })
            .unwrap();

        let mut visible = renderer.begin_frame(FrameInput::default()).unwrap();
        visible
            .render_view(ViewDesc {
                label: Some("visible_layer".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::single(RenderLayer(2)),
                graph_extensions: Vec::new(),
            })
            .unwrap();

        let visible_stats = visible.finish().unwrap();
        assert_eq!(
            visible_stats.area_light_outputs,
            vec![FrameAreaLightOutput {
                view_label: Some("visible_layer".to_owned()),
                area_lights: 1,
            }]
        );

        let mut hidden = renderer.begin_frame(FrameInput::default()).unwrap();
        hidden
            .render_view(ViewDesc {
                label: Some("hidden_layer".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::single(RenderLayer(6)),
                graph_extensions: Vec::new(),
            })
            .unwrap();

        let hidden_stats = hidden.finish().unwrap();
        assert!(hidden_stats.area_light_outputs.is_empty());
    }

    #[test]
    fn light_updates_validate_physical_and_shadow_values() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("lights".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                let light = scene
                    .add_light(LightDesc::Point(PointLightDesc {
                        label: Some("point".to_owned()),
                        position: Vec3::ZERO,
                        color: Color::WHITE,
                        intensity_lumen: 1.0,
                        radius: 1.0,
                        shadow: None,
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
                assert!(matches!(
                    scene.update_light(
                        light,
                        LightUpdate {
                            desc: LightDesc::Spot(SpotLightDesc {
                                label: Some("bad_spot".to_owned()),
                                position: Vec3::ZERO,
                                direction: Vec3::new(0.0, -1.0, 0.0),
                                color: Color::WHITE,
                                intensity_lumen: 1.0,
                                range: 10.0,
                                inner_angle: 1.0,
                                outer_angle: 0.5,
                                shadow: None,
                                layer_mask: RenderLayerMask::all(),
                            }),
                        },
                    ),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    scene.update_light(
                        light,
                        LightUpdate {
                            desc: LightDesc::Directional(DirectionalLightDesc {
                                label: Some("bad_direction".to_owned()),
                                direction: Vec3::ZERO,
                                color: Color::WHITE,
                                illuminance_lux: 1.0,
                                shadow: None,
                                layer_mask: RenderLayerMask::all(),
                            }),
                        },
                    ),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    scene.update_light(
                        light,
                        LightUpdate {
                            desc: LightDesc::Directional(DirectionalLightDesc {
                                label: Some("bad_shadow".to_owned()),
                                direction: Vec3::new(0.0, -1.0, 0.0),
                                color: Color::WHITE,
                                illuminance_lux: 1.0,
                                shadow: Some(DirectionalShadowDesc {
                                    resolution: 0,
                                    cascades: 1,
                                    max_distance: 10.0,
                                    split_lambda: 0.5,
                                    filter: ShadowFilter::Pcf { taps: 1 },
                                    bias: ShadowBias {
                                        constant: 0.0,
                                        slope: 0.0,
                                        normal: 0.0,
                                    },
                                }),
                                layer_mask: RenderLayerMask::all(),
                            }),
                        },
                    ),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    scene.add_light(LightDesc::Area(AreaLightDesc {
                        label: Some("bad_area".to_owned()),
                        position: Vec3::ZERO,
                        direction: Vec3::ZERO,
                        color: Color::WHITE,
                        intensity: 1.0,
                        range: 1.0,
                        shape: AreaLightShape::Disk { radius: 1.0 },
                        layer_mask: RenderLayerMask::all(),
                    })),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    scene.add_light(LightDesc::Custom(CustomLightDesc {
                        label: Some("bad_custom".to_owned()),
                        type_id: 7,
                        position: Vec3::ZERO,
                        color: Color::WHITE,
                        intensity: 1.0,
                        range: 1.0,
                        layer_mask: RenderLayerMask::all(),
                        parameters: vec![0.0; 17],
                    })),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    scene.add_light(LightDesc::Custom(CustomLightDesc {
                        label: Some("bad_custom_nan".to_owned()),
                        type_id: 8,
                        position: Vec3::ZERO,
                        color: Color::WHITE,
                        intensity: 1.0,
                        range: 1.0,
                        layer_mask: RenderLayerMask::all(),
                        parameters: vec![f32::NAN],
                    })),
                    Err(RendererError::Validation(_))
                ));
                scene.set_environment(None).unwrap();
            })
            .unwrap();
    }

    #[test]
    fn resource_priority_covers_public_facade_resources() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = renderer
            .create_mesh(MeshDesc {
                label: Some("triangle"),
                vertex_layout: VertexLayout::default(),
                vertices: VertexData::Interleaved(&[0_u8; 36]),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::default(),
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            })
            .unwrap();
        let buffer = renderer
            .create_buffer(BufferDesc {
                label: Some("camera_buffer"),
                size: 64,
                usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
                initial_data: None,
            })
            .unwrap();
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("albedo"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let sampler = renderer
            .create_sampler(SamplerDesc {
                address_u: AddressMode::ClampToEdge,
                address_v: AddressMode::ClampToEdge,
                address_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mip_filter: FilterMode::Linear,
                compare: None,
                anisotropy: 1,
                lod_min: OrderedF32::new(0.0),
                lod_max: OrderedF32::new(16.0),
            })
            .unwrap();
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: None,
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        let environment = renderer
            .create_environment(EnvironmentDesc {
                label: Some("environment".to_owned()),
                skybox: Some(texture),
                irradiance: Some(texture),
                prefiltered_specular: Some(texture),
                brdf_lut: None,
                intensity: 0.5,
                rotation: Quat::IDENTITY,
                diffuse_color: Color::WHITE,
                diffuse_intensity: 0.25,
                specular_color: Color::WHITE,
                specular_intensity: 0.5,
                texture: Some(texture),
                background_intensity: 0.1,
            })
            .unwrap();
        let target_color = renderer
            .create_texture(TextureDesc {
                label: Some("target_color"),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET | TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let render_target = renderer
            .create_render_target(RenderTargetDesc {
                label: Some("offscreen".to_owned()),
                color: target_color,
                depth: None,
                width: 4,
                height: 4,
                samples: 1,
            })
            .unwrap();
        let camera = renderer.create_camera(test_camera()).unwrap();
        let graph_extension = renderer.register_graph_extension(NoopExtension).unwrap();
        let skeleton = renderer
            .create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("skeleton"),
                joint_matrices: &[IDENTITY_MAT4],
                inverse_bind_matrices: None,
                usage: AnimationDataUsage::Dynamic,
            })
            .unwrap();
        let morph_weights = renderer
            .create_morph_weights(MorphWeightsDesc {
                label: Some("morphs"),
                weights: &[0.0, 1.0],
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let view = {
            let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
            frame
                .render_view(ViewDesc {
                    label: Some("view".to_owned()),
                    scene,
                    camera: CameraDesc {
                        label: Some("camera".to_owned()),
                        transform: IDENTITY_MAT4,
                        projection: Projection::Orthographic {
                            width: 1.0,
                            height: 1.0,
                            near: 0.0,
                            far: 10.0,
                            reverse_z: false,
                        },
                        exposure: Exposure::Manual(1.0),
                        clear: ClearOptions::ColorDepth(Color::BLACK),
                        viewport: None,
                        scissor: None,
                        jitter: None,
                        previous_view_proj: None,
                        flags: CameraFlags::MAIN,
                    },
                    target: RenderTarget::MainSurface,
                    render_path: RenderPath::Forward,
                    quality: ViewQualitySettings {
                        hdr: false,
                        bloom: false,
                        taa: false,
                        fxaa: false,
                        ssao: false,
                        ssr: false,
                        depth_of_field: false,
                        motion_blur: false,
                        variable_rate_shading: false,
                        bindless_textures: false,
                        mesh_shaders: false,
                        virtual_texturing: false,
                        ray_tracing: false,
                        color_grading: ColorGradingMode::None,
                    },
                    layers: RenderLayerMask::all(),
                    graph_extensions: Vec::new(),
                })
                .unwrap()
        };

        renderer
            .set_resource_priority(mesh, ResidencyPriority::Critical)
            .unwrap();
        renderer
            .set_resource_priority(buffer, ResidencyPriority::High)
            .unwrap();
        renderer
            .set_resource_priority(texture, ResidencyPriority::High)
            .unwrap();
        renderer
            .set_resource_priority(sampler, ResidencyPriority::Low)
            .unwrap();
        renderer
            .set_resource_priority(shader, ResidencyPriority::Streamable)
            .unwrap();
        renderer
            .set_resource_priority(template, ResidencyPriority::High)
            .unwrap();
        renderer
            .set_resource_priority(material, ResidencyPriority::Low)
            .unwrap();
        renderer
            .set_resource_priority(environment, ResidencyPriority::High)
            .unwrap();
        renderer
            .set_resource_priority(render_target, ResidencyPriority::High)
            .unwrap();
        renderer
            .set_resource_priority(camera, ResidencyPriority::Critical)
            .unwrap();
        renderer
            .set_resource_priority(graph_extension, ResidencyPriority::Streamable)
            .unwrap();
        renderer
            .set_resource_priority(skeleton, ResidencyPriority::High)
            .unwrap();
        renderer
            .set_resource_priority(morph_weights, ResidencyPriority::Low)
            .unwrap();
        renderer
            .set_resource_priority(scene, ResidencyPriority::Critical)
            .unwrap();
        renderer
            .set_resource_priority(view, ResidencyPriority::Streamable)
            .unwrap();

        assert_eq!(
            renderer.resource_priority(mesh),
            Some(ResidencyPriority::Critical)
        );
        assert_eq!(
            renderer.resource_priority(buffer),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.resource_priority(texture),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.resource_priority(sampler),
            Some(ResidencyPriority::Low)
        );
        assert_eq!(
            renderer.resource_priority(shader),
            Some(ResidencyPriority::Streamable)
        );
        assert_eq!(
            renderer.resource_priority(template),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.resource_priority(material),
            Some(ResidencyPriority::Low)
        );
        assert_eq!(
            renderer.resource_priority(environment),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.resource_priority(render_target),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.resource_priority(camera),
            Some(ResidencyPriority::Critical)
        );
        assert_eq!(
            renderer.resource_priority(graph_extension),
            Some(ResidencyPriority::Streamable)
        );
        assert_eq!(
            renderer.resource_priority(skeleton),
            Some(ResidencyPriority::High)
        );
        assert_eq!(
            renderer.resource_priority(morph_weights),
            Some(ResidencyPriority::Low)
        );
        assert_eq!(
            renderer.resource_priority(scene),
            Some(ResidencyPriority::Critical)
        );
        assert_eq!(
            renderer.resource_priority(view),
            Some(ResidencyPriority::Streamable)
        );
    }

    #[test]
    fn frame_builds_stats_from_scene_and_view() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        struct TestExtension(Arc<AtomicUsize>);

        impl RenderGraphExtension for TestExtension {
            fn name(&self) -> &str {
                "test_extension"
            }

            fn build(
                &self,
                ctx: &RenderGraphExtensionContext,
                graph: &mut RenderGraphBuilder<'_>,
            ) -> Result<(), RendererError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                graph
                    .add_pass("test_extension")
                    .read_texture(ctx.main_color(), TextureReadUsage::Sampled)
                    .color_attachment(ctx.main_color(), ColorAttachmentOps::load_store())
                    .execute(|ctx| {
                        let view = ctx.view().expect("facade view info is attached");
                        assert_eq!(view.label.as_deref(), Some("view"));
                        assert_eq!(view.render_path, RenderPath::Deferred);
                        let pipeline = ctx.pipeline("test_extension_pipeline")?;
                        let mut pass =
                            ctx.begin_render_pass(RenderPassDesc::label("test_extension"));
                        pass.set_pipeline(pipeline);
                        Ok(())
                    });
                Ok(())
            }
        }

        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.enable_gpu_profiler(true).unwrap();
        renderer
            .set_frame_capture_backend_available(FrameCaptureBackend::RenderDoc, true)
            .unwrap();
        renderer
            .capture_next_frame(CaptureOptions {
                label: Some("frame_builds_stats".to_owned()),
                backend: FrameCaptureBackend::RenderDoc,
                include_resource_dump: true,
                open_after_capture: false,
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("main".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: true,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                label: Some("mat".to_owned()),
                domain: MaterialDomain::Opaque,
                base_color: Color::WHITE,
                base_color_texture: None,
                normal_texture: None,
                metallic_roughness_texture: None,
                occlusion_texture: None,
                emissive_texture: None,
                metallic: 0.0,
                roughness: 0.7,
                emissive: Vec3::ZERO,
                alpha_mode: AlphaMode::Opaque,
                double_sided: false,
                receive_shadows: true,
                cast_shadows: true,
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let skeleton = renderer
            .create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("frame_skeleton"),
                joint_matrices: &[IDENTITY_MAT4],
                inverse_bind_matrices: Some(&[IDENTITY_MAT4]),
                usage: AnimationDataUsage::Dynamic,
            })
            .unwrap();
        let morph_weights = renderer
            .create_morph_weights(MorphWeightsDesc {
                label: Some("frame_morphs"),
                weights: &[0.25],
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    skeleton: Some(skeleton),
                    morph_weights: Some(morph_weights),
                    flags: ObjectFlags::STATIC | ObjectFlags::GPU_CULLABLE,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let extension_calls = Arc::new(AtomicUsize::new(0));
        let view_extension = renderer
            .register_graph_extension(TestExtension(extension_calls.clone()))
            .unwrap();
        assert_eq!(
            renderer.graph_extension_name(view_extension),
            Some("test_extension")
        );
        let mut frame = renderer
            .begin_frame(FrameInput {
                delta_time: 1.0 / 60.0,
                absolute_time: 0.0,
                frame_index_override: None,
                wait_for_gpu: false,
            })
            .unwrap();
        frame
            .add_graph_extension(TestExtension(extension_calls.clone()))
            .unwrap();
        let view = frame
            .render_view(ViewDesc {
                label: Some("view".to_owned()),
                scene,
                camera: CameraDesc {
                    label: Some("camera".to_owned()),
                    transform: IDENTITY_MAT4,
                    projection: Projection::Perspective {
                        vertical_fov: 1.0,
                        aspect: 1.0,
                        near: 0.1,
                        far: None,
                        reverse_z: true,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Deferred,
                quality: ViewQualitySettings::high(),
                layers: RenderLayerMask::all(),
                graph_extensions: vec![view_extension],
            })
            .unwrap();
        assert_eq!(view.kind_tag(), ResourceKind::View.tag());
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 1);
        assert_eq!(
            stats.culling_outputs,
            vec![FrameCullingOutput {
                view_label: Some("view".to_owned()),
                gpu_culling: true,
                occlusion_culling: false,
                tested_objects: 1,
                visible_objects: 1,
                culled_objects: 0,
                visibility_buffer_label: "gpu_visibility".to_owned(),
                visibility_buffer_bytes: 4,
                indirect_args_buffer_label: "gpu_indirect_args".to_owned(),
                indirect_args_buffer_bytes: 16,
                occlusion_result_buffer_label: None,
                occlusion_result_buffer_bytes: 0,
            }]
        );
        assert_eq!(
            stats.ssao_outputs,
            vec![FrameSsaoOutput {
                view_label: Some("view".to_owned()),
                width: 1,
                height: 1,
                format: TextureFormat::Rgba8Unorm,
                output_texture_label: "ssao_occlusion".to_owned(),
            }]
        );
        assert_eq!(
            stats.gbuffer_outputs,
            vec![FrameGBufferOutput {
                view_label: Some("view".to_owned()),
                width: 1,
                height: 1,
                albedo_format: TextureFormat::Rgba8Unorm,
                normal_format: TextureFormat::Rgba16Float,
                material_format: TextureFormat::Rgba8Unorm,
                albedo_texture_label: "gbuffer_albedo".to_owned(),
                normal_texture_label: "gbuffer_normal".to_owned(),
                material_texture_label: "gbuffer_material".to_owned(),
            }]
        );
        assert_eq!(stats.skinned_objects, 1);
        assert_eq!(stats.morphed_objects, 1);
        assert_eq!(stats.deformed_objects, 1);
        assert_eq!(
            stats.deformation_outputs,
            vec![FrameDeformationOutput {
                view_label: Some("view".to_owned()),
                skinned_objects: 1,
                morphed_objects: 1,
                deformed_objects: 1,
                output_buffer_label: "gpu_deformed_vertices".to_owned(),
                output_buffer_bytes: 96,
            }]
        );
        assert_eq!(stats.motion_vector_objects, 0);
        assert_eq!(stats.motion_vector_views, 1);
        assert_eq!(
            stats.motion_vector_outputs,
            vec![FrameMotionVectorOutput {
                view_label: Some("view".to_owned()),
                width: 1,
                height: 1,
                format: TextureFormat::Rgba16Float,
                moving_objects: 0,
                camera_motion: true,
            }]
        );
        assert_eq!(stats.draw_calls, 1);
        assert!(stats.graph.pass_count >= 13);
        assert_eq!(stats.dispatch_calls, stats.graph.compute_dispatches);
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "gpu_culling"));
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "gpu_deformation"));
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "prepare_gpu_data"));
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("gpu_culling") < pass_index("depth_prepass"));
        assert!(pass_index("depth_prepass") < pass_index("picking_id"));
        assert!(stats.graph.transient_buffers >= 1);
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "picking_id"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "ssao"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "taa"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "fxaa"));
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "motion_blur"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "ssr"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "bloom"));
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "depth_of_field"));
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "color_grading"));
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("gbuffer") < pass_index("ssao"));
        assert!(pass_index("ssao") < pass_index("deferred_lighting"));
        assert!(pass_index("motion_vectors") < pass_index("taa"));
        assert!(pass_index("taa") < pass_index("fxaa"));
        assert!(pass_index("fxaa") < pass_index("motion_blur"));
        assert!(pass_index("motion_blur") < pass_index("ssr"));
        assert!(pass_index("ssr") < pass_index("bloom"));
        assert!(pass_index("bloom") < pass_index("depth_of_field"));
        assert!(pass_index("depth_of_field") < pass_index("tonemap"));
        assert!(pass_index("tonemap") < pass_index("color_grading"));
        assert!(pass_index("color_grading") < pass_index("present"));
        assert!(stats.graph.render_passes >= 10);
        assert!(stats.graph.pipeline_binds >= 17);
        assert!(stats.graph.fullscreen_draws >= 15);
        assert!(stats.gpu_profiler_enabled);
        assert!(stats.cpu_build_time_ms.is_finite());
        assert!(stats.cpu_submit_time_ms.is_finite());
        assert!(stats.cpu_build_time_ms >= 0.0);
        assert!(stats.cpu_submit_time_ms >= 0.0);
        assert_eq!(stats.gpu_time_ms, Some(0.0));
        let profile = stats.profile.as_ref().expect("profile data is attached");
        assert_eq!(profile.frame_index, stats.frame_index);
        assert_eq!(profile.cpu_build_time_ms, stats.cpu_build_time_ms);
        assert_eq!(profile.cpu_submit_time_ms, stats.cpu_submit_time_ms);
        assert_eq!(profile.gpu_time_ms, stats.gpu_time_ms);
        assert_eq!(profile.graph_passes, stats.graph.pass_count);
        assert_eq!(profile.graph_barriers, stats.graph.barriers);
        assert_eq!(profile.draw_calls, stats.draw_calls);
        assert_eq!(profile.dispatch_calls, stats.dispatch_calls);
        assert_eq!(profile.deformed_objects, stats.deformed_objects);
        assert_eq!(profile.motion_vector_objects, stats.motion_vector_objects);
        assert_eq!(profile.motion_vector_views, stats.motion_vector_views);
        assert_eq!(profile.pipeline_statistics, stats.pipeline_statistics);
        assert!(stats.capture_triggered);
        assert_eq!(stats.capture_label.as_deref(), Some("frame_builds_stats"));
        let capture = stats.capture.as_ref().expect("capture data is attached");
        assert_eq!(capture.label.as_deref(), Some("frame_builds_stats"));
        assert_eq!(capture.backend, FrameCaptureBackend::RenderDoc);
        assert_eq!(capture.status, FrameCaptureStatus::BackendHookRequested);
        assert!(capture.include_resource_dump);
        assert!(!capture.open_after_capture);
        assert_eq!(capture.frame_index, stats.frame_index);
        assert_eq!(capture.graph.pass_count, stats.graph.pass_count);
        assert!(capture
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "test_extension"));
        assert_eq!(capture.graph.pipeline_binds, stats.graph.pipeline_binds);
        assert_eq!(capture.cpu_build_time_ms, stats.cpu_build_time_ms);
        assert_eq!(capture.cpu_submit_time_ms, stats.cpu_submit_time_ms);
        assert_eq!(capture.draw_calls, stats.draw_calls);
        assert_eq!(capture.visible_objects, stats.visible_objects);
        assert_eq!(capture.skinned_objects, stats.skinned_objects);
        assert_eq!(capture.morphed_objects, stats.morphed_objects);
        assert_eq!(capture.deformed_objects, stats.deformed_objects);
        assert_eq!(capture.motion_vector_objects, stats.motion_vector_objects);
        assert_eq!(capture.motion_vector_views, stats.motion_vector_views);
        assert_eq!(capture.culling_outputs, stats.culling_outputs);
        assert_eq!(capture.ssao_outputs, stats.ssao_outputs);
        assert_eq!(capture.light_cluster_outputs, stats.light_cluster_outputs);
        assert_eq!(capture.shadow_outputs, stats.shadow_outputs);
        assert_eq!(capture.gbuffer_outputs, stats.gbuffer_outputs);
        assert_eq!(capture.lod_outputs, stats.lod_outputs);
        assert_eq!(capture.area_light_outputs, stats.area_light_outputs);
        assert_eq!(capture.streaming_outputs, stats.streaming_outputs);
        assert_eq!(capture.debug_draw_outputs, stats.debug_draw_outputs);
        assert_eq!(capture.picking_outputs, stats.picking_outputs);
        assert_eq!(capture.environment_outputs, stats.environment_outputs);
        assert_eq!(capture.deformation_outputs, stats.deformation_outputs);
        assert_eq!(capture.motion_vector_outputs, stats.motion_vector_outputs);
        assert_eq!(capture.post_process_outputs, stats.post_process_outputs);
        assert!(!capture.picking_outputs.is_empty());
        assert!(!capture.post_process_outputs.is_empty());
        assert_eq!(capture.pipeline_statistics, stats.pipeline_statistics);
        let resource_dump = capture
            .resource_dump
            .as_ref()
            .expect("resource dump is attached");
        assert!(resource_dump.meshes >= 1);
        assert!(resource_dump.materials >= 1);
        assert!(resource_dump.scenes >= 1);
        assert!(resource_dump.graph_extensions >= 1);
        assert_eq!(resource_dump.resident_bytes, stats.memory.resident_bytes);
        assert_eq!(
            resource_dump.delayed_destroy_count,
            stats.memory.delayed_destroy_count
        );
        assert_eq!(extension_calls.load(Ordering::SeqCst), 2);
        assert_eq!(count_ready(&renderer.graph_extensions), 1);
        assert_eq!(
            renderer.graph_extension_name(view_extension),
            Some("test_extension")
        );
        assert_eq!(renderer.last_frame_stats().unwrap().frame_index, 0);
        let next_stats = renderer
            .begin_frame(FrameInput::default())
            .unwrap()
            .finish()
            .unwrap();
        assert!(!next_stats.capture_triggered);
        assert!(next_stats.capture.is_none());
        assert!(next_stats.profile.is_some());
    }

    #[test]
    fn frame_owned_graph_extensions_are_released_after_finish() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());

        {
            let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
            frame.add_graph_extension(NoopExtension).unwrap();
            frame
                .add_post_process(CustomPostProcessDesc::new("frame_only_post"))
                .unwrap();
            assert_eq!(count_ready(&frame.renderer.graph_extensions), 2);
            frame.finish().unwrap();
        }

        assert_eq!(count_ready(&renderer.graph_extensions), 0);
        assert_eq!(count_destroy_queued(&renderer.graph_extensions), 2);
    }

    #[test]
    fn custom_post_process_passes_are_registered_and_reported() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("custom_post_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let view_post = renderer
            .register_post_process(CustomPostProcessDesc {
                label: "view_grade".to_owned(),
                pipeline_label: Some("view_grade_pipeline".to_owned()),
                output_texture_label: Some("view_grade_color".to_owned()),
            })
            .unwrap();
        assert_eq!(renderer.graph_extension_name(view_post), Some("view_grade"));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .add_post_process(CustomPostProcessDesc::new("frame_vignette"))
            .unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("custom_post_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 8,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: vec![view_post],
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        let labels = &stats.graph.pass_labels;
        assert!(labels.iter().any(|label| label == "frame_vignette"));
        assert!(labels.iter().any(|label| label == "frame_vignette_resolve"));
        assert!(labels.iter().any(|label| label == "view_grade"));
        assert!(labels.iter().any(|label| label == "view_grade_resolve"));
        assert_eq!(
            stats.post_process_outputs,
            vec![
                FramePostProcessOutput {
                    view_label: Some("custom_post_view".to_owned()),
                    pass_label: "frame_vignette".to_owned(),
                    width: 16,
                    height: 8,
                    format: TextureFormat::Rgba8Unorm,
                    output_texture_label: "frame_vignette_output".to_owned(),
                },
                FramePostProcessOutput {
                    view_label: Some("custom_post_view".to_owned()),
                    pass_label: "view_grade".to_owned(),
                    width: 16,
                    height: 8,
                    format: TextureFormat::Rgba8Unorm,
                    output_texture_label: "view_grade_color".to_owned(),
                },
            ]
        );
        assert!(stats.graph.fullscreen_draws >= 4);
        assert!(stats.graph.pipeline_binds >= 4);
    }

    #[test]
    fn graph_extensions_can_draw_custom_render_phases() {
        struct CustomPhaseExtension;

        impl RenderGraphExtension for CustomPhaseExtension {
            fn name(&self) -> &str {
                "custom_phase_extension"
            }

            fn build(
                &self,
                _ctx: &RenderGraphExtensionContext,
                graph: &mut RenderGraphBuilder<'_>,
            ) -> Result<(), RendererError> {
                graph.add_pass("custom_phase_draw").execute(|ctx| {
                    ctx.draw_render_phase(RenderPhaseKind::Custom(2))?;
                    Ok(())
                });
                Ok(())
            }
        }

        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("custom_phase_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame.add_graph_extension(CustomPhaseExtension).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("custom_phase_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 8,
                    height: 8,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "custom_phase_draw"));
        assert!(stats.graph.phase_draws >= 4);
    }

    #[test]
    fn render_targets_are_validated_and_can_back_offscreen_views() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let color = renderer
            .create_texture(TextureDesc {
                label: Some("offscreen_color"),
                dimension: TextureDimension::D2,
                width: 8,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET | TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let depth = renderer
            .create_texture(TextureDesc {
                label: Some("offscreen_depth"),
                dimension: TextureDimension::D2,
                width: 8,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::DEPTH_STENCIL,
                initial_data: None,
            })
            .unwrap();
        let render_target = renderer
            .create_render_target(RenderTargetDesc {
                label: Some("offscreen".to_owned()),
                color,
                depth: Some(depth),
                width: 8,
                height: 4,
                samples: 1,
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        let view = frame
            .render_view(ViewDesc {
                label: Some("offscreen_view".to_owned()),
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::External(render_target),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(view.kind_tag(), ResourceKind::View.tag());
        assert_eq!(stats.graph.pass_count, 6);
        assert_eq!(
            renderer.resource_status(render_target),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(renderer.render_target_desc(render_target).unwrap().width, 8);

        let layered_color = renderer
            .create_texture(TextureDesc {
                label: Some("layered_color"),
                dimension: TextureDimension::D2Array,
                width: 8,
                height: 4,
                depth_or_layers: 2,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.create_render_target(RenderTargetDesc {
                label: None,
                color: layered_color,
                depth: None,
                width: 8,
                height: 4,
                samples: 1,
            }),
            Err(RendererError::Validation(_))
        ));

        let mipmapped_color = renderer
            .create_texture(TextureDesc {
                label: Some("mipmapped_color"),
                dimension: TextureDimension::D2,
                width: 8,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        assert!(matches!(
            renderer.create_render_target(RenderTargetDesc {
                label: None,
                color: mipmapped_color,
                depth: None,
                width: 8,
                height: 4,
                samples: 1,
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn auto_render_path_uses_renderer_preference() {
        let mut renderer = Renderer::new_headless(RendererConfig {
            preferred_render_path: RenderPath::Forward,
            ..RendererConfig::default()
        });
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("auto_path_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Auto,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "forward_opaque"));
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "gbuffer"));
    }

    #[test]
    fn forward_plus_builds_light_cluster_buffer_for_forward_opaque() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene
                    .add_light(LightDesc::Directional(DirectionalLightDesc {
                        label: Some("shadowed_sun".to_owned()),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        illuminance_lux: 10_000.0,
                        shadow: Some(DirectionalShadowDesc {
                            resolution: 128,
                            cascades: 2,
                            max_distance: 20.0,
                            split_lambda: 0.5,
                            filter: ShadowFilter::Pcf { taps: 4 },
                            bias: ShadowBias {
                                constant: 0.001,
                                slope: 0.0,
                                normal: 0.0,
                            },
                        }),
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
                scene
                    .add_light(LightDesc::Point(PointLightDesc {
                        label: Some("clustered_point".to_owned()),
                        position: Vec3::new(0.0, 2.0, 0.0),
                        color: Color::WHITE,
                        intensity_lumen: 100.0,
                        radius: 5.0,
                        shadow: None,
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
                scene
                    .add_light(LightDesc::Custom(CustomLightDesc {
                        label: Some("clustered_custom".to_owned()),
                        type_id: 42,
                        position: Vec3::new(1.0, 2.0, 0.0),
                        color: Color::WHITE,
                        intensity: 50.0,
                        range: 6.0,
                        layer_mask: RenderLayerMask::all(),
                        parameters: vec![0.25, 0.5],
                    }))
                    .unwrap();
                scene
                    .add_light(LightDesc::Custom(CustomLightDesc {
                        label: Some("filtered_custom".to_owned()),
                        type_id: 43,
                        position: Vec3::new(1.0, 2.0, 0.0),
                        color: Color::WHITE,
                        intensity: 50.0,
                        range: 6.0,
                        layer_mask: RenderLayerMask::none(),
                        parameters: vec![1.0],
                    }))
                    .unwrap();
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("forward_plus_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 17,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::ForwardPlus,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };

        assert!(pass_index("depth_prepass") < pass_index("light_cluster_build"));
        assert!(pass_index("light_cluster_build") < pass_index("forward_opaque"));
        assert_eq!(stats.graph.compute_dispatches, 1);
        assert_eq!(stats.graph.transient_buffers, 1);
        assert_eq!(
            stats.light_cluster_outputs,
            vec![FrameLightClusterOutput {
                view_label: Some("forward_plus_view".to_owned()),
                tile_size_px: 16,
                z_slices: 24,
                cluster_count: 96,
                clustered_lights: 2,
                cluster_buffer_label: "light_cluster_grid".to_owned(),
                cluster_buffer_bytes: 1536,
            }]
        );
    }

    #[test]
    fn forward_plus_uses_async_compute_queue_for_light_clustering_when_supported() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features = renderer.caps.features | RendererFeatures::ASYNC_COMPUTE;
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene
                    .add_light(LightDesc::Point(PointLightDesc {
                        label: Some("clustered_point".to_owned()),
                        position: Vec3::new(0.0, 0.0, 2.0),
                        color: Color::WHITE,
                        intensity_lumen: 100.0,
                        radius: 8.0,
                        shadow: None,
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("forward_plus_async_compute".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 17,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::ForwardPlus,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };

        assert_eq!(stats.graph.async_compute_queue_passes, 1);
        assert_eq!(stats.graph.compute_queue_passes, 0);
        assert!(pass_index("depth_prepass") < pass_index("light_cluster_build"));
        assert!(pass_index("light_cluster_build") < pass_index("forward_opaque"));
    }

    #[test]
    fn deferred_graph_includes_point_spot_shadow_pass_when_needed() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    visibility: VisibilityFlags::CAMERA | VisibilityFlags::SHADOW,
                    flags: ObjectFlags::STATIC | ObjectFlags::CAST_SHADOW,
                    ..RenderObjectDesc::default()
                });
                scene
                    .add_light(LightDesc::Directional(DirectionalLightDesc {
                        label: Some("shadowed_sun".to_owned()),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        illuminance_lux: 10_000.0,
                        shadow: Some(DirectionalShadowDesc {
                            resolution: 128,
                            cascades: 2,
                            max_distance: 20.0,
                            split_lambda: 0.5,
                            filter: ShadowFilter::Pcf { taps: 4 },
                            bias: ShadowBias {
                                constant: 0.001,
                                slope: 0.0,
                                normal: 0.0,
                            },
                        }),
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
                scene
                    .add_light(LightDesc::Point(PointLightDesc {
                        label: Some("shadowed_point".to_owned()),
                        position: Vec3::new(0.0, 2.0, 0.0),
                        color: Color::WHITE,
                        intensity_lumen: 100.0,
                        radius: 5.0,
                        shadow: Some(PointShadowDesc {
                            resolution: 256,
                            bias: ShadowBias {
                                constant: 0.001,
                                slope: 0.0,
                                normal: 0.0,
                            },
                            filter: ShadowFilter::Hard,
                        }),
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("deferred_shadowed_point".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Deferred,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "shadow_csm"));
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "shadow_point_spot"));
        assert_eq!(
            stats.shadow_outputs,
            vec![
                FrameShadowOutput {
                    view_label: Some("deferred_shadowed_point".to_owned()),
                    pass_label: "shadow_csm".to_owned(),
                    width: 128,
                    height: 256,
                    format: TextureFormat::Depth32Float,
                    shadowed_lights: 1,
                    atlas_texture_label: "shadow_csm_atlas".to_owned(),
                },
                FrameShadowOutput {
                    view_label: Some("deferred_shadowed_point".to_owned()),
                    pass_label: "shadow_point_spot".to_owned(),
                    width: 256,
                    height: 1536,
                    format: TextureFormat::Depth32Float,
                    shadowed_lights: 1,
                    atlas_texture_label: "shadow_point_spot_atlas".to_owned(),
                }
            ]
        );

        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("deferred_no_point_shadow".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Deferred,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "shadow_csm"));
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "shadow_point_spot"));
        assert!(stats.shadow_outputs.is_empty());
    }

    #[test]
    fn shadow_pass_requires_visible_casting_object() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
                scene
                    .add_light(LightDesc::Directional(DirectionalLightDesc {
                        label: Some("shadowed_sun_without_caster".to_owned()),
                        direction: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                        illuminance_lux: 10_000.0,
                        shadow: Some(DirectionalShadowDesc {
                            resolution: 128,
                            cascades: 2,
                            max_distance: 20.0,
                            split_lambda: 0.5,
                            filter: ShadowFilter::Pcf { taps: 4 },
                            bias: ShadowBias {
                                constant: 0.001,
                                slope: 0.0,
                                normal: 0.0,
                            },
                        }),
                        layer_mask: RenderLayerMask::all(),
                    }))
                    .unwrap();
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("shadow_without_caster".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Deferred,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "shadow_csm"));
        assert!(stats.shadow_outputs.is_empty());
    }

    #[test]
    fn main_color_format_follows_hdr_and_target_format() {
        struct ExpectedMainColor {
            format: TextureFormat,
            extent: (u32, u32),
        }

        impl RenderGraphExtension for ExpectedMainColor {
            fn name(&self) -> &str {
                "expected_main_color"
            }

            fn build(
                &self,
                ctx: &RenderGraphExtensionContext,
                graph: &mut RenderGraphBuilder<'_>,
            ) -> Result<(), RendererError> {
                let desc = graph.texture_desc(ctx.main_color()).ok_or_else(|| {
                    RendererError::RenderGraphValidation(
                        "main color texture was not declared".to_owned(),
                    )
                })?;
                if desc.format != self.format || (desc.width, desc.height) != self.extent {
                    return Err(RendererError::Validation(format!(
                        "expected main color {:?} {:?}, got {:?} {:?}",
                        self.format,
                        self.extent,
                        desc.format,
                        (desc.width, desc.height)
                    )));
                }
                Ok(())
            }
        }

        let mut config = RendererConfig::default();
        config.surface_format = Some(TextureFormat::Bgra8UnormSrgb);
        let mut renderer = Renderer::new_headless(config);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();

        let ldr_extension = renderer
            .register_graph_extension(ExpectedMainColor {
                format: TextureFormat::Rgba8Unorm,
                extent: (16, 16),
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("ldr_headless".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: vec![ldr_extension],
            })
            .unwrap();
        frame.finish().unwrap();

        let hdr_extension = renderer
            .register_graph_extension(ExpectedMainColor {
                format: TextureFormat::Rgba16Float,
                extent: (32, 24),
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("hdr_headless".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 24,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::high(),
                layers: RenderLayerMask::all(),
                graph_extensions: vec![hdr_extension],
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "color_grading"));

        let surface_extension = renderer
            .register_graph_extension(ExpectedMainColor {
                format: TextureFormat::Bgra8UnormSrgb,
                extent: (640, 360),
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        let mut surface_camera = test_camera();
        surface_camera.viewport = Some([0.0, 0.0, 640.0, 360.0]);
        frame
            .render_view(ViewDesc {
                label: Some("surface_ldr".to_owned()),
                scene,
                camera: surface_camera,
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: vec![surface_extension],
            })
            .unwrap();
        frame.finish().unwrap();

        assert!(matches!(
            renderer.resize_surface(0, 180),
            Err(RendererError::Validation(_))
        ));
        renderer.resize_surface(320, 180).unwrap();
        let resized_surface_extension = renderer
            .register_graph_extension(ExpectedMainColor {
                format: TextureFormat::Bgra8UnormSrgb,
                extent: (320, 180),
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("resized_surface_ldr".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: vec![resized_surface_extension],
            })
            .unwrap();
        frame.finish().unwrap();

        let mut config = RendererConfig::default();
        config.hdr = false;
        let mut renderer = Renderer::new_headless(config);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let global_hdr_disabled = renderer
            .register_graph_extension(ExpectedMainColor {
                format: TextureFormat::Rgba8Unorm,
                extent: (16, 16),
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("global_hdr_disabled".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::high(),
                layers: RenderLayerMask::all(),
                graph_extensions: vec![global_hdr_disabled],
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "color_grading"));
    }

    #[test]
    fn headless_render_target_views_are_validated() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("headless_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba16Float,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.pass_count, 6);
        assert_eq!(stats.graph.phase_draws, 4);
        assert!(!stats.graph.pass_labels.iter().any(|label| label == "ssao"));
        assert!(!stats.graph.pass_labels.iter().any(|label| label == "taa"));
        assert!(!stats.graph.pass_labels.iter().any(|label| label == "fxaa"));
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "motion_blur"));
        assert!(!stats.graph.pass_labels.iter().any(|label| label == "ssr"));
        assert!(!stats.graph.pass_labels.iter().any(|label| label == "bloom"));
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "depth_of_field"));
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "color_grading"));
        assert_eq!(stats.motion_vector_objects, 0);
        assert_eq!(stats.motion_vector_views, 0);

        let invalid_surface = SurfaceHandle::from_raw(NonZeroU64::new(1).unwrap());
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert_eq!(
            frame.render_view(ViewDesc {
                label: Some("invalid_surface_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Surface(invalid_surface),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Surface,
                raw: invalid_surface.raw().get(),
            })
        );

        let surface = make_handle(ResourceKind::Surface, 0, 1);
        renderer.main_surface = Some(surface);
        renderer.surface_extent = Some((64, 32));
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("surface_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Surface(surface),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        assert_eq!(frame.finish().unwrap().graph.pass_count, 6);

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("taa_forward_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: true,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.pass_count, 9);
        assert_eq!(stats.motion_vector_views, 1);
        assert_eq!(
            stats.motion_vector_outputs,
            vec![FrameMotionVectorOutput {
                view_label: Some("taa_forward_view".to_owned()),
                width: 64,
                height: 32,
                format: TextureFormat::Rgba16Float,
                moving_objects: 0,
                camera_motion: true,
            }]
        );
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "motion_vectors"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "taa"));
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("taa") < pass_index("post_process_resolve"));
        assert!(pass_index("post_process_resolve") < pass_index("present"));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        let mut flagged_camera = test_camera();
        flagged_camera.flags = CameraFlags::MAIN
            | CameraFlags::ENABLE_TAA
            | CameraFlags::ENABLE_BLOOM
            | CameraFlags::ENABLE_SSAO;
        frame
            .render_view(ViewDesc {
                label: Some("camera_flag_quality_view".to_owned()),
                scene,
                camera: flagged_camera,
                target: RenderTarget::Headless {
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.motion_vector_views, 1);
        assert!(stats.graph.pass_labels.iter().any(|label| label == "ssao"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "taa"));
        assert!(stats.graph.pass_labels.iter().any(|label| label == "bloom"));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("fxaa_forward_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: true,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.pass_count, 8);
        assert!(stats.graph.pass_labels.iter().any(|label| label == "fxaa"));
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("fxaa") < pass_index("post_process_resolve"));
        assert!(pass_index("post_process_resolve") < pass_index("present"));
        assert_eq!(stats.motion_vector_views, 0);

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("optional_postprocess_forward_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: true,
                    depth_of_field: true,
                    motion_blur: true,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.pass_count, 11);
        assert_eq!(stats.graph.phase_draws, 5);
        assert_eq!(stats.graph.transient_textures, 7);
        assert_eq!(stats.motion_vector_objects, 0);
        assert_eq!(stats.motion_vector_views, 1);
        assert_eq!(
            stats.motion_vector_outputs,
            vec![FrameMotionVectorOutput {
                view_label: Some("optional_postprocess_forward_view".to_owned()),
                width: 64,
                height: 32,
                format: TextureFormat::Rgba16Float,
                moving_objects: 0,
                camera_motion: false,
            }]
        );
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("picking_id") < pass_index("transparent"));
        assert!(pass_index("transparent") < pass_index("motion_vectors"));
        assert!(pass_index("motion_vectors") < pass_index("motion_blur"));
        assert!(pass_index("motion_blur") < pass_index("ssr"));
        assert!(pass_index("ssr") < pass_index("depth_of_field"));
        assert!(pass_index("depth_of_field") < pass_index("post_process_resolve"));
        assert!(pass_index("post_process_resolve") < pass_index("present"));
        assert_eq!(
            stats.post_process_outputs,
            vec![
                FramePostProcessOutput {
                    view_label: Some("optional_postprocess_forward_view".to_owned()),
                    pass_label: "motion_blur".to_owned(),
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                    output_texture_label: "motion_blur_output".to_owned(),
                },
                FramePostProcessOutput {
                    view_label: Some("optional_postprocess_forward_view".to_owned()),
                    pass_label: "ssr".to_owned(),
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                    output_texture_label: "ssr_output".to_owned(),
                },
                FramePostProcessOutput {
                    view_label: Some("optional_postprocess_forward_view".to_owned()),
                    pass_label: "depth_of_field".to_owned(),
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                    output_texture_label: "depth_of_field_output".to_owned(),
                },
                FramePostProcessOutput {
                    view_label: Some("optional_postprocess_forward_view".to_owned()),
                    pass_label: "post_process_resolve".to_owned(),
                    width: 64,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                    output_texture_label: "main_color".to_owned(),
                },
            ]
        );

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 0,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn motion_vectors_count_only_object_transform_deltas() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let mut moved_transform = IDENTITY_MAT4;
        moved_transform[3][0] = 2.0;
        let mut sky_transform = IDENTITY_MAT4;
        sky_transform[3][0] = 3.0;
        let motion_flags = ObjectFlags::DYNAMIC | ObjectFlags::MOTION_VECTORS;
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    flags: motion_flags,
                    previous_transform: Some(IDENTITY_MAT4),
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: moved_transform,
                    flags: motion_flags,
                    previous_transform: Some(IDENTITY_MAT4),
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: moved_transform,
                    previous_transform: Some(IDENTITY_MAT4),
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![sky_material],
                    transform: sky_transform,
                    flags: motion_flags,
                    previous_transform: Some(IDENTITY_MAT4),
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("object_motion_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.visible_objects, 4);
        assert_eq!(stats.motion_vector_objects, 1);
        assert_eq!(stats.motion_vector_views, 1);
        assert_eq!(
            stats.motion_vector_outputs,
            vec![FrameMotionVectorOutput {
                view_label: Some("object_motion_view".to_owned()),
                width: 32,
                height: 16,
                format: TextureFormat::Rgba16Float,
                moving_objects: 1,
                camera_motion: false,
            }]
        );
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "motion_vectors"));
    }

    #[test]
    fn deformation_stats_count_only_geometry_phase_objects() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let skeleton = renderer
            .create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("deformation_skeleton"),
                joint_matrices: &[IDENTITY_MAT4],
                inverse_bind_matrices: Some(&[IDENTITY_MAT4]),
                usage: AnimationDataUsage::Dynamic,
            })
            .unwrap();
        let morph_weights = renderer
            .create_morph_weights(MorphWeightsDesc {
                label: Some("deformation_morphs"),
                weights: &[0.25],
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    skeleton: Some(skeleton),
                    morph_weights: Some(morph_weights),
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![sky_material],
                    skeleton: Some(skeleton),
                    morph_weights: Some(morph_weights),
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("deformation_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.skinned_objects, 1);
        assert_eq!(stats.morphed_objects, 1);
        assert_eq!(stats.deformed_objects, 1);
        assert_eq!(
            stats.deformation_outputs,
            vec![FrameDeformationOutput {
                view_label: Some("deformation_view".to_owned()),
                skinned_objects: 1,
                morphed_objects: 1,
                deformed_objects: 1,
                output_buffer_label: "gpu_deformed_vertices".to_owned(),
                output_buffer_bytes: 96,
            }]
        );
    }

    #[test]
    fn texture_view_render_targets_validate_subresource_ranges() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("array_target"),
                dimension: TextureDimension::D2Array,
                width: 16,
                height: 16,
                depth_or_layers: 4,
                mip_levels: 3,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET | TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("texture_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::TextureView(TextureViewDesc {
                    texture,
                    base_mip: 1,
                    mip_count: 1,
                    base_layer: 2,
                    layer_count: 2,
                }),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        assert_eq!(frame.finish().unwrap().graph.pass_count, 6);

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::TextureView(TextureViewDesc {
                    texture,
                    base_mip: 0,
                    mip_count: 2,
                    base_layer: 0,
                    layer_count: 1,
                }),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::TextureView(TextureViewDesc {
                    texture,
                    base_mip: 2,
                    mip_count: 2,
                    base_layer: 0,
                    layer_count: 1,
                }),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));

        let line_texture = renderer
            .create_texture(TextureDesc {
                label: Some("line_target"),
                dimension: TextureDimension::D1,
                width: 4,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::TextureView(TextureViewDesc {
                    texture: line_texture,
                    base_mip: 0,
                    mip_count: 1,
                    base_layer: 0,
                    layer_count: 1,
                }),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));

        let volume_texture = renderer
            .create_texture(TextureDesc {
                label: Some("volume_target"),
                dimension: TextureDimension::D3,
                width: 4,
                height: 4,
                depth_or_layers: 4,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::TextureView(TextureViewDesc {
                    texture: volume_texture,
                    base_mip: 0,
                    mip_count: 1,
                    base_layer: 0,
                    layer_count: 1,
                }),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn variable_rate_shading_quality_requires_capability_and_adds_graph_pass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features =
            RendererFeatures(renderer.caps.features.0 & !RendererFeatures::VARIABLE_RATE_SHADING.0);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let vrs_quality = ViewQualitySettings {
            hdr: false,
            bloom: false,
            taa: false,
            fxaa: false,
            ssao: false,
            ssr: false,
            depth_of_field: false,
            motion_blur: false,
            variable_rate_shading: true,
            bindless_textures: false,
            mesh_shaders: false,
            virtual_texturing: false,
            ray_tracing: false,
            color_grading: ColorGradingMode::None,
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: Some("vrs_unsupported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: vrs_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::VariableRateShading
            ))
        ));

        renderer.caps.features = renderer.caps.features | RendererFeatures::VARIABLE_RATE_SHADING;
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("vrs_supported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: vrs_quality,
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.variable_rate_shading_passes, 1);
        assert_eq!(stats.graph.transient_textures, 4);
        assert!(stats.graph.compute_dispatches >= 1);
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("vrs_shading_rate") < pass_index("depth_prepass"));
        assert!(pass_index("depth_prepass") < pass_index("picking_id"));
    }

    #[test]
    fn bindless_textures_require_capability_and_track_texture_table_pass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features =
            RendererFeatures(renderer.caps.features.0 & !RendererFeatures::BINDLESS_TEXTURES.0);
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("albedo"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let bindless_quality = ViewQualitySettings {
            bindless_textures: true,
            mesh_shaders: false,
            virtual_texturing: false,
            ray_tracing: false,
            ..ViewQualitySettings::default()
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: Some("bindless_unsupported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: bindless_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::BindlessTextures
            ))
        ));

        renderer.caps.features = renderer.caps.features | RendererFeatures::BINDLESS_TEXTURES;
        let sky_scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(sky_scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![sky_material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("bindless_ignores_sky_textures".to_owned()),
                scene: sky_scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: bindless_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.bindless_texture_table_passes, 0);
        assert_eq!(stats.graph.transient_buffers, 0);
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "bindless_texture_table"));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("bindless_supported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: bindless_quality,
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.bindless_texture_table_passes, 1);
        assert_eq!(stats.graph.transient_buffers, 1);
        assert!(stats.graph.compute_dispatches >= 1);
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("bindless_texture_table") < pass_index("depth_prepass"));
        assert!(pass_index("bindless_texture_table") < pass_index("forward_opaque"));
    }

    #[test]
    fn bindless_texture_table_uses_async_compute_queue_when_supported() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features =
            renderer.caps.features | RendererFeatures::BINDLESS_TEXTURES | RendererFeatures::ASYNC_COMPUTE;
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("async_bindless_albedo"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("bindless_async_compute".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    bindless_textures: true,
                    ..ViewQualitySettings::default()
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.graph.async_compute_queue_passes, 1);
        assert_eq!(stats.graph.compute_queue_passes, 0);
    }

    #[test]
    fn mesh_shaders_require_capability_and_track_meshlet_pass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features =
            RendererFeatures(renderer.caps.features.0 & !RendererFeatures::MESH_SHADER.0);
        let material = test_standard_material(&mut renderer);
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let mesh = test_mesh_with_meshlets(
            &mut renderer,
            0.0,
            Some(MeshletData {
                bytes: &[1, 2, 3, 4],
            }),
        );
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mesh_shader_quality = ViewQualitySettings {
            mesh_shaders: true,
            virtual_texturing: false,
            ray_tracing: false,
            ..ViewQualitySettings::default()
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: Some("mesh_shader_unsupported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: mesh_shader_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::MeshShader
            ))
        ));

        renderer.caps.features = renderer.caps.features | RendererFeatures::MESH_SHADER;
        let sky_scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(sky_scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![sky_material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("mesh_shader_ignores_sky_meshlets".to_owned()),
                scene: sky_scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: mesh_shader_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.mesh_shader_passes, 0);
        assert_eq!(stats.graph.transient_buffers, 0);
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "meshlet_culling"));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("mesh_shader_supported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: mesh_shader_quality,
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.mesh_shader_passes, 1);
        assert_eq!(stats.graph.transient_buffers, 1);
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("meshlet_culling") < pass_index("depth_prepass"));
    }

    #[test]
    fn meshlet_culling_runs_after_gpu_culling_and_before_depth_prepass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features = renderer.caps.features | RendererFeatures::MESH_SHADER;
        let material = test_standard_material(&mut renderer);
        let mesh = test_mesh_with_meshlets(
            &mut renderer,
            0.0,
            Some(MeshletData {
                bytes: &[1, 2, 3, 4],
            }),
        );
        let scene = renderer
            .create_scene(SceneDesc {
                enable_gpu_culling: true,
                ..SceneDesc::default()
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    flags: ObjectFlags::STATIC | ObjectFlags::GPU_CULLABLE,
                    bounds: Some(test_bounds()),
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("meshlet_after_gpu_culling".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    mesh_shaders: true,
                    ..ViewQualitySettings::default()
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("gpu_culling") < pass_index("meshlet_culling"));
        assert!(pass_index("meshlet_culling") < pass_index("depth_prepass"));
    }

    #[test]
    fn virtual_texturing_requires_capability_and_tracks_feedback_pass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features =
            RendererFeatures(renderer.caps.features.0 & !RendererFeatures::VIRTUAL_TEXTURING.0);
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("streamed_albedo"),
                dimension: TextureDimension::D2,
                width: 4,
                height: 4,
                depth_or_layers: 1,
                mip_levels: 3,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
                initial_data: None,
            })
            .unwrap();
        renderer
            .set_resource_priority(texture, ResidencyPriority::Streamable)
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let virtual_texture_quality = ViewQualitySettings {
            virtual_texturing: true,
            ray_tracing: false,
            ..ViewQualitySettings::default()
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: Some("virtual_texturing_unsupported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: virtual_texture_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::VirtualTexturing
            ))
        ));

        renderer.caps.features = renderer.caps.features | RendererFeatures::VIRTUAL_TEXTURING;
        let sky_scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(sky_scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![sky_material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("virtual_texturing_ignores_sky_textures".to_owned()),
                scene: sky_scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: virtual_texture_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.virtual_texture_feedback_passes, 0);
        assert_eq!(stats.graph.transient_buffers, 0);
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "virtual_texture_feedback"));

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("virtual_texturing_supported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: virtual_texture_quality,
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.virtual_texture_feedback_passes, 1);
        assert_eq!(stats.graph.transient_buffers, 1);
        assert_eq!(
            stats.streaming_outputs,
            vec![FrameStreamingOutput {
                view_label: Some("virtual_texturing_supported".to_owned()),
                streamable_textures: 1,
                streamable_texture_mips: 3,
                streamable_meshes: 0,
                streamable_mesh_bytes: 0,
            }]
        );
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("virtual_texture_feedback") < pass_index("depth_prepass"));
        assert!(pass_index("virtual_texture_feedback") < pass_index("forward_opaque"));
    }

    #[test]
    fn ray_tracing_requires_capability_and_tracks_acceleration_build_pass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features =
            RendererFeatures(renderer.caps.features.0 & !RendererFeatures::RAY_TRACING.0);
        let material = test_standard_material(&mut renderer);
        let mesh = test_mesh_with_usage(
            &mut renderer,
            0.0,
            MeshUsage::STATIC | MeshUsage::RAY_TRACING,
        );
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let ray_tracing_quality = ViewQualitySettings {
            ray_tracing: true,
            ..ViewQualitySettings::default()
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: Some("ray_tracing_unsupported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ray_tracing_quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::UnsupportedFeature(
                RendererFeature::RayTracing
            ))
        ));

        renderer.caps.features = renderer.caps.features | RendererFeatures::RAY_TRACING;
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("ray_tracing_supported".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ray_tracing_quality,
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.ray_tracing_passes, 1);
        assert_eq!(stats.graph.transient_buffers, 1);
        assert_eq!(
            stats.ray_tracing_outputs,
            vec![FrameRayTracingOutput {
                view_label: Some("ray_tracing_supported".to_owned()),
                visible_geometries: 1,
                accel_buffer_label: "ray_tracing_accel".to_owned(),
                accel_buffer_bytes: 64,
            }]
        );
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("ray_tracing_accel_build") < pass_index("depth_prepass"));
    }

    #[test]
    fn ray_tracing_accel_build_uses_only_ray_tracing_meshes() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features = renderer.caps.features | RendererFeatures::RAY_TRACING;
        let material = test_standard_material(&mut renderer);
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let non_rt_mesh = test_mesh(&mut renderer, 0.0);
        let rt_mesh = test_mesh_with_usage(
            &mut renderer,
            1.0,
            MeshUsage::STATIC | MeshUsage::RAY_TRACING,
        );
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh: non_rt_mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let quality = ViewQualitySettings {
            ray_tracing: true,
            ..ViewQualitySettings::default()
        };

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("ray_tracing_without_rt_mesh".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.ray_tracing_passes, 0);
        assert!(stats.ray_tracing_outputs.is_empty());
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "ray_tracing_accel_build"));

        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh: rt_mesh,
                    materials: vec![sky_material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("ray_tracing_with_sky_rt_mesh".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: quality.clone(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.ray_tracing_passes, 0);
        assert!(stats.ray_tracing_outputs.is_empty());
        assert!(!stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "ray_tracing_accel_build"));

        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh: rt_mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("ray_tracing_with_rt_mesh".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality,
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.graph.ray_tracing_passes, 1);
        assert_eq!(
            stats.ray_tracing_outputs,
            vec![FrameRayTracingOutput {
                view_label: Some("ray_tracing_with_rt_mesh".to_owned()),
                visible_geometries: 1,
                accel_buffer_label: "ray_tracing_accel".to_owned(),
                accel_buffer_bytes: 64,
            }]
        );
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "ray_tracing_accel_build"));
    }

    #[test]
    fn ray_tracing_accel_build_runs_after_gpu_culling_and_before_depth_prepass() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        renderer.caps.features = renderer.caps.features | RendererFeatures::RAY_TRACING;
        let material = test_standard_material(&mut renderer);
        let mesh = test_mesh_with_usage(
            &mut renderer,
            0.0,
            MeshUsage::STATIC | MeshUsage::RAY_TRACING,
        );
        let scene = renderer
            .create_scene(SceneDesc {
                enable_gpu_culling: true,
                ..SceneDesc::default()
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    flags: ObjectFlags::STATIC | ObjectFlags::GPU_CULLABLE,
                    bounds: Some(test_bounds()),
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("ray_tracing_after_gpu_culling".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 32,
                    height: 32,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    ray_tracing: true,
                    ..ViewQualitySettings::default()
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("gpu_culling") < pass_index("ray_tracing_accel_build"));
        assert!(pass_index("ray_tracing_accel_build") < pass_index("depth_prepass"));
    }

    #[test]
    fn render_view_rejects_texture_target_without_render_target_usage() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("sampled_only"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: None,
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();

        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::Texture(texture),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));

        let layered_texture = renderer
            .create_texture(TextureDesc {
                label: Some("layered_target"),
                dimension: TextureDimension::D2Array,
                width: 1,
                height: 1,
                depth_or_layers: 2,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::Texture(layered_texture),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));

        let mipmapped_texture = renderer
            .create_texture(TextureDesc {
                label: Some("mipmapped_target"),
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 2,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::RENDER_TARGET,
                initial_data: None,
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::Texture(mipmapped_texture),
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::Validation(_))
        ));
    }

    #[test]
    fn render_view_rejects_invalid_graph_extension_handles() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let extension = renderer.register_graph_extension(NoopExtension).unwrap();
        renderer.destroy(extension).unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();

        let err = frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: vec![extension],
            })
            .unwrap_err();
        assert!(matches!(
            err,
            RendererError::InvalidHandle {
                kind: ResourceKind::RenderGraphExtension,
                ..
            }
        ));
    }

    #[test]
    fn render_view_rejects_scene_objects_with_invalid_resource_handles() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh: make_handle(ResourceKind::Mesh, 99, 1),
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();

        assert!(matches!(
            frame.render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            }),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Mesh,
                ..
            })
        ));
    }

    #[test]
    fn render_view_filters_objects_by_camera_visibility_and_layer_mask() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let visible_mesh = test_mesh(&mut renderer, 0.0);
        let hidden_invalid_mesh = make_handle(ResourceKind::Mesh, 99, 1);
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh: visible_mesh,
                    layer: RenderLayer(2),
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh: hidden_invalid_mesh,
                    layer: RenderLayer(3),
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh: hidden_invalid_mesh,
                    layer: RenderLayer(2),
                    visibility: VisibilityFlags::SHADOW,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::single(RenderLayer(2)),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 1);
        assert_eq!(stats.draw_calls, 1);
    }

    #[test]
    fn frame_stats_count_frustum_culled_objects_when_culling_is_enabled() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("culled_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: true,
                enable_occlusion_culling: true,
            })
            .unwrap();
        let mut outside_transform = IDENTITY_MAT4;
        outside_transform[3][0] = 100.0;
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    bounds: Some(test_bounds()),
                    flags: ObjectFlags::STATIC | ObjectFlags::GPU_CULLABLE,
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: outside_transform,
                    bounds: Some(test_bounds()),
                    flags: ObjectFlags::STATIC | ObjectFlags::GPU_CULLABLE,
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: outside_transform,
                    bounds: Some(test_bounds()),
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.culled_objects, 1);
        assert_eq!(
            stats.culling_outputs,
            vec![FrameCullingOutput {
                view_label: None,
                gpu_culling: true,
                occlusion_culling: true,
                tested_objects: 2,
                visible_objects: 1,
                culled_objects: 1,
                visibility_buffer_label: "gpu_visibility".to_owned(),
                visibility_buffer_bytes: 8,
                indirect_args_buffer_label: "gpu_indirect_args".to_owned(),
                indirect_args_buffer_bytes: 16,
                occlusion_result_buffer_label: Some("gpu_occlusion_results".to_owned()),
                occlusion_result_buffer_bytes: 16,
            }]
        );
        assert_eq!(stats.draw_calls, 1);
        assert_eq!(stats.dispatch_calls, 2);
        assert_eq!(stats.graph.transient_buffers, 3);
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "gpu_culling"));
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "occlusion_culling"));
        let pass_index = |label: &str| {
            stats
                .graph
                .pass_labels
                .iter()
                .position(|pass| pass == label)
                .expect("expected pass label")
        };
        assert!(pass_index("gpu_culling") < pass_index("depth_prepass"));
        assert!(pass_index("depth_prepass") < pass_index("occlusion_culling"));
    }

    #[test]
    fn frame_stats_batch_visible_objects_by_mesh_and_material() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("batched_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mut second_transform = IDENTITY_MAT4;
        second_transform[3][0] = 1.0;
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    transform: second_transform,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.draw_calls, 1);
        assert_eq!(stats.triangles, 2);
        assert_eq!(stats.pipeline_switches, 1);
        assert_eq!(stats.material_switches, 1);
    }

    #[test]
    fn object_no_batch_flag_splits_batch_keys() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mut second_transform = IDENTITY_MAT4;
        second_transform[3][0] = 1.0;
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    transform: second_transform,
                    flags: ObjectFlags::STATIC | ObjectFlags::NO_BATCH,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.draw_calls, 2);
        assert_eq!(stats.pipeline_switches, 2);
        assert_eq!(stats.material_switches, 1);
    }

    #[test]
    fn mesh_no_merge_flag_splits_batch_keys() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh_with_flags(
            &mut renderer,
            0.0,
            MeshFlags::GPU_CULLABLE | MeshFlags::NO_MERGE,
        );
        let material = test_standard_material(&mut renderer);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mut second_transform = IDENTITY_MAT4;
        second_transform[3][0] = 1.0;
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    transform: second_transform,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.draw_calls, 2);
        assert_eq!(stats.pipeline_switches, 2);
        assert_eq!(stats.material_switches, 1);
    }

    #[test]
    fn object_receive_shadow_flag_splits_render_state_batches() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mut second_transform = IDENTITY_MAT4;
        second_transform[3][0] = 1.0;
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    transform: second_transform,
                    flags: ObjectFlags::STATIC | ObjectFlags::RECEIVE_SHADOW,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 2);
        assert_eq!(stats.draw_calls, 2);
        assert_eq!(stats.pipeline_switches, 2);
        assert_eq!(stats.material_switches, 1);
    }

    #[test]
    fn frame_stats_batches_submeshes_by_material_slot() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mut vertices = Vec::new();
        for (position, uv) in [
            ([-0.5_f32, 0.5, 0.0], [0.0_f32, 0.0]),
            ([-0.5, -0.5, 0.0], [0.0, 1.0]),
            ([0.5, 0.5, 0.0], [1.0, 0.0]),
            ([0.5, -0.5, 0.0], [1.0, 1.0]),
        ] {
            for value in position {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
            for value in [0.0_f32, 0.0, 1.0] {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
            for value in uv {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
        }
        let mesh = renderer
            .create_mesh(MeshDesc {
                label: Some("two_submesh_quad"),
                vertex_layout: VertexLayout {
                    streams: vec![VertexStreamLayout {
                        stride: 32,
                        step: VertexStepMode::Vertex,
                        attributes: vec![
                            VertexAttribute {
                                semantic: VertexSemantic::Position,
                                format: VertexFormat::Float32x3,
                                offset: 0,
                            },
                            VertexAttribute {
                                semantic: VertexSemantic::Normal,
                                format: VertexFormat::Float32x3,
                                offset: 12,
                            },
                            VertexAttribute {
                                semantic: VertexSemantic::TexCoord(0),
                                format: VertexFormat::Float32x2,
                                offset: 24,
                            },
                        ],
                    }],
                },
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2, 2, 1, 3])),
                submeshes: vec![
                    SubMeshDesc {
                        index_range: 0..3,
                        vertex_range: 0..3,
                        material_slot: 0,
                        bounds: test_bounds(),
                    },
                    SubMeshDesc {
                        index_range: 3..6,
                        vertex_range: 1..4,
                        material_slot: 1,
                        bounds: test_bounds(),
                    },
                ],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::GPU_CULLABLE,
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            })
            .unwrap();
        let first_material = test_standard_material(&mut renderer);
        let second_material = test_standard_material(&mut renderer);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![first_material, second_material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 1);
        assert_eq!(stats.draw_calls, 2);
        assert_eq!(stats.triangles, 2);
        assert_eq!(stats.pipeline_switches, 2);
        assert_eq!(stats.material_switches, 2);
    }

    #[test]
    fn frame_stats_respect_material_pass_flags_for_view_batches() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("motion_only_shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("motion_only_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc::default(),
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::MOTION | MaterialPassFlags::PICKING,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: Some("motion_only_material".to_owned()),
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 1);
        assert_eq!(stats.draw_calls, 0);
        assert_eq!(stats.pipeline_switches, 0);
        assert_eq!(stats.material_switches, 0);
    }

    #[test]
    fn frame_stats_count_custom_material_draw_items_per_phase() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("multi_phase_shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("multi_phase_template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc::default(),
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD | MaterialPassFlags::TRANSPARENT,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: Some("multi_phase_material".to_owned()),
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::Headless {
                    width: 16,
                    height: 16,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 1);
        assert_eq!(stats.draw_calls, 2);
        assert_eq!(stats.pipeline_switches, 2);
        assert_eq!(stats.material_switches, 1);
    }

    #[test]
    fn frame_pipeline_statistics_reflect_pipeline_statistics_feature() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        renderer
            .capture_next_frame(CaptureOptions {
                label: Some("pipeline_stats".to_owned()),
                ..CaptureOptions::default()
            })
            .unwrap();
        renderer.enable_gpu_profiler(true).unwrap();

        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("pipeline_stats_view".to_owned()),
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(
            stats.pipeline_statistics.is_some(),
            cfg!(feature = "pipeline-statistics")
        );
        let Some(pipeline_stats) = &stats.pipeline_statistics else {
            return;
        };
        assert_eq!(pipeline_stats.input_assembly_vertices, 3);
        assert_eq!(pipeline_stats.input_assembly_primitives, 1);
        assert_eq!(pipeline_stats.vertex_shader_invocations, 3);
        assert_eq!(pipeline_stats.clipping_invocations, 1);
        assert_eq!(pipeline_stats.clipping_primitives, 1);
        assert_eq!(pipeline_stats.fragment_shader_invocations, 1);
        assert_eq!(
            pipeline_stats.compute_shader_invocations,
            u64::from(stats.graph.compute_dispatches)
        );
        assert_eq!(pipeline_stats.draw_calls, stats.draw_calls);
        assert_eq!(pipeline_stats.dispatch_calls, stats.dispatch_calls);
        assert_eq!(
            stats
                .profile
                .as_ref()
                .expect("profile data is attached")
                .pipeline_statistics
                .as_ref(),
            Some(pipeline_stats)
        );
        assert_eq!(
            stats
                .capture
                .as_ref()
                .expect("capture data is attached")
                .pipeline_statistics
                .as_ref(),
            Some(pipeline_stats)
        );
    }

    #[test]
    fn frame_stats_track_material_and_pipeline_switches_from_batches() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let first_material = test_standard_material(&mut renderer);
        let second_material = test_standard_material(&mut renderer);
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("switch_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![first_material],
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![second_material],
                    ..RenderObjectDesc::default()
                });
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![first_material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.visible_objects, 3);
        assert_eq!(stats.draw_calls, 2);
        assert_eq!(stats.triangles, 3);
        assert_eq!(stats.pipeline_switches, 2);
        assert_eq!(stats.material_switches, 2);
    }

    #[test]
    fn frame_stats_report_resident_memory_and_delayed_destroy_count() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("resident_texture"),
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsage::SAMPLED,
                initial_data: Some(TextureInitialData {
                    bytes: &[255; 16],
                    bytes_per_row: 8,
                    rows_per_image: 2,
                }),
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("memory_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        renderer.destroy(texture).unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert!(stats.memory.resident_bytes > 0);
        assert_eq!(stats.memory.delayed_destroy_count, 1);
        assert_eq!(renderer.memory_stats(), stats.memory);
    }

    #[test]
    fn resource_residency_controls_streamed_meshes_and_textures() {
        let mut renderer = Renderer::new_headless(RendererConfig {
            validation: ValidationMode::Full,
            ..RendererConfig::default()
        });
        let mesh = test_mesh_with_usage(&mut renderer, 0.0, MeshUsage::STREAMING);
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("streamed_albedo"),
                dimension: TextureDimension::D2,
                width: 2,
                height: 2,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
                initial_data: Some(TextureInitialData {
                    bytes: &[255; 16],
                    bytes_per_row: 8,
                    rows_per_image: 2,
                }),
            })
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                base_color_texture: Some(texture),
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("residency_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();

        let initial_memory = renderer.memory_stats().resident_bytes;
        renderer.evict_resource(texture).unwrap();
        assert_eq!(
            renderer.texture_info(texture).unwrap().status,
            ResourceStatus::Evicted
        );
        assert!(renderer.memory_stats().resident_bytes < initial_memory);
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        let missing_texture = frame.render_view(ViewDesc {
            label: Some("residency_view".to_owned()),
            scene,
            camera: test_camera(),
            target: RenderTarget::MainSurface,
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings {
                hdr: false,
                bloom: false,
                taa: false,
                fxaa: false,
                ssao: false,
                ssr: false,
                depth_of_field: false,
                motion_blur: false,
                variable_rate_shading: false,
                bindless_textures: false,
                mesh_shaders: false,
                virtual_texturing: false,
                ray_tracing: false,
                color_grading: ColorGradingMode::None,
            },
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        });
        assert_eq!(
            missing_texture,
            Err(RendererError::ResourceNotReady(ResourceKind::Texture))
        );
        drop(frame);

        renderer.make_resource_resident(texture).unwrap();
        renderer.evict_resource(mesh).unwrap();
        assert_eq!(
            renderer.mesh_info(mesh).unwrap().status,
            ResourceStatus::Evicted
        );
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        let missing_mesh = frame.render_view(ViewDesc {
            label: Some("residency_view".to_owned()),
            scene,
            camera: test_camera(),
            target: RenderTarget::MainSurface,
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings {
                hdr: false,
                bloom: false,
                taa: false,
                fxaa: false,
                ssao: false,
                ssr: false,
                depth_of_field: false,
                motion_blur: false,
                variable_rate_shading: false,
                bindless_textures: false,
                mesh_shaders: false,
                virtual_texturing: false,
                ray_tracing: false,
                color_grading: ColorGradingMode::None,
            },
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        });
        assert_eq!(
            missing_mesh,
            Err(RendererError::ResourceNotReady(ResourceKind::Mesh))
        );
        drop(frame);

        renderer.make_resource_resident(mesh).unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame
            .render_view(ViewDesc {
                label: Some("residency_view".to_owned()),
                scene,
                camera: test_camera(),
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.visible_objects, 1);
        assert!(renderer.memory_stats().resident_bytes >= initial_memory);
    }

    #[test]
    fn frame_wait_for_gpu_flushes_pending_upload_stats() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mesh = test_mesh(&mut renderer, 0.0);
        assert!(renderer.upload_stats().bytes_queued > 0);
        assert!(renderer.upload_stats().pending_uploads > 0);
        assert!(renderer.upload_stats().staging_bytes_in_use > 0);
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("wait_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let mut frame = renderer
            .begin_frame(FrameInput {
                wait_for_gpu: true,
                ..FrameInput::default()
            })
            .unwrap();
        frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();

        assert_eq!(stats.upload.bytes_queued, 0);
        assert!(stats.upload.bytes_uploaded_this_frame > 0);
        assert_eq!(stats.upload.pending_uploads, 0);
        assert_eq!(stats.upload.staging_bytes_in_use, 0);
        assert_eq!(renderer.upload_stats().bytes_queued, 0);
    }

    #[test]
    fn upload_stats_track_pending_staging_until_flush() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let buffer = renderer
            .create_buffer(BufferDesc {
                label: Some("upload_buffer"),
                size: 8,
                usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                initial_data: Some(&[1, 2, 3, 4]),
            })
            .unwrap();
        assert_eq!(
            renderer.upload_stats(),
            UploadStats {
                bytes_queued: 4,
                bytes_uploaded_this_frame: 0,
                pending_uploads: 1,
                staging_bytes_in_use: 4,
            }
        );

        renderer
            .update_buffer(
                buffer,
                BufferUpdate {
                    byte_offset: 4,
                    data: &[5, 6],
                },
            )
            .unwrap();
        assert_eq!(
            renderer.upload_stats(),
            UploadStats {
                bytes_queued: 6,
                bytes_uploaded_this_frame: 0,
                pending_uploads: 2,
                staging_bytes_in_use: 6,
            }
        );

        renderer.flush_uploads().unwrap();
        assert_eq!(
            renderer.upload_stats(),
            UploadStats {
                bytes_queued: 0,
                bytes_uploaded_this_frame: 6,
                pending_uploads: 0,
                staging_bytes_in_use: 0,
            }
        );
    }

    #[test]
    fn scene_command_buffer_debug_draw_and_picking_are_exposed() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("commands".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let object = renderer.reserve_scene_object(scene).unwrap();
        assert!(matches!(
            renderer.reserve_scene_object(make_handle(ResourceKind::Scene, 99, 1)),
            Err(RendererError::InvalidHandle { .. })
        ));
        let light = renderer.reserve_scene_light(scene).unwrap();
        assert!(matches!(
            renderer.reserve_scene_light(make_handle(ResourceKind::Scene, 99, 1)),
            Err(RendererError::InvalidHandle { .. })
        ));
        let mesh = test_mesh(&mut renderer, 0.0);
        let material = test_standard_material(&mut renderer);
        let mut commands = SceneCommandBuffer::new(scene);
        commands.spawn_reserved(
            object,
            RenderObjectDesc {
                mesh,
                bounds: Some(Bounds3::new(Vec3::ZERO, Vec3::ONE)),
                user_id: 42,
                ..RenderObjectDesc::default()
            },
        );
        commands.set_transform(object, IDENTITY_MAT4);
        commands.set_material(object, 0, material);
        commands.set_visibility(object, VisibilityFlags::CAMERA | VisibilityFlags::PICKING);
        commands.set_flags(object, ObjectFlags::DYNAMIC | ObjectFlags::MOTION_VECTORS);
        commands.add_light_reserved(
            light,
            LightDesc::Point(PointLightDesc {
                label: Some("queued_point".to_owned()),
                position: Vec3::ZERO,
                color: Color::WHITE,
                intensity_lumen: 10.0,
                radius: 1.0,
                shadow: None,
                layer_mask: RenderLayerMask::all(),
            }),
        );
        commands.update_light(
            light,
            LightUpdate {
                desc: LightDesc::Point(PointLightDesc {
                    label: Some("queued_point_updated".to_owned()),
                    position: Vec3::ONE,
                    color: Color::WHITE,
                    intensity_lumen: 20.0,
                    radius: 2.0,
                    shadow: None,
                    layer_mask: RenderLayerMask::all(),
                }),
            },
        );
        assert_eq!(commands.len(), 7);
        assert!(matches!(
            commands.commands.first(),
            Some(SceneCommand::Spawn(_, command_object)) if *command_object == object
        ));
        assert!(matches!(
            commands.commands.get(1),
            Some(SceneCommand::SetTransform(command_object, _)) if *command_object == object
        ));
        assert!(matches!(
            commands.commands.get(2),
            Some(SceneCommand::SetMaterial(command_object, 0, command_material))
                if *command_object == object && *command_material == material
        ));
        assert!(matches!(
            commands.commands.get(4),
            Some(SceneCommand::SetFlags {
                object: command_object,
                flags,
            }) if *command_object == object
                && *flags == (ObjectFlags::DYNAMIC | ObjectFlags::MOTION_VECTORS)
        ));
        assert!(matches!(
            commands.commands.get(5),
            Some(SceneCommand::AddLight(_, command_light)) if *command_light == light
        ));
        renderer.apply_scene_commands(commands).unwrap();
        let stored_scene = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        assert!(matches!(
            stored_scene
                .lights
                .get(ResourceKind::Light, light)
                .and_then(|slot| slot.value.as_ref()),
            Some(LightDesc::Point(PointLightDesc {
                intensity_lumen: 20.0,
                radius: 2.0,
                ..
            }))
        ));
        let stored_object = stored_scene
            .objects
            .get(ResourceKind::Object, object)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        assert_eq!(stored_object.materials.first(), Some(&material));
        assert_eq!(
            stored_object.visibility,
            VisibilityFlags::CAMERA | VisibilityFlags::PICKING
        );
        assert_eq!(
            stored_object.flags,
            ObjectFlags::DYNAMIC | ObjectFlags::MOTION_VECTORS
        );

        let mut debug = renderer.debug_draw();
        debug.line(Vec3::ZERO, Vec3::ONE, Color::WHITE);
        debug.sphere(Vec3::ZERO, 1.0, Color::BLACK);
        assert_eq!(debug.len(), 2);
        drop(debug);
        assert_eq!(renderer.debug_draw_commands().len(), 2);

        let mut frame = renderer
            .begin_frame(FrameInput {
                delta_time: 0.0,
                absolute_time: 0.0,
                frame_index_override: None,
                wait_for_gpu: false,
            })
            .unwrap();
        frame
            .debug_draw()
            .ray(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 1.0, Color::WHITE);
        let view = frame
            .render_view(ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 1.0,
                        height: 1.0,
                        near: 0.0,
                        far: 1.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Manual(1.0),
                    clear: ClearOptions::None,
                    viewport: Some([0.0, 0.0, 16.0, 16.0]),
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(stats.draw_calls, 4);
        assert!(stats
            .graph
            .pass_labels
            .iter()
            .any(|label| label == "debug_overlay"));
        assert_eq!(
            stats.debug_draw_outputs,
            vec![FrameDebugDrawOutput {
                view_label: None,
                command_count: 3,
                target_texture_label: "main_color".to_owned(),
            }]
        );
        assert_eq!(
            stats.picking_outputs,
            vec![FramePickingOutput {
                view_label: None,
                width: 16,
                height: 16,
                format: TextureFormat::Rgba8Unorm,
                pickable_objects: 1,
                target_texture_label: "picking_id".to_owned(),
                ready_results: 0,
            }]
        );
        assert!(renderer.debug_draw_commands().is_empty());

        let ticket = renderer
            .request_picking(PickingRequest {
                view,
                pixel: UVec2::new(4, 8),
            })
            .unwrap();
        let ticket_handle = PickingHandle::from_raw(ticket.raw);
        assert_eq!(ticket_handle.kind_tag(), ResourceKind::Picking.tag());
        assert_eq!(
            renderer.resource_status(ticket_handle),
            Some(ResourceStatus::Ready)
        );
        let picked = renderer.poll_picking(ticket).unwrap();
        assert_eq!(picked.object, Some(object));
        assert_eq!(picked.user_id, 42);
        assert_eq!(picked.world_position, Vec3::new(0.5, 0.5, 0.5));
        assert_eq!(picked.source, PickingResultSource::GpuReadback);
        assert_eq!(picked.readback_pixel, Some(UVec2::new(4, 8)));
        assert_eq!(
            picked.encoded_object_id,
            encode_gpu_picking_object_index(object)
        );
    }

    #[test]
    fn picking_uses_view_pixel_and_nearest_visible_object() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let bounds = Bounds3::new(Vec3::new(-0.25, -0.25, -0.25), Vec3::new(0.25, 0.25, 0.25));
        let mut near_transform = IDENTITY_MAT4;
        near_transform[3][2] = -1.0;
        let mut sky_transform = IDENTITY_MAT4;
        sky_transform[3][2] = -0.25;
        let mut unpickable_transform = IDENTITY_MAT4;
        unpickable_transform[3][2] = -0.5;
        let mut far_transform = IDENTITY_MAT4;
        far_transform[3][2] = -3.0;
        let sky_material = renderer
            .create_standard_material(StandardMaterialDesc {
                domain: MaterialDomain::Sky,
                ..StandardMaterialDesc::default()
            })
            .unwrap();
        let (hidden, unpickable, sky, near, far) = renderer
            .edit_scene(scene, |scene| {
                let hidden = scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: IDENTITY_MAT4,
                    bounds: Some(bounds),
                    visibility: VisibilityFlags(0),
                    user_id: 1,
                    ..RenderObjectDesc::default()
                });
                let far = scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: far_transform,
                    bounds: Some(bounds),
                    visibility: VisibilityFlags::CAMERA | VisibilityFlags::PICKING,
                    user_id: 2,
                    ..RenderObjectDesc::default()
                });
                let unpickable = scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: unpickable_transform,
                    bounds: Some(bounds),
                    visibility: VisibilityFlags::CAMERA,
                    user_id: 4,
                    ..RenderObjectDesc::default()
                });
                let sky = scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![sky_material],
                    transform: sky_transform,
                    bounds: Some(bounds),
                    visibility: VisibilityFlags::CAMERA | VisibilityFlags::PICKING,
                    user_id: 5,
                    ..RenderObjectDesc::default()
                });
                let near = scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: near_transform,
                    bounds: Some(bounds),
                    visibility: VisibilityFlags::CAMERA | VisibilityFlags::PICKING,
                    user_id: 3,
                    ..RenderObjectDesc::default()
                });
                (hidden, unpickable, sky, near, far)
            })
            .unwrap();
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        let view = frame
            .render_view(ViewDesc {
                label: Some("pick_view".to_owned()),
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 4.0,
                        height: 4.0,
                        near: 0.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Manual(1.0),
                    clear: ClearOptions::None,
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::Headless {
                    width: 64,
                    height: 64,
                    format: TextureFormat::Rgba8Unorm,
                },
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        frame.finish().unwrap();

        let center = renderer
            .request_picking(PickingRequest {
                view,
                pixel: UVec2::new(32, 32),
            })
            .unwrap();
        let picked = renderer.poll_picking(center).unwrap();
        assert_eq!(picked.object, Some(near));
        assert_eq!(picked.user_id, 3);
        assert_eq!(picked.depth, 1.0);
        assert_eq!(picked.source, PickingResultSource::GpuReadback);
        assert_eq!(picked.readback_pixel, Some(UVec2::new(32, 32)));
        assert_eq!(
            picked.encoded_object_id,
            encode_gpu_picking_object_index(near)
        );

        let gpu_decoded = renderer
            .decode_gpu_picking_pixel(
                view,
                encode_gpu_picking_object_index(near),
                0.25,
                Vec3::new(1.0, 2.0, 3.0),
            )
            .unwrap();
        assert_eq!(gpu_decoded.object, Some(near));
        assert_eq!(gpu_decoded.user_id, 3);
        assert_eq!(gpu_decoded.depth, 0.25);
        assert_eq!(gpu_decoded.world_position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(gpu_decoded.source, PickingResultSource::GpuReadback);
        assert_eq!(gpu_decoded.readback_pixel, None);
        assert_eq!(
            gpu_decoded.encoded_object_id,
            encode_gpu_picking_object_index(near)
        );
        assert!(matches!(
            renderer.decode_gpu_picking_pixel(
                view,
                encode_gpu_picking_object_index(near),
                f32::NAN,
                Vec3::ZERO,
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.decode_gpu_picking_pixel(
                view,
                encode_gpu_picking_object_index(near),
                1.25,
                Vec3::ZERO,
            ),
            Err(RendererError::Validation(_))
        ));
        assert!(matches!(
            renderer.decode_gpu_picking_pixel(
                view,
                encode_gpu_picking_object_index(near),
                0.25,
                Vec3::new(f32::INFINITY, 0.0, 0.0),
            ),
            Err(RendererError::Validation(_))
        ));
        assert_eq!(
            renderer
                .decode_gpu_picking_pixel(view, [0, 0, 0, 0], 1.0, Vec3::ZERO)
                .unwrap()
                .object,
            None
        );

        let stale_far_id = encode_gpu_picking_object_index(far);
        let replacement = renderer
            .edit_scene(scene, |scene| {
                scene.despawn(far).unwrap();
                scene.spawn(RenderObjectDesc {
                    mesh,
                    transform: far_transform,
                    bounds: Some(bounds),
                    user_id: 99,
                    ..RenderObjectDesc::default()
                })
            })
            .unwrap();
        let stale_decoded = renderer
            .decode_gpu_picking_pixel(view, stale_far_id, 0.75, Vec3::ZERO)
            .unwrap();
        assert_eq!(stale_decoded.object, None);
        let replacement_decoded = renderer
            .decode_gpu_picking_pixel(
                view,
                encode_gpu_picking_object_index(replacement),
                0.75,
                Vec3::ZERO,
            )
            .unwrap();
        assert_eq!(replacement_decoded.object, Some(replacement));
        assert_eq!(replacement_decoded.user_id, 99);

        let miss = renderer
            .request_picking(PickingRequest {
                view,
                pixel: UVec2::new(0, 0),
            })
            .unwrap();
        let miss = renderer.poll_picking(miss).unwrap();
        assert_eq!(miss.object, None);
        assert_eq!(miss.readback_pixel, Some(UVec2::new(0, 0)));
        assert_eq!(miss.encoded_object_id, [0, 0, 0, 0]);
        assert_ne!(picked.object, Some(hidden));
        assert_ne!(picked.object, Some(unpickable));
        assert_ne!(picked.object, Some(sky));
        assert_ne!(picked.object, Some(far));
    }

    #[test]
    fn extract_render_data_applies_scene_command_buffer() {
        struct ExtractedRenderable {
            mesh: MeshHandle,
        }

        impl ExtractRenderData for ExtractedRenderable {
            fn extract(&self, commands: &mut SceneCommandBuffer) {
                commands.spawn(RenderObjectDesc {
                    label: Some("extracted".to_owned()),
                    mesh: self.mesh,
                    user_id: 7,
                    ..RenderObjectDesc::default()
                });
            }
        }

        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("extract".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        renderer
            .extract_render_data(scene, &ExtractedRenderable { mesh })
            .unwrap();

        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        let extracted = stored
            .objects
            .resources
            .iter()
            .filter_map(|slot| slot.value.as_ref())
            .find(|object| object.user_id == 7)
            .unwrap();
        assert_eq!(extracted.label.as_deref(), Some("extracted"));
        assert_eq!(extracted.mesh, mesh);
    }

    #[test]
    fn scene_editing_returns_values_and_command_errors_propagate() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        let count = renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    ..RenderObjectDesc::default()
                });
                1_usize
            })
            .unwrap();
        assert_eq!(count, 1);

        fn spawn_with_documented_writer(writer: &mut SceneWriter<'_>, mesh: MeshHandle) {
            writer.spawn(RenderObjectDesc {
                mesh,
                ..RenderObjectDesc::default()
            });
        }
        renderer
            .edit_scene(scene, |writer| spawn_with_documented_writer(writer, mesh))
            .unwrap();

        let mut commands = SceneCommandBuffer::new(scene);
        commands.set_transform(make_handle(ResourceKind::Object, 99, 1), IDENTITY_MAT4);
        assert!(matches!(
            renderer.apply_scene_commands(commands),
            Err(RendererError::InvalidHandle {
                kind: ResourceKind::Object,
                ..
            })
        ));

        let mut bad_transform = IDENTITY_MAT4;
        bad_transform[0][0] = f32::NAN;
        let mut commands = SceneCommandBuffer::new(scene);
        commands.spawn(RenderObjectDesc {
            mesh,
            transform: bad_transform,
            ..RenderObjectDesc::default()
        });
        assert!(matches!(
            renderer.apply_scene_commands(commands),
            Err(RendererError::Validation(_))
        ));

        let reserved = renderer.reserve_scene_object(scene).unwrap();
        let mut bad_previous = IDENTITY_MAT4;
        bad_previous[0][0] = f32::INFINITY;
        let mut commands = SceneCommandBuffer::new(scene);
        commands.spawn_reserved(
            reserved,
            RenderObjectDesc {
                mesh,
                previous_transform: Some(bad_previous),
                ..RenderObjectDesc::default()
            },
        );
        assert!(matches!(
            renderer.apply_scene_commands(commands),
            Err(RendererError::Validation(_))
        ));

        let object = renderer.reserve_scene_object(scene).unwrap();
        let mut commands = SceneCommandBuffer::new(scene);
        commands.spawn_reserved(
            object,
            RenderObjectDesc {
                mesh,
                previous_transform: Some(IDENTITY_MAT4),
                ..RenderObjectDesc::default()
            },
        );
        commands.clear_previous_transform(object);
        renderer.apply_scene_commands(commands).unwrap();
        let stored = renderer
            .scenes
            .get(ResourceKind::Scene, scene)
            .and_then(|slot| slot.value.as_ref())
            .unwrap();
        assert_eq!(
            stored
                .objects
                .get(ResourceKind::Object, object)
                .and_then(|slot| slot.value.as_ref())
                .and_then(|object| object.previous_transform),
            None
        );
    }

    #[test]
    fn scene_writer_rejects_non_finite_transforms_and_invalid_bounds() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let mesh = test_mesh(&mut renderer, 0.0);
        renderer
            .edit_scene(scene, |scene| {
                let object = scene.spawn(RenderObjectDesc {
                    mesh,
                    ..RenderObjectDesc::default()
                });
                let mut bad_transform = IDENTITY_MAT4;
                bad_transform[0][0] = f32::NAN;
                assert!(matches!(
                    scene.set_transform(object, bad_transform),
                    Err(RendererError::Validation(_))
                ));
                assert!(matches!(
                    scene.set_bounds(object, Bounds3::new(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO),),
                    Err(RendererError::Validation(_))
                ));
            })
            .unwrap();

        let mut assert_invalid_spawn_rejected = |object: RenderObjectDesc| {
            let scene = renderer.create_scene(SceneDesc::default()).unwrap();
            renderer
                .edit_scene(scene, |scene| scene.spawn(object))
                .unwrap();
            let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
            assert!(matches!(
                frame.render_view(ViewDesc {
                    label: None,
                    scene,
                    camera: test_camera(),
                    target: RenderTarget::Headless {
                        width: 16,
                        height: 16,
                        format: TextureFormat::Rgba8Unorm,
                    },
                    render_path: RenderPath::Forward,
                    quality: ViewQualitySettings::default(),
                    layers: RenderLayerMask::all(),
                    graph_extensions: Vec::new(),
                }),
                Err(RendererError::Validation(_))
            ));
        };
        let mut bad_spawn_transform = IDENTITY_MAT4;
        bad_spawn_transform[0][0] = f32::NAN;
        assert_invalid_spawn_rejected(RenderObjectDesc {
            mesh,
            transform: bad_spawn_transform,
            ..RenderObjectDesc::default()
        });
        let mut bad_previous_transform = IDENTITY_MAT4;
        bad_previous_transform[0][0] = f32::INFINITY;
        assert_invalid_spawn_rejected(RenderObjectDesc {
            mesh,
            previous_transform: Some(bad_previous_transform),
            ..RenderObjectDesc::default()
        });
        assert_invalid_spawn_rejected(RenderObjectDesc {
            mesh,
            bounds: Some(Bounds3::new(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO)),
            ..RenderObjectDesc::default()
        });
    }

    #[test]
    fn generic_resource_lifecycle_covers_public_resource_kinds() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let buffer = renderer
            .create_buffer(BufferDesc {
                label: Some("buffer"),
                size: 16,
                usage: BufferUsage::STORAGE | BufferUsage::COPY_DST,
                initial_data: Some(&[1, 2, 3, 4]),
            })
            .unwrap();
        let shader = renderer
            .create_shader(ShaderDesc {
                label: Some("shader"),
                source: ShaderSource::Wgsl("@vertex fn vs() {}"),
                stages: ShaderStages::VERTEX,
                entry_points: ShaderEntryPoints {
                    vertex: Some("vs"),
                    fragment: None,
                    compute: None,
                },
                reflection: ShaderReflectionMode::Disabled,
                features: ShaderFeatureSet::default(),
                hot_reload_key: None,
            })
            .unwrap();
        let template = renderer
            .create_material_template(MaterialTemplateDesc {
                label: Some("template".to_owned()),
                shader,
                domain: MaterialDomain::Opaque,
                render_state: RenderStateDesc { depth_write: true },
                parameter_schema: MaterialParameterSchema::default(),
                passes: MaterialPassFlags::FORWARD,
            })
            .unwrap();
        let material = renderer
            .create_material(MaterialDesc {
                label: None,
                template,
                parameters: Vec::new(),
                overrides: MaterialOverrides::default(),
            })
            .unwrap();
        let sampler = renderer
            .create_sampler(SamplerDesc {
                address_u: AddressMode::Repeat,
                address_v: AddressMode::Repeat,
                address_w: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mip_filter: FilterMode::Linear,
                compare: None,
                anisotropy: 1,
                lod_min: OrderedF32::new(0.0),
                lod_max: OrderedF32::new(16.0),
            })
            .unwrap();
        let environment = renderer
            .create_environment(EnvironmentDesc {
                label: Some("environment".to_owned()),
                skybox: None,
                irradiance: None,
                prefiltered_specular: None,
                brdf_lut: None,
                intensity: 0.0,
                rotation: Quat::IDENTITY,
                diffuse_color: Color::WHITE,
                diffuse_intensity: 0.25,
                specular_color: Color::WHITE,
                specular_intensity: 0.5,
                texture: None,
                background_intensity: 0.0,
            })
            .unwrap();
        let skeleton = renderer
            .create_skeleton_instance(SkeletonInstanceDesc {
                label: Some("skeleton"),
                joint_matrices: &[IDENTITY_MAT4],
                inverse_bind_matrices: None,
                usage: AnimationDataUsage::Dynamic,
            })
            .unwrap();
        let morph_weights = renderer
            .create_morph_weights(MorphWeightsDesc {
                label: Some("morphs"),
                weights: &[0.0, 1.0],
            })
            .unwrap();
        let camera = renderer.create_camera(test_camera()).unwrap();
        let graph_extension = renderer.register_graph_extension(NoopExtension).unwrap();

        assert_eq!(
            renderer.resource_status(buffer),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(shader),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(template),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(material),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(sampler),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(environment),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(skeleton),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(morph_weights),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(camera),
            Some(ResourceStatus::Ready)
        );
        assert_eq!(
            renderer.resource_status(graph_extension),
            Some(ResourceStatus::Ready)
        );

        renderer.destroy(buffer).unwrap();
        renderer.destroy(shader).unwrap();
        renderer.destroy(template).unwrap();
        renderer.destroy(material).unwrap();
        renderer.destroy(sampler).unwrap();
        renderer.destroy(environment).unwrap();
        renderer.destroy(skeleton).unwrap();
        renderer.destroy(morph_weights).unwrap();
        renderer.destroy(camera).unwrap();
        renderer.destroy(graph_extension).unwrap();

        assert_eq!(renderer.memory_stats().delayed_destroy_count, 10);

        assert_eq!(
            renderer.resource_status(buffer),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(shader),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(template),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(material),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(sampler),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(environment),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(skeleton),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(morph_weights),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(camera),
            Some(ResourceStatus::DestroyQueued)
        );
        assert_eq!(
            renderer.resource_status(graph_extension),
            Some(ResourceStatus::DestroyQueued)
        );
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn facade_scene_can_build_legacy_render_scene() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let vertex_layout = VertexLayout {
            streams: vec![VertexStreamLayout {
                stride: 32,
                step: VertexStepMode::Vertex,
                attributes: vec![
                    VertexAttribute {
                        semantic: VertexSemantic::Position,
                        format: VertexFormat::Float32x3,
                        offset: 0,
                    },
                    VertexAttribute {
                        semantic: VertexSemantic::Normal,
                        format: VertexFormat::Float32x3,
                        offset: 12,
                    },
                    VertexAttribute {
                        semantic: VertexSemantic::TexCoord(0),
                        format: VertexFormat::Float32x2,
                        offset: 24,
                    },
                ],
            }],
        };
        let mut vertices = Vec::new();
        for (position, uv) in [
            ([0.0_f32, 0.5, 0.0], [0.5_f32, 0.0]),
            ([-0.5, -0.5, 0.0], [0.0, 1.0]),
            ([0.5, -0.5, 0.0], [1.0, 1.0]),
        ] {
            for value in position {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
            for value in [0.0_f32, 0.0, 1.0] {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
            for value in uv {
                vertices.extend_from_slice(&value.to_le_bytes());
            }
        }
        let mesh = renderer
            .create_mesh(MeshDesc {
                label: Some("triangle"),
                vertex_layout,
                vertices: VertexData::Interleaved(&vertices),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::default(),
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            })
            .unwrap();
        let texture = renderer
            .create_texture(TextureDesc {
                label: Some("white"),
                dimension: TextureDimension::D2,
                width: 1,
                height: 1,
                depth_or_layers: 1,
                mip_levels: 1,
                samples: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::SAMPLED,
                initial_data: Some(TextureInitialData {
                    bytes: &[255, 255, 255, 255],
                    bytes_per_row: 4,
                    rows_per_image: 1,
                }),
            })
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                label: Some("standard".to_owned()),
                domain: MaterialDomain::Opaque,
                base_color: Color::WHITE,
                base_color_texture: Some(texture),
                normal_texture: None,
                metallic_roughness_texture: None,
                occlusion_texture: None,
                emissive_texture: None,
                metallic: 0.0,
                roughness: 0.5,
                emissive: Vec3::ZERO,
                alpha_mode: AlphaMode::Opaque,
                double_sided: false,
                receive_shadows: true,
                cast_shadows: true,
            })
            .unwrap();
        let environment = renderer
            .create_environment(EnvironmentDesc {
                label: Some("studio".to_owned()),
                skybox: Some(texture),
                irradiance: None,
                prefiltered_specular: None,
                brdf_lut: None,
                intensity: 0.8,
                rotation: Quat::IDENTITY,
                diffuse_color: Color::WHITE,
                diffuse_intensity: 0.4,
                specular_color: Color::WHITE,
                specular_intensity: 0.8,
                texture: Some(texture),
                background_intensity: 0.2,
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
                scene.set_environment(Some(environment)).unwrap();
            })
            .unwrap();
        let view = ViewDesc {
            label: Some("view".to_owned()),
            scene,
            camera: CameraDesc {
                label: Some("camera".to_owned()),
                transform: IDENTITY_MAT4,
                projection: Projection::Perspective {
                    vertical_fov: 1.0,
                    aspect: 1.0,
                    near: 0.1,
                    far: Some(100.0),
                    reverse_z: false,
                },
                exposure: Exposure::Auto,
                clear: ClearOptions::ColorDepth(Color::BLACK),
                viewport: None,
                scissor: None,
                jitter: None,
                previous_view_proj: None,
                flags: CameraFlags::MAIN,
            },
            target: RenderTarget::MainSurface,
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings {
                hdr: false,
                bloom: false,
                taa: false,
                fxaa: false,
                ssao: false,
                ssr: false,
                depth_of_field: false,
                motion_blur: false,
                variable_rate_shading: false,
                bindless_textures: false,
                mesh_shaders: false,
                virtual_texturing: false,
                ray_tracing: false,
                color_grading: ColorGradingMode::None,
            },
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };
        let legacy = renderer.build_legacy_scene(&view).unwrap();
        let queue = engine_render::RenderQueue::from_scene(&legacy);

        assert_eq!(queue.stats().item_count, 1);
        assert_eq!(queue.stats().draw_call_count, 1);
        assert_eq!(legacy.mesh_entries().count(), 1);
        assert_eq!(legacy.texture_entries().count(), 1);
        assert_eq!(legacy.lighting().environment.diffuse_intensity, 0.4);
        assert_eq!(legacy.lighting().environment.specular_intensity, 0.8);
        assert_eq!(legacy.lighting().environment.background_intensity, 0.2);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn facade_streamed_mesh_can_build_legacy_render_scene() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let mut positions = Vec::new();
        let mut texcoords = Vec::new();
        for (position, uv) in [
            ([0.0_f32, 0.5, 0.0], [0.5_f32, 0.0]),
            ([-0.5, -0.5, 0.0], [0.0, 1.0]),
            ([0.5, -0.5, 0.0], [1.0, 1.0]),
        ] {
            for value in position {
                positions.extend_from_slice(&value.to_le_bytes());
            }
            for value in uv {
                texcoords.extend_from_slice(&value.to_le_bytes());
            }
        }
        let mesh = renderer
            .create_mesh(MeshDesc {
                label: Some("streamed_triangle"),
                vertex_layout: VertexLayout {
                    streams: vec![
                        VertexStreamLayout {
                            stride: 12,
                            step: VertexStepMode::Vertex,
                            attributes: vec![VertexAttribute {
                                semantic: VertexSemantic::Position,
                                format: VertexFormat::Float32x3,
                                offset: 0,
                            }],
                        },
                        VertexStreamLayout {
                            stride: 8,
                            step: VertexStepMode::Vertex,
                            attributes: vec![VertexAttribute {
                                semantic: VertexSemantic::TexCoord(0),
                                format: VertexFormat::Float32x2,
                                offset: 0,
                            }],
                        },
                    ],
                },
                vertices: VertexData::Streams(vec![
                    VertexStream {
                        data: &positions,
                        stride: 12,
                    },
                    VertexStream {
                        data: &texcoords,
                        stride: 8,
                    },
                ]),
                indices: Some(IndexData::U16(&[0, 1, 2])),
                submeshes: vec![SubMeshDesc {
                    index_range: 0..3,
                    vertex_range: 0..3,
                    material_slot: 0,
                    bounds: test_bounds(),
                }],
                bounds: test_bounds(),
                usage: MeshUsage::STATIC,
                flags: MeshFlags::default(),
                skin: None,
                morph_targets: Vec::new(),
                meshlets: None,
            })
            .unwrap();
        let material = renderer
            .create_standard_material(StandardMaterialDesc {
                label: Some("standard".to_owned()),
                domain: MaterialDomain::Opaque,
                base_color: Color::WHITE,
                base_color_texture: None,
                normal_texture: None,
                metallic_roughness_texture: None,
                occlusion_texture: None,
                emissive_texture: None,
                metallic: 0.0,
                roughness: 0.5,
                emissive: Vec3::ZERO,
                alpha_mode: AlphaMode::Opaque,
                double_sided: false,
                receive_shadows: true,
                cast_shadows: true,
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh,
                    materials: vec![material],
                    ..RenderObjectDesc::default()
                });
            })
            .unwrap();
        let legacy = renderer
            .build_legacy_scene(&ViewDesc {
                label: Some("view".to_owned()),
                scene,
                camera: CameraDesc {
                    label: Some("camera".to_owned()),
                    transform: IDENTITY_MAT4,
                    projection: Projection::Perspective {
                        vertical_fov: 1.0,
                        aspect: 1.0,
                        near: 0.1,
                        far: Some(100.0),
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let queue = engine_render::RenderQueue::from_scene(&legacy);

        assert_eq!(queue.stats().item_count, 1);
        assert_eq!(legacy.mesh_entries().count(), 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn debug_draw_lines_are_added_to_legacy_render_scene() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("debug_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        renderer.debug_draw().line(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Color::rgba(1.0, 0.0, 0.0, 0.75),
        );

        let legacy = renderer
            .build_legacy_scene(&ViewDesc {
                label: None,
                scene,
                camera: CameraDesc {
                    label: None,
                    transform: IDENTITY_MAT4,
                    projection: Projection::Perspective {
                        vertical_fov: 1.0,
                        aspect: 1.0,
                        near: 0.1,
                        far: Some(100.0),
                        reverse_z: false,
                    },
                    exposure: Exposure::Auto,
                    clear: ClearOptions::ColorDepth(Color::BLACK),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings {
                    hdr: false,
                    bloom: false,
                    taa: false,
                    fxaa: false,
                    ssao: false,
                    ssr: false,
                    depth_of_field: false,
                    motion_blur: false,
                    variable_rate_shading: false,
                    bindless_textures: false,
                    mesh_shaders: false,
                    virtual_texturing: false,
                    ray_tracing: false,
                    color_grading: ColorGradingMode::None,
                },
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let queue = engine_render::RenderQueue::from_scene(&legacy);

        assert_eq!(legacy.mesh_entries().count(), 1);
        assert_eq!(legacy.material_entries().count(), 2);
        assert_eq!(queue.stats().item_count, 1);
        assert_eq!(queue.stats().transparent_item_count, 1);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn debug_draw_frustum_is_added_to_legacy_render_scene() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .debug_draw()
            .frustum(IDENTITY_MAT4, Color::rgba(0.0, 1.0, 0.0, 1.0));

        let legacy = renderer
            .build_legacy_scene(&ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let queue = engine_render::RenderQueue::from_scene(&legacy);

        assert_eq!(legacy.mesh_entries().count(), 12);
        assert_eq!(legacy.material_entries().count(), 13);
        assert_eq!(queue.stats().item_count, 12);
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn debug_draw_text_3d_is_added_to_legacy_render_scene() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let scene = renderer.create_scene(SceneDesc::default()).unwrap();
        renderer
            .debug_draw()
            .text_3d(Vec3::new(1.0, 2.0, 3.0), "A", Color::WHITE);

        let legacy = renderer
            .build_legacy_scene(&ViewDesc {
                label: None,
                scene,
                camera: test_camera(),
                target: RenderTarget::MainSurface,
                render_path: RenderPath::Forward,
                quality: ViewQualitySettings::default(),
                layers: RenderLayerMask::all(),
                graph_extensions: Vec::new(),
            })
            .unwrap();
        let queue = engine_render::RenderQueue::from_scene(&legacy);

        assert_eq!(legacy.mesh_entries().count(), 12);
        assert_eq!(legacy.material_entries().count(), 13);
        assert_eq!(queue.stats().item_count, 12);
    }

    #[test]
    fn frustum_corners_are_recovered_from_identity_view_projection() {
        let corners = frustum_corners_from_inverse_view_projection(IDENTITY_MAT4).unwrap();
        assert_eq!(corners[0], Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(corners[3], Vec3::new(1.0, 1.0, -1.0));
        assert_eq!(corners[4], Vec3::new(-1.0, -1.0, 1.0));
        assert_eq!(corners[7], Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(invert_mat4(IDENTITY_MAT4), Some(IDENTITY_MAT4));
    }

    #[cfg(feature = "backend-wgpu")]
    #[test]
    fn lod_group_selects_legacy_mesh_by_camera_distance() {
        let mut renderer = Renderer::new_headless(RendererConfig::default());
        let near_mesh = test_mesh(&mut renderer, 0.0);
        let far_mesh = test_mesh(&mut renderer, 10.0);
        let material = test_standard_material(&mut renderer);
        assert!(matches!(
            renderer.create_lod_group(LodGroupDesc {
                label: Some("bad_lod".to_owned()),
                levels: vec![LodLevelDesc {
                    max_distance: 5.0,
                    mesh: near_mesh,
                    materials: vec![material],
                    bounds: Some(Bounds3::new(
                        Vec3::new(1.0, 0.0, 0.0),
                        Vec3::new(0.0, 1.0, 1.0),
                    )),
                }],
            }),
            Err(RendererError::Validation(_))
        ));
        let lod_group = renderer
            .create_lod_group(LodGroupDesc {
                label: Some("lod".to_owned()),
                levels: vec![
                    LodLevelDesc {
                        max_distance: 5.0,
                        mesh: near_mesh,
                        materials: vec![material],
                        bounds: Some(test_bounds()),
                    },
                    LodLevelDesc {
                        max_distance: 50.0,
                        mesh: far_mesh,
                        materials: vec![material],
                        bounds: Some(test_bounds()),
                    },
                ],
            })
            .unwrap();
        let scene = renderer
            .create_scene(SceneDesc {
                label: Some("lod_scene".to_owned()),
                max_objects_hint: None,
                max_lights_hint: None,
                enable_gpu_culling: false,
                enable_occlusion_culling: false,
            })
            .unwrap();
        let object = renderer
            .edit_scene(scene, |scene| {
                scene.spawn(RenderObjectDesc {
                    mesh: near_mesh,
                    materials: vec![material],
                    transform: IDENTITY_MAT4,
                    lod_group: Some(lod_group),
                    ..RenderObjectDesc::default()
                })
            })
            .unwrap();
        let mut camera_transform = IDENTITY_MAT4;
        camera_transform[3][2] = 20.0;
        let view = ViewDesc {
            label: Some("lod_view".to_owned()),
            scene,
            camera: CameraDesc {
                label: None,
                transform: camera_transform,
                projection: Projection::Perspective {
                    vertical_fov: 1.0,
                    aspect: 1.0,
                    near: 0.1,
                    far: Some(100.0),
                    reverse_z: false,
                },
                exposure: Exposure::Auto,
                clear: ClearOptions::ColorDepth(Color::BLACK),
                viewport: None,
                scissor: None,
                jitter: None,
                previous_view_proj: None,
                flags: CameraFlags::MAIN,
            },
            target: RenderTarget::MainSurface,
            render_path: RenderPath::Forward,
            quality: ViewQualitySettings {
                hdr: false,
                bloom: false,
                taa: false,
                fxaa: false,
                ssao: false,
                ssr: false,
                depth_of_field: false,
                motion_blur: false,
                variable_rate_shading: false,
                bindless_textures: false,
                mesh_shaders: false,
                virtual_texturing: false,
                ray_tracing: false,
                color_grading: ColorGradingMode::None,
            },
            layers: RenderLayerMask::all(),
            graph_extensions: Vec::new(),
        };
        let legacy = renderer.build_legacy_scene(&view).unwrap();
        let queue = engine_render::RenderQueue::from_scene(&legacy);

        assert_eq!(queue.items()[0].mesh.index(), far_mesh.index() as usize);
        let mut frame = renderer.begin_frame(FrameInput::default()).unwrap();
        frame.render_view(view).unwrap();
        let stats = frame.finish().unwrap();
        assert_eq!(
            stats.lod_outputs,
            vec![FrameLodOutput {
                view_label: Some("lod_view".to_owned()),
                object,
                lod_group,
                level_index: 1,
                selected_mesh: far_mesh,
                distance: 20.0,
            }]
        );
        assert_eq!(
            renderer.resource_status(lod_group),
            Some(ResourceStatus::Ready)
        );
    }
}
