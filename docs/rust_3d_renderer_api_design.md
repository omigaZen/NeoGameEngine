# Rust 游戏引擎 Renderer 层对外 API 设计文档

版本：0.1  
目标读者：引擎架构开发者、Renderer 开发者、工具链开发者、ECS/Asset 系统开发者  
目标：设计一套 **Rust 游戏引擎 Renderer 层对外 API**，支持完整 3D 渲染，并为未来 GPU Driven、Bindless、Ray Tracing、Meshlet、Editor 工具链和多后端扩展预留空间。

---

## 1. 设计结论摘要

Renderer 对外 API 应该分三层暴露：

```text
Game / ECS / Editor
        │
        ▼
High-level Renderer API
Scene / Mesh / Material / Texture / Camera / Light / View
        │
        ▼
RenderGraph Extension API
Custom Pass / Post Process / Debug Pass / Tool Pass
        │
        ▼
RHI API, mostly internal
Device / Queue / Buffer / Texture / Pipeline / Barrier
```

核心选择：

1. **普通游戏代码只接触 Scene、Mesh、Material、Texture、Camera、Light、View，不接触 descriptor set / bind group / command buffer。**
2. **Renderer 使用 handle 管理 GPU 资源，避免 Rust 生命周期与 GPU 生命周期互相缠斗。**
3. **每帧渲染使用 RenderGraph 编译执行，统一处理 pass 依赖、资源生命周期、barrier、transient resource、异步 compute 和 debug capture。**
4. **资源 API 是 retained mode，帧提交 API 是 declarative mode。**
5. **标准 3D 渲染管线内置，custom pass 通过 RenderGraph 插件扩展。**
6. **RHI 作为内部层存在，只有高级插件开发者可以选择性访问。**
7. **完整 3D 的基线能力包括 PBR、IBL、shadow、deferred/forward+、透明物、skinning、morph、post process、GPU culling、instancing、debug/profiling。**

---

## 2. Renderer 边界

### 2.1 Renderer 应该负责

Renderer 负责把游戏世界中的可见数据变成 GPU 命令，并输出到 surface 或 texture：

```text
Render Assets
+ Render Scene
+ View / Camera / Light
        │
        ▼
Extraction / Prepare / Queue
        │
        ▼
RenderGraph
        │
        ▼
RHI Commands
        │
        ▼
Swapchain / RenderTarget
```

Renderer 应该管理：

- GPU device / queue / swapchain / surface
- GPU buffer / texture / sampler / pipeline
- mesh / texture / material / shader / environment
- camera / view / render target
- light / shadow / IBL
- RenderGraph pass 编排
- GPU resource upload / streaming / delayed destroy
- shader compilation / hot reload / variant cache
- culling / batching / sorting / instancing
- frame stats / GPU markers / profiling

### 2.2 Renderer 不应该负责

Renderer 不应该直接管理：

- 游戏逻辑
- Transform 层级系统
- 物理系统
- 动画状态机
- ECS 主世界
- Asset 文件格式本身
- Window 事件循环

但 Renderer 需要提供 API 让这些系统把数据提交进来。

---

## 3. Crate 拆分建议

推荐拆分为多个 crate，避免 public API 被 backend 细节污染：

```text
engine_renderer              // 用户主要依赖的 facade crate
engine_renderer_core         // handle、desc、scene、material、view、error
engine_renderer_graph        // RenderGraph API
engine_renderer_rhi          // RHI traits/types, mostly internal
engine_renderer_pbr          // 标准 3D 管线、PBR、shadow、IBL
engine_renderer_backend_wgpu // wgpu backend
engine_renderer_backend_vk   // 可选 Vulkan backend
engine_renderer_editor       // debug draw、gizmo、capture、inspector
```

对游戏代码暴露：

```rust
use engine_renderer::prelude::*;
```

`prelude` 应包含：

```rust
pub mod prelude {
    pub use crate::{
        Renderer, RendererConfig, RendererError,
        Handle, MeshHandle, TextureHandle, MaterialHandle,
        SceneHandle, ObjectHandle, CameraHandle, LightHandle,
        MeshDesc, TextureDesc, MaterialDesc, MaterialInfo,
        MaterialTemplateInfo, MaterialReflectionCoverageStats, StandardMaterialDesc,
        SceneDesc, RenderObjectDesc, CameraDesc, LightDesc,
        ViewDesc, RenderTarget, FrameInput, FrameStats,
    };
}
```

---

## 4. API 设计原则

### 4.1 不把 GPU API 细节扔给游戏层

游戏层不应该写：

```rust
cmd.bind_pipeline(...);
cmd.bind_descriptor_set(...);
cmd.draw_indexed(...);
```

游戏层应该写：

```rust
scene.spawn(RenderObjectDesc {
    mesh,
    material,
    transform,
    ..Default::default()
});
```

Renderer 内部负责：

- 选择 pipeline
- 创建 bind group / descriptor
- 排序 draw call
- 合批
- 更新 per-object buffer
- 判断 object 是否进 shadow pass / depth prepass / transparent pass

### 4.2 Handle 优先，不把 GPU 资源引用暴露给上层

推荐：

```rust
MeshHandle
TextureHandle
MaterialHandle
SceneHandle
ObjectHandle
```

不推荐在 public API 中长期暴露：

```rust
&GpuBuffer
&GpuTexture
&mut CommandEncoder
```

原因：GPU 资源生命周期跨帧，CPU borrow 生命周期是词法作用域，二者天生不同频。强行统一会让 API 变成生命周期迷宫。

### 4.3 描述式创建，命令式更新

创建资源使用 `Desc`：

```rust
let mesh = renderer.create_mesh(MeshDesc { ... })?;
let tex = renderer.create_texture(TextureDesc { ... })?;
let mat = renderer.create_material(MaterialDesc { ... })?;
```

运行时更新使用 update command：

```rust
renderer.update_texture(tex, TextureUpdate { ... })?;
renderer.update_material(mat, MaterialUpdate { ... })?;
let mat_info: MaterialInfo = renderer.material_info(mat).unwrap();
renderer.edit_scene(scene, |s| {
    s.set_transform(object, new_transform);
})?;
```

材质查询必须暴露 renderer 层真实状态，而不是只在 frame 阶段报错：

```rust
pub struct MaterialInfo {
    pub label: Option<String>,
    pub domain: MaterialDomain,
    pub template: Option<MaterialTemplateHandle>,
    pub template_ready: bool,
    pub is_standard: bool,
    pub parameter_count: usize,
    pub template_parameter_count: usize,
    pub material_covers_template: bool,
    pub shader_interface_layout_hash: u64,
    pub texture_bindings: usize,
    pub sampler_bindings: usize,
    pub material_covers_reflection: bool,
    pub missing_reflected_bindings: usize,
    pub missing_reflected_texture_bindings: usize,
    pub missing_reflected_sampler_bindings: usize,
    pub missing_reflected_buffer_bindings: usize,
    pub pipeline_ready: bool,
    pub status: ResourceStatus,
}

impl Renderer {
    pub fn material_info(&self, material: MaterialHandle) -> Option<MaterialInfo>;
}
```

`pipeline_ready` 必须由 material 自身资源状态和 material template 资源状态共同决定；template 被销毁后，material 仍可被查询，但 `template_ready` 和 `pipeline_ready` 必须变为 `false`。`MaterialInfo` 必须同时暴露 material 实例是否覆盖 template schema、是否覆盖 shader reflection、当前 shader interface layout hash，以及缺失 reflected binding 的总数和 texture/sampler/buffer 分类型数量。

material template 也必须可独立查询，以便编辑器、inspector、pipeline warmup 和错误定位能在提交 frame 前看到 shader/template 是否仍可用于 pipeline：

```rust
pub struct MaterialTemplateInfo {
    pub label: Option<String>,
    pub shader: ShaderHandle,
    pub shader_ready: bool,
    pub domain: MaterialDomain,
    pub render_state: RenderStateDesc,
    pub parameter_count: usize,
    pub shader_interface_layout_hash: u64,
    pub reflected_binding_count: usize,
    pub reflected_texture_bindings: usize,
    pub reflected_sampler_bindings: usize,
    pub reflected_buffer_bindings: usize,
    pub schema_covers_reflection: bool,
    pub missing_reflected_bindings: usize,
    pub missing_reflected_texture_bindings: usize,
    pub missing_reflected_sampler_bindings: usize,
    pub missing_reflected_buffer_bindings: usize,
    pub passes: MaterialPassFlags,
    pub pipeline_ready: bool,
    pub status: ResourceStatus,
}

impl Renderer {
    pub fn material_template_info(
        &self,
        template: MaterialTemplateHandle,
    ) -> Option<MaterialTemplateInfo>;
}
```

`pipeline_ready` 必须同时要求 template 自身为 `Ready` 且 shader 依赖为 `Ready`。shader 被销毁后，template 仍可查询，但 `shader_ready=false` 且 `pipeline_ready=false`；template 自身被销毁后，该 template handle 不再返回 `MaterialTemplateInfo`。

当 shader 提供 `ShaderInterfaceDesc.resources` 时，material template 的 `parameter_schema.parameters` 必须引用 shader reflection 中已声明的 binding 名称。texture / sampler / uniform / storage buffer binding 必须保持 binding class 与 binding type 一致；不在 shader interface 中的 schema 参数必须在 `create_material_template` 阶段返回 `MaterialParameterMismatch`。`MaterialTemplateInfo` 必须暴露 shader interface layout hash、reflection binding 总数、texture/sampler/buffer binding 数量、schema 是否覆盖全部 reflected binding，以及未被 schema 覆盖的 reflected binding 总数和 texture/sampler/buffer 分类型数量。当 shader reflection 被禁用或 interface 为空时，template 可以继续使用手写 schema，以兼容无反射材质。

```rust
pub struct MaterialReflectionCoverageStats {
    pub ready_material_templates: usize,
    pub pipeline_ready_material_templates: usize,
    pub reflected_material_templates: usize,
    pub reflection_covered_material_templates: usize,
    pub reflection_incomplete_material_templates: usize,
    pub missing_template_reflected_bindings: usize,
    pub missing_template_reflected_texture_bindings: usize,
    pub missing_template_reflected_sampler_bindings: usize,
    pub missing_template_reflected_buffer_bindings: usize,
    pub ready_materials: usize,
    pub template_ready_materials: usize,
    pub pipeline_ready_materials: usize,
    pub materials_with_shader_interface: usize,
    pub reflection_covered_materials: usize,
    pub reflection_incomplete_materials: usize,
    pub missing_material_reflected_bindings: usize,
    pub missing_material_reflected_texture_bindings: usize,
    pub missing_material_reflected_sampler_bindings: usize,
    pub missing_material_reflected_buffer_bindings: usize,
}

impl Renderer {
    pub fn material_reflection_coverage_stats(&self) -> MaterialReflectionCoverageStats;
}
```

`Renderer::material_reflection_coverage_stats()` 必须把 per-template/per-material reflection coverage 汇总成工具可直接消费的统计，包括 Ready template/material 数量、pipeline-ready 数量、shader-interface/material-template readiness、schema/material 覆盖 reflection 的数量、未覆盖 template/material 数量，以及缺失 reflected binding 的总数和 texture/sampler/buffer 分类型数量。该汇总必须与 `MaterialInfo` / `MaterialTemplateInfo` 的 per-resource 判断使用同一 source of truth，并传播到 `FrameStats`、`FrameDebugReport`、`FrameCapture` 和 `FrameCaptureResourceDump`，让 capture/debug artifact 能定位 material schema/reflection 缺口，而不必逐个遍历资源。

WGSL auto reflection 必须解析 `@group/@binding` 资源声明，并区分 `var<uniform>` uniform buffer、`var<storage>` / `var<storage, read>` storage buffer、sampled texture、storage texture 和 sampler。storage buffer 即使使用 struct 类型而不是 `array<>` / `atomic<>`，也必须被报告为 `BindingClass::Storage` + `BindingType::Buffer`。`texture_storage_*` 必须报告为 `BindingClass::Storage` + `BindingType::StorageTexture { dimension, format, access }`，material 参数绑定仍通过 `TextureHandle` 验证，backend layout planning 可直接创建 wgpu storage texture binding。

WGSL texture dimension 反射必须区分 `texture_1d` / `texture_storage_1d` 和 2D/cube/3D 类型，不能把 1D texture 默认归类为 2D。

WGSL auto reflection 还必须把 `var<push_constant>` 记录到 `ShaderInterfaceDesc.push_constants`。当前实现至少要能从简单 struct/scalar/vector/matrix 字段估算 byte range，并把 range 暴露给 shader info、hot reload compatibility、material/template 诊断和后续 pipeline layout 接线。

`ShaderSource::File` 指向 `.wgsl` 文件且使用 `ShaderReflectionMode::Auto` 时，必须走同一套 WGSL reflection 路径；文件源码中的 resource bindings、push constants 和 vertex inputs 都必须进入 `ShaderInterfaceDesc`，不能只支持内存 WGSL 字符串。

WGSL auto reflection 还必须解析 vertex entry point 参数中的 `@location(n) name: type`，并写入 `ShaderInterfaceDesc.vertex_inputs`。常见参数名要映射到内置语义，例如 `position`、`normal`、`uv0`、`color0`；未知参数名必须保留为 `VertexSemantic::Custom(location)`，避免丢失 pipeline layout 所需的输入要求。

`VertexFormat` 必须覆盖 wgpu 可直接表达且常用的 WGSL vertex input storage formats，包括 packed `u8/i8`、normalized `unorm/snorm`、`u16/i16` vector formats、`float16` vector formats，以及 scalar/vector `f32`、`u32`、`i32`。64-bit vertex attributes 必须通过 `RendererFeature::VertexAttribute64Bit` / `RendererFeatures::VERTEX_ATTRIBUTE_64BIT` gate 暴露；backend-wgpu 只有在 adapter/device 支持 `VERTEX_ATTRIBUTE_64BIT` 时才启用该 feature，否则使用 `Float64*` vertex formats 的 shader interface 必须返回 `UnsupportedFeature(VertexAttribute64Bit)`。WGSL auto reflection、shader interface layout hash、RHI pipeline mapping 和 backend-wgpu reflected draw vertex layout mapping 必须使用同一套 `VertexFormat`，避免 reflection 能识别但 backend 无法创建 native vertex layout。WGSL auto reflection 只能从 shader 参数类型推断 `f32/u32/i32` 类格式；packed、normalized、float16 和 float64 storage formats 必须通过 explicit reflection 或 mesh layout 显式表达。

`vertex_inputs` 只能来自 `ShaderEntryPoints::vertex` 指定的 vertex entry point。fragment entry 的 `@location` 输入/输出不能被记录为 vertex input，否则会污染 pipeline vertex layout。

vertex entry 使用输入 struct 时，WGSL auto reflection 必须解析 struct 成员上的 `@location`，并展开为 `ShaderInterfaceDesc.vertex_inputs`。例如 `fn vs_main(input: VertexInput)` 中 `VertexInput` 的 `position`、`uv0`、`joints0` 等成员必须映射到对应 `VertexSemantic` 和 `VertexFormat`。

shader hot reload 兼容性必须基于最终 `ShaderInterfaceDesc` 判断，因此 `ShaderReflectionMode::Auto` 产生的 resource bindings、push constants 和 vertex inputs 与 explicit reflection 一样，发生 layout 变化时必须拒绝 reload 并返回 shader compile error。vertex input struct 被展开后的 layout 也必须参与同一套兼容性检查。

### 4.4 Renderer 是稳定 facade，RHI 是可替换内核

Public API 不应依赖 Vulkan、D3D12、Metal、wgpu 中任何一个后端的专有语义。后端差异通过 `RendererCaps` 和 `FeatureFlags` 表达。

### 4.5 标准 3D 管线内置，扩展点显式

内置 pipeline 负责 90% 游戏渲染：

```text
Depth Prepass
Shadow Pass
GBuffer or Forward+ Depth
Lighting
Sky / IBL
Transparent
PostProcess
UI / Debug
Present
```

扩展点：

- custom material shader
- custom render graph pass
- custom post process
- custom light type, 可选
- custom render phase, 可选高级功能

---

## 5. 完整 3D 能力定义

本设计中的“完整 3D Renderer”至少包含以下能力。

### 5.1 基础 3D

- Perspective / Orthographic camera
- Reverse-Z 支持
- Multiple view / camera
- Surface render 和 offscreen render
- Mesh / submesh / index buffer / vertex layout
- Static mesh / dynamic mesh / instancing
- Frustum culling
- LOD group
- Debug draw

### 5.2 材质与光照

- Metallic-Roughness PBR
- Normal / ORM / Emissive / Alpha map
- IBL diffuse irradiance + specular prefilter
- Directional / Point / Spot light
- Cascaded Shadow Map
- Point light cube shadow
- Spot light shadow
- Area light 可作为高级扩展

### 5.3 渲染路径

- Deferred rendering
- Forward+ rendering
- Transparent forward pass
- Depth prepass
- Shadow pass
- Sky pass
- Post process chain

### 5.4 动画与变形

- CPU/GPU skinning
- Morph target
- Per-object motion vectors
- TAA 所需 previous transform

### 5.5 后处理

- HDR
- Tone mapping
- Bloom
- TAA / FXAA
- SSAO
- SSR, 可选
- DOF / motion blur, 可选
- Color grading LUT

### 5.6 未来高级功能

通过 `RendererCaps` 探测支持：

- Bindless texture
- GPU driven rendering
- Multi draw indirect
- Occlusion culling
- Meshlet / mesh shader
- Ray tracing
- Virtual texturing
- Async compute

---

## 6. 顶层 API

### 6.1 RendererConfig

```rust
#[derive(Clone, Debug)]
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
pub enum RenderPath {
    Auto,
    Deferred,
    ForwardPlus,
    Forward,
}
```

### 6.2 Renderer 初始化

```rust
pub struct Renderer {
    // private
}

impl Renderer {
    pub async fn new(config: RendererConfig) -> Result<Self, RendererError>;

    pub async fn with_surface<W>(
        config: RendererConfig,
        window: &W,
    ) -> Result<Self, RendererError>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle;

    pub fn capabilities(&self) -> &RendererCaps;
    pub fn config(&self) -> &RendererConfig;

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), RendererError>;
    pub fn set_vsync(&mut self, mode: VSyncMode) -> Result<(), RendererError>;
    pub fn device_status(&self) -> DeviceStatus;
}
```

### 6.3 能力查询

```rust
#[derive(Clone, Debug)]
pub struct RendererCaps {
    pub backend_name: String,
    pub adapter_name: String,
    pub features: RendererFeatures,
    pub limits: RendererLimits,
    pub formats: FormatCaps,
}

bitflags::bitflags! {
    pub struct RendererFeatures: u64 {
        const COMPUTE               = 1 << 0;
        const INDIRECT_DRAW          = 1 << 1;
        const MULTI_DRAW_INDIRECT    = 1 << 2;
        const BINDLESS_TEXTURES      = 1 << 3;
        const STORAGE_TEXTURES       = 1 << 4;
        const TIMESTAMP_QUERY        = 1 << 5;
        const PIPELINE_STATISTICS    = 1 << 6;
        const ASYNC_COMPUTE          = 1 << 7;
        const RAY_TRACING            = 1 << 8;
        const MESH_SHADER            = 1 << 9;
        const VARIABLE_RATE_SHADING  = 1 << 10;
    }
}
```

---

## 7. Handle 系统

### 7.1 类型安全 handle

```rust
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    raw: std::num::NonZeroU64,
    _marker: std::marker::PhantomData<fn() -> T>,
}

pub enum MeshTag {}
pub enum TextureTag {}
pub enum MaterialTag {}
pub enum ShaderTag {}
pub enum SceneTag {}
pub enum ObjectTag {}
pub enum CameraTag {}
pub enum LightTag {}
pub enum EnvironmentTag {}
pub enum RenderTargetTag {}

pub type MeshHandle = Handle<MeshTag>;
pub type TextureHandle = Handle<TextureTag>;
pub type MaterialHandle = Handle<MaterialTag>;
pub type ShaderHandle = Handle<ShaderTag>;
pub type SceneHandle = Handle<SceneTag>;
pub type ObjectHandle = Handle<ObjectTag>;
pub type CameraHandle = Handle<CameraTag>;
pub type LightHandle = Handle<LightTag>;
pub type EnvironmentHandle = Handle<EnvironmentTag>;
pub type RenderTargetHandle = Handle<RenderTargetTag>;
```

`raw` 建议编码：

```text
bits 0..31   index
bits 32..55  generation
bits 56..63  resource kind / debug tag
```

### 7.2 ResourceStatus

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceStatus {
    PendingUpload,
    Ready,
    Failed,
    Evicted,
    DestroyQueued,
}

impl Renderer {
    pub fn resource_status<T>(&self, handle: Handle<T>) -> Option<ResourceStatus>;
    pub fn destroy<T>(&mut self, handle: Handle<T>) -> Result<(), RendererError>;
}
```

资源销毁不应立即 free GPU memory，而是进入 delayed destroy queue：

```text
destroy(handle)
    -> mark generation invalid
    -> push gpu resource into garbage list with frame_index
    -> after N frames or fence signaled
    -> actual backend destroy
```

---

## 8. Mesh API

### 8.1 MeshDesc

```rust
#[derive(Clone, Debug)]
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

