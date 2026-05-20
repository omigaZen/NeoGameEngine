struct MaterialUniform {
    tint: vec4<f32>,
    surface: vec4<f32>,
    emissive_occlusion: vec4<f32>,
    clearcoat_transmission: vec4<f32>,
    sheen: vec4<f32>,
    specular: vec4<f32>,
    anisotropy: vec4<f32>,
    iridescence: vec4<f32>,
    volume: vec4<f32>,
    volume_options: vec4<f32>,
    base_color_uv_transform_0: vec4<f32>,
    base_color_uv_transform_1: vec4<f32>,
    metallic_roughness_uv_transform_0: vec4<f32>,
    metallic_roughness_uv_transform_1: vec4<f32>,
    normal_uv_transform_0: vec4<f32>,
    normal_uv_transform_1: vec4<f32>,
};

struct RenderUniform {
    view_projection: mat4x4<f32>,
    ambient_color: vec4<f32>,
    directional_color: vec4<f32>,
    directional_direction: vec4<f32>,
    camera_position: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) world_position: vec3<f32>,
    @location(5) tangent: vec4<f32>,
};

struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) material: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> material: MaterialUniform;

@group(0) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(0) @binding(2)
var base_color_sampler: sampler;

@group(0) @binding(3)
var metallic_roughness_texture: texture_2d<f32>;

@group(0) @binding(4)
var normal_texture: texture_2d<f32>;

@group(0) @binding(16)
var metallic_roughness_sampler: sampler;

@group(0) @binding(17)
var normal_sampler: sampler;

@group(1) @binding(0)
var<uniform> render: RenderUniform;

fn safe_normalize(vector: vec3<f32>) -> vec3<f32> {
    return vector / max(length(vector), 0.0001);
}

fn transformed_texture_uv(
    uv0: vec2<f32>,
    uv1: vec2<f32>,
    transform_0: vec4<f32>,
    transform_1: vec4<f32>,
) -> vec2<f32> {
    var uv = uv0;
    if (transform_1.z > 0.5) {
        uv = uv1;
    }
    return vec2<f32>(
        transform_0.x * uv.x + transform_0.y * uv.y + transform_1.x,
        transform_0.z * uv.x + transform_0.w * uv.y + transform_1.y,
    );
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) tangent: vec4<f32>,
    @location(6) normal_matrix_0: vec4<f32>,
    @location(7) normal_matrix_1: vec4<f32>,
    @location(8) normal_matrix_2: vec4<f32>,
    @location(9) model_0: vec4<f32>,
    @location(10) model_1: vec4<f32>,
    @location(11) model_2: vec4<f32>,
    @location(12) model_3: vec4<f32>,
) -> VertexOutput {
    let normal_matrix = mat3x3<f32>(
        normal_matrix_0.xyz,
        normal_matrix_1.xyz,
        normal_matrix_2.xyz,
    );
    let model = mat4x4<f32>(
        model_0,
        model_1,
        model_2,
        model_3,
    );

    var out: VertexOutput;
    out.position = render.view_projection * model * vec4<f32>(position, 1.0);
    out.color = color;
    out.normal = normalize(normal_matrix * normal);
    out.uv = uv;
    out.uv1 = uv1;
    out.world_position = (model * vec4<f32>(position, 1.0)).xyz;
    out.tangent = vec4<f32>(normalize(normal_matrix * tangent.xyz), tangent.w);
    return out;
}

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front_facing: bool) -> GBufferOutput {
    let base_color_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.base_color_uv_transform_0,
        material.base_color_uv_transform_1,
    );
    let metallic_roughness_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.metallic_roughness_uv_transform_0,
        material.metallic_roughness_uv_transform_1,
    );
    let normal_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.normal_uv_transform_0,
        material.normal_uv_transform_1,
    );

    let sampled_color = textureSample(base_color_texture, base_color_sampler, base_color_uv);
    let sampled_surface =
        textureSample(metallic_roughness_texture, metallic_roughness_sampler, metallic_roughness_uv);
    var sampled_normal = textureSample(normal_texture, normal_sampler, normal_uv).xyz * 2.0 - 1.0;
    sampled_normal.x = sampled_normal.x * material.surface.z;
    sampled_normal.y = sampled_normal.y * material.surface.z;

    let base_color = in.color.rgb * material.tint.rgb * sampled_color.rgb;
    let alpha = in.color.a * material.tint.a * sampled_color.a;
    if (material.surface.w > 0.0 && alpha < material.surface.w) {
        discard;
    }

    let face_direction = select(-1.0, 1.0, is_front_facing);
    let vertex_normal = safe_normalize(in.normal);
    let tangent = safe_normalize(in.tangent.xyz - vertex_normal * dot(vertex_normal, in.tangent.xyz));
    let bitangent = cross(vertex_normal, tangent) * in.tangent.w;
    let normal = safe_normalize(
        (
            tangent * sampled_normal.x +
            bitangent * sampled_normal.y +
            vertex_normal * sampled_normal.z
        ) * face_direction,
    );
    let roughness = clamp(material.surface.x * sampled_surface.g, 0.04, 1.0);
    let metallic = clamp(material.surface.y * sampled_surface.b, 0.0, 1.0);

    var out: GBufferOutput;
    out.albedo = vec4<f32>(base_color, alpha);
    out.normal = vec4<f32>(normal * 0.5 + vec3<f32>(0.5), 1.0);
    out.material = vec4<f32>(metallic, roughness, material.surface.w, 1.0);
    return out;
}
