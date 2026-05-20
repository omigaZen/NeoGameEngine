use std::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use engine_graphics::Color;
use engine_platform::{
    ButtonState, InputEvent, KeyCode, PhysicalSize, Platform, PlatformApp, PlatformContext,
    PlatformError, PlatformEvent, PlatformResult, RunMode, WindowDesc, WindowEvent, WindowId,
};
use engine_renderer::prelude::*;
use engine_renderer::{
    BufferWriteUsage, GraphBufferDesc, GraphTextureDesc, RenderGraphBuilder, RenderGraphExtension,
    RenderGraphExtensionContext, TextureWriteUsage,
};
use platform_winit::WinitPlatform;

struct FacadeWindowApp {
    options: AppOptions,
    window: Option<WindowId>,
    renderer: Option<Renderer>,
    scene: Option<SceneHandle>,
    graph_export_extension: Option<RenderGraphExtensionHandle>,
    elapsed_seconds: f32,
    rendered_frames: u32,
    surface_readback_materialized: u32,
    surface_readback_texture: Option<TextureHandle>,
    last_stats: Option<FrameStats>,
}

#[derive(Clone, Debug, Default)]
struct AppOptions {
    smoke_frames: Option<u32>,
    wait_for_gpu: bool,
    print_stats: bool,
    require_gpu_time: bool,
    surface_readback: bool,
    require_surface_readback: bool,
    graph_export: bool,
    require_graph_export: bool,
}

impl FacadeWindowApp {
    fn new(options: AppOptions) -> Self {
        Self {
            options,
            window: None,
            renderer: None,
            scene: None,
            graph_export_extension: None,
            elapsed_seconds: 0.0,
            rendered_frames: 0,
            surface_readback_materialized: 0,
            surface_readback_texture: None,
            last_stats: None,
        }
    }
}

impl PlatformApp for FacadeWindowApp {
    fn on_resumed(&mut self, ctx: &mut dyn PlatformContext) -> PlatformResult<()> {
        if self.window.is_some() {
            return Ok(());
        }

        let window_id = ctx.create_window(WindowDesc {
            title: "Neo Renderer Facade Window Usecase".to_owned(),
            ..WindowDesc::default()
        })?;
        let window = ctx.window(window_id).ok_or(PlatformError::WindowNotFound)?;
        let mut renderer = block_on_ready(Renderer::with_surface(
            RendererConfig {
                backend: BackendPreference::Auto,
                preferred_render_path: RenderPath::ForwardPlus,
                debug_labels: true,
                gpu_profiling: true,
                ..RendererConfig::default()
            },
            window,
        ))
        .map_err(platform_error)?;
        let scene = build_scene(&mut renderer).map_err(platform_error)?;
        let graph_export_extension = if self.options.graph_export {
            Some(
                renderer
                    .register_graph_extension(FacadeWindowGraphExport)
                    .map_err(platform_error)?,
            )
        } else {
            None
        };

        self.window = Some(window_id);
        self.renderer = Some(renderer);
        self.scene = Some(scene);
        self.graph_export_extension = graph_export_extension;
        ctx.set_run_mode(RunMode::Poll);
        Ok(())
    }

