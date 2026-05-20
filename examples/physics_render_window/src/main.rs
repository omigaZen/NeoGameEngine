use engine_graphics::{Color, RenderSurface, SurfaceSize};
use engine_physics::prelude as physics;
use engine_physics::prelude::PhysicsBackend;
use engine_platform::{
    ButtonState, InputEvent, KeyCode, PhysicalSize, Platform, PlatformApp, PlatformContext,
    PlatformError, PlatformEvent, PlatformResult, RunMode, WindowDesc, WindowEvent, WindowId,
};
use engine_render::{
    DirectionalLight, Mat4, Material, Mesh, MeshInstanceHandle, PerspectiveCamera, PointLight,
    RenderLighting, RenderQueue, RenderScene, Texture, TextureSize,
};
use graphics_wgpu::{WgpuGraphics, WgpuGraphicsOptions, WgpuSurface};
use platform_winit::WinitPlatform;
use render_wgpu::{MeshRenderer, WgpuRenderScene};

const FIXED_DT: f32 = 1.0 / 60.0;
const RESET_INTERVAL_SECONDS: f32 = 9.0;

#[derive(Clone, Debug, Default)]
struct AppOptions {
    smoke_frames: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
struct BodyVisual {
    body: physics::BodyId,
    instance: MeshInstanceHandle,
    scale: [f32; 3],
}

struct PhysicsRenderApp {
    options: AppOptions,
    window: Option<WindowId>,
    graphics: Option<WgpuGraphics>,
    surface: Option<WgpuSurface>,
    renderer: Option<MeshRenderer>,
    gpu_scene: Option<WgpuRenderScene>,
    scene: RenderScene,
    queue: RenderQueue,
    physics: physics::DefaultPhysicsBackend,
    initial_snapshot: physics::PhysicsSnapshot,
    visuals: Vec<BodyVisual>,
    events: Vec<physics::PhysicsEvent>,
    backend_name: &'static str,
    elapsed_seconds: f32,
    next_reset_seconds: f32,
    physics_accumulator: f32,
    rendered_frames: u32,
    last_active_bodies: usize,
    total_events: usize,
}

impl PhysicsRenderApp {
    fn new(options: AppOptions) -> PlatformResult<Self> {
        let DemoScene {
            scene,
            physics,
            initial_snapshot,
            visuals,
            backend_name,
        } = build_demo_scene().map_err(platform_error)?;
        let queue = RenderQueue::from_scene(&scene);

        Ok(Self {
            options,
            window: None,
            graphics: None,
            surface: None,
            renderer: None,
            gpu_scene: None,
            scene,
            queue,
            physics,
            initial_snapshot,
            visuals,
            events: Vec::new(),
            backend_name,
            elapsed_seconds: 0.0,
            next_reset_seconds: RESET_INTERVAL_SECONDS,
            physics_accumulator: 0.0,
            rendered_frames: 0,
            last_active_bodies: 0,
            total_events: 0,
        })
    }

    fn sync_scene_from_physics(&mut self) {
        let snapshot = self.physics.snapshot();
        for visual in &self.visuals {
            if let Some(body) = snapshot.bodies.iter().find(|body| body.id == visual.body) {
                let _ = self.scene.set_instance_model_matrix(
                    visual.instance,
                    physics_matrix(body.transform, visual.scale),
                );
            }
        }
        self.queue = RenderQueue::from_scene(&self.scene);
    }

