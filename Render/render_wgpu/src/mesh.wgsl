const MAX_POINT_LIGHTS: u32 = 4u;
const MAX_SPOT_LIGHTS: u32 = 4u;
const MAX_DIRECTIONAL_SHADOW_CASCADES: u32 = 4u;
const POINT_SHADOW_FACE_COUNT: u32 = 6u;
const MAX_POINT_SHADOW_FACES: u32 = 24u;
const MAX_ENVIRONMENT_PROBES: u32 = 4u;
const PI: f32 = 3.14159265359;

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
    emissive_uv_transform_0: vec4<f32>,
    emissive_uv_transform_1: vec4<f32>,
    occlusion_uv_transform_0: vec4<f32>,
    occlusion_uv_transform_1: vec4<f32>,
    clearcoat_uv_transform_0: vec4<f32>,
    clearcoat_uv_transform_1: vec4<f32>,
    clearcoat_roughness_uv_transform_0: vec4<f32>,
    clearcoat_roughness_uv_transform_1: vec4<f32>,
    clearcoat_normal_uv_transform_0: vec4<f32>,
    clearcoat_normal_uv_transform_1: vec4<f32>,
    sheen_color_uv_transform_0: vec4<f32>,
    sheen_color_uv_transform_1: vec4<f32>,
    sheen_roughness_uv_transform_0: vec4<f32>,
    sheen_roughness_uv_transform_1: vec4<f32>,
    transmission_uv_transform_0: vec4<f32>,
    transmission_uv_transform_1: vec4<f32>,
    specular_uv_transform_0: vec4<f32>,
    specular_uv_transform_1: vec4<f32>,
    specular_color_uv_transform_0: vec4<f32>,
    specular_color_uv_transform_1: vec4<f32>,
    anisotropy_uv_transform_0: vec4<f32>,
    anisotropy_uv_transform_1: vec4<f32>,
    iridescence_uv_transform_0: vec4<f32>,
    iridescence_uv_transform_1: vec4<f32>,
    iridescence_thickness_uv_transform_0: vec4<f32>,
    iridescence_thickness_uv_transform_1: vec4<f32>,
    thickness_uv_transform_0: vec4<f32>,
    thickness_uv_transform_1: vec4<f32>,
};