    fn on_event(
        &mut self,
        ctx: &mut dyn PlatformContext,
        event: PlatformEvent,
    ) -> PlatformResult<()> {
        match event {
            PlatformEvent::Window { id, event } if Some(id) == self.window => match event {
                WindowEvent::CloseRequested => {
                    self.last_stats = None;
                    self.scene = None;
                    self.graph_export_extension = None;
                    self.renderer = None;
                    self.window = None;
                    ctx.destroy_window(id)?;
                    ctx.exit();
                }
                WindowEvent::Resized { size } => self.resize(size)?,
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    self.resize(new_inner_size)?;
                }
                _ => {}
            },
            PlatformEvent::Input { event, .. } => {
                if let InputEvent::Keyboard(keyboard) = event {
                    if keyboard.key == KeyCode::Escape && keyboard.state == ButtonState::Pressed {
                        ctx.exit();
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn on_update(
        &mut self,
        ctx: &mut dyn PlatformContext,
        dt: std::time::Duration,
    ) -> PlatformResult<()> {
        self.elapsed_seconds += dt.as_secs_f32();
        if let Some(window) = self.window {
            ctx.request_redraw(window);
        }
        Ok(())
    }

    fn on_redraw(
        &mut self,
        ctx: &mut dyn PlatformContext,
        window_id: WindowId,
    ) -> PlatformResult<()> {
        if Some(window_id) != self.window {
            return Ok(());
        }

        let Some(renderer) = &mut self.renderer else {
            return Ok(());
        };
        let Some(scene) = self.scene else {
            return Ok(());
        };
        let graph_extensions = self
            .graph_export_extension
            .map_or_else(Vec::new, |extension| vec![extension]);

        if self.options.surface_readback {
            poll_and_materialize_surface_readback(
                renderer,
                &mut self.surface_readback_materialized,
                &mut self.surface_readback_texture,
            )
            .map_err(platform_error)?;
            renderer.request_surface_frame_readback_next_frame();
        }

        let mut frame = renderer
            .begin_frame(FrameInput {
                delta_time: 1.0 / 60.0,
                absolute_time: f64::from(self.elapsed_seconds),
                frame_index_override: None,
                wait_for_gpu: self.options.wait_for_gpu,
            })
            .map_err(platform_error)?;
        frame
            .render_view(ViewDesc {
                label: Some("facade_window_view".to_owned()),
                scene,
                camera: CameraDesc {
                    label: Some("facade_window_camera".to_owned()),
                    transform: IDENTITY_MAT4,
                    projection: Projection::Orthographic {
                        width: 3.2,
                        height: 2.0,
                        near: -10.0,
                        far: 10.0,
                        reverse_z: false,
                    },
                    exposure: Exposure::Manual(1.0),
                    clear: ClearOptions::ColorDepth(Color::rgb(0.025, 0.035, 0.045)),
                    viewport: None,
                    scissor: None,
                    jitter: None,
                    previous_view_proj: None,
                    flags: CameraFlags::MAIN,
                },
                target: RenderTarget::MainSurface,
                render_path: RenderPath::ForwardPlus,
                quality: ViewQualitySettings {
                    bloom: false,
                    taa: false,
                    ..ViewQualitySettings::high()
                },
                layers: RenderLayerMask::all(),
                graph_extensions,
            })
            .map_err(platform_error)?;
        let stats = frame.finish().map_err(platform_error)?;
        if self.options.surface_readback {
            poll_and_materialize_surface_readback(
                renderer,
                &mut self.surface_readback_materialized,
                &mut self.surface_readback_texture,
            )
            .map_err(platform_error)?;
        }
        let surface_readback_frame_outputs = backend_surface_readback_output_count(&stats);
        if let Some(window) = ctx.window(window_id) {
            window.set_title(&stats_window_title(
                &stats,
                self.surface_readback_materialized,
                surface_readback_frame_outputs,
            ));
        }
        self.rendered_frames = self.rendered_frames.saturating_add(1);
        if let Some(target_frames) = self.options.smoke_frames {
            if self.rendered_frames >= target_frames {
                if self.options.require_gpu_time
                    && stats.gpu_profiler_enabled
                    && stats.gpu_time_ms.is_none()
                {
                    return Err(PlatformError::BackendError(
                        "GPU profiling was enabled but the surface smoke run did not report GPU time"
                            .to_owned(),
                    ));
                }
                if self.options.print_stats {
                    println!(
                        "{}",
                        stats_summary(
                            &stats,
                            renderer.surface_frame_readback_pending(),
                            renderer.surface_frame_readback_available(),
                            self.surface_readback_materialized,
                            surface_readback_frame_outputs,
                        )
                    );
                }
                if self.options.require_graph_export && !graph_export_requirements_met(&stats) {
                    return Err(PlatformError::BackendError(
                        "graph export was required but the expected public graph exports were not promoted"
                            .to_owned(),
                    ));
                }
                if self.options.require_surface_readback
                    && self.surface_readback_materialized == 0
                    && surface_readback_frame_outputs == 0
                {
                    return Err(PlatformError::BackendError(
                        "surface readback was required but no durable public texture was materialized or reported as a frame public output"
                            .to_owned(),
                    ));
                }
                ctx.exit();
            }
        }
        self.last_stats = Some(stats);
        Ok(())
    }
}

impl FacadeWindowApp {
    fn resize(&mut self, size: PhysicalSize<u32>) -> PlatformResult<()> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if let Some(renderer) = &mut self.renderer {
            renderer
                .resize_surface(size.width, size.height)
                .map_err(platform_error)?;
        }
        Ok(())
    }
}

struct FacadeWindowGraphExport;

impl RenderGraphExtension for FacadeWindowGraphExport {
    fn name(&self) -> &str {
        "facade_window_graph_export"
    }

    fn build(
        &self,
        ctx: &RenderGraphExtensionContext,
        graph: &mut RenderGraphBuilder<'_>,
    ) -> Result<(), RendererError> {
        let texture = graph.create_texture(GraphTextureDesc {
            label: Some("facade_window_graph_export_texture".to_owned()),
            width: 4,
            height: 4,
            format: TextureFormat::Rgba8Unorm,
        });
        let buffer = graph.create_buffer(GraphBufferDesc {
            label: Some("facade_window_graph_export_buffer".to_owned()),
            size: 16,
        });
        graph
            .add_pass("facade_window_graph_export_pass")
            .write_texture(texture, TextureWriteUsage::Storage)
            .write_buffer(buffer, BufferWriteUsage::Storage)
            .execute(move |ctx| {
                ctx.rhi_texture(texture)?;
                ctx.rhi_buffer(buffer)?;
                Ok(())
            });
        graph.export_texture("facade_window_graph_texture_output", texture);
        graph.export_texture("facade_window_main_color_output", ctx.main_color());
        graph.export_texture("facade_window_main_depth_output", ctx.main_depth());
        graph.export_buffer("facade_window_graph_buffer_output", buffer);
        Ok(())
    }
}

fn main() -> PlatformResult<()> {
    WinitPlatform::new().run(Box::new(FacadeWindowApp::new(parse_options()?)))
}

fn stats_window_title(
    stats: &FrameStats,
    surface_readbacks: u32,
    surface_readback_frame_outputs: usize,
) -> String {
    let gpu = if !stats.gpu_profiler_enabled {
        "gpu:off".to_owned()
    } else if let Some(time_ms) = stats.gpu_time_ms {
        format!("gpu:{time_ms:.3}ms")
    } else {
        "gpu:n/a".to_owned()
    };
    format!(
        "Neo Renderer Facade Window Usecase | draws:{} visible:{} {} rb:{} rb_outputs:{} graph_exports:{}",
        stats.draw_calls,
        stats.visible_objects,
        gpu,
        surface_readbacks,
        surface_readback_frame_outputs,
        stats.public_graph_promoted_export_count()
    )
}

fn stats_summary(
    stats: &FrameStats,
    surface_readback_pending: bool,
    surface_readback_available: bool,
    surface_readbacks: u32,
    surface_readback_frame_outputs: usize,
) -> String {
    format!(
        "surface-smoke frame={} draws={} visible={} profiler={} gpu_time_ms={:?} graph_passes={} rhi_passes={} semantic_passes={} pipeline_cache_total={} backend_objects={} shader_layouts={} public_outputs={} unsupported_public_outputs={} public_graph_exports={} public_graph_promoted_exports={} public_graph_promoted_textures={} public_graph_promoted_buffers={} public_graph_promoted_texture_labels={:?} public_graph_promoted_buffer_labels={:?} surface_readback_pending={} surface_readback_available={} surface_readbacks={} surface_readback_frame_outputs={}",
        stats.frame_index,
        stats.draw_calls,
        stats.visible_objects,
        stats.gpu_profiler_enabled,
        stats.gpu_time_ms,
        stats.graph.pass_count,
        stats.graph.rhi_executed_passes,
        stats.graph.semantic_passes,
        stats.pipeline_cache.total,
        stats.pipeline_cache.backend_objects,
        stats.pipeline_cache.shader_interface_layouts,
        stats.public_frame_outputs.len(),
        stats.unsupported_public_frame_output_count(),
        stats.public_graph_export_count(),
        stats.public_graph_promoted_export_count(),
        stats.public_graph_promoted_textures,
        stats.public_graph_promoted_buffers,
        stats.public_graph_promoted_texture_labels,
        stats.public_graph_promoted_buffer_labels,
        surface_readback_pending,
        surface_readback_available,
        surface_readbacks,
        surface_readback_frame_outputs,
    )
}

fn backend_surface_readback_output_count(stats: &FrameStats) -> usize {
    stats
        .public_frame_outputs
        .iter()
        .filter(|output| {
            matches!(
                output.source,
                FramePublicOutputSource::BackendMainSurfaceReadback
                    | FramePublicOutputSource::BackendSurfaceReadback
            )
        })
        .count()
}

fn graph_export_requirements_met(stats: &FrameStats) -> bool {
    stats.public_graph_promoted_export_count() == 4
        && stats.public_graph_promoted_textures == 3
        && stats.public_graph_promoted_buffers == 1
        && stats
            .public_graph_promoted_texture_labels
            .iter()
            .any(|label| label == "facade_window_graph_texture_output")
        && stats
            .public_graph_promoted_texture_labels
            .iter()
            .any(|label| label == "facade_window_main_color_output")
        && stats
            .public_graph_promoted_texture_labels
            .iter()
            .any(|label| label == "facade_window_main_depth_output")
        && stats
            .public_graph_promoted_buffer_labels
            .iter()
            .any(|label| label == "facade_window_graph_buffer_output")
}

fn poll_and_materialize_surface_readback(
    renderer: &mut Renderer,
    materialized_count: &mut u32,
    readback_texture: &mut Option<TextureHandle>,
) -> Result<(), RendererError> {
    renderer.poll_surface_frame_readback()?;
    if renderer.surface_frame_readback_available() {
        if let Some(texture) =
            renderer.materialize_surface_frame_readback(Some("facade_window_surface_readback"))?
        {
            *readback_texture = Some(texture);
            *materialized_count = materialized_count.saturating_add(1);
        }
    }
    Ok(())
}

fn parse_options() -> PlatformResult<AppOptions> {
    let mut options = AppOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke-frames" => {
                let Some(value) = args.next() else {
                    return Err(PlatformError::BackendError(
                        "--smoke-frames requires a positive frame count".to_owned(),
                    ));
                };
                let frames = value.parse::<u32>().map_err(|_| {
                    PlatformError::BackendError(
                        "--smoke-frames requires a positive frame count".to_owned(),
                    )
                })?;
                if frames == 0 {
                    return Err(PlatformError::BackendError(
                        "--smoke-frames requires a positive frame count".to_owned(),
                    ));
                }
                options.smoke_frames = Some(frames);
            }
            "--wait-for-gpu" => {
                options.wait_for_gpu = true;
            }
            "--print-stats" => {
                options.print_stats = true;
            }
            "--require-gpu-time" => {
                options.require_gpu_time = true;
            }
            "--surface-readback" => {
                options.surface_readback = true;
            }
            "--require-surface-readback" => {
                options.surface_readback = true;
                options.require_surface_readback = true;
            }
            "--graph-export" => {
                options.graph_export = true;
            }
            "--require-graph-export" => {
                options.graph_export = true;
                options.require_graph_export = true;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: render_facade_window_usecase [--smoke-frames N] [--wait-for-gpu] [--print-stats] [--require-gpu-time] [--surface-readback] [--require-surface-readback] [--graph-export] [--require-graph-export]"
                );
                std::process::exit(0);
            }
            _ => {
                return Err(PlatformError::BackendError(format!(
                    "unknown argument: {arg}"
                )));
            }
        }
    }
    Ok(options)
}

