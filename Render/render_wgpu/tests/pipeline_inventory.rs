use render_wgpu::MeshRenderer;

#[test]
fn mesh_renderer_static_pipeline_inventory_is_reported() {
    assert_eq!(MeshRenderer::STATIC_RENDER_PIPELINE_COUNT, 33);
}
