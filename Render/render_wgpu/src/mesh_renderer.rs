use std::{cell::Cell, cmp::Ordering, sync::mpsc};

use engine_graphics::{Color, GraphicsError, GraphicsResult, RenderSurface};
use engine_render::{
    BlendMode, Camera, DirectionalShadow, Mat4, Material, PerspectiveCamera, PointLight,
    RenderDepthDesc, RenderLighting, SpotLight, Texture, TextureAddressMode, TextureFilterMode,
    TextureSampler, TextureTransform, MAX_DIRECTIONAL_SHADOW_CASCADES, MAX_POINT_LIGHTS,
    MAX_SPOT_LIGHTS,
};
use graphics_wgpu::{wgpu, WgpuGraphics, WgpuSurface, DEFAULT_SAMPLE_COUNT};

use crate::{
    EnvironmentProbeBlend, EnvironmentProbeDesc, WgpuEnvironmentProbe, WgpuEnvironmentTexture,
    WgpuMesh, WgpuTexture, MAX_ENVIRONMENT_PROBE_BLEND,
};

const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const BRDF_LUT_SIZE: u32 = 128;
const POINT_SHADOW_FACE_COUNT: usize = 6;
const MAX_POINT_SHADOW_FACES: usize = MAX_POINT_LIGHTS * POINT_SHADOW_FACE_COUNT;
const MATERIAL_SAMPLER_COUNT: usize = 15;

#[derive(Debug, Clone, Copy)]
struct DirectionalShadowCascades {
    count: usize,
    view_projections: [Mat4; MAX_DIRECTIONAL_SHADOW_CASCADES],
    splits: [f32; MAX_DIRECTIONAL_SHADOW_CASCADES],
    camera_forward: [f32; 3],
}