struct RenderUniform {
    view_projection: mat4x4<f32>,
    ambient_color: vec4<f32>,
    directional_color: vec4<f32>,
    directional_direction: vec4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
    directional_shadow_view_projections: array<mat4x4<f32>, 4>,
    shadow_options: vec4<f32>,
    directional_shadow_splits: vec4<f32>,
    spot_shadow_view_projections: array<mat4x4<f32>, 4>,
    spot_shadow_options: array<vec4<f32>, 4>,
    point_shadow_view_projections: array<mat4x4<f32>, 24>,
    point_shadow_options: array<vec4<f32>, 4>,
    environment_diffuse: vec4<f32>,
    environment_specular: vec4<f32>,
    environment_probe_options: vec4<f32>,
    environment_probe_positions_weights: array<vec4<f32>, 4>,
    environment_probe_box_mins: array<vec4<f32>, 4>,
    environment_probe_box_maxs: array<vec4<f32>, 4>,
    point_light_count: vec4<u32>,
    point_light_positions: array<vec4<f32>, 4>,
    point_light_colors: array<vec4<f32>, 4>,
    spot_light_count: vec4<u32>,
    spot_light_positions: array<vec4<f32>, 4>,
    spot_light_directions: array<vec4<f32>, 4>,
    spot_light_colors: array<vec4<f32>, 4>,
    spot_light_angles: array<vec4<f32>, 4>,
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

@group(0) @binding(5)
var emissive_texture: texture_2d<f32>;

@group(0) @binding(6)
var occlusion_texture: texture_2d<f32>;

@group(0) @binding(7)
var clearcoat_texture: texture_2d<f32>;

@group(0) @binding(8)
var clearcoat_roughness_texture: texture_2d<f32>;

@group(0) @binding(9)
var sheen_color_texture: texture_2d<f32>;

@group(0) @binding(10)
var sheen_roughness_texture: texture_2d<f32>;

@group(0) @binding(11)
var transmission_texture: texture_2d<f32>;

@group(0) @binding(12)
var specular_texture: texture_2d<f32>;

@group(0) @binding(13)
var specular_color_texture: texture_2d<f32>;

@group(0) @binding(14)
var anisotropy_texture: texture_2d<f32>;

@group(0) @binding(15)
var optical_extension_texture: texture_2d<f32>;

@group(0) @binding(29)
var clearcoat_normal_texture: texture_2d<f32>;

@group(0) @binding(16)
var metallic_roughness_sampler: sampler;

@group(0) @binding(17)
var normal_sampler: sampler;

@group(0) @binding(18)
var emissive_sampler: sampler;

@group(0) @binding(19)
var occlusion_sampler: sampler;

@group(0) @binding(20)
var clearcoat_sampler: sampler;

@group(0) @binding(21)
var clearcoat_roughness_sampler: sampler;

@group(0) @binding(22)
var sheen_color_sampler: sampler;

@group(0) @binding(23)
var sheen_roughness_sampler: sampler;

@group(0) @binding(24)
var transmission_sampler: sampler;

@group(0) @binding(25)
var specular_sampler: sampler;

@group(0) @binding(26)
var specular_color_sampler: sampler;

@group(0) @binding(27)
var anisotropy_sampler: sampler;

@group(0) @binding(28)
var optical_extension_sampler: sampler;

@group(0) @binding(30)
var clearcoat_normal_sampler: sampler;

@group(1) @binding(0)
var<uniform> render: RenderUniform;

@group(1) @binding(1)
var environment_texture_0: texture_cube<f32>;

@group(1) @binding(2)
var environment_sampler: sampler;

@group(1) @binding(3)
var environment_brdf_lut_texture: texture_2d<f32>;

@group(1) @binding(5)
var environment_texture_1: texture_cube<f32>;

@group(1) @binding(6)
var environment_texture_2: texture_cube<f32>;

@group(1) @binding(7)
var environment_texture_3: texture_cube<f32>;

@group(2) @binding(0)
var shadow_texture: texture_depth_2d_array;

@group(2) @binding(1)
var shadow_sampler: sampler_comparison;

@group(2) @binding(2)
var spot_shadow_texture: texture_depth_2d_array;

@group(2) @binding(3)
var point_shadow_texture: texture_depth_2d_array;

fn safe_normalize(vector: vec3<f32>) -> vec3<f32> {
    return vector / max(length(vector), 0.0001);
}

fn non_zero(value: f32) -> f32 {
    if (abs(value) >= 0.0001) {
        return value;
    }
    if (value < 0.0) {
        return -0.0001;
    }
    return 0.0001;
}

fn parallax_correct_environment_direction(
    probe_index: u32,
    world_position: vec3<f32>,
    direction: vec3<f32>,
) -> vec3<f32> {
    let box_min = render.environment_probe_box_mins[probe_index];
    if (box_min.w < 0.5) {
        return direction;
    }

    let box_max = render.environment_probe_box_maxs[probe_index].xyz;
    let probe_position = render.environment_probe_positions_weights[probe_index].xyz;
    let safe_direction = vec3<f32>(
        non_zero(direction.x),
        non_zero(direction.y),
        non_zero(direction.z),
    );
    let t0 = (box_min.xyz - world_position) / safe_direction;
    let t1 = (box_max - world_position) / safe_direction;
    let tmax = max(t0, t1);
    let distance = min(tmax.x, min(tmax.y, tmax.z));

    if (distance <= 0.0) {
        return direction;
    }

    let hit_position = world_position + direction * distance;
    return safe_normalize(hit_position - probe_position);
}

fn sample_environment_texture(index: u32, direction: vec3<f32>, lod: f32) -> vec3<f32> {
    switch index {
        case 1u: {
            return textureSampleLevel(environment_texture_1, environment_sampler, direction, lod).rgb;
        }
        case 2u: {
            return textureSampleLevel(environment_texture_2, environment_sampler, direction, lod).rgb;
        }
        case 3u: {
            return textureSampleLevel(environment_texture_3, environment_sampler, direction, lod).rgb;
        }
        default: {
            return textureSampleLevel(environment_texture_0, environment_sampler, direction, lod).rgb;
        }
    }
}

fn sample_environment(world_position: vec3<f32>, direction: vec3<f32>, lod: f32) -> vec3<f32> {
    let probe_count = min(u32(render.environment_probe_options.x), MAX_ENVIRONMENT_PROBES);
    if (probe_count == 0u) {
        return sample_environment_texture(0u, direction, lod);
    }

    var color = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var i = 0u; i < MAX_ENVIRONMENT_PROBES; i = i + 1u) {
        if (i < probe_count) {
            let weight = max(render.environment_probe_positions_weights[i].w, 0.0);
            let corrected_direction = parallax_correct_environment_direction(
                i,
                world_position,
                direction,
            );
            color += sample_environment_texture(i, corrected_direction, lod) * weight;
            weight_sum += weight;
        }
    }

