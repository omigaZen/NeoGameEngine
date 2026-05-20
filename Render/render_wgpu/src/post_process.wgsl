struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

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
    return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