impl DirectionalShadowCascades {
    fn disabled(camera_forward: [f32; 3]) -> Self {
        Self {
            count: 0,
            view_projections: [Mat4::IDENTITY; MAX_DIRECTIONAL_SHADOW_CASCADES],
            splits: [0.0; MAX_DIRECTIONAL_SHADOW_CASCADES],
            camera_forward,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InstanceRaw {
    model_view_projection: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 3],
    model: [[f32; 4]; 4],
}

unsafe impl bytemuck::Zeroable for InstanceRaw {}
unsafe impl bytemuck::Pod for InstanceRaw {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SampledPostProcessUniform {
    texel_size_and_flags: [f32; 4],
    color_grade_flags: [f32; 4],
    effect_flags: [f32; 4],
    screen_space_flags: [f32; 4],
}

unsafe impl bytemuck::Zeroable for SampledPostProcessUniform {}
unsafe impl bytemuck::Pod for SampledPostProcessUniform {}

impl SampledPostProcessUniform {
    fn new(width: u32, height: u32, options: WgpuPostProcessOptions) -> Self {
        Self {
            texel_size_and_flags: [
                1.0 / width.max(1) as f32,
                1.0 / height.max(1) as f32,
                if options.fxaa { 1.0 } else { 0.0 },
                if options.bloom { 1.0 } else { 0.0 },
            ],
            color_grade_flags: [if options.color_grading { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
            effect_flags: [
                if options.taa { 1.0 } else { 0.0 },
                if options.motion_blur { 1.0 } else { 0.0 },
                if options.ssr { 1.0 } else { 0.0 },
                if options.depth_of_field { 1.0 } else { 0.0 },
            ],
            screen_space_flags: [
                if options.ssao { 1.0 } else { 0.0 },
                if options.hdr { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WgpuPostProcessOptions {
    pub fxaa: bool,
    pub bloom: bool,
    pub color_grading: bool,
    pub taa: bool,
    pub motion_blur: bool,
    pub ssr: bool,
    pub depth_of_field: bool,
    pub ssao: bool,
    pub hdr: bool,
}

fn sampled_post_process_pass_label(options: WgpuPostProcessOptions) -> String {
    let mut label = String::from("Neo");
    if options.hdr {
        label.push_str(" Hdr");
    }
    if options.bloom {
        label.push_str(" Bloom");
    }
    if options.ssao {
        label.push_str(" Ssao");
    }
    if options.taa {
        label.push_str(" Taa");
    }
    if options.fxaa {
        label.push_str(" Fxaa");
    }
    if options.motion_blur {
        label.push_str(" Motion Blur");
    }
    if options.ssr {
        label.push_str(" Ssr");
    }
    if options.depth_of_field {
        label.push_str(" Depth Of Field");
    }
    label.push_str(" Tonemap");
    if options.color_grading {
        label.push_str(" Color Grading");
    }
    label.push_str(" Post Process Pass");
    label
}

impl InstanceRaw {
    const VEC4_SIZE: wgpu::BufferAddress = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
    const NORMAL_MATRIX_OFFSET: wgpu::BufferAddress =
        std::mem::size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress;
    const MODEL_OFFSET: wgpu::BufferAddress =
        Self::NORMAL_MATRIX_OFFSET + std::mem::size_of::<[[f32; 4]; 3]>() as wgpu::BufferAddress;

    const ATTRIBUTES: [wgpu::VertexAttribute; 7] = [
        wgpu::VertexAttribute {
            offset: Self::NORMAL_MATRIX_OFFSET,
            shader_location: 6,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::NORMAL_MATRIX_OFFSET + Self::VEC4_SIZE,
            shader_location: 7,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::NORMAL_MATRIX_OFFSET + Self::VEC4_SIZE * 2,
            shader_location: 8,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET,
            shader_location: 9,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET + Self::VEC4_SIZE,
            shader_location: 10,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET + Self::VEC4_SIZE * 2,
            shader_location: 11,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET + Self::VEC4_SIZE * 3,
            shader_location: 12,
            format: wgpu::VertexFormat::Float32x4,
        },
    ];

    const SHADOW_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET,
            shader_location: 9,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET + Self::VEC4_SIZE,
            shader_location: 10,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET + Self::VEC4_SIZE * 2,
            shader_location: 11,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: Self::MODEL_OFFSET + Self::VEC4_SIZE * 3,
            shader_location: 12,
            format: wgpu::VertexFormat::Float32x4,
        },
    ];

    fn from_matrices(model_view_projection: Mat4, normal_matrix: Mat4, model: Mat4) -> Self {
        let normal_matrix = normal_matrix.to_cols_array();
        Self {
            model_view_projection: model_view_projection.to_cols_array(),
            normal_matrix: [normal_matrix[0], normal_matrix[1], normal_matrix[2]],
            model: model.to_cols_array(),
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }

    fn shadow_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::SHADOW_ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MaterialUniform {
    tint: [f32; 4],
    surface: [f32; 4],
    emissive_occlusion: [f32; 4],
    clearcoat_transmission: [f32; 4],
    sheen: [f32; 4],
    specular: [f32; 4],
    anisotropy: [f32; 4],
    iridescence: [f32; 4],
    volume: [f32; 4],
    volume_options: [f32; 4],
    base_color_uv_transform_0: [f32; 4],
    base_color_uv_transform_1: [f32; 4],
    metallic_roughness_uv_transform_0: [f32; 4],
    metallic_roughness_uv_transform_1: [f32; 4],
    normal_uv_transform_0: [f32; 4],
    normal_uv_transform_1: [f32; 4],
    emissive_uv_transform_0: [f32; 4],
    emissive_uv_transform_1: [f32; 4],
    occlusion_uv_transform_0: [f32; 4],
    occlusion_uv_transform_1: [f32; 4],
    clearcoat_uv_transform_0: [f32; 4],
    clearcoat_uv_transform_1: [f32; 4],
    clearcoat_roughness_uv_transform_0: [f32; 4],
    clearcoat_roughness_uv_transform_1: [f32; 4],
    clearcoat_normal_uv_transform_0: [f32; 4],
    clearcoat_normal_uv_transform_1: [f32; 4],
    sheen_color_uv_transform_0: [f32; 4],
    sheen_color_uv_transform_1: [f32; 4],
    sheen_roughness_uv_transform_0: [f32; 4],
    sheen_roughness_uv_transform_1: [f32; 4],
    transmission_uv_transform_0: [f32; 4],
    transmission_uv_transform_1: [f32; 4],
    specular_uv_transform_0: [f32; 4],
    specular_uv_transform_1: [f32; 4],
    specular_color_uv_transform_0: [f32; 4],
    specular_color_uv_transform_1: [f32; 4],
    anisotropy_uv_transform_0: [f32; 4],
    anisotropy_uv_transform_1: [f32; 4],
    iridescence_uv_transform_0: [f32; 4],
    iridescence_uv_transform_1: [f32; 4],
    iridescence_thickness_uv_transform_0: [f32; 4],
    iridescence_thickness_uv_transform_1: [f32; 4],
    thickness_uv_transform_0: [f32; 4],
    thickness_uv_transform_1: [f32; 4],
}

unsafe impl bytemuck::Zeroable for MaterialUniform {}
unsafe impl bytemuck::Pod for MaterialUniform {}

#[derive(Debug, Clone, Copy)]
struct EnvironmentProbeUniforms {
    count: usize,
    positions_weights: [[f32; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
    box_mins: [[f32; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
    box_maxs: [[f32; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
}

impl EnvironmentProbeUniforms {
    const EMPTY: Self = Self {
        count: 0,
        positions_weights: [[0.0; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
        box_mins: [[0.0; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
        box_maxs: [[0.0; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
    };

    fn from_blend(probes: &[EnvironmentProbeBlend<'_>]) -> Self {
        let mut uniforms = Self::EMPTY;
        let total_weight = probes
            .iter()
            .take(MAX_ENVIRONMENT_PROBE_BLEND)
            .map(|probe| probe.weight.max(0.0))
            .sum::<f32>();

        if total_weight <= f32::EPSILON {
            return uniforms;
        }

        for (index, probe) in probes.iter().take(MAX_ENVIRONMENT_PROBE_BLEND).enumerate() {
            uniforms.count += 1;
            uniforms.positions_weights[index] = [
                probe.position[0],
                probe.position[1],
                probe.position[2],
                probe.weight.max(0.0) / total_weight,
            ];
            uniforms.box_mins[index] = [
                probe.bounds_min[0],
                probe.bounds_min[1],
                probe.bounds_min[2],
                if probe.parallax_correction { 1.0 } else { 0.0 },
            ];
            uniforms.box_maxs[index] = [
                probe.bounds_max[0],
                probe.bounds_max[1],
                probe.bounds_max[2],
                0.0,
            ];
        }

        uniforms
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RenderUniform {
    view_projection: [[f32; 4]; 4],
    ambient_color: [f32; 4],
    directional_color: [f32; 4],
    directional_direction: [f32; 4],
    camera_position: [f32; 4],
    camera_forward: [f32; 4],
    directional_shadow_view_projections: [[[f32; 4]; 4]; MAX_DIRECTIONAL_SHADOW_CASCADES],
    shadow_options: [f32; 4],
    directional_shadow_splits: [f32; 4],
    spot_shadow_view_projections: [[[f32; 4]; 4]; MAX_SPOT_LIGHTS],
    spot_shadow_options: [[f32; 4]; MAX_SPOT_LIGHTS],
    point_shadow_view_projections: [[[f32; 4]; 4]; MAX_POINT_SHADOW_FACES],
    point_shadow_options: [[f32; 4]; MAX_POINT_LIGHTS],
    environment_diffuse: [f32; 4],
    environment_specular: [f32; 4],
    environment_probe_options: [f32; 4],
    environment_probe_positions_weights: [[f32; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
    environment_probe_box_mins: [[f32; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
    environment_probe_box_maxs: [[f32; 4]; MAX_ENVIRONMENT_PROBE_BLEND],
    point_light_count: [u32; 4],
    point_light_positions: [[f32; 4]; MAX_POINT_LIGHTS],
    point_light_colors: [[f32; 4]; MAX_POINT_LIGHTS],
    spot_light_count: [u32; 4],
    spot_light_positions: [[f32; 4]; MAX_SPOT_LIGHTS],
    spot_light_directions: [[f32; 4]; MAX_SPOT_LIGHTS],
    spot_light_colors: [[f32; 4]; MAX_SPOT_LIGHTS],
    spot_light_angles: [[f32; 4]; MAX_SPOT_LIGHTS],
}

unsafe impl bytemuck::Zeroable for RenderUniform {}
unsafe impl bytemuck::Pod for RenderUniform {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SkyboxUniform {
    camera_right: [f32; 4],
    camera_up: [f32; 4],
    camera_forward: [f32; 4],
    options: [f32; 4],
}

unsafe impl bytemuck::Zeroable for SkyboxUniform {}
unsafe impl bytemuck::Pod for SkyboxUniform {}

impl RenderUniform {
    fn from_lighting(
        view_projection: Mat4,
        lighting: RenderLighting,
        camera_position: [f32; 3],
        directional_shadow_cascades: DirectionalShadowCascades,
        spot_shadow_view_projections: [Mat4; MAX_SPOT_LIGHTS],
        spot_shadow_options: [[f32; 4]; MAX_SPOT_LIGHTS],
        point_shadow_view_projections: [Mat4; MAX_POINT_SHADOW_FACES],
        point_shadow_options: [[f32; 4]; MAX_POINT_LIGHTS],
        environment_mip_level_count: u32,
        environment_probes: EnvironmentProbeUniforms,
    ) -> Self {
        let directional_direction = normalize_or(
            lighting.directional.direction,
            RenderLighting::DEFAULT.directional.direction,
        );
        let point_lights = lighting.point_lights();
        let mut point_light_positions = [[0.0; 4]; MAX_POINT_LIGHTS];
        let mut point_light_colors = [[0.0; 4]; MAX_POINT_LIGHTS];
        let spot_lights = lighting.spot_lights();
        let mut spot_light_positions = [[0.0; 4]; MAX_SPOT_LIGHTS];
        let mut spot_light_directions = [[0.0; 4]; MAX_SPOT_LIGHTS];
        let mut spot_light_colors = [[0.0; 4]; MAX_SPOT_LIGHTS];
        let mut spot_light_angles = [[0.0; 4]; MAX_SPOT_LIGHTS];

        for (index, light) in point_lights.iter().enumerate() {
            let intensity = light.intensity.max(0.0);
            point_light_positions[index] = [
                light.position[0],
                light.position[1],
                light.position[2],
                light.range.max(0.0001),
            ];
            point_light_colors[index] = [
                light.color[0] * intensity,
                light.color[1] * intensity,
                light.color[2] * intensity,
                1.0,
            ];
        }

        for (index, light) in spot_lights.iter().enumerate() {
            let intensity = light.intensity.max(0.0);
            let direction = normalize_or(light.direction, [0.0, -1.0, 0.0]);
            let (inner_cos, outer_cos) =
                spotlight_angle_cosines(light.inner_angle_radians, light.outer_angle_radians);

            spot_light_positions[index] = [
                light.position[0],
                light.position[1],
                light.position[2],
                light.range.max(0.0001),
            ];
            spot_light_directions[index] = [direction[0], direction[1], direction[2], 0.0];
            spot_light_colors[index] = [
                light.color[0] * intensity,
                light.color[1] * intensity,
                light.color[2] * intensity,
                1.0,
            ];
            spot_light_angles[index] = [inner_cos, outer_cos, 0.0, 0.0];
        }

        Self {
            view_projection: view_projection.to_cols_array(),
            ambient_color: [
                lighting.ambient_color[0] * lighting.ambient_intensity.max(0.0),
                lighting.ambient_color[1] * lighting.ambient_intensity.max(0.0),
                lighting.ambient_color[2] * lighting.ambient_intensity.max(0.0),
                1.0,
            ],
            directional_color: [
                lighting.directional.color[0] * lighting.directional.intensity.max(0.0),
                lighting.directional.color[1] * lighting.directional.intensity.max(0.0),
                lighting.directional.color[2] * lighting.directional.intensity.max(0.0),
                1.0,
            ],
            directional_direction: [
                directional_direction[0],
                directional_direction[1],
                directional_direction[2],
                0.0,
            ],
            camera_position: [
                camera_position[0],
                camera_position[1],
                camera_position[2],
                1.0,
            ],
            camera_forward: [
                directional_shadow_cascades.camera_forward[0],
                directional_shadow_cascades.camera_forward[1],
                directional_shadow_cascades.camera_forward[2],
                0.0,
            ],
            directional_shadow_view_projections: directional_shadow_cascades
                .view_projections
                .map(Mat4::to_cols_array),
            shadow_options: [
                if lighting.directional_shadow.enabled {
                    lighting.directional_shadow.strength.clamp(0.0, 1.0)
                } else {
                    0.0
                },
                lighting.directional_shadow.bias.max(0.0),
                if directional_shadow_cascades.count > 0 {
                    1.0
                } else {
                    0.0
                },
                directional_shadow_cascades.count as f32,
            ],
            directional_shadow_splits: directional_shadow_cascades.splits,
            spot_shadow_view_projections: spot_shadow_view_projections.map(Mat4::to_cols_array),
            spot_shadow_options,
            point_shadow_view_projections: point_shadow_view_projections.map(Mat4::to_cols_array),
            point_shadow_options,
            environment_diffuse: [
                lighting.environment.diffuse_color[0]
                    * lighting.environment.diffuse_intensity.max(0.0),
                lighting.environment.diffuse_color[1]
                    * lighting.environment.diffuse_intensity.max(0.0),
                lighting.environment.diffuse_color[2]
                    * lighting.environment.diffuse_intensity.max(0.0),
                1.0,
            ],
            environment_specular: [
                lighting.environment.specular_color[0]
                    * lighting.environment.specular_intensity.max(0.0),
                lighting.environment.specular_color[1]
                    * lighting.environment.specular_intensity.max(0.0),
                lighting.environment.specular_color[2]
                    * lighting.environment.specular_intensity.max(0.0),
                environment_mip_level_count.saturating_sub(1) as f32,
            ],
            environment_probe_options: [environment_probes.count as f32, 0.0, 0.0, 0.0],
            environment_probe_positions_weights: environment_probes.positions_weights,
            environment_probe_box_mins: environment_probes.box_mins,
            environment_probe_box_maxs: environment_probes.box_maxs,
            point_light_count: [point_lights.len() as u32, 0, 0, 0],
            point_light_positions,
            point_light_colors,
            spot_light_count: [spot_lights.len() as u32, 0, 0, 0],
            spot_light_positions,
            spot_light_directions,
            spot_light_colors,
            spot_light_angles,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ShadowUniform {
    shadow_view_projection: [[f32; 4]; 4],
}

unsafe impl bytemuck::Zeroable for ShadowUniform {}
unsafe impl bytemuck::Pod for ShadowUniform {}

pub struct WgpuMeshInstance {
    model_view_projection: Mat4,
    normal_matrix: Mat4,
    model: Mat4,
}

impl WgpuMeshInstance {
    fn new(model_view_projection: Mat4) -> Self {
        Self {
            model_view_projection,
            normal_matrix: Mat4::IDENTITY,
            model: model_view_projection,
        }
    }

    pub fn set_model_view_projection(&mut self, _graphics: &WgpuGraphics, matrix: Mat4) {
        self.model_view_projection = matrix;
        self.normal_matrix = Mat4::IDENTITY;
        self.model = matrix;
    }

    pub fn set_model_view_projection_and_normal_matrix(
        &mut self,
        _graphics: &WgpuGraphics,
        model_view_projection: Mat4,
        normal_matrix: Mat4,
    ) {
        self.model_view_projection = model_view_projection;
        self.normal_matrix = normal_matrix;
        self.model = model_view_projection;
    }

    pub fn set_model_view_projection_normal_and_model_matrix(
        &mut self,
        _graphics: &WgpuGraphics,
        model_view_projection: Mat4,
        normal_matrix: Mat4,
        model: Mat4,
    ) {
        self.model_view_projection = model_view_projection;
        self.normal_matrix = normal_matrix;
        self.model = model;
    }

    fn raw(&self) -> InstanceRaw {
        InstanceRaw::from_matrices(self.model_view_projection, self.normal_matrix, self.model)
    }
}

struct ShadowResources {
    _directional_texture: wgpu::Texture,
    _directional_view: wgpu::TextureView,
    directional_layer_views: Vec<wgpu::TextureView>,
    _spot_texture: wgpu::Texture,
    _spot_view: wgpu::TextureView,
    spot_layer_views: Vec<wgpu::TextureView>,
    _point_texture: wgpu::Texture,
    _point_view: wgpu::TextureView,
    point_layer_views: Vec<wgpu::TextureView>,
    bind_group: wgpu::BindGroup,
    size: u32,
    spot_size: u32,
    point_size: u32,
}

impl ShadowResources {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        size: u32,
        spot_size: u32,
        point_size: u32,
    ) -> Self {
        let size = size.max(1);
        let spot_size = spot_size.max(1);
        let point_size = point_size.max(1);
        let directional_texture = create_shadow_texture_array(
            device,
            "Neo Directional Shadow Texture Array",
            size,
            MAX_DIRECTIONAL_SHADOW_CASCADES as u32,
        );
        let view = directional_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Neo Directional Shadow Texture Array View"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..wgpu::TextureViewDescriptor::default()
        });
        let directional_layer_views = (0..MAX_DIRECTIONAL_SHADOW_CASCADES as u32)
            .map(|layer| {
                directional_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Neo Directional Shadow Layer View"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..wgpu::TextureViewDescriptor::default()
                })
            })
            .collect::<Vec<_>>();
        let spot_texture = create_shadow_texture_array(
            device,
            "Neo Spot Shadow Texture Array",
            spot_size,
            MAX_SPOT_LIGHTS as u32,
        );
        let spot_view = spot_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Neo Spot Shadow Texture Array View"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..wgpu::TextureViewDescriptor::default()
        });
        let spot_layer_views = (0..MAX_SPOT_LIGHTS as u32)
            .map(|layer| {
                spot_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Neo Spot Shadow Layer View"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..wgpu::TextureViewDescriptor::default()
                })
            })
            .collect::<Vec<_>>();
        let point_texture = create_shadow_texture_array(
            device,
            "Neo Point Shadow Texture Array",
            point_size,
            MAX_POINT_SHADOW_FACES as u32,
        );
        let point_view = point_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Neo Point Shadow Texture Array View"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..wgpu::TextureViewDescriptor::default()
        });
        let point_layer_views = (0..MAX_POINT_SHADOW_FACES as u32)
            .map(|layer| {
                point_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Neo Point Shadow Layer View"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..wgpu::TextureViewDescriptor::default()
                })
            })
            .collect::<Vec<_>>();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Neo Shadow Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&spot_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&point_view),
                },
            ],
        });

        Self {
            _directional_texture: directional_texture,
            _directional_view: view,
            directional_layer_views,
            _spot_texture: spot_texture,
            _spot_view: spot_view,
            spot_layer_views,
            _point_texture: point_texture,
            _point_view: point_view,
            point_layer_views,
            bind_group,
            size,
            spot_size,
            point_size,
        }
    }
}

fn create_shadow_texture_array(
    device: &wgpu::Device,
    label: &'static str,
    size: u32,
    layers: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: layers.max(1),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SHADOW_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

const MATERIAL_UNIFORM_BINDING: u32 = 0;
const MATERIAL_TEXTURE_BINDINGS: [u32; 15] = [1, 3, 4, 5, 6, 7, 8, 29, 9, 10, 11, 12, 13, 14, 15];
const MATERIAL_SAMPLER_BINDINGS: [u32; MATERIAL_SAMPLER_COUNT] =
    [2, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 30];
const MATERIAL_OCCUPIED_BINDINGS: [u32; 31] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WgpuMaterialLayoutInfo {
    pub uniform_binding: u32,
    pub texture_bindings: &'static [u32],
    pub sampler_bindings: &'static [u32],
    pub occupied_bindings: &'static [u32],
    pub binding_count: usize,
    pub highest_binding: u32,
}

pub fn wgpu_material_layout_info() -> WgpuMaterialLayoutInfo {
    WgpuMaterialLayoutInfo {
        uniform_binding: MATERIAL_UNIFORM_BINDING,
        texture_bindings: &MATERIAL_TEXTURE_BINDINGS,
        sampler_bindings: &MATERIAL_SAMPLER_BINDINGS,
        occupied_bindings: &MATERIAL_OCCUPIED_BINDINGS,
        binding_count: MATERIAL_OCCUPIED_BINDINGS.len(),
        highest_binding: 30,
    }
}

fn material_bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 31] {
    [
        material_uniform_layout_entry(0),
        material_texture_layout_entry(1),
        material_sampler_layout_entry(2),
        material_texture_layout_entry(3),
        material_texture_layout_entry(4),
        material_texture_layout_entry(5),
        material_texture_layout_entry(6),
        material_texture_layout_entry(7),
        material_texture_layout_entry(8),
        material_texture_layout_entry(9),
        material_texture_layout_entry(10),
        material_texture_layout_entry(11),
        material_texture_layout_entry(12),
        material_texture_layout_entry(13),
        material_texture_layout_entry(14),
        material_texture_layout_entry(15),
        material_sampler_layout_entry(16),
        material_sampler_layout_entry(17),
        material_sampler_layout_entry(18),
        material_sampler_layout_entry(19),
        material_sampler_layout_entry(20),
        material_sampler_layout_entry(21),
        material_sampler_layout_entry(22),
        material_sampler_layout_entry(23),
        material_sampler_layout_entry(24),
        material_sampler_layout_entry(25),
        material_sampler_layout_entry(26),
        material_sampler_layout_entry(27),
        material_sampler_layout_entry(28),
        material_texture_layout_entry(29),
        material_sampler_layout_entry(30),
    ]
}

fn material_uniform_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn material_texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn material_sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

pub struct WgpuMaterial {
    material_buffer: wgpu::Buffer,
    material_bind_group: wgpu::BindGroup,
    _samplers: Vec<wgpu::Sampler>,
    blend_mode: Cell<BlendMode>,
    depth_write: Cell<bool>,
    double_sided: Cell<bool>,
}

impl WgpuMaterial {
    pub fn layout_info() -> WgpuMaterialLayoutInfo {
        wgpu_material_layout_info()
    }

    fn new(
        graphics: &WgpuGraphics,
        material_bind_group_layout: &wgpu::BindGroupLayout,
        source: Material,
        base_color_texture: &WgpuTexture,
        metallic_roughness_texture: &WgpuTexture,
        normal_texture: &WgpuTexture,
        emissive_texture: &WgpuTexture,
        occlusion_texture: &WgpuTexture,
        clearcoat_texture: &WgpuTexture,
        clearcoat_roughness_texture: &WgpuTexture,
        clearcoat_normal_texture: &WgpuTexture,
        sheen_color_texture: &WgpuTexture,
        sheen_roughness_texture: &WgpuTexture,
        transmission_texture: &WgpuTexture,
        specular_texture: &WgpuTexture,
        specular_color_texture: &WgpuTexture,
        anisotropy_texture: &WgpuTexture,
        optical_extension_texture: &WgpuTexture,
    ) -> Self {
        let material_buffer = graphics.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Material Uniform Buffer"),
            size: std::mem::size_of::<MaterialUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let material_samplers = create_material_samplers(graphics.device(), source);
        let material_bind_group = graphics
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Neo Material Bind Group"),
                layout: material_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_UNIFORM_BINDING,
                        resource: material_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[0],
                        resource: wgpu::BindingResource::TextureView(base_color_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[0],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[1],
                        resource: wgpu::BindingResource::TextureView(
                            metallic_roughness_texture.view(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[2],
                        resource: wgpu::BindingResource::TextureView(normal_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[3],
                        resource: wgpu::BindingResource::TextureView(emissive_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[4],
                        resource: wgpu::BindingResource::TextureView(occlusion_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[5],
                        resource: wgpu::BindingResource::TextureView(clearcoat_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[6],
                        resource: wgpu::BindingResource::TextureView(
                            clearcoat_roughness_texture.view(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[8],
                        resource: wgpu::BindingResource::TextureView(sheen_color_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[9],
                        resource: wgpu::BindingResource::TextureView(
                            sheen_roughness_texture.view(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[10],
                        resource: wgpu::BindingResource::TextureView(transmission_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[11],
                        resource: wgpu::BindingResource::TextureView(specular_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[12],
                        resource: wgpu::BindingResource::TextureView(specular_color_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[13],
                        resource: wgpu::BindingResource::TextureView(anisotropy_texture.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[14],
                        resource: wgpu::BindingResource::TextureView(
                            optical_extension_texture.view(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_TEXTURE_BINDINGS[7],
                        resource: wgpu::BindingResource::TextureView(
                            clearcoat_normal_texture.view(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[1],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[2],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[2]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[3],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[3]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[4],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[4]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[5],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[5]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[6],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[6]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[7],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[7]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[8],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[8]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[9],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[9]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[10],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[10]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[11],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[11]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[12],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[12]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[13],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[13]),
                    },
                    wgpu::BindGroupEntry {
                        binding: MATERIAL_SAMPLER_BINDINGS[14],
                        resource: wgpu::BindingResource::Sampler(&material_samplers[14]),
                    },
                ],
            });

        let material = Self {
            material_buffer,
            material_bind_group,
            _samplers: material_samplers,
            blend_mode: Cell::new(source.blend_mode),
            depth_write: Cell::new(source.depth_write),
            double_sided: Cell::new(source.double_sided),
        };
        material.set_material(graphics, source);
        material
    }

    pub fn set_material(&self, graphics: &WgpuGraphics, material: Material) {
        self.blend_mode.set(material.blend_mode);
        self.depth_write.set(material.depth_write);
        self.double_sided.set(material.double_sided);

        let uniform = material_uniform(material);
        graphics
            .queue()
            .write_buffer(&self.material_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode.get()
    }

    pub fn depth_write(&self) -> bool {
        self.depth_write.get()
    }

    pub fn double_sided(&self) -> bool {
        self.double_sided.get()
    }
}

fn create_material_samplers(device: &wgpu::Device, material: Material) -> Vec<wgpu::Sampler> {
    material_sampler_slots(material)
        .into_iter()
        .enumerate()
        .map(|(index, sampler)| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(material_sampler_label(index)),
                address_mode_u: wgpu_address_mode(sampler.address_mode_u),
                address_mode_v: wgpu_address_mode(sampler.address_mode_v),
                address_mode_w: wgpu_address_mode(sampler.address_mode_w),
                mag_filter: wgpu_filter_mode(sampler.mag_filter),
                min_filter: wgpu_filter_mode(sampler.min_filter),
                mipmap_filter: wgpu_filter_mode(sampler.mipmap_filter),
                ..wgpu::SamplerDescriptor::default()
            })
        })
        .collect()
}

fn material_sampler_slots(material: Material) -> [TextureSampler; MATERIAL_SAMPLER_COUNT] {
    let samplers = material.texture_samplers;
    [
        samplers.base_color,
        samplers.metallic_roughness,
        samplers.normal,
        samplers.emissive,
        samplers.occlusion,
        samplers.clearcoat,
        samplers.clearcoat_roughness,
        samplers.clearcoat_normal,
        samplers.sheen_color,
        samplers.sheen_roughness,
        samplers.transmission,
        samplers.specular,
        samplers.specular_color,
        samplers.anisotropy,
        optical_extension_sampler(material),
    ]
}

fn optical_extension_sampler(material: Material) -> TextureSampler {
    if material.iridescence_texture.is_some() {
        return material.texture_samplers.iridescence;
    }
    if material.iridescence_thickness_texture.is_some() {
        return material.texture_samplers.iridescence_thickness;
    }
    if material.thickness_texture.is_some() {
        return material.texture_samplers.thickness;
    }

    TextureSampler::DEFAULT
}

fn material_sampler_label(index: usize) -> &'static str {
    match index {
        0 => "Neo Base Color Sampler",
        1 => "Neo Metallic Roughness Sampler",
        2 => "Neo Normal Sampler",
        3 => "Neo Emissive Sampler",
        4 => "Neo Occlusion Sampler",
        5 => "Neo Clearcoat Sampler",
        6 => "Neo Clearcoat Roughness Sampler",
        7 => "Neo Clearcoat Normal Sampler",
        8 => "Neo Sheen Color Sampler",
        9 => "Neo Sheen Roughness Sampler",
        10 => "Neo Transmission Sampler",
        11 => "Neo Specular Sampler",
        12 => "Neo Specular Color Sampler",
        13 => "Neo Anisotropy Sampler",
        _ => "Neo Optical Extension Sampler",
    }
}

fn wgpu_address_mode(mode: TextureAddressMode) -> wgpu::AddressMode {
    match mode {
        TextureAddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        TextureAddressMode::Repeat => wgpu::AddressMode::Repeat,
        TextureAddressMode::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
    }
}

fn wgpu_filter_mode(mode: TextureFilterMode) -> wgpu::FilterMode {
    match mode {
        TextureFilterMode::Nearest => wgpu::FilterMode::Nearest,
        TextureFilterMode::Linear => wgpu::FilterMode::Linear,
    }
}

fn material_uniform(material: Material) -> MaterialUniform {
    MaterialUniform {
        tint: material.tint,
        surface: [
            material.roughness,
            material.metallic,
            material.normal_scale,
            material.alpha_cutoff,
        ],
        emissive_occlusion: [
            material.emissive[0] * material.emissive_strength,
            material.emissive[1] * material.emissive_strength,
            material.emissive[2] * material.emissive_strength,
            material.occlusion_strength,
        ],
        clearcoat_transmission: [
            material.clearcoat,
            material.clearcoat_roughness,
            material.transmission,
            material.ior,
        ],
        sheen: [
            material.sheen_color[0],
            material.sheen_color[1],
            material.sheen_color[2],
            material.sheen_roughness,
        ],
        specular: [
            material.specular_color[0],
            material.specular_color[1],
            material.specular_color[2],
            material.specular_factor,
        ],
        anisotropy: [
            material.anisotropy_strength,
            material.anisotropy_rotation,
            material.clearcoat_normal_scale,
            0.0,
        ],
        iridescence: [
            material.iridescence_factor,
            material.iridescence_ior,
            material.iridescence_thickness_min,
            material.iridescence_thickness_max,
        ],
        volume: [
            material.attenuation_color[0],
            material.attenuation_color[1],
            material.attenuation_color[2],
            material.thickness_factor,
        ],
        volume_options: [
            material.attenuation_distance,
            material.dispersion,
            if material.unlit { 1.0 } else { 0.0 },
            if material.specular_glossiness_workflow {
                1.0
            } else {
                0.0
            },
        ],
        base_color_uv_transform_0: texture_transform_uniform_0(
            material.base_color_texture_transform,
        ),
        base_color_uv_transform_1: texture_transform_uniform_1(
            material.base_color_texture_transform,
        ),
        metallic_roughness_uv_transform_0: texture_transform_uniform_0(
            material.metallic_roughness_texture_transform,
        ),
        metallic_roughness_uv_transform_1: texture_transform_uniform_1(
            material.metallic_roughness_texture_transform,
        ),
        normal_uv_transform_0: texture_transform_uniform_0(material.normal_texture_transform),
        normal_uv_transform_1: texture_transform_uniform_1(material.normal_texture_transform),
        emissive_uv_transform_0: texture_transform_uniform_0(material.emissive_texture_transform),
        emissive_uv_transform_1: texture_transform_uniform_1(material.emissive_texture_transform),
        occlusion_uv_transform_0: texture_transform_uniform_0(material.occlusion_texture_transform),
        occlusion_uv_transform_1: texture_transform_uniform_1(material.occlusion_texture_transform),
        clearcoat_uv_transform_0: texture_transform_uniform_0(material.clearcoat_texture_transform),
        clearcoat_uv_transform_1: texture_transform_uniform_1(material.clearcoat_texture_transform),
        clearcoat_roughness_uv_transform_0: texture_transform_uniform_0(
            material.clearcoat_roughness_texture_transform,
        ),
        clearcoat_roughness_uv_transform_1: texture_transform_uniform_1(
            material.clearcoat_roughness_texture_transform,
        ),
        clearcoat_normal_uv_transform_0: texture_transform_uniform_0(
            material.clearcoat_normal_texture_transform,
        ),
        clearcoat_normal_uv_transform_1: texture_transform_uniform_1(
            material.clearcoat_normal_texture_transform,
        ),
        sheen_color_uv_transform_0: texture_transform_uniform_0(
            material.sheen_color_texture_transform,
        ),
        sheen_color_uv_transform_1: texture_transform_uniform_1(
            material.sheen_color_texture_transform,
        ),
        sheen_roughness_uv_transform_0: texture_transform_uniform_0(
            material.sheen_roughness_texture_transform,
        ),
        sheen_roughness_uv_transform_1: texture_transform_uniform_1(
            material.sheen_roughness_texture_transform,
        ),
        transmission_uv_transform_0: texture_transform_uniform_0(
            material.transmission_texture_transform,
        ),
        transmission_uv_transform_1: texture_transform_uniform_1(
            material.transmission_texture_transform,
        ),
        specular_uv_transform_0: texture_transform_uniform_0(material.specular_texture_transform),
        specular_uv_transform_1: texture_transform_uniform_1(material.specular_texture_transform),
        specular_color_uv_transform_0: texture_transform_uniform_0(
            material.specular_color_texture_transform,
        ),
        specular_color_uv_transform_1: texture_transform_uniform_1(
            material.specular_color_texture_transform,
        ),
        anisotropy_uv_transform_0: texture_transform_uniform_0(
            material.anisotropy_texture_transform,
        ),
        anisotropy_uv_transform_1: texture_transform_uniform_1(
            material.anisotropy_texture_transform,
        ),
        iridescence_uv_transform_0: texture_transform_uniform_0(
            material.iridescence_texture_transform,
        ),
        iridescence_uv_transform_1: texture_transform_uniform_1(
            material.iridescence_texture_transform,
        ),
        iridescence_thickness_uv_transform_0: texture_transform_uniform_0(
            material.iridescence_thickness_texture_transform,
        ),
        iridescence_thickness_uv_transform_1: texture_transform_uniform_1(
            material.iridescence_thickness_texture_transform,
        ),
        thickness_uv_transform_0: texture_transform_uniform_0(material.thickness_texture_transform),
        thickness_uv_transform_1: texture_transform_uniform_1(material.thickness_texture_transform),
    }
}

fn texture_transform_uniform_0(transform: TextureTransform) -> [f32; 4] {
    let (sin, cos) = transform.rotation.sin_cos();
    [
        transform.scale[0] * cos,
        -transform.scale[1] * sin,
        transform.scale[0] * sin,
        transform.scale[1] * cos,
    ]
}

fn texture_transform_uniform_1(transform: TextureTransform) -> [f32; 4] {
    [
        transform.offset[0],
        transform.offset[1],
        transform.tex_coord.min(1) as f32,
        0.0,
    ]
}

#[derive(Clone, Copy)]
pub struct MeshDraw<'a> {
    pub mesh: &'a WgpuMesh,
    pub instance: &'a WgpuMeshInstance,
    pub material: &'a WgpuMaterial,
}

impl<'a> MeshDraw<'a> {
    pub const fn new(
        mesh: &'a WgpuMesh,
        instance: &'a WgpuMeshInstance,
        material: &'a WgpuMaterial,
    ) -> Self {
        Self {
            mesh,
            instance,
            material,
        }
    }
}

pub struct MeshBatchDraw<'a> {
    pub mesh: &'a WgpuMesh,
    pub material: &'a WgpuMaterial,
    pub instances: Vec<&'a WgpuMeshInstance>,
}

impl<'a> MeshBatchDraw<'a> {
    pub fn new(
        mesh: &'a WgpuMesh,
        material: &'a WgpuMaterial,
        instances: Vec<&'a WgpuMeshInstance>,
    ) -> Self {
        Self {
            mesh,
            material,
            instances,
        }
    }
}

pub const MAX_NATIVE_PASS_LABELS: usize = 128;
pub const GBUFFER_COLOR_ATTACHMENT_COUNT: usize = 3;
const GBUFFER_ALBEDO_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const GBUFFER_NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const GBUFFER_MATERIAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRenderStats {
    pub batch_count: usize,
    pub draw_call_count: usize,
    pub native_pass_label_count: usize,
    pub native_pass_labels_dropped: usize,
    pub native_pass_labels: [Option<String>; MAX_NATIVE_PASS_LABELS],
    pub mesh_pass_draw_call_count: usize,
    pub skybox_draw_call_count: usize,
    pub gbuffer_draw_call_count: usize,
    pub deferred_lighting_draw_call_count: usize,
    pub depth_prepass_draw_call_count: usize,
    pub shadow_draw_call_count: usize,
    pub directional_shadow_draw_call_count: usize,
    pub spot_shadow_draw_call_count: usize,
    pub point_shadow_draw_call_count: usize,
    pub opaque_draw_call_count: usize,
    pub transparent_draw_call_count: usize,
    pub post_process_draw_call_count: usize,
    pub instance_count: usize,
    pub instance_buffer_capacity: usize,
    pub timestamp_writes: usize,
    pub gpu_time_ns: Option<u64>,
}

impl Default for MeshRenderStats {
    fn default() -> Self {
        Self {
            batch_count: 0,
            draw_call_count: 0,
            native_pass_label_count: 0,
            native_pass_labels_dropped: 0,
            native_pass_labels: std::array::from_fn(|_| None),
            mesh_pass_draw_call_count: 0,
            skybox_draw_call_count: 0,
            gbuffer_draw_call_count: 0,
            deferred_lighting_draw_call_count: 0,
            depth_prepass_draw_call_count: 0,
            shadow_draw_call_count: 0,
            directional_shadow_draw_call_count: 0,
            spot_shadow_draw_call_count: 0,
            point_shadow_draw_call_count: 0,
            opaque_draw_call_count: 0,
            transparent_draw_call_count: 0,
            post_process_draw_call_count: 0,
            instance_count: 0,
            instance_buffer_capacity: 0,
            timestamp_writes: 0,
            gpu_time_ns: None,
        }
    }
}

impl MeshRenderStats {
    pub const fn native_pass_label_capacity() -> usize {
        MAX_NATIVE_PASS_LABELS
    }

    pub fn record_native_pass_label(&mut self, label: impl Into<String>) {
        if self.native_pass_label_count >= MAX_NATIVE_PASS_LABELS {
            self.native_pass_labels_dropped = self.native_pass_labels_dropped.saturating_add(1);
            return;
        }
        self.native_pass_labels[self.native_pass_label_count] = Some(label.into());
        self.native_pass_label_count += 1;
    }

    pub fn native_pass_label_strings(&self) -> Vec<String> {
        self.native_pass_labels
            .iter()
            .take(self.native_pass_label_count)
            .filter_map(Clone::clone)
            .collect()
    }

    pub const fn instance_buffer_bytes(&self) -> usize {
        self.instance_buffer_capacity * std::mem::size_of::<InstanceRaw>()
    }

    pub fn gpu_time_ms(&self) -> Option<f32> {
        self.gpu_time_ns.map(|time_ns| time_ns as f32 / 1_000_000.0)
    }
}

pub struct MeshRenderer {
    skybox_color_pipeline: wgpu::RenderPipeline,
    skybox_depth_pipeline: wgpu::RenderPipeline,
    post_process_color_pipeline: wgpu::RenderPipeline,
    sampled_post_process_pipeline: wgpu::RenderPipeline,
    sampled_post_process_bind_group_layout: wgpu::BindGroupLayout,
    sampled_post_process_sampler: wgpu::Sampler,
    sampled_post_process_uniform_buffer: wgpu::Buffer,
    post_process_options: WgpuPostProcessOptions,
    gbuffer_color_pipeline: wgpu::RenderPipeline,
    gbuffer_depth_pipeline: wgpu::RenderPipeline,
    double_sided_gbuffer_color_pipeline: wgpu::RenderPipeline,
    double_sided_gbuffer_depth_pipeline: wgpu::RenderPipeline,
    deferred_lighting_pipeline: wgpu::RenderPipeline,
    deferred_lighting_bind_group_layout: wgpu::BindGroupLayout,
    deferred_lighting_sampler: wgpu::Sampler,
    shadow_pipeline: wgpu::RenderPipeline,
    double_sided_shadow_pipeline: wgpu::RenderPipeline,
    depth_prepass_pipeline: wgpu::RenderPipeline,
    double_sided_depth_prepass_pipeline: wgpu::RenderPipeline,
    opaque_color_pipeline: wgpu::RenderPipeline,
    opaque_depth_pipeline: wgpu::RenderPipeline,
    opaque_depth_read_pipeline: wgpu::RenderPipeline,
    alpha_blend_color_pipeline: wgpu::RenderPipeline,
    alpha_blend_depth_pipeline: wgpu::RenderPipeline,
    alpha_blend_depth_write_pipeline: wgpu::RenderPipeline,
    double_sided_opaque_color_pipeline: wgpu::RenderPipeline,
    double_sided_opaque_depth_pipeline: wgpu::RenderPipeline,
    double_sided_opaque_depth_read_pipeline: wgpu::RenderPipeline,
    double_sided_alpha_blend_color_pipeline: wgpu::RenderPipeline,
    double_sided_alpha_blend_depth_pipeline: wgpu::RenderPipeline,
    double_sided_alpha_blend_depth_write_pipeline: wgpu::RenderPipeline,
    single_sample_opaque_depth_pipeline: wgpu::RenderPipeline,
    single_sample_opaque_depth_read_pipeline: wgpu::RenderPipeline,
    single_sample_alpha_blend_depth_pipeline: wgpu::RenderPipeline,
    single_sample_alpha_blend_depth_write_pipeline: wgpu::RenderPipeline,
    single_sample_double_sided_opaque_depth_pipeline: wgpu::RenderPipeline,
    single_sample_double_sided_opaque_depth_read_pipeline: wgpu::RenderPipeline,
    single_sample_double_sided_alpha_blend_depth_pipeline: wgpu::RenderPipeline,
    single_sample_double_sided_alpha_blend_depth_write_pipeline: wgpu::RenderPipeline,
    material_bind_group_layout: wgpu::BindGroupLayout,
    render_bind_group_layout: wgpu::BindGroupLayout,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    render_bind_group: wgpu::BindGroup,
    render_environment_mip_level_count: u32,
    directional_shadow_uniform_bind_groups: Vec<wgpu::BindGroup>,
    spot_shadow_uniform_bind_groups: Vec<wgpu::BindGroup>,
    point_shadow_uniform_bind_groups: Vec<wgpu::BindGroup>,
    render_uniform_buffer: wgpu::Buffer,
    skybox_uniform_buffer: wgpu::Buffer,
    skybox_bind_group: wgpu::BindGroup,
    directional_shadow_uniform_buffers: Vec<wgpu::Buffer>,
    spot_shadow_uniform_buffers: Vec<wgpu::Buffer>,
    point_shadow_uniform_buffers: Vec<wgpu::Buffer>,
    default_environment_texture: WgpuEnvironmentTexture,
    brdf_lut_texture: WgpuTexture,
    shadow_sampler: wgpu::Sampler,
    shadow_resources: ShadowResources,
    instance_buffer: Option<wgpu::Buffer>,
    instance_buffer_capacity: usize,
    last_stats: MeshRenderStats,
    gpu_profiling_enabled: bool,
    clear_color: Color,
    depth: RenderDepthDesc,
    lighting: RenderLighting,
    camera_position: [f32; 3],
    view_projection: Mat4,
    shadow_camera: Camera,
    shadow_camera_aspect_ratio: f32,
    sample_count: u32,
    surface_format: wgpu::TextureFormat,
}

impl MeshRenderer {
    pub fn new(
        graphics: &WgpuGraphics,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> GraphicsResult<Self> {
        Self::new_with_sample_count(graphics, surface_format, depth_format, DEFAULT_SAMPLE_COUNT)
    }

    pub fn new_with_sample_count(
        graphics: &WgpuGraphics,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> GraphicsResult<Self> {
        let sample_count = validate_sample_count(sample_count)?;
        let device = graphics.device();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mesh.wgsl").into()),
        });
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo Shadow Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
        let skybox_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo Skybox Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("skybox.wgsl").into()),
        });
        let post_process_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo Post Process Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post_process.wgsl").into()),
        });
        let sampled_post_process_shader =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Neo Sampled Post Process Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("post_process_sampled.wgsl").into()),
            });
        let gbuffer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo GBuffer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gbuffer.wgsl").into()),
        });
        let deferred_lighting_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo Deferred Lighting Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("deferred_lighting.wgsl").into()),
        });
        let material_bind_group_layout_entries = material_bind_group_layout_entries();
        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Material Bind Group Layout"),
                entries: &material_bind_group_layout_entries,
            });
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Render Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let shadow_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Shadow Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Shadow Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let deferred_lighting_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Deferred Lighting Bind Group Layout"),
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sampled_post_process_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Sampled Post Process Bind Group Layout"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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
        let skybox_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Skybox Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let render_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Render Uniform Buffer"),
            size: std::mem::size_of::<RenderUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skybox_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Skybox Uniform Buffer"),
            size: std::mem::size_of::<SkyboxUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skybox_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Neo Skybox Bind Group"),
            layout: &skybox_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: skybox_uniform_buffer.as_entire_binding(),
            }],
        });
        let directional_shadow_uniform_buffers = (0..MAX_DIRECTIONAL_SHADOW_CASCADES)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Neo Directional Shadow Uniform Buffer"),
                    size: std::mem::size_of::<ShadowUniform>() as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect::<Vec<_>>();
        let spot_shadow_uniform_buffers = (0..MAX_SPOT_LIGHTS)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Neo Spot Shadow Uniform Buffer"),
                    size: std::mem::size_of::<ShadowUniform>() as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect::<Vec<_>>();
        let point_shadow_uniform_buffers = (0..MAX_POINT_SHADOW_FACES)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Neo Point Shadow Uniform Buffer"),
                    size: std::mem::size_of::<ShadowUniform>() as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect::<Vec<_>>();
        let default_environment_texture =
            WgpuTexture::from_environment_texture(graphics, &Texture::white_1x1())?;
        let render_environment_mip_level_count = default_environment_texture.mip_level_count();
        let brdf_lut_texture = WgpuTexture::brdf_lut(graphics, BRDF_LUT_SIZE)?;
        let render_bind_group = create_render_bind_group(
            device,
            &render_bind_group_layout,
            &render_uniform_buffer,
            &default_environment_texture,
            &brdf_lut_texture,
            &[],
        );
        let directional_shadow_uniform_bind_groups = directional_shadow_uniform_buffers
            .iter()
            .map(|buffer| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Neo Directional Shadow Uniform Bind Group"),
                    layout: &shadow_uniform_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            })
            .collect::<Vec<_>>();
        let spot_shadow_uniform_bind_groups = spot_shadow_uniform_buffers
            .iter()
            .map(|buffer| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Neo Spot Shadow Uniform Bind Group"),
                    layout: &shadow_uniform_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            })
            .collect::<Vec<_>>();
        let point_shadow_uniform_bind_groups = point_shadow_uniform_buffers
            .iter()
            .map(|buffer| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Neo Point Shadow Uniform Bind Group"),
                    layout: &shadow_uniform_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            })
            .collect::<Vec<_>>();
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Neo Shadow Comparison Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..wgpu::SamplerDescriptor::default()
        });
        let deferred_lighting_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Neo Deferred Lighting Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let sampled_post_process_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Neo Sampled Post Process Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let sampled_post_process_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Sampled Post Process Uniform Buffer"),
            size: std::mem::size_of::<SampledPostProcessUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_resources = ShadowResources::new(
            device,
            &shadow_bind_group_layout,
            &shadow_sampler,
            DirectionalShadow::DISABLED.map_size,
            1,
            1,
        );
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Neo Mesh Pipeline Layout"),
            bind_group_layouts: &[
                &material_bind_group_layout,
                &render_bind_group_layout,
                &shadow_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Neo Shadow Pipeline Layout"),
            bind_group_layouts: &[&shadow_uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let skybox_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Neo Skybox Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout, &skybox_bind_group_layout],
            push_constant_ranges: &[],
        });
        let post_process_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Neo Post Process Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let deferred_lighting_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Neo Deferred Lighting Pipeline Layout"),
                bind_group_layouts: &[&deferred_lighting_bind_group_layout],
                push_constant_ranges: &[],
            });
        let sampled_post_process_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Neo Sampled Post Process Pipeline Layout"),
                bind_group_layouts: &[&sampled_post_process_bind_group_layout],
                push_constant_ranges: &[],
            });
        let skybox_color_pipeline = create_skybox_pipeline(
            device,
            &skybox_shader,
            &skybox_layout,
            surface_format,
            sample_count,
            None,
        );
        let skybox_depth_pipeline = create_skybox_pipeline(
            device,
            &skybox_shader,
            &skybox_layout,
            surface_format,
            sample_count,
            Some(depth_format),
        );
        let post_process_color_pipeline = create_post_process_pipeline(
            device,
            &post_process_shader,
            &post_process_layout,
            surface_format,
            sample_count,
            Some(depth_format),
        );
        let sampled_post_process_pipeline = create_sampled_post_process_pipeline(
            device,
            &sampled_post_process_shader,
            &sampled_post_process_layout,
            surface_format,
            sample_count,
            Some(depth_format),
        );
        let gbuffer_color_pipeline = create_mesh_gbuffer_pipeline(
            device,
            &gbuffer_shader,
            &layout,
            sample_count,
            None,
            false,
        );
        let gbuffer_depth_pipeline = create_mesh_gbuffer_pipeline(
            device,
            &gbuffer_shader,
            &layout,
            sample_count,
            Some(depth_format),
            false,
        );
        let double_sided_gbuffer_color_pipeline = create_mesh_gbuffer_pipeline(
            device,
            &gbuffer_shader,
            &layout,
            sample_count,
            None,
            true,
        );
        let double_sided_gbuffer_depth_pipeline = create_mesh_gbuffer_pipeline(
            device,
            &gbuffer_shader,
            &layout,
            sample_count,
            Some(depth_format),
            true,
        );
        let deferred_lighting_pipeline = create_deferred_lighting_pipeline(
            device,
            &deferred_lighting_shader,
            &deferred_lighting_layout,
            surface_format,
        );
        let shadow_pipeline = create_shadow_pipeline(device, &shadow_shader, &shadow_layout, false);
        let double_sided_shadow_pipeline =
            create_shadow_pipeline(device, &shadow_shader, &shadow_layout, true);
        let depth_prepass_pipeline = create_mesh_depth_prepass_pipeline(
            device,
            &shader,
            &layout,
            sample_count,
            depth_format,
            false,
        );
        let double_sided_depth_prepass_pipeline = create_mesh_depth_prepass_pipeline(
            device,
            &shader,
            &layout,
            sample_count,
            depth_format,
            true,
        );
        let opaque_color_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Opaque Mesh Color Pipeline",
            BlendMode::Opaque,
            None,
            false,
            false,
        );
        let opaque_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Opaque Mesh Depth Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            true,
            false,
        );
        let opaque_depth_read_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Opaque Mesh Depth Read Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            false,
            false,
        );
        let alpha_blend_color_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Alpha Blend Mesh Color Pipeline",
            BlendMode::AlphaBlend,
            None,
            false,
            false,
        );
        let alpha_blend_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Alpha Blend Mesh Depth Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            false,
            false,
        );
        let alpha_blend_depth_write_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Alpha Blend Mesh Depth Write Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            true,
            false,
        );
        let double_sided_opaque_color_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Double-Sided Opaque Mesh Color Pipeline",
            BlendMode::Opaque,
            None,
            false,
            true,
        );
        let double_sided_opaque_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Double-Sided Opaque Mesh Depth Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            true,
            true,
        );
        let double_sided_opaque_depth_read_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Double-Sided Opaque Mesh Depth Read Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            false,
            true,
        );
        let double_sided_alpha_blend_color_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Double-Sided Alpha Blend Mesh Color Pipeline",
            BlendMode::AlphaBlend,
            None,
            false,
            true,
        );
        let double_sided_alpha_blend_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Double-Sided Alpha Blend Mesh Depth Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            false,
            true,
        );
        let double_sided_alpha_blend_depth_write_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            sample_count,
            "Neo Double-Sided Alpha Blend Mesh Depth Write Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            true,
            true,
        );
        let single_sample_opaque_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Opaque Mesh Depth Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            true,
            false,
        );
        let single_sample_opaque_depth_read_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Opaque Mesh Depth Read Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            false,
            false,
        );
        let single_sample_alpha_blend_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Alpha Blend Mesh Depth Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            false,
            false,
        );
        let single_sample_alpha_blend_depth_write_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Alpha Blend Mesh Depth Write Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            true,
            false,
        );
        let single_sample_double_sided_opaque_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Double-Sided Opaque Mesh Depth Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            true,
            true,
        );
        let single_sample_double_sided_opaque_depth_read_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Double-Sided Opaque Mesh Depth Read Pipeline",
            BlendMode::Opaque,
            Some(depth_format),
            false,
            true,
        );
        let single_sample_double_sided_alpha_blend_depth_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Double-Sided Alpha Blend Mesh Depth Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            false,
            true,
        );
        let single_sample_double_sided_alpha_blend_depth_write_pipeline = create_mesh_pipeline(
            device,
            &shader,
            &layout,
            surface_format,
            DEFAULT_SAMPLE_COUNT,
            "Neo Single-Sample Double-Sided Alpha Blend Mesh Depth Write Pipeline",
            BlendMode::AlphaBlend,
            Some(depth_format),
            true,
            true,
        );

        Ok(Self {
            skybox_color_pipeline,
            skybox_depth_pipeline,
            post_process_color_pipeline,
            sampled_post_process_pipeline,
            sampled_post_process_bind_group_layout,
            sampled_post_process_sampler,
            sampled_post_process_uniform_buffer,
            post_process_options: WgpuPostProcessOptions::default(),
            gbuffer_color_pipeline,
            gbuffer_depth_pipeline,
            double_sided_gbuffer_color_pipeline,
            double_sided_gbuffer_depth_pipeline,
            deferred_lighting_pipeline,
            deferred_lighting_bind_group_layout,
            deferred_lighting_sampler,
            shadow_pipeline,
            double_sided_shadow_pipeline,
            depth_prepass_pipeline,
            double_sided_depth_prepass_pipeline,
            opaque_color_pipeline,
            opaque_depth_pipeline,
            opaque_depth_read_pipeline,
            alpha_blend_color_pipeline,
            alpha_blend_depth_pipeline,
            alpha_blend_depth_write_pipeline,
            double_sided_opaque_color_pipeline,
            double_sided_opaque_depth_pipeline,
            double_sided_opaque_depth_read_pipeline,
            double_sided_alpha_blend_color_pipeline,
            double_sided_alpha_blend_depth_pipeline,
            double_sided_alpha_blend_depth_write_pipeline,
            single_sample_opaque_depth_pipeline,
            single_sample_opaque_depth_read_pipeline,
            single_sample_alpha_blend_depth_pipeline,
            single_sample_alpha_blend_depth_write_pipeline,
            single_sample_double_sided_opaque_depth_pipeline,
            single_sample_double_sided_opaque_depth_read_pipeline,
            single_sample_double_sided_alpha_blend_depth_pipeline,
            single_sample_double_sided_alpha_blend_depth_write_pipeline,
            material_bind_group_layout,
            render_bind_group_layout,
            shadow_bind_group_layout,
            render_bind_group,
            render_environment_mip_level_count,
            directional_shadow_uniform_bind_groups,
            spot_shadow_uniform_bind_groups,
            point_shadow_uniform_bind_groups,
            render_uniform_buffer,
            skybox_uniform_buffer,
            skybox_bind_group,
            directional_shadow_uniform_buffers,
            spot_shadow_uniform_buffers,
            point_shadow_uniform_buffers,
            default_environment_texture,
            brdf_lut_texture,
            shadow_sampler,
            shadow_resources,
            instance_buffer: None,
            instance_buffer_capacity: 0,
            last_stats: MeshRenderStats::default(),
            gpu_profiling_enabled: false,
            clear_color: Color::rgb(0.05, 0.09, 0.13),
            depth: RenderDepthDesc::default(),
            lighting: RenderLighting::default(),
            camera_position: [0.0, 0.0, 0.0],
            view_projection: Mat4::IDENTITY,
            shadow_camera: Camera::default(),
            shadow_camera_aspect_ratio: 1.0,
            sample_count,
            surface_format,
        })
    }

    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    pub fn set_clear_color(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
    }

    pub fn depth(&self) -> RenderDepthDesc {
        self.depth
    }

    pub fn set_depth(&mut self, depth: RenderDepthDesc) {
        self.depth = depth;
    }

    pub fn lighting(&self) -> RenderLighting {
        self.lighting
    }

    pub fn set_lighting(&mut self, lighting: RenderLighting) {
        self.lighting = lighting;
    }

    pub fn set_environment_texture(
        &mut self,
        graphics: &WgpuGraphics,
        environment_texture: Option<&WgpuEnvironmentTexture>,
    ) {
        let environment_texture = environment_texture.unwrap_or(&self.default_environment_texture);
        self.render_bind_group = create_render_bind_group(
            graphics.device(),
            &self.render_bind_group_layout,
            &self.render_uniform_buffer,
            environment_texture,
            &self.brdf_lut_texture,
            &[],
        );
        self.render_environment_mip_level_count = environment_texture.mip_level_count();
    }

    pub fn camera_position(&self) -> [f32; 3] {
        self.camera_position
    }

    pub fn set_camera_position(&mut self, camera_position: [f32; 3]) {
        self.camera_position = camera_position;
    }

    pub fn set_shadow_camera(&mut self, camera: Camera, aspect_ratio: f32) {
        self.shadow_camera = camera;
        self.shadow_camera_aspect_ratio = aspect_ratio.max(0.0001);
        self.view_projection = camera.view_projection(self.shadow_camera_aspect_ratio);
    }

    pub fn last_stats(&self) -> MeshRenderStats {
        self.last_stats.clone()
    }

    pub fn set_gpu_profiling_enabled(&mut self, enabled: bool) {
        self.gpu_profiling_enabled = enabled;
    }

    pub fn set_post_process_options(&mut self, options: WgpuPostProcessOptions) {
        self.post_process_options = options;
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn material_layout_info(&self) -> WgpuMaterialLayoutInfo {
        wgpu_material_layout_info()
    }

    pub fn create_instance(
        &self,
        _graphics: &WgpuGraphics,
        model_view_projection: Mat4,
    ) -> WgpuMeshInstance {
        WgpuMeshInstance::new(model_view_projection)
    }

    pub fn create_material(
        &self,
        graphics: &WgpuGraphics,
        material: Material,
        base_color_texture: &WgpuTexture,
        metallic_roughness_texture: &WgpuTexture,
        normal_texture: &WgpuTexture,
        emissive_texture: &WgpuTexture,
        occlusion_texture: &WgpuTexture,
        clearcoat_texture: &WgpuTexture,
        clearcoat_roughness_texture: &WgpuTexture,
        clearcoat_normal_texture: &WgpuTexture,
        sheen_color_texture: &WgpuTexture,
        sheen_roughness_texture: &WgpuTexture,
        transmission_texture: &WgpuTexture,
        specular_texture: &WgpuTexture,
        specular_color_texture: &WgpuTexture,
        anisotropy_texture: &WgpuTexture,
        optical_extension_texture: &WgpuTexture,
    ) -> WgpuMaterial {
        WgpuMaterial::new(
            graphics,
            &self.material_bind_group_layout,
            material,
            base_color_texture,
            metallic_roughness_texture,
            normal_texture,
            emissive_texture,
            occlusion_texture,
            clearcoat_texture,
            clearcoat_roughness_texture,
            clearcoat_normal_texture,
            sheen_color_texture,
            sheen_roughness_texture,
            transmission_texture,
            specular_texture,
            specular_color_texture,
            anisotropy_texture,
            optical_extension_texture,
        )
    }

    pub fn render(
        &mut self,
        surface: &mut WgpuSurface,
        draws: &[MeshDraw<'_>],
    ) -> GraphicsResult<()> {
        let batches = draws
            .iter()
            .map(|draw| MeshBatchDraw::new(draw.mesh, draw.material, vec![draw.instance]))
            .collect::<Vec<_>>();
        self.render_batches(surface, &batches)
    }

    pub fn render_batches(
        &mut self,
        surface: &mut WgpuSurface,
        batches: &[MeshBatchDraw<'_>],
    ) -> GraphicsResult<()> {
        self.render_batches_with_environment(surface, batches, None)
    }

    pub fn capture_environment_probe(
        &mut self,
        graphics: &WgpuGraphics,
        probe: &mut WgpuEnvironmentProbe,
        desc: EnvironmentProbeDesc,
        batches: &[MeshBatchDraw<'_>],
    ) -> GraphicsResult<()> {
        let mut source_instances = Vec::new();
        let mut draws = Vec::with_capacity(batches.len());
        for batch in batches {
            let instance_start = u32::try_from(source_instances.len()).map_err(|_| {
                GraphicsError::InvalidResource(
                    "mesh renderer has more than u32::MAX queued probe instances".to_owned(),
                )
            })?;
            let instance_count = u32::try_from(batch.instances.len()).map_err(|_| {
                GraphicsError::InvalidResource(
                    "mesh probe batch has more than u32::MAX instances".to_owned(),
                )
            })?;

            source_instances.extend(batch.instances.iter().map(|instance| instance.raw()));
            draws.push((batch, instance_start, instance_count));
        }

        let lighting = self.lighting;
        let render_bind_group = create_render_bind_group(
            graphics.device(),
            &self.render_bind_group_layout,
            &self.render_uniform_buffer,
            &self.default_environment_texture,
            &self.brdf_lut_texture,
            &[],
        );
        let spot_shadow_view_projections = [Mat4::IDENTITY; MAX_SPOT_LIGHTS];
        let spot_shadow_options = [[0.0; 4]; MAX_SPOT_LIGHTS];
        let point_shadow_view_projections = [Mat4::IDENTITY; MAX_POINT_SHADOW_FACES];
        let point_shadow_options = [[0.0; 4]; MAX_POINT_LIGHTS];
        let near = desc.near.max(0.0001);
        let far = desc.far.max(near + 0.0001);

        for face in 0..crate::probe::CUBE_FACE_COUNT {
            let Some(color_view) = probe.capture_face_view(face) else {
                continue;
            };
            let Some(depth_view) = probe.depth_face_view(face) else {
                continue;
            };
            let (direction, up) = point_shadow_face_direction_and_up(face);
            let view_projection = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, near, far)
                * look_to_rh_with_up(desc.position, direction, up);
            let probe_instances = source_instances
                .iter()
                .map(|instance| {
                    let model = Mat4::from_cols_array(instance.model);
                    let normal_matrix = Mat4::from_cols_array([
                        instance.normal_matrix[0],
                        instance.normal_matrix[1],
                        instance.normal_matrix[2],
                        [0.0, 0.0, 0.0, 1.0],
                    ]);
                    InstanceRaw::from_matrices(view_projection * model, normal_matrix, model)
                })
                .collect::<Vec<_>>();

            graphics.queue().write_buffer(
                &self.render_uniform_buffer,
                0,
                bytemuck::bytes_of(&RenderUniform::from_lighting(
                    view_projection,
                    lighting,
                    desc.position,
                    DirectionalShadowCascades::disabled(direction),
                    spot_shadow_view_projections,
                    spot_shadow_options,
                    point_shadow_view_projections,
                    point_shadow_options,
                    self.default_environment_texture.mip_level_count(),
                    EnvironmentProbeUniforms::EMPTY,
                )),
            );

            if !probe_instances.is_empty() {
                self.ensure_instance_capacity(graphics.device(), probe_instances.len());
                if let Some(instance_buffer) = &self.instance_buffer {
                    graphics.queue().write_buffer(
                        instance_buffer,
                        0,
                        bytemuck::cast_slice(&probe_instances),
                    );
                }
            }

            let mut encoder =
                graphics
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Neo Environment Probe Capture Encoder"),
                    });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Neo Environment Probe Capture Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu_color(desc.clear_color)),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                if let Some(instance_buffer) = &self.instance_buffer {
                    pass.set_vertex_buffer(1, instance_buffer.slice(..));
                }
                pass.set_bind_group(1, &render_bind_group, &[]);
                pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);

                for (batch, instance_start, instance_count) in draws.iter().copied() {
                    if instance_count == 0 {
                        continue;
                    }

                    pass.set_pipeline(self.single_sample_depth_pipeline_for(
                        batch.material.blend_mode(),
                        batch.material.depth_write(),
                        batch.material.double_sided(),
                    ));
                    pass.set_bind_group(0, &batch.material.material_bind_group, &[]);
                    pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                    let instance_range = instance_start..instance_start + instance_count;

                    if let Some(index_buffer) = batch.mesh.index_buffer() {
                        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..batch.mesh.index_count(), 0, instance_range);
                    } else {
                        pass.draw(0..batch.mesh.vertex_count(), instance_range);
                    }
                }
            }
            graphics.queue().submit(Some(encoder.finish()));
        }

        probe.prefilter(graphics)
    }

    pub fn render_batches_with_environment(
        &mut self,
        surface: &mut WgpuSurface,
        batches: &[MeshBatchDraw<'_>],
        environment_texture: Option<&WgpuEnvironmentTexture>,
    ) -> GraphicsResult<()> {
        self.render_batches_with_environment_probes(surface, batches, environment_texture, &[])
    }

    pub fn render_batches_with_environment_probes(
        &mut self,
        surface: &mut WgpuSurface,
        batches: &[MeshBatchDraw<'_>],
        environment_texture: Option<&WgpuEnvironmentTexture>,
        environment_probes: &[EnvironmentProbeBlend<'_>],
    ) -> GraphicsResult<()> {
        self.render_batches_with_environment_probes_and_post_pass(
            surface,
            batches,
            environment_texture,
            environment_probes,
            |_| {},
        )
    }

    pub fn render_batches_with_environment_probes_and_post_pass<F>(
        &mut self,
        surface: &mut WgpuSurface,
        batches: &[MeshBatchDraw<'_>],
        environment_texture: Option<&WgpuEnvironmentTexture>,
        environment_probes: &[EnvironmentProbeBlend<'_>],
        mut post_pass: F,
    ) -> GraphicsResult<()>
    where
        F: FnMut(&mut wgpu::RenderPass<'_>),
    {
        if self.sample_count != surface.sample_count() {
            return Err(GraphicsError::InvalidResource(format!(
                "mesh renderer sample count {} does not match surface sample count {}; create the renderer with MeshRenderer::new_with_sample_count",
                self.sample_count,
                surface.sample_count()
            )));
        }

        let mut instances = Vec::new();
        let mut draws = Vec::with_capacity(batches.len());
        let mut draw_call_count = 0_usize;
        let mut opaque_draw_call_count = 0_usize;
        let mut transparent_draw_call_count = 0_usize;
        for batch in batches {
            let instance_start = u32::try_from(instances.len()).map_err(|_| {
                GraphicsError::InvalidResource(
                    "mesh renderer has more than u32::MAX queued instances".to_owned(),
                )
            })?;
            let instance_count = u32::try_from(batch.instances.len()).map_err(|_| {
                GraphicsError::InvalidResource(
                    "mesh batch has more than u32::MAX instances".to_owned(),
                )
            })?;

            instances.extend(batch.instances.iter().map(|instance| instance.raw()));
            if instance_count > 0 {
                draw_call_count += 1;
                match batch.material.blend_mode() {
                    BlendMode::Opaque => opaque_draw_call_count += 1,
                    BlendMode::AlphaBlend => transparent_draw_call_count += 1,
                }
            }
            draws.push((batch, instance_start, instance_count));
        }

        let batch_count = batches.len();
        let instance_count = instances.len();
        let clear_color = self.clear_color;
        let depth = self.depth;
        let lighting = self.lighting;
        let camera_position = self.camera_position;
        let view_projection = self.view_projection;
        let shadow_camera = self.shadow_camera;
        let shadow_camera_aspect_ratio = self.shadow_camera_aspect_ratio;
        let skybox_background_intensity = lighting.environment.background_intensity.max(0.0);
        let environment_probe_textures = environment_probes
            .iter()
            .take(MAX_ENVIRONMENT_PROBE_BLEND)
            .map(|probe| probe.environment)
            .collect::<Vec<_>>();
        let active_environment_texture = environment_probe_textures
            .first()
            .copied()
            .or(environment_texture);
        let environment_mip_level_count = active_environment_texture.map_or(
            self.render_environment_mip_level_count,
            WgpuEnvironmentTexture::mip_level_count,
        );
        let skybox_environment_texture = environment_texture.or(active_environment_texture);
        let directional_shadow_cascades =
            directional_shadow_cascades(lighting, shadow_camera, shadow_camera_aspect_ratio);
        let spot_shadows = active_spot_shadows(lighting);
        let point_shadows = active_point_shadows(lighting);
        let mut spot_shadow_view_projections = [Mat4::IDENTITY; MAX_SPOT_LIGHTS];
        let mut spot_shadow_options = [[0.0; 4]; MAX_SPOT_LIGHTS];
        for (index, light) in &spot_shadows {
            spot_shadow_view_projections[*index] = spot_shadow_view_projection(*light);
            spot_shadow_options[*index] = [
                light.shadow.strength.clamp(0.0, 1.0),
                light.shadow.bias.max(0.0),
                1.0,
                0.0,
            ];
        }
        let mut point_shadow_view_projections = [Mat4::IDENTITY; MAX_POINT_SHADOW_FACES];
        let mut point_shadow_options = [[0.0; 4]; MAX_POINT_LIGHTS];
        for (index, light) in &point_shadows {
            for face in 0..POINT_SHADOW_FACE_COUNT {
                point_shadow_view_projections[*index * POINT_SHADOW_FACE_COUNT + face] =
                    point_shadow_view_projection(*light, face);
            }
            point_shadow_options[*index] = [
                light.shadow.strength.clamp(0.0, 1.0),
                light.shadow.bias.max(0.0),
                1.0,
                0.0,
            ];
        }
        let shadow_enabled = directional_shadow_cascades.count > 0 && !instances.is_empty();
        let spot_shadow_enabled = !spot_shadows.is_empty() && !instances.is_empty();
        let point_shadow_enabled = !point_shadows.is_empty() && !instances.is_empty();
        let directional_shadow_size = if shadow_enabled {
            lighting.directional_shadow.map_size
        } else {
            self.shadow_resources.size
        };
        let spot_shadow_size = spot_shadows
            .iter()
            .map(|(_, light)| light.shadow.map_size)
            .max()
            .unwrap_or(self.shadow_resources.spot_size);
        let point_shadow_size = point_shadows
            .iter()
            .map(|(_, light)| light.shadow.map_size)
            .max()
            .unwrap_or(self.shadow_resources.point_size);

        let mut frame_timer = None;
        let mut skybox_draw_call_count = 0_usize;
        let mut gbuffer_draw_call_count = 0_usize;
        let mut deferred_lighting_draw_call_count = 0_usize;
        let mut depth_prepass_draw_call_count = 0_usize;
        let mut directional_shadow_draw_call_count = 0_usize;
        let mut spot_shadow_draw_call_count = 0_usize;
        let mut point_shadow_draw_call_count = 0_usize;
        let mut post_process_draw_call_count = 0_usize;
        let mut native_pass_label_stats = MeshRenderStats::default();
        let surface_size = surface.size();
        let result = surface.render_frame("Neo Mesh Encoder", |frame| {
            let timer = if self.gpu_profiling_enabled {
                WgpuFrameTimestampReadback::new(frame.device)
            } else {
                None
            };
            if let Some(timer) = &timer {
                timer.write_start(frame.encoder);
            }

            if shadow_enabled || spot_shadow_enabled || point_shadow_enabled {
                self.ensure_shadow_resources(
                    frame.device,
                    directional_shadow_size,
                    spot_shadow_size,
                    point_shadow_size,
                );
            }
            let environment_render_bind_group = active_environment_texture.map(|texture| {
                create_render_bind_group(
                    frame.device,
                    &self.render_bind_group_layout,
                    &self.render_uniform_buffer,
                    texture,
                    &self.brdf_lut_texture,
                    environment_probe_textures.as_slice(),
                )
            });
            let skybox_render_bind_group = skybox_environment_texture.map(|texture| {
                create_render_bind_group(
                    frame.device,
                    &self.render_bind_group_layout,
                    &self.render_uniform_buffer,
                    texture,
                    &self.brdf_lut_texture,
                    &[],
                )
            });

            frame.queue.write_buffer(
                &self.render_uniform_buffer,
                0,
                bytemuck::bytes_of(&RenderUniform::from_lighting(
                    view_projection,
                    lighting,
                    camera_position,
                    directional_shadow_cascades,
                    spot_shadow_view_projections,
                    spot_shadow_options,
                    point_shadow_view_projections,
                    point_shadow_options,
                    environment_mip_level_count,
                    EnvironmentProbeUniforms::from_blend(environment_probes),
                )),
            );
            if skybox_background_intensity > 0.0 && skybox_render_bind_group.is_some() {
                frame.queue.write_buffer(
                    &self.skybox_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&skybox_uniform(
                        shadow_camera,
                        shadow_camera_aspect_ratio,
                        skybox_background_intensity,
                    )),
                );
            }
            for cascade_index in 0..directional_shadow_cascades.count {
                if let Some(buffer) = self.directional_shadow_uniform_buffers.get(cascade_index) {
                    frame.queue.write_buffer(
                        buffer,
                        0,
                        bytemuck::bytes_of(&ShadowUniform {
                            shadow_view_projection: directional_shadow_cascades.view_projections
                                [cascade_index]
                                .to_cols_array(),
                        }),
                    );
                }
            }
            for (index, _) in &spot_shadows {
                if let Some(buffer) = self.spot_shadow_uniform_buffers.get(*index) {
                    frame.queue.write_buffer(
                        buffer,
                        0,
                        bytemuck::bytes_of(&ShadowUniform {
                            shadow_view_projection: spot_shadow_view_projections[*index]
                                .to_cols_array(),
                        }),
                    );
                }
            }
            for (index, _) in &point_shadows {
                for face in 0..POINT_SHADOW_FACE_COUNT {
                    let layer = *index * POINT_SHADOW_FACE_COUNT + face;
                    if let Some(buffer) = self.point_shadow_uniform_buffers.get(layer) {
                        frame.queue.write_buffer(
                            buffer,
                            0,
                            bytemuck::bytes_of(&ShadowUniform {
                                shadow_view_projection: point_shadow_view_projections[layer]
                                    .to_cols_array(),
                            }),
                        );
                    }
                }
            }

            if !instances.is_empty() {
                self.ensure_instance_capacity(frame.device, instances.len());
                if let Some(instance_buffer) = &self.instance_buffer {
                    frame
                        .queue
                        .write_buffer(instance_buffer, 0, bytemuck::cast_slice(&instances));
                }
            }

            if shadow_enabled {
                if let Some(instance_buffer) = &self.instance_buffer {
                    for cascade_index in 0..directional_shadow_cascades.count {
                        let Some(shadow_view) = self
                            .shadow_resources
                            .directional_layer_views
                            .get(cascade_index)
                        else {
                            continue;
                        };
                        let Some(shadow_bind_group) = self
                            .directional_shadow_uniform_bind_groups
                            .get(cascade_index)
                        else {
                            continue;
                        };
                        native_pass_label_stats
                            .record_native_pass_label("Neo Directional Shadow Pass");
                        let mut shadow_pass =
                            frame
                                .encoder
                                .begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Neo Directional Shadow Pass"),
                                    color_attachments: &[],
                                    depth_stencil_attachment: Some(
                                        wgpu::RenderPassDepthStencilAttachment {
                                            view: shadow_view,
                                            depth_ops: Some(wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(1.0),
                                                store: wgpu::StoreOp::Store,
                                            }),
                                            stencil_ops: None,
                                        },
                                    ),
                                    occlusion_query_set: None,
                                    timestamp_writes: None,
                                });

                        shadow_pass.set_bind_group(0, shadow_bind_group, &[]);
                        shadow_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                        for (batch, instance_start, instance_count) in draws.iter().copied() {
                            if instance_count == 0 || !batch.material.depth_write() {
                                continue;
                            }

                            if batch.material.double_sided() {
                                shadow_pass.set_pipeline(&self.double_sided_shadow_pipeline);
                            } else {
                                shadow_pass.set_pipeline(&self.shadow_pipeline);
                            }
                            shadow_pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                            let instance_range = instance_start..instance_start + instance_count;

                            if let Some(index_buffer) = batch.mesh.index_buffer() {
                                shadow_pass.set_index_buffer(
                                    index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                shadow_pass.draw_indexed(
                                    0..batch.mesh.index_count(),
                                    0,
                                    instance_range,
                                );
                            } else {
                                shadow_pass.draw(0..batch.mesh.vertex_count(), instance_range);
                            }
                            directional_shadow_draw_call_count =
                                directional_shadow_draw_call_count.saturating_add(1);
                        }
                    }
                }
            }

            if spot_shadow_enabled {
                if let Some(instance_buffer) = &self.instance_buffer {
                    for (spot_index, _) in &spot_shadows {
                        let Some(spot_view) =
                            self.shadow_resources.spot_layer_views.get(*spot_index)
                        else {
                            continue;
                        };
                        let Some(spot_bind_group) =
                            self.spot_shadow_uniform_bind_groups.get(*spot_index)
                        else {
                            continue;
                        };
                        native_pass_label_stats.record_native_pass_label("Neo Spot Shadow Pass");
                        let mut shadow_pass =
                            frame
                                .encoder
                                .begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("Neo Spot Shadow Pass"),
                                    color_attachments: &[],
                                    depth_stencil_attachment: Some(
                                        wgpu::RenderPassDepthStencilAttachment {
                                            view: spot_view,
                                            depth_ops: Some(wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(1.0),
                                                store: wgpu::StoreOp::Store,
                                            }),
                                            stencil_ops: None,
                                        },
                                    ),
                                    occlusion_query_set: None,
                                    timestamp_writes: None,
                                });

                        shadow_pass.set_bind_group(0, spot_bind_group, &[]);
                        shadow_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                        for (batch, instance_start, instance_count) in draws.iter().copied() {
                            if instance_count == 0 || !batch.material.depth_write() {
                                continue;
                            }

                            if batch.material.double_sided() {
                                shadow_pass.set_pipeline(&self.double_sided_shadow_pipeline);
                            } else {
                                shadow_pass.set_pipeline(&self.shadow_pipeline);
                            }
                            shadow_pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                            let instance_range = instance_start..instance_start + instance_count;

                            if let Some(index_buffer) = batch.mesh.index_buffer() {
                                shadow_pass.set_index_buffer(
                                    index_buffer.slice(..),
                                    wgpu::IndexFormat::Uint32,
                                );
                                shadow_pass.draw_indexed(
                                    0..batch.mesh.index_count(),
                                    0,
                                    instance_range,
                                );
                            } else {
                                shadow_pass.draw(0..batch.mesh.vertex_count(), instance_range);
                            }
                            spot_shadow_draw_call_count =
                                spot_shadow_draw_call_count.saturating_add(1);
                        }
                    }
                }
            }

            if point_shadow_enabled {
                if let Some(instance_buffer) = &self.instance_buffer {
                    for (point_index, _) in &point_shadows {
                        for face in 0..POINT_SHADOW_FACE_COUNT {
                            let layer = *point_index * POINT_SHADOW_FACE_COUNT + face;
                            let Some(point_view) =
                                self.shadow_resources.point_layer_views.get(layer)
                            else {
                                continue;
                            };
                            let Some(point_bind_group) =
                                self.point_shadow_uniform_bind_groups.get(layer)
                            else {
                                continue;
                            };
                            native_pass_label_stats
                                .record_native_pass_label("Neo Point Shadow Pass");
                            let mut shadow_pass =
                                frame
                                    .encoder
                                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Neo Point Shadow Pass"),
                                        color_attachments: &[],
                                        depth_stencil_attachment: Some(
                                            wgpu::RenderPassDepthStencilAttachment {
                                                view: point_view,
                                                depth_ops: Some(wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(1.0),
                                                    store: wgpu::StoreOp::Store,
                                                }),
                                                stencil_ops: None,
                                            },
                                        ),
                                        occlusion_query_set: None,
                                        timestamp_writes: None,
                                    });

                            shadow_pass.set_bind_group(0, point_bind_group, &[]);
                            shadow_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                            for (batch, instance_start, instance_count) in draws.iter().copied() {
                                if instance_count == 0 || !batch.material.depth_write() {
                                    continue;
                                }

                                if batch.material.double_sided() {
                                    shadow_pass.set_pipeline(&self.double_sided_shadow_pipeline);
                                } else {
                                    shadow_pass.set_pipeline(&self.shadow_pipeline);
                                }
                                shadow_pass
                                    .set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                                let instance_range =
                                    instance_start..instance_start + instance_count;

                                if let Some(index_buffer) = batch.mesh.index_buffer() {
                                    shadow_pass.set_index_buffer(
                                        index_buffer.slice(..),
                                        wgpu::IndexFormat::Uint32,
                                    );
                                    shadow_pass.draw_indexed(
                                        0..batch.mesh.index_count(),
                                        0,
                                        instance_range,
                                    );
                                } else {
                                    shadow_pass.draw(0..batch.mesh.vertex_count(), instance_range);
                                }
                                point_shadow_draw_call_count =
                                    point_shadow_draw_call_count.saturating_add(1);
                            }
                        }
                    }
                }
            }

            let active_render_bind_group = environment_render_bind_group
                .as_ref()
                .unwrap_or(&self.render_bind_group);
            let depth_view = depth.enabled.then_some(frame.depth_view).flatten();
            let mut deferred_lighting_post_process_view = None;
            let mut deferred_lighting_post_process_texture = None;
            let mut depth_prepass_draws = 0_u32;
            if let (Some(depth_view), Some(instance_buffer)) = (depth_view, &self.instance_buffer) {
                native_pass_label_stats.record_native_pass_label("Neo Depth Prepass");
                let mut depth_pass = frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Neo Depth Prepass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(depth.clear_depth.clamp(0.0, 1.0)),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                depth_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                depth_pass.set_bind_group(1, active_render_bind_group, &[]);
                depth_pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);

                for (batch, instance_start, instance_count) in draws.iter().copied() {
                    if instance_count == 0
                        || !batch.material.depth_write()
                        || !matches!(batch.material.blend_mode(), BlendMode::Opaque)
                    {
                        continue;
                    }

                    if batch.material.double_sided() {
                        depth_pass.set_pipeline(&self.double_sided_depth_prepass_pipeline);
                    } else {
                        depth_pass.set_pipeline(&self.depth_prepass_pipeline);
                    }
                    depth_pass.set_bind_group(0, &batch.material.material_bind_group, &[]);
                    depth_pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                    let instance_range = instance_start..instance_start + instance_count;

                    if let Some(index_buffer) = batch.mesh.index_buffer() {
                        depth_pass
                            .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        depth_pass.draw_indexed(0..batch.mesh.index_count(), 0, instance_range);
                    } else {
                        depth_pass.draw(0..batch.mesh.vertex_count(), instance_range);
                    }
                    depth_prepass_draws = depth_prepass_draws.saturating_add(1);
                }
            }
            depth_prepass_draw_call_count = depth_prepass_draws as usize;

            if let Some(instance_buffer) = &self.instance_buffer {
                native_pass_label_stats.record_native_pass_label("Neo GBuffer Pass");
                let gbuffer_size = wgpu::Extent3d {
                    width: surface_size.width.max(1),
                    height: surface_size.height.max(1),
                    depth_or_array_layers: 1,
                };
                let create_gbuffer_texture =
                    |label: &'static str,
                     format: wgpu::TextureFormat,
                     sample_count: u32,
                     usage: wgpu::TextureUsages| {
                        frame.device.create_texture(&wgpu::TextureDescriptor {
                            label: Some(label),
                            size: gbuffer_size,
                            mip_level_count: 1,
                            sample_count,
                            dimension: wgpu::TextureDimension::D2,
                            format,
                            usage,
                            view_formats: &[],
                        })
                    };
                let gbuffer_texture_usage =
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
                let gbuffer_albedo_texture = create_gbuffer_texture(
                    "Neo GBuffer Albedo Texture",
                    GBUFFER_ALBEDO_FORMAT,
                    1,
                    gbuffer_texture_usage,
                );
                let gbuffer_normal_texture = create_gbuffer_texture(
                    "Neo GBuffer Normal Texture",
                    GBUFFER_NORMAL_FORMAT,
                    1,
                    gbuffer_texture_usage,
                );
                let gbuffer_material_texture = create_gbuffer_texture(
                    "Neo GBuffer Material Texture",
                    GBUFFER_MATERIAL_FORMAT,
                    1,
                    gbuffer_texture_usage,
                );
                let gbuffer_albedo_msaa_texture = (frame.sample_count > 1).then(|| {
                    create_gbuffer_texture(
                        "Neo GBuffer Albedo MSAA Texture",
                        GBUFFER_ALBEDO_FORMAT,
                        frame.sample_count,
                        wgpu::TextureUsages::RENDER_ATTACHMENT,
                    )
                });
                let gbuffer_normal_msaa_texture = (frame.sample_count > 1).then(|| {
                    create_gbuffer_texture(
                        "Neo GBuffer Normal MSAA Texture",
                        GBUFFER_NORMAL_FORMAT,
                        frame.sample_count,
                        wgpu::TextureUsages::RENDER_ATTACHMENT,
                    )
                });
                let gbuffer_material_msaa_texture = (frame.sample_count > 1).then(|| {
                    create_gbuffer_texture(
                        "Neo GBuffer Material MSAA Texture",
                        GBUFFER_MATERIAL_FORMAT,
                        frame.sample_count,
                        wgpu::TextureUsages::RENDER_ATTACHMENT,
                    )
                });
                let gbuffer_albedo_view =
                    gbuffer_albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let gbuffer_normal_view =
                    gbuffer_normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let gbuffer_material_view =
                    gbuffer_material_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let gbuffer_albedo_msaa_view = gbuffer_albedo_msaa_texture
                    .as_ref()
                    .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
                let gbuffer_normal_msaa_view = gbuffer_normal_msaa_texture
                    .as_ref()
                    .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
                let gbuffer_material_msaa_view = gbuffer_material_msaa_texture
                    .as_ref()
                    .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
                let gbuffer_albedo_attachment_view = gbuffer_albedo_msaa_view
                    .as_ref()
                    .unwrap_or(&gbuffer_albedo_view);
                let gbuffer_normal_attachment_view = gbuffer_normal_msaa_view
                    .as_ref()
                    .unwrap_or(&gbuffer_normal_view);
                let gbuffer_material_attachment_view = gbuffer_material_msaa_view
                    .as_ref()
                    .unwrap_or(&gbuffer_material_view);
                let gbuffer_color_attachments: [_; GBUFFER_COLOR_ATTACHMENT_COUNT] = [
                    Some(wgpu::RenderPassColorAttachment {
                        view: gbuffer_albedo_attachment_view,
                        resolve_target: gbuffer_albedo_msaa_view
                            .as_ref()
                            .map(|_| &gbuffer_albedo_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: store_op_for_resolve(gbuffer_albedo_msaa_view.as_ref()),
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: gbuffer_normal_attachment_view,
                        resolve_target: gbuffer_normal_msaa_view
                            .as_ref()
                            .map(|_| &gbuffer_normal_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.5,
                                g: 0.5,
                                b: 1.0,
                                a: 1.0,
                            }),
                            store: store_op_for_resolve(gbuffer_normal_msaa_view.as_ref()),
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: gbuffer_material_attachment_view,
                        resolve_target: gbuffer_material_msaa_view
                            .as_ref()
                            .map(|_| &gbuffer_material_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: store_op_for_resolve(gbuffer_material_msaa_view.as_ref()),
                        },
                    }),
                ];
                let gbuffer_depth_attachment =
                    depth_view.map(|view| wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });
                let has_gbuffer_depth_attachment = gbuffer_depth_attachment.is_some();
                let mut gbuffer_pass =
                    frame
                        .encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Neo GBuffer Pass"),
                            color_attachments: &gbuffer_color_attachments,
                            depth_stencil_attachment: gbuffer_depth_attachment,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                gbuffer_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                gbuffer_pass.set_bind_group(1, active_render_bind_group, &[]);
                gbuffer_pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);

                for (batch, instance_start, instance_count) in draws.iter().copied() {
                    if instance_count == 0
                        || !matches!(batch.material.blend_mode(), BlendMode::Opaque)
                    {
                        continue;
                    }

                    gbuffer_pass.set_pipeline(self.gbuffer_pipeline_for(
                        has_gbuffer_depth_attachment,
                        batch.material.double_sided(),
                    ));
                    gbuffer_pass.set_bind_group(0, &batch.material.material_bind_group, &[]);
                    gbuffer_pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                    let instance_range = instance_start..instance_start + instance_count;

                    if let Some(index_buffer) = batch.mesh.index_buffer() {
                        gbuffer_pass
                            .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        gbuffer_pass.draw_indexed(0..batch.mesh.index_count(), 0, instance_range);
                    } else {
                        gbuffer_pass.draw(0..batch.mesh.vertex_count(), instance_range);
                    }
                    gbuffer_draw_call_count = gbuffer_draw_call_count.saturating_add(1);
                }
                drop(gbuffer_pass);

                native_pass_label_stats.record_native_pass_label("Neo Deferred Lighting Pass");
                let deferred_lighting_bind_group =
                    frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Neo Deferred Lighting Bind Group"),
                        layout: &self.deferred_lighting_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&gbuffer_albedo_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&gbuffer_normal_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(
                                    &gbuffer_material_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::Sampler(
                                    &self.deferred_lighting_sampler,
                                ),
                            },
                        ],
                    });
                let deferred_lighting_texture =
                    frame.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Neo Deferred Lighting Texture"),
                        size: gbuffer_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: self.surface_format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                let deferred_lighting_view =
                    deferred_lighting_texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut deferred_lighting_pass =
                    frame
                        .encoder
                        .begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Neo Deferred Lighting Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &deferred_lighting_view,
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
                deferred_lighting_pass.set_pipeline(&self.deferred_lighting_pipeline);
                deferred_lighting_pass.set_bind_group(0, &deferred_lighting_bind_group, &[]);
                deferred_lighting_pass.draw(0..3, 0..1);
                drop(deferred_lighting_pass);
                deferred_lighting_post_process_view = Some(deferred_lighting_view);
                deferred_lighting_post_process_texture = Some(deferred_lighting_texture);
                deferred_lighting_draw_call_count =
                    deferred_lighting_draw_call_count.saturating_add(1);
            }

            let mut mesh_pass_draw_order = (0..draws.len()).collect::<Vec<_>>();
            mesh_pass_draw_order.sort_by(|left, right| {
                compare_mesh_pass_draw_order(draws[*left].0, draws[*right].0, camera_position)
            });

            let has_depth_attachment = depth_view.is_some();
            {
                native_pass_label_stats.record_native_pass_label("Neo Forward Opaque Pass");
                let depth_stencil_attachment =
                    depth_view.map(|view| wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: if depth_prepass_draws > 0 {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(depth.clear_depth.clamp(0.0, 1.0))
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });
                let mut pass = frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Neo Forward Opaque Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu_color(clear_color)),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                if let Some(instance_buffer) = &self.instance_buffer {
                    pass.set_vertex_buffer(1, instance_buffer.slice(..));
                }
                pass.set_bind_group(1, active_render_bind_group, &[]);
                pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);

                if skybox_background_intensity > 0.0 {
                    if let Some(skybox_render_bind_group) = &skybox_render_bind_group {
                        pass.set_pipeline(if has_depth_attachment {
                            &self.skybox_depth_pipeline
                        } else {
                            &self.skybox_color_pipeline
                        });
                        pass.set_bind_group(0, skybox_render_bind_group, &[]);
                        pass.set_bind_group(1, &self.skybox_bind_group, &[]);
                        pass.draw(0..3, 0..1);
                        skybox_draw_call_count = skybox_draw_call_count.saturating_add(1);
                        pass.set_bind_group(1, active_render_bind_group, &[]);
                        pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);
                    }
                }

                for index in mesh_pass_draw_order.iter().copied() {
                    let (batch, instance_start, instance_count) = draws[index];
                    if instance_count == 0
                        || !matches!(batch.material.blend_mode(), BlendMode::Opaque)
                    {
                        continue;
                    }

                    pass.set_pipeline(self.pipeline_for(
                        batch.material.blend_mode(),
                        batch.material.depth_write(),
                        has_depth_attachment,
                        batch.material.double_sided(),
                    ));
                    pass.set_bind_group(0, &batch.material.material_bind_group, &[]);
                    pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                    let instance_range = instance_start..instance_start + instance_count;

                    if let Some(index_buffer) = batch.mesh.index_buffer() {
                        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..batch.mesh.index_count(), 0, instance_range);
                    } else {
                        pass.draw(0..batch.mesh.vertex_count(), instance_range);
                    }
                }
            }

            let depth_stencil_attachment =
                depth_view.map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                });
            native_pass_label_stats.record_native_pass_label("Neo Transparent Pass");
            let mut pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Neo Transparent Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            if let Some(instance_buffer) = &self.instance_buffer {
                pass.set_vertex_buffer(1, instance_buffer.slice(..));
            }
            pass.set_bind_group(1, active_render_bind_group, &[]);
            pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);

            for index in mesh_pass_draw_order {
                let (batch, instance_start, instance_count) = draws[index];
                if instance_count == 0
                    || !matches!(batch.material.blend_mode(), BlendMode::AlphaBlend)
                {
                    continue;
                }

                pass.set_pipeline(self.pipeline_for(
                    batch.material.blend_mode(),
                    batch.material.depth_write(),
                    has_depth_attachment,
                    batch.material.double_sided(),
                ));
                pass.set_bind_group(0, &batch.material.material_bind_group, &[]);
                pass.set_vertex_buffer(0, batch.mesh.vertex_buffer().slice(..));
                let instance_range = instance_start..instance_start + instance_count;

                if let Some(index_buffer) = batch.mesh.index_buffer() {
                    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..batch.mesh.index_count(), 0, instance_range);
                } else {
                    pass.draw(0..batch.mesh.vertex_count(), instance_range);
                }
            }
            drop(pass);

            let sampled_post_process_bind_group =
                deferred_lighting_post_process_view.as_ref().map(|view| {
                    frame.queue.write_buffer(
                        &self.sampled_post_process_uniform_buffer,
                        0,
                        bytemuck::bytes_of(&SampledPostProcessUniform::new(
                            surface_size.width,
                            surface_size.height,
                            self.post_process_options,
                        )),
                    );
                    frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Neo Sampled Post Process Bind Group"),
                        layout: &self.sampled_post_process_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(
                                    &self.sampled_post_process_sampler,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: self
                                    .sampled_post_process_uniform_buffer
                                    .as_entire_binding(),
                            },
                        ],
                    })
                });
            let post_process_pass_label = if sampled_post_process_bind_group.is_some() {
                sampled_post_process_pass_label(self.post_process_options)
            } else {
                "Neo Post Process Pass".to_owned()
            };
            native_pass_label_stats.record_native_pass_label(&post_process_pass_label);
            let post_process_depth_stencil_attachment =
                frame
                    .depth_view
                    .map(|view| wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    });
            let mut pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(post_process_pass_label.as_str()),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: frame.view,
                        resolve_target: frame.resolve_target,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: store_op_for_resolve(frame.resolve_target),
                        },
                    })],
                    depth_stencil_attachment: post_process_depth_stencil_attachment,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            if let Some(bind_group) = sampled_post_process_bind_group.as_ref() {
                pass.set_pipeline(&self.sampled_post_process_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
            } else {
                if let Some(instance_buffer) = &self.instance_buffer {
                    pass.set_vertex_buffer(1, instance_buffer.slice(..));
                }
                pass.set_bind_group(1, active_render_bind_group, &[]);
                pass.set_bind_group(2, &self.shadow_resources.bind_group, &[]);
                pass.set_pipeline(&self.post_process_color_pipeline);
            }
            pass.draw(0..3, 0..1);
            post_process_draw_call_count = post_process_draw_call_count.saturating_add(1);
            post_pass(&mut pass);
            drop(pass);
            drop(deferred_lighting_post_process_texture);

            if let Some(timer) = &timer {
                timer.write_end_and_resolve(frame.encoder);
            }
            frame_timer = timer;
        });

        let gpu_time_ns = if result.is_ok() {
            match frame_timer.as_ref() {
                Some(timer) => timer.read_gpu_time_ns(surface.device(), surface.queue())?,
                None => None,
            }
        } else {
            None
        };

        if result.is_ok() {
            let shadow_draw_call_count = directional_shadow_draw_call_count
                .saturating_add(spot_shadow_draw_call_count)
                .saturating_add(point_shadow_draw_call_count);
            self.last_stats = MeshRenderStats {
                batch_count,
                draw_call_count: draw_call_count
                    .saturating_add(skybox_draw_call_count)
                    .saturating_add(gbuffer_draw_call_count)
                    .saturating_add(deferred_lighting_draw_call_count)
                    .saturating_add(depth_prepass_draw_call_count)
                    .saturating_add(shadow_draw_call_count)
                    .saturating_add(post_process_draw_call_count),
                native_pass_label_count: native_pass_label_stats.native_pass_label_count,
                native_pass_labels_dropped: native_pass_label_stats.native_pass_labels_dropped,
                native_pass_labels: native_pass_label_stats.native_pass_labels,
                mesh_pass_draw_call_count: draw_call_count,
                skybox_draw_call_count,
                gbuffer_draw_call_count,
                deferred_lighting_draw_call_count,
                depth_prepass_draw_call_count,
                shadow_draw_call_count,
                directional_shadow_draw_call_count,
                spot_shadow_draw_call_count,
                point_shadow_draw_call_count,
                opaque_draw_call_count,
                transparent_draw_call_count,
                post_process_draw_call_count,
                instance_count,
                instance_buffer_capacity: self.instance_buffer_capacity,
                timestamp_writes: if gpu_time_ns.is_some() { 2 } else { 0 },
                gpu_time_ns,
            };
        }

        result
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, instance_count: usize) {
        if self.instance_buffer_capacity >= instance_count {
            return;
        }

        let capacity = instance_count.next_power_of_two().max(1);
        let size = capacity * std::mem::size_of::<InstanceRaw>();
        self.instance_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Mesh Instance Buffer"),
            size: size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.instance_buffer_capacity = capacity;
    }

    fn ensure_shadow_resources(
        &mut self,
        device: &wgpu::Device,
        requested_size: u32,
        requested_spot_size: u32,
        requested_point_size: u32,
    ) {
        let requested_size = requested_size.max(1);
        let requested_spot_size = requested_spot_size.max(1);
        let requested_point_size = requested_point_size.max(1);
        if self.shadow_resources.size == requested_size
            && self.shadow_resources.spot_size == requested_spot_size
            && self.shadow_resources.point_size == requested_point_size
        {
            return;
        }

        self.shadow_resources = ShadowResources::new(
            device,
            &self.shadow_bind_group_layout,
            &self.shadow_sampler,
            requested_size,
            requested_spot_size,
            requested_point_size,
        );
    }

    pub const STATIC_RENDER_PIPELINE_COUNT: usize = 33;
    pub const STATIC_RENDER_PIPELINE_LAYOUT_COUNT: usize = 3;

    pub fn render_pipeline_count(&self) -> usize {
        Self::STATIC_RENDER_PIPELINE_COUNT
    }

    pub fn render_pipeline_layout_count(&self) -> usize {
        Self::STATIC_RENDER_PIPELINE_LAYOUT_COUNT
    }

    fn pipeline_for(
        &self,
        blend_mode: BlendMode,
        depth_write: bool,
        depth_enabled: bool,
        double_sided: bool,
    ) -> &wgpu::RenderPipeline {
        if double_sided {
            match (blend_mode, depth_enabled, depth_write) {
                (BlendMode::Opaque, false, _) => &self.double_sided_opaque_color_pipeline,
                (BlendMode::Opaque, true, true) => &self.double_sided_opaque_depth_pipeline,
                (BlendMode::Opaque, true, false) => &self.double_sided_opaque_depth_read_pipeline,
                (BlendMode::AlphaBlend, false, _) => &self.double_sided_alpha_blend_color_pipeline,
                (BlendMode::AlphaBlend, true, false) => {
                    &self.double_sided_alpha_blend_depth_pipeline
                }
                (BlendMode::AlphaBlend, true, true) => {
                    &self.double_sided_alpha_blend_depth_write_pipeline
                }
            }
        } else {
            match (blend_mode, depth_enabled, depth_write) {
                (BlendMode::Opaque, false, _) => &self.opaque_color_pipeline,
                (BlendMode::Opaque, true, true) => &self.opaque_depth_pipeline,
                (BlendMode::Opaque, true, false) => &self.opaque_depth_read_pipeline,
                (BlendMode::AlphaBlend, false, _) => &self.alpha_blend_color_pipeline,
                (BlendMode::AlphaBlend, true, false) => &self.alpha_blend_depth_pipeline,
                (BlendMode::AlphaBlend, true, true) => &self.alpha_blend_depth_write_pipeline,
            }
        }
    }

    fn gbuffer_pipeline_for(
        &self,
        depth_enabled: bool,
        double_sided: bool,
    ) -> &wgpu::RenderPipeline {
        match (depth_enabled, double_sided) {
            (true, true) => &self.double_sided_gbuffer_depth_pipeline,
            (true, false) => &self.gbuffer_depth_pipeline,
            (false, true) => &self.double_sided_gbuffer_color_pipeline,
            (false, false) => &self.gbuffer_color_pipeline,
        }
    }

    fn single_sample_depth_pipeline_for(
        &self,
        blend_mode: BlendMode,
        depth_write: bool,
        double_sided: bool,
    ) -> &wgpu::RenderPipeline {
        if double_sided {
            match (blend_mode, depth_write) {
                (BlendMode::Opaque, true) => &self.single_sample_double_sided_opaque_depth_pipeline,
                (BlendMode::Opaque, false) => {
                    &self.single_sample_double_sided_opaque_depth_read_pipeline
                }
                (BlendMode::AlphaBlend, false) => {
                    &self.single_sample_double_sided_alpha_blend_depth_pipeline
                }
                (BlendMode::AlphaBlend, true) => {
                    &self.single_sample_double_sided_alpha_blend_depth_write_pipeline
                }
            }
        } else {
            match (blend_mode, depth_write) {
                (BlendMode::Opaque, true) => &self.single_sample_opaque_depth_pipeline,
                (BlendMode::Opaque, false) => &self.single_sample_opaque_depth_read_pipeline,
                (BlendMode::AlphaBlend, false) => &self.single_sample_alpha_blend_depth_pipeline,
                (BlendMode::AlphaBlend, true) => {
                    &self.single_sample_alpha_blend_depth_write_pipeline
                }
            }
        }
    }
}

