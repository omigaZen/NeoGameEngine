struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var gbuffer_albedo: texture_2d<f32>;

@group(0) @binding(1)
var gbuffer_normal: texture_2d<f32>;

@group(0) @binding(2)
var gbuffer_material: texture_2d<f32>;

@group(0) @binding(3)
var gbuffer_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var position = vec2<f32>(-1.0, 1.0);
    if (vertex_index == 0u) {
        position = vec2<f32>(-1.0, -3.0);
    } else if (vertex_index == 1u) {
        position = vec2<f32>(3.0, 1.0);
    }

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSampleLevel(gbuffer_albedo, gbuffer_sampler, in.uv, 0.0);
    let encoded_normal = textureSampleLevel(gbuffer_normal, gbuffer_sampler, in.uv, 0.0).xyz;
    let material = textureSampleLevel(gbuffer_material, gbuffer_sampler, in.uv, 0.0);
    let normal = normalize(encoded_normal * 2.0 - vec3<f32>(1.0));
    let light_dir = normalize(vec3<f32>(0.35, 0.75, 0.45));
    let diffuse = max(dot(normal, light_dir), 0.0);
    let ambient = 0.08 + 0.08 * (1.0 - material.y);
    let lit = albedo.rgb * (ambient + diffuse * (1.0 - material.x * 0.25));
    return vec4<f32>(lit, albedo.a);
}