    return color / max(weight_sum, 0.0001);
}

fn directional_shadow_factor(world_position: vec3<f32>) -> f32 {
    if (render.shadow_options.z < 0.5 || render.shadow_options.x <= 0.0) {
        return 1.0;
    }

    let cascade_count = min(u32(render.shadow_options.w), MAX_DIRECTIONAL_SHADOW_CASCADES);
    if (cascade_count == 0u) {
        return 1.0;
    }

    let view_depth = dot(world_position - render.camera_position.xyz, normalize(render.camera_forward.xyz));
    var cascade_index = cascade_count;
    for (var i = 0u; i < MAX_DIRECTIONAL_SHADOW_CASCADES; i = i + 1u) {
        if (i < cascade_count && view_depth <= render.directional_shadow_splits[i]) {
            cascade_index = i;
            break;
        }
    }
    if (cascade_index >= cascade_count) {
        return 1.0;
    }

    let shadow_clip = render.directional_shadow_view_projections[cascade_index] * vec4<f32>(world_position, 1.0);
    let shadow_ndc = shadow_clip.xyz / max(shadow_clip.w, 0.0001);
    if (
        shadow_ndc.x < -1.0 ||
        shadow_ndc.x > 1.0 ||
        shadow_ndc.y < -1.0 ||
        shadow_ndc.y > 1.0 ||
        shadow_ndc.z < 0.0 ||
        shadow_ndc.z > 1.0
    ) {
        return 1.0;
    }

    let shadow_uv = vec2<f32>(shadow_ndc.x * 0.5 + 0.5, 0.5 - shadow_ndc.y * 0.5);
    let visibility = textureSampleCompare(
        shadow_texture,
        shadow_sampler,
        shadow_uv,
        i32(cascade_index),
        shadow_ndc.z - render.shadow_options.y,
    );

    return mix(1.0, visibility, render.shadow_options.x);
}

fn spot_shadow_factor(world_position: vec3<f32>, light_index: u32) -> f32 {
    if (light_index >= MAX_SPOT_LIGHTS) {
        return 1.0;
    }
    let options = render.spot_shadow_options[light_index];
    if (
        options.z < 0.5 ||
        options.x <= 0.0
    ) {
        return 1.0;
    }

    let shadow_clip = render.spot_shadow_view_projections[light_index] * vec4<f32>(world_position, 1.0);
    let shadow_ndc = shadow_clip.xyz / max(shadow_clip.w, 0.0001);
    if (
        shadow_ndc.x < -1.0 ||
        shadow_ndc.x > 1.0 ||
        shadow_ndc.y < -1.0 ||
        shadow_ndc.y > 1.0 ||
        shadow_ndc.z < 0.0 ||
        shadow_ndc.z > 1.0
    ) {
        return 1.0;
    }

    let shadow_uv = vec2<f32>(shadow_ndc.x * 0.5 + 0.5, 0.5 - shadow_ndc.y * 0.5);
    let visibility = textureSampleCompare(
        spot_shadow_texture,
        shadow_sampler,
        shadow_uv,
        i32(light_index),
        shadow_ndc.z - options.y,
    );

    return mix(1.0, visibility, options.x);
}

fn point_shadow_face(direction: vec3<f32>) -> u32 {
    let absolute_direction = abs(direction);

    if (absolute_direction.x >= absolute_direction.y && absolute_direction.x >= absolute_direction.z) {
        if (direction.x >= 0.0) {
            return 0u;
        }
        return 1u;
    }

    if (absolute_direction.y >= absolute_direction.z) {
        if (direction.y >= 0.0) {
            return 2u;
        }
        return 3u;
    }

    if (direction.z >= 0.0) {
        return 4u;
    }
    return 5u;
}

