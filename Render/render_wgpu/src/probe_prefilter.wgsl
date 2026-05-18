const SAMPLE_COUNT: u32 = 32u;
const PI: f32 = 3.14159265359;

struct ProbePrefilterUniform {
    face_roughness_size: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> probe: ProbePrefilterUniform;

@group(0) @binding(1)
var source_environment: texture_cube<f32>;

@group(0) @binding(2)
var source_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(vertex_index) - 1);
    let y = f32(i32(vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x * 3.0, y * 3.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let face = u32(probe.face_roughness_size.x + 0.5);
    let roughness = clamp(probe.face_roughness_size.y, 0.0, 1.0);
    let size = max(probe.face_roughness_size.z, 1.0);
    let uv = position.xy / size * 2.0 - vec2<f32>(1.0);
    let normal = cube_direction(face, uv);

    if (roughness <= 0.001) {
        return textureSampleLevel(source_environment, source_sampler, normal, 0.0);
    }

    var color = vec3<f32>(0.0);
    var total_weight = 0.0;
    let view = normal;

    for (var sample_index = 0u; sample_index < SAMPLE_COUNT; sample_index = sample_index + 1u) {
        let xi = hammersley(sample_index, SAMPLE_COUNT);
        let half_dir = importance_sample_ggx_world(xi, roughness, normal);
        let view_dot_half = max(dot(view, half_dir), 0.0);
        let light = normalize(2.0 * view_dot_half * half_dir - view);
        let n_dot_light = max(dot(normal, light), 0.0);

        if (n_dot_light > 0.0) {
            color += textureSampleLevel(source_environment, source_sampler, light, 0.0).rgb * n_dot_light;
            total_weight += n_dot_light;
        }
    }

    if (total_weight > 0.0) {
        color = color / total_weight;
    } else {
        color = textureSampleLevel(source_environment, source_sampler, normal, 0.0).rgb;
    }

    return vec4<f32>(color, 1.0);
}

fn cube_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let forward = cube_face_forward(face);
    let up = cube_face_up(face);
    let right = normalize(cross(forward, up));
    return normalize(forward + right * uv.x - up * uv.y);
}

fn cube_face_forward(face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3<f32>(1.0, 0.0, 0.0); }
        case 1u: { return vec3<f32>(-1.0, 0.0, 0.0); }
        case 2u: { return vec3<f32>(0.0, 1.0, 0.0); }
        case 3u: { return vec3<f32>(0.0, -1.0, 0.0); }
        case 4u: { return vec3<f32>(0.0, 0.0, 1.0); }
        default: { return vec3<f32>(0.0, 0.0, -1.0); }
    }
}

fn cube_face_up(face: u32) -> vec3<f32> {
    switch face {
        case 2u: { return vec3<f32>(0.0, 0.0, 1.0); }
        case 3u: { return vec3<f32>(0.0, 0.0, -1.0); }
        default: { return vec3<f32>(0.0, -1.0, 0.0); }
    }
}

fn hammersley(index: u32, sample_count: u32) -> vec2<f32> {
    return vec2<f32>(f32(index) / f32(max(sample_count, 1u)), radical_inverse_vdc(index));
}

fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064e-10;
}

fn importance_sample_ggx(xi: vec2<f32>, roughness: f32) -> vec3<f32> {
    let alpha = max(roughness, 0.001) * max(roughness, 0.001);
    let phi = 2.0 * PI * xi.x;
    let cos_theta = sqrt(max((1.0 - xi.y) / (1.0 + (alpha * alpha - 1.0) * xi.y), 0.0));
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    return vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);
}

fn importance_sample_ggx_world(xi: vec2<f32>, roughness: f32, normal: vec3<f32>) -> vec3<f32> {
    let half_dir = importance_sample_ggx(xi, roughness);
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(normal.z) < 0.999);
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    return normalize(tangent * half_dir.x + bitangent * half_dir.y + normal * half_dir.z);
}
