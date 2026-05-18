# Render Smoke

This example opens a platform window, creates a `wgpu` surface through
`graphics_wgpu`, and drives `render_wgpu::MeshRenderer` each redraw.

The CPU side owns an `engine_render::RenderScene` with a perspective camera, one
textured cube mesh with normals, one procedural checkerboard texture, one opaque
material, two alpha-blended materials, and four mesh instances. The first two
instances share the opaque material so the queue can submit them as one
instanced batch. Scene resources use generational handles, so removed resources
can safely reuse slots without letting stale handles point at the new object.

Each frame builds an `engine_render::RenderQueue` from the scene. The queue owns
the main pass state, including depth settings, filters hidden instances, and
produces a stable list of render items plus consecutive mesh/material batches.
The smoke scene enables frustum culling, so mesh bounds outside the active
camera are skipped before batching. Opaque items are sorted by `sort_order` and
instance index, then drawn with depth writes enabled by default. Alpha-blended
items are placed after opaque items and sorted back-to-front using the active
camera depth; the helper materials disable depth writes so they still depth-test
against opaque geometry without blocking later transparent draws.

`graphics_wgpu::WgpuSurface` owns the color surface, a resize-aware depth
target, and a multisampled color target when the adapter supports 4x or 2x
MSAA for the selected surface/depth formats. The resolved image is presented
through the swapchain. `render_wgpu::WgpuRenderScene` mirrors scene handles with
sparse GPU-side caches, prepares GPU meshes, textures, material bind groups, and
instance data, then submits the batches to `MeshRenderer`. Material textures are
uploaded with CPU-generated mip chains and repeat/linear/linear sampler state.
The mesh vertex path carries primary and secondary UVs, and the material uniform
can apply glTF-style core and extension material texture transforms before sampling. Material
bind groups carry per-slot sampler state for glTF wrap and filter modes. The renderer
reuses one instance buffer, sends normal and model-view-projection matrices to the
shader, uploads pass lighting uniforms, selects opaque or alpha-blend pipelines
per batch, and issues instanced draws for compatible consecutive items. The
smoke app also captures the scene into a 128 px runtime `WgpuEnvironmentProbe`
cubemap each redraw, prefilters the probe into roughness mips, bakes the first
captured probe back to validated RGBA8 cubemap mip/face data, reloads that baked
asset once as a `WgpuEnvironmentTexture`, and uses the live cubemap for the main
IBL pass through a probe volume. This exercises the renderer's automatic probe
selection, baked-probe asset path, and box-projected parallax correction path.
The shader uses mesh
normals with configurable ambient, directional, point, and spot lighting plus
cascaded directional shadows, point/spot shadows, PBR surface terms including
clearcoat, sheen, transmission, specular, anisotropy, iridescence, volume
attenuation, and dispersion. The smoke materials also reuse the checkerboard as
iridescence and volume-thickness data so the packed optical-extension texture
path is exercised alongside cubemap environment response. One transparent
material is marked unlit so the `KHR_materials_unlit` shader branch is exercised
without removing alpha coverage. The same environment texture is rendered as a
visible skybox background so the background pass is covered by the runtime smoke
test. The smoke scene keeps rotating cubes at different z values while animated
point and spot lights move through the scene, so perspective, lighting,
shadowing, material response, unlit shading, depth, batching, transparent
ordering, runtime cubemap probe capture, baked probe readback/reload, and skybox
rendering are all exercised. Press Escape or close the window to exit.
