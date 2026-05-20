# Render Architecture

Graphics and render code are split so backend-neutral scene policy stays out of
WGPU resource management.

- `engine_graphics`: shared graphics surface contracts, color/size types, and
  backend-neutral errors.
- `graphics_wgpu`: WGPU device and surface ownership, swapchain frame lifecycle,
  resize handling, configurable 1x/2x/4x/8x/16x surface MSAA when supported by
  the chosen color/depth formats, resolve into the swapchain image, and the
  depth target attached to each surface.
- `engine_render`: render scene resources, generational handles, orthographic,
  perspective, and imported view-projection camera descriptions, 3D transforms, mesh vertex data with
  normals, Wavefront OBJ geometry import with MTL material-library colors and
  surface parameters, static glTF/GLB mesh/material import, resolver-driven
  MTL/glTF/GLB base-color texture references, glTF/GLB metallic-roughness,
  normal, emissive, and occlusion texture references, glTF alpha masks,
  glTF scenes with or without mesh primitives, glTF triangle list/strip/fan primitive import,
  glTF double-sided material flags, glTF `KHR_materials_unlit` shading flags,
  glTF `KHR_lights_punctual` directional, point, and spot light import from
  active scene nodes, glTF perspective and orthographic camera import from
  active scene nodes, glTF `TEXCOORD_1` vertex UV import with FLOAT and
  normalized unsigned 8/16-bit UV accessors, sparse accessor overlays for
  `KHR_mesh_quantization` BYTE/SHORT normalized vertex positions, normals, and tangents,
  imported glTF attributes, RGBA vertex colors, indices, skins, morphs, and animations,
  `KHR_texture_transform` offset/rotation/scale and texCoord import for core
  material textures and supported material extension texture slots,
  glTF texture sampler wrap/filter import,
  glTF `EXT_texture_webp` and `KHR_texture_basisu` source selection for resolver-backed
  external WebP/KTX2 texture references,
  glTF `EXT_mesh_gpu_instancing` node instance transform import,
  including FLOAT and normalized signed 8/16-bit instance rotation accessors,
  glTF material extension factors for
  clearcoat, sheen, transmission, IOR, emissive strength, specular, and
  anisotropy, plus specular-glossiness, iridescence, volume attenuation, and dispersion,
  glTF material extension texture references for clearcoat, clearcoat normal, sheen,
  transmission, specular, anisotropy, iridescence, iridescence thickness, and
  volume thickness, generated mesh normals when glTF omits NORMAL, generated mesh tangents, glTF TANGENT
  attributes, glTF/GLB embedded image buffer views, glTF animation clip
  import/sampling for node translation,
  rotation, scale, and morph weights with STEP/LINEAR/CUBICSPLINE interpolation,
  including FLOAT and normalized signed 8/16-bit rotation outputs,
  animation layer playback, looping, speed control, and weighted clip blending,
  sampled glTF node TRS animation application to imported scene instances,
  sampled glTF morph weight application to imported scene meshes including
  POSITION/NORMAL/TANGENT target deltas, sampled glTF
  joint TRS application to imported skinned meshes, scene lighting with
  configurable ambient and environment diffuse/specular IBL terms, optional
  texture-backed environment maps with optional visible skybox backgrounds,
  cascaded directional shadows,
  point-light shadow configuration, and a spot-light shadow configuration,
  PNG/JPEG/WebP/TGA and uncompressed plus zlib-supercompressed R/RG/RGB/BGR/RGBA/BGRA KTX2 texture decoding,
  uncompressed/RLE4/RLE8 1-bit paletted/4-bit paletted/8-bit paletted/16-bit RGB555/bitfields/24-bit/32-bit BMP and uncompressed/RLE color-mapped/true-color/grayscale TGA texture decoding,
  glTF 2.0 asset-version validation,
  `extensionsRequired` validation for known supported glTF extensions,
  glTF primitive material-index, attribute-count, skin joint-index, and index-bounds validation,
  glTF animation target-node and material alpha-mode validation, material pipeline state, and queue policy.
  The queue filters
  visibility, can cull mesh bounds against the active camera frustum when
  enabled, draws opaque items first, sorts transparent items back-to-front using
  camera-aware depth, and batches only consecutive compatible mesh/material
  items.