fn build_scene(renderer: &mut Renderer) -> Result<SceneHandle, RendererError> {
    let mesh = renderer.create_mesh(triangle_mesh_desc())?;
    let texture = renderer.create_texture(TextureDesc {
        label: Some("facade_window_checker"),
        dimension: TextureDimension::D2,
        width: 2,
        height: 2,
        depth_or_layers: 1,
        mip_levels: 1,
        samples: 1,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        initial_data: Some(TextureInitialData {
            bytes: &[
                255, 180, 80, 255, 32, 80, 160, 255, 32, 80, 160, 255, 255, 180, 80, 255,
            ],
            bytes_per_row: 8,
            rows_per_image: 2,
        }),
    })?;
    let material = renderer.create_standard_material(StandardMaterialDesc {
        label: Some("facade_window_material".to_owned()),
        domain: MaterialDomain::Opaque,
        base_color: Color::WHITE,
        base_color_texture: Some(texture),
        normal_texture: None,
        metallic_roughness_texture: None,
        occlusion_texture: None,
        emissive_texture: None,
        metallic: 0.05,
        roughness: 0.55,
        emissive: Vec3::ZERO,
        alpha_mode: AlphaMode::Opaque,
        double_sided: true,
        receive_shadows: true,
        cast_shadows: true,
    })?;
    let reflected_material = create_reflected_facade_material(renderer, texture)?;
    let scene = renderer.create_scene(SceneDesc {
        label: Some("facade_window_scene".to_owned()),
        max_objects_hint: Some(2),
        max_lights_hint: Some(1),
        enable_gpu_culling: false,
        enable_occlusion_culling: false,
    })?;

    renderer.edit_scene(scene, |scene| {
        scene.spawn(RenderObjectDesc {
            label: Some("facade_window_triangle".to_owned()),
            mesh,
            materials: vec![material],
            transform: IDENTITY_MAT4,
            visibility: VisibilityFlags::CAMERA | VisibilityFlags::SHADOW,
            flags: ObjectFlags::STATIC | ObjectFlags::CAST_SHADOW | ObjectFlags::RECEIVE_SHADOW,
            ..RenderObjectDesc::default()
        });
        scene.spawn(RenderObjectDesc {
            label: Some("facade_window_reflected_triangle".to_owned()),
            mesh,
            materials: vec![reflected_material],
            transform: IDENTITY_MAT4,
            visibility: VisibilityFlags::CAMERA,
            flags: ObjectFlags::STATIC,
            ..RenderObjectDesc::default()
        });
        scene.add_light(LightDesc::Directional(DirectionalLightDesc {
            label: Some("facade_window_sun".to_owned()),
            direction: Vec3::new(-0.25, -1.0, -0.35),
            color: Color::WHITE,
            illuminance_lux: 70_000.0,
            shadow: None,
            layer_mask: RenderLayerMask::all(),
        }))?;
        Ok::<(), RendererError>(())
    })??;

    Ok(scene)
}