fn mesh_pass_phase_order(blend_mode: BlendMode) -> u8 {
    match blend_mode {
        BlendMode::Opaque => 0,
        BlendMode::AlphaBlend => 1,
    }
}

fn compare_mesh_pass_draw_order(
    left: &MeshBatchDraw<'_>,
    right: &MeshBatchDraw<'_>,
    camera_position: [f32; 3],
) -> Ordering {
    let left_phase = mesh_pass_phase_order(left.material.blend_mode());
    let right_phase = mesh_pass_phase_order(right.material.blend_mode());
    left_phase.cmp(&right_phase).then_with(|| {
        if matches!(left.material.blend_mode(), BlendMode::AlphaBlend)
            && matches!(right.material.blend_mode(), BlendMode::AlphaBlend)
        {
            mesh_batch_distance_sq(right, camera_position)
                .partial_cmp(&mesh_batch_distance_sq(left, camera_position))
                .unwrap_or(Ordering::Equal)
        } else {
            Ordering::Equal
        }
    })
}

fn mesh_batch_distance_sq(batch: &MeshBatchDraw<'_>, camera_position: [f32; 3]) -> f32 {
    mesh_instances_distance_sq(&batch.instances, camera_position)
}

fn mesh_instances_distance_sq(instances: &[&WgpuMeshInstance], camera_position: [f32; 3]) -> f32 {
    instances
        .iter()
        .map(|instance| {
            let translation = instance.model.to_cols_array()[3];
            distance_sq(
                [translation[0], translation[1], translation[2]],
                camera_position,
            )
        })
        .fold(0.0_f32, f32::max)
}