pub enum VertexData<'a> {
    Interleaved(&'a [u8]),
    Streams(Vec<VertexStream<'a>>),
}

pub enum IndexData<'a> {
    U16(&'a [u16]),
    U32(&'a [u32]),
}

#[derive(Clone, Debug)]
pub struct VertexLayout {
    pub streams: Vec<VertexStreamLayout>,
}

#[derive(Clone, Debug)]
pub struct VertexStreamLayout {
    pub stride: u32,
    pub step: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct SubMeshDesc {
    pub index_range: std::ops::Range<u32>,
    pub vertex_range: std::ops::Range<u32>,
    pub material_slot: u16,
    pub bounds: Bounds3,
}

bitflags::bitflags! {
    pub struct MeshUsage: u32 {
        const STATIC        = 1 << 0;
        const DYNAMIC       = 1 << 1;
        const STREAMING     = 1 << 2;
        const CPU_READBACK  = 1 << 3;
        const RAY_TRACING   = 1 << 4;
    }
}
```

### 8.2 Mesh 创建与更新

```rust
impl Renderer {
    pub fn create_mesh(&mut self, desc: MeshDesc<'_>) -> Result<MeshHandle, RendererError>;

    pub fn update_mesh_vertices(
        &mut self,
        mesh: MeshHandle,
        stream: u32,
        byte_offset: u64,
        data: &[u8],
    ) -> Result<(), RendererError>;

    pub fn update_mesh_indices(
        &mut self,
        mesh: MeshHandle,
        byte_offset: u64,
        data: &[u8],
    ) -> Result<(), RendererError>;

    pub fn mesh_info(&self, mesh: MeshHandle) -> Option<MeshInfo>;
}
```

设计要求：

- `STATIC` mesh 默认上传后 CPU 数据可释放。
- `DYNAMIC` mesh 内部使用 ring buffer 或 staging upload。
- `STREAMING` mesh 支持多帧分块上传。
- `RAY_TRACING` mesh 在支持时自动构建 BLAS。

---

## 9. Texture / Sampler API

### 9.1 TextureDesc

```rust
#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
    D2Array,
    CubeArray,
}

bitflags::bitflags! {
    pub struct TextureUsage: u32 {
        const SAMPLED          = 1 << 0;
        const RENDER_TARGET    = 1 << 1;
        const DEPTH_STENCIL    = 1 << 2;
        const STORAGE          = 1 << 3;
        const COPY_SRC         = 1 << 4;
        const COPY_DST         = 1 << 5;
        const PRESENT          = 1 << 6;
    }
}
```

### 9.2 Texture 创建与更新

```rust
impl Renderer {
    pub fn create_texture(&mut self, desc: TextureDesc<'_>) -> Result<TextureHandle, RendererError>;

    pub fn update_texture(
        &mut self,
        texture: TextureHandle,
        update: TextureUpdate<'_>,
    ) -> Result<(), RendererError>;

    pub fn texture_info(&self, texture: TextureHandle) -> Option<TextureInfo>;
    pub fn texture_bytes(&self, texture: TextureHandle) -> Option<&[u8]>;
    pub fn generate_mips(&mut self, texture: TextureHandle) -> Result<(), RendererError>;
}

pub struct TextureInfo {
    pub label: Option<String>,
    pub dimension: TextureDimension,
    pub width: u32,
    pub height: u32,
    pub depth_or_layers: u32,
    pub mip_levels: u32,
    /// True when the current mip chain was generated by Renderer::generate_mips.
    pub mips_generated: bool,
    pub samples: u32,
    pub format: TextureFormat,
    pub usage: TextureUsage,
    pub status: ResourceStatus,
}

pub struct TextureUpdate<'a> {
    pub subresource: TextureSubresource,
    pub region: TextureRegion,
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
    pub data: &'a [u8],
}
```

### 9.3 Sampler

```rust
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

pub type SamplerHandle = Handle<SamplerTag>;

pub struct SamplerConfigurationStats {
    pub comparison_samplers: usize,
    pub anisotropic_samplers: usize,
    pub custom_lod_samplers: usize,
}

pub struct TextureConfigurationStats {
    pub sampled_textures: usize,
    pub render_target_textures: usize,
    pub copy_src_textures: usize,
    pub multi_mip_textures: usize,
    pub generated_mip_textures: usize,
    pub multisampled_textures: usize,
}

impl Renderer {
    pub fn create_sampler(&mut self, desc: SamplerDesc) -> Result<SamplerHandle, RendererError>;
    pub fn texture_configuration_stats(&self) -> TextureConfigurationStats;
    pub fn sampler_configuration_stats(&self) -> SamplerConfigurationStats;
}
```

---

## 10. Shader API

Shader API 要同时服务：

1. 标准 PBR 材质。
2. 自定义材质。
3. 后处理 pass。
4. Compute pass。
5. shader hot reload。
6. pipeline variant cache。

### 10.1 ShaderDesc

```rust
#[derive(Clone, Debug)]
pub struct ShaderDesc<'a> {
    pub label: Option<&'a str>,
    pub source: ShaderSource<'a>,
    pub stages: ShaderStages,
    pub entry_points: ShaderEntryPoints<'a>,
    pub reflection: ShaderReflectionMode,
    pub features: ShaderFeatureSet,
    pub hot_reload_key: Option<String>,
}

pub enum ShaderSource<'a> {
    Wgsl(&'a str),
    SpirV(&'a [u32]),
    Msl(&'a str),
    Hlsl(&'a str),
    Slang(&'a str),
    File(std::path::PathBuf),
}

bitflags::bitflags! {
    pub struct ShaderStages: u32 {
        const VERTEX   = 1 << 0;
        const FRAGMENT = 1 << 1;
        const COMPUTE  = 1 << 2;
        const MESH     = 1 << 3;
        const TASK     = 1 << 4;
        const RAYGEN   = 1 << 5;
        const MISS     = 1 << 6;
        const CLOSEST_HIT = 1 << 7;
    }
}

pub enum ShaderReflectionMode {
    Auto,
    Explicit(ShaderInterfaceDesc),
    Disabled,
}
```

### 10.2 Shader 创建

```rust
impl Renderer {
    pub fn create_shader(&mut self, desc: ShaderDesc<'_>) -> Result<ShaderHandle, RendererError>;
    pub fn reload_shader(&mut self, shader: ShaderHandle) -> Result<(), RendererError>;
    pub fn shader_info(&self, shader: ShaderHandle) -> Option<ShaderInfo>;
    pub fn warm_up_shader_variants(&mut self, requests: &[ShaderVariantWarmupRequest]) -> Result<(), RendererError>;
    pub fn shader_variant_info(&self, shader: ShaderHandle, features: &ShaderFeatureSet) -> Option<ShaderVariantInfo>;
    pub fn shader_variant_cache_entries(&self) -> Vec<ShaderVariantInfo>;
}

pub struct ShaderVariantWarmupRequest {
    pub shader: ShaderHandle,
    pub features: ShaderFeatureSet,
}

pub struct ShaderVariantInfo {
    pub shader: ShaderHandle,
    pub features: ShaderFeatureSet,
    pub shader_interface_layout_hash: u64,
    pub backend_compiled: bool,
    pub last_used_frame: Option<u64>,
    pub used_this_frame: bool,
}
```

Shader variant warmup canonicalizes feature flags, rejects duplicate/blank flags, rejects feature requests not declared by the shader's `ShaderFeatureSet`, records the shader interface layout hash for the cached variant, and exposes per-frame use state for editor/cache diagnostics. When a wgpu runtime exists, warmup also compiles and caches a native WGSL shader module for the requested variant and reports that through `ShaderVariantInfo::backend_compiled`. `FrameStats`, `FrameDebugReport`, and `FrameCapture` mirror aggregate shader variant cache entry count, variants used this frame, ready-but-unused variant count, backend-compiled variant count, variants without backend modules, and unique shader interface layout count so frame tooling and capture artifacts can inspect variant cache pressure without enumerating every variant. Shader reload and shader destroy invalidate matching variant cache entries and backend-wgpu cached variant shader modules. Invalidated backend-wgpu shader variant modules are moved into backend resource tombstones and retired through `poll_backend_resource_retirements()` rather than being dropped immediately.

```rust
pub struct ShaderVariantCacheStats {
    pub entries: usize,
    pub used_this_frame: usize,
    pub ready_unused: usize,
    pub backend_compiled: usize,
    pub without_backend_module: usize,
    pub interface_layouts: usize,
}

impl Renderer {
    pub fn shader_variant_cache_stats(&self) -> ShaderVariantCacheStats;
}
```

### 10.3 Shader interface

Renderer 内部使用 reflection 或 explicit layout 生成 material schema：

```rust
#[derive(Clone, Debug)]
pub struct ShaderInterfaceDesc {
    pub resources: Vec<ShaderResourceBinding>,
    pub push_constants: Vec<PushConstantRange>,
    pub vertex_inputs: Vec<VertexInputRequirement>,
}

#[derive(Clone, Debug)]
pub struct ShaderResourceBinding {
    pub name: String,
    pub group: u32,
    pub binding: u32,
    pub binding_class: BindingClass,
    pub visibility: ShaderStages,
    pub ty: BindingType,
}

pub enum BindingType {
    Buffer,
    Texture(TextureDimension),
    StorageTexture {
        dimension: TextureDimension,
        format: TextureFormat,
        access: StorageTextureAccess,
    },
    Sampler,
}

pub enum StorageTextureAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}
```

游戏层仍然不应该直接绑定 binding。这个信息用于：

- 创建 pipeline layout
- 校验 material 参数
- 自动生成 UI inspector
- shader hot reload 时做兼容性检查

---

## 11. Material API

Material 分为两层：

```text
MaterialTemplate
    描述 shader、render state、参数 schema、pass 行为

MaterialInstance
    一份具体参数，引用 template
```

### 11.1 标准材质

```rust
#[derive(Clone, Debug)]
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

impl Renderer {
    pub fn create_standard_material(
        &mut self,
        desc: StandardMaterialDesc,
    ) -> Result<MaterialHandle, RendererError>;
}
```

### 11.2 自定义材质

```rust
#[derive(Clone, Debug)]
pub struct MaterialTemplateDesc {
    pub label: Option<String>,
    pub shader: ShaderHandle,
    pub domain: MaterialDomain,
    pub render_state: RenderStateDesc,
    pub parameter_schema: MaterialParameterSchema,
    pub passes: MaterialPassFlags,
}

bitflags::bitflags! {
    pub struct MaterialPassFlags: u32 {
        const DEPTH_PREPASS = 1 << 0;
        const SHADOW       = 1 << 1;
        const GBUFFER      = 1 << 2;
        const FORWARD      = 1 << 3;
        const TRANSPARENT  = 1 << 4;
        const MOTION       = 1 << 5;
        const PICKING      = 1 << 6;
    }
}

pub type MaterialTemplateHandle = Handle<MaterialTemplateTag>;

impl Renderer {
    pub fn create_material_template(
        &mut self,
        desc: MaterialTemplateDesc,
    ) -> Result<MaterialTemplateHandle, RendererError>;

    pub fn create_material(
        &mut self,
        desc: MaterialDesc,
    ) -> Result<MaterialHandle, RendererError>;
}

#[derive(Clone, Debug)]
pub struct MaterialDesc {
    pub label: Option<String>,
    pub template: MaterialTemplateHandle,
    pub parameters: MaterialParameters,
    pub overrides: MaterialOverrides,
}
```

### 11.3 Material 参数更新

```rust
impl Renderer {
    pub fn update_material(
        &mut self,
        material: MaterialHandle,
        update: MaterialUpdate,
    ) -> Result<(), RendererError>;
}

#[derive(Clone, Debug)]
pub enum MaterialUpdate {
    SetFloat(String, f32),
    SetVec2(String, Vec2),
    SetVec3(String, Vec3),
    SetVec4(String, Vec4),
    SetMat4(String, Mat4),
    SetColor(String, Color),
    SetTexture(String, Option<TextureHandle>),
    SetSampler(String, Option<SamplerHandle>),
    SetBytes(String, Vec<u8>),
    ReplaceAll(MaterialParameters),
}
```

对性能敏感路径，提供 interned parameter id：

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialParamId(u32);

impl Renderer {
    pub fn intern_material_param(&mut self, name: &str) -> MaterialParamId;

    pub fn update_material_fast(
        &mut self,
        material: MaterialHandle,
        param: MaterialParamId,
        value: MaterialValue,
    ) -> Result<(), RendererError>;
}
```

---

## 12. Scene API

Scene API 采用 retained mode：Renderer 维护一个 render-only scene。ECS 主世界通过 extract/update 把可渲染数据同步过来。

### 12.1 Scene 创建

```rust
#[derive(Clone, Debug, Default)]
pub struct SceneDesc {
    pub label: Option<String>,
    pub max_objects_hint: Option<u32>,
    pub max_lights_hint: Option<u32>,
    pub enable_gpu_culling: bool,
    pub enable_occlusion_culling: bool,
}

impl Renderer {
    pub fn create_scene(&mut self, desc: SceneDesc) -> Result<SceneHandle, RendererError>;

    pub fn edit_scene<R>(
        &mut self,
        scene: SceneHandle,
        f: impl FnOnce(&mut SceneWriter<'_>) -> R,
    ) -> Result<R, RendererError>;

    pub fn apply_scene_commands(
        &mut self,
        commands: SceneCommandBuffer,
    ) -> Result<(), RendererError>;
}
```

### 12.2 RenderObjectDesc

```rust
#[derive(Clone, Debug)]
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

bitflags::bitflags! {
    pub struct VisibilityFlags: u32 {
        const CAMERA      = 1 << 0;
        const SHADOW      = 1 << 1;
        const REFLECTION  = 1 << 2;
        const PICKING     = 1 << 3;
    }
}

bitflags::bitflags! {
    pub struct ObjectFlags: u32 {
        const STATIC          = 1 << 0;
        const DYNAMIC         = 1 << 1;
        const CAST_SHADOW     = 1 << 2;
        const RECEIVE_SHADOW  = 1 << 3;
        const MOTION_VECTORS  = 1 << 4;
        const GPU_CULLABLE    = 1 << 5;
        const NO_BATCH        = 1 << 6;
    }
}
```

### 12.3 SceneWriter

```rust
pub struct SceneWriter<'a> {
    // private
}

impl<'a> SceneWriter<'a> {
    pub fn spawn(&mut self, desc: RenderObjectDesc) -> ObjectHandle;
    pub fn despawn(&mut self, object: ObjectHandle) -> Result<(), RendererError>;

    pub fn set_transform(&mut self, object: ObjectHandle, transform: Mat4) -> Result<(), RendererError>;
    pub fn set_previous_transform(&mut self, object: ObjectHandle, transform: Mat4) -> Result<(), RendererError>;
    pub fn set_mesh(&mut self, object: ObjectHandle, mesh: MeshHandle) -> Result<(), RendererError>;
    pub fn set_material(&mut self, object: ObjectHandle, slot: usize, material: MaterialHandle) -> Result<(), RendererError>;
    pub fn set_visibility(&mut self, object: ObjectHandle, flags: VisibilityFlags) -> Result<(), RendererError>;
    pub fn set_layer(&mut self, object: ObjectHandle, layer: RenderLayer) -> Result<(), RendererError>;
    pub fn set_bounds(&mut self, object: ObjectHandle, bounds: Bounds3) -> Result<(), RendererError>;

    pub fn add_light(&mut self, desc: LightDesc) -> LightHandle;
    pub fn update_light(&mut self, light: LightHandle, update: LightUpdate) -> Result<(), RendererError>;
    pub fn remove_light(&mut self, light: LightHandle) -> Result<(), RendererError>;

    pub fn set_environment(&mut self, env: Option<EnvironmentHandle>) -> Result<(), RendererError>;
}
```

### 12.4 SceneCommandBuffer

多线程 ECS extract 可用 command buffer，避免多个系统同时借用 Renderer：

```rust
pub struct SceneCommandBuffer {
    scene: SceneHandle,
    commands: Vec<SceneCommand>,
}

pub enum SceneCommand {
    Spawn(RenderObjectDesc, ObjectHandle),
    Despawn(ObjectHandle),
    SetTransform(ObjectHandle, Mat4),
    SetMaterial(ObjectHandle, usize, MaterialHandle),
    SetVisibility(ObjectHandle, VisibilityFlags),
    AddLight(LightDesc, LightHandle),
    UpdateLight(LightHandle, LightUpdate),
    RemoveLight(LightHandle),
}
```

---

## 13. Camera / View API

Camera 是视图参数，View 是一次渲染请求。

### 13.1 CameraDesc

```rust
#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

bitflags::bitflags! {
    pub struct CameraFlags: u32 {
        const MAIN              = 1 << 0;
        const ENABLE_TAA        = 1 << 1;
        const ENABLE_BLOOM      = 1 << 2;
        const ENABLE_SSAO       = 1 << 3;
        const ENABLE_SKY        = 1 << 4;
        const ENABLE_DEBUG_DRAW = 1 << 5;
    }
}
```

### 13.2 ViewDesc

```rust
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

#[derive(Clone, Debug)]
pub enum RenderTarget {
    MainSurface,
    Surface(SurfaceHandle),
    Texture(TextureHandle),
    TextureView(TextureViewDesc),
    Headless { width: u32, height: u32, format: TextureFormat },
}
```

---

## 14. Light / Shadow / Environment API

### 14.1 LightDesc

```rust
#[derive(Clone, Debug)]
pub enum LightDesc {
    Directional(DirectionalLightDesc),
    Point(PointLightDesc),
    Spot(SpotLightDesc),
    Area(AreaLightDesc),
}

#[derive(Clone, Debug)]
pub struct DirectionalLightDesc {
    pub label: Option<String>,
    pub direction: Vec3,
    pub color: Color,
    pub illuminance_lux: f32,
    pub shadow: Option<DirectionalShadowDesc>,
    pub layer_mask: RenderLayerMask,
}

#[derive(Clone, Debug)]
pub struct PointLightDesc {
    pub label: Option<String>,
    pub position: Vec3,
    pub color: Color,
    pub intensity_lumen: f32,
    pub radius: f32,
    pub shadow: Option<PointShadowDesc>,
    pub layer_mask: RenderLayerMask,
}

#[derive(Clone, Debug)]
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
```

### 14.2 Shadow

```rust
#[derive(Clone, Debug)]
pub struct DirectionalShadowDesc {
    pub resolution: u32,
    pub cascades: u8,
    pub max_distance: f32,
    pub split_lambda: f32,
    pub filter: ShadowFilter,
    pub bias: ShadowBias,
}

#[derive(Clone, Debug)]
pub enum ShadowFilter {
    Hard,
    Pcf { taps: u8 },
    Evsm,
    Vsm,
}

#[derive(Clone, Debug)]
pub struct ShadowBias {
    pub constant: f32,
    pub slope: f32,
    pub normal: f32,
}
```

### 14.3 Environment / IBL

```rust
#[derive(Clone, Debug)]
pub struct EnvironmentDesc {
    pub label: Option<String>,
    pub skybox: Option<TextureHandle>,
    pub irradiance: Option<TextureHandle>,
    pub prefiltered_specular: Option<TextureHandle>,
    pub brdf_lut: Option<TextureHandle>,
    pub intensity: f32,
    pub rotation: Quat,
}

impl Renderer {
    pub fn create_environment(&mut self, desc: EnvironmentDesc) -> Result<EnvironmentHandle, RendererError>;

    pub fn bake_environment(
        &mut self,
        source: TextureHandle,
        desc: EnvironmentBakeDesc,
    ) -> Result<EnvironmentHandle, RendererError>;
}
```

---

## 15. Animation / Skinning API

Renderer 不管理动画状态机，但需要接收骨骼矩阵、morph weight、previous transform。

```rust
pub type SkeletonInstanceHandle = Handle<SkeletonInstanceTag>;
pub type MorphWeightsHandle = Handle<MorphWeightsTag>;

#[derive(Clone, Debug)]
pub struct SkeletonInstanceDesc<'a> {
    pub label: Option<&'a str>,
    pub joint_matrices: &'a [Mat4],
    pub inverse_bind_matrices: Option<&'a [Mat4]>,
    pub usage: AnimationDataUsage,
}

impl Renderer {
    pub fn create_skeleton_instance(
        &mut self,
        desc: SkeletonInstanceDesc<'_>,
    ) -> Result<SkeletonInstanceHandle, RendererError>;

    pub fn update_skeleton_joints(
        &mut self,
        skeleton: SkeletonInstanceHandle,
        joint_matrices: &[Mat4],
    ) -> Result<(), RendererError>;

    pub fn create_morph_weights(
        &mut self,
        weights: &[f32],
    ) -> Result<MorphWeightsHandle, RendererError>;

    pub fn update_morph_weights(
        &mut self,
        handle: MorphWeightsHandle,
        weights: &[f32],
    ) -> Result<(), RendererError>;
}
```

---

## 16. Frame API

### 16.1 每帧入口

```rust
impl Renderer {
    pub fn begin_frame(&mut self, input: FrameInput) -> Result<Frame<'_>, RendererError>;
}

pub struct FrameInput {
    pub delta_time: f32,
    pub absolute_time: f64,
    pub frame_index_override: Option<u64>,
    pub wait_for_gpu: bool,
}

pub struct Frame<'a> {
    // private, mutably borrows Renderer
}

impl<'a> Frame<'a> {
    pub fn render_view(&mut self, view: ViewDesc) -> Result<ViewHandle, RendererError>;

    pub fn add_graph_extension(
        &mut self,
        extension: impl RenderGraphExtension + 'static,
    ) -> Result<(), RendererError>;

    pub fn debug_draw(&mut self) -> DebugDraw<'_>;

    pub fn finish(self) -> Result<FrameStats, RendererError>;
}

pub enum EditorGizmoKind {
    Translate,
    Rotate,
    Scale,
}

impl<'a> DebugDraw<'a> {
    pub fn editor_gizmo(&mut self, transform: Mat4, kind: EditorGizmoKind, size: f32);
    pub fn scene_object_gizmo(
        &mut self,
        scene: SceneHandle,
        object: ObjectHandle,
        kind: EditorGizmoKind,
        size: f32,
    ) -> Result<(), RendererError>;
    pub fn translation_gizmo(&mut self, transform: Mat4, size: f32);
    pub fn rotation_gizmo(&mut self, transform: Mat4, size: f32);
    pub fn scale_gizmo(&mut self, transform: Mat4, size: f32);
}

pub struct FrameEditorGizmoOutput {
    pub scene: SceneHandle,
    pub object: ObjectHandle,
    pub kind: EditorGizmoKind,
}

pub struct FrameDebugDrawOutput {
    pub command_count: u32,
    pub primitive_command_count: u32,
    pub text_command_count: u32,
    pub editor_gizmo_count: u32,
    pub pickable_editor_gizmo_count: u32,
    pub pickable_editor_gizmos: Vec<FrameEditorGizmoOutput>,
}
```

### 16.2 使用示例

```rust
let mut frame = renderer.begin_frame(FrameInput {
    delta_time,
    absolute_time,
    frame_index_override: None,
    wait_for_gpu: false,
})?;

frame.render_view(ViewDesc {
    label: Some("main_view".into()),
    scene,
    camera: CameraDesc {
        transform: camera_transform,
        projection: Projection::Perspective {
            vertical_fov: 60.0_f32.to_radians(),
            aspect: width as f32 / height as f32,
            near: 0.05,
            far: None,
            reverse_z: true,
        },
        exposure: Exposure::Auto,
        clear: ClearOptions::ColorDepth(Color::BLACK),
        viewport: None,
        scissor: None,
        jitter: taa_jitter,
        previous_view_proj,
        flags: CameraFlags::MAIN | CameraFlags::ENABLE_TAA | CameraFlags::ENABLE_BLOOM,
        label: None,
    },
    target: RenderTarget::MainSurface,
    render_path: RenderPath::Deferred,
    quality: ViewQualitySettings::high(),
    layers: RenderLayerMask::all(),
    graph_extensions: vec![],
})?;

let stats = frame.finish()?;
```

---

## 17. RenderGraph API

RenderGraph 是 renderer 的心脏。它不是“用户每帧手写 Vulkan 命令”，而是“用户或内置管线声明 pass、资源读写和执行逻辑，由 graph compiler 负责编译”。

### 17.1 Graph resource

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphTexture(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphBuffer(u32);

#[derive(Clone, Debug)]
pub enum GraphResource {
    Texture(GraphTexture),
    Buffer(GraphBuffer),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledResourceExport {
    pub resource: GraphResource,
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RhiResourceExports {
    pub textures: Vec<RhiTextureExport>,
    pub buffers: Vec<RhiBufferExport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiTextureExport {
    pub graph: GraphTexture,
    pub label: String,
    pub texture: RhiTexture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RhiBufferExport {
    pub graph: GraphBuffer,
    pub label: String,
    pub buffer: RhiBuffer,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RhiGraphExecution {
    pub stats: RenderGraphStats,
    pub exports: RhiResourceExports,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderGraphResourceLabels {
    pub textures: Vec<String>,
    pub buffers: Vec<String>,
}
```

### 17.2 RenderGraphBuilder

```rust
pub struct RenderGraphBuilder<'a> {
    // private
}

impl<'a> RenderGraphBuilder<'a> {
    pub fn import_texture(
        &mut self,
        name: impl Into<String>,
        texture: TextureHandle,
        usage: GraphTextureUsage,
    ) -> GraphTexture;

    pub fn create_texture(
        &mut self,
        name: impl Into<String>,
        desc: TextureDesc<'static>,
    ) -> GraphTexture;

    pub fn import_buffer(
        &mut self,
        name: impl Into<String>,
        buffer: BufferHandle,
        usage: GraphBufferUsage,
    ) -> GraphBuffer;

    pub fn export_texture(
        &mut self,
        name: impl Into<String>,
        texture: GraphTexture,
    ) -> GraphTexture;

    pub fn export_buffer(
        &mut self,
        name: impl Into<String>,
        buffer: GraphBuffer,
    ) -> GraphBuffer;

    pub fn execute_on_rhi_with_exports(
        &mut self,
        frame_index: u64,
        caps: &RendererCaps,
        view: Option<ViewInfo>,
        device: &dyn RhiDevice,
    ) -> Result<RhiGraphExecution, RendererError>;

    pub fn create_buffer(
        &mut self,
        name: impl Into<String>,
        desc: BufferDesc,
    ) -> GraphBuffer;

    pub fn add_pass<'p>(
        &'p mut self,
        name: impl Into<String>,
    ) -> PassBuilder<'p>;
}
```

### 17.3 PassBuilder

```rust
pub struct PassBuilder<'a> {
    // private
}

impl<'a> PassBuilder<'a> {
    pub fn queue(self, queue: QueueType) -> Self;

    pub fn read_texture(
        self,
        texture: GraphTexture,
        usage: TextureReadUsage,
    ) -> Self;

    pub fn write_texture(
        self,
        texture: GraphTexture,
        usage: TextureWriteUsage,
    ) -> Self;

    pub fn read_buffer(
        self,
        buffer: GraphBuffer,
        usage: BufferReadUsage,
    ) -> Self;

    pub fn write_buffer(
        self,
        buffer: GraphBuffer,
        usage: BufferWriteUsage,
    ) -> Self;

    pub fn color_attachment(
        self,
        texture: GraphTexture,
        ops: ColorAttachmentOps,
    ) -> Self;

    pub fn depth_attachment(
        self,
        texture: GraphTexture,
        ops: DepthAttachmentOps,
    ) -> Self;

    pub fn depends_on(self, pass: PassId) -> Self;

    pub fn execute(
        self,
        callback: impl for<'ctx> FnOnce(&mut PassContext<'ctx>) -> Result<(), RendererError>
            + Send
            + 'static,
    ) -> PassId;
}
```

注意：真实实现中如果 `FnOnce + 'static` 不适合存储，可改为 `Box<dyn RenderPassNode>`：

```rust
pub trait RenderPassNode: Send + Sync + 'static {
    fn setup(&self, builder: &mut PassBuilder<'_>);
    fn execute(&self, ctx: &mut PassContext<'_>) -> Result<(), RendererError>;
}
```

更推荐插件系统使用 trait，工具脚本或测试使用 closure。

### 17.4 PassContext

```rust
pub struct PassContext<'a> {
    // private
}

impl<'a> PassContext<'a> {
    pub fn frame_index(&self) -> u64;
    pub fn view(&self) -> Option<ViewInfo>;
    pub fn renderer_caps(&self) -> &RendererCaps;

    pub fn texture(&self, texture: GraphTexture) -> TextureViewRef<'_>;
    pub fn buffer(&self, buffer: GraphBuffer) -> BufferRef<'_>;

    pub fn draw_render_phase(&mut self, phase: RenderPhaseId) -> Result<(), RendererError>;

    pub fn begin_render_pass(&mut self, desc: RenderPassDesc<'_>) -> RenderPassEncoder<'_>;
    pub fn begin_compute_pass(&mut self, desc: ComputePassDesc<'_>) -> ComputePassEncoder<'_>;

    pub fn push_debug_group(&mut self, label: &str);
    pub fn pop_debug_group(&mut self);
}
```

高级插件作者可以在 `PassContext` 中调用受控的 encoder，但无法破坏 graph 已声明的资源依赖。

### 17.5 RenderGraphExtension

```rust
pub trait RenderGraphExtension: Send + Sync + 'static {
    fn name(&self) -> &str;

    fn build(
        &self,
        ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError>;
}
```

用途：

- 自定义后处理
- editor outline pass
- object picking pass
- GPU particle pass
- terrain clipmap pass
- debug visualization pass
- custom atmospheric scattering pass

---

## 18. 标准 3D RenderGraph

### 18.1 Deferred 默认图

```text
Import Backbuffer
Import / Create Main Depth

[Upload / Prepare GPU Data]
        │
        ▼
[GPU Culling Compute] optional
        │
        ▼
[Shadow CSM Pass]
[Shadow Point/Spot Pass]
        │
        ▼
[Depth Prepass]
        │
        ▼
[GBuffer Pass]
        │
        ▼
[SSAO Pass] optional
        │
        ▼
[Deferred Lighting Pass]
        │
        ▼
[Sky Pass]
        │
        ▼
[Forward Transparent Pass]
        │
        ▼
[Motion Vector Pass]
        │
        ▼
[TAA Pass]
        │
        ▼
[Bloom Pass]
        │
        ▼
[Tonemap / Color Grade]
        │
        ▼
[Debug / Gizmo / UI]
        │
        ▼
Present
```

### 18.2 Forward+ 默认图

```text
Depth Prepass
        │
        ▼
Light Cluster Build Compute
        │
        ▼
Forward Opaque
        │
        ▼
Sky
        │
        ▼
Transparent
        │
        ▼
PostProcess
        │
        ▼
Present
```

### 18.3 Render phase

Renderer 内部使用 phase 概念分类和排序 draw item：

```rust
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
}
```

排序策略：

```rust
pub enum PhaseSortMode {
    FrontToBack,
    BackToFront,
    MaterialThenMesh,
    PipelineThenMaterialThenMesh,
    Unsorted,
}
```

内部 draw item：

```rust
pub struct DrawItem {
    pub object: ObjectHandle,
    pub mesh: MeshHandle,
    pub submesh_index: u32,
    pub material: MaterialHandle,
    pub pipeline_key: PipelineKey,
    pub sort_key: u64,
    pub instance_range: std::ops::Range<u32>,
}
```

---

## 19. RHI API 设计

RHI 是 backend 抽象层。它应尽量小，不要把上层变成“换皮 Vulkan”。

### 19.1 RHI 边界

RHI 只负责：

- Device / Queue
- Buffer / Texture / Sampler
- Shader module
- Pipeline
- Command encoding
- Surface / Swapchain
- Barrier / resource state, 可由 RenderGraph 生成
- Queries / debug labels

RHI 不负责：

- PBR
- Material parameter schema
- Render object sorting
- Scene culling
- Asset loading

### 19.2 RHI trait 草案

```rust
pub trait RhiDevice: Send + Sync {
    fn caps(&self) -> &RhiCaps;

    fn create_buffer(&self, desc: &RhiBufferDesc) -> Result<RhiBuffer, RhiError>;
    fn create_texture(&self, desc: &RhiTextureDesc) -> Result<RhiTexture, RhiError>;
    fn create_sampler(&self, desc: &RhiSamplerDesc) -> Result<RhiSampler, RhiError>;
    fn create_shader_module(&self, desc: &RhiShaderModuleDesc) -> Result<RhiShaderModule, RhiError>;
    fn create_graphics_pipeline(&self, desc: &RhiGraphicsPipelineDesc) -> Result<RhiGraphicsPipeline, RhiError>;
    fn create_compute_pipeline(&self, desc: &RhiComputePipelineDesc) -> Result<RhiComputePipeline, RhiError>;

    fn create_command_encoder(&self, label: Option<&str>) -> Result<Box<dyn RhiCommandEncoder>, RhiError>;
    fn submit(&self, commands: Vec<RhiCommandBuffer>) -> Result<SubmissionIndex, RhiError>;

    fn poll(&self, mode: PollMode);
}
```

### 19.3 RHI 不直接暴露给游戏层

`engine_renderer_rhi` 可以对高级插件开放：

```rust
pub struct RhiAccess<'a> {
    device: &'a dyn RhiDevice,
    // restricted
}
```

但默认 `engine_renderer::prelude` 不导出 RHI 类型。

---

## 20. Pipeline / PipelineKey

Pipeline 不应该由游戏层频繁创建。Material、shader、vertex layout、render state 共同生成 `PipelineKey`，由 Renderer 内部 cache。

```rust
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
```

Pipeline cache：

```rust
pub struct PipelineCacheStats {
    pub total: usize,
    pub ready: usize,
    pub compiling: usize,
    pub failed: usize,
    /// Native backend render pipeline objects observed by the active backend.
    pub backend_objects: usize,
    pub shader_interface_layouts: usize,
    pub entries_used_this_frame: usize,
    pub ready_unused_entries: usize,
    pub ready_entries_without_backend_object: usize,
    pub used_entries_without_backend_object: usize,
    pub cache_hits_this_frame: u32,
    pub cache_misses_this_frame: u32,
    pub invalidated_this_frame: u32,
}

impl PipelineCacheStats {
    pub fn ready_backend_object_gap(&self) -> usize;
    pub fn used_backend_object_gap(&self) -> usize;
    pub fn all_ready_entries_have_backend_objects(&self) -> bool;
    pub fn all_used_entries_have_backend_objects(&self) -> bool;
    pub fn has_complete_facade_backend_object_coverage(&self) -> bool;
}

pub enum PipelineCacheEntryStatus {
    Ready,
    Compiling,
    Failed,
}

pub struct PipelineCacheEntryInfo {
    pub key: PipelineKey,
    pub status: PipelineCacheEntryStatus,
    pub has_backend_object: bool,
    pub shader_interface_layout_hash: u64,
    pub last_used_frame: Option<u64>,
    pub used_this_frame: bool,
}

impl Renderer {
    pub fn pipeline_cache_stats(&self) -> PipelineCacheStats;
    pub fn pipeline_cache_entries(&self) -> Vec<PipelineCacheEntryInfo>;
    pub fn warm_up_pipelines(&mut self, requests: &[PipelineWarmupRequest]) -> Result<(), RendererError>;
}
```

`PipelineCacheEntryInfo::shader_interface_layout_hash` 必须与该 entry 的 shader interface 资源绑定、push constants 和 vertex inputs 匹配，用于从 pipeline cache 观测 shader reflection / material template layout identity。

`PipelineCacheStats::shader_interface_layouts` 必须统计 pipeline cache 中非零 shader interface layout hash 的唯一数量，用于 frame/debug stats 聚合观测当前 cache 涉及多少种 shader interface layout。

---

## 21. GPU Memory / Upload / Streaming

### 21.1 Upload queue

Renderer 内部维护 upload queue：

```text
CPU data
  -> staging buffer / mapped upload heap
  -> copy command
  -> target GPU resource
  -> fence
  -> release staging memory
```

Public API 不直接暴露 staging buffer。

```rust
pub struct UploadStats {
    pub bytes_queued: u64,
    pub bytes_uploaded_this_frame: u64,
    pub pending_uploads: usize,
    pub staging_bytes_in_use: u64,
}

impl Renderer {
    pub fn upload_stats(&self) -> UploadStats;
    pub fn flush_uploads(&mut self) -> Result<(), RendererError>;
}
```

### 21.2 资源驻留

可选高级 API：

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidencyPriority {
    Critical,
    High,
    Normal,
    Low,
    Streamable,
}

impl Renderer {
    pub fn set_resource_priority<T>(
        &mut self,
        handle: Handle<T>,
        priority: ResidencyPriority,
    ) -> Result<(), RendererError>;
}
```

---

## 22. ECS 集成建议

Renderer 不应该强绑某个 ECS，但 API 应支持 ECS 提取模型。

推荐流程：

```text
Main World frame N+1 simulation
        │
        ├── Extract visible/renderable data into Render World snapshot
        │
        ▼
Render World frame N rendering
```

Renderer-facing trait：

```rust
pub trait ExtractRenderData {
    fn extract(&self, commands: &mut SceneCommandBuffer);
}
```

引擎层可以这样组织：

```rust
fn extract_renderables(world: &World, renderer: &mut Renderer, scene: SceneHandle) {
    let mut commands = SceneCommandBuffer::new(scene);

    for entity in world.query::<RenderableQuery>() {
        commands.set_transform(entity.object, entity.transform.matrix());
        commands.set_material(entity.object, 0, entity.material);
    }

    renderer.apply_scene_commands(commands).unwrap();
}
```

---

## 23. Debug Draw / Editor API

Debug draw 属于 Renderer 的便利功能，但不能污染主材质系统。

```rust
pub struct DebugDraw<'a> {
    // private
}

impl<'a> DebugDraw<'a> {
    pub fn line(&mut self, a: Vec3, b: Vec3, color: Color);
    pub fn ray(&mut self, origin: Vec3, dir: Vec3, len: f32, color: Color);
    pub fn sphere(&mut self, center: Vec3, radius: f32, color: Color);
    pub fn aabb(&mut self, bounds: Bounds3, color: Color);
    pub fn frustum(&mut self, view_proj: Mat4, color: Color);
    pub fn text_3d(&mut self, position: Vec3, text: &str, color: Color);
}
```

Editor picking：

```rust
#[derive(Clone, Debug)]
pub struct PickingRequest {
    pub view: ViewHandle,
    pub pixel: UVec2,
}

#[derive(Clone, Debug)]
pub struct PickingResult {
    pub object: Option<ObjectHandle>,
    pub user_id: u64,
    pub depth: f32,
    pub world_position: Vec3,
}

impl Renderer {
    pub fn request_picking(&mut self, request: PickingRequest) -> Result<PickingTicket, RendererError>;
    pub fn poll_picking(&mut self, ticket: PickingTicket) -> Option<PickingResult>;
}
```

---

## 24. Profiling / Capture / Stats

```rust
#[derive(Clone, Debug)]
pub struct FrameStats {
    pub frame_index: u64,
    pub cpu_build_time_ms: f32,
    pub cpu_submit_time_ms: f32,
    pub gpu_time_ms: Option<f32>,
    pub draw_calls: u32,
    pub dispatch_calls: u32,
    pub triangles: u64,
    pub visible_objects: u32,
    pub culled_objects: u32,
    pub pipeline_switches: u32,
    pub material_switches: u32,
    pub pipeline_cache: PipelineCacheStats,
    pub material_backend_support: MaterialBackendSupport,
    pub material_reflection_coverage: MaterialReflectionCoverageStats,
    pub deformation_support: DeformationSupport,
    pub lighting_support: RendererLightingSupport,
    pub resource_lifecycle_support: ResourceLifecycleSupport,
    pub backend_synchronization_support: BackendSynchronizationSupport,
    pub post_process_support: PostProcessSupport,
    pub frame_capture_support: FrameCaptureSupport,
    pub debug_tooling_support: DebugToolingSupport,
    pub rhi_support: RendererRhiSupport,
    pub shader_variant_cache: ShaderVariantCacheStats,
    pub shader_variant_cache_entries: usize,
    pub shader_variants_used_this_frame: usize,
    pub shader_variants_ready_unused: usize,
    pub shader_variants_backend_compiled: usize,
    pub shader_variants_without_backend_module: usize,
    pub shader_variant_interface_layouts: usize,
    pub texture_configuration: TextureConfigurationStats,
    pub sampler_configuration: SamplerConfigurationStats,
    pub upload: UploadStats,
    pub memory: MemoryStats,
    pub graph: RenderGraphStats,
    pub environment_outputs: Vec<FrameEnvironmentOutput>,
    pub deformation_outputs: Vec<FrameDeformationOutput>,
    pub motion_vector_outputs: Vec<FrameMotionVectorOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameEnvironmentOutput {
    pub view_label: Option<String>,
    pub environment_label: Option<String>,
    pub skybox_texture_label: Option<String>,
    pub irradiance_texture_label: Option<String>,
    pub prefiltered_specular_texture_label: Option<String>,
    pub brdf_lut_texture_label: Option<String>,
    pub skybox_mip_levels: Option<u32>,
    pub irradiance_mip_levels: Option<u32>,
    pub prefiltered_specular_mip_levels: Option<u32>,
    pub brdf_lut_mip_levels: Option<u32>,
    pub skybox_mips_generated: Option<bool>,
    pub irradiance_mips_generated: Option<bool>,
    pub prefiltered_specular_mips_generated: Option<bool>,
    pub brdf_lut_mips_generated: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameDeformationOutput {
    pub view_label: Option<String>,
    pub skinned_objects: u32,
    pub morphed_objects: u32,
    pub deformed_objects: u32,
    pub skeleton_instances: u32,
    pub skeleton_buffer_bytes: u64,
    pub morph_weight_sets: u32,
    pub morph_weight_buffer_bytes: u64,
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
    pub moving_meshes: u32,
    pub moving_mesh_vertex_bytes: u64,
    pub camera_motion: bool,
}

#[derive(Clone, Debug)]
pub struct RenderGraphStats {
    pub pass_count: u32,
    pub semantic_passes: u32,
    pub rhi_executed_passes: u32,
    pub rhi_executed_pass_labels: Vec<String>,
    pub transient_textures: u32,
    pub transient_buffers: u32,
    pub imported_textures: u32,
    pub imported_buffers: u32,
    pub imported_texture_labels: Vec<String>,
    pub imported_buffer_labels: Vec<String>,
    pub exported_textures: u32,
    pub exported_buffers: u32,
    pub exported_texture_labels: Vec<String>,
    pub exported_buffer_labels: Vec<String>,
    pub aliased_memory_bytes: u64,
    pub barriers: u32,
}

impl RenderGraphStats {
    pub fn imported_resource_labels(&self) -> RenderGraphResourceLabels;
    pub fn exported_resource_labels(&self) -> RenderGraphResourceLabels;
    pub fn has_resource_imports(&self) -> bool;
    pub fn has_resource_exports(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameCaptureIntegration {
    Internal,
    ExternalSdkRequired,
    ExternalHookRegistered,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCaptureHookDesc {
    pub label: Option<String>,
    pub sdk_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCaptureHookEvent {
    pub backend: FrameCaptureBackend,
    pub request_id: u64,
    pub label: Option<String>,
    pub queued_at_frame_index: u64,
    pub frame_index: u64,
    pub include_resource_dump: bool,
    pub open_after_capture: bool,
    pub hook_label: Option<String>,
    pub hook_sdk_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameCaptureBackendInfo {
    pub backend: FrameCaptureBackend,
    pub available: bool,
    pub requires_external_hook: bool,
    pub integration: FrameCaptureIntegration,
    pub sdk_name: Option<&'static str>,
    pub registered_hook_label: Option<String>,
    pub registered_sdk_name: Option<String>,
    pub unavailable_reason: Option<&'static str>,
    pub status: FrameCaptureStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameCaptureRequestInfo {
    pub request_id: u64,
    pub label: Option<String>,
    pub backend: FrameCaptureBackend,
    pub status: FrameCaptureStatus,
    pub backend_integration: FrameCaptureIntegration,
    pub backend_requires_external_hook: bool,
    pub backend_sdk_name: Option<String>,
    pub backend_unavailable_reason: Option<String>,
    pub include_resource_dump: bool,
    pub open_after_capture: bool,
    pub queued_at_frame_index: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameCapture {
    pub label: Option<String>,
    pub backend: FrameCaptureBackend,
    pub status: FrameCaptureStatus,
    pub backend_integration: FrameCaptureIntegration,
    pub backend_requires_external_hook: bool,
    pub backend_sdk_name: Option<String>,
    pub backend_unavailable_reason: Option<String>,
    /// True when a registered external capture hook was still available at frame finish
    /// and the capture request was handed off to that hook.
    pub external_hook_triggered: bool,
    /// True when the handoff used a registered callable hook and the renderer invoked it.
    pub external_hook_callback_invoked: bool,
    /// True when the callable hook panicked during frame-finish handoff.
    pub external_hook_callback_failed: bool,
    /// Panic payload captured from a failed callable hook, when available.
    pub external_hook_callback_failure: Option<String>,
    pub external_hook_label: Option<String>,
    pub external_hook_sdk_name: Option<String>,
    pub request_id: u64,
    pub queued_at_frame_index: u64,
    pub capture_latency_frames: u64,
    pub include_resource_dump: bool,
    pub open_after_capture: bool,
    pub frame_index: u64,
    pub graph: RenderGraphStats,
    pub pipeline_cache: PipelineCacheStats,
    pub material_backend_support: MaterialBackendSupport,
    pub material_reflection_coverage: MaterialReflectionCoverageStats,
    pub deformation_support: DeformationSupport,
    pub lighting_support: RendererLightingSupport,
    pub resource_lifecycle_support: ResourceLifecycleSupport,
    pub backend_synchronization_support: BackendSynchronizationSupport,
    pub post_process_support: PostProcessSupport,
    pub frame_capture_support: FrameCaptureSupport,
    pub debug_tooling_support: DebugToolingSupport,
    pub rhi_support: RendererRhiSupport,
    pub pipeline_shader_interface_layouts: usize,
    pub shader_variant_cache: ShaderVariantCacheStats,
    pub shader_variant_cache_entries: usize,
    pub shader_variants_used_this_frame: usize,
    pub shader_variants_ready_unused: usize,
    pub shader_variants_backend_compiled: usize,
    pub shader_variants_without_backend_module: usize,
    pub shader_variant_interface_layouts: usize,
    pub texture_configuration: TextureConfigurationStats,
    pub sampler_configuration: SamplerConfigurationStats,
    pub retired_submission_frame: Option<u64>,
    pub pending_submission_frame: Option<u64>,
    pub resource_dump: Option<FrameCaptureResourceDump>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrameCaptureResourceDump {
    pub meshes: usize,
    pub buffers: usize,
    pub textures: usize,
    pub generated_mip_textures: usize,
    pub samplers: usize,
    pub material_backend_support: MaterialBackendSupport,
    pub material_reflection_coverage: MaterialReflectionCoverageStats,
    pub deformation_support: DeformationSupport,
    pub lighting_support: RendererLightingSupport,
    pub resource_lifecycle_support: ResourceLifecycleSupport,
    pub backend_synchronization_support: BackendSynchronizationSupport,
    pub post_process_support: PostProcessSupport,
    pub frame_capture_support: FrameCaptureSupport,
    pub debug_tooling_support: DebugToolingSupport,
    pub rhi_support: RendererRhiSupport,
    pub shader_variant_cache: ShaderVariantCacheStats,
    pub texture_configuration: TextureConfigurationStats,
    pub sampler_configuration: SamplerConfigurationStats,
    pub comparison_samplers: usize,
    pub anisotropic_samplers: usize,
    pub custom_lod_samplers: usize,
    pub shaders: usize,
    pub materials: usize,
    pub resident_bytes: u64,
    pub resident_resources: usize,
    pub evicted_resources: usize,
    pub streamable_resources: usize,
    pub resident_streamable_resources: usize,
    pub evicted_streamable_resources: usize,
    pub streamable_texture_mips: u32,
    pub resident_streamable_texture_mips: u32,
    pub evicted_streamable_texture_mips: u32,
    pub streamable_mesh_bytes: u64,
    pub resident_streamable_mesh_bytes: u64,
    pub evicted_streamable_mesh_bytes: u64,
    pub reclaim_policy: ResourceReclaimPolicy,
    pub delayed_destroy_count: usize,
    pub delayed_destroy_bytes: u64,
    pub reclaimed_this_frame: usize,
    pub reclaimed_bytes_this_frame: u64,
    pub backend_retirement: BackendResourceRetirementStats,
    pub pending_uploads: usize,
    pub staging_bytes_in_use: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceReclaimPolicy {
    FrameLatency { frames: u32 },
    BackendFence,
    SubmissionBoundary,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceRetirementStats {
    pub submission_complete: bool,
    pub retired_submission_frame: Option<u64>,
    pub pending_submission_frame: Option<u64>,
    pub upload: UploadStats,
    pub memory: MemoryStats,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TombstoneSubmissionIndexCoverage {
    #[default]
    NotApplicable,
    None,
    Partial,
    All,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackendResourceRetirementStats {
    pub tombstones: usize,
    pub last_poll_queue_empty: bool,
    pub retired_after_queue_empty_poll: bool,
    pub last_poll_completed_submission_index_recorded: bool,
    pub retired_after_completed_submission_index_poll: bool,
    pub tombstones_with_submission_index: usize,
    pub tombstones_without_submission_index: usize,
    pub tombstone_submission_index_coverage: TombstoneSubmissionIndexCoverage,
    pub all_tombstones_have_submission_index: bool,
    pub partial_tombstone_submission_index_coverage: bool,
    pub no_tombstones_have_submission_index: bool,
    pub native_pipeline_entries: usize,
    pub render_pipeline_refs: usize,
    pub shader_modules: usize,
    pub shader_variant_modules: usize,
    pub material_textures: usize,
    pub material_samplers: usize,
    pub post_pass_vertex_buffers: usize,
    pub post_pass_index_buffers: usize,
    pub fence_objects: usize,
    pub fence_submission_indices: usize,
    pub fence_objects_without_submission_index: usize,
    pub bind_groups: usize,
    pub owned_buffers: usize,
    pub retired_tombstones_this_poll: usize,
    pub retired_tombstones_with_submission_index_this_poll: usize,
    pub retired_tombstones_without_submission_index_this_poll: usize,
    pub retired_tombstone_submission_index_coverage_this_poll: TombstoneSubmissionIndexCoverage,
    pub retired_all_tombstones_had_submission_index_this_poll: bool,
    pub retired_partial_tombstone_submission_index_coverage_this_poll: bool,
    pub retired_no_tombstones_had_submission_index_this_poll: bool,
    pub retired_native_pipeline_entries_this_poll: usize,
    pub retired_render_pipeline_refs_this_poll: usize,
    pub retired_shader_modules_this_poll: usize,
    pub retired_shader_variant_modules_this_poll: usize,
    pub retired_material_textures_this_poll: usize,
    pub retired_material_samplers_this_poll: usize,
    pub retired_post_pass_vertex_buffers_this_poll: usize,
    pub retired_post_pass_index_buffers_this_poll: usize,
    pub retired_fence_objects_this_poll: usize,
    pub retired_fence_submission_indices_this_poll: usize,
    pub retired_fence_objects_without_submission_index_this_poll: usize,
    pub retired_bind_groups_this_poll: usize,
    pub retired_owned_buffers_this_poll: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryStats {
    pub resident_bytes: u64,
    pub resident_resources: usize,
    pub evicted_resources: usize,
    pub streamable_resources: usize,
    pub resident_streamable_resources: usize,
    pub evicted_streamable_resources: usize,
    pub streamable_texture_mips: u32,
    pub resident_streamable_texture_mips: u32,
    pub evicted_streamable_texture_mips: u32,
    pub streamable_mesh_bytes: u64,
    pub resident_streamable_mesh_bytes: u64,
    pub evicted_streamable_mesh_bytes: u64,
    pub reclaim_policy: ResourceReclaimPolicy,
    pub delayed_destroy_count: usize,
    pub delayed_destroy_bytes: u64,
    pub reclaimed_this_frame: usize,
    pub reclaimed_bytes_this_frame: u64,
    pub backend_retirement: BackendResourceRetirementStats,
}

impl Renderer {
    pub fn last_frame_stats(&self) -> Option<&FrameStats>;
    pub fn poll_resource_retirements(&mut self) -> ResourceRetirementStats;
    pub fn enable_gpu_profiler(&mut self, enabled: bool) -> Result<(), RendererError>;
    pub fn capture_next_frame(&mut self, options: CaptureOptions) -> Result<(), RendererError>;
    pub fn pending_frame_capture_info(&self) -> Option<FrameCaptureRequestInfo>;
    pub fn register_frame_capture_backend_hook(&mut self, backend: FrameCaptureBackend, desc: FrameCaptureHookDesc) -> Result<(), RendererError>;
    pub fn register_frame_capture_backend_callback<F>(&mut self, backend: FrameCaptureBackend, desc: FrameCaptureHookDesc, callback: F) -> Result<(), RendererError>
    where
        F: Fn(FrameCaptureHookEvent) + Send + Sync + 'static;
    pub fn unregister_frame_capture_backend_hook(&mut self, backend: FrameCaptureBackend) -> Result<(), RendererError>;
    pub fn frame_capture_backend_info(&self, backend: FrameCaptureBackend) -> FrameCaptureBackendInfo;
    pub fn frame_capture_backend_infos(&self) -> Vec<FrameCaptureBackendInfo>;
}
```

---

## 25. 错误处理

Renderer API 不应该到处 panic。除非是明显的程序员不变量被破坏，public API 应返回 `Result`。

```rust
#[derive(thiserror::Error, Debug)]
pub enum RendererError {
    #[error("backend error: {0}")]
    Backend(String),

    #[error("device lost: {reason}")]
    DeviceLost { reason: String },

    #[error("out of memory: {0}")]
    OutOfMemory(String),

    #[error("invalid handle: kind={kind:?}, raw={raw}")]
    InvalidHandle { kind: ResourceKind, raw: u64 },

    #[error("resource is not ready: {0:?}")]
    ResourceNotReady(ResourceKind),

    #[error("unsupported feature: {0:?}")]
    UnsupportedFeature(RendererFeature),

    #[error("shader compile error: {0}")]
    ShaderCompile(String),

    #[error("pipeline compile error: {0}")]
    PipelineCompile(String),

    #[error("material parameter mismatch: {0}")]
    MaterialParameterMismatch(String),

    #[error("render graph validation error: {0}")]
    RenderGraphValidation(String),

    #[error("validation error: {0}")]
    Validation(String),
}
```

建议提供 validation mode：

```rust
pub enum ValidationMode {
    Off,
    Basic,
    Full,
    GpuAssisted,
}
```

---

## 26. Custom Pass 示例：Outline Pass

```rust
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
        ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError> {
        let depth = ctx.main_depth();
        let color = ctx.main_color();

        graph.add_pass("editor_outline")
            .queue(QueueType::Graphics)
            .read_texture(depth, TextureReadUsage::Sampled)
            .color_attachment(color, ColorAttachmentOps::load_store())
            .execute(|ctx| {
                let mut pass = ctx.begin_render_pass(RenderPassDesc::label("editor_outline"));
                pass.set_pipeline(ctx.pipeline("outline_pipeline")?);
                pass.draw_fullscreen_triangle();
                Ok(())
            });

        Ok(())
    }
}
```

---

## 27. 完整使用示例

```rust
use engine_renderer::prelude::*;

async fn init_renderer(window: &winit::window::Window) -> anyhow::Result<Renderer> {
    let renderer = Renderer::with_surface(
        RendererConfig {
            backend: BackendPreference::Auto,
            validation: ValidationMode::Full,
            frame_latency: 2,
            surface_format: None,
            depth_format: DepthFormat::D32Float,
            msaa_samples: 1,
            vsync: VSyncMode::Adaptive,
            hdr: true,
            preferred_render_path: RenderPath::Deferred,
            shader_hot_reload: cfg!(debug_assertions),
            transient_resource_aliasing: true,
            gpu_profiling: cfg!(debug_assertions),
            debug_labels: cfg!(debug_assertions),
        },
        window,
    ).await?;

    Ok(renderer)
}

fn build_scene(renderer: &mut Renderer) -> anyhow::Result<SceneHandle> {
    let scene = renderer.create_scene(SceneDesc {
        label: Some("main_scene".into()),
        max_objects_hint: Some(100_000),
        max_lights_hint: Some(2048),
        enable_gpu_culling: true,
        enable_occlusion_culling: true,
    })?;

    let albedo = renderer.create_texture(TextureDesc {
        label: Some("crate_albedo"),
        dimension: TextureDimension::D2,
        width: 1024,
        height: 1024,
        depth_or_layers: 1,
        mip_levels: 1,
        samples: 1,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        initial_data: None,
    })?;

    let material = renderer.create_standard_material(StandardMaterialDesc {
        label: Some("crate_mat".into()),
        domain: MaterialDomain::Opaque,
        base_color: Color::WHITE,
        base_color_texture: Some(albedo),
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
    })?;

    let mesh = create_cube_mesh(renderer)?;

    renderer.edit_scene(scene, |s| {
        s.spawn(RenderObjectDesc {
            label: Some("cube".into()),
            mesh,
            materials: vec![material],
            transform: Mat4::IDENTITY,
            previous_transform: None,
            bounds: None,
            layer: RenderLayer::default(),
            visibility: VisibilityFlags::CAMERA | VisibilityFlags::SHADOW,
            flags: ObjectFlags::STATIC | ObjectFlags::CAST_SHADOW | ObjectFlags::RECEIVE_SHADOW,
            skeleton: None,
            morph_weights: None,
            lod_group: None,
            user_id: 1,
        });

        s.add_light(LightDesc::Directional(DirectionalLightDesc {
            label: Some("sun".into()),
            direction: Vec3::new(-0.3, -1.0, -0.2).normalize(),
            color: Color::WHITE,
            illuminance_lux: 80_000.0,
            shadow: Some(DirectionalShadowDesc {
                resolution: 2048,
                cascades: 4,
                max_distance: 200.0,
                split_lambda: 0.7,
                filter: ShadowFilter::Pcf { taps: 16 },
                bias: ShadowBias {
                    constant: 0.001,
                    slope: 1.5,
                    normal: 0.05,
                },
            }),
            layer_mask: RenderLayerMask::all(),
        }));
    })?;

    Ok(scene)
}

fn render_frame(renderer: &mut Renderer, scene: SceneHandle, camera_transform: Mat4, size: UVec2) -> anyhow::Result<()> {
    let mut frame = renderer.begin_frame(FrameInput {
        delta_time: 1.0 / 60.0,
        absolute_time: 0.0,
        frame_index_override: None,
        wait_for_gpu: false,
    })?;

    frame.render_view(ViewDesc {
        label: Some("main_view".into()),
        scene,
        camera: CameraDesc {
            label: Some("main_camera".into()),
            transform: camera_transform,
            projection: Projection::Perspective {
                vertical_fov: 60.0_f32.to_radians(),
                aspect: size.x as f32 / size.y as f32,
                near: 0.05,
                far: None,
                reverse_z: true,
            },
            exposure: Exposure::Auto,
            clear: ClearOptions::ColorDepth(Color::BLACK),
            viewport: None,
            scissor: None,
            jitter: None,
            previous_view_proj: None,
            flags: CameraFlags::MAIN | CameraFlags::ENABLE_TAA | CameraFlags::ENABLE_BLOOM,
        },
        target: RenderTarget::MainSurface,
        render_path: RenderPath::Deferred,
        quality: ViewQualitySettings::high(),
        layers: RenderLayerMask::all(),
        graph_extensions: vec![],
    })?;

    let stats = frame.finish()?;
    log::trace!("draws={} gpu={:?}ms", stats.draw_calls, stats.gpu_time_ms);

    Ok(())
}
```

---

## 28. API 稳定性分层

建议定义三个稳定等级：

```text
Stable API
    Renderer / Scene / Resource / Material / View
    游戏代码长期依赖

Extension API
    RenderGraphExtension / MaterialTemplate / CustomPass
    插件代码依赖，小版本可能扩展

Internal API
    RHI / Backend / PipelineCache / MemoryAllocator
    不承诺稳定，或只在 engine 内部使用
```

Rust feature flags：

```toml
[features]
default = ["backend-wgpu", "pbr", "render-graph"]
backend-wgpu = []
backend-vulkan = []
backend-metal = []
backend-d3d12 = []
ray-tracing = []
mesh-shader = []
bindless = []
gpu-profiler = []
editor = []
```

---

## 29. 实现路线

### Phase 1：可用 3D Renderer

- Renderer init / surface
- handle resource manager
- mesh / texture / standard material
- scene retained mode
- forward renderer
- directional light + shadow
- camera / render target
- simple post process

### Phase 2：现代 3D Renderer

- RenderGraph
- deferred renderer
- PBR + IBL
- point / spot light
- CSM
- transparent pass
- bloom / tonemap / TAA
- GPU profiler
- shader hot reload

### Phase 3：大规模场景

- GPU culling
- indirect draw
- instancing
- LOD
- occlusion culling
- streaming texture / mesh
- bindless where available

### Phase 4：高级图形

- ray tracing backend feature
- meshlet / mesh shader
- virtual texturing
- async compute
- editor frame debugger
- RenderDoc integration hooks

---

## 30. 最终判断

这个 Renderer API 的关键不是暴露多少函数，而是保护几个边界：

1. 游戏层只声明“我要渲染什么”。
2. Renderer 内部决定“怎样最高效地渲染”。
3. RenderGraph 决定“这一帧的 GPU 工作如何排布”。
4. RHI 决定“如何映射到底层图形 API”。

推荐的对外 API 心智模型是：

```text
Renderer 是一个声明式 3D 渲染服务。

你提交 scene、assets、view。
它生成 graph、资源状态、pipeline、draw calls。
```

不要让游戏代码天天捧着 command buffer 在 GPU 地窖里点蜡烛。那不是 API 设计，是把引擎用户发配去 Vulkan 煤矿。

### FrameDebugReport pipeline layout observability

`FrameDebugReport` must mirror `PipelineCacheStats::shader_interface_layouts` through a top-level `pipeline_shader_interface_layouts` field, so editor/inspector panels can display the number of shader interface layouts represented by the current pipeline cache without parsing nested cache stats.

### FrameCapture pipeline layout observability

`FrameCapture` must mirror `PipelineCacheStats::shader_interface_layouts` through `pipeline_shader_interface_layouts`, matching `FrameDebugReport` so capture artifacts and editor reports expose the same pipeline shader interface layout aggregate.

`FrameDebugReport` and `FrameCapture` must also mirror `FrameStats::shader_variant_cache_entries`, `shader_variants_used_this_frame`, `shader_variants_backend_compiled`, and `shader_variant_interface_layouts`, matching shader variant cache public inspection with frame-level and capture-level observability.

### WGPU material layout observability

`render_wgpu` exposes `WgpuMaterialLayoutInfo` through `wgpu_material_layout_info()`,
`WgpuMaterial::layout_info()`, and `MeshRenderer::material_layout_info()`. The report captures
the fixed backend material bind group contract used by the mesh shader: uniform binding `0`,
texture bindings, sampler bindings, occupied binding slots, total binding count, and highest
binding index. This makes the legacy fixed wgpu material path inspectable while the higher-level
renderer material-template path continues to evolve toward fully reflected dynamic layouts.

### WGPU material bind group source of truth

The fixed `render_wgpu` mesh material bind group layout is now built from the same backend contract exposed by `WgpuMaterialLayoutInfo`. This keeps the public backend observability API, the actual `wgpu::BindGroupLayout` entries, and the `mesh.wgsl` binding declarations aligned by one targeted unit test.

### WGPU backend pipeline layout inventory

`MeshRenderer` reports its fixed backend pipeline inventory through `STATIC_RENDER_PIPELINE_COUNT`, `STATIC_RENDER_PIPELINE_LAYOUT_COUNT`, `render_pipeline_count()`, and `render_pipeline_layout_count()`. The wgpu runtime forwards the layout count into `FrameStats.pipeline_cache.shader_interface_layouts`, so editor debug reports and frame captures can see native backend pipeline layout inventory instead of only facade-level cache entries.

### WGPU shader interface layout planning

`ShaderResourceBinding` stores both `group` and `binding`, preserving the WGSL `@group(n)` / `@binding(n)` slot needed for native backend layout creation. WGSL auto reflection fills these fields, explicit shader interfaces must provide them, validation rejects duplicate `(group, binding)` slots, and shader interface layout hashes include both values.

With the `backend-wgpu` feature, `wgpu_shader_interface_layout_plan()` converts a `ShaderInterfaceDesc` into grouped `wgpu::BindGroupLayoutEntry` plans plus push constant ranges. The current mapping supports uniform buffers, storage buffers, sampled textures, samplers, and vertex/fragment/compute visibility. Storage texture layout creation is supported when the reflected storage format maps to the engine storage-texture subset (`Rgba8Unorm`, `Rgba16Float`, `Rgba32Float`) and access is `read`, `write`, or `read_write`; unsupported formats are rejected during layout planning.

### Storage texture reflection and wgpu mapping

Storage textures are represented as `BindingType::StorageTexture { dimension, format, access }`, not as sampled textures. WGSL auto reflection parses `texture_storage_*<format, access>` into this variant, material parameter validation still binds them through `TextureHandle`, and the wgpu layout planner maps supported storage texture bindings to `wgpu::BindingType::StorageTexture`. The supported storage texture format subset is `Rgba8Unorm`, `Rgba16Float`, and `Rgba32Float`; unsupported storage formats are rejected before native layout creation.

### WGPU material bind group resource planning

`wgpu_material_bind_group_resource_plan()` maps reflected material parameters into backend bind group resource entries before native object creation. It uses `ShaderResourceBinding::{group, binding}` to group entries by bind group index and sort them by binding number, and it records whether each entry is backed by a texture handle, sampler handle, or byte-buffer payload. The planner rejects duplicate material parameters, parameters that do not match a reflected shader binding, and parameter values whose resource kind does not match the binding type.

### WGPU reflected native object creation

`create_wgpu_shader_interface_layout_objects()` and `WgpuRendererRuntime::create_shader_interface_layout_objects()` create native `wgpu::BindGroupLayout` objects plus a `wgpu::PipelineLayout` from a reflected `ShaderInterfaceDesc`. `create_wgpu_material_bind_groups_from_plan()` creates native `wgpu::BindGroup` objects from a `WgpuMaterialBindGroupResourcePlan` and caller-provided resource resolver, so runtime resource tables can resolve texture, sampler, and buffer handles without hard-coding lookup into the planner.

### WGPU reflected render pipeline creation

`WgpuRenderPipelineDesc`, `create_wgpu_render_pipeline()`, and `WgpuRendererRuntime::create_render_pipeline()` provide the native render pipeline creation entry for reflected pipelines. The caller supplies the `wgpu::ShaderModule`, reflected `wgpu::PipelineLayout`, vertex/fragment entry points, vertex buffer layouts, color/depth formats, sample count, depth-write mode, and blend state. The helper validates basic pipeline invariants before calling `wgpu::Device::create_render_pipeline`.

### WGPU shader module creation

`create_wgpu_shader_module()` and `WgpuRendererRuntime::create_shader_module()` create native `wgpu::ShaderModule` objects from WGSL shader sources. In-memory `ShaderSource::Wgsl` and `.wgsl` `ShaderSource::File` inputs are supported; SPIR-V/MSL/HLSL/Slang inputs return `ShaderCompile` until translation or backend-specific compilation is implemented.

### WGPU native pipeline cache metadata

`WgpuNativePipelineCacheMetadata` tracks reflected native pipeline cache metadata keyed by `PipelineKey`. It records ready backend pipeline entries, shader interface layout hashes, per-frame usage, invalidations, and exposes a `PipelineCacheStats` view with backend object counts and unique shader interface layout counts. This is the metadata/statistics layer required before storing actual `wgpu::RenderPipeline` handles in the runtime cache.

### WGPU runtime pipeline cache stats integration

`WgpuRendererRuntime` now owns `WgpuNativePipelineCacheMetadata` and exposes methods to record, mark-used, invalidate, and query reflected native pipeline metadata. During `render_scene()`, fixed `MeshRenderer` pipeline inventory is merged with native reflected pipeline cache stats before publishing `FrameStats.pipeline_cache`, so editor/debug consumers can observe both legacy fixed pipelines and reflected backend pipeline cache entries through the same stats path.

Reflected custom-material facade pipeline entries preserve an alias set from the public/facade `PipelineKey` to material-specific backend-wgpu native reflected pipeline keys. When any aliased native object exists, `Renderer::pipeline_cache_entries()`, `PipelineCacheStats::{ready_entries_without_backend_object, used_entries_without_backend_object}`, and `PipelineCacheBackendCoverage` report the facade entry as backend-backed instead of leaving a false missing-backend-object gap. Cache-stat refresh prunes dead native aliases, so invalidating one material-specific native object does not hide another live native object that shares the same facade key.

When facade frame stats are merged with backend-wgpu stats, `PipelineCacheStats::backend_objects` preserves the stronger backend-object evidence from either side. This keeps backend native inventory visible without erasing facade cache entries that are known to be backend-backed through reflected native aliases.

### WGPU backend resource tombstones

Invalidated reflected native pipeline entries are moved into backend-owned tombstones instead of being dropped immediately. A tombstone keeps invalidated reflected-pipeline `wgpu::ShaderModule` objects, shader-interface layout objects, material `wgpu::BindGroup` objects, owned material buffers, referenced `wgpu::RenderPipeline` objects, invalidated shader-variant `wgpu::ShaderModule` cache entries, and replaced or unregistered material external texture/sampler bindings, and reflected post-pass temporary vertex/index buffers alive until `WgpuRendererRuntime::poll_backend_resource_retirements()` observes a completed backend submission boundary. Each tombstone records a backend fence object carrying the latest `wgpu::SubmissionIndex` when available; retirement stats distinguish indexed fence objects from tombstones that were queued before any backend submission index existed, and expose whether the latest non-blocking backend retirement poll observed an empty queue before retiring the current tombstone set, plus whether that queue-empty poll was tied to a recorded backend submission index. `retired_after_completed_submission_index_poll` is true only when the retired tombstone set includes tombstones whose own fence captured a submission index, so a later unrelated submission cannot make unindexed tombstones appear submission-index protected. Live and retired tombstone-level indexed/unindexed counts are exposed separately from fence counts so tools can reason about the current pending tombstone set directly; A `TombstoneSubmissionIndexCoverage` enum is also exposed for both live and retired tombstone sets so tools can consume one stable `NotApplicable` / `None` / `Partial` / `All` value. Boolean all-indexed, partial-coverage, and no-indexed coverage fields are kept for callers that prefer direct predicates without recomputing those cases from raw counters. Enqueuing new tombstones invalidates that queue-empty gate state so tools cannot mistake an older idle poll for coverage of newly queued backend resources. `WgpuBackendResourceRetirementStats` exposes live tombstone/fence counts plus the number of native pipeline entries, render-pipeline references, reflected-pipeline shader modules, shader-variant modules, material external textures/samplers, reflected post-pass vertex/index buffers, bind groups, owned buffers, and fence objects retired by the last poll. The renderer-level `BackendResourceRetirementStats` mirrors those counts through `MemoryStats`, `ResourceRetirementStats::memory`, and `FrameCaptureResourceDump`, so tools can observe backend tombstone pressure without downcasting to backend-wgpu. `Renderer::begin_frame()` performs non-blocking backend tombstone maintenance so an otherwise empty frame can still advance completed backend retirement and publish the retired counts in `FrameStats::memory`.

### WGPU reflected pipeline build-and-cache entry

`WgpuNativeRenderPipelineBuildDesc` and `WgpuRendererRuntime::create_and_cache_native_render_pipeline()` compose the reflected native pipeline creation steps: WGSL shader module creation, shader-interface layout object creation, render pipeline creation, and insertion into the runtime native pipeline handle cache. The method accepts caller-created material bind groups, so runtime resource lookup can be connected independently before final submission uses the cached pipeline.

### WGPU material bind group auto creation with owned buffers

`WgpuMaterialBindingResource::BufferBytes` preserves the full byte payload for buffer-backed material parameters. `create_wgpu_material_bind_groups_with_owned_buffers_from_plan()` creates and owns native `wgpu::Buffer` objects for uniform/storage byte parameters, then creates `wgpu::BindGroup` objects from the material resource plan. Texture and sampler bindings still use a caller-provided resolver so runtime resource tables can supply the correct `TextureView` and `Sampler` handles. `WgpuNativeRenderPipelineBuildDesc` can now carry an optional material resource plan, and `WgpuRendererRuntime::create_and_cache_native_render_pipeline_with_resource_resolver()` creates material bind groups as part of the build-and-cache flow.

### WGPU material external resource registry and submission binding

`WgpuMaterialExternalResourceRegistry` maps renderer `TextureHandle` and `SamplerHandle` values to native `wgpu::TextureView` and `wgpu::Sampler` objects for reflected material bind group creation. `WgpuRendererRuntime` exposes registration/unregistration methods and `create_and_cache_native_render_pipeline_with_registered_resources()` to build pipelines using the runtime registry. `native_pipeline_objects_for_submission()` marks cached reflected pipelines as used, and `bind_wgpu_native_pipeline_for_render_pass()` binds the cached render pipeline plus all material bind groups into a `wgpu::RenderPass`.

### WGPU reflected pipeline smoke coverage

The backend includes an actual wgpu smoke test that builds a reflected WGSL pipeline, creates an owned uniform buffer material bind group, creates a native render pipeline, binds the cached pipeline and material bind group into an offscreen render pass, submits the encoder, waits for the device, and verifies native pipeline cache usage stats. The smoke test skips only if no wgpu adapter/device can be created in the current environment.

### WGPU runtime draw-to-view submission

`WgpuNativePipelineDrawDesc` and `WgpuRendererRuntime::submit_native_pipeline_draw_to_view()` submit a cached reflected native pipeline directly to a caller-provided `wgpu::TextureView`. The method marks the pipeline used in the native cache, creates a command encoder and render pass, binds the cached pipeline and material bind groups, issues the draw, submits the command buffer, and returns `WgpuNativePipelineSubmissionInfo` with bind group, vertex, and instance counts.

`WgpuSurface` records the latest `wgpu::SubmissionIndex` produced by `render_frame()`, and `WgpuRendererRuntime` records the latest submission from both surface-backed rendering and direct reflected draw-to-view submissions. `WgpuRendererRuntime::wait_for_gpu()` prefers `wgpu::Maintain::WaitForSubmissionIndex` for that exact submission before falling back to a device-wide wait, so `FrameInput::wait_for_gpu` and BackendFence reclamation are tied to a backend submission fence when one exists. Default submitted frames may also reclaim facade DestroyQueued resources at a completed submission boundary without forcing GPU idle; this is reported as `ResourceReclaimPolicy::SubmissionBoundary`, while unfinished backend submissions remain on the configured `FrameLatency` path. `Renderer::poll_resource_retirements()` exposes the same non-blocking retirement path outside frame finish, returning upload and memory snapshots plus the retired/pending submission frame so tools can drive completed-boundary upload and destroy retirement without idling the GPU. `FrameStats`, `FrameDebugReport`, and `FrameCapture` mirror `retired_submission_frame` and `pending_submission_frame`, so frame tooling and capture artifacts can see completed-boundary retirement progress without calling the poll API directly.

### WGPU render_scene reflected draw queue integration

`MeshRenderer` exposes `render_batches_with_environment_probes_and_post_pass()` and `WgpuRenderScene::render_with_post_pass()` so backend integrations can append reflected passes inside the same acquired swapchain frame instead of performing a second acquire/present. `WgpuRendererRuntime::queue_native_pipeline_draw()` queues cached reflected pipeline draws, and `render_scene()` drains that queue in the mesh pass post-hook, binds cached native pipelines/material bind groups, issues draws, marks native pipeline cache usage, and merges the reflected draw count into frame stats.

### Renderer facade reflected material scheduling

`Renderer` now derives wgpu reflected draw work from retained scene draw items for custom material templates that are currently closed by the backend: WGSL reflected shaders with reflected material bindings plus mesh vertex inputs that can be matched to the retained mesh vertex layout. Before a facade main-surface view is rendered, the renderer builds native shader modules, reflected bind group layouts, owned buffer-backed material bind groups, mesh vertex/index buffer uploads, render pipelines, and queued native draws through the backend runtime. This closes the first public facade-to-wgpu path for reflected custom material shaders without requiring game code to bind pipelines or descriptor sets directly.

The automatic path also registers renderer-owned sampled texture and sampler material parameters into the wgpu backend registry when those resources are required by reflected material bindings. The backend creates native `wgpu::Texture`, `wgpu::TextureView`, and `wgpu::Sampler` objects from renderer texture/sampler descriptions, uploads concrete subresource data, splits renderer-generated mip chains into per-mip `queue.write_texture` submissions, uses those resources while building reflected bind groups, and keeps texture ownership alive with the registry binding.

Renderer-owned textures carry a retained revision that advances when `update_texture()`, `generate_mips()`, or renderer-side mip bake state changes the texture content visible to reflected materials. The facade reflected native key includes the TextureHandle revision for texture material parameters, so content changes rebuild the native texture binding/bind group instead of reusing a bind group that points at a stale `TextureView`. The backend cache keeps the structural native render pipeline key separate from the material/bind-group key, so a texture or material-resource update can rebuild bind groups while reusing the same `wgpu::RenderPipeline`; `PipelineCacheStats::backend_objects` reports unique native render pipeline objects rather than material bind-group entries.

Shader reload/destroy, material-template destroy, and material parameter updates must invalidate both renderer facade pipeline cache entries and backend-wgpu reflected native pipeline entries where applicable. Backend-wgpu exposes batch invalidation by ShaderHandle, MaterialTemplateHandle, and MaterialHandle, removes matching material/bind-group entries from the active cache, and moves their native backend objects into tombstones. Structural `wgpu::RenderPipeline` cache entries are removed from the active map once no remaining native entry references their render-pipeline key, while tombstones keep the actual backend objects alive until completed-boundary retirement.

The automatic path validates reflected vertex input requirements against the selected mesh layout, creates matching `wgpu::VertexBufferLayout` descriptors, queues renderer mesh vertex/index bytes for native post-pass submission, binds `wgpu::Buffer` vertex/index buffers during the swapchain render pass, and returns explicit errors when required semantics or formats are missing from the mesh layout.

















## 2026-05-19 本轮进展：后台 resource retirement 能力边界显式化

- `RendererFeature::BackgroundResourceRetirement` 和 `RendererFeatures::BACKGROUND_RESOURCE_RETIREMENT` 已加入 public capability 体系。
- 当前实现明确不支持独立后台线程式 resource retirement；`Renderer::start_background_resource_retirement()` 返回 `RendererError::UnsupportedFeature(RendererFeature::BackgroundResourceRetirement)`。
- 已支持的路径仍是 frame-driven / explicit polling：`begin_frame()`、frame finish 和 `Renderer::poll_resource_retirements()` 触发非阻塞 retirement 维护。
- 该项状态为 `Partial`：capability gate、用户可见错误和测试已闭合；跨线程 worker 调度仍未实现；cooperative startup/active observability 已接入。

## 2026-05-19 本轮进展：backend-wgpu tombstone per-fence retirement 过滤

- backend-wgpu resource tombstone retirement 不再只依赖 queue-empty 后整体 drain；每个带 wgpu `SubmissionIndex` 的 tombstone 会捕获 renderer 内部单调 submission order。
- retirement 现在通过 tombstone 自己的 fence order 与 completed submission order 判断是否可释放；未达到 completed order 的 tombstone 会保留在 pending 队列。
- wgpu 0.20.1 的 `SubmissionIndex` 不可排序，因此内部 order 只作为 renderer retirement 边界，不替代 public submission-index 观测字段。
- unindexed tombstone 仍只在 queue-empty 时释放，避免把后续无关 submission 的完成误报为该 tombstone 的 completed-submission retirement。
- 新增验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`，并回归 `wgpu_backend_tombstone_enqueue_invalidates_previous_queue_empty_gate`、`wgpu_backend_unindexed_tombstone_does_not_claim_completed_submission_retirement`、`wgpu_backend_mixed_tombstone_set_reports_partial_submission_index_coverage`。
- 状态：GPU memory / upload / delayed destroy 项继续保持 `Partial`；per-fence filtering 已闭合，跨线程 worker 和更广泛 renderer 层完整性仍未完成。

## 2026-05-19 本轮进展：backend tombstone pending 原因 public observability

- `WgpuBackendResourceRetirementStats` 和 public `BackendResourceRetirementStats` 新增 `tombstones_waiting_for_submission_index` 与 `tombstones_waiting_for_queue_empty`。
- 这两个字段把 live tombstone 的等待原因暴露到 `MemoryStats.backend_retirement`、frame capture resource dump 和 debug/report 传播路径：带 fence 的资源等待自身 submission order，无 submission index 的资源等待 queue-empty。
- per-fence retirement 过滤现在不只在 backend 内部测试可见，也能通过 renderer facade 的 stats/capture 观测 pending 原因。
- 新增/回归验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`、`renderer_backend_retirement_stats_map_post_pass_buffers`、`renderer_memory_stats_expose_backend_tombstone_retirement`。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；pending 原因观测已闭合，跨线程 worker、更多资源类覆盖和完整 renderer 层闭环仍未完成。

## 2026-05-19 本轮进展：backend retirement poll 粒度显式公开

- `WgpuBackendResourceRetirementStats` 与 public `BackendResourceRetirementStats` 新增：`nonblocking_submission_index_poll_supported`、`queue_empty_poll_fallback`、`last_poll_used_queue_empty_fallback`。
- 当前 wgpu 0.20.1 backend 明确报告 `nonblocking_submission_index_poll_supported = false`、`queue_empty_poll_fallback = true`；public `poll_resource_retirements()` 只能稳定使用 queue-empty 粒度确认完成。
- 内部 per-fence order 过滤仍保留，用于在可提供 completed order 的路径上只释放自身 fence 已完成的 tombstone；公开 stats 不再误导为已具备真正非阻塞 per-submission 查询。
- 新增/回归验证：`wgpu_backend_retirement_filters_tombstones_by_completed_submission_index`、`renderer_backend_retirement_stats_map_post_pass_buffers`、`renderer_memory_stats_expose_backend_tombstone_retirement`。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；poll 粒度与限制已公开闭合，真实非阻塞 submission-index 查询和后台 worker 仍是未实现能力。

## 2026-05-19 本轮进展：非阻塞 submission-index retirement poll capability gate

- public `RendererFeature::NonblockingResourceRetirementPoll` 与 `RendererFeatures::NONBLOCKING_RESOURCE_RETIREMENT_POLL` 已加入统一 feature/capability 体系。
- 当前 headless/wgpu 路径均不声明该 capability；`feature_info()` 返回 `supported = false`、`implementation = ConfigGate`，reason 明确为当前 wgpu backend 使用 queue-empty fallback，尚不支持真正非阻塞 submission-index retirement polling。
- 该 gate 与 `BackendResourceRetirementStats::{nonblocking_submission_index_poll_supported, queue_empty_poll_fallback, last_poll_used_queue_empty_fallback}` 对齐，用户可同时从 feature API 和 stats/capture 观察能力边界。
- 验证：`renderer_feature`、`background_resource_retirement`、`renderer_memory_stats_expose_backend_tombstone_retirement` 相关测试通过。
- 状态：GPU memory / upload / delayed destroy 仍为 `Partial`；能力 gate 已闭合，真实非阻塞 per-submission 完成查询仍未实现。

## 2026-05-19 本轮进展：frame capture/tooling feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `FrameCapture`、`ExternalFrameCaptureHooks`、`NativeFrameDebuggerCapture`。
- `FrameCapture` 和 `ExternalFrameCaptureHooks` 在当前 facade/tooling 层声明 supported，分别对应 internal capture payload 与已存在的 registered external-hook handoff API。
- `NativeFrameDebuggerCapture` 显式 unsupported，reason 指向原生 RenderDoc/external debugger SDK 未链接；当前可用路径是注册外部 capture hook，而不是内置 SDK 调用。
- 该 gate 与 `FrameCaptureBackendInfo`、`FrameCaptureIntegration`、`capture_next_frame()` 的 hook-gated 错误路径对齐，capture/tooling 能力不再只隐藏在 backend info API 中。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer capture_options_validate_backend_hooks -- --nocapture` 通过。
- 状态：Frame API / stats / capture 仍为 `Partial`；内部 capture 与 external-hook handoff capability 已闭合，真实 RenderDoc/外部调试器 SDK 调用仍是外部阻塞/未实现项。

## 2026-05-19 本轮进展：debug draw / editor report feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `DebugDraw` 与 `EditorDebugReports`。
- 两项在当前 facade/tooling 层声明 supported，对应已存在的 debug draw command/output 与 `Renderer::frame_debug_report()` editor-facing summary 路径。
- 这些 tooling 能力现在进入统一 `RendererCaps::features`、`Renderer::supports_feature()`、`Renderer::feature_info()` 与 `Renderer::feature_audit()`，不再只作为散落的 public API 存在。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` 通过。
- 状态：Debug draw / editor API 继续保持 `Partial`；facade capability gate 已闭合，但更深 editor 集成、外部调试器 SDK 与 backend-specific tooling 行为仍未完整闭合。

## 2026-05-19 本轮进展：animation / deformation / LOD feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `SkeletalAnimation`、`MorphTargets`、`LodSelection`、`MotionVectors`、`BackendGpuDeformation`。
- `SkeletalAnimation`、`MorphTargets`、`LodSelection` 作为 supported facade-semantic capability，对应当前 skeleton instance、morph weights、LOD group 与 frame output 语义。
- `MotionVectors` 作为 supported graph-semantic capability，对应当前 motion-vector frame output / RHI observable path。
- `BackendGpuDeformation` 显式 unsupported，reason 说明 backend GPU skinning/morph deformation buffers 尚未实现；当前 deformation 输出仍是 renderer/RHI observable facade semantics。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer deformation -- --nocapture`、`cargo test -p engine_renderer lod -- --nocapture`、`cargo test -p engine_renderer motion_vector -- --nocapture` 通过。
- 状态：Animation / skinning / morph / LOD 仍为 `Partial`；facade/graph capability tracking 已闭合，backend-real GPU deformation 路径仍未实现。

## 2026-05-19 本轮进展：light / shadow / environment / IBL feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `Lights`、`ShadowMapping`、`EnvironmentIbl`、`BackendIblConvolution`。
- `Lights`、`ShadowMapping`、`EnvironmentIbl` 作为 supported graph-semantic capability，对应当前 retained light resources、shadow/environment frame outputs、environment graph import 和 facade-retained IBL bake observability。
- `BackendIblConvolution` 显式 unsupported，reason 说明 backend-real IBL/environment convolution 尚未实现；当前 environment bake 是 renderer-retained facade output。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer environment_ -- --nocapture`、`cargo test -p engine_renderer light -- --nocapture` 通过。
- 状态：Light / shadow / environment / IBL 仍为 `Partial`；facade/graph capability tracking 已闭合，backend-real convolution/capture path 仍未实现。

## 2026-05-19 本轮进展：RenderGraph / standard 3D pipeline feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `RenderGraph`、`CustomRenderGraphPasses`、`Standard3dPipeline`、`BackendRealStandard3dPipeline`。
- `RenderGraph` 作为 supported core graph-semantic capability，对应当前 graph builder、resource lifetime、RHI execution hook 与 validation 语义。
- `CustomRenderGraphPasses` 与 `Standard3dPipeline` 作为 supported graph-semantic capability，对应 custom graph extension 和 standard 3D graph/frame output 语义。
- `BackendRealStandard3dPipeline` 显式 unsupported，reason 说明完整 backend-real standard 3D pass execution 尚未闭合；当前标准管线仍混合 facade、RHI 与部分 backend-wgpu 执行证据。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer graph_ -- --nocapture`、`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` 通过。
- 状态：RenderGraph 基础能力继续保持可验；Standard 3D RenderGraph 仍为 `Partial`，因为全 backend-real standard pass execution 尚未实现。

## 2026-05-19 本轮进展：pipeline cache / shader variant cache feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `PipelineCache`、`ShaderVariantCache`、`CompleteBackendPipelineCache`。
- `PipelineCache` 与 `ShaderVariantCache` 作为 supported facade-semantic capability，对应 public pipeline warmup/cache stats/entry introspection 与 shader variant warmup/cache observability。
- `CompleteBackendPipelineCache` 显式 unsupported，reason 说明 complete backend-native pipeline cache coverage 尚未实现；当前 backend-wgpu reflected native cache 已有真实对象/统计，但 facade cache entries 仍可能缺 backend object，整体仍是 partial。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer pipeline -- --nocapture`、`cargo test -p engine_renderer shader -- --nocapture` 通过。
- 状态：Pipeline / pipeline key / cache 与 Shader variants 仍为 `Partial`；public facade/cache observability capability 已闭合，完整 backend-native cache coverage 仍未实现。

## 2026-05-19 本轮进展：upload / residency / streaming / delayed destroy feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `UploadQueue`、`ResourceResidency`、`StreamingResources`、`DelayedResourceDestroy`。
- 四项作为 supported facade-semantic capability，对应当前 upload stats/flush/submitted-frame bookkeeping、resource residency transitions、streaming memory/capture observability、frame-latency/submission-boundary delayed destroy semantics。
- 这些 memory/resource capabilities 现在进入统一 `RendererCaps::features`、`Renderer::supports_feature()`、`Renderer::feature_info()` 与 `Renderer::feature_audit()`。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`cargo test -p engine_renderer resource_residency_controls_streamed_meshes_and_textures -- --nocapture`、`cargo test -p engine_renderer submitted_frame -- --nocapture`、`cargo test -p engine_renderer poll_resource_retirements_completes_only_prior_submission_work -- --nocapture` 通过。
- 状态：GPU memory / upload / streaming 仍为 `Partial`；facade memory/resource capability tracking 已闭合，backend tombstone coverage、true nonblocking per-submission polling 和 background/cooperative retirement 仍未完整实现。

## 2026-05-19 本轮进展：基础 facade resource / scene / view feature gates

- public `RendererFeature` / `RendererFeatures` 新增 `ResourceLifecycle`、`MeshResources`、`BufferResources`、`TextureResources`、`SamplerResources`、`MaterialSystem`、`RetainedScene`、`CameraViewRenderTargets`、`EcsExtractBoundary`。
- `ResourceLifecycle`、mesh/buffer/texture/sampler/material、retained scene、camera/view/render target 作为 supported core facade-semantic capability，对应当前 public resource create/update/destroy/status/info、scene command buffer、view/render target validation 语义。
- `EcsExtractBoundary` 作为 supported optional facade-semantic capability，对应当前 ECS-like extract fixture 到 retained scene/frame stats 的边界语义。
- 这些基础 facade capabilities 现在进入统一 `RendererCaps::features`、`Renderer::supports_feature()`、`Renderer::feature_info()` 与 `Renderer::feature_audit()`。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture`、`generic_resource_lifecycle_covers_public_resource_kinds`、`custom_material_parameters_are_schema_validated`、`scene_command_buffer_rejects_destroyed_resource_handles_before_mutation`、`render_targets_are_validated_and_can_back_offscreen_views`、`ecs_like_extract_fixture_drives_scene_commands_and_frame_stats` 通过。
- 状态：基础 facade capability tracking 已闭合；backend-real execution、specialized stale-handle coverage 和完整 renderer 层闭环仍按矩阵继续保留未完成/Partial 项。

## 2026-05-19 本轮进展：native frame debugger capture unsupported error 对齐

- public `RendererFeature::NativeFrameDebuggerCapture` 已作为 reserved unsupported feature 暴露，reason 指向当前未链接 RenderDoc/external debugger 原生 SDK。
- `Renderer::capture_next_frame()` 现在在直接请求 `FrameCaptureBackend::RenderDoc` 或 `FrameCaptureBackend::ExternalDebugger` 且没有可用外部 hook / SDK 时，返回 `RendererError::UnsupportedFeature(RendererFeature::NativeFrameDebuggerCapture)`。
- 已注册外部 capture hook 的路径仍允许排队并在 frame finish 输出 `BackendHookRequested`、hook label、SDK name、request id、queued frame 和 capture latency。
- 该变更把 feature gate、backend info、用户可见错误和 capture 测试断言对齐；真实 RenderDoc SDK / external debugger SDK 调用仍是外部阻塞，不能计为完整 renderer 层实现。

验证：`cargo test -p engine_renderer capture_options_validate_backend_hooks -- --nocapture` passed，1 passed；`cargo test -p engine_renderer renderer_feature -- --nocapture` passed，3 passed。

## 2026-05-19 本轮进展：standard 3D backend-native pass 覆盖率观测

- `RenderGraphStats` 新增 backend-native standard pass 覆盖字段：`backend_native_standard_passes`、`backend_native_standard_pass_labels`、`backend_missing_standard_pass_labels`、`backend_real_standard_pipeline_complete`。
- facade/backend graph stats 合并时，现在会把 backend-wgpu native pass label 映射到标准 3D 语义 pass：当前可识别 `Neo Directional Shadow Pass -> shadow_csm`、`Neo Spot/Point Shadow Pass -> shadow_point_spot`、`Neo Forward Opaque Pass -> forward_opaque`。
- 未被 backend native pass 覆盖的 standard graph pass 会进入 `backend_missing_standard_pass_labels`，例如 `depth_prepass`、`present`、gbuffer/deferred/post 等仍可被 frame stats / capture / debug report 观察为未完整 backend-real 覆盖。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 状态：这是 `BackendRealStandard3dPipeline` 缺口的可观测性收口；完整 backend-real standard 3D pipeline 仍未实现，`RendererFeature::BackendRealStandard3dPipeline` 继续保持 unsupported/config-gated。

## 2026-05-19 本轮进展：editor debug report 暴露 standard backend 覆盖字段

- `FrameDebugReport` 新增平铺字段：`backend_native_standard_passes`、`backend_native_standard_pass_labels`、`backend_missing_standard_pass_labels`、`backend_real_standard_pipeline_complete`。
- 这些字段直接镜像 `FrameStats.graph` 中的 standard 3D backend-native pass 覆盖状态，避免 editor/tooling 只读取 pass label 或 RHI label 时误判完整 backend-real standard pipeline。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 状态：debug/editor 可观测性已增强；`BackendRealStandard3dPipeline` 仍未完成，gbuffer/deferred/post/present 等 backend-real pass 仍需继续实现。

## 2026-05-19 本轮进展：frame capture 暴露 standard backend 覆盖字段

- `FrameCapture` 新增平铺字段：`backend_native_standard_passes`、`backend_native_standard_pass_labels`、`backend_missing_standard_pass_labels`、`backend_real_standard_pipeline_complete`。
- capture payload 现在直接镜像 `FrameStats.graph` 的 standard 3D backend-native pass 覆盖状态，与 editor/debug report 和 graph stats 保持一致。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：frame stats / capture / debug report 对 `BackendRealStandard3dPipeline` 缺口的观测面更完整；真实 backend-wgpu gbuffer/deferred/post/present pass 仍未实现。

## 2026-05-19 本轮进展：backend-wgpu present 覆盖映射

- standard 3D backend-native 覆盖统计现在把 backend-wgpu `Neo Forward Opaque Pass` 映射到 `forward_opaque`，并把 `Neo Transparent Pass` 映射到 `transparent` 与 `present`。
- 该映射表达当前 surface path 中 opaque draw 与 transparent/final output 已分离，最终 surface output/present 语义由 `Neo Transparent Pass` 承担，避免把已由 backend-wgpu 完成的 native output 误报为 missing standard pass。
- `backend_missing_standard_pass_labels` 仍会保留 `depth_prepass`、`gbuffer`、`deferred_lighting`、post process 等尚未被 backend-native pass 覆盖的标准 3D pass。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 状态：`BackendRealStandard3dPipeline` 仍未完成；本轮只修正 surface output/present 覆盖观测。

## 2026-05-19 本轮进展：standard backend 覆盖计数字段

- `RenderGraphStats`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_total_standard_passes` 和 `backend_missing_standard_passes`。
- standard 3D backend-native 覆盖现在同时暴露总 standard pass 数、backend-native 覆盖数、缺失数、覆盖 label、缺失 label 和 complete bool，tooling/CI 不再需要解析 label 列表才能判断 `BackendRealStandard3dPipeline` 差距。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：这是 standard 3D backend 覆盖缺口的观测面收口；真实 backend-wgpu gbuffer/deferred/post 等 pass 仍未实现，renderer goal 未完成。

## 2026-05-19 本轮进展：RHI standard pass 观测字段

- `RenderGraphStats` 新增 `rhi_standard_passes` 与 `rhi_standard_pass_labels`，在 `execute_on_rhi` 路径中记录哪些标准 3D pass 真正进入 RHI command execution。
- `FrameDebugReport` 与 `FrameCapture` 同步平铺这两个字段，使 editor/debug/capture 可以直接区分 RHI-executed standard pass 和 backend-wgpu native standard pass 覆盖。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed，覆盖 deferred standard graph 的 RHI standard pass labels。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed，覆盖 editor/debug report 镜像。
- 验证：`cargo test -p engine_renderer graph_ -- --nocapture` passed，34 passed，覆盖 graph/RHI 执行路径。
- 状态：RHI standard pass 观测面已增强；该字段不把 headless/RHI 结果等同为 backend-wgpu native standard pipeline 完成，`BackendRealStandard3dPipeline` 仍未完成。

## 2026-05-19 本轮进展：backend-wgpu native depth prepass

- `render_wgpu::MeshRenderer` 新增 fragment-less depth-only pipeline：`Neo Mesh Depth Prepass Pipeline` 与 `Neo Double-Sided Mesh Depth Prepass Pipeline`。
- backend-wgpu surface frame 现在在 `Neo Forward Opaque Pass` 前执行真实 `Neo Depth Prepass`：对 depth-enabled surface depth target clear/store，只绘制 visible opaque + depth_write batches，随后主 mesh pass load 已写入 depth。
- backend native pass label 顺序现在包含 `Neo Depth Prepass`，renderer-level standard coverage 将其映射为 `depth_prepass`。
- `backend_native_standard_pass_labels` 现在能把 `shadow_csm`、`depth_prepass`、`forward_opaque`、`present` 标为 backend-native 覆盖，减少 `BackendRealStandard3dPipeline` 的真实缺口。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 状态：这是 `BackendRealStandard3dPipeline` 的真实 backend-wgpu 增量；完整 standard 3D backend pipeline 仍未完成，`gbuffer`、`deferred_lighting`、post process 等仍缺 backend-native pass。

## 2026-05-19 本轮进展：native depth prepass stats / pipeline inventory 收口

- `MeshRenderStats` 新增 `mesh_pass_draw_call_count` 与 `depth_prepass_draw_call_count`，`draw_call_count` 现在包含主 mesh pass draw 与 native depth prepass draw 的总和。
- backend-wgpu frame stats 通过 `MeshRenderStats::draw_call_count` 暴露真实 native draw work，不再在新增 `Neo Depth Prepass` 后低报 draw calls。
- `MeshRenderer::STATIC_RENDER_PIPELINE_COUNT` 从 24 更新为 26，包含两个新增 fragment-less depth-prepass pipeline，pipeline cache/backend inventory 观测不再低报。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture` passed，integration target 1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 状态：native depth prepass 的 frame stats 与 pipeline inventory 观测已收口；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：backend-wgpu transparent 覆盖映射

- standard 3D backend-native 覆盖统计现在把 backend-wgpu `Neo Transparent Pass` 映射到 `transparent`。
- 该映射基于 `render_wgpu::MeshRenderer` 已存在的 alpha-blend pipelines、`transparent_draw_call_count` 和同一 native mesh render pass 中的 alpha-blend draw 执行路径。
- coverage 现在可把 `shadow_csm`、`depth_prepass`、`forward_opaque`、`transparent`、`present` 标为 backend-native 覆盖；`gbuffer`、`deferred_lighting`、post process 等仍保持 missing。
- 验证：`cargo test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer renderer_feature -- --nocapture` passed，3 passed。
- 状态：这是 `BackendRealStandard3dPipeline` 的覆盖映射收口，不代表完整 renderer goal 完成。

## 2026-05-19 本轮进展：backend native draw breakdown 高层透传

- `FrameStats` 新增 backend native draw breakdown：`backend_mesh_pass_draw_calls`、`backend_depth_prepass_draw_calls`、`backend_opaque_draw_calls`、`backend_transparent_draw_calls`。
- `FrameDebugReport` 与 `FrameCapture` 同步平铺这些字段，editor/debug/capture 现在能区分主 mesh pass、native depth prepass、opaque draw 与 transparent draw。
- `frame_stats_from_wgpu_metrics` 从 `MeshRenderStats` 透传 draw breakdown，`draw_calls` 仍表示 backend native draw 总量。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：native depth/mesh pass 的高层观测面已收口；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：native mesh pass opaque/transparent phase 顺序

- backend-wgpu surface path 现在拆成真实 native `Neo Forward Opaque Pass` 与 `Neo Transparent Pass`：opaque batches 在前者执行，alpha-blend transparent batches 在后者执行。
- 新增 `mesh_pass_phase_order` helper，防止 batch 输入顺序把 transparent draw 排到 opaque draw 之前。
- 该改动让 `Neo Forward Opaque Pass -> forward_opaque` and `Neo Transparent Pass -> transparent` 的 backend-native coverage 映射更接近标准 3D 管线语义；仍不是独立 transparent render pass。
- 验证：`cargo test -p render_wgpu mesh_pass_phase_order_draws_opaque_before_transparent -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture` passed，integration target 1 passed。
- 状态：forward/transparent native mesh phase 顺序已收口；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：backend-wgpu transparent back-to-front batch 排序

- backend-wgpu `Neo Transparent Pass` 的 transparent batches 现在按相机距离 back-to-front 排序，并在 `Neo Forward Opaque Pass` 之后执行。
- 对 instanced batch，排序距离使用 batch 内最远 instance 的 model-matrix translation 到相机位置的距离，避免拆分 instance batch 的大重构，同时比输入顺序更符合透明渲染语义。
- 新增纯 CPU helper 验证：opaque phase 排在 transparent 前、transparent batch 距离使用最远 instance。
- 验证：`cargo test -p render_wgpu mesh_pass_phase_order_draws_opaque_before_transparent -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_batch_distance_uses_farthest_instance_for_transparent_sorting -- --nocapture` passed，1 passed。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 状态：transparent native mesh-pass 语义更完整；这仍不是独立 transparent render pass，完整 backend-real standard 3D pipeline 仍未完成。

## 2026-05-19 本轮进展：native shadow draw breakdown 高层透传

- `MeshRenderStats` 新增 `shadow_draw_call_count`、`directional_shadow_draw_call_count`、`spot_shadow_draw_call_count`、`point_shadow_draw_call_count`。
- `MeshRenderStats::draw_call_count` 现在表示 backend native 总 draw work：shadow draw + depth prepass draw + mesh pass draw。
- `FrameStats`、`FrameDebugReport` 与 `FrameCapture` 同步新增 backend shadow draw breakdown 字段，native directional/spot/point shadow pass 的实际 draw work 不再只通过 pass label 间接可见。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：native shadow/depth/mesh draw observability 更完整；完整 backend-real standard 3D pipeline 仍未完成，gbuffer/deferred/post 等 backend-native pass 仍缺。

## 2026-05-19 本轮进展：backend native draw breakdown 进入 FrameProfile

- `FrameProfile` 新增 backend native draw breakdown 字段：mesh pass、depth prepass、shadow total、directional shadow、spot shadow、point shadow、opaque、transparent。
- profiling payload 现在和 `FrameStats`、`FrameCapture`、`FrameDebugReport` 一样能观察 backend native draw work，不再只暴露总 `draw_calls`。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 状态：profiling/tooling 观测面继续收口；完整 renderer goal 仍未完成，backend-native gbuffer/deferred/post 等 standard pass 仍缺。

## 2026-05-19 本轮进展：backend native pass draw 结构化快照

- 新增 `BackendNativePassDrawStats { pass_label, draw_calls }`，并在 `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 中暴露 `backend_native_pass_draws`。
- backend-wgpu metrics 现在把 native pass label 与 draw count 结构化绑定：directional shadow、spot shadow、point shadow、depth prepass、mesh pass。
- 该字段防止 tooling 只能分别读取 `rhi_executed_pass_labels` 和 draw breakdown 后自行推断 native pass work，也能避免 pass label 与 draw stats 漂移。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：backend native pass/draw observability 更完整；完整 renderer goal 仍未完成，gbuffer/deferred/post 等 backend-native standard pass 仍缺。

## 2026-05-19 本轮进展：backend native pass instance 计数

- `BackendNativePassDrawStats` 新增 `pass_instances`，用于表达同名 native pass label 的实际实例数量。
- backend-wgpu native pass draw 快照现在能区分 draw count 与 pass instance count，例如 directional shadow cascades 或 point shadow cube faces 会形成多个同名 native pass instance。
- 新增验证：`cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture` passed，1 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 回归验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 状态：backend native pass/draw observability 更精确；完整 renderer goal 仍未完成，gbuffer/deferred/post 等 backend-native standard pass 仍缺。

## 2026-05-19 本轮进展：native skybox draw 统计透传

- `MeshRenderStats` 新增 `skybox_draw_call_count`，`draw_call_count` 现在包含 skybox draw + shadow draw + depth prepass draw + mesh batch draw。
- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_skybox_draw_calls`，backend-wgpu skybox draw 不再从高层观测面丢失。
- `BackendNativePassDrawStats` 现在分别报告 `Neo Forward Opaque Pass` 的 opaque mesh + skybox draw count，以及 `Neo Transparent Pass` 的 transparent draw count。
- 验证：`cargo test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer wgpu_metrics_ -- --nocapture` passed，2 passed。
- 验证：`cargo test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture` passed，1 passed。
- 验证：`cargo test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture` passed，1 passed。
- 状态：native mesh/skybox/shadow/depth draw observability 更完整；完整 renderer goal 仍未完成，gbuffer/deferred/post 等 backend-native standard pass 仍缺。
## 2026-05-19 本轮进展：backend-wgpu forward/transparent native pass 拆分

- backend-wgpu surface frame 不再用单个 `Neo Mesh Pass` 同时承载 opaque、transparent 和 present 语义；现在真实创建 `Neo Forward Opaque Pass` 与 `Neo Transparent Pass` 两个 wgpu render pass。
- `Neo Forward Opaque Pass` 负责 clear color、load/clear depth、skybox draw 和 opaque mesh draw；MSAA 路径在该 pass 只 store 中间 color，不提前 resolve。
- `Neo Transparent Pass` load opaque pass 的 color/depth 并执行 alpha-blend transparent batches；`Neo Post Process Pass` 再 load color/depth，执行 post-pass hook，并在该最终 pass 处理 resolve/store 到 surface output。
- backend native standard pass coverage 更新为 `Neo Forward Opaque Pass -> forward_opaque`、`Neo Transparent Pass -> transparent/present`；不再通过旧 `Neo Mesh Pass` 映射多个标准 pass。
- `BackendNativePassDrawStats` 现在按真实 native pass 分别输出 `Neo Forward Opaque Pass` 和 `Neo Transparent Pass` 的 draw count，skybox draw 归属 forward opaque pass。
- 状态：forward opaque / transparent / present 的 backend-native pass 语义更接近标准 3D 管线；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。


### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，2 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu actual native pass label 快照

- `render_wgpu::MeshRenderStats` 新增固定容量 actual native pass label 快照，保持 `Copy` 兼容，同时记录本帧真实进入 `begin_render_pass` 的 wgpu pass label 顺序。
- `MeshRenderer::render_batches_with_environment_probes_and_post_pass()` 现在在实际创建 directional/spot/point shadow、depth prepass、forward opaque 和 transparent render pass 时记录 label；这比按 scene/visible count 推导更接近真实 backend 行为。
- `engine_renderer` backend-wgpu frame stats 现在优先使用 `MeshRenderStats` 的 actual native pass labels；仅当旧路径或手写 stats 没有 label 快照时，才回退到 `default_wgpu_pass_labels()`。
- 新增测试覆盖 actual label 优先级和 fallback 行为，避免 graph/debug/capture 继续依赖预估 pass label 作为真实 backend 证据。
- 状态：backend native pass observability 更真实；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_prefer_actual_native_pass_labels_over_default_estimate -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_reports_gpu_time_ms -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：render_wgpu native pass label stats API 验证

- `render_wgpu` 新增底层 stats API 测试，直接覆盖 `MeshRenderStats::record_native_pass_label()` 与 `native_pass_label_strings()`，确认 actual native pass label 顺序能从 backend stats 中导出。
- 新增固定容量边界测试，确认 native pass label 快照超过容量时不会溢出或破坏 stats 结构；这保持 `MeshRenderStats` 的 `Copy` 兼容，同时提供 bounded observability。
- 该验证补齐了上一轮 engine_renderer 层测试的底层证据：上层不再只依赖手写 `MeshRenderStats` fixture，而有 render_wgpu stats API 自身的行为测试。
- 状态：backend native pass label observability 的底层 API 证据更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。

## 2026-05-19 本轮进展：post-pass native draw 归入真实 native pass breakdown

- backend-wgpu queued native pipeline draws 原本已计入 `FrameStats::draw_calls`，但结构化 `BackendNativePassDrawStats` 只统计 material transparent draw，未把 post-pass native draws 归入独立的 `Neo Post Process Pass`。
- 新增 `record_native_post_pass_draws()`，统一更新总 draw call 和 `Neo Transparent Pass` 的 per-pass draw count，避免总数与 per-native-pass breakdown 不一致。
- 新增测试确认 native post-pass draw 会增加 `draw_calls`，并被合并到 `BackendNativePassDrawStats { pass_label: "Neo Post Process Pass" }`。
- 状态：post-pass/native custom draw 的 frame observability 更一致；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。

## 2026-05-19 本轮进展：post-pass draw flat stats 暴露到 profile/debug/capture

- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_post_pass_draw_calls`，用于直接观察 queued native pipeline draw / post-pass custom draw 数量。
- backend-wgpu `record_native_post_pass_draws()` 现在同时更新总 `draw_calls`、flat `backend_post_pass_draw_calls` 与 `Neo Post Process Pass` 的 `BackendNativePassDrawStats`，让总量、平铺字段和 per-pass breakdown 保持一致。
- profile/debug/capture 映射测试已补断言，确认编辑器报告、profiling payload 和 capture payload 不丢该字段。
- 状态：post-pass/native custom draw 的 public observability 更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、post-process pass family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
## 2026-05-19 本轮进展：backend-wgpu 独立 Neo Post Process Pass

- backend-wgpu surface path 现在从 `Neo Transparent Pass` 后拆出真实 `Neo Post Process Pass`：transparent pass 只负责 alpha-blend transparent batches 并 store 中间 color/depth，post-process pass 再 load color/depth、执行 post-pass hook，并在最终 pass 上 resolve/store 到 surface output。
- actual native pass label、`default_wgpu_pass_labels()`、frame stats、debug report 和 graph coverage 现在都包含 `Neo Post Process Pass`。
- standard backend-native coverage 新增 `post_process_resolve -> Neo Post Process Pass`，`present` 也改由最终 `Neo Post Process Pass` 覆盖；`post_process_resolve` 已加入 standard 3D pass label 识别。
- queued native post-pass draw 的 flat `backend_post_pass_draw_calls` 与 per-pass `BackendNativePassDrawStats` 现在归属 `Neo Post Process Pass`，不再混入 `Neo Transparent Pass`。
- 状态：post-process/custom native draw 具备独立 backend-native pass 语义和可观测输出；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。



## 2026-05-19 本轮进展：post-process native pass flat observability

- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_post_process_passes`，直接暴露本帧实际执行的 `Neo Post Process Pass` 实例数。
- `backend_post_process_passes` 与 `backend_post_pass_draw_calls` 分离：前者表达 native post-process pass 是否/执行几次，后者表达该 pass 中 queued native/custom draw 数量。
- backend-wgpu frame stats 由 actual/native pass labels 统计 `Neo Post Process Pass` 实例数，debug/profile/capture 映射测试已补断言，避免 editor/capture 只能从 label list 或 per-pass draw breakdown 间接推断。
- 状态：post-process pass 的 flat observability 更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native standard pass flat instance counters

- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 flat native pass instance counters：`backend_directional_shadow_passes`、`backend_spot_shadow_passes`、`backend_point_shadow_passes`、`backend_depth_prepass_passes`、`backend_forward_opaque_passes`、`backend_transparent_passes`。
- 这些字段与 draw-call counters 分离，表达真实 backend native pass 执行实例数；编辑器、profile 和 capture 不再必须解析 `rhi_executed_pass_labels` 或 `BackendNativePassDrawStats` 才能知道各类 native pass 是否执行。
- backend-wgpu stats 现在从 actual/native pass label 快照统计这些 pass instance counters，并继续保留 per-pass draw breakdown 用于 draw work 归因。
- 状态：standard pass observability 更平铺、更适合 editor/debug/capture 消费；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native pass instance 与 draw-call 分离验证

- 新增 `wgpu_metrics_count_native_pass_instances_separately_from_draw_calls`，用重复 directional shadow cascade、spot shadow、point shadow cube faces、depth/forward/transparent/post passes 的同一帧 fixture 验证 flat pass instance counters 与 draw-call counters 分离。
- 该测试确认 `backend_directional_shadow_passes`、`backend_spot_shadow_passes`、`backend_point_shadow_passes`、`backend_depth_prepass_passes`、`backend_forward_opaque_passes`、`backend_transparent_passes`、`backend_post_process_passes` 只表达 native pass 实例数，不被 draw count 污染。
- 同一测试同时确认 shadow/depth/opaque/transparent/post draw counters 仍表达 draw work，避免 editor/debug/capture 把 repeated pass instance 和 draw workload 混为一类指标。
- 状态：native standard pass observability 的测试证据更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_count_native_pass_instances_separately_from_draw_calls -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。

## 2026-05-19 本轮进展：native pass label 快照截断可观测

- `render_wgpu::MeshRenderStats` 新增 `native_pass_labels_dropped`，当 fixed-capacity actual native pass label 快照超过 `MAX_NATIVE_PASS_LABELS` 时记录被截断数量，不再静默丢失观测信息。
- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_native_pass_labels_dropped`，把 backend label 快照截断情况透传到 editor/debug/profile/capture。
- `mesh_render_stats_native_pass_labels_are_bounded` 现在验证超出容量时 dropped count 增加；`wgpu_metrics_count_native_pass_instances_separately_from_draw_calls` 验证 backend-wgpu stats 会保留该 dropped count。
- 状态：backend native pass label observability 的边界行为更明确；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native pass label 快照容量可观测

- `render_wgpu::MeshRenderStats` 新增 `native_pass_label_capacity()`，显式暴露 actual native pass label 快照容量，避免上层依赖未导出的内部常量。
- `FrameStats`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 新增 `backend_native_pass_label_capacity`，与 `backend_native_pass_labels_dropped` 配套，让 editor/debug/profile/capture 能判断 native pass label 快照是否截断以及截断比例。
- backend-wgpu stats 现在从 `MeshRenderStats::native_pass_label_capacity()` 填充容量字段，避免 hard-code 容量或只暴露 dropped count。
- 状态：native pass label snapshot 的容量与截断行为均可观察；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：native pass label 截断时 graph pass count 修正

- backend-wgpu frame stats 现在在 actual native pass label 快照被截断时，将 `graph.pass_count` / `graph.rhi_executed_passes` 计算为 `recorded labels + backend_native_pass_labels_dropped`，避免只按保留下来的 label 数低估真实 native pass 执行次数。
- `graph.rhi_executed_pass_labels` 仍只保存未截断的 label 快照；`backend_native_pass_label_capacity` 与 `backend_native_pass_labels_dropped` 用于解释 label list 与 pass count 的差异。
- `wgpu_metrics_count_native_pass_instances_separately_from_draw_calls` 已补断言，覆盖 recorded label 数为 13、dropped 为 5 时 `rhi_executed_passes == pass_count == 18` 的边界。
- 状态：native pass label snapshot 截断时 graph stats 不再低估 pass 执行数量；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_count_native_pass_instances_separately_from_draw_calls -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。

## 2026-05-19 本轮进展：backend-wgpu 内建 identity fullscreen post-process draw

- `render_wgpu` 新增 `post_process.wgsl`，提供 fullscreen triangle vertex shader 与 alpha=0 fragment output；配合 alpha blending 形成不改变画面的 identity post-process draw，避免采样/写入同一 render target。
- `MeshRenderer` 新增 `Neo Post Process Color Pipeline` 与 `Neo Post Process Depth Pipeline`，按 post-process pass 是否带 depth attachment 选择；`STATIC_RENDER_PIPELINE_COUNT` 从 26 更新到 28。
- `Neo Post Process Pass` 现在默认执行一个真实 fullscreen draw，再执行 queued custom post-pass hook；`MeshRenderStats::post_process_draw_call_count`、`FrameStats::backend_post_process_draw_calls`、`FrameProfile`、`FrameDebugReport` 与 `FrameCapture` 均可观察该内建 draw。
- `BackendNativePassDrawStats { pass_label: "Neo Post Process Pass" }` 现在包含内建 post-process fullscreen draw，并会继续叠加 queued native/custom post-pass draw。
- 状态：post-process pass 不再只是空 pass 或 custom hook 容器，已有真实 backend-native fullscreen pipeline/draw；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu fullscreen draw 映射到 RenderGraphStats

- backend-wgpu `frame_stats_from_wgpu_metrics()` 现在把 `MeshRenderStats::post_process_draw_call_count` 映射到 `RenderGraphStats::fullscreen_draws`，让 graph stats 也能观察内建 `Neo Post Process Pass` fullscreen work。
- 该字段与 `backend_post_process_draw_calls` 保持一致，但语义不同：`fullscreen_draws` 属于 graph/workload 视角，`backend_post_process_draw_calls` 属于 backend pass draw breakdown 视角。
- `wgpu_metrics_map_gpu_timestamps_to_frame_stats` 和 `wgpu_metrics_count_native_pass_instances_separately_from_draw_calls` 已补断言，确认 backend-native post-process fullscreen draw 不只出现在 draw breakdown，也进入 graph fullscreen draw 统计。
- 状态：post-process fullscreen work 的 graph observability 更完整；完整 renderer goal 仍未完成，gbuffer、deferred lighting、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 等仍缺。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu 真实 Neo GBuffer Pass

- `render_wgpu::MeshRenderer` 现在在 depth prepass 之后、forward opaque 之前执行真实 `Neo GBuffer Pass`，创建 transient offscreen color render target 并用现有 mesh/material pipeline 绘制 opaque batches；该 pass 不读取、不采样、不影响 surface output，先作为 backend-native GBuffer 执行与观测闭环。
- `MeshRenderStats::gbuffer_draw_call_count` 进入总 draw count、actual native pass label snapshot 和 `Neo GBuffer Pass` label 顺序，避免 `gbuffer` 只停留在 facade/RHI semantic pass。
- backend-wgpu `FrameStats` 现在把 `Neo GBuffer Pass` 映射到 `backend_gbuffer_passes`、`backend_gbuffer_draw_calls` 和 `BackendNativePassDrawStats { pass_label: "Neo GBuffer Pass" }`；`default_wgpu_pass_labels()` 也在 visible item 路径中按真实 native order 插入该 pass。
- facade/backend graph merge 现在将 standard `gbuffer` 覆盖到 native `Neo GBuffer Pass`，debug report、profile/capture 字段继续透传 flat native pass/draw counters。
- 状态：`gbuffer` 在 backend-wgpu 已不再只是 label-only/facade 语义；但完整 deferred lighting、MRT GBuffer attachments、GBuffer sampling、完整 post-process shader family、完整 backend pipeline cache 和真实外部 RenderDoc SDK 仍未完成，因此 renderer goal 仍未完成。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu GBuffer MRT attachments

- `render_wgpu` 新增 `gbuffer.wgsl`，使用现有 mesh vertex/instance/material/render bind group 语义输出 3 个 MRT：`@location(0) albedo`、`@location(1) normal`、`@location(2) material`。
- `Neo GBuffer Pass` 现在创建真实 transient MRT render targets：`Neo GBuffer Albedo Texture`、`Neo GBuffer Normal Texture`、`Neo GBuffer Material Texture`，并在 depth prepass 之后以 depth load/read 的方式绘制 opaque batches。
- `MeshRenderer` 新增 GBuffer 单面/双面、带 depth/不带 depth 的 4 条 native render pipelines，`STATIC_RENDER_PIPELINE_COUNT` 从 28 更新为 32；backend-wgpu pipeline cache inventory 测试改为基于 `MeshRenderer::STATIC_RENDER_PIPELINE_COUNT`，避免再次写死陈旧数量。
- `post_process.wgsl` 修复 fullscreen triangle 顶点选择方式，避免 naga/wgpu 0.20 对局部数组动态索引的 shader validation error；隐藏启动 `render_smoke.exe` 已验证 shader/pipeline 创建可通过真实 wgpu 设备路径。
- 状态：GBuffer 已从单 offscreen color target 提升为真实 MRT backend pass；完整 deferred renderer 仍未完成，因为 deferred lighting 尚未采样这些 GBuffer attachments，GBuffer resource export/import 和完整 post-process shader family 也仍未闭合。

### 本轮验证

- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu gbuffer_shader_declares_mrt_outputs -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_renderer_static_pipeline_inventory_is_reported -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_metrics_ -- --nocapture`: passed，4 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p render_wgpu mesh_render_stats_ -- --nocapture`: passed，3 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer default_wgpu_pass_labels_match_native_render_pass_order -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_frame_debug_report_preserves_native_backend_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer backend_native_pass_draw_stats_counts_repeated_pass_instances -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer native_post_pass_draws_are_counted_in_post_process_native_pass_stats -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_builds_stats_from_scene_and_view -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture`: passed，1 passed。
- `C:\Users\JM\.cargo\bin\cargo.exe build --bin render_smoke`: passed。
- hidden smoke launch `target\debug\render_smoke.exe`: passed，3 秒后 `closed:True exit:0`。
## 2026-05-19 backend-wgpu deferred lighting implementation note

`render_wgpu::MeshRenderer` now records a real `Neo Deferred Lighting Pass` after `Neo GBuffer Pass`.
The pass samples the resolved single-sample GBuffer MRT textures (`albedo`, `normal`, `material`) through `deferred_lighting.wgsl` and writes a transient `Neo Deferred Lighting Texture`.
The GBuffer pass now creates sampleable single-sample MRT targets and uses MSAA render attachments with resolve targets when the renderer sample count is greater than one.

Observable evidence added in this slice:

- `MeshRenderStats::deferred_lighting_draw_call_count`.
- `FrameStats::backend_deferred_lighting_draw_calls`.
- `FrameStats::backend_deferred_lighting_passes`.
- `FrameProfile`, `FrameDebugReport`, and `FrameCapture` mirror the deferred lighting backend stats.
- `backend_native_pass_draws` includes `Neo Deferred Lighting Pass`.
- `RenderGraphStats::fullscreen_draws` counts both deferred lighting and post-process fullscreen draws.
- Standard pass backend coverage maps semantic `deferred_lighting` to native `Neo Deferred Lighting Pass`.

Update: the deferred lighting target is now consumed by a sampled `Neo Post Process Pass`. The sampled post-process path reads `Neo Deferred Lighting Texture` and blends the lit opaque result into the final surface while preserving existing forward-rendered skybox and transparent content outside GBuffer-covered pixels.

Update: when facade `ViewQualitySettings::fxaa` is enabled, backend-wgpu forwards that quality flag to the sampled post-process path and exposes the native label `Neo Fxaa Tonemap Post Process Pass`. That pass samples the deferred lighting output, applies a simple FXAA filter before tonemap/gamma output, and maps to semantic `fxaa`, `tonemap`, `post_process_resolve`, and `present`. This is a minimal FXAA backend path, not completion of the full post-process family.

Update: when facade `ViewQualitySettings::bloom` is enabled, backend-wgpu forwards that quality flag to the sampled post-process path and exposes `Neo Bloom Tonemap Post Process Pass` or `Neo Bloom Fxaa Tonemap Post Process Pass`. The shader adds a small HDR bright-neighbor bloom contribution before tonemap/gamma output. This is a minimal single-pass bloom path; a production multi-resolution bloom chain with separate output resources and artist-facing parameters is still future work.

Update: when facade `ViewQualitySettings::color_grading` is `ColorGradingMode::Lut`, backend-wgpu forwards that mode to the sampled post-process path and exposes native labels containing `Color Grading`. The shader applies a small post-tonemap grading curve before gamma output. This is only a minimal backend-visible color grading path; the full documented LUT workflow still requires user-provided LUT texture resources, validation, shader sampling, and public artist controls.

Update: facade `ViewQualitySettings::taa`, `motion_blur`, `ssr`, and `depth_of_field` now reach backend-wgpu through `WgpuPostProcessOptions`. The sampled post-process shader executes minimal single-pass sampled branches for those effects and emits a dynamic native label containing the enabled effect names. This provides backend-visible execution and stats/debug/capture observability, but it is not the final production implementation for TAA history/reprojection, velocity-buffer motion blur, depth/normal SSR ray marching, or CoC-based depth of field.

Update: facade `ViewQualitySettings::ssao` now reaches backend-wgpu through `WgpuPostProcessOptions`. The sampled post-process shader executes a minimal local-contrast AO-style darkening branch and emits dynamic native labels containing `Ssao`. This provides backend-visible execution and observability, but it is not the final production SSAO implementation because there is no separate depth/normal AO pass, AO blur, deferred-lighting AO input, or public AO parameter API yet.

Update: facade `ViewQualitySettings::hdr` now reaches backend-wgpu through `WgpuPostProcessOptions`. The sampled post-process shader executes a small HDR exposure step and emits dynamic native labels containing `Hdr`. This gives HDR mode backend-visible execution and observability, but full HDR display support still requires HDR surface format negotiation, render-target policy, exposure/white-point controls, and display mapping.

Update: external frame capture hooks can now be real callbacks. `Renderer::register_frame_capture_backend_callback` registers external capture metadata plus a callable hook, and frame finish invokes it with `FrameCaptureHookEvent` when a queued RenderDoc or external-debugger capture reaches `BackendHookRequested`. This is a real user-supplied handoff path, but it is not a built-in RenderDoc SDK binding; native SDK invocation remains the responsibility of the callback.

Update: `FrameCapture::external_hook_callback_invoked` and `FrameDebugReport::capture_external_hook_callback_invoked` make callback invocation directly observable. Tools can now distinguish metadata-only external hook handoff from an actual callable callback invocation.

## 2026-05-19 本轮进展：external capture callback failure observability

- `FrameCaptureStatus` 新增 `BackendHookFailed`，用于表达外部 capture callback 在 frame finish handoff 期间 panic 的可观察失败状态。
- `FrameCapture` 新增 `external_hook_callback_failed` 与 `external_hook_callback_failure`，成功、metadata-only、unregistered 和 panic callback 路径都能区分。
- `FrameDebugReport` 新增 `capture_external_hook_callback_failed` 与 `capture_external_hook_callback_failure`，editor/debug report 不再只能看到 callback 是否被调用，还能看到 callback handoff 失败原因。
- `Renderer::register_frame_capture_backend_callback` 的 callback 调用现在由 renderer 捕获 panic；失败不会让 frame finish 崩溃，而是记录为 `BackendHookFailed` 和 panic payload 文本。
- 状态：external capture hook callback handoff 的成功/失败 observability 已闭合；真实 RenderDoc/external debugger SDK 加载、begin/end capture 调用仍是外部阻塞，不能计为完整 renderer 层完成。

验证：`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer capture -- --nocapture` passed，2 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph imported resource observability

- `RenderGraphStats` 新增 `imported_textures`、`imported_buffers`、`imported_texture_labels`、`imported_buffer_labels`。
- `RenderGraphBuilder::compile()` 和 fallback `stats()` 都会统计 imported renderer resources；labels 会排序输出，避免 HashMap 迭代顺序影响调试/测试。
- imported texture/buffer 现在不只存在于 builder 内部和 RHI import map，也会随 `FrameStats.graph`、`FrameDebugReport.graph`、`FrameCapture.graph` 进入统一观测面。
- 状态：RenderGraph resource import 的 stats/debug/capture observability 已增强；完整 resource export、跨帧 transient export 和 backend-wgpu graph resource export/import 仍是 renderer goal 剩余缺口。

验证：`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph resource export markers and stats

- `RenderGraphBuilder` 新增 `export_texture()` 与 `export_buffer()`，允许 graph extension 显式标记 texture/buffer 作为 graph 输出资源。
- `RenderGraphStats` 新增 `exported_textures`、`exported_buffers`、`exported_texture_labels`、`exported_buffer_labels`，并与 imported resource stats 一起进入 frame/debug/capture 观测面。
- Exported labels 采用排序输出，保证 debug/capture/test 中的 graph resource output 快照稳定。
- Graph validation 现在会拒绝导出不存在于当前 graph 的 texture/buffer，避免 export 只成为无效 label bookkeeping。
- 状态：RenderGraph resource export 的最小 public API、错误路径和 observability 已实现；完整跨帧 exported transient lifetime、由 export 结果驱动后续 facade resource、以及 backend-wgpu graph resource export/import 执行仍未闭合。

验证：`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。

### 2026-05-19 补充：RenderGraph export lifetime semantics

- Exported graph resources now extend their compiled `ResourceLifetime::last_pass` to the final graph pass, so export is reflected in lifetime planning instead of remaining a stats-only marker.
- Graph validation rejects export declarations on an otherwise empty graph, because there is no frame work that can produce or preserve the exported output.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph compiled export list

- `CompiledRenderGraph` 新增 `resource_exports: Vec<CompiledResourceExport>`，结构化记录每个 exported graph resource，而不是要求工具或 backend 从 stats label 反推。
- `CompiledResourceExport` 包含 `GraphResource` 与 export label，并按 texture/buffer 与 graph id 稳定排序。
- `engine_renderer` crate root re-export `CompiledResourceExport`，但 prelude 仍不暴露 graph/RHI 类型，保持游戏层 import 边界。
- 状态：RenderGraph export 现在有 public marker、validation、stats、lifetime 和 compile artifact；下一步仍需把 compile artifact 接到 durable facade resource 或 backend-wgpu graph output。

验证：`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph RHI execution exports

- Added `RhiGraphExecution { stats, exports }` as an RHI execution result that preserves the existing stats while returning materialized exported RHI resources.
- Added `RhiResourceExports`, `RhiTextureExport`, and `RhiBufferExport` to carry exported graph texture/buffer ids, labels, and actual `RhiTexture` / `RhiBuffer` handles.
- Added `RenderGraphBuilder::execute_on_rhi_with_exports()` and `execute_on_rhi_with_imports_exports()`; existing `execute_on_rhi*` methods remain compatible and return stats only.
- RHI execution now maps `CompiledResourceExport` records to materialized imported or transient RHI handles and reports validation errors if an export was not materialized.
- Status: RenderGraph export now has marker, stats, lifetime, compile artifact, and RHI execution output. Remaining work is connecting this to durable public renderer resources and backend-wgpu graph/surface integration.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture` passed，1 passed。

### 2026-05-19 补充：RHI transient resource exports

- `execute_on_rhi_with_exports()` now covers transient graph resources as well as imported resources: exported transient textures/buffers are returned with their actual materialized `RhiTexture` / `RhiBuffer` handles after graph execution.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_maps_imported_resources -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer prelude_keeps_graph_and_rhi_types_out_of_game_layer_imports -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph import/export stats aggregation

- `accumulate_graph_stats()` now accumulates imported/exported texture and buffer counts and extends their label snapshots.
- This prevents frame-level aggregation across views or frame graph extensions from dropping RenderGraph import/export observability that individual graph tests already produce.
- `FrameStats.graph`, `FrameDebugReport.graph`, and `FrameCapture.graph` now preserve import/export counts and labels after multi-graph accumulation.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer accumulate_graph_stats_preserves_import_export_resource_observability -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：facade/backend graph import-export merge

- `merge_facade_and_backend_graph_stats()` now preserves RenderGraph import/export resource counts and label snapshots from both facade graph stats and backend graph stats.
- This keeps future backend-wgpu graph/surface export integration from losing imported/exported resource observability when backend stats are merged into the public frame stats.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer facade_backend_graph_merge_preserves_semantic_and_native_execution_stats -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer accumulate_graph_stats_preserves_import_export_resource_observability -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph export label validation

- RenderGraph export labels now participate in validation: empty exported texture/buffer labels are rejected, and duplicate export labels across texture/buffer outputs are rejected.
- This makes export labels usable as stable identifiers for tooling, RHI export lookup, and future durable facade resource promotion instead of best-effort debug strings.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RHI export label lookup

- `RhiResourceExports` now provides label-based lookup helpers: `texture_export()`, `buffer_export()`, `texture()`, and `buffer()`.
- These helpers rely on the export-label validation added earlier, so advanced RenderGraph/RHI callers can consume exported outputs without manually scanning vectors or depending on insertion order.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph extension export observability through facade

- Public `RenderGraphExtension` code can now export transient graph texture/buffer outputs and have those exports appear in `FrameStats.graph` and `FrameDebugReport.graph` after a normal `Renderer::begin_frame().render_view().finish()` path.
- This proves export observability is no longer limited to isolated `RenderGraphBuilder` unit tests or direct RHI execution helpers.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_exports_transient_resources -- --nocapture` passed，1 passed。

### 2026-05-19 补充：RenderGraph extension exports in frame capture

- The facade-level RenderGraph extension export test now also queues an internal frame capture and verifies `FrameCapture.graph` preserves exported texture/buffer counts and labels.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：profiled facade RenderGraph exports

- Public `RenderGraphExtension` exports remain visible when the renderer takes the GPU-profiler/headless-RHI execution path instead of the non-RHI stats path.
- The profiled facade test verifies exported texture labels survive into `FrameStats.graph`, `FrameDebugReport.graph`, and `FrameCapture.graph`, while GPU profiling still reports a GPU time value.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer profiled_render_graph_extension_exports_remain_visible_in_frame_outputs -- --nocapture` passed，1 passed。

### 2026-05-19 graph/export regression suite

- After the RenderGraph import/export API, validation, aggregation, facade observability, and profiled facade path updates, the focused graph test suite was run.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_ -- --nocapture` passed，38 passed。

## 2026-05-19 本轮进展：RenderGraph resource label summaries

- `RenderGraphStats` now exposes structured resource-label helpers: `imported_resource_labels()`, `exported_resource_labels()`, `has_resource_imports()`, and `has_resource_exports()`.
- Added `RenderGraphResourceLabels { textures, buffers }` so facade users can consume graph import/export labels without manually combining texture and buffer label vectors.
- The type is re-exported from `engine_renderer` crate root while remaining outside the game-layer prelude boundary.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_exports_are_visible_in_frame_stats_and_debug_report -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：RenderGraph export validation through facade

- Duplicate export labels declared by a public `RenderGraphExtension` now surface through the normal renderer facade frame path as `RendererError::RenderGraphValidation`.
- This closes the error-path observability gap between direct `RenderGraphBuilder::compile()` validation and `Renderer::begin_frame().render_view().finish()` usage.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer render_graph_extension_rejects_duplicate_export_labels_through_facade -- --nocapture` passed，1 passed；`C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer builder_imports_external_texture_and_buffer_handles -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：wgpu RHI RenderGraph exports

- `execute_on_rhi_with_exports()` is now covered on a real `WgpuRhiDevice`, not only the headless RHI implementation.
- The wgpu test creates transient graph texture/buffer resources, exports them, executes the graph on wgpu, and verifies the exported graph ids can be looked up from `RhiResourceExports` by label.
- Status: backend-wgpu graph-level RHI export materialization is covered; backend-wgpu surface/standard-frame integration still does not produce durable public renderer export handles.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_transient_resources -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：pipeline cache backend-object coverage helpers

- `PipelineCacheStats` now exposes backend-object coverage helpers: `ready_backend_object_gap()`, `used_backend_object_gap()`, `all_ready_entries_have_backend_objects()`, `all_used_entries_have_backend_objects()`, and `has_complete_facade_backend_object_coverage()`.
- These helpers make the current `CompleteBackendPipelineCache` gap directly observable through public API: facade-ready pipeline entries without backend objects can be detected without reinterpreting raw counter fields.
- Status: this improves public observability and tooling safety; it does not mark complete backend-native pipeline cache coverage as implemented.

Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_warmup_validates_pipeline_keys -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：pipeline cache merge coverage semantics

- The facade/backend pipeline cache merge test now explicitly verifies that backend inventory (`backend_objects`) does not mask facade entries that are ready but still lack backend objects.
- `PipelineCacheStats::has_complete_facade_backend_object_coverage()` remains false when facade gap counters are non-zero, even if backend-wgpu reports native backend objects.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer pipeline_cache_stats_merge_preserves_facade_counts_and_backend_inventory -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：backend-wgpu pipeline cache coverage merge semantics

- backend-wgpu pipeline cache merge tests now cover the backend-object coverage helper semantics.
- Native backend pipeline inventory increases `backend_objects`, but existing facade gap counters (`ready_entries_without_backend_object`, `used_entries_without_backend_object`) remain authoritative for `has_complete_facade_backend_object_coverage()`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_pipeline_cache_stats_merge_preserves_static_and_native_inventory -- --nocapture` passed，1 passed。

## 2026-05-19 本轮进展：pipeline cache coverage helpers in debug/capture outputs

- `FrameDebugReport.pipeline_cache` and `FrameCapture.pipeline_cache` now have test coverage proving the backend-object coverage helper methods produce the same results as `FrameStats.pipeline_cache`.
- This confirms editor/debug/capture consumers can use the public helper methods directly from their payloads instead of only from immediate frame stats.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed，1 passed。

### Frame debug/capture pipeline backend-object coverage

- 2026-05-19: `PipelineCacheStats` now exposes explicit facade/backend coverage helpers for editor diagnostics: `ready_backend_object_gap`, `used_backend_object_gap`, `all_ready_entries_have_backend_objects`, `all_used_entries_have_backend_objects`, and `has_complete_facade_backend_object_coverage`.
- `FrameDebugReport.pipeline_cache` and `FrameCapture.pipeline_cache` preserve those helper results, so tools can distinguish facade-visible warmup readiness from concrete backend/native pipeline-object residency.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_summarizes_last_frame_for_editor -- --nocapture` passed: 1 test.

### Public RenderGraph export resource promotion

- 2026-05-19: `Renderer::execute_graph_to_resources` executes a public `RenderGraphBuilder` through the active RHI path and promotes exported transient graph resources into durable public renderer handles.
- Exported transient textures are read back through RHI and created as public `TextureHandle` resources; exported transient buffers are read back and created as public `BufferHandle` resources. Exported imported renderer resources resolve to their original public handles instead of duplicating resources.
- `RendererGraphExecution`, `RendererGraphResourceExports`, `RendererGraphTextureExport`, and `RendererGraphBufferExport` expose the promoted handles, export labels, graph ids, and whether the handle was newly promoted.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_exported_transients_to_public_handles -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

### Public RenderGraph imported resource upload/writeback

- 2026-05-19: `Renderer::execute_graph_to_resources` now uploads imported renderer buffer/texture data into the RHI resources before graph execution, so graph callbacks see the current public resource contents instead of zero-initialized import mirrors.
- When an imported renderer buffer or texture is exported by the graph, the RHI result is written back to the original public handle and the export record reports `promoted: false`. Transient exports still materialize new public handles with `promoted: true`.
- Current texture import/writeback support is intentionally explicit: single-sample 2D base-level `Rgba8Unorm`, `Rgba16Float`, and `Rgba32Float` resources use RHI upload/readback; unsupported imported texture formats or shapes return a render-graph validation error instead of silently reporting stale public data.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_uploads_and_writes_back_imported_exports -- --nocapture` passed: 1 test. Regression checks also passed for `execute_graph_to_resources_promotes_exported_transients_to_public_handles` and `graph_execute_on_rhi_`.

### Public graph Depth32Float import/writeback

- 2026-05-19: RHI now exposes `write_texture_depth32f`, implemented for headless and backend-wgpu RHI devices, so public graph execution can upload and write back depth textures instead of treating depth as readback-only.
- `Renderer::execute_graph_to_resources` now supports imported single-sample 2D base-level `Depth32Float` textures for upload, graph writes, export writeback, and public `texture_bytes` observability.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_uploads_and_writes_back_depth_imports -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 3 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

### Public graph 8-bit sRGB/BGRA export promotion

- 2026-05-19: RHI `write_texture_rgba8` / `read_texture_rgba8` now support the renderer's 8-bit color formats: `Rgba8Unorm`, `Rgba8UnormSrgb`, and `Bgra8UnormSrgb`.
- `Renderer::execute_graph_to_resources` can now promote exported transient `Rgba8UnormSrgb` and `Bgra8UnormSrgb` graph textures into durable public `TextureHandle` resources with format and byte contents preserved.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_promotes_8bit_srgb_and_bgra_exports -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 4 tests. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_rhi_ -- --nocapture` passed: 3 tests.

### Public graph unsupported texture-shape errors

- 2026-05-19: `Renderer::execute_graph_to_resources` now rejects imported texture upload/writeback for unsupported shapes with explicit `RenderGraphValidation` errors instead of letting mip/array/MSAA resources enter the RHI mirror path with ambiguous data semantics.
- The currently supported public graph texture data path is single-sample 2D, single-layer, base-level textures for supported read/write formats. Mipped, array/cube/3D, and resolved MSAA imports have compatibility paths; native sample-level MSAA graph textures remain future work and fail through explicit capability/validation boundaries.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_rejects_unsupported_imported_texture_shapes -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

### Last public graph execution query surface

- 2026-05-19: `Renderer::last_graph_execution` and `Renderer::last_graph_resource_exports` expose the most recent successful `Renderer::execute_graph_to_resources` result for tools that need to inspect promoted graph export handles after the call site has returned.
- Failed graph execution clears the last public graph result, so editor/debug tooling cannot accidentally read stale export handles from an earlier successful graph.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer last_graph_execution_tracks_public_export_handles_and_clears_on_failure -- --nocapture` passed: 1 test. `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_ -- --nocapture` passed: 5 tests.

### Frame debug/capture public graph export observability

- 2026-05-19: `FrameDebugReport::public_graph_execution` and `FrameCapture::public_graph_execution` now expose the most recent successful `Renderer::execute_graph_to_resources` result, including promoted public texture/buffer export handles.
- This gives editor/debug/capture tooling a frame-adjacent observation path for explicit public graph exports without requiring callers to retain the immediate return value themselves.
- Failed public graph execution still clears `Renderer::last_graph_execution`, so these frame/capture fields do not report stale handles after graph validation or execution errors.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed: 1 test. `frame_debug_report_` passed: 3 tests. `execute_graph_to_resources_` passed: 5 tests.

### FrameStats public graph export observability

- 2026-05-19: `FrameStats::public_graph_execution` now mirrors the latest successful explicit public graph execution, so promoted public graph export handles are visible directly from the frame return value as well as debug reports and captures.
- `FrameCapture::public_graph_execution` now mirrors the `FrameStats` field, keeping frame stats, debug report, and capture payloads on the same source of truth.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_debug_report_and_capture_expose_public_graph_export_handles -- --nocapture` passed: 1 test. `frame_debug_report_` passed: 3 tests. `execute_graph_to_resources_` passed: 5 tests.

### Frame capture resource dump graph export counts

- 2026-05-19: `FrameCaptureResourceDump` now reports explicit public graph export counts: exported textures/buffers, promoted texture/buffer exports, and imported texture/buffer exports.
- The resource dump remains a lightweight count summary; concrete public handles stay available through `FrameStats::public_graph_execution`, `FrameDebugReport::public_graph_execution`, and `FrameCapture::public_graph_execution`.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_capture_resource_dump_counts_public_graph_exports -- --nocapture` passed: 1 test. `resource_dump` passed: 2 tests. `execute_graph_to_resources_` passed: 5 tests.

### Public graph export handle lifetime cleanup

- 2026-05-19: destroying a public texture or buffer handle that is referenced by the latest explicit public graph execution now clears `Renderer::last_graph_execution`, preventing stats/debug/capture tooling from reporting stale graph export handles.
- Destroying an older graph export handle does not clear a newer graph execution result unless that newer result references the same handle.
- The memory stats regression for destroyed resources now matches the current submitted-frame behavior: a submitted frame reports `ResourceReclaimPolicy::SubmissionBoundary` and current-frame reclaimed resource counts instead of the older frame-latency delayed count.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer destroying_public_graph_export_handles_clears_last_graph_execution -- --nocapture` passed: 1 test. `frame_stats_report_resident_memory_and_delayed_destroy_count` passed: 1 test. `destroy` passed: 13 tests. `execute_graph_to_resources_` passed: 5 tests.

### Durable public frame outputs for headless/texture targets

- 2026-05-19: `FrameStats::public_frame_outputs`, `FrameDebugReport::public_frame_outputs`, and `FrameCapture::public_frame_outputs` expose durable public color-output handles for non-surface frame targets.
- Headless targets now create a public `TextureHandle` marked as `FramePublicOutputSource::HeadlessGenerated`; texture, texture-view, and external render-target views report their existing public texture handle with a source enum instead of duplicating resources.
- Surface targets still report no public frame output handle. This avoids presenting backend-wgpu swapchain output as a durable public texture until a real backend surface export path exists.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_are_durable_public_textures -- --nocapture` passed: 1 test. `resource_dump` passed: 2 tests. `frame_debug_report_` passed: 3 tests. `frame_builds_stats_from_scene_and_view` passed: 1 test.

### Headless public frame output clear-color data

- 2026-05-19: headless `FramePublicOutputSource::HeadlessGenerated` textures now initialize their public `texture_bytes` from `CameraDesc::clear` instead of zero-filling unconditionally.
- `ClearOptions::ColorDepth` writes a solid color texture in the target format; `DepthOnly` and `None` retain deterministic black/empty-color bytes. This makes the durable headless public frame output carry real frame clear semantics. Backend-wgpu surface outputs are handled by the later surface-readback path when COPY_SRC readback is supported.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_are_durable_public_textures -- --nocapture` passed: 1 test. `resource_dump` passed: 2 tests. `frame_builds_stats_from_scene_and_view` passed: 1 test.

### Public texture-target frame clear writeback

- 2026-05-19: non-surface public frame outputs now write camera clear color into direct public texture targets and external render-target color textures, not only generated headless outputs.
- `RenderTarget::Texture` reports `FramePublicOutputSource::ExistingTargetTexture` and updates the referenced public texture bytes for single-sample targets. `RenderTarget::External` reports `ExistingExternalRenderTarget` and updates the external target's public color texture bytes on the same path.
- Texture-view targets still report the existing public texture handle, but subresource byte writeback remains future work to avoid overwriting unrelated mips/layers.
- Validation: `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_frame_outputs_write_clear_color_to_existing_public_texture -- --nocapture` passed: 1 test. `frame_outputs` passed: 3 tests. `resource_dump` passed: 2 tests.

### Frame public output writeback update

The renderer now exposes durable public frame outputs for non-surface targets through `FramePublicFrameOutput` on frame stats, debug reports, and captures. Headless views create a public texture containing the camera clear color. `RenderTarget::Texture` and external render targets report their existing public color texture handles and write the camera clear color into those handles for CPU readback and downstream renderer API use.

`RenderTarget::TextureView` reports the owning texture handle and view-adjusted extent, and writes camera clear color bytes into the selected single-mip, 2D-compatible mip/layer range for CPU readback and downstream renderer API use. Main-surface and swapchain-backed views still do not fabricate a durable public texture because the presentation image is backend-owned.

Validated with targeted engine_renderer tests:

- `cargo test -p engine_renderer texture_frame_outputs_write_clear_color_to_existing_public_texture -- --nocapture`
- `cargo test -p engine_renderer frame_outputs -- --nocapture`
- `cargo test -p engine_renderer resource_dump -- --nocapture`


### TextureView public frame output writeback update

`RenderTarget::TextureView` now participates in public frame output writeback for 2D-compatible, single-mip views. The renderer records the existing owner texture handle as the public output, computes the base-mip extent, and writes the camera clear color bytes for the selected mip/layer range into the texture backing data.

Current limitation: this is still clear-color writeback, not full shaded frame readback. Multi-mip view output and complete surface/swapchain public output remain outside this slice.

Validated with targeted engine_renderer tests:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_outputs_write_clear_color_to_target_subresource -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture`

### Public frame output subresource metadata update

`FramePublicFrameOutput` now includes subresource metadata for every public frame output: `base_mip`, `mip_count`, `base_layer`, and `layer_count`. Headless, direct texture, and external render-target outputs report the default base subresource. `RenderTarget::TextureView` outputs report the actual view subresource so stats, debug reports, captures, and downstream public API consumers can identify the exact mip/layer range represented by the output handle.

Validated with targeted engine_renderer tests:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer texture_view_frame_outputs_write_clear_color_to_target_subresource -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture`

### Public frame output scene/material preview update

Public frame output bytes are no longer limited to camera clear color when a view contains visible geometry. For headless, direct texture, texture-view, and external render-target public outputs, the renderer now derives a deterministic scene/material preview color from the actual view: visibility, layer filtering, LOD-selected resources, standard material `base_color`, optional `base_color_texture` average color, and emissive contribution. Empty views still write the camera clear color.

This is a renderer-layer observable preview path, not a complete rasterized shaded-frame readback. It closes part of the clear-only output gap by making public output bytes depend on real scene/material state.

Validated with targeted engine_renderer tests:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_use_base_color_texture_average -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture`

### Public frame output content provenance update

`FramePublicFrameOutput` now reports the provenance of its bytes. The new `FramePublicOutputContent` enum distinguishes `ClearColor` output from `SceneMaterialPreview` output. The payload also includes `visible_geometry` and `material_samples`, allowing stats, debug reports, captures, and public API consumers to verify whether a durable output texture came from an empty clear-only view or from actual visible scene/material state.

Validated with targeted engine_renderer tests:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_use_base_color_texture_average -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture`

### Public frame output lighting/environment preview update

The public frame output preview path now also accounts for basic standard-3D scene context. When visible geometry contributes to `SceneMaterialPreview`, the preview tint includes view-layer-matched lights, optional environment diffuse/background contribution, and manual exposure. `FramePublicFrameOutput` now reports `light_samples` and `environment_samples` alongside visible geometry and material sample counts.

This remains a deterministic renderer-layer preview, not per-pixel shaded readback. It improves public output observability by making the durable output texture and metadata respond to lights, environment, and exposure state already represented by the public facade.

Validated with targeted engine_renderer tests:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer headless_frame_outputs_include_light_environment_and_exposure_preview -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer frame_outputs -- --nocapture`

### Public frame output post-process preview observability (2026-05-19 18:00:29 +08:00)

Public frame outputs now expose deterministic post-process preview provenance through FramePublicFrameOutput::post_process_samples. The preview path applies render-path tonemapping plus enabled quality features such as bloom, motion blur attenuation, and LUT-style color grading as deterministic CPU-side preview effects.

This is intentionally an observability preview, not a claim that the real GPU post-process chain has been read back. It lets headless stats, debug reports, captures, and public output bytes reflect configured render-path and post-process intent while the full renderer backend work continues.

### Public graph imported mipped 2D texture base-mip execution (2026-05-19 18:05:59 +08:00)

Renderer::execute_graph_to_resources supports imported public TextureDimension::D2 textures with multiple declared mip levels for the explicit public graph path by uploading, executing, and writing back the base mip through the current RHI texture abstraction. The renderer preserves the public texture descriptor's declared mip_levels; the graph import/export data flow updates the base-mip byte payload and records the export as the original public texture handle rather than a promoted replacement.

This is a base-mip compatibility slice. The current RHI texture import/export API is still two-dimensional and single-subresource, so true native multi-mip subresource execution/readback and native sample-level MSAA graph textures remain separate renderer-layer gaps.

### Public graph generated-mip texture base upload (2026-05-19 18:08:01 +08:00)

When a public D2 texture has generated mip data, explicit graph import now extracts the compact base mip from the generated mip chain for current RHI upload. If the graph writes the imported texture and exports it, the renderer writes the updated base mip back to the original public texture handle and clears mips_generated so callers can regenerate derived mip levels explicitly.

This keeps generated-mip imports usable without pretending that the current RHI path can address every mip level. Full mip-subresource selection remains a future RHI/API expansion.

### Headless surface public frame output observability (2026-05-19 18:11:15 +08:00)

Headless/stub surface targets now produce durable public frame output textures through FramePublicFrameOutput. RenderTarget::MainSurface without an active wgpu runtime reports FramePublicOutputSource::HeadlessMainSurfaceGenerated, and a valid stub RenderTarget::Surface(handle) reports FramePublicOutputSource::HeadlessSurfaceGenerated. These outputs use the resolved surface extent and target color format, carry the same preview provenance fields as other public frame outputs, and expose durable TextureHandle bytes for stats/debug/capture consumers.

This does not claim real swapchain readback for the headless/stub surface path. When a wgpu runtime owns the surface, public surface output is handled by the later opt-in backend-wgpu surface readback path.

### Public frame output multi-mip texture-view preview (2026-05-19 18:15:25 +08:00)

RenderTarget::TextureView now accepts a non-zero mip range instead of requiring exactly one mip. Public frame output writeback packs deterministic preview bytes for each selected mip in increasing mip order, using the selected layer range for every mip. FramePublicFrameOutput continues to report the base mip, mip count, base layer, and layer count so stats, debug reports, and captures can identify the exact view range represented by the payload.

Because the current stored texture upload layout can describe only one subresource, multi-mip frame-output payloads are treated as packed public preview/readback bytes rather than as a reusable single-layout RHI upload. True per-mip/per-layer RHI subresource execution remains a separate renderer-layer requirement.

### Public frame output subresource byte layout (2026-05-19 18:18:02 +08:00)

FramePublicFrameOutput now includes subresources: Vec<FramePublicFrameOutputSubresource>. Each entry reports the represented mip level, selected layer range, mip extent, byte offset, byte length, bytes per row, and rows per image within the public output byte payload. This makes packed texture-view output bytes directly inspectable by stats, debug report, and capture consumers without requiring them to recompute layout from texture metadata.

For single-subresource outputs the layout contains one entry. For multi-mip texture-view preview outputs, entries are ordered by increasing mip level and match the packed byte order. This is public-output observability; true RHI/backend subresource execution remains a separate requirement.

### Public graph texture export descriptor metadata (2026-05-19 18:20:16 +08:00)

RendererGraphTextureExport now carries descriptor metadata for the exported public texture: dimension, width, height, depth/layer count, mip count, sample count, format, and usage. This metadata is populated for both transient graph resources promoted into durable public textures and imported public textures written back through Renderer::execute_graph_to_resources.

Callers no longer need an immediate secondary 	exture_info() query just to understand what an explicit graph export represents. The texture handle remains the authoritative resource identity; the metadata is an export-time snapshot for tooling, debug reports, captures, and graph integration checks.

### Public graph buffer export descriptor metadata (2026-05-19 18:22:37 +08:00)

RendererGraphBufferExport now carries descriptor metadata for exported public buffers: byte size and usage flags. The metadata is populated for both transient graph buffers promoted into durable public buffers and imported public buffers written back through Renderer::execute_graph_to_resources.

This mirrors texture export descriptor observability and gives graph tooling, debug reports, and capture consumers enough export-time metadata to reason about buffer outputs without an immediate buffer_info() lookup. The public buffer handle remains the authoritative resource identity.

### Public graph texture export represented subresource layout (2026-05-19 18:25:44 +08:00)

RendererGraphTextureExport now includes subresources describing which texture subresource bytes are represented by an explicit public graph export. Each RendererGraphTextureExportSubresource records mip level, layer range, region offset, extent, byte offset, byte length, bytes per row, and rows per image.

For current explicit graph execution, promoted transient texture exports and imported public texture exports represent one base-mip 2D subresource. Imported public textures can still report a larger descriptor mip count when the public texture owns multiple mips; the subresources field makes it explicit that only base mip bytes were uploaded/read back through the current RHI path.

### Public graph buffer export represented byte range (2026-05-19 18:27:59 +08:00)

RendererGraphBufferExport now reports the represented byte range for explicit public graph buffer exports through byte_offset and byte_len. Current explicit graph exports represent the full buffer, so byte_offset is 0 and byte_len equals size for both promoted transient buffers and imported public buffers written back through Renderer::execute_graph_to_resources.

This makes buffer export payloads self-describing in the same way texture exports now describe represented subresources. Future partial buffer export support can use the same fields without changing the public payload shape.

### Public graph export source provenance (2026-05-19 18:30:31 +08:00)

RendererGraphTextureExport and RendererGraphBufferExport now carry RendererGraphExportSource. The source distinguishes graph transients promoted into durable public resources from imported public resources written back through Renderer::execute_graph_to_resources. The existing promoted boolean remains for compatibility, while source provides explicit provenance for tooling, debug reports, captures, and integration checks.

Current sources are PromotedTransient, ImportedPublic, BackendMainSurfaceReadback, and BackendSurfaceReadback. This makes public graph export payloads self-describing without requiring callers to infer semantics from a boolean alone.

### Public graph texture export subresource coverage flags (2026-05-19 18:33:14 +08:00)

RendererGraphTextureExport now reports complete_mip_coverage, complete_layer_coverage, and complete_subresource_coverage. These flags summarize whether the represented export subresources cover the exported texture descriptor's declared mip and layer space.

For current explicit graph execution, single-mip D2 promoted or imported exports report complete coverage. Imported public D2 textures with multiple declared mips but only base-mip RHI representation report complete_layer_coverage true, complete_mip_coverage false, and complete_subresource_coverage false. This makes partial export coverage explicit in tooling and capture payloads.

### Public graph imported texture subregion upload coverage (2026-05-19 18:35:17 +08:00)

Renderer::execute_graph_to_resources preserves TextureUpdate subregion upload layout when importing public textures into explicit graph execution. A public texture whose latest stored payload covers only a subregion is uploaded to the RHI texture at the declared offset and extent before graph passes run. If the graph then writes and exports the imported texture, the public texture payload is replaced with full-texture readback bytes and the export payload reports the represented full base-mip subresource.

This closes the explicit graph path for 2D single-sample subregion upload into base-mip RHI execution. It does not add public API for graph passes to export arbitrary subregions; current explicit graph texture export still reads back the represented base mip.

### Public graph imported buffer subrange update coverage (2026-05-19 18:37:57 +08:00)

Renderer::execute_graph_to_resources now has focused coverage for public buffers that were modified through BufferUpdate at a non-zero byte offset before graph import. BufferUpdate merges the changed range into the public buffer's full byte payload; explicit graph import uploads that current full payload to the RHI buffer before passes execute. If the graph writes and exports the imported buffer, the public buffer payload is replaced with the full exported buffer bytes and RendererGraphBufferExport reports the represented full-buffer byte range.

This verifies offset update semantics for explicit graph buffer imports. It does not claim minimal-range staging uploads; current public buffer import semantics are full-current-payload upload into the graph.

### Public graph buffer export byte coverage flag (2026-05-19 18:39:59 +08:00)

RendererGraphBufferExport now reports complete_byte_coverage. Current explicit graph buffer exports represent full-buffer byte ranges, so promoted transient buffers and imported public buffer writebacks report byte_offset = 0, byte_len = size, and complete_byte_coverage = true.

The flag makes full-buffer export semantics explicit for graph tooling, debug reports, and capture consumers. It also leaves a stable payload shape for future partial buffer export ranges.

### Public graph export aggregate coverage helpers (2026-05-19 18:42:11 +08:00)

RendererGraphResourceExports now exposes aggregate coverage helpers for explicit public graph exports: texture_exports_with_incomplete_subresource_coverage, buffer_exports_with_incomplete_byte_coverage, all_texture_exports_complete_subresource_coverage, all_buffer_exports_complete_byte_coverage, and has_incomplete_export_coverage. These helpers summarize per-export coverage flags so tools can quickly determine whether an execution contains partial texture subresource coverage or partial buffer byte coverage.

The helpers are derived from the export payload itself and do not change resource identity or execution behavior. They make partial explicit graph export coverage directly queryable without requiring every caller to iterate and reimplement the same checks.

### Public frame output aggregate subresource helpers (2026-05-19 18:45:00 +08:00)

FramePublicFrameOutput now exposes helper methods for its represented payload: subresource_byte_len, has_packed_subresources, and has_complete_view_subresource_layout. FrameStats exposes aggregate helpers across all public outputs in a frame: public_frame_outputs_with_packed_subresources, public_frame_output_subresource_bytes, and all_public_frame_outputs_have_complete_subresource_layouts.

These helpers let tooling distinguish simple single-subresource outputs from packed multi-mip texture-view outputs without reimplementing byte-layout checks. They are derived from FramePublicFrameOutput::subresources and do not change output bytes or rendering behavior.

### Public graph imported D1 texture base-mip execution (2026-05-19 18:46:48 +08:00)

Renderer::execute_graph_to_resources now supports imported public TextureDimension::D1 textures through the explicit graph path by treating them as height-1 RHI-compatible textures. The D1 import uploads base-mip bytes before graph execution, graph passes can read/write the represented line texture through the RHI texture interface, and export writeback updates the original public TextureHandle.

RendererGraphTextureExport preserves the public descriptor dimension as D1 while its represented subresource layout reports the height-1 base-mip byte payload. D2 remains supported through the same path; array, cube, 3D, and multisampled imports remain unsupported until the RHI/backend subresource model is expanded.

### Public graph imported D2Array texture flattened base-mip execution (2026-05-19 18:50:42 +08:00)

Renderer::execute_graph_to_resources now supports imported public TextureDimension::D2Array textures for explicit graph execution by flattening base-mip layers into a height-stacked RHI-compatible 2D texture. The graph sees width x (height * layers) bytes through the current RHI texture interface. Export writeback restores the bytes to the original public TextureHandle payload while RendererGraphTextureExport preserves the descriptor dimension as D2Array and reports layer coverage in the represented subresource layout.

This is a compatibility path for public graph base-mip array payloads. It is not a true layer-aware backend/RHI texture model: graph passes still address the flattened representation, and cube, cube-array, 3D, multisampled, and arbitrary mip/layer subresource execution remain separate requirements.

### Public graph imported Cube/CubeArray flattened base-mip execution (2026-05-19 18:52:35 +08:00)

Renderer::execute_graph_to_resources now supports imported public TextureDimension::Cube and TextureDimension::CubeArray textures for explicit graph execution by flattening base-mip faces/layers into a height-stacked RHI-compatible 2D texture. The graph sees width x (height * face_or_layer_count) bytes through the current RHI texture interface. Export writeback restores the bytes to the original public TextureHandle payload while RendererGraphTextureExport preserves the descriptor dimension as Cube or CubeArray and reports layer coverage in the represented subresource layout.

This is a compatibility path for public graph base-mip cube payloads. It is not a true cube-aware backend/RHI texture model: graph passes still address the flattened representation, and 3D, multisampled, and arbitrary mip/layer subresource execution remain separate requirements.

### Public graph imported D3 flattened base-mip execution (2026-05-19 18:54:23 +08:00)

Renderer::execute_graph_to_resources now supports imported public TextureDimension::D3 textures for explicit graph execution by flattening base-mip depth slices into a height-stacked RHI-compatible 2D texture. The graph sees width x (height * depth) bytes through the current RHI texture interface. Export writeback restores the bytes to the original public TextureHandle payload while RendererGraphTextureExport preserves the descriptor dimension as D3 and reports depth-slice coverage in the represented subresource layout.

This is a compatibility path for public graph base-mip volume payloads. It is not a true volume-aware backend/RHI texture model: graph passes still address the flattened representation, and native sample-level multisampled execution plus arbitrary mip/layer/depth subresource execution remain separate requirements.

### Public graph imported Depth32Float array flattened execution (2026-05-19 18:55:52 +08:00)

Renderer::execute_graph_to_resources now has focused coverage for imported TextureFormat::Depth32Float D2Array textures through the flattened layer-stack explicit graph path. Depth array base-mip values are uploaded as width x (height * layers), graph passes can read/write them through the RHI depth32f texture functions, and export writeback restores full public texture bytes while preserving D2Array descriptor metadata and represented layer coverage.

This verifies that flattened public graph shape support is not limited to color formats. It remains a compatibility path, not native depth-array RHI addressing.

### Public graph imported non-base mip represented execution (2026-05-19 18:59:48 +08:00)

Renderer::execute_graph_to_resources now supports imported public textures whose latest payload is a complete non-base mip update. The explicit graph import creates an RHI-compatible texture sized to the represented mip extent, uploads that mip payload, lets graph passes read/write it, and writes the result back to the same public TextureHandle with the represented layout preserved.

RendererGraphTextureExport now reports the represented mip level in its subresource layout. A descriptor with multiple mip levels and only one represented non-base mip reports incomplete mip/subresource coverage, so tools can distinguish single-mip execution from full texture execution.

### Public graph imported non-base mip subregion execution (2026-05-19 19:01:56 +08:00)

Renderer::execute_graph_to_resources now supports imported public textures whose latest payload is a subregion update inside a non-base mip. The explicit graph import creates an RHI-compatible texture sized to the full represented mip, uploads the updated subregion at its declared x/y offset, leaves untouched texels zero-initialized for that graph execution, and writes export results back as the represented full mip payload.

RendererGraphTextureExport reports the represented non-base mip layout and partial mip/subresource coverage. This extends represented-single-mip execution to partial x/y updates while keeping layer/depth coverage complete for the represented mip.

### Public graph texture import support query (2026-05-19 19:04:18 +08:00)

Renderer now exposes graph_texture_import_support(texture), returning RendererGraphTextureImportSupport for explicit public graph imports. The support payload reports whether the texture can be imported, an unsupported reason when it cannot, descriptor metadata, whether the current execution path is flattened RHI-compatible, and the represented mip/layer count that will be used for import.

This makes MSAA rejection and flattened array/cube/volume compatibility queryable before executing a graph. It also reports non-base mip represented imports so tooling can warn about partial coverage before graph execution.

### Public graph imported layer/depth subregion upload in flattened textures (2026-05-19 19:06:39 +08:00)

Renderer::execute_graph_to_resources now supports public texture updates that target a non-zero array layer or depth slice before explicit graph import. For flattened-compatible textures, the import path computes the corresponding flattened RHI y offset from layer/depth index and uploads only the updated region into the full represented mip. Graph passes then observe untouched layers/slices as zero-initialized and can write back the represented full mip payload.

This extends the flattened public graph compatibility path beyond x/y subregions to layer/depth offsets. It remains a flattened representation, not native layer/depth RHI addressing.

### Public graph buffer import support query (2026-05-19 19:08:30 +08:00)

Renderer now exposes graph_buffer_import_support(buffer), returning RendererGraphBufferImportSupport for explicit public graph buffer imports. The support payload reports supported status, optional unsupported reason, buffer size, usage flags, represented byte range, and whether the represented range covers the full buffer.

Current explicit graph buffer import semantics upload the current full public buffer payload, even when the latest public update touched only a subrange. The support query makes that full-buffer represented range visible before graph execution.

### Public graph aggregate import support query (2026-05-19 19:10:14 +08:00)

Renderer now exposes graph_import_support(graph), returning RendererGraphImportSupport for all explicit public graph imports. The aggregate support payload contains per-texture and per-buffer import support entries plus helper methods for unsupported texture import count, unsupported buffer import count, total unsupported imports, and all-imports-supported status.

This lets tooling preflight an entire RenderGraphBuilder before execution and report mixed supported/unsupported imports such as supported flattened array textures, supported full-buffer imports, and unsupported multisampled textures without executing the graph.

### Public graph import preflight execution gate (2026-05-19 19:11:40 +08:00)

Renderer::execute_graph_to_resources now runs graph_import_support as an execution preflight before creating RHI import resources. Unsupported imports fail with a RenderGraphValidation error that includes the aggregate unsupported import count and the first public support reason; resolved MSAA D2 imports are supported, while native sample-level MSAA graph textures remain outside the current RHI model.

Supported explicit graph imports continue to execute normally. This connects the public capability query to the execution path so tooling and runtime failures report the same import-support semantics.

### Public graph texture import represented layout in support query (2026-05-19 19:13:32 +08:00)

RendererGraphTextureImportSupport now reports the represented import layout that execute_graph_to_resources will use: represented mip, represented width/height, represented layer/depth count, represented byte length, and complete mip/layer/subresource coverage flags. The support query uses the same represented-layout calculation as execution, including non-base mip imports and flattened array/cube/volume compatibility.

This lets tooling inspect not only whether an import is supported, but exactly which mip/range will be imported before graph execution.

### Public graph generated mip-chain import support query coverage (2026-05-19)

RendererGraphTextureImportSupport now explicitly distinguishes a texture descriptor's declared mip chain from the subresource bytes currently represented by explicit public graph import execution. For generated mip-chain textures, graph_texture_import_support reports the descriptor mip count while representing only mip 0 for execution, including base extent, base-mip byte length, complete layer coverage, incomplete mip coverage, and incomplete subresource coverage.

This gives tools a stable preflight signal for generated mip-chain imports and prevents callers from assuming that all generated mips participate in explicit graph IO. True simultaneous multi-mip graph execution and native backend mip/layer addressing remain renderer-layer completion requirements.

### Public graph generated mip-chain writeback regeneration (2026-05-19)

Renderer::execute_graph_to_resources now preserves generated mip-chain semantics when an imported generated public texture is exported after graph execution. The graph imports the represented base mip for RHI execution; after the pass writes the base mip, the renderer regenerates the retained mip chain from the updated base bytes, keeps TextureInfo::mips_generated true, and exposes the complete generated mip-chain byte layout through RendererGraphTextureExport::subresources.

This makes generated mip-chain public graph writeback deterministic and observable while preserving the current limitation that graph passes do not directly read or write multiple mips in one execution resource.

### Public graph layered generated mip-chain writeback coverage (2026-05-19)

Generated mip-chain writeback regeneration is covered for layered D2Array textures as well as single-layer D2 textures. Explicit graph execution imports the flattened base-layer representation, writes updated base bytes, regenerates per-layer mip chains on writeback, and exposes packed mip subresource metadata with complete coverage flags.

This improves public graph compatibility for layered generated textures while preserving the documented limitation that graph passes still operate on the represented base mip rather than directly addressing every mip/layer subresource natively.

### Public graph volume generated mip-chain writeback coverage (2026-05-19)

Generated mip-chain writeback regeneration is covered for D3 volume textures. Explicit graph execution imports the flattened base-volume representation, writes updated base bytes, regenerates the retained volume mip chain on writeback, and reports packed mip subresource metadata with per-mip depth counts.

RendererGraphTextureExport coverage calculation now uses descriptor-aware expected depth/layer counts per mip. This keeps complete D3 generated mip chains from being incorrectly reported as incomplete solely because lower mips have smaller depth than the base mip.

### Public graph cube generated mip-chain writeback coverage (2026-05-19)

Generated mip-chain writeback regeneration is covered for Cube textures. Explicit graph execution imports the flattened base-face representation, writes updated base-face bytes, regenerates the retained cube mip chain on writeback, and exposes packed mip subresource metadata with complete coverage for all six faces.

This improves public graph compatibility for generated cube textures while preserving the current limitation that graph passes still operate on a flattened represented base mip rather than native cube face/mip subresources.

### Public graph CubeArray generated mip-chain writeback coverage (2026-05-19)

Generated mip-chain writeback regeneration is covered for CubeArray textures. Explicit graph execution imports the flattened base-face/layer representation, writes updated base bytes, regenerates the retained cube-array mip chain on writeback, and exposes packed mip subresource metadata with complete coverage for all represented faces/layers.

This improves public graph compatibility for generated cube-array textures while preserving the current limitation that graph passes still operate on a flattened represented base mip rather than native cube-array face/layer/mip subresources.

### Public graph D1 generated mip-chain writeback coverage (2026-05-19)

Generated mip-chain writeback regeneration is covered for D1 textures. Explicit graph execution imports the base line representation, writes updated base bytes, regenerates the retained D1 mip chain on writeback, and exposes packed mip subresource metadata with complete coverage.

Together with the D2, D2Array, D3, Cube, and CubeArray tests, this gives the current explicit public graph generated-mip compatibility path targeted shape coverage across all single-sample texture dimensions supported by public graph imports. The limitation remains that graph passes operate on the represented base mip rather than native multi-mip subresources.

### Public graph generated mip-chain packed import read support (2026-05-19)

Generated mip-chain public texture imports now upload the complete retained mip chain into explicit graph execution using a packed RHI-compatible representation. Each mip is written at x=0 and a vertically stacked y offset derived from the mip-chain subresource layout, so graph passes can read lower generated mips deterministically through the same packed layout exposed by public export metadata.

graph_texture_import_support reports complete coverage and the packed represented RHI height for generated full-chain imports. Non-generated flattened imports continue to report their logical represented height. Writeback still regenerates the retained generated mip chain from the graph-written base mip, preserving the generated-mips invariant rather than treating lower mip graph writes as authored public data.

### Public graph generated lower-mip writeback retention (2026-05-19)

Generated mip-chain explicit graph writeback now reads packed mip-chain exports by subresource layout instead of reading one base-width padded rectangle. This keeps public texture bytes compact and aligned with RendererGraphTextureExport::subresources.

When graph execution only changes the base mip and lower mips remain unchanged, writeback regenerates the retained mip chain from the base and keeps TextureInfo::mips_generated true. When graph execution authors lower mip bytes, writeback preserves the full packed mip-chain payload and marks TextureInfo::mips_generated false, making it explicit that the public texture now contains authored mip bytes rather than a purely generated chain.

Authored packed mip-chain payloads remain importable by explicit graph execution and retain complete subresource coverage metadata. The remaining limitation is that this is still a packed compatibility representation, not native backend mip/layer/depth subresource addressing.

### Public graph texture import support subresource layout metadata (2026-05-19)

RendererGraphTextureImportSupport now exposes the represented import subresources using the same layout fields as RendererGraphTextureExport. Preflight callers can inspect mip level, base layer/depth, represented extent, byte offset, byte length, bytes per row, and rows per image before graph execution.

For generated packed mip-chain imports, this reports each represented mip in the packed public layout instead of requiring callers to infer lower mip offsets from descriptor dimensions and aggregate byte length.

### Public graph import support subresource aggregate helpers (2026-05-19)

RendererGraphTextureImportSupport now provides subresource_byte_len and has_packed_subresources helper methods. RendererGraphImportSupport provides aggregate helpers for texture imports with packed subresources, incomplete texture subresource coverage, total represented subresource bytes, all-texture-complete status, and any incomplete import coverage.

These helpers make graph preflight tooling consume the same represented layout facts used by import execution and export metadata without reimplementing byte summation or coverage checks.

### Public texture info subresource layout metadata (2026-05-19)

TextureInfo now includes retained payload subresource metadata: mip level, base layer/depth, layer/depth count, extent, byte offset, byte length, bytes per row, and rows per image. It also reports complete mip/layer/subresource coverage and helper methods for total represented subresource bytes and packed-subresource detection.

This lets callers inspect whether a public texture currently stores a single represented subresource, a generated/packed mip chain, or an authored packed mip payload without relying only on mip_levels and mips_generated.

### Public buffer info byte coverage metadata (2026-05-19)

BufferInfo now reports retained public buffer byte coverage through byte_offset, byte_len, and complete_byte_coverage. It also exposes represented_byte_len and has_complete_byte_coverage helpers.

This lets callers inspect whether the public buffer payload currently represents the full declared buffer without relying on size alone, and keeps BufferInfo aligned with RendererGraphBufferImportSupport and RendererGraphBufferExport byte-range metadata.

### Public buffer represented byte-range import support (2026-05-19)

Public buffers now track the represented byte range of retained CPU/public payload updates. update_buffer merges each update into a single represented range, BufferInfo and RendererGraphBufferImportSupport expose that range, and explicit graph import uploads only the represented range into the newly created RHI buffer. Bytes outside the represented range remain zero-initialized for that graph execution.

When a graph exports an imported public buffer, writeback reads the full exported buffer and marks the retained public buffer as full-byte coverage. This closes the explicit graph path for single merged represented buffer ranges while leaving multiple disjoint dirty ranges and persistent backend dirty synchronization as future renderer-layer work.

### Public buffer disjoint represented byte-range import support (2026-05-19)

Public buffers now preserve multiple disjoint represented byte ranges. BufferInfo and RendererGraphBufferImportSupport expose byte_ranges for exact range inspection while byte_offset and byte_len continue to describe the bounding span. BufferInfo::represented_byte_len returns the sum of exact represented ranges.

Explicit graph import uploads each represented range separately into the graph RHI buffer, so gaps between updated ranges remain zero-initialized instead of being treated as represented public data. Graph writeback reads the full exported buffer and restores full-byte coverage for the retained public buffer.

### Public graph buffer import represented-range aggregate helpers (2026-05-19)

RendererGraphBufferImportSupport now exposes represented_byte_len and has_disjoint_byte_ranges helper methods. RendererGraphImportSupport provides aggregate buffer helpers for incomplete byte coverage count, disjoint-range import count, total represented bytes, and all-buffer-complete status.

These helpers let graph preflight tooling consume precise retained public-buffer range facts without recomputing disjoint range totals or coverage checks from byte_ranges manually.

### Public graph buffer export byte-range metadata alignment (2026-05-19)

RendererGraphBufferExport now includes byte_ranges alongside byte_offset, byte_len, and complete_byte_coverage. It also exposes represented_byte_len and has_disjoint_byte_ranges helpers.

Current explicit graph buffer exports still represent full buffers, but the public payload shape now matches BufferInfo and RendererGraphBufferImportSupport, giving tooling a consistent byte-range model across retained resources, graph imports, and graph exports.

### Public graph partial buffer export ranges (2026-05-19)

RenderGraphBuilder now supports export_buffer_range(label, buffer, byte_offset, byte_len). The RHI export payload carries the requested byte range, and the renderer facade reads back only that range when promoting transient graph buffers or writing back imported public buffers.

A promoted transient partial export creates a public BufferHandle with the graph buffer size but only the exported byte range represented in retained bytes. An imported public partial export updates only the exported byte range in the existing public buffer. RendererGraphBufferExport and BufferInfo report incomplete byte coverage and the represented byte range consistently.

### Public graph imported buffer export range preflight (2026-05-19)

execute_graph_to_resources now validates imported public buffer export ranges before graph execution. A range requested through export_buffer_range must fit inside the retained public buffer size; otherwise the renderer returns RenderGraphValidation before passes run. The error includes the public export label.

RenderGraph validation also rejects empty explicit buffer export ranges and validates transient graph buffer export ranges against graph buffer descriptors. Imported public buffers require the renderer facade preflight because their sizes are known by the retained resource layer rather than the graph descriptor map.

### Public graph multiple buffer export ranges (2026-05-19)

RenderGraphBuilder now supports export_buffer_ranges(label, buffer, ranges), allowing one exported graph buffer to expose multiple disjoint byte ranges. RhiBufferExport carries byte_ranges, and the renderer facade reads each range separately when writing back imported public buffers or promoting transient graph buffers.

RendererGraphBufferExport and BufferInfo preserve both the bounding byte_offset/byte_len and the exact byte_ranges. export_buffer remains the full-buffer API and export_buffer_range remains the single-range convenience API.

### Public graph buffer export range validation errors (2026-05-19)

export_buffer_ranges now treats an empty range list as invalid instead of a full-buffer export. Graph validation rejects empty explicit buffer export ranges and out-of-bounds transient graph buffer ranges. The renderer facade preflight rejects out-of-bounds imported public buffer ranges before graph execution. Error messages include the export label.

### Public graph buffer export range aggregate helpers (2026-05-19)

RendererGraphResourceExports now includes buffer_exports_with_disjoint_byte_ranges and buffer_export_represented_bytes. These aggregate helpers let tooling summarize partial and multi-range buffer exports without iterating every RendererGraphBufferExport manually.

### Public graph buffer export range normalization (2026-05-19)

export_buffer_ranges canonicalizes byte ranges by sorting them and merging overlapping or adjacent ranges. RendererGraphBufferExport, BufferInfo, and aggregate represented-byte helpers report the normalized ranges, avoiding duplicate readback and duplicate represented-byte accounting.
## RenderGraph texture region export note (2026-05-19)

The public graph-to-resource path supports D1/D2 base-mip rectangular texture exports, D2 non-base mip rectangular texture exports, D2Array single-layer non-base mip region exports, Cube/CubeArray single-face non-base mip region exports, D3 non-base mip aligned depth-slice texture exports, D1/D2 generated base-mip rectangular texture exports, and aligned whole-layer D2Array/D3/Cube/CubeArray base-mip texture exports:

```rust
graph.export_texture_region("region_output", graph_texture, x, y, width, height);
```

Public preflight:

```rust
let support = renderer.graph_texture_region_export_support(texture, region)?;
let graph_support = renderer.graph_region_export_support(&graph)?;
let import_support = renderer.graph_import_support(&graph)?;
```

Runtime behavior:

- Transient graph texture exports promote the requested D2 rectangle into a public texture handle while preserving the original texture descriptor extent.
- `Renderer::graph_texture_region_export_support` reports whether a public texture region export is supported before graph execution and exposes the exact `RendererGraphTextureExportSubresource` metadata, subresource byte length, packed-subresource status, coverage flags, and unsupported reason that tooling can surface to users.
- `Renderer::graph_region_export_support` batch-preflights imported public texture region exports declared by a graph, preserving deterministic export labels and aggregating supported/unsupported counts, boolean unsupported gates, supported/unsupported label lists, unsupported reasons, reason count/bool helpers, label+reason summaries, label coverage, and subresource byte totals for tooling.
- `Renderer::graph_import_support` includes the same imported texture region export preflight results, aggregate helpers, boolean gate helpers, reason count/bool helpers, and label+reason summaries, so tools that already query graph import support can also display explicit region export gates without a separate pass.
- `RendererGraphTextureExport.region` is `Some(RhiTextureExportRegion)` for region exports and `None` for full texture exports.
- `RendererGraphTextureExport::subresource_byte_len()` and `has_packed_subresources()` expose compact multi-subresource export payloads directly, while `RendererGraphResourceExports` exposes aggregate packed-subresource export counts and exported texture subresource bytes.
- `RendererGraphResourceExports::texture_exports()`, `buffer_exports()`, `promoted_texture_exports()`, `promoted_buffer_exports()`, `promoted_texture_export_labels()`, `promoted_buffer_export_labels()`, `imported_texture_exports()`, `imported_buffer_exports()`, `imported_texture_export_labels()`, `imported_buffer_export_labels()`, `export_count()`, `promoted_export_count()`, `imported_export_count()`, `export_label_count()`, `promoted_export_label_count()`, `imported_export_label_count()`, `texture_region_export_label_count()`, `has_complete_texture_region_export_label_coverage()`, `has_complete_export_label_coverage()`, `texture_exports_with_regions()`, and `has_texture_region_exports()` summarize graph export usage for the execution result.
- `RenderGraphStats.exported_texture_regions`, `exported_texture_region_labels`, `has_texture_region_exports()`, label-count/coverage helpers, and sorted label helpers expose region-export counts and labels in graph/frame/capture/debug stats.
- `RenderGraphStats.backend_exported_texture_regions`, `backend_exported_texture_region_labels`, `has_backend_texture_region_exports()`, backend label-count/coverage helpers, and backend sorted label helpers expose backend-origin texture region export provenance after facade/backend graph-stat merge.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` directly expose public graph exported texture/buffer counts and labels for explicit graph export tooling.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` directly expose promoted public graph texture/buffer counts and labels for explicit graph export tooling.
- `FrameStats`, `FrameProfile`, `FrameDebugReport`, and `FrameCapture` directly expose imported public graph texture/buffer counts and labels for explicit graph export tooling.
- `FrameStats::public_graph_export_count()`, `public_graph_promoted_export_count()`, `public_graph_imported_export_count()`, `public_graph_export_label_count()`, `public_graph_promoted_export_label_count()`, `public_graph_imported_export_label_count()`, `public_graph_texture_region_export_label_count()`, `has_complete_public_graph_texture_region_export_label_coverage()`, and `has_complete_public_graph_export_label_coverage()` summarize immediate-frame public graph export totals and label coverage for tooling.
- `FrameProfile`, `FrameDebugReport`, and `FrameCapture` expose the same public graph export aggregate helper set as `FrameStats`, so profiling, editor/debug, and capture payloads can summarize export totals and label coverage without parsing nested graph execution data.
- `FrameStats.public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` directly expose the latest explicit public graph texture region export count and labels for immediate frame output.
- `FrameProfile.public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` directly expose the latest explicit public graph texture region export count and labels for profiling payloads.
- `FrameCaptureResourceDump.public_graph_exported_texture_labels`, `public_graph_exported_buffer_labels`, `public_graph_promoted_texture_labels`, `public_graph_promoted_buffer_labels`, `public_graph_imported_texture_export_labels`, `public_graph_imported_buffer_export_labels`, `public_graph_texture_region_exports`, and `public_graph_texture_region_export_labels` expose explicit public graph export counts, labels, and promoted/imported classification in capture resource dumps.
- `FrameCaptureResourceDump::public_graph_export_count()`, `public_graph_promoted_export_count()`, `public_graph_imported_export_count()`, `public_graph_export_label_count()`, `public_graph_promoted_export_label_count()`, `public_graph_imported_export_label_count()`, `public_graph_texture_region_export_label_count()`, `has_complete_public_graph_texture_region_export_label_coverage()`, and `has_complete_public_graph_export_label_coverage()` summarize export totals and label coverage for capture tooling.
- `FrameDebugReport.public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` directly expose the latest explicit public graph texture region export count and labels for editor/debug tooling.
- `FrameCapture.public_graph_texture_region_exports` and `public_graph_texture_region_export_labels` directly expose the latest explicit public graph texture region export count and labels for capture artifacts.
- Imported public D2 texture exports write back only the requested rectangle to the public resource payload.
- Imported public D2 non-base mip texture exports use represented graph/RHI mip coordinates and report the public `mip_level`, `offset`, and extent in subresource metadata.
- Imported public D2Array single-layer and aligned multi-layer non-base mip texture exports use flattened layer coordinates and report the public `mip_level`, `base_layer`, `layer_count`, `offset`, and extent in subresource metadata.
- Imported public Cube/CubeArray single-face and aligned multi-layer non-base mip texture exports use flattened face coordinates and report the public `mip_level`, `base_layer`, `layer_count`, `offset`, and extent in subresource metadata.
- Imported public D3 non-base mip texture exports use represented flattened depth-slice coordinates and report the public `mip_level`, `base_layer`, `layer_count`, `offset`, and extent in subresource metadata.
- Imported public D1/D2 generated mip-chain texture exports accept regions inside a single generated packed mip; writeback maps packed RHI y coordinates back to public `mip_level` and mip-local `offset` metadata, then clears generated-mip status because the public payload now represents a partial subresource region.
- Imported public D1 texture exports write back `y=0,height=1` ranges to the public resource payload.
- Imported public D2Array/D3/Cube/CubeArray texture exports accept flattened regions whose `y` and `height` align to full layer/depth-slice height; metadata reports the corresponding public `base_layer`, `layer_count`, `offset`, and extent.
- Imported public D2Array/D3/Cube/CubeArray texture exports also accept partial flattened regions across one or more layers/depth-slices; metadata maps flattened `y` to public mip-local `offset.y` and splits cross-layer partial exports into multiple public subresources when needed.
- Cross-layer partial flattened exports retain multi-subresource public texture layout metadata, so later public graph imports can upload the compact bytes back to the correct flattened RHI coordinates.
- Out-of-bounds flattened regions are rejected before graph execution and can be detected through `Renderer::graph_texture_region_export_support`.
- `RendererGraphTextureExport.subresources` and `TextureInfo.subresources` describe the represented rectangle with both `offset` and extent; `complete_subresource_coverage` is false when the exported rectangle is smaller than the full mip extent.
- `RendererGraphTextureImportSupport.subresources` uses the same offset-aware subresource metadata for public import observability, while `represented_width`, `represented_height`, and `represented_layer_count` describe the RHI-compatible upload shape.
- Invalid imported public regions are rejected before graph pass execution.

Current scope:

- Implemented for D1/D2 base-mip, D2 non-base mip, D2Array single-layer and aligned multi-layer non-base mip, Cube/CubeArray single-face and aligned multi-layer non-base mip, D3 non-base mip aligned depth-slice regions, D1/D2 generated base/lower-mip, aligned whole-layer D2Array/D3/Cube/CubeArray base-mip, and single-layer/cross-layer partial-layer flattened region exports on the headless/RHI graph-to-resource path.
- Backend-wgpu native region export execution/readback behavior remains a separate renderer-layer completion item.

### Backend-wgpu surface readback public frame outputs

`graphics_wgpu::WgpuSurface` now configures surface textures with `COPY_SRC` when the platform surface reports that usage as supported. When enabled, a completed backend-wgpu surface frame is copied into a CPU readback buffer, recorded as a pending map operation, and exposed as `WgpuFrameReadback` only after the pending readback is resolved.

Readback is opt-in rather than automatic. Callers can inspect `Renderer::surface_frame_readback_supported()`, inspect current state through `Renderer::surface_frame_readback_enabled()`, enable it with `Renderer::set_surface_frame_readback_enabled(true)`, or request a scoped readback through `Renderer::request_surface_frame_readback_next_frame()`. The next-frame request temporarily enables surface readback for the next successfully finished frame and restores the previous state after `Frame::finish()`; `Renderer::cancel_surface_frame_readback_next_frame()` cancels a queued one-shot request before the frame begins. When enabled, `WgpuSurface` records a pending readback after surface submission without immediately waiting inside `render_frame()`. `Renderer::public_frame_output_for_view` uses nonblocking try-resolve when it needs to materialize `RenderTarget::MainSurface` or `RenderTarget::Surface(_)` backend-owned outputs into durable public texture handles; if the map callback is not ready, the frame reports `BackendSurfaceReadbackUnavailable` instead of stalling.

For tooling that wants to consume a completed readback later, `Renderer::surface_frame_readback_pending()` and `Renderer::surface_frame_readback_available()` expose current state, `Renderer::poll_surface_frame_readback()` nonblockingly advances pending completion, and `Renderer::materialize_surface_frame_readback(label)` creates a durable public texture from the latest completed backend surface readback.

`FramePublicOutputSource::BackendMainSurfaceReadback` and `FramePublicOutputSource::BackendSurfaceReadback` identify these outputs. The generated `FramePublicFrameOutput` preserves the readback width, height, texture format, represented base subresource, and public texture bytes for stats, debug report, capture, and downstream public API use.

When a backend-owned surface frame also has a standard-frame graph export for `main_color`, the renderer remaps that promoted graph texture export to the durable backend surface readback texture once the readback-backed `FramePublicFrameOutput` is materialized. The graph export source becomes `RendererGraphExportSource::BackendMainSurfaceReadback` or `RendererGraphExportSource::BackendSurfaceReadback`, so tooling can distinguish real surface-backed graph promotion from ordinary offscreen transient promotion.

If the backend surface public output cannot be materialized, exported standard-frame `main_color` graph resources are marked unpromoted with `RendererGraphExportSource::BackendSurfaceReadbackUnsupported`, `BackendSurfaceReadbackDisabled`, or `BackendSurfaceReadbackUnavailable`. Imported graph export counts are based on `ImportedPublic` provenance, so unsupported surface exports remain visible as exports without being misclassified as imported public writebacks.

`Renderer::surface_graph_export_support()` reports direct swapchain image graph export support separately from readback-backed surface graph export support/enabled state. The current renderer reports direct swapchain graph export as unsupported and directs tooling to use readback-backed materialization when available or consume explicit unsupported provenance.

If backend surface output cannot be materialized, `FrameStats.unsupported_public_frame_outputs`, `FrameDebugReport.unsupported_public_frame_outputs`, and `FrameCapture.unsupported_public_frame_outputs` report the affected view label, intended backend surface output source, and a `BackendSurfaceReadbackUnsupported`, `BackendSurfaceReadbackDisabled`, or `BackendSurfaceReadbackUnavailable` reason.

The windowed facade usecase exposes this path with `--surface-readback` and `--require-surface-readback`. The required mode now accepts either an explicitly materialized texture from `Renderer::materialize_surface_frame_readback()` or a backend surface readback `FramePublicFrameOutput` in `FrameStats.public_frame_outputs` as durable readback evidence. The local validation command `.\target\debug\render_facade_window_usecase.exe --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback` passed with `public_outputs=1`, `unsupported_public_outputs=0`, and `surface_readback_frame_outputs=1`.

Surface readback smoke also requires the native post-process path to remain render-pass compatible with reflected material post-pass submissions. `render_wgpu::MeshRenderer` now creates the post-process render pass with the surface depth attachment when available and creates both the color and sampled post-process pipelines with matching depth-stencil state, using non-writing `Always` depth compare for fullscreen post-process work.

Current limitation: render submission no longer blocks unconditionally on readback, frame public-output materialization no longer waits for an incomplete map callback, and public ready/poll/materialize APIs exist for completed readbacks outside the originating frame. Platforms without surface `COPY_SRC` support still cannot expose a durable backend surface output, though the unsupported reason is now visible. Backend-wgpu RenderGraph surface exports, durable graph promotion, stronger real-device integration validation, and native texture-region graph export/readback remain separate renderer-layer completion items.

### Backend-wgpu texture-region graph export/readback proof

The explicit RenderGraph/RHI path now has a backend-wgpu texture-region export/readback proof in addition to the broader headless retained-resource coverage. `graph_execute_on_wgpu_exports_texture_region_with_readback` creates a transient 4x4 RGBA8 graph texture, writes deterministic bytes through a graph callback using `WgpuRhiDevice`, exports a 2x2 region with `export_texture_region`, verifies the exported `RhiTextureExportRegion`, and reads the same 2x2 region back with `WgpuRhiDevice::read_texture_rgba8`.

`graph_execute_on_wgpu_exports_float_and_depth_regions_with_readback` extends the backend-wgpu proof to transient RGBA16F, RGBA32F, and Depth32Float D2 texture regions. Backend-wgpu depth writes no longer use the forbidden `Queue::write_texture` depth-copy path; `WgpuRhiDevice::write_texture_depth32f` writes depth values through a depth-only render pass whose fragment stage returns `frag_depth` values sourced from a storage buffer. Exported Depth32Float regions carry correct region metadata and can be read back with the written values.

`wgpu_rhi_write_texture_depth32f_writes_readable_region` covers the direct backend-wgpu RHI depth write/readback path outside RenderGraph.

Validated with:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_texture_region_with_readback -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer graph_execute_on_wgpu_exports_float_and_depth_regions_with_readback -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer wgpu_rhi_write_texture_depth32f_writes_readable_region -- --nocapture`

Current limitation: this is backend-wgpu proof for transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float graph texture-region export/readback plus true direct Depth32Float write/readback execution for the current D2 RHI path. It does not complete standard-frame/surface graph export promotion, native multi-mip/layer/depth region addressing, or broader platform surface coverage.

### Backend-wgpu imported public texture writeback proof

`Renderer::execute_graph_to_resources` now has direct backend-wgpu proof for imported public color texture upload and exported-import writeback. `execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_texture_shapes` creates a real `Renderer::new` with `BackendPreference::Wgpu`, imports public textures into a `RenderGraphBuilder`, verifies the backend-wgpu RHI sees uploaded initial bytes before graph mutation, writes updated bytes inside the graph callback, exports the imported graph texture, and verifies the original public `TextureHandle` contains the updated bytes and imported-public export metadata.

The proof covers D1, D2, D2Array, D3, Cube, and CubeArray RGBA8 through the current flattened-compatible single-mip representation, plus D2 RGBA16F, D2 RGBA32F, D2 Depth32Float, and flattened D2Array Depth32Float upload/read/writeback on backend-wgpu. `execute_graph_to_resources_wgpu_writes_back_imported_rgba8_texture_export_region`, `execute_graph_to_resources_wgpu_writes_back_imported_float_texture_export_regions`, and `execute_graph_to_resources_wgpu_writes_back_imported_depth_texture_export_region` also cover imported public D2 RGBA8, D2 RGBA16F/RGBA32F, and D2 Depth32Float `export_texture_region` writeback, including partial public bytes/layout and incomplete subresource coverage metadata. `execute_graph_to_resources_wgpu_writes_back_imported_layered_rgba8_texture_export_regions` covers D2Array, D3, Cube, and CubeArray RGBA8 whole-layer/face flattened region writeback on backend-wgpu. `execute_graph_to_resources_wgpu_writes_back_imported_cross_layer_rgba8_texture_export_regions` covers D2Array, D3, Cube, and CubeArray RGBA8 cross-layer/cross-face flattened partial region writeback with multi-subresource public layout metadata. `execute_graph_to_resources_wgpu_writes_back_imported_non_base_mip_rgba8_texture_export_region` covers represented non-base mip D2 RGBA8 region writeback with mip-level metadata. `execute_graph_to_resources_wgpu_regenerates_generated_mip_imports` and `execute_graph_to_resources_wgpu_regenerates_generated_mip_import_shapes` cover generated D1/D2/D2Array/D3/Cube/CubeArray RGBA8 mip-chain upload/read/writeback/regeneration on backend-wgpu using the current packed import representation.

Validated with:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_texture_shapes -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_rgba8_texture_export_region -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_float_texture_export_regions -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_depth_texture_export_region -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_layered_rgba8_texture_export_regions -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_cross_layer_rgba8_texture_export_regions -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_non_base_mip_rgba8_texture_export_region -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_regenerates_generated_mip_imports -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_regenerates_generated_mip_import_shapes -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture`

Current limitation: this closes backend-wgpu imported public texture writeback for the current explicit graph compatibility model across the covered single-mip and packed/generated-mip color shapes plus D2 float/depth formats and flattened D2Array depth, including D2 RGBA8/RGBA16F/RGBA32F/Depth32Float partial region writeback, D2Array/D3/Cube/CubeArray RGBA8 whole-layer/face plus cross-layer/cross-face flattened region writeback, represented non-base mip D2 RGBA8 region writeback, and generated D1/D2/D2Array/D3/Cube/CubeArray RGBA8 mip-chain regeneration. It does not complete standard-frame/surface graph export promotion, native simultaneous multi-mip/layer/depth addressing, native MSAA texture/sample-level graph execution, persistent backend-resident dirty synchronization, or broader platform surface coverage.

### Backend-wgpu transient public graph export promotion proof

`Renderer::execute_graph_to_resources` now has direct backend-wgpu proof for transient graph resource promotion. `execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles` creates a real `Renderer::new` with `BackendPreference::Wgpu`, writes transient graph D2 RGBA8/RGBA16F/RGBA32F/Depth32Float textures plus a transient graph buffer through the backend-wgpu RHI path, exports the resources, and verifies the execution result exposes durable public `TextureHandle` / `BufferHandle` resources with matching public bytes. `execute_graph_to_resources_wgpu_promotes_partial_transient_texture_and_buffer_exports` extends this to transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region promotion and transient buffer disjoint-range promotion, including incomplete coverage metadata and durable public bytes.

The proof covers promoted/export source metadata, descriptor metadata, complete represented texture subresource coverage for each covered format, complete buffer byte coverage, and label lookup through `RendererGraphResourceExports`.

Validated with:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_exported_transients_to_public_handles -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_promotes_partial_transient_texture_and_buffer_exports -- --nocapture`

Current limitation: this closes explicit backend-wgpu graph-to-public promotion for transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture and buffer exports, plus partial transient D2 RGBA8/RGBA16F/RGBA32F/Depth32Float texture-region and disjoint buffer-range exports. It does not complete standard-frame/surface graph export promotion, native multi-shape/multi-mip transient texture promotion, persistent backend-resident graph resources, or broader platform surface coverage.

### Backend-wgpu imported public buffer writeback proof

`Renderer::execute_graph_to_resources` now has direct backend-wgpu proof for imported public buffer upload and exported-import writeback. `execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export` creates a real `Renderer::new` with `BackendPreference::Wgpu`, imports a public buffer into a `RenderGraphBuilder`, verifies the backend-wgpu RHI sees the uploaded bytes before graph mutation, writes updated bytes inside the graph callback, exports the imported graph buffer, and verifies the original public `BufferHandle` contains the updated bytes.

The proof includes `ImportedPublic` provenance, full-buffer byte range metadata, complete byte coverage, and label lookup through `RendererGraphResourceExports`. `execute_graph_to_resources_wgpu_writes_back_imported_buffer_export_ranges` extends this to partial/disjoint imported buffer export ranges, incomplete byte coverage metadata, retained public buffer byte-range layout, and public bytes updated only for the exported ranges.

`WgpuRhiDevice` preserves public/RHI logical buffer size while allocating backend buffers at a 4-byte-aligned physical size. `write_buffer` and `read_buffer` handle renderer-valid byte ranges that are not naturally aligned to `wgpu::COPY_BUFFER_ALIGNMENT` by using aligned physical backend ranges and slicing the caller-visible bytes. This avoids backend-wgpu validation panics when public graph imports or exports use small byte ranges, including partial ranges at the end of non-4-byte-sized buffers.

Validated with:

- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_uploads_and_writes_back_imported_buffer_export -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_writes_back_imported_buffer_export_ranges -- --nocapture`
- `C:\Users\JM\.cargo\bin\cargo.exe test -p engine_renderer execute_graph_to_resources_wgpu_ -- --nocapture`

Current limitation: this closes explicit backend-wgpu imported public buffer writeback for full-buffer exports and partial/disjoint byte-range exports. It does not complete persistent backend-resident dirty-range synchronization, standard-frame/surface graph export promotion, or broader platform surface coverage.

### Standard-frame graph extension export promotion update

2026-05-20: Public `RenderGraphExtension` exports declared during standard frame rendering now use the RHI export path when a frame/view graph contains exported textures or buffers. Exported transient graph resources are promoted through the same durable public resource path used by explicit `Renderer::execute_graph_to_resources`, and the promoted execution is recorded in `Renderer::last_graph_execution` for `FrameStats`, `FrameDebugReport`, `FrameCapture`, and resource dumps. When backend-wgpu is active, this standard-frame promotion path uses the backend-wgpu RHI device; otherwise it uses the headless RHI device.

Status: Standard-frame graph extension export promotion is implemented for headless and backend-wgpu headless frame targets. Native surface/swapchain graph export promotion, native multi-mip/layer/depth graph resource addressing, native MSAA texture/sample-level graph execution, persistent backend-resident graph resources, and broader surface integration remain incomplete.

Additional note: same-frame graph export promotions are aggregated by renderer frame index, so multiple export-producing views in one frame contribute to the public graph execution instead of overwriting earlier promoted frame exports.

### Resolved MSAA public graph import compatibility

2026-05-20: Public multisampled D2 textures with a single mip/layer can now be imported into explicit public RenderGraph execution as resolved single-sample payloads. This matches the existing public texture byte model: `TextureDesc::initial_data`, `Renderer::texture_bytes`, and `TextureInfo` expose one resolved byte payload while preserving the public descriptor's `samples` metadata. If the graph exports the imported texture, writeback updates the same public `TextureHandle` and keeps `TextureInfo.samples` / `RendererGraphTextureExport.samples` at the original multisample count. Resolved MSAA D2 texture-region exports are also supported and report partial subresource coverage when only a region is exported.

Status: resolved MSAA public graph import/writeback is implemented for headless and backend-wgpu explicit graph execution. Native sample-level MSAA graph textures, native RHI sample-count descriptors, programmable resolves, and per-sample graph access remain incomplete.




Resolved MSAA observability update: `RendererGraphTextureImportSupport`, `RendererGraphTextureRegionExportSupport`, and `RendererGraphTextureExport` expose `resolved_msaa_compatible`. Aggregate helpers on `RendererGraphImportSupport` and `RendererGraphResourceExports` count resolved-MSAA imports/exports, so tooling can report the compatibility path separately from native sample-level MSAA graph support.

Surface-adjacent graph export promotion update: the standard frame export promotion path is covered for `RenderTarget::MainSurface` in the headless/stub renderer path, where graph extension exports are promoted to durable public handles and reported through frame public graph stats. Native swapchain/surface image export promotion remains incomplete.

Resolved MSAA frame tooling update: `FrameStats`, `FrameProfile`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` expose `public_graph_resolved_msaa_texture_exports` plus matching labels for the latest public graph execution, making the resolved-MSAA compatibility path visible outside the immediate graph execution return value.

Resolved MSAA helper parity update: `FrameDebugReport` and `FrameCapture` now expose `public_graph_resolved_msaa_texture_export_label_count()` alongside the existing resolved-MSAA count and labels, matching the helper shape on stats/profile/resource-dump surfaces.

Resolved MSAA region-support helper update: `RendererGraphRegionExportSupport::resolved_msaa_texture_region_exports()` reports resolved-MSAA texture-region export compatibility for direct region-export preflight, matching the aggregate `RendererGraphImportSupport` helper.

### Window usecase graph export option

2026-05-20: `render_facade_window_usecase` supports `--graph-export` and `--require-graph-export`. The option registers a public `RenderGraphExtension` while rendering `RenderTarget::MainSurface` through `Renderer::with_surface`, exports a transient texture and buffer, prints public graph export/promoted counts and labels through `--print-stats`, and can fail a smoke run if no promoted public graph export is observed. This makes standard frame graph export promotion visible in the windowed facade example. Native swapchain image export/promotion remains incomplete.

Window graph export smoke validation: `render_facade_window_usecase --smoke-frames 8 --wait-for-gpu --print-stats --graph-export --require-graph-export` passed on the local window path and reported one promoted texture export plus one promoted buffer export from the MainSurface frame graph extension.

Combined window smoke validation: `render_facade_window_usecase --smoke-frames 8 --wait-for-gpu --print-stats --surface-readback --require-surface-readback --graph-export --require-graph-export` passed locally, reporting one backend surface public frame output and two promoted public graph exports in the same MainSurface run.

MainColor window graph export validation: `render_facade_window_usecase --graph-export` now exports the standard frame `main_color` graph resource as `facade_window_main_color_output`. A local combined smoke run with surface readback and graph export requirements passed, reporting three promoted graph exports including the main-color output plus one backend surface public frame output. This validates standard-frame main-color graph promotion in the window usecase; direct native swapchain image export remains incomplete.

MainColor graph export regression coverage: `render_graph_extension_exports_main_color_to_public_handle` verifies that `RenderGraphExtensionContext::main_color()` can be exported by a public graph extension and promoted to a durable public texture handle on the standard frame path.

MainDepth graph export validation: public graph extensions can now export both `RenderGraphExtensionContext::main_color()` and `main_depth()` through the standard frame path. Focused tests verify durable public color and Depth32Float texture promotion, and the window MainSurface smoke run reports promoted main-color and main-depth graph outputs alongside the backend surface public frame output. Direct native swapchain image graph export remains incomplete.

Strict graph-export smoke gate update: `render_facade_window_usecase --require-graph-export` now requires the exact promoted main-color, main-depth, extension texture, and extension buffer graph outputs. The combined local MainSurface smoke with surface readback and graph export requirements passed under this stricter gate.

### Safe graph texture descriptor creation gate

2026-05-20: `RenderGraphBuilder::try_create_texture_from_desc(label, TextureDesc)` validates public renderer texture descriptors before creating graph transients. It accepts the currently implemented native graph-created texture shape, single-layer/single-mip/single-sample D2, and returns `RendererError::RenderGraphValidation` for array/layered, mipped, or multisampled descriptors. This provides a non-lossy entry point for tools and new code while keeping the existing `create_texture_from_desc` compatibility helper. Native graph-created multi-mip/layer/depth/MSAA texture execution remains incomplete.

Graph descriptor API safety update: `RenderGraphBuilder::create_texture_from_desc` is documented and deprecated as a legacy descriptor projection helper. New code should call `try_create_texture_from_desc`, which validates graph-created texture shape support and returns explicit `RenderGraphValidation` errors for unsupported descriptors.

## 2026-05-20 API note: `GraphTextureDescSupport`

The renderer graph now exposes a preflight API for renderer texture descriptors:

- `RenderGraphBuilder::texture_desc_support(&TextureDesc) -> GraphTextureDescSupport`
- `RenderGraphBuilder::try_create_texture_from_desc(label, TextureDesc) -> Result<GraphTexture, RendererError>`

`GraphTextureDescSupport` reports whether the descriptor is accepted by the current native graph resource model and includes diagnostic fields for dimension, width, height, depth/layers, mip levels, sample count, and format. The current native graph texture shape remains intentionally strict: D1, D2, flattened D2Array, D3, Cube, or CubeArray, one mip level, and one sample. Array/depth, mipmapped, and multisampled native graph textures are still full-renderer follow-up work.


## 2026-05-20 API note: graph-created D1 descriptors

`RenderGraphBuilder::try_create_texture_from_desc` now preserves renderer descriptor metadata for supported graph-created transient textures. The supported native graph-created descriptor set is currently D1, single-layer D2, flattened D2Array, D3, Cube, and CubeArray, both with one mip level and one sample. When a D1 graph-created transient is exported through `Renderer::execute_graph_to_resources`, the promoted public texture reports `TextureDimension::D1` and keeps the D1 subresource metadata.

Mip-chain and MSAA graph-created transients remain unsupported until the native graph resource model grows those shapes end to end.


## 2026-05-20 API note: graph-created D2Array descriptors

`RenderGraphBuilder::try_create_texture_from_desc` now accepts D2Array descriptors with one mip level and one sample. The current native graph execution represents those resources as flattened RHI 2D textures, but exported transients are promoted back to public `TextureDimension::D2Array` textures with the original width, height, layer count, and complete subresource metadata.

Mip-chain and MSAA graph-created transients remain unsupported until the graph resource model supports those shapes end to end.


## 2026-05-20 API note: graph-created D3/Cube/CubeArray descriptors

`RenderGraphBuilder::try_create_texture_from_desc` now accepts D3, Cube, and CubeArray descriptors with one mip level and one sample. The RHI execution path represents these as flattened 2D textures internally, while public transient export promotion restores the original `TextureDimension`, extent, depth/layer count, and complete subresource metadata.

graph-created MSAA transients remain unsupported until native graph resource execution can represent those shapes end to end.


## 2026-05-20 API note: graph-created packed mip-chain descriptors

`RenderGraphBuilder::try_create_texture_from_desc` now accepts supported one-sample graph-created descriptors with multiple mip levels. The graph RHI backing stores these as packed mip-chain rows, and full transient export promotion reads each mip subresource back through `read_rhi_packed_mip_chain_bytes` so the resulting public texture keeps packed bytes and complete subresource metadata.

Headless/RHI coverage exists for D1, D2, D2Array, D3, Cube, and CubeArray mip chains. Backend-wgpu coverage exists for D2 packed mip-chain transients. Graph-created D2 MSAA transient creation and backend-wgpu resolve promotion are supported; custom MSAA resolve validation evidence remains open.


## 2026-05-20 API note: graph-created MSAA descriptors

`RhiTextureDesc` now includes `samples`, and backend-wgpu RHI texture creation uses that sample count. `RenderGraphBuilder::try_create_texture_from_desc` accepts D2, one-mip graph-created MSAA descriptors. Exported RGBA8 MSAA graph transients are resolved internally before readback and promotion, while the promoted public texture keeps the original sample count in metadata.

This is native MSAA texture creation plus resolve promotion, not programmable per-sample graph execution. Custom resolve paths are documented below through the RHI support matrix.

## 2026-05-20 API note: RHI texture sample query

`RhiDevice` now exposes `texture_samples(RhiTexture)`, returning the texture sample count recorded by headless RHI or used by backend-wgpu native texture creation. This makes MSAA state observable at the RHI layer and aligns the RHI API with `RhiTextureDesc.samples`.

`RhiGraphicsPipelineDesc` also carries `sample_count`. Backend-wgpu maps it to `wgpu::MultisampleState::count`, and RHI render-pass validation checks that color/depth target sample counts match the selected graphics pipeline. This lets programmable RHI graph passes render into graph-created MSAA targets with a matching native pipeline, and the explicit custom resolve APIs below expose user-supplied shader execution for supported resolve paths.

`RhiDevice::resolve_texture_rgba8(source, target)` exposes an explicit MSAA resolve operation for RGBA8 textures. The source must be a multisampled render attachment, and the target must be a same-sized single-sample render attachment. Backend-wgpu implements this with a native render-pass resolve target; headless RHI uses deterministic resolved payload copy. `PassContext::resolve_rhi_texture_rgba8()` forwards graph texture handles to the same RHI operation, so graph passes can request an explicit resolve instead of relying only on readback-time implicit resolve.

`RhiResolveMode` adds mode-selectable custom resolve behavior. `Average` maps to the native render-pass resolve path, while `FirstSample` maps to sample index 0 and `Sample(u32)` selects an explicit source sample. Backend-wgpu implements indexed-sample resolve with a compute shader that loads the requested sample from `texture_multisampled_2d<f32>` and stores it into a single-sample RGBA8 storage texture. `PassContext::resolve_rhi_texture_rgba8_with_mode()` exposes the same behavior inside graph callbacks. This is the built-in indexed-sample programmable resolve mode; user-supplied shader-kernel APIs are documented below.

`RhiResolveShaderDesc` and `RhiDevice::resolve_texture_rgba8_with_shader()` expose a backend-wgpu custom WGSL resolve path. The shader ABI reserves group 0 binding 0 for the multisampled RGBA8 source texture and group 0 binding 1 for the single-sample RGBA8 storage target. The renderer owns bind group and compute pipeline creation, then dispatches enough workgroups to cover the target extent. Headless RHI reports this path as unsupported rather than emulating WGSL execution.

`RhiDevice::resolve_texture_rgba16f_with_shader()` and `PassContext::resolve_rhi_texture_rgba16f_with_shader()` extend the same custom WGSL resolve ABI to RGBA16F/HDR textures. The source binding remains a multisampled float texture, while the target storage texture uses `rgba16float`.

`RhiDevice::resolve_texture_rgba32f_with_shader()` and `PassContext::resolve_rhi_texture_rgba32f_with_shader()` extend the custom WGSL resolve ABI to RGBA32F textures, using a multisampled float source binding and `rgba32float` storage output.

## 2026-05-20 API note: persistent graph texture import cache

Backend-wgpu graph execution now reuses a persistent RHI device state. Public texture imports are cached by `TextureHandle` and synchronized by public texture revision. `Renderer::graph_rhi_texture_import_cache_entries()` reports the number of cached public texture imports, and `Renderer::clear_graph_rhi_texture_import_cache()` drops the cache explicitly.

This currently covers texture imports. Buffer import caching requires public buffer revisions and remains a separate renderer-layer follow-up.

## 2026-05-20 API note: persistent graph buffer import cache

Backend-wgpu graph execution now caches public buffer imports as well as texture imports. Public buffers carry revisions, so compatible backend RHI buffers can be reused across graph executions and synchronized only when represented byte ranges change. `Renderer::graph_rhi_buffer_import_cache_entries()` reports cached buffer imports, and `Renderer::clear_graph_rhi_buffer_import_cache()` clears them explicitly.

Destroying public texture or buffer handles also evicts the matching persistent graph RHI import cache entry. This keeps backend-wgpu graph imports tied to public renderer resource lifetime instead of retaining stale cache entries after handle destruction.

### 2026-05-20 API note: custom MSAA resolve support query

`RhiCustomResolveSupport` exposes the custom resolve paths available on a device. Backend-wgpu reports support for `Rgba8StorageCompute`, `Rgba16FloatStorageCompute`, `Rgba32FloatStorageCompute`, `EightBitColorFragment`, and `Depth32FloatFragment`. Headless RHI reports those user-WGSL paths as unsupported rather than emulating shader execution. `RhiDevice::custom_resolve_support()` and `PassContext::rhi_custom_resolve_support()` let tools and graph callbacks choose a supported custom resolve path before dispatching it.





### 2026-05-20 API note: cooperative background resource retirement

`Renderer::start_background_resource_retirement()` enables cooperative background retirement instead of returning `UnsupportedFeature`. Starting the service performs one retirement tick using the same upload/submission-boundary/backend tombstone path as `poll_resource_retirements()`. `Renderer::background_resource_retirement_active()`, `Renderer::stop_background_resource_retirement()`, `ResourceRetirementStats::background_retirement_active`, and `MemoryStats::background_retirement_active` expose the service state to tools and captures. This is cooperative renderer-owned retirement; it uses a lightweight scheduler thread for tick requests but does not mutate renderer/wgpu state off the main thread or claim true nonblocking per-submission backend completion query.


Implementation refinement: the background retirement service starts a lightweight scheduler thread that atomically requests retirement ticks. Renderer-thread safe points consume those tick requests and run the existing retirement path, so renderer arenas and backend-wgpu objects remain thread-affine.

### 2026-05-20 API note: pipeline cache backend coverage

`Renderer::pipeline_cache_backend_coverage()` exposes `PipelineCacheBackendCoverage`, a per-cache summary of facade entries versus backend-native object coverage. It reports total entries, ready entries, backend-object-backed entries, missing backend-object entries, used missing entries, a `complete` flag, and the missing `PipelineKey` list. Facade entries synchronize `has_backend_object` from active backend-wgpu native pipeline objects when cache stats refresh.

### 2026-05-20 API note: post-process backend coverage

`FramePostProcessBackendCoverage` reports whether declared post-process outputs were covered by backend-native post-process labels. `FrameStats`, `FrameDebugReport`, and `FrameCapture` expose `post_process_backend_coverage()` helpers. The matcher understands dynamic backend-wgpu post-process labels and maps tokens such as Bloom, Fxaa, Taa, Motion Blur, Ssr, Depth Of Field, Tonemap, and Color Grading back to semantic pass labels.

### 2026-05-20 API note: post-process support matrix

`Renderer::post_process_support()` returns `PostProcessSupport`, which lists HDR, bloom, TAA, FXAA, SSAO, SSR, depth of field, motion blur, tonemap, and color grading support. The matrix distinguishes `FacadeOnly` from `BackendSampledMinimal`, exposes backend label tokens used for native coverage matching, and keeps production readiness separate from backend visibility.

`PostProcessSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `post_process_support`. The renderer fills the field from `Renderer::post_process_support()` during frame instrumentation/resource-dump construction, so capture/debug artifacts preserve the same per-effect backend visibility and production-readiness gap as the standalone query.

### 2026-05-20 API note: deformation support matrix

`Renderer::deformation_support()` returns `DeformationSupport`, covering skeletal animation, morph targets, LOD selection, motion vectors, and backend GPU deformation. Each entry exposes whether it is supported, whether the implementation is facade-semantic, graph-observable, or backend-GPU, and the remaining limitation when applicable.

`DeformationSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `deformation_support`. The renderer fills the field from `Renderer::deformation_support()` during frame instrumentation/resource-dump construction, so tooling can see the supported facade/graph deformation semantics and the remaining backend GPU deformation gap directly from captures and editor frame reports.

### 2026-05-20 API note: lighting and IBL support matrix

`Renderer::lighting_support()` returns `RendererLightingSupport`, covering retained lights, shadow mapping, environment IBL, backend IBL convolution, and environment capture. Each entry exposes whether it is supported, whether the implementation is facade-semantic, graph-observable, or backend-generated, and the remaining limitation when applicable.

`RendererLightingSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `lighting_support`. The renderer fills the field from `Renderer::lighting_support()` during frame instrumentation/resource-dump construction, so editor/debug and capture tooling can distinguish retained/graph-observable lighting from the still-missing backend IBL convolution and runtime environment capture paths.

### 2026-05-20 API note: frame capture support matrix

`Renderer::frame_capture_support()` returns `FrameCaptureSupport`, aggregating per-backend capture info into internal capture support, registered external hook backends, native-SDK blocked backends, unavailable backends, and `complete_native_sdk_integration`. It complements `frame_capture_backend_info()` / `frame_capture_backend_infos()` for tools that need one capture capability snapshot.

`FrameCaptureSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `frame_capture_support`. The renderer fills the field from `Renderer::frame_capture_support()` during frame instrumentation/resource-dump construction, so capture/debug artifacts preserve the same internal capture, external hook, native SDK blocker, unavailable backend, and complete-native-integration snapshot as the standalone query.

### 2026-05-20 API note: debug tooling support matrix

`Renderer::debug_tooling_support()` returns `DebugToolingSupport`, covering debug draw commands, picking readback, frame debug reports, frame capture, and native frame debugger capture. Each entry exposes support state, implementation level, and remaining limitation text.

`DebugToolingSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `debug_tooling_support`. The renderer fills the field from `Renderer::debug_tooling_support()` during frame instrumentation/resource-dump construction, so debug/editor and capture artifacts preserve the same debug draw, picking, frame report, frame capture, and native debugger SDK blocker snapshot as the standalone query.

### 2026-05-20 API note: resource lifecycle support matrix

`Renderer::resource_lifecycle_support()` returns `ResourceLifecycleSupport`, covering mesh, buffer, texture, sampler, shader, material, material template, scene, view, render target, camera, environment, graph extension, skeleton instance, morph weights, LOD group, and pipeline cache entries. Each class exposes lifecycle/stale-handle coverage, upload/readback applicability, residency, stats/capture/debug observability, backend residency level, and limitation text.

`ResourceLifecycleSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `resource_lifecycle_support`. The renderer fills the field from `Renderer::resource_lifecycle_support()` during frame instrumentation/resource-dump construction, so lifecycle/stale-handle/residency/backend-persistent gaps are visible directly from editor frame reports and capture payloads.

### 2026-05-20 API note: backend synchronization support matrix

`Renderer::backend_synchronization_support()` returns `BackendSynchronizationSupport`, covering submission-boundary retirement, backend tombstone retirement, queue-empty fallback polling, true nonblocking submission-index polling, and background retirement scheduling. Each entry exposes support state, implementation level, and limitation text; the aggregate also reports whether background retirement is currently active.

`BackendSynchronizationSupport` is also included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `backend_synchronization_support`. The renderer fills the field from `Renderer::backend_synchronization_support()` during frame instrumentation/resource-dump construction, so capture/debug artifacts preserve the same backend retirement, fallback polling, true-nonblocking polling, and scheduler-state boundary as the standalone query.

### 2026-05-20 API note: RenderGraph support matrix

`Renderer::render_graph_support()` returns `RendererRenderGraphSupport`, a product-facing query for renderer graph capability boundaries. The report contains `RendererGraphCapabilitySupport` entries keyed by `RendererGraphCapability` and classified by `RendererGraphCapabilityLevel`.

Covered capabilities include public buffer import/export, public D2 texture import/export, packed mip compatibility, flattened layer compatibility, graph-created D2 transient promotion, graph-created MSAA resolve promotion, custom MSAA resolve PassContext integration, persistent backend import cache, readback-backed surface graph export, and direct swapchain graph export.

The current matrix reports direct swapchain graph export as capability-gated/unsupported. This API is intended for tools and examples to make renderer boundaries explicit; it does not replace the full renderer implementation work required by the goal.

### 2026-05-20 API note: backend material resource dependency invalidation

Renderer resource mutations now actively invalidate backend material dependencies. Texture updates, generated mip changes, and texture destruction resolve materials referencing the texture, unregister backend-wgpu material texture bindings, and invalidate affected native pipeline objects. Sampler destruction performs the same cleanup for material sampler bindings. Material destruction, parameter removal, and parameter replacement invalidate native pipelines tagged with the material.

This behavior is observable through backend pipeline cache invalidation and backend tombstone retirement stats. It improves backend-resident lifecycle coherence for material-bound textures and samplers, but it is not a full replacement for complete backend dirty synchronization across every resource class.

### 2026-05-20 API note: backend material resource stats

`Renderer::backend_material_resource_stats()` returns `BackendMaterialResourceStats`, exposing whether a backend runtime is active and how many material texture/sampler bindings are currently registered in backend-wgpu. Headless renderers return an inactive zero-count report.

This API is intended for debug/editor tooling and lifecycle audits. It makes backend material resource residency observable, while the full renderer goal still requires complete backend dirty synchronization for every relevant resource class.

### 2026-05-20 API note: backend material stats in frame observability

`BackendMaterialResourceStats` is now included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. The renderer fills the field during frame instrumentation from `Renderer::backend_material_resource_stats()`, so debug/editor tools and captures can inspect backend material texture/sampler binding residency without issuing a separate query.

This improves observability for the backend material lifecycle path. It remains scoped to material external texture/sampler bindings and does not claim complete backend residency synchronization for all resource classes.

### 2026-05-20 API note: graph RHI import cache stats

`Renderer::graph_rhi_import_cache_stats()` returns `RendererGraphRhiImportCacheStats`, exposing persistent graph RHI import cache entry counts and stale public-resource revision counts for texture and buffer imports. The aggregate `all_entries_synchronized` flag reports whether every cached import matches the current public resource revision.

This API supports backend/resource lifecycle audits without disabling persistent graph import cache reuse. A stale entry means the next graph import path must synchronize the cached RHI resource before using it.

### 2026-05-20 API note: graph RHI import cache stats in frame observability

`RendererGraphRhiImportCacheStats` is now included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. The renderer fills the field during frame instrumentation from `Renderer::graph_rhi_import_cache_stats()`, allowing debug/editor tooling and captures to inspect persistent graph import cache entry counts and stale public-resource revision counts.

This improves observability for graph import cache dirty synchronization. It does not by itself complete backend residency synchronization for all renderer resource classes.

### 2026-05-20 API note: graph import cache stale byte/range stats

`RendererGraphRhiImportCacheStats` now includes stale data footprint fields: `stale_texture_bytes`, `stale_buffer_ranges`, `stale_buffer_bytes`, and `stale_bytes`. These fields quantify the public resource data that must be synchronized into cached graph RHI imports on the next graph import path.

This makes persistent graph import dirty synchronization measurable through direct renderer queries and the frame/debug/capture surfaces that already carry `RendererGraphRhiImportCacheStats`.

### 2026-05-20 API note: pipeline cache backend coverage in frame observability

`PipelineCacheBackendCoverage` is now included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. The renderer fills the field during frame instrumentation from `Renderer::pipeline_cache_backend_coverage()`, allowing debug/editor tooling and captures to inspect facade/backend pipeline object coverage alongside regular pipeline cache counters.

This improves observability for backend pipeline cache completeness. It does not mark `CompleteBackendPipelineCache` implemented unless the coverage report itself shows complete backend object coverage for real rendering paths.

### 2026-05-20 API note: pipeline cache backend missing-entry classification

`PipelineCacheBackendCoverage` now reports `ready_missing_backend_object_entries` and `unused_missing_backend_object_entries` in addition to existing total and used missing backend object counters. Tools can use these fields to distinguish pipeline cache entries that are ready in the facade but not backend-backed, and whether those entries were used this frame.

This is diagnostic coverage for backend pipeline cache completeness; it does not by itself make `CompleteBackendPipelineCache` implemented.

### 2026-05-20 API note: backend submission completion report

`Renderer::backend_submission_completion_report()` returns `BackendSubmissionCompletionReport`, describing backend submission completion observability. The report includes backend-active state, queue-empty polling support, last poll queue-empty result, queue-empty fallback use, recorded submission-index state, true nonblocking submission-index polling support, and limitation text.

`BackendSubmissionCompletionReport` is also included in `FrameStats`, `FrameDebugReport`, and `FrameCapture`. Current backend-wgpu behavior remains queue-empty fallback based; the report intentionally keeps that separate from true nonblocking per-submission completion support.

### 2026-05-20 API note: backend submission completion in resource dumps

`FrameCaptureResourceDump` now includes `backend_submission_completion: BackendSubmissionCompletionReport`. Resource dumps therefore preserve the same backend completion observability as `FrameStats`, `FrameDebugReport`, and `FrameCapture`, including queue-empty fallback state and true nonblocking completion limitation text.

### 2026-05-20 API note: backend submission completion in retirement stats

`ResourceRetirementStats` now includes `backend_submission_completion: BackendSubmissionCompletionReport`. Callers of `Renderer::poll_resource_retirements()` can inspect backend-active state, queue-empty fallback state, submission-index recording state, and true nonblocking completion support without separately querying frame or debug objects.

### 2026-05-20 API note: tombstone counters in backend completion report

`BackendSubmissionCompletionReport` now includes tombstone wait/retire counters: `pending_tombstones`, `tombstones_waiting_for_submission_index`, `tombstones_waiting_for_queue_empty`, `retired_tombstones_this_poll`, `retired_after_queue_empty_poll`, and `retired_after_completed_submission_index_poll`. These fields connect completion polling behavior to backend resource retirement pressure.

### 2026-05-20 API note: explicit nonblocking backend completion poll gate

`Renderer::poll_backend_submission_completion_nonblocking()` attempts to use a true nonblocking backend submission completion query and returns `BackendSubmissionCompletionReport` only when that capability is available. Current backend paths return `RendererError::Validation` explaining that true nonblocking per-submission completion polling is unavailable and that queue-empty fallback remains the active behavior.

This API provides a public error path for the unsupported capability; it does not mark true nonblocking completion as implemented.

### 2026-05-20 API note: explicit direct swapchain graph export gate

`Renderer::require_direct_swapchain_graph_export_supported()` checks whether the current renderer path supports direct swapchain image graph export. Current paths return `RendererError::Validation` with the same limitation reported by `Renderer::surface_graph_export_support()`.

This API provides public capability-gate/error semantics for the unsupported feature. It does not implement native swapchain image export or promotion.

### 2026-05-20 API note: surface graph export support in frame observability

`RendererSurfaceGraphExportSupport` is now included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. The renderer fills the field from `Renderer::surface_graph_export_support()`, making direct swapchain graph export support, readback-backed export support, readback enablement, and limitation text visible to debug/editor tooling and captures.

### 2026-05-20 API note: RenderGraph support matrix in frame observability

`RendererRenderGraphSupport` is now included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump`. The renderer fills the field from `Renderer::render_graph_support()`, making graph capability support and limitations visible to debug/editor tooling and captures without a separate query.

### 2026-05-22 API note: RHI support matrix in frame observability

`Renderer::rhi_support()` returns `RendererRhiSupport`, covering headless RHI device availability, RenderGraph RHI execution, active backend-wgpu runtime, native backend pass metrics, and complete backend execution coverage. Each `RendererRhiFeatureSupport` entry exposes support state, implementation level, and limitation text so tools can distinguish graph/headless evidence from full backend-renderer coverage.

`RendererRhiSupport` is included in `FrameStats`, `FrameDebugReport`, `FrameCapture`, and `FrameCaptureResourceDump` as `rhi_support`. The renderer fills the field from `Renderer::rhi_support()` during frame instrumentation/resource-dump construction, so debug/editor and capture artifacts preserve the same RHI/backend execution boundary as the standalone query.