fn point_shadow_factor(world_position: vec3<f32>, light_index: u32) -> f32 {
    if (light_index >= MAX_POINT_LIGHTS) {
        return 1.0;
    }
    let options = render.point_shadow_options[light_index];
    if (
        options.z < 0.5 ||
        options.x <= 0.0
    ) {
        return 1.0;
    }

    let point = render.point_light_positions[light_index];
    let light_to_fragment = world_position - point.xyz;
    let distance = length(light_to_fragment);
    if (distance > point.w) {
        return 1.0;
    }

    let face = point_shadow_face(light_to_fragment);
    let layer = light_index * POINT_SHADOW_FACE_COUNT + face;
    if (layer >= MAX_POINT_SHADOW_FACES) {
        return 1.0;
    }

    let shadow_clip = render.point_shadow_view_projections[layer] * vec4<f32>(world_position, 1.0);
    let shadow_ndc = shadow_clip.xyz / max(shadow_clip.w, 0.0001);
    if (
        shadow_ndc.x < -1.0 ||
        shadow_ndc.x > 1.0 ||
        shadow_ndc.y < -1.0 ||
        shadow_ndc.y > 1.0 ||
        shadow_ndc.z < 0.0 ||
        shadow_ndc.z > 1.0
    ) {
        return 1.0;
    }

    let shadow_uv = vec2<f32>(shadow_ndc.x * 0.5 + 0.5, 0.5 - shadow_ndc.y * 0.5);
    let visibility = textureSampleCompare(
        point_shadow_texture,
        shadow_sampler,
        shadow_uv,
        i32(layer),
        shadow_ndc.z - options.y,
    );

    return mix(1.0, visibility, options.x);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

fn fresnel_schlick_roughness(cos_theta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
    let max_reflectance = max(vec3<f32>(1.0 - roughness), f0);
    return f0 + (max_reflectance - f0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

fn thin_film_iridescence(n_dot_v: f32, thickness_nm: f32, factor: f32, ior: f32) -> vec3<f32> {
    let view_phase = (1.0 - clamp(n_dot_v, 0.0, 1.0)) * PI * 2.0;
    let phase = thickness_nm * 0.018 + view_phase + max(ior, 1.0) * 0.5;
    let film_color = 0.5 + 0.5 * cos(vec3<f32>(phase, phase + 2.094395, phase + 4.18879));
    return mix(vec3<f32>(1.0), max(film_color, vec3<f32>(0.0)), clamp(factor, 0.0, 1.0));
}

fn volume_attenuation(
    attenuation_color: vec3<f32>,
    thickness_factor: f32,
    attenuation_distance: f32,
) -> vec3<f32> {
    if (thickness_factor <= 0.0 || attenuation_distance <= 0.0) {
        return vec3<f32>(1.0);
    }

    let safe_color = max(attenuation_color, vec3<f32>(0.0001));
    return pow(safe_color, vec3<f32>(thickness_factor / max(attenuation_distance, 0.0001)));
}

fn sample_dispersion_environment(
    world_position: vec3<f32>,
    direction: vec3<f32>,
    tangent: vec3<f32>,
    lod: f32,
    dispersion: f32,
) -> vec3<f32> {
    let amount = clamp(dispersion, 0.0, 1.0) * 0.08;
    if (amount <= 0.0001) {
        return sample_environment(world_position, direction, lod);
    }

    let red_direction = safe_normalize(direction + tangent * amount);
    let blue_direction = safe_normalize(direction - tangent * amount);
    let red = sample_environment(world_position, red_direction, lod).r;
    let green = sample_environment(world_position, direction, lod).g;
    let blue = sample_environment(world_position, blue_direction, lod).b;
    return vec3<f32>(red, green, blue);
}

fn distribution_ggx(normal: vec3<f32>, half_dir: vec3<f32>, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denominator = n_dot_h2 * (alpha2 - 1.0) + 1.0;
    return alpha2 / max(PI * denominator * denominator, 0.0001);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let k = ((roughness + 1.0) * (roughness + 1.0)) / 8.0;
    return n_dot_v / max(n_dot_v * (1.0 - k) + k, 0.0001);
}

fn geometry_smith(
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    roughness: f32,
) -> f32 {
    let n_dot_v = max(dot(normal, view_dir), 0.0);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    return geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness);
}

fn pbr_light_contribution(
    normal: vec3<f32>,
    clearcoat_normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    radiance: vec3<f32>,
    base_color: vec3<f32>,
    roughness: f32,
    metallic: f32,
    f0: vec3<f32>,
    clearcoat_factor: f32,
    clearcoat_roughness: f32,
    sheen_color: vec3<f32>,
    sheen_roughness: f32,
    transmission: f32,
) -> vec3<f32> {
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let clearcoat_n_dot_l = max(dot(clearcoat_normal, light_dir), 0.0);
    if (n_dot_l <= 0.0 && clearcoat_n_dot_l <= 0.0) {
        return vec3<f32>(0.0);
    }

    let half_dir = safe_normalize(light_dir + view_dir);
    var base_lighting = vec3<f32>(0.0);
    if (n_dot_l > 0.0) {
        let fresnel = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), f0);
        let distribution = distribution_ggx(normal, half_dir, roughness);
        let geometry = geometry_smith(normal, view_dir, light_dir, roughness);
        let denominator = max(4.0 * max(dot(normal, view_dir), 0.0) * n_dot_l, 0.0001);
        let specular = (distribution * geometry * fresnel) / denominator;
        let diffuse =
            (vec3<f32>(1.0) - fresnel) * (1.0 - metallic) * (1.0 - transmission) * base_color / PI;
        let sheen_fresnel = pow(1.0 - clamp(dot(half_dir, view_dir), 0.0, 1.0), 5.0);
        let sheen = sheen_color * sheen_fresnel * (1.0 - sheen_roughness * 0.5) * (1.0 - metallic) * (1.0 - transmission) / PI;
        base_lighting = (diffuse + sheen + specular) * n_dot_l;
    }

    var clearcoat_lighting = vec3<f32>(0.0);
    if (clearcoat_factor > 0.0 && clearcoat_n_dot_l > 0.0) {
        let clearcoat_fresnel = fresnel_schlick(max(dot(half_dir, view_dir), 0.0), vec3<f32>(0.04));
        let clearcoat_distribution = distribution_ggx(clearcoat_normal, half_dir, clearcoat_roughness);
        let clearcoat_geometry = geometry_smith(clearcoat_normal, view_dir, light_dir, clearcoat_roughness);
        let clearcoat_denominator =
            max(4.0 * max(dot(clearcoat_normal, view_dir), 0.0) * clearcoat_n_dot_l, 0.0001);
        clearcoat_lighting =
            (clearcoat_distribution * clearcoat_geometry * clearcoat_fresnel) / clearcoat_denominator *
            clearcoat_factor *
            clearcoat_n_dot_l;
    }

    return (base_lighting + clearcoat_lighting) * radiance;
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

@fragment
fn fs_main(in: VertexOutput, @builtin(front_facing) is_front_facing: bool) -> @location(0) vec4<f32> {
    let base_color_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.base_color_uv_transform_0,
        material.base_color_uv_transform_1,
    );
    let sampled_color = textureSample(base_color_texture, base_color_sampler, base_color_uv);
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
    let emissive_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.emissive_uv_transform_0,
        material.emissive_uv_transform_1,
    );
    let occlusion_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.occlusion_uv_transform_0,
        material.occlusion_uv_transform_1,
    );
    let clearcoat_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.clearcoat_uv_transform_0,
        material.clearcoat_uv_transform_1,
    );
    let clearcoat_roughness_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.clearcoat_roughness_uv_transform_0,
        material.clearcoat_roughness_uv_transform_1,
    );
    let clearcoat_normal_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.clearcoat_normal_uv_transform_0,
        material.clearcoat_normal_uv_transform_1,
    );
    let sheen_color_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.sheen_color_uv_transform_0,
        material.sheen_color_uv_transform_1,
    );
    let sheen_roughness_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.sheen_roughness_uv_transform_0,
        material.sheen_roughness_uv_transform_1,
    );
    let transmission_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.transmission_uv_transform_0,
        material.transmission_uv_transform_1,
    );
    let specular_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.specular_uv_transform_0,
        material.specular_uv_transform_1,
    );
    let specular_color_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.specular_color_uv_transform_0,
        material.specular_color_uv_transform_1,
    );
    let anisotropy_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.anisotropy_uv_transform_0,
        material.anisotropy_uv_transform_1,
    );
    let iridescence_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.iridescence_uv_transform_0,
        material.iridescence_uv_transform_1,
    );
    let iridescence_thickness_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.iridescence_thickness_uv_transform_0,
        material.iridescence_thickness_uv_transform_1,
    );
    let thickness_uv = transformed_texture_uv(
        in.uv,
        in.uv1,
        material.thickness_uv_transform_0,
        material.thickness_uv_transform_1,
    );
    let sampled_surface =
        textureSample(metallic_roughness_texture, metallic_roughness_sampler, metallic_roughness_uv);
    var sampled_normal = textureSample(normal_texture, normal_sampler, normal_uv).xyz * 2.0 - 1.0;
    let sampled_emissive = textureSample(emissive_texture, emissive_sampler, emissive_uv).rgb;
    let sampled_occlusion = textureSample(occlusion_texture, occlusion_sampler, occlusion_uv).r;
    let sampled_clearcoat = textureSample(clearcoat_texture, clearcoat_sampler, clearcoat_uv).r;
    let sampled_clearcoat_roughness =
        textureSample(clearcoat_roughness_texture, clearcoat_roughness_sampler, clearcoat_roughness_uv).g;
    var sampled_clearcoat_normal =
        textureSample(clearcoat_normal_texture, clearcoat_normal_sampler, clearcoat_normal_uv).xyz * 2.0 - 1.0;
    let sampled_sheen_color = textureSample(sheen_color_texture, sheen_color_sampler, sheen_color_uv).rgb;
    let sampled_sheen_roughness =
        textureSample(sheen_roughness_texture, sheen_roughness_sampler, sheen_roughness_uv).a;
    let sampled_transmission = textureSample(transmission_texture, transmission_sampler, transmission_uv).r;
    let sampled_specular = textureSample(specular_texture, specular_sampler, specular_uv).a;
    let sampled_specular_color =
        textureSample(specular_color_texture, specular_color_sampler, specular_color_uv).rgb;
    let sampled_anisotropy = textureSample(anisotropy_texture, anisotropy_sampler, anisotropy_uv).b;
    let sampled_optical_extension = vec3<f32>(
        textureSample(optical_extension_texture, optical_extension_sampler, iridescence_uv).r,
        textureSample(optical_extension_texture, optical_extension_sampler, iridescence_thickness_uv).g,
        textureSample(optical_extension_texture, optical_extension_sampler, thickness_uv).b,
    );
    sampled_normal.x = sampled_normal.x * material.surface.z;
    sampled_normal.y = sampled_normal.y * material.surface.z;
    sampled_clearcoat_normal.x = sampled_clearcoat_normal.x * material.anisotropy.z;
    sampled_clearcoat_normal.y = sampled_clearcoat_normal.y * material.anisotropy.z;
    let base_color = in.color.rgb * material.tint.rgb * sampled_color.rgb;
    let alpha = in.color.a * material.tint.a * sampled_color.a;
    if (material.surface.w > 0.0 && alpha < material.surface.w) {
        discard;
    }
    if (material.volume_options.z > 0.5) {
        return vec4<f32>(base_color, alpha);
    }
    var roughness = clamp(material.surface.x * sampled_surface.g, 0.04, 1.0);
    var metallic = clamp(material.surface.y * sampled_surface.b, 0.0, 1.0);
    if (material.volume_options.w > 0.5) {
        let glossiness_factor = clamp(1.0 - material.surface.x, 0.0, 1.0);
        let glossiness = clamp(glossiness_factor * sampled_surface.a, 0.0, 1.0);
        roughness = clamp(1.0 - glossiness, 0.04, 1.0);
        metallic = 0.0;
    }
    let clearcoat_factor = clamp(material.clearcoat_transmission.x * sampled_clearcoat, 0.0, 1.0);
    let clearcoat_roughness =
        clamp(material.clearcoat_transmission.y * sampled_clearcoat_roughness, 0.04, 1.0);
    let transmission = clamp(material.clearcoat_transmission.z * sampled_transmission, 0.0, 1.0);
    let ior = max(material.clearcoat_transmission.w, 1.0001);
    let dielectric_f0_scalar = pow((ior - 1.0) / (ior + 1.0), 2.0);
    var specular_factor = clamp(material.specular.a * sampled_specular, 0.0, 1.0);
    var specular_color =
        clamp(material.specular.rgb * sampled_specular_color, vec3<f32>(0.0), vec3<f32>(1.0));
    var diffuse_brdf_color = base_color;
    if (material.volume_options.w > 0.5) {
        specular_factor = 1.0;
        specular_color =
            clamp(material.specular.rgb * sampled_specular_color, vec3<f32>(0.0), vec3<f32>(1.0));
        let max_specular = max(specular_color.r, max(specular_color.g, specular_color.b));
        diffuse_brdf_color = base_color * (1.0 - max_specular);
    }
    let sheen_color =
        clamp(material.sheen.rgb * sampled_sheen_color, vec3<f32>(0.0), vec3<f32>(1.0));
    let sheen_roughness = clamp(material.sheen.a * sampled_sheen_roughness, 0.0, 1.0);
    let occlusion = mix(1.0, sampled_occlusion, clamp(material.emissive_occlusion.a, 0.0, 1.0));
    let emissive = sampled_emissive * material.emissive_occlusion.rgb;
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
    let clearcoat_normal = safe_normalize(
        (
            tangent * sampled_clearcoat_normal.x +
            bitangent * sampled_clearcoat_normal.y +
            vertex_normal * sampled_clearcoat_normal.z
        ) * face_direction,
    );
    let anisotropy_strength = clamp(material.anisotropy.x * sampled_anisotropy, -1.0, 1.0);
    let anisotropy_rotation_sin = sin(material.anisotropy.y);
    let anisotropy_rotation_cos = cos(material.anisotropy.y);
    let anisotropy_tangent =
        safe_normalize(
            (tangent * anisotropy_rotation_cos + bitangent * anisotropy_rotation_sin) *
            face_direction,
        );
    let anisotropic_normal = safe_normalize(
        mix(
            normal,
            safe_normalize(normal + anisotropy_tangent * anisotropy_strength * 0.35),
            abs(anisotropy_strength),
        )
    );
    let anisotropic_roughness = clamp(roughness * (1.0 - abs(anisotropy_strength) * 0.35), 0.04, 1.0);
    var dielectric_f0 = vec3<f32>(dielectric_f0_scalar) * specular_factor * specular_color;
    if (material.volume_options.w > 0.5) {
        dielectric_f0 = specular_color;
    }
    let view_dir = safe_normalize(render.camera_position.xyz - in.world_position);
    let n_dot_v = max(dot(anisotropic_normal, view_dir), 0.0);
    let iridescence_factor = clamp(material.iridescence.x * sampled_optical_extension.r, 0.0, 1.0);
    let iridescence_thickness_min = max(material.iridescence.z, 0.0);
    let iridescence_thickness_max = max(material.iridescence.w, iridescence_thickness_min);
    let iridescence_thickness =
        mix(iridescence_thickness_min, iridescence_thickness_max, sampled_optical_extension.g);
    let iridescence_tint = thin_film_iridescence(
        n_dot_v,
        iridescence_thickness,
        iridescence_factor,
        material.iridescence.y,
    );
    let base_f0 = mix(dielectric_f0, diffuse_brdf_color, metallic);
    let f0 = mix(base_f0, max(base_f0 * iridescence_tint, vec3<f32>(0.02) * iridescence_tint), iridescence_factor);
    let environment_fresnel = fresnel_schlick_roughness(n_dot_v, f0, anisotropic_roughness);
    let reflection_dir = reflect(-view_dir, anisotropic_normal);
    let environment_max_lod = render.environment_specular.a;
    let environment_specular_lod = anisotropic_roughness * environment_max_lod;
    let environment_diffuse_sample =
        sample_environment(in.world_position, anisotropic_normal, environment_max_lod);
    let environment_specular_sample =
        sample_environment(in.world_position, reflection_dir, environment_specular_lod);
    let environment_brdf =
        textureSample(
            environment_brdf_lut_texture,
            environment_sampler,
            vec2<f32>(n_dot_v, anisotropic_roughness),
        ).rg;
    let environment_diffuse =
        render.environment_diffuse.xyz * environment_diffuse_sample * diffuse_brdf_color * (1.0 - metallic) * occlusion;
    let environment_specular =
        render.environment_specular.xyz * environment_specular_sample * (environment_fresnel * environment_brdf.x + vec3<f32>(environment_brdf.y)) * occlusion;
    let clearcoat_n_dot_v = max(dot(clearcoat_normal, view_dir), 0.0);
    let clearcoat_reflection_dir = reflect(-view_dir, clearcoat_normal);
    let clearcoat_environment_specular_sample =
        sample_environment(in.world_position, clearcoat_reflection_dir, clearcoat_roughness * environment_max_lod);
    let clearcoat_environment_specular =
        render.environment_specular.xyz * clearcoat_environment_specular_sample * fresnel_schlick(clearcoat_n_dot_v, vec3<f32>(0.04)) * clearcoat_factor * occlusion;
    let sheen_environment =
        render.environment_diffuse.xyz * environment_diffuse_sample * sheen_color * (1.0 - sheen_roughness * 0.5) * (1.0 - metallic) * (1.0 - transmission) * occlusion;
    var refraction_dir = -anisotropic_normal;
    let raw_refraction_dir = refract(-view_dir, anisotropic_normal, 1.0 / ior);
    if (length(raw_refraction_dir) > 0.0001) {
        refraction_dir = safe_normalize(raw_refraction_dir);
    }
    let transmission_environment_sample = sample_dispersion_environment(
        in.world_position,
        refraction_dir,
        anisotropy_tangent,
        roughness * environment_max_lod,
        material.volume_options.y,
    );
    let volume_filter = volume_attenuation(
        material.volume.rgb,
        material.volume.w * sampled_optical_extension.b,
        material.volume_options.x,
    );
    let transmission_environment =
        render.environment_diffuse.xyz * transmission_environment_sample * diffuse_brdf_color * volume_filter * transmission * occlusion;

    let light_dir = normalize(render.directional_direction.xyz);
    let shadow_factor = directional_shadow_factor(in.world_position);
    let directional_color = render.directional_color.xyz * shadow_factor;
    var lit_color = render.ambient_color.xyz * diffuse_brdf_color * occlusion;
    lit_color += environment_diffuse + environment_specular + clearcoat_environment_specular + sheen_environment + transmission_environment;
    lit_color += pbr_light_contribution(
        anisotropic_normal,
        clearcoat_normal,
        view_dir,
        light_dir,
        directional_color,
        diffuse_brdf_color,
        anisotropic_roughness,
        metallic,
        f0,
        clearcoat_factor,
        clearcoat_roughness,
        sheen_color,
        sheen_roughness,
        transmission,
    );
    let point_light_count = min(render.point_light_count.x, MAX_POINT_LIGHTS);

    for (var i = 0u; i < point_light_count; i = i + 1u) {
        let point = render.point_light_positions[i];
        let to_light = point.xyz - in.world_position;
        let distance = length(to_light);
        let point_light_dir = to_light / max(distance, 0.0001);
        let point_color = render.point_light_colors[i].xyz;
        let attenuation = max(1.0 - distance / max(point.w, 0.0001), 0.0);
        let shadow = point_shadow_factor(in.world_position, i);
        let attenuated_color = point_color * attenuation * attenuation * shadow;
        lit_color += pbr_light_contribution(
            anisotropic_normal,
            clearcoat_normal,
            view_dir,
            point_light_dir,
            attenuated_color,
            diffuse_brdf_color,
            anisotropic_roughness,
            metallic,
            f0,
            clearcoat_factor,
            clearcoat_roughness,
            sheen_color,
            sheen_roughness,
            transmission,
        );
    }

    let spot_light_count = min(render.spot_light_count.x, MAX_SPOT_LIGHTS);
    for (var i = 0u; i < spot_light_count; i = i + 1u) {
        let spot = render.spot_light_positions[i];
        let to_light = spot.xyz - in.world_position;
        let distance = length(to_light);
        let spot_light_dir = to_light / max(distance, 0.0001);
        let spot_direction = normalize(render.spot_light_directions[i].xyz);
        let spot_cos = dot(-spot_light_dir, spot_direction);
        let angles = render.spot_light_angles[i];
        let cone = smoothstep(angles.y, angles.x, spot_cos);
        let attenuation = max(1.0 - distance / max(spot.w, 0.0001), 0.0);
        let shadow = spot_shadow_factor(in.world_position, i);
        let attenuated_color =
            render.spot_light_colors[i].xyz * attenuation * attenuation * cone * shadow;
        lit_color += pbr_light_contribution(
            anisotropic_normal,
            clearcoat_normal,
            view_dir,
            spot_light_dir,
            attenuated_color,
            diffuse_brdf_color,
            anisotropic_roughness,
            metallic,
            f0,
            clearcoat_factor,
            clearcoat_roughness,
            sheen_color,
            sheen_roughness,
            transmission,
        );
    }

    return vec4<f32>(lit_color + emissive, alpha);
}