fn distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn validate_sample_count(sample_count: u32) -> GraphicsResult<u32> {
    match sample_count {
        1 | 2 | 4 | 8 | 16 => Ok(sample_count),
        _ => Err(GraphicsError::InvalidResource(format!(
            "mesh renderer sample count must be one of 1, 2, 4, 8, or 16; got {sample_count}"
        ))),
    }
}

fn store_op_for_resolve(resolve_target: Option<&wgpu::TextureView>) -> wgpu::StoreOp {
    if resolve_target.is_some() {
        wgpu::StoreOp::Discard
    } else {
        wgpu::StoreOp::Store
    }
}

fn create_render_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    render_uniform_buffer: &wgpu::Buffer,
    environment_texture: &WgpuEnvironmentTexture,
    brdf_lut_texture: &WgpuTexture,
    environment_probe_textures: &[&WgpuEnvironmentTexture],
) -> wgpu::BindGroup {
    let environment_texture_1 = environment_probe_textures
        .get(1)
        .copied()
        .unwrap_or(environment_texture);
    let environment_texture_2 = environment_probe_textures
        .get(2)
        .copied()
        .unwrap_or(environment_texture);
    let environment_texture_3 = environment_probe_textures
        .get(3)
        .copied()
        .unwrap_or(environment_texture);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Neo Render Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: render_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(environment_texture.view()),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(environment_texture.sampler()),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(brdf_lut_texture.view()),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(environment_texture_1.view()),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(environment_texture_2.view()),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(environment_texture_3.view()),
            },
        ],
    })
}