    fn reset_physics(&mut self) -> PlatformResult<()> {
        self.physics
            .restore(self.initial_snapshot.clone())
            .map_err(platform_error)?;
        self.physics_accumulator = 0.0;
        self.events.clear();
        self.sync_scene_from_physics();
        Ok(())
    }
}

impl PlatformApp for PhysicsRenderApp {
    fn on_resumed(&mut self, ctx: &mut dyn PlatformContext) -> PlatformResult<()> {
        if self.window.is_some() {
            return Ok(());
        }

        let window_id = ctx.create_window(WindowDesc {
            title: window_title(self.backend_name, 0, 0, 0),
            ..WindowDesc::default()
        })?;
        let window = ctx.window(window_id).ok_or(PlatformError::WindowNotFound)?;
        let size = surface_size(window.inner_size());
        let graphics = WgpuGraphics::new(WgpuGraphicsOptions::default()).map_err(platform_error)?;
        let mut surface = graphics
            .create_surface(window, size)
            .map_err(platform_error)?;
        surface
            .set_sample_count(preferred_sample_count(&surface))
            .map_err(platform_error)?;
        self.scene.set_aspect_ratio(aspect_ratio(surface.size()));
        self.queue = RenderQueue::from_scene(&self.scene);
        let renderer = MeshRenderer::new_with_sample_count(
            &graphics,
            surface.format(),
            surface.depth_format(),
            surface.sample_count(),
        )
        .map_err(platform_error)?;
        let gpu_scene = WgpuRenderScene::prepare(
            &graphics,
            &renderer,
            &self.scene,
            &self.queue,
            aspect_ratio(surface.size()),
        )
        .map_err(platform_error)?;

        self.window = Some(window_id);
        self.graphics = Some(graphics);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.gpu_scene = Some(gpu_scene);
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
                    self.gpu_scene = None;
                    self.renderer = None;
                    self.surface = None;
                    self.graphics = None;
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
                    if keyboard.state == ButtonState::Pressed {
                        match keyboard.key {
                            KeyCode::Escape => ctx.exit(),
                            KeyCode::Space => self.reset_physics()?,
                            _ => {}
                        }
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
        let frame_dt = dt.as_secs_f32().min(0.05);
        self.elapsed_seconds += frame_dt;
        if self.elapsed_seconds >= self.next_reset_seconds {
            self.reset_physics()?;
            self.next_reset_seconds = self.elapsed_seconds + RESET_INTERVAL_SECONDS;
        }

        self.physics_accumulator += frame_dt;
        let mut stepped = false;
        while self.physics_accumulator + f32::EPSILON >= FIXED_DT {
            let report = self.physics.step(FIXED_DT, &mut self.events);
            self.last_active_bodies = report.active_bodies;
            self.total_events = self.total_events.saturating_add(report.events_generated);
            self.physics_accumulator -= FIXED_DT;
            stepped = true;
        }
        if stepped {
            self.sync_scene_from_physics();
        }

        if let (Some(graphics), Some(renderer), Some(surface), Some(gpu_scene)) = (
            &self.graphics,
            &self.renderer,
            &self.surface,
            &mut self.gpu_scene,
        ) {
            self.scene.set_aspect_ratio(aspect_ratio(surface.size()));
            gpu_scene
                .sync(
                    graphics,
                    renderer,
                    &self.scene,
                    &self.queue,
                    aspect_ratio(surface.size()),
                )
                .map_err(platform_error)?;
        }

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

        if let (Some(renderer), Some(surface), Some(gpu_scene)) =
            (&mut self.renderer, &mut self.surface, &self.gpu_scene)
        {
            gpu_scene
                .render(renderer, surface, &self.queue)
                .map_err(platform_error)?;
        }

        self.rendered_frames = self.rendered_frames.saturating_add(1);
        if let Some(window) = ctx.window(window_id) {
            window.set_title(&window_title(
                self.backend_name,
                self.rendered_frames,
                self.last_active_bodies,
                self.total_events,
            ));
        }
        if let Some(target_frames) = self.options.smoke_frames {
            if self.rendered_frames >= target_frames {
                ctx.exit();
            }
        }
        Ok(())
    }
}

impl PhysicsRenderApp {
    fn resize(&mut self, size: PhysicalSize<u32>) -> PlatformResult<()> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if let Some(surface) = &mut self.surface {
            surface.resize(surface_size(size)).map_err(platform_error)?;
        }
        Ok(())
    }
}

struct DemoScene {
    scene: RenderScene,
    physics: physics::DefaultPhysicsBackend,
    initial_snapshot: physics::PhysicsSnapshot,
    visuals: Vec<BodyVisual>,
    backend_name: &'static str,
}

fn build_demo_scene() -> physics::PhysicsResult<DemoScene> {
    let mut camera = PerspectiveCamera::new(45.0_f32.to_radians(), 0.05, 80.0);
    camera.position = [0.0, 1.5, 8.0];
    camera.rotation_radians = [-0.16, 0.0, 0.0];

    let mut scene = RenderScene::new(camera);
    scene.set_clear_color(Color::rgb(0.025, 0.03, 0.035));
    scene.set_frustum_culling(true);
    scene.set_lighting(
        RenderLighting::new(
            [0.7, 0.78, 0.9],
            0.35,
            DirectionalLight::new([-0.35, -1.0, -0.45], [1.0, 0.95, 0.86], 1.6),
        )
        .with_point_lights(&[
            PointLight::new([-2.5, 2.8, 3.2], [1.0, 0.42, 0.22], 1.4, 7.0),
            PointLight::new([2.3, 3.2, 2.6], [0.36, 0.62, 1.0], 1.2, 7.0),
        ]),
    );

    let cube_mesh = scene.add_mesh(Mesh::textured_cube(1.0, [1.0, 1.0, 1.0]));
    let ground_texture = scene.add_texture(Texture::checkerboard_rgba8(
        TextureSize::new(128, 128),
        16,
        [42, 48, 54, 255],
        [28, 34, 40, 255],
    ));
    let ground_material = scene.add_material(
        Material::opaque_textured([0.9, 0.95, 1.0, 1.0], ground_texture).with_surface(0.72, 0.0),
    );
    let ramp_material =
        scene.add_material(Material::solid([0.18, 0.32, 0.34, 1.0]).with_surface(0.58, 0.05));
    let cube_materials = [
        scene.add_material(Material::solid([0.92, 0.30, 0.22, 1.0]).with_surface(0.42, 0.0)),
        scene.add_material(Material::solid([0.20, 0.62, 0.94, 1.0]).with_surface(0.38, 0.0)),
        scene.add_material(Material::solid([0.98, 0.72, 0.24, 1.0]).with_surface(0.45, 0.0)),
        scene.add_material(Material::solid([0.52, 0.86, 0.42, 1.0]).with_surface(0.45, 0.0)),
        scene.add_material(Material::solid([0.78, 0.46, 0.96, 1.0]).with_surface(0.40, 0.0)),
    ];

    let mut config = physics::PhysicsConfig::default();
    config.sleeping.enabled = false;
    config.timestep.fixed_dt = FIXED_DT;
    let mut ids = physics::PhysicsWorld::new(config.clone());
    let mut physics = physics::DefaultPhysicsBackend::try_new(config)?;
    let backend_name = physics.capabilities().backend_name;
    let mut visuals = Vec::new();

    add_fixed_box(
        &mut ids,
        &mut physics,
        &mut scene,
        &mut visuals,
        cube_mesh,
        ground_material,
        physics::Transform::from_translation(physics::Vec3::new(0.0, -1.35, 0.0)),
        physics::Vec3::new(4.6, 0.18, 2.2),
        "ground",
    )?;
    add_fixed_box(
        &mut ids,
        &mut physics,
        &mut scene,
        &mut visuals,
        cube_mesh,
        ramp_material,
        physics::Transform::from_translation_rotation(
            physics::Vec3::new(-0.85, -0.1, 0.0),
            physics::Quat::from_axis_angle(physics::Vec3::Z, -0.22),
        ),
        physics::Vec3::new(1.8, 0.12, 1.45),
        "ramp",
    )?;

    let seeds = [
        (
            [-1.45, 3.4, -0.35],
            0.36,
            [-0.15, 0.0, 0.0],
            [0.8, 0.2, 1.3],
        ),
        (
            [-0.65, 4.55, 0.20],
            0.42,
            [0.12, 0.0, 0.0],
            [-0.9, 0.5, 0.6],
        ),
        (
            [0.25, 5.75, -0.10],
            0.34,
            [-0.08, 0.0, 0.0],
            [0.5, 0.8, -1.0],
        ),
        ([1.10, 6.85, 0.38], 0.40, [0.04, 0.0, 0.0], [-0.3, 1.1, 0.4]),
        (
            [1.65, 8.0, -0.22],
            0.32,
            [-0.20, 0.0, 0.0],
            [1.2, -0.4, 0.8],
        ),
    ];
    for (index, (position, half, linear, angular)) in seeds.into_iter().enumerate() {
        let transform = physics::Transform::from_translation(physics::Vec3::new(
            position[0],
            position[1],
            position[2],
        ));
        let desc = physics::BodyDesc::dynamic()
            .with_transform(transform)
            .with_linear_velocity(physics::Vec3::new(linear[0], linear[1], linear[2]))
            .with_angular_velocity(physics::Vec3::new(angular[0], angular[1], angular[2]))
            .with_debug_name(format!("falling_box_{index}"));
        add_body_box(
            &mut ids,
            &mut physics,
            &mut scene,
            &mut visuals,
            cube_mesh,
            cube_materials[index],
            desc,
            physics::Vec3::splat(half),
            &format!("falling_box_{index}"),
        )?;
    }

    let initial_snapshot = physics.snapshot();
    Ok(DemoScene {
        scene,
        physics,
        initial_snapshot,
        visuals,
        backend_name,
    })
}

fn add_fixed_box(
    ids: &mut physics::PhysicsWorld,
    physics: &mut physics::DefaultPhysicsBackend,
    scene: &mut RenderScene,
    visuals: &mut Vec<BodyVisual>,
    mesh: engine_render::MeshHandle,
    material: engine_render::MaterialHandle,
    transform: physics::Transform,
    half_extents: physics::Vec3,
    label: &str,
) -> physics::PhysicsResult<()> {
    let desc = physics::BodyDesc::fixed()
        .with_transform(transform)
        .with_debug_name(label);
    add_body_box(
        ids,
        physics,
        scene,
        visuals,
        mesh,
        material,
        desc,
        half_extents,
        label,
    )
}

fn add_body_box(
    ids: &mut physics::PhysicsWorld,
    physics: &mut physics::DefaultPhysicsBackend,
    scene: &mut RenderScene,
    visuals: &mut Vec<BodyVisual>,
    mesh: engine_render::MeshHandle,
    material: engine_render::MaterialHandle,
    desc: physics::BodyDesc,
    half_extents: physics::Vec3,
    label: &str,
) -> physics::PhysicsResult<()> {
    let collider_desc =
        physics::ColliderDesc::cuboid(half_extents).with_debug_name(format!("{label}_collider"));
    let body = ids.create_body(desc.clone())?;
    let collider = ids.create_collider_with_parent(body, collider_desc.clone())?;
    physics.create_body(body, desc.clone())?;
    physics.create_collider(collider, Some(body), collider_desc)?;

    let scale = [
        half_extents.x * 2.0,
        half_extents.y * 2.0,
        half_extents.z * 2.0,
    ];
    let instance = scene.add_instance_with_material_matrix(
        mesh,
        material,
        physics_matrix(desc.transform, scale),
    );
    visuals.push(BodyVisual {
        body,
        instance,
        scale,
    });
    Ok(())
}

fn physics_matrix(transform: physics::Transform, scale: [f32; 3]) -> Mat4 {
    Mat4::translation([
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
    ]) * Mat4::rotation_quaternion([
        transform.rotation.x,
        transform.rotation.y,
        transform.rotation.z,
        transform.rotation.w,
    ]) * Mat4::scale(scale)
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
            "--help" | "-h" => {
                println!("Usage: physics_render_window [--smoke-frames N]");
                std::process::exit(0);
            }
            _ => {
                return Err(PlatformError::BackendError(format!(
                    "unknown argument: {arg}"
                )))
            }
        }
    }
    Ok(options)
}

fn main() -> PlatformResult<()> {
    WinitPlatform::new().run(Box::new(PhysicsRenderApp::new(parse_options()?)?))
}

fn surface_size(size: PhysicalSize<u32>) -> SurfaceSize {
    SurfaceSize::new(size.width, size.height)
}

fn preferred_sample_count(surface: &WgpuSurface) -> u32 {
    if surface.supported_sample_counts().contains(&4) {
        4
    } else if surface.supported_sample_counts().contains(&2) {
        2
    } else {
        1
    }
}

fn aspect_ratio(size: SurfaceSize) -> f32 {
    if size.height == 0 {
        1.0
    } else {
        size.width as f32 / size.height as f32
    }
}

fn window_title(backend: &str, frames: u32, active_bodies: usize, events: usize) -> String {
    format!(
        "Neo Physics Render Window | backend:{backend} frames:{frames} active:{active_bodies} events:{events}"
    )
}

fn platform_error(error: impl std::error::Error) -> PlatformError {
    PlatformError::BackendError(error.to_string())
}