fn create_reflected_facade_material(
    renderer: &mut Renderer,
    texture: TextureHandle,
) -> Result<MaterialHandle, RendererError> {
    let sampler = renderer.create_sampler(SamplerDesc::default())?;
    let shader = renderer.create_shader(ShaderDesc {
        label: Some("facade_window_reflected_shader"),
        source: ShaderSource::Wgsl(
            r#"
struct Tint {
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> tint: Tint;

@group(0) @binding(1)
var base_tex: texture_2d<f32>;

@group(0) @binding(2)
var base_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(2) uv0: vec2<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position.xy + vec2<f32>(0.45, 0.05), position.z, 1.0);
    out.uv = uv0;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(base_tex, base_sampler, in.uv) * tint.color;
}
"#,
        ),
        stages: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
        entry_points: ShaderEntryPoints {
            vertex: Some("vs_main"),
            fragment: Some("fs_main"),
            compute: None,
        },
        reflection: ShaderReflectionMode::Auto,
        features: ShaderFeatureSet::default(),
        hot_reload_key: None,
    })?;
    let template = renderer.create_material_template(MaterialTemplateDesc {
        label: Some("facade_window_reflected_template".to_owned()),
        shader,
        domain: MaterialDomain::Opaque,
        render_state: RenderStateDesc { depth_write: false },
        parameter_schema: MaterialParameterSchema {
            parameters: vec![
                "tint".to_owned(),
                "base_tex".to_owned(),
                "base_sampler".to_owned(),
            ],
        },
        passes: MaterialPassFlags::FORWARD,
    })?;
    renderer.create_material(MaterialDesc {
        label: Some("facade_window_reflected_material".to_owned()),
        template,
        parameters: vec![
            MaterialParameter {
                name: "tint".to_owned(),
                value: MaterialParameterValue::Bytes(vec![
                    0, 0, 192, 62, 0, 0, 64, 63, 0, 0, 128, 63, 0, 0, 128, 63,
                ]),
            },
            MaterialParameter {
                name: "base_tex".to_owned(),
                value: MaterialParameterValue::Texture(texture),
            },
            MaterialParameter {
                name: "base_sampler".to_owned(),
                value: MaterialParameterValue::Sampler(sampler),
            },
        ],
        overrides: MaterialOverrides::default(),
    })
}