fn create_mesh_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    surface_format: wgpu::TextureFormat,
    sample_count: u32,
    label: &'static str,
    blend_mode: BlendMode,
    depth_format: Option<wgpu::TextureFormat>,
    depth_write_enabled: bool,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuMesh::vertex_layout(), InstanceRaw::layout()];
    let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });
    let blend = match blend_mode {
        BlendMode::Opaque => wgpu::BlendState::REPLACE,
        BlendMode::AlphaBlend => wgpu::BlendState::ALPHA_BLENDING,
    };

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: if double_sided {
                None
            } else {
                Some(wgpu::Face::Back)
            },
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_mesh_gbuffer_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuMesh::vertex_layout(), InstanceRaw::layout()];
    let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });
    let targets = [
        Some(wgpu::ColorTargetState {
            format: GBUFFER_ALBEDO_FORMAT,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(wgpu::ColorTargetState {
            format: GBUFFER_NORMAL_FORMAT,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(wgpu::ColorTargetState {
            format: GBUFFER_MATERIAL_FORMAT,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        }),
    ];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(match (depth_format.is_some(), double_sided) {
            (true, true) => "Neo Double-Sided GBuffer Depth Pipeline",
            (true, false) => "Neo GBuffer Depth Pipeline",
            (false, true) => "Neo Double-Sided GBuffer Color Pipeline",
            (false, false) => "Neo GBuffer Color Pipeline",
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: if double_sided {
                None
            } else {
                Some(wgpu::Face::Back)
            },
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &targets,
        }),
        multiview: None,
    })
}

fn create_deferred_lighting_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Neo Deferred Lighting Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_sampled_post_process_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    surface_format: wgpu::TextureFormat,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
) -> wgpu::RenderPipeline {
    let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Neo Sampled Post Process Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_mesh_depth_prepass_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    sample_count: u32,
    depth_format: wgpu::TextureFormat,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuMesh::vertex_layout(), InstanceRaw::layout()];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if double_sided {
            "Neo Double-Sided Mesh Depth Prepass Pipeline"
        } else {
            "Neo Mesh Depth Prepass Pipeline"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: if double_sided {
                None
            } else {
                Some(wgpu::Face::Back)
            },
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: depth_format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..wgpu::MultisampleState::default()
        },
        fragment: None,
        multiview: None,
    })
}

