struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

struct PostProcessUniform {
    texel_size_and_flags: vec4<f32>,
    color_grade_flags: vec4<f32>,
    effect_flags: vec4<f32>,
    screen_space_flags: vec4<f32>,
};

@group(0) @binding(2)
var<uniform> post_process: PostProcessUniform;

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

fn luma(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

fn apply_fxaa(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let texel = post_process.texel_size_and_flags.xy;
    let north = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(0.0, -texel.y), 0.0);
    let south = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(0.0, texel.y), 0.0);
    let east = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(texel.x, 0.0), 0.0);
    let west = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(-texel.x, 0.0), 0.0);
    let center_luma = luma(center.rgb);
    let min_luma = min(center_luma, min(min(luma(north.rgb), luma(south.rgb)), min(luma(east.rgb), luma(west.rgb))));
    let max_luma = max(center_luma, max(max(luma(north.rgb), luma(south.rgb)), max(luma(east.rgb), luma(west.rgb))));
    let contrast = max_luma - min_luma;
    let blended = (north + south + east + west) * 0.25;
    let edge_weight = smoothstep(0.03125, 0.125, contrast);
    return mix(center, blended, edge_weight);
}

fn bloom_source(color: vec3<f32>) -> vec3<f32> {
    let brightness = luma(color);
    let weight = smoothstep(1.0, 2.0, brightness);
    return color * weight;
}

fn apply_bloom(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let texel = post_process.texel_size_and_flags.xy;
    let north = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(0.0, -texel.y * 2.0), 0.0);
    let south = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(0.0, texel.y * 2.0), 0.0);
    let east = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(texel.x * 2.0, 0.0), 0.0);
    let west = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(-texel.x * 2.0, 0.0), 0.0);
    let bloom =
        bloom_source(center.rgb) * 0.35 +
        bloom_source(north.rgb) * 0.1625 +
        bloom_source(south.rgb) * 0.1625 +
        bloom_source(east.rgb) * 0.1625 +
        bloom_source(west.rgb) * 0.1625;
    return vec4<f32>(center.rgb + bloom * 0.22, center.a);
}

fn apply_color_grading(color: vec3<f32>) -> vec3<f32> {
    let contrasted = (color - vec3<f32>(0.5)) * 1.08 + vec3<f32>(0.5);
    let graded = contrasted * vec3<f32>(1.035, 1.0, 0.965) + vec3<f32>(0.012, 0.0, -0.004);
    return clamp(graded, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_taa_resolve(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let texel = post_process.texel_size_and_flags.xy;
    let a = textureSampleLevel(source_texture, source_sampler, uv + texel * vec2<f32>(0.5, 0.5), 0.0);
    let b = textureSampleLevel(source_texture, source_sampler, uv + texel * vec2<f32>(-0.5, 0.5), 0.0);
    let c = textureSampleLevel(source_texture, source_sampler, uv + texel * vec2<f32>(0.5, -0.5), 0.0);
    let d = textureSampleLevel(source_texture, source_sampler, uv + texel * vec2<f32>(-0.5, -0.5), 0.0);
    let neighborhood = (a + b + c + d) * 0.25;
    return mix(center, neighborhood, 0.18);
}

fn apply_motion_blur(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let texel = post_process.texel_size_and_flags.xy;
    let direction = normalize(vec2<f32>(uv.x - 0.5, 0.18));
    let a = textureSampleLevel(source_texture, source_sampler, uv - direction * texel * 1.5, 0.0);
    let b = textureSampleLevel(source_texture, source_sampler, uv + direction * texel * 1.5, 0.0);
    return center * 0.72 + (a + b) * 0.14;
}

fn apply_ssr(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let reflected_uv = clamp(vec2<f32>(uv.x, 1.0 - uv.y) * vec2<f32>(1.0, 0.92) + vec2<f32>(0.0, 0.04), vec2<f32>(0.0), vec2<f32>(1.0));
    let reflected = textureSampleLevel(source_texture, source_sampler, reflected_uv, 0.0);
    let reflectance = smoothstep(0.65, 1.45, luma(center.rgb));
    return vec4<f32>(mix(center.rgb, reflected.rgb, reflectance * 0.08), center.a);
}

fn apply_depth_of_field(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let texel = post_process.texel_size_and_flags.xy;
    let coc = smoothstep(0.18, 0.58, distance(uv, vec2<f32>(0.5, 0.5)));
    let a = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(texel.x * 2.0, texel.y * 2.0), 0.0);
    let b = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(-texel.x * 2.0, texel.y * 2.0), 0.0);
    let c = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(texel.x * 2.0, -texel.y * 2.0), 0.0);
    let d = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(-texel.x * 2.0, -texel.y * 2.0), 0.0);
    let blurred = (a + b + c + d) * 0.25;
    return mix(center, blurred, coc * 0.35);
}

fn apply_ssao(uv: vec2<f32>, center: vec4<f32>) -> vec4<f32> {
    let texel = post_process.texel_size_and_flags.xy;
    let north = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(0.0, -texel.y), 0.0);
    let south = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(0.0, texel.y), 0.0);
    let east = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(texel.x, 0.0), 0.0);
    let west = textureSampleLevel(source_texture, source_sampler, uv + vec2<f32>(-texel.x, 0.0), 0.0);
    let neighbor_luma = (luma(north.rgb) + luma(south.rgb) + luma(east.rgb) + luma(west.rgb)) * 0.25;
    let local_contrast = abs(luma(center.rgb) - neighbor_luma);
    let occlusion = smoothstep(0.08, 0.35, local_contrast) * 0.18;
    return vec4<f32>(center.rgb * (1.0 - occlusion), center.a);
}

fn apply_hdr_exposure(center: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(center.rgb * 1.12, center.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = textureSampleLevel(source_texture, source_sampler, in.uv, 0.0);
    var color = center;
    if (post_process.screen_space_flags.y > 0.5) {
        color = apply_hdr_exposure(color);
    }
    if (post_process.texel_size_and_flags.w > 0.5) {
        color = apply_bloom(in.uv, color);
    }
    if (post_process.screen_space_flags.x > 0.5) {
        color = apply_ssao(in.uv, color);
    }
    if (post_process.effect_flags.x > 0.5) {
        color = apply_taa_resolve(in.uv, color);
    }
    if (post_process.effect_flags.y > 0.5) {
        color = apply_motion_blur(in.uv, color);
    }
    if (post_process.effect_flags.z > 0.5) {
        color = apply_ssr(in.uv, color);
    }
    if (post_process.effect_flags.w > 0.5) {
        color = apply_depth_of_field(in.uv, color);
    }
    if (post_process.texel_size_and_flags.z > 0.5) {
        color = apply_fxaa(in.uv, color);
    }
    var mapped = color.rgb / (color.rgb + vec3<f32>(1.0));
    if (post_process.color_grade_flags.x > 0.5) {
        mapped = apply_color_grading(mapped);
    }
    let gamma_corrected = pow(mapped, vec3<f32>(1.0 / 2.2));
    return vec4<f32>(gamma_corrected, color.a);
}