fn triangle_mesh_desc() -> MeshDesc<'static> {
    const VERTICES: &[u8] = &[0; 96];
    MeshDesc {
        label: Some("facade_window_triangle"),
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
            bounds: Bounds3::new(Vec3::new(-0.8, -0.6, 0.0), Vec3::new(0.8, 0.7, 0.0)),
        }],
        bounds: Bounds3::new(Vec3::new(-0.8, -0.6, 0.0), Vec3::new(0.8, 0.7, 0.0)),
        usage: MeshUsage::STATIC,
        flags: MeshFlags::default(),
        skin: None,
        morph_targets: Vec::new(),
        meshlets: None,
    }
}

fn block_on_ready<F: Future>(future: F) -> F::Output {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("renderer initialization future unexpectedly yielded pending"),
    }
}

fn noop_waker() -> Waker {
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

fn noop_raw_waker() -> RawWaker {
    RawWaker::new(std::ptr::null(), &NOOP_WAKER_VTABLE)
}

static NOOP_WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);

unsafe fn noop_clone(_: *const ()) -> RawWaker {
    noop_raw_waker()
}

unsafe fn noop_wake(_: *const ()) {}

unsafe fn noop_drop(_: *const ()) {}

fn platform_error(error: impl std::error::Error) -> PlatformError {
    PlatformError::BackendError(error.to_string())
}