fn create_skybox_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    surface_format: wgpu::TextureFormat,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
) -> wgpu::RenderPipeline {
    let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if depth_format.is_some() {
            "Neo Skybox Depth Pipeline"
        } else {
            "Neo Skybox Color Pipeline"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_post_process_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    surface_format: wgpu::TextureFormat,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
) -> wgpu::RenderPipeline {
    let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if depth_format.is_some() {
            "Neo Post Process Depth Pipeline"
        } else {
            "Neo Post Process Color Pipeline"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

fn create_shadow_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    let vertex_buffers = [WgpuMesh::vertex_layout(), InstanceRaw::shadow_layout()];

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if double_sided {
            "Neo Double-Sided Directional Shadow Pipeline"
        } else {
            "Neo Directional Shadow Pipeline"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &vertex_buffers,
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: if double_sided {
                None
            } else {
                Some(wgpu::Face::Back)
            },
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: SHADOW_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview: None,
    })
}

fn normalize_or(vector: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length_squared = vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2];
    if length_squared > f32::EPSILON {
        let length = length_squared.sqrt();
        [vector[0] / length, vector[1] / length, vector[2] / length]
    } else {
        normalize_or(fallback, [0.0, 1.0, 0.0])
    }
}

fn directional_shadow_view_projection(lighting: RenderLighting) -> Mat4 {
    let shadow = lighting.directional_shadow;
    let size = shadow.projection_size.max(0.0001) * 0.5;
    let near = shadow.near.min(shadow.far - 0.0001);
    let far = shadow.far.max(near + 0.0001);
    let projection = Mat4::orthographic(-size, size, -size, size, near, far);
    let light_dir = normalize_or(
        lighting.directional.direction,
        RenderLighting::DEFAULT.directional.direction,
    );
    let forward = normalize_or(
        [-light_dir[0], -light_dir[1], -light_dir[2]],
        [0.0, -1.0, 0.0],
    );
    let up_hint = if forward[1].abs() > 0.95 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = normalize_or(cross(up_hint, forward), [1.0, 0.0, 0.0]);
    let up = cross(forward, right);
    let view = Mat4::from_cols_array([
        [right[0], up[0], forward[0], 0.0],
        [right[1], up[1], forward[1], 0.0],
        [right[2], up[2], forward[2], 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    projection * view
}

fn directional_shadow_cascades(
    lighting: RenderLighting,
    camera: Camera,
    aspect_ratio: f32,
) -> DirectionalShadowCascades {
    let camera_forward = camera_forward(camera);
    let shadow = lighting.directional_shadow;
    if !shadow.enabled || shadow.strength <= 0.0 {
        return DirectionalShadowCascades::disabled(camera_forward);
    }

    let requested_count = shadow
        .cascade_count
        .clamp(1, MAX_DIRECTIONAL_SHADOW_CASCADES);
    if requested_count == 1 {
        let mut view_projections = [Mat4::IDENTITY; MAX_DIRECTIONAL_SHADOW_CASCADES];
        let mut splits = [0.0; MAX_DIRECTIONAL_SHADOW_CASCADES];
        view_projections[0] = directional_shadow_view_projection(lighting);
        splits[0] = shadow
            .cascade_max_distance
            .max(shadow.projection_size)
            .max(0.0001);
        return DirectionalShadowCascades {
            count: 1,
            view_projections,
            splits,
            camera_forward,
        };
    }

    let Camera::Perspective(camera) = camera else {
        let mut view_projections = [Mat4::IDENTITY; MAX_DIRECTIONAL_SHADOW_CASCADES];
        let mut splits = [0.0; MAX_DIRECTIONAL_SHADOW_CASCADES];
        view_projections[0] = directional_shadow_view_projection(lighting);
        splits[0] = shadow
            .cascade_max_distance
            .max(shadow.projection_size)
            .max(0.0001);
        return DirectionalShadowCascades {
            count: 1,
            view_projections,
            splits,
            camera_forward,
        };
    };

    let near = camera.near.max(0.0001);
    let far = camera
        .far
        .max(near + 0.0001)
        .min(shadow.cascade_max_distance.max(near + 0.0001));
    if far <= near + 0.0001 {
        return DirectionalShadowCascades::disabled(camera_forward);
    }

    let mut view_projections = [Mat4::IDENTITY; MAX_DIRECTIONAL_SHADOW_CASCADES];
    let mut splits = [0.0; MAX_DIRECTIONAL_SHADOW_CASCADES];
    let split_lambda = shadow.cascade_split_lambda.clamp(0.0, 1.0);
    let mut previous_split = near;

    for cascade_index in 0..requested_count {
        let split = cascade_split_distance(near, far, requested_count, cascade_index, split_lambda);
        let corners = perspective_frustum_corners(camera, aspect_ratio, previous_split, split);
        view_projections[cascade_index] = fit_directional_shadow_projection(lighting, &corners);
        splits[cascade_index] = split;
        previous_split = split;
    }

    DirectionalShadowCascades {
        count: requested_count,
        view_projections,
        splits,
        camera_forward,
    }
}

fn cascade_split_distance(
    near: f32,
    far: f32,
    cascade_count: usize,
    cascade_index: usize,
    split_lambda: f32,
) -> f32 {
    let fraction = (cascade_index + 1) as f32 / cascade_count.max(1) as f32;
    let logarithmic = near * (far / near).powf(fraction);
    let uniform = near + (far - near) * fraction;
    split_lambda * logarithmic + (1.0 - split_lambda) * uniform
}

fn perspective_frustum_corners(
    camera: PerspectiveCamera,
    aspect_ratio: f32,
    near: f32,
    far: f32,
) -> [[f32; 3]; 8] {
    let (right, up, forward) = perspective_camera_basis(camera);
    let near_center = add3(camera.position, scale3(forward, near));
    let far_center = add3(camera.position, scale3(forward, far));
    let tan_half_fov = (camera.vertical_fov_radians.max(0.0001) * 0.5).tan();
    let near_half_height = tan_half_fov * near;
    let near_half_width = near_half_height * aspect_ratio.max(0.0001);
    let far_half_height = tan_half_fov * far;
    let far_half_width = far_half_height * aspect_ratio.max(0.0001);

    [
        add3(
            add3(near_center, scale3(right, -near_half_width)),
            scale3(up, -near_half_height),
        ),
        add3(
            add3(near_center, scale3(right, near_half_width)),
            scale3(up, -near_half_height),
        ),
        add3(
            add3(near_center, scale3(right, near_half_width)),
            scale3(up, near_half_height),
        ),
        add3(
            add3(near_center, scale3(right, -near_half_width)),
            scale3(up, near_half_height),
        ),
        add3(
            add3(far_center, scale3(right, -far_half_width)),
            scale3(up, -far_half_height),
        ),
        add3(
            add3(far_center, scale3(right, far_half_width)),
            scale3(up, -far_half_height),
        ),
        add3(
            add3(far_center, scale3(right, far_half_width)),
            scale3(up, far_half_height),
        ),
        add3(
            add3(far_center, scale3(right, -far_half_width)),
            scale3(up, far_half_height),
        ),
    ]
}

fn fit_directional_shadow_projection(lighting: RenderLighting, corners: &[[f32; 3]; 8]) -> Mat4 {
    let light_dir = normalize_or(
        lighting.directional.direction,
        RenderLighting::DEFAULT.directional.direction,
    );
    let forward = normalize_or(
        [-light_dir[0], -light_dir[1], -light_dir[2]],
        [0.0, -1.0, 0.0],
    );
    let center = corners
        .iter()
        .copied()
        .fold([0.0; 3], add3)
        .map(|value| value / corners.len() as f32);
    let up_hint = if forward[1].abs() > 0.95 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let view = look_to_rh_with_up(center, forward, up_hint);
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];

    for corner in corners {
        let light_space = view.transform_point3(*corner);
        for axis in 0..3 {
            min[axis] = min[axis].min(light_space[axis]);
            max[axis] = max[axis].max(light_space[axis]);
        }
    }

    let x_padding = ((max[0] - min[0]) * 0.02).max(0.01);
    let y_padding = ((max[1] - min[1]) * 0.02).max(0.01);
    let z_padding = ((max[2] - min[2]) * 0.5).max(0.5);
    Mat4::orthographic(
        min[0] - x_padding,
        max[0] + x_padding,
        min[1] - y_padding,
        max[1] + y_padding,
        min[2] - z_padding,
        max[2] + z_padding,
    ) * view
}

fn camera_forward(camera: Camera) -> [f32; 3] {
    camera.basis().2
}

fn perspective_camera_basis(camera: PerspectiveCamera) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let rotation = Mat4::rotation_z(camera.rotation_radians[2])
        * Mat4::rotation_y(camera.rotation_radians[1])
        * Mat4::rotation_x(camera.rotation_radians[0]);
    (
        normalize_or(rotation.transform_vector3([1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]),
        normalize_or(rotation.transform_vector3([0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]),
        normalize_or(
            rotation.transform_vector3([0.0, 0.0, -1.0]),
            [0.0, 0.0, -1.0],
        ),
    )
}

fn active_spot_shadows(lighting: RenderLighting) -> Vec<(usize, SpotLight)> {
    lighting
        .spot_lights()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, light)| {
            light.shadow.enabled && light.shadow.strength > 0.0 && light.intensity > 0.0
        })
        .collect()
}

fn active_point_shadows(lighting: RenderLighting) -> Vec<(usize, PointLight)> {
    lighting
        .point_lights()
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, light)| {
            light.shadow.enabled && light.shadow.strength > 0.0 && light.intensity > 0.0
        })
        .collect()
}