- `render_wgpu`: GPU mirrors for meshes, textures, materials, and instances;
  reusable instance buffers; opaque and alpha-blend pipeline variants with
  material-driven single-sided back-face culling or double-sided rendering;
  CPU-generated mip chains and repeat/linear/linear sampler state for ordinary
  material textures;
  render pipelines configured for the surface sample count plus single-sample
  probe-capture pipelines;
  shader inputs for model and normal matrices plus pass-level view-projection; pass
  uniforms for ambient, directional, up to four point lights, and up to four
  spot lights; environment diffuse/specular IBL uniforms, source environment
  textures converted into cubemaps, CPU-generated GGX-prefiltered cubemap mip
  chains, roughness-driven environment LOD selection, a generated split-sum BRDF
  LUT, a full-screen cubemap skybox pass for visible environment backgrounds,
  and runtime `WgpuEnvironmentProbe` cubemap capture with GGX-prefiltered
  probe mips; baked environment probe readback into validated RGBA8 cubemap
  mip/face assets and reload into `WgpuEnvironmentTexture`; automatic selection and normalized blending for up to four
  environment probe volumes, with optional box-projected parallax correction;
  up to four cascaded directional shadow maps in a depth texture array; up to four point-light shadow maps represented as six depth-array
  faces per light; up to four spot-light shadow maps in a depth texture array; material uniforms for base
  tint, roughness, metallic, normal scale, alpha cutoff, emissive, and
  occlusion strength; base-color, metallic-roughness, normal, emissive, and
  occlusion texture bindings; clearcoat, clearcoat-roughness, clearcoat-normal, sheen-color,
  sheen-roughness, transmission, specular, specular-color, and anisotropy
  texture bindings; a packed optical-extension texture binding for
  iridescence intensity, iridescence thickness, and volume thickness;
  `TEXCOORD_1` vertex attributes, core and extension material
  `KHR_texture_transform` UV selection and offset/rotation/scale in the shader;
  per-slot material sampler bindings for glTF wrap and filter modes; tangent-space base and clearcoat normal map shading; unlit base-color shading for
  `KHR_materials_unlit` materials; alpha-mask
  fragment discard; direct-light Cook-Torrance/GGX BRDF shading;
  roughness-aware environment Fresnel; approximate clearcoat, sheen,
  transmission, specular color/factor, anisotropy, iridescence, volume
  attenuation, dispersion, IOR-adjusted dielectric F0, and emissive-strength material
  terms; and draw submission from `RenderBatch`
  ranges.

Default materials are opaque and write depth. Alpha-blended helper materials use
alpha blending and disable depth writes, which lets transparent geometry
depth-test against opaque geometry while still drawing in sorted order. The
current 3D path includes perspective projection, indexed cube geometry, normals,
normal matrices, configurable ambient/directional/point/spot lighting, OBJ
geometry import, roughness/metallic material parameters with a simple specular
term, MTL diffuse/alpha/roughness/metallic import, MTL base-color texture
reference resolution, explicit model-matrix instances, minimal static glTF/GLB
triangle mesh import with scene node hierarchy traversal, matrix/TRS node
transforms, quaternion rotation, FLOAT POSITION/NORMAL/TEXCOORD_0 attributes,
COLOR_0 RGB/RGBA vertex colors, u8/u16/u32 indices, triangle strip/fan triangulation,
generated normal and tangent frames, glTF
TANGENT attributes, `KHR_mesh_quantization` BYTE/SHORT normalized
POSITION/NORMAL/TANGENT attributes, secondary `TEXCOORD_1` UVs, normalized unsigned 8/16-bit
UV accessor import, sparse accessor import, base-color texture references,
core and extension material `KHR_texture_transform` atlas transforms for base-color,
metallic-roughness, normal, emissive, occlusion, clearcoat, clearcoat-normal, sheen,
transmission, specular, anisotropy, iridescence, and volume thickness textures,
glTF texture sampler wrap/filter import and per-slot WGPU material samplers,
  glTF `EXT_texture_webp` and `KHR_texture_basisu` source selection for resolver-backed
  external WebP/KTX2 texture references,
  glTF `EXT_mesh_gpu_instancing` expansion into imported scene instances,
