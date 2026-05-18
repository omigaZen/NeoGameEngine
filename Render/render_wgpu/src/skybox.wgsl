struct SkyboxUniform {
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    camera_forward: vec4<f32>,
    options: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) direction: vec3<f32>,
};

@group(0) @binding(1)
var environment_texture: texture_cube<f32>;

@group(0) @binding(2)
var environment_sampler: sampler;

@group(1) @binding(0)
var<uniform> skybox: SkyboxUniform;

fn safe_normalize(vector: vec3<f32>) -> vec3<f32> {
    return vector / max(length(vector), 0.0001);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    let clip = positions[vertex_index];
    let tan_half_fov = skybox.options.x;
    let aspect_ratio = skybox.options.y;

    var direction = skybox.camera_forward.xyz;
    if (tan_half_fov > 0.0) {
        direction = skybox.camera_right.xyz * clip.x * aspect_ratio * tan_half_fov +
            skybox.camera_up.xyz * clip.y * tan_half_fov +
            skybox.camera_forward.xyz;
    }

    var out: VertexOutput;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.direction = safe_normalize(direction);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color =
        textureSampleLevel(environment_texture, environment_sampler, in.direction, skybox.options.w).rgb *
        skybox.options.z;
    return vec4<f32>(color, 1.0);
}