fn spot_shadow_view_projection(light: SpotLight) -> Mat4 {
    let near = light.shadow.near.max(0.0001);
    let far = light
        .shadow
        .far
        .max(light.range.max(near + 0.0001))
        .max(near + 0.0001);
    let vertical_fov =
        (light.outer_angle_radians.max(0.0001) * 2.0).clamp(0.0001, std::f32::consts::PI - 0.0001);
    Mat4::perspective_rh(vertical_fov, 1.0, near, far) * look_to_rh(light.position, light.direction)
}

fn point_shadow_view_projection(light: PointLight, face: usize) -> Mat4 {
    let near = light.shadow.near.max(0.0001);
    let far = light
        .shadow
        .far
        .max(light.range.max(near + 0.0001))
        .max(near + 0.0001);
    let (direction, up) = point_shadow_face_direction_and_up(face);
    Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, near, far)
        * look_to_rh_with_up(light.position, direction, up)
}

fn point_shadow_face_direction_and_up(face: usize) -> ([f32; 3], [f32; 3]) {
    match face {
        0 => ([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        1 => ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        2 => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        3 => ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
        4 => ([0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),
        _ => ([0.0, 0.0, -1.0], [0.0, -1.0, 0.0]),
    }
}

fn look_to_rh(position: [f32; 3], direction: [f32; 3]) -> Mat4 {
    let forward = normalize_or(direction, [0.0, 0.0, -1.0]);
    let up_hint = if forward[1].abs() > 0.95 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    look_to_rh_with_up(position, forward, up_hint)
}

fn look_to_rh_with_up(position: [f32; 3], direction: [f32; 3], up_hint: [f32; 3]) -> Mat4 {
    let forward = normalize_or(direction, [0.0, 0.0, -1.0]);
    let right = normalize_or(cross(forward, up_hint), [1.0, 0.0, 0.0]);
    let up = cross(right, forward);

    Mat4::from_cols_array([
        [right[0], up[0], -forward[0], 0.0],
        [right[1], up[1], -forward[1], 0.0],
        [right[2], up[2], -forward[2], 0.0],
        [
            -dot(right, position),
            -dot(up, position),
            dot(forward, position),
            1.0,
        ],
    ])
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(vector: [f32; 3], scale: f32) -> [f32; 3] {
    [vector[0] * scale, vector[1] * scale, vector[2] * scale]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn spotlight_angle_cosines(inner_angle_radians: f32, outer_angle_radians: f32) -> (f32, f32) {
    let inner_angle = inner_angle_radians.clamp(0.0, std::f32::consts::PI);
    let outer_angle = outer_angle_radians
        .max(inner_angle)
        .clamp(0.0, std::f32::consts::PI);
    let outer_cos = outer_angle.cos();
    let mut inner_cos = inner_angle.cos();

    if inner_cos <= outer_cos {
        inner_cos = (outer_cos + 0.0001).min(1.0);
    }

    (inner_cos, outer_cos)
}

fn wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn skybox_uniform(camera: Camera, aspect_ratio: f32, intensity: f32) -> SkyboxUniform {
    let (right, up, forward) = camera.basis();
    let (tan_half_fov, aspect_ratio) = camera.skybox_projection(aspect_ratio);

    SkyboxUniform {
        camera_right: [right[0], right[1], right[2], 0.0],
        camera_up: [up[0], up[1], up[2], 0.0],
        camera_forward: [forward[0], forward[1], forward[2], 0.0],
        options: [tan_half_fov, aspect_ratio, intensity.max(0.0), 0.0],
    }
}

struct WgpuFrameTimestampReadback {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
}

impl WgpuFrameTimestampReadback {
    const COUNT: u32 = 2;
    const BYTE_SIZE: u64 = std::mem::size_of::<u64>() as u64 * Self::COUNT as u64;

    fn new(device: &wgpu::Device) -> Option<Self> {
        if !device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            return None;
        }

        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Neo Mesh Frame Timestamp Query"),
            ty: wgpu::QueryType::Timestamp,
            count: Self::COUNT,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Mesh Frame Timestamp Resolve"),
            size: Self::BYTE_SIZE,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Mesh Frame Timestamp Readback"),
            size: Self::BYTE_SIZE,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Some(Self {
            query_set,
            resolve_buffer,
            readback_buffer,
        })
    }

    fn write_start(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.query_set, 0);
    }

    fn write_end_and_resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.query_set, 1);
        encoder.resolve_query_set(&self.query_set, 0..Self::COUNT, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            &self.resolve_buffer,
            0,
            &self.readback_buffer,
            0,
            Self::BYTE_SIZE,
        );
    }

    fn read_gpu_time_ns(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> GraphicsResult<Option<u64>> {
        let slice = self.readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                return Err(GraphicsError::Backend(format!(
                    "mesh renderer timestamp readback mapping failed: {error}"
                )));
            }
            Err(_) => {
                return Err(GraphicsError::Backend(
                    "mesh renderer timestamp readback callback was canceled".to_owned(),
                ));
            }
        }

        let mapped = slice.get_mapped_range();
        let start = u64::from_le_bytes(
            mapped
                .get(..std::mem::size_of::<u64>())
                .expect("timestamp readback buffer contains start timestamp")
                .try_into()
                .expect("start timestamp readback slice is one u64"),
        );
        let end = u64::from_le_bytes(
            mapped
                .get(std::mem::size_of::<u64>()..std::mem::size_of::<u64>() * 2)
                .expect("timestamp readback buffer contains end timestamp")
                .try_into()
                .expect("end timestamp readback slice is one u64"),
        );
        drop(mapped);
        self.readback_buffer.unmap();

        let ticks = end.saturating_sub(start);
        Ok(Some(
            (ticks as f64 * queue.get_timestamp_period() as f64) as u64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_render::{OrthographicCamera, PointShadow, RenderLighting, ViewCamera};

    #[test]
    fn directional_shadow_cascades_respect_requested_count_and_splits() {
        let mut camera = PerspectiveCamera::new(std::f32::consts::FRAC_PI_3, 0.1, 100.0);
        camera.position = [0.0, 0.0, 4.0];
        let shadow = DirectionalShadow::enabled(1024, 8.0, -8.0, 8.0, 0.7, 0.002)
            .with_cascades(4, 20.0, 0.5);
        let lighting = RenderLighting::DEFAULT.with_directional_shadow(shadow);

        let cascades = directional_shadow_cascades(lighting, Camera::from(camera), 16.0 / 9.0);

        assert_eq!(cascades.count, 4);
        assert!(cascades.splits[0] > camera.near);
        assert!(cascades.splits[0] < cascades.splits[1]);
        assert!(cascades.splits[1] < cascades.splits[2]);
        assert!(cascades.splits[2] < cascades.splits[3]);
        assert!(cascades.splits[3] <= shadow.cascade_max_distance);
        assert_eq!(cascades.camera_forward, [0.0, 0.0, -1.0]);
    }

    #[test]
    fn directional_shadow_cascades_fall_back_to_single_for_orthographic_camera() {
        let camera = OrthographicCamera::new_2d(4.0);
        let shadow = DirectionalShadow::enabled(1024, 8.0, -8.0, 8.0, 0.7, 0.002)
            .with_cascades(4, 20.0, 0.5);
        let lighting = RenderLighting::DEFAULT.with_directional_shadow(shadow);

        let cascades = directional_shadow_cascades(lighting, Camera::from(camera), 1.0);

        assert_eq!(cascades.count, 1);
        assert_eq!(cascades.camera_forward, [0.0, 0.0, -1.0]);
    }

    #[test]
    fn active_point_shadows_include_only_enabled_lit_lights() {
        let enabled = PointLight::new([1.0, 2.0, 3.0], [1.0, 1.0, 1.0], 1.0, 4.0)
            .with_shadow(PointShadow::enabled(512, 0.05, 6.0, 0.5, 0.002));
        let disabled_shadow = PointLight::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 1.0, 4.0);
        let disabled_intensity = PointLight::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], 0.0, 4.0)
            .with_shadow(PointShadow::enabled(512, 0.05, 6.0, 0.5, 0.002));
        let lighting = RenderLighting::DEFAULT.with_point_lights(&[
            disabled_shadow,
            enabled,
            disabled_intensity,
        ]);

        let active = active_point_shadows(lighting);

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].0, 1);
        assert_eq!(active[0].1, enabled);
    }

    #[test]
    fn point_shadow_faces_match_shader_order() {
        let faces = (0..POINT_SHADOW_FACE_COUNT)
            .map(point_shadow_face_direction_and_up)
            .collect::<Vec<_>>();

        assert_eq!(faces[0].0, [1.0, 0.0, 0.0]);
        assert_eq!(faces[1].0, [-1.0, 0.0, 0.0]);
        assert_eq!(faces[2].0, [0.0, 1.0, 0.0]);
        assert_eq!(faces[3].0, [0.0, -1.0, 0.0]);
        assert_eq!(faces[4].0, [0.0, 0.0, 1.0]);
        assert_eq!(faces[5].0, [0.0, 0.0, -1.0]);
    }

    #[test]
    fn mesh_renderer_accepts_webgpu_sample_counts() {
        for sample_count in [1, 2, 4, 8, 16] {
            assert_eq!(validate_sample_count(sample_count).unwrap(), sample_count);
        }

        assert!(validate_sample_count(3).is_err());
    }

    #[test]
    fn mesh_render_stats_reports_gpu_time_ms() {
        let stats = MeshRenderStats {
            mesh_pass_draw_call_count: 3,
            skybox_draw_call_count: 1,
            gbuffer_draw_call_count: 2,
            deferred_lighting_draw_call_count: 1,
            depth_prepass_draw_call_count: 2,
            shadow_draw_call_count: 3,
            directional_shadow_draw_call_count: 1,
            spot_shadow_draw_call_count: 1,
            point_shadow_draw_call_count: 1,
            draw_call_count: 12,
            gpu_time_ns: Some(2_500_000),
            timestamp_writes: 2,
            ..MeshRenderStats::default()
        };

        assert_eq!(stats.gpu_time_ms(), Some(2.5));
        assert_eq!(
            stats.draw_call_count,
            stats.mesh_pass_draw_call_count
                + stats.skybox_draw_call_count
                + stats.gbuffer_draw_call_count
                + stats.deferred_lighting_draw_call_count
                + stats.depth_prepass_draw_call_count
                + stats.shadow_draw_call_count
        );
        assert_eq!(
            stats.shadow_draw_call_count,
            stats.directional_shadow_draw_call_count
                + stats.spot_shadow_draw_call_count
                + stats.point_shadow_draw_call_count
        );
    }

    #[test]
    fn mesh_render_stats_preserve_actual_native_pass_labels() {
        let mut stats = MeshRenderStats::default();
        stats.record_native_pass_label("Neo Depth Prepass");
        stats.record_native_pass_label("Neo GBuffer Pass");
        stats.record_native_pass_label("Neo Deferred Lighting Pass");
        stats.record_native_pass_label("Neo Forward Opaque Pass");
        stats.record_native_pass_label("Neo Transparent Pass");
        stats.record_native_pass_label("Neo Tonemap Post Process Pass");

        assert_eq!(stats.native_pass_label_count, 6);
        assert_eq!(
            stats.native_pass_label_strings(),
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
    fn mesh_render_stats_native_pass_labels_are_bounded() {
        let mut stats = MeshRenderStats::default();
        for _ in 0..MAX_NATIVE_PASS_LABELS + 4 {
            stats.record_native_pass_label("Neo Transparent Pass");
        }

        assert_eq!(stats.native_pass_label_count, MAX_NATIVE_PASS_LABELS);
        assert_eq!(
            stats.native_pass_label_strings().len(),
            MAX_NATIVE_PASS_LABELS
        );
        assert_eq!(stats.native_pass_labels_dropped, 4);
    }

    #[test]
    fn mesh_pass_phase_order_draws_opaque_before_transparent() {
        let mut phases = vec![
            BlendMode::AlphaBlend,
            BlendMode::Opaque,
            BlendMode::AlphaBlend,
        ];
        phases.sort_by_key(|phase| mesh_pass_phase_order(*phase));

        assert_eq!(
            phases,
            vec![
                BlendMode::Opaque,
                BlendMode::AlphaBlend,
                BlendMode::AlphaBlend
            ]
        );
    }

    #[test]
    fn mesh_batch_distance_uses_farthest_instance_for_transparent_sorting() {
        let near = WgpuMeshInstance {
            model_view_projection: Mat4::IDENTITY,
            normal_matrix: Mat4::IDENTITY,
            model: Mat4::translation([0.0, 0.0, 2.0]),
        };
        let far = WgpuMeshInstance {
            model_view_projection: Mat4::IDENTITY,
            normal_matrix: Mat4::IDENTITY,
            model: Mat4::translation([0.0, 0.0, 8.0]),
        };
        let camera_position = [0.0, 0.0, 0.0];
        let instances = vec![&near, &far];

        assert_eq!(
            mesh_instances_distance_sq(&instances, camera_position),
            64.0
        );
    }

    #[test]
    fn mesh_vertex_layouts_fit_webgpu_attribute_limit() {
        let mesh_attribute_count = WgpuMesh::vertex_layout().attributes.len();

        assert!(mesh_attribute_count + InstanceRaw::layout().attributes.len() <= 16);
        assert!(mesh_attribute_count + InstanceRaw::shadow_layout().attributes.len() <= 16);
    }

    #[test]
    fn mesh_renderer_reports_static_pipeline_inventory() {
        assert_eq!(MeshRenderer::STATIC_RENDER_PIPELINE_COUNT, 33);
        assert_eq!(MeshRenderer::STATIC_RENDER_PIPELINE_LAYOUT_COUNT, 3);
    }

    #[test]
    fn gbuffer_shader_declares_mrt_outputs() {
        let shader = include_str!("gbuffer.wgsl");

        assert_eq!(GBUFFER_COLOR_ATTACHMENT_COUNT, 3);
        assert!(shader.contains("@location(0) albedo"));
        assert!(shader.contains("@location(1) normal"));
        assert!(shader.contains("@location(2) material"));
        assert!(shader.contains("textureSample(base_color_texture"));
        assert!(shader.contains("textureSample(normal_texture"));
        assert!(shader.contains("textureSample(metallic_roughness_texture"));
    }

    #[test]
    fn deferred_lighting_shader_samples_gbuffer_mrt() {
        let shader = include_str!("deferred_lighting.wgsl");

        assert!(shader.contains("var gbuffer_albedo: texture_2d<f32>"));
        assert!(shader.contains("var gbuffer_normal: texture_2d<f32>"));
        assert!(shader.contains("var gbuffer_material: texture_2d<f32>"));
        assert!(shader.contains("textureSampleLevel(gbuffer_albedo"));
        assert!(shader.contains("textureSampleLevel(gbuffer_normal"));
        assert!(shader.contains("textureSampleLevel(gbuffer_material"));
    }

    #[test]
    fn sampled_post_process_shader_samples_deferred_lighting_target() {
        let shader = include_str!("post_process_sampled.wgsl");

        assert!(shader.contains("var source_texture: texture_2d<f32>"));
        assert!(shader.contains("var source_sampler: sampler"));
        assert!(shader.contains("var<uniform> post_process"));
        assert!(shader.contains("textureSampleLevel(source_texture"));
        assert!(shader.contains("apply_fxaa"));
        assert!(shader.contains("apply_bloom"));
        assert!(shader.contains("apply_color_grading"));
        assert!(shader.contains("apply_taa_resolve"));
        assert!(shader.contains("apply_motion_blur"));
        assert!(shader.contains("apply_ssr"));
        assert!(shader.contains("apply_depth_of_field"));
        assert!(shader.contains("apply_ssao"));
        assert!(shader.contains("apply_hdr_exposure"));
        assert!(shader.contains("post_process.texel_size_and_flags.w > 0.5"));
        assert!(shader.contains("post_process.color_grade_flags.x > 0.5"));
        assert!(shader.contains("post_process.effect_flags.x > 0.5"));
        assert!(shader.contains("post_process.screen_space_flags.x > 0.5"));
        assert!(shader.contains("post_process.screen_space_flags.y > 0.5"));
        assert!(shader.contains("pow(mapped"));
    }

    #[test]
    fn sampled_post_process_uniform_packs_fxaa_and_bloom_flags() {
        let uniform = SampledPostProcessUniform::new(
            640,
            480,
            WgpuPostProcessOptions {
                fxaa: true,
                bloom: true,
                color_grading: true,
                taa: true,
                motion_blur: true,
                ssr: true,
                depth_of_field: true,
                ssao: true,
                hdr: true,
            },
        );

        assert_eq!(uniform.texel_size_and_flags[0], 1.0 / 640.0);
        assert_eq!(uniform.texel_size_and_flags[1], 1.0 / 480.0);
        assert_eq!(uniform.texel_size_and_flags[2], 1.0);
        assert_eq!(uniform.texel_size_and_flags[3], 1.0);
        assert_eq!(uniform.color_grade_flags[0], 1.0);
        assert_eq!(uniform.effect_flags, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(uniform.screen_space_flags[0], 1.0);
        assert_eq!(uniform.screen_space_flags[1], 1.0);
    }

    #[test]
    fn sampled_post_process_label_reports_combined_effects() {
        assert_eq!(
            sampled_post_process_pass_label(WgpuPostProcessOptions {
                fxaa: true,
                bloom: true,
                color_grading: true,
                taa: true,
                motion_blur: true,
                ssr: true,
                depth_of_field: true,
                ssao: true,
                hdr: true,
            }),
            "Neo Hdr Bloom Ssao Taa Fxaa Motion Blur Ssr Depth Of Field Tonemap Color Grading Post Process Pass"
        );
    }

    #[test]
    fn material_uniform_marks_unlit_materials() {
        let lit_uniform = material_uniform(Material::WHITE);
        let unlit_uniform = material_uniform(Material::WHITE.with_unlit(true));

        assert_eq!(lit_uniform.volume_options[2], 0.0);
        assert_eq!(unlit_uniform.volume_options[2], 1.0);
    }

    #[test]
    fn material_uniform_marks_specular_glossiness_workflow() {
        let metallic_roughness_uniform = material_uniform(Material::WHITE);
        let specular_glossiness_uniform =
            material_uniform(Material::WHITE.with_specular_glossiness_workflow(true));

        assert_eq!(metallic_roughness_uniform.volume_options[3], 0.0);
        assert_eq!(specular_glossiness_uniform.volume_options[3], 1.0);
    }

    #[test]
    fn material_uniform_packs_clearcoat_normal_scale() {
        let uniform = material_uniform(Material::WHITE.with_clearcoat_normal_scale(0.55));

        assert_eq!(uniform.anisotropy[2], 0.55);
    }

    #[test]
    fn material_backend_layout_info_matches_mesh_shader_bindings() {
        let info = wgpu_material_layout_info();

        assert_eq!(info.uniform_binding, 0);
        assert_eq!(
            info.texture_bindings,
            &[1, 3, 4, 5, 6, 7, 8, 29, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_eq!(
            info.sampler_bindings,
            &[2, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 30]
        );
        assert_eq!(
            info.sampler_bindings.len(),
            material_sampler_slots(Material::WHITE).len()
        );
        assert_eq!(info.binding_count, 31);
        assert_eq!(info.highest_binding, 30);
        assert_eq!(WgpuMaterial::layout_info(), info);

        let expected_bindings = (0..=30).collect::<Vec<_>>();
        assert_eq!(info.occupied_bindings, expected_bindings.as_slice());

        let entries = material_bind_group_layout_entries();
        assert_eq!(entries.len(), info.binding_count);
        for (entry, expected_binding) in entries.iter().zip(info.occupied_bindings) {
            assert_eq!(entry.binding, *expected_binding);
            assert_eq!(entry.visibility, wgpu::ShaderStages::FRAGMENT);
            if entry.binding == info.uniform_binding {
                match &entry.ty {
                    wgpu::BindingType::Buffer {
                        ty,
                        has_dynamic_offset,
                        ..
                    } => {
                        assert_eq!(*ty, wgpu::BufferBindingType::Uniform);
                        assert!(!*has_dynamic_offset);
                    }
                    _ => panic!("material uniform binding has wrong layout type"),
                }
            } else if info.texture_bindings.contains(&entry.binding) {
                match &entry.ty {
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable },
                        view_dimension,
                        multisampled,
                    } => {
                        assert!(*filterable);
                        assert_eq!(*view_dimension, wgpu::TextureViewDimension::D2);
                        assert!(!*multisampled);
                    }
                    _ => panic!("material texture binding has wrong layout type"),
                }
            } else if info.sampler_bindings.contains(&entry.binding) {
                assert!(matches!(
                    &entry.ty,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                ));
            } else {
                panic!("unknown material binding {}", entry.binding);
            }
        }

        let shader = include_str!("mesh.wgsl");
        for binding in info.occupied_bindings {
            assert!(
                shader.contains(&format!("@binding({binding})")),
                "mesh shader missing material binding {binding}"
            );
        }
    }

    #[test]
    fn material_sampler_slots_preserve_per_texture_samplers() {
        let nearest_clamp = TextureSampler::new(
            TextureAddressMode::ClampToEdge,
            TextureAddressMode::MirrorRepeat,
            TextureFilterMode::Nearest,
            TextureFilterMode::Nearest,
            TextureFilterMode::Linear,
        );
        let linear_repeat = TextureSampler::new(
            TextureAddressMode::Repeat,
            TextureAddressMode::ClampToEdge,
            TextureFilterMode::Linear,
            TextureFilterMode::Linear,
            TextureFilterMode::Nearest,
        );
        let mut samplers = engine_render::MaterialTextureSamplers::DEFAULT;
        samplers.base_color = nearest_clamp;
        samplers.normal = linear_repeat;
        samplers.clearcoat_normal = linear_repeat;
        samplers.iridescence_thickness = linear_repeat;
        let material = Material::WHITE.with_texture_samplers(samplers);

        let slots = material_sampler_slots(material);

        assert_eq!(slots[0], nearest_clamp);
        assert_eq!(slots[2], linear_repeat);
        assert_eq!(slots[7], linear_repeat);
        assert_eq!(slots[13], TextureSampler::DEFAULT);
        assert_eq!(slots[14], TextureSampler::DEFAULT);
        assert_eq!(
            wgpu_address_mode(TextureAddressMode::MirrorRepeat),
            wgpu::AddressMode::MirrorRepeat
        );
        assert_eq!(
            wgpu_filter_mode(TextureFilterMode::Nearest),
            wgpu::FilterMode::Nearest
        );
    }

    #[test]
    fn material_uniform_packs_base_color_texture_transform() {
        let base_color_transform =
            TextureTransform::new([0.25, 0.5], std::f32::consts::FRAC_PI_2, [2.0, 3.0], 1);
        let material = Material::WHITE
            .with_base_color_texture_transform(base_color_transform)
            .with_metallic_roughness_texture_transform(TextureTransform::new(
                [0.1, 0.2],
                0.0,
                [4.0, 5.0],
                0,
            ))
            .with_normal_texture_transform(TextureTransform::new([0.3, 0.4], 0.0, [6.0, 7.0], 1))
            .with_emissive_texture_transform(TextureTransform::new([0.5, 0.6], 0.0, [8.0, 9.0], 0))
            .with_occlusion_texture_transform(TextureTransform::new(
                [0.7, 0.8],
                0.0,
                [10.0, 11.0],
                1,
            ))
            .with_clearcoat_texture_transform(TextureTransform::new(
                [0.9, 1.0],
                0.0,
                [12.0, 13.0],
                0,
            ))
            .with_clearcoat_roughness_texture_transform(TextureTransform::new(
                [1.1, 1.2],
                0.0,
                [14.0, 15.0],
                1,
            ))
            .with_clearcoat_normal_texture_transform(TextureTransform::new(
                [1.21, 1.22],
                0.0,
                [14.5, 15.5],
                0,
            ))
            .with_sheen_color_texture_transform(TextureTransform::new(
                [1.3, 1.4],
                0.0,
                [16.0, 17.0],
                0,
            ))
            .with_sheen_roughness_texture_transform(TextureTransform::new(
                [1.5, 1.6],
                0.0,
                [18.0, 19.0],
                1,
            ))
            .with_transmission_texture_transform(TextureTransform::new(
                [1.7, 1.8],
                0.0,
                [20.0, 21.0],
                0,
            ))
            .with_specular_texture_transform(TextureTransform::new(
                [1.9, 2.0],
                0.0,
                [22.0, 23.0],
                1,
            ))
            .with_specular_color_texture_transform(TextureTransform::new(
                [2.1, 2.2],
                0.0,
                [24.0, 25.0],
                0,
            ))
            .with_anisotropy_texture_transform(TextureTransform::new(
                [2.3, 2.4],
                0.0,
                [26.0, 27.0],
                1,
            ))
            .with_iridescence_texture_transform(TextureTransform::new(
                [2.5, 2.6],
                0.0,
                [28.0, 29.0],
                0,
            ))
            .with_iridescence_thickness_texture_transform(TextureTransform::new(
                [2.7, 2.8],
                0.0,
                [30.0, 31.0],
                1,
            ))
            .with_thickness_texture_transform(TextureTransform::new(
                [2.9, 3.0],
                0.0,
                [32.0, 33.0],
                0,
            ));
        let uniform = material_uniform(material);

        assert!(uniform.base_color_uv_transform_0[0].abs() < 0.0001);
        assert!((uniform.base_color_uv_transform_0[1] + 3.0).abs() < 0.0001);
        assert!((uniform.base_color_uv_transform_0[2] - 2.0).abs() < 0.0001);
        assert!(uniform.base_color_uv_transform_0[3].abs() < 0.0001);
        assert_eq!(uniform.base_color_uv_transform_1, [0.25, 0.5, 1.0, 0.0]);
        assert_eq!(
            uniform.metallic_roughness_uv_transform_0,
            [4.0, -0.0, 0.0, 5.0]
        );
        assert_eq!(
            uniform.metallic_roughness_uv_transform_1,
            [0.1, 0.2, 0.0, 0.0]
        );
        assert_eq!(uniform.normal_uv_transform_0, [6.0, -0.0, 0.0, 7.0]);
        assert_eq!(uniform.normal_uv_transform_1, [0.3, 0.4, 1.0, 0.0]);
        assert_eq!(uniform.emissive_uv_transform_0, [8.0, -0.0, 0.0, 9.0]);
        assert_eq!(uniform.emissive_uv_transform_1, [0.5, 0.6, 0.0, 0.0]);
        assert_eq!(uniform.occlusion_uv_transform_0, [10.0, -0.0, 0.0, 11.0]);
        assert_eq!(uniform.occlusion_uv_transform_1, [0.7, 0.8, 1.0, 0.0]);
        assert_eq!(uniform.clearcoat_uv_transform_0, [12.0, -0.0, 0.0, 13.0]);
        assert_eq!(uniform.clearcoat_uv_transform_1, [0.9, 1.0, 0.0, 0.0]);
        assert_eq!(
            uniform.clearcoat_roughness_uv_transform_0,
            [14.0, -0.0, 0.0, 15.0]
        );
        assert_eq!(
            uniform.clearcoat_roughness_uv_transform_1,
            [1.1, 1.2, 1.0, 0.0]
        );
        assert_eq!(
            uniform.clearcoat_normal_uv_transform_0,
            [14.5, -0.0, 0.0, 15.5]
        );
        assert_eq!(
            uniform.clearcoat_normal_uv_transform_1,
            [1.21, 1.22, 0.0, 0.0]
        );
        assert_eq!(uniform.sheen_color_uv_transform_0, [16.0, -0.0, 0.0, 17.0]);
        assert_eq!(uniform.sheen_color_uv_transform_1, [1.3, 1.4, 0.0, 0.0]);
        assert_eq!(
            uniform.sheen_roughness_uv_transform_0,
            [18.0, -0.0, 0.0, 19.0]
        );
        assert_eq!(uniform.sheen_roughness_uv_transform_1, [1.5, 1.6, 1.0, 0.0]);
        assert_eq!(uniform.transmission_uv_transform_0, [20.0, -0.0, 0.0, 21.0]);
        assert_eq!(uniform.transmission_uv_transform_1, [1.7, 1.8, 0.0, 0.0]);
        assert_eq!(uniform.specular_uv_transform_0, [22.0, -0.0, 0.0, 23.0]);
        assert_eq!(uniform.specular_uv_transform_1, [1.9, 2.0, 1.0, 0.0]);
        assert_eq!(
            uniform.specular_color_uv_transform_0,
            [24.0, -0.0, 0.0, 25.0]
        );
        assert_eq!(uniform.specular_color_uv_transform_1, [2.1, 2.2, 0.0, 0.0]);
        assert_eq!(uniform.anisotropy_uv_transform_0, [26.0, -0.0, 0.0, 27.0]);
        assert_eq!(uniform.anisotropy_uv_transform_1, [2.3, 2.4, 1.0, 0.0]);
        assert_eq!(uniform.iridescence_uv_transform_0, [28.0, -0.0, 0.0, 29.0]);
        assert_eq!(uniform.iridescence_uv_transform_1, [2.5, 2.6, 0.0, 0.0]);
        assert_eq!(
            uniform.iridescence_thickness_uv_transform_0,
            [30.0, -0.0, 0.0, 31.0]
        );
        assert_eq!(
            uniform.iridescence_thickness_uv_transform_1,
            [2.7, 2.8, 1.0, 0.0]
        );
        assert_eq!(uniform.thickness_uv_transform_0, [32.0, -0.0, 0.0, 33.0]);
        assert_eq!(uniform.thickness_uv_transform_1, [2.9, 3.0, 0.0, 0.0]);
    }

    #[test]
    fn mesh_shader_flips_double_sided_backface_normals() {
        let shader = include_str!("mesh.wgsl");

        assert!(shader.contains("@builtin(front_facing) is_front_facing: bool"));
        assert!(shader.contains("let face_direction = select(-1.0, 1.0, is_front_facing);"));
        assert!(shader.contains(") * face_direction,"));
    }

    #[test]
    fn skybox_uniform_uses_camera_orientation_without_translation() {
        let mut camera = PerspectiveCamera::default();
        camera.position = [10.0, 20.0, 30.0];
        camera.rotation_radians = [0.0, std::f32::consts::FRAC_PI_2, 0.0];

        let uniform = skybox_uniform(Camera::from(camera), 16.0 / 9.0, 0.4);

        assert!((uniform.camera_forward[0] - -1.0).abs() < 0.0001);
        assert!(uniform.camera_forward[1].abs() < 0.0001);
        assert!(uniform.camera_forward[2].abs() < 0.0001);
        assert_eq!(uniform.options[1], 16.0 / 9.0);
        assert_eq!(uniform.options[2], 0.4);
    }

    #[test]
    fn skybox_uniform_uses_view_camera_basis_and_projection_aspect() {
        let camera = ViewCamera::perspective(
            [10.0, 20.0, 30.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            std::f32::consts::FRAC_PI_2,
            2.0,
            0.1,
            Some(100.0),
        );

        let uniform = skybox_uniform(Camera::from(camera), 1.0, 0.4);

        assert_eq!(uniform.camera_right, [0.0, 0.0, -1.0, 0.0]);
        assert_eq!(uniform.camera_up, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(uniform.camera_forward, [-1.0, 0.0, 0.0, 0.0]);
        assert!((uniform.options[0] - 1.0).abs() < 0.0001);
        assert_eq!(uniform.options[1], 2.0);
        assert_eq!(uniform.options[2], 0.4);
    }
}