including normalized signed 8/16-bit instance rotation accessors,
metallic-roughness texture references sampled by the WGPU shader, normal texture references sampled in
tangent space, emissive and occlusion texture references, alpha-mask cutoff,
glTF double-sided and `KHR_materials_unlit` material flags, glTF
`KHR_lights_punctual` scene light import, glTF scene camera import including
camera/light-only scene assets,
material-driven face culling, and PBR base
color/roughness/metallic/emissive/occlusion factors plus clearcoat, sheen,
transmission, IOR, emissive-strength, specular, specular-glossiness factors and packed
specular/glossiness texture workflow with glossiness-alpha roughness and diffuse/F0 remapping, anisotropy, iridescence,
volume, and dispersion factors and common clearcoat, clearcoat-normal, sheen, transmission,
specular, anisotropy, iridescence, iridescence-thickness, and volume-thickness
extension textures, static morph target baking
from mesh/node weights with POSITION/NORMAL/TANGENT target deltas, static CPU skinning from JOINTS_0/WEIGHTS_0, skin
joints, inverse bind matrices, and tangent vectors, embedded image buffer views
when the image bytes are in a decodable format, extension- or magic-sniffed
PNG/JPEG/WebP/TGA decoding, uncompressed plus zlib-supercompressed KTX2 R/RG/RGB/BGR/RGBA/BGRA decoding, uncompressed/RLE4/RLE8 1-bit paletted/4-bit paletted/8-bit paletted/16-bit RGB555/bitfields/24-bit/32-bit BMP texture decoding, uncompressed/RLE color-mapped/true-color/grayscale TGA texture decoding, mipmapped linear material texture sampling,
direct-light GGX PBR shading, up to four cascaded
directional shadow maps, up to four point-light shadow maps, up to four
spot-light shadow maps, environment diffuse/specular IBL terms with
optional texture-backed cubemap sampling and visible skybox backgrounds,
surface MSAA with swapchain resolve when the adapter supports the requested
sample count,
importance-sampled GGX environment
mip prefiltering for roughness LOD sampling, a generated split-sum BRDF LUT for
environment specular response, opt-in frustum culling, and glTF
animation clip data sampling for node TRS and morph weights, including normalized signed
8/16-bit rotation outputs, plus applying
sampled node TRS clips to imported scene instances and sampled morph weights or
hierarchical joint TRS clips to imported scene meshes, with layer
playback/looping/speed control and weighted clip blending, plus explicit
runtime cubemap environment probe capture, baked probe readback/reload for
offline or editor-authored asset workflows, and parallax-corrected probe volume
blending. Assets that are not glTF 2.0, declare unsupported required glTF extensions, or have invalid
primitive material, attribute, skin joint, or index layouts are rejected instead of imported partially.
Animations that target missing nodes are rejected during asset import.
Materials with invalid alpha modes are rejected instead of being silently rendered opaque.
It does not yet include built-in KTX2 BasisLZ/ETC1S/UASTC or Zstd-supercompressed texture decoding,
Draco/meshopt compressed geometry decoding,
an editor UI for authoring bake jobs, or full
spectral/anisotropic BRDF parity with glTF reference renderers. The
iridescence, volume, and dispersion paths are still realtime approximations,
but iridescence intensity, iridescence thickness, and volume thickness textures
are packed into one internal data texture so the shader does not consume three
additional sampled-texture bindings.
