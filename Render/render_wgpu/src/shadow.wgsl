struct ShadowUniform {
    shadow_view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> shadow: ShadowUniform;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(9) model_0: vec4<f32>,
    @location(10) model_1: vec4<f32>,
    @location(11) model_2: vec4<f32>,
    @location(12) model_3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(
        model_0,
        model_1,
        model_2,
        model_3,
    );

    return shadow.shadow_view_projection * model * vec4<f32>(position, 1.0);
}
