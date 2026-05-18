use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use engine_graphics::Color;
use engine_renderer::{graph::*, prelude::*};

struct CountingPass {
    calls: Arc<AtomicUsize>,
}

impl RenderGraphExtension for CountingPass {
    fn name(&self) -> &str {
        "counting_pass"
    }

    fn build(
        &self,
        ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        graph
            .add_pass("counting_pass")
            .read_texture(ctx.main_color(), TextureReadUsage::Sampled)
            .color_attachment(ctx.main_color(), ColorAttachmentOps::load_store())
            .execute(|ctx| {
                ctx.push_debug_group("counting_pass");
                ctx.pop_debug_group();
                Ok(())
            });
        Ok(())
    }
}

fn main() -> Result<(), RendererError> {
    let mut renderer = Renderer::new_headless(RendererConfig::default());
    let mesh = renderer.create_mesh(triangle_mesh_desc())?;
    let texture = renderer.create_texture(TextureDesc {
        label: Some("white"),
        dimension: TextureDimension::D2,
        width: 1,
        height: 1,
        depth_or_layers: 1,
        mip_levels: 1,
        samples: 1,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        initial_data: Some(TextureInitialData {
            bytes: &[255, 255, 255, 255],
            bytes_per_row: 4,
            rows_per_image: 1,
        }),
    })?;
    let material = renderer.create_standard_material(StandardMaterialDesc {
        label: Some("facade_standard".to_owned()),
        domain: MaterialDomain::Opaque,
        base_color: Color::WHITE,
        base_color_texture: Some(texture),
        normal_texture: None,
        metallic_roughness_texture: None,
        occlusion_texture: None,
        emissive_texture: None,
        metallic: 0.0,
        roughness: 0.65,
        emissive: Vec3::ZERO,
        alpha_mode: AlphaMode::Opaque,
        double_sided: false,
        receive_shadows: true,
        cast_shadows: true,
    })?;
    let scene = renderer.create_scene(SceneDesc {
        label: Some("facade_scene".to_owned()),
        max_objects_hint: Some(1),
        max_lights_hint: Some(1),
        enable_gpu_culling: false,
        enable_occlusion_culling: false,
    })?;
    renderer.edit_scene(scene, |scene| {
        scene.spawn(RenderObjectDesc {
            label: Some("triangle".to_owned()),
            mesh,
            materials: vec![material],
            transform: IDENTITY_MAT4,
            visibility: VisibilityFlags::CAMERA | VisibilityFlags::SHADOW,
            flags: ObjectFlags::STATIC | ObjectFlags::CAST_SHADOW | ObjectFlags::RECEIVE_SHADOW,
            ..RenderObjectDesc::default()
        });
        scene
            .add_light(LightDesc::Directional(DirectionalLightDesc {
                label: Some("sun".to_owned()),
                direction: Vec3::new(-0.3, -1.0, -0.2),
                color: Color::WHITE,
                illuminance_lux: 80_000.0,
                shadow: None,
                layer_mask: RenderLayerMask::all(),
            }))
            .unwrap();
    })?;

    let extension_calls = Arc::new(AtomicUsize::new(0));
    let counting_pass = renderer.register_graph_extension(CountingPass {
        calls: extension_calls.clone(),
    })?;
    let mut frame = renderer.begin_frame(FrameInput {
        delta_time: 1.0 / 60.0,
        absolute_time: 0.0,
        frame_index_override: None,
        wait_for_gpu: false,
    })?;
    frame.render_view(ViewDesc {
        label: Some("main_view".to_owned()),
        scene,
        camera: CameraDesc {
            label: Some("main_camera".to_owned()),
            transform: IDENTITY_MAT4,
            projection: Projection::Perspective {
                vertical_fov: 60.0_f32.to_radians(),
                aspect: 16.0 / 9.0,
                near: 0.05,
                far: Some(100.0),
                reverse_z: false,
            },
            exposure: Exposure::Auto,
            clear: ClearOptions::ColorDepth(Color::BLACK),
            viewport: None,
            scissor: None,
            jitter: None,
            previous_view_proj: None,
            flags: CameraFlags::MAIN,
        },
        target: RenderTarget::MainSurface,
        render_path: RenderPath::ForwardPlus,
        quality: ViewQualitySettings::high(),
        layers: RenderLayerMask::all(),
        graph_extensions: vec![counting_pass],
    })?;
    let stats = frame.finish()?;

    assert_eq!(stats.visible_objects, 1);
    assert_eq!(extension_calls.load(Ordering::SeqCst), 1);
    println!(
        "Render facade use case: draws={} graph_passes={}",
        stats.draw_calls, stats.graph.pass_count
    );
    Ok(())
}

fn triangle_mesh_desc() -> MeshDesc<'static> {
    const VERTICES: &[u8] = &[0; 96];
    MeshDesc {
        label: Some("triangle"),
        vertex_layout: VertexLayout {
            streams: vec![VertexStreamLayout {
                stride: 32,
                step: VertexStepMode::Vertex,
                attributes: vec![
                    VertexAttribute {
                        semantic: VertexSemantic::Position,
                        format: VertexFormat::Float32x3,
                        offset: 0,
                    },
                    VertexAttribute {
                        semantic: VertexSemantic::Normal,
                        format: VertexFormat::Float32x3,
                        offset: 12,
                    },
                    VertexAttribute {
                        semantic: VertexSemantic::TexCoord(0),
                        format: VertexFormat::Float32x2,
                        offset: 24,
                    },
                ],
            }],
        },
        vertices: VertexData::Interleaved(VERTICES),
        indices: Some(IndexData::U16(&[0, 1, 2])),
        submeshes: vec![SubMeshDesc {
            index_range: 0..3,
            vertex_range: 0..3,
            material_slot: 0,
            bounds: Bounds3::new(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0)),
        }],
        bounds: Bounds3::new(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 0.0)),
        usage: MeshUsage::STATIC,
        flags: MeshFlags::default(),
        skin: None,
        morph_targets: Vec::new(),
        meshlets: None,
    }
}
