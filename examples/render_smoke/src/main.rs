use engine_graphics::{Color, RenderSurface, SurfaceSize};
use engine_platform::{
    ButtonState, InputEvent, KeyCode, PhysicalSize, Platform, PlatformApp, PlatformContext,
    PlatformError, PlatformEvent, PlatformResult, RunMode, WindowDesc, WindowEvent, WindowId,
};
use engine_render::{
    DirectionalLight, DirectionalShadow, EnvironmentLight, Material, MaterialHandle, Mesh,
    MeshInstanceHandle, PerspectiveCamera, PointLight, PointShadow, RenderLighting, RenderQueue,
    RenderScene, SpotLight, SpotShadow, Texture, TextureHandle, TextureSize, Transform,
};
use graphics_wgpu::{WgpuGraphics, WgpuGraphicsOptions, WgpuSurface};
use platform_winit::WinitPlatform;
use render_wgpu::{
    EnvironmentProbeDesc, EnvironmentProbeVolume, EnvironmentProbeVolumeDesc, MeshRenderer,
    WgpuEnvironmentProbe, WgpuEnvironmentTexture, WgpuRenderScene,
};

struct RenderSmokeApp {
    window: Option<WindowId>,
    graphics: Option<WgpuGraphics>,
    surface: Option<WgpuSurface>,
    renderer: Option<MeshRenderer>,
    gpu_scene: Option<WgpuRenderScene>,
    probe: Option<WgpuEnvironmentProbe>,
    scene: RenderScene,
    queue: RenderQueue,
    instance_handles: Vec<MeshInstanceHandle>,
    material_handles: Vec<MaterialHandle>,
    material_textures: Vec<TextureHandle>,
    environment_texture: TextureHandle,
    elapsed_seconds: f32,
    baked_probe_checked: bool,
}

impl RenderSmokeApp {
    fn new() -> Self {
        let mut camera = PerspectiveCamera::default();
        camera.position = [0.0, 0.0, 4.0];
        let mut scene = RenderScene::new(camera);
        scene.set_frustum_culling(true);
        let cube = scene.add_mesh(Mesh::textured_cube(1.0, [1.0, 1.0, 1.0]));
        let material_textures = vec![
            scene.add_texture(showcase_texture(
                "alloy_ceramic.png",
                [235, 240, 238, 255],
                [42, 54, 70, 255],
            )),
            scene.add_texture(showcase_texture(
                "glass_lattice.png",
                [94, 180, 220, 210],
                [18, 26, 34, 255],
            )),
            scene.add_texture(showcase_texture(
                "emissive_panel.png",
                [255, 138, 42, 255],
                [12, 16, 22, 255],
            )),
        ];
        let environment_texture = scene.add_texture(showcase_texture(
            "environment_hangar.png",
            [58, 78, 118, 255],
            [238, 148, 64, 255],
        ));
        scene.set_lighting(animated_lighting(0.0, environment_texture));
        let material_handles = material_textures
            .iter()
            .copied()
            .enumerate()
            .map(|(index, texture)| scene.add_material(animated_material(0.0, index, texture)))
            .collect::<Vec<_>>();
        let instance_materials = [
            material_handles[0],
            material_handles[0],
            material_handles[1],
            material_handles[2],
        ];
        let instance_handles = instance_materials
            .iter()
            .copied()
            .map(|material| scene.add_instance_with_material(cube, material, Transform::IDENTITY))
            .collect();
        let queue = RenderQueue::from_scene(&scene);

        Self {
            window: None,
            graphics: None,
            surface: None,
            renderer: None,
            gpu_scene: None,
            probe: None,
            scene,
            queue,
            instance_handles,
            material_handles,
            material_textures,
            environment_texture,
            elapsed_seconds: 0.0,
            baked_probe_checked: false,
        }
    }
}

impl PlatformApp for RenderSmokeApp {
    fn on_resumed(&mut self, ctx: &mut dyn PlatformContext) -> PlatformResult<()> {
        if self.window.is_some() {
            return Ok(());
        }

        let window_id = ctx.create_window(WindowDesc {
            title: window_title().to_owned(),
            ..WindowDesc::default()
        })?;
        let window = ctx.window(window_id).ok_or(PlatformError::WindowNotFound)?;
        let size = surface_size(window.inner_size());
        let graphics = WgpuGraphics::new(WgpuGraphicsOptions::default()).map_err(platform_error)?;
        let mut surface = graphics
            .create_surface(window, size)
            .map_err(platform_error)?;
        let sample_count = preferred_sample_count(&surface);
        surface
            .set_sample_count(sample_count)
            .map_err(platform_error)?;
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
        let probe_size = if is_feature_showcase() { 64 } else { 128 };
        let probe = WgpuEnvironmentProbe::new(
            &graphics,
            probe_size,
            surface.format(),
            surface.depth_format(),
        )
        .map_err(platform_error)?;

        self.window = Some(window_id);
        self.graphics = Some(graphics);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.gpu_scene = Some(gpu_scene);
        self.probe = Some(probe);

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
                    self.probe = None;
                    self.renderer = None;
                    self.surface = None;
                    self.graphics = None;
                    self.window = None;
                    ctx.destroy_window(id)?;
                    ctx.exit();
                }
                WindowEvent::Resized { size } => {
                    if let Some(surface) = &mut self.surface {
                        surface.resize(surface_size(size)).map_err(platform_error)?;
                    }
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    if let Some(surface) = &mut self.surface {
                        surface
                            .resize(surface_size(new_inner_size))
                            .map_err(platform_error)?;
                    }
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
        self.scene
            .set_clear_color(animated_clear_color(self.elapsed_seconds));
        self.scene.set_camera(animated_camera(self.elapsed_seconds));
        self.scene.set_lighting(animated_lighting(
            self.elapsed_seconds,
            self.environment_texture,
        ));
        if let Some(surface) = &self.surface {
            self.scene.set_aspect_ratio(aspect_ratio(surface.size()));
        }

        for (index, handle) in self.instance_handles.iter().copied().enumerate() {
            let _ = self.scene.set_instance_transform(
                handle,
                animated_instance_transform(self.elapsed_seconds, index),
            );
        }
        for (index, handle) in self.material_handles.iter().copied().enumerate() {
            let texture = self
                .material_textures
                .get(index)
                .copied()
                .unwrap_or(self.environment_texture);
            let _ = self.scene.replace_material(
                handle,
                animated_material(self.elapsed_seconds, index, texture),
            );
        }
        self.queue = RenderQueue::from_scene(&self.scene);

        if let (Some(renderer), Some(graphics), Some(surface), Some(gpu_scene)) = (
            &self.renderer,
            &self.graphics,
            &self.surface,
            &mut self.gpu_scene,
        ) {
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
        _ctx: &mut dyn PlatformContext,
        window_id: WindowId,
    ) -> PlatformResult<()> {
        if Some(window_id) == self.window {
            if let (Some(renderer), Some(graphics), Some(surface), Some(gpu_scene), Some(probe)) = (
                &mut self.renderer,
                &self.graphics,
                &mut self.surface,
                &self.gpu_scene,
                &mut self.probe,
            ) {
                let probe_desc = EnvironmentProbeDesc::at([0.0, 0.0, 0.0])
                    .with_range(0.05, 8.0)
                    .with_clear_color(self.scene.clear_color());
                if !is_feature_showcase() || !self.baked_probe_checked {
                    gpu_scene
                        .capture_environment_probe(
                            renderer,
                            graphics,
                            probe,
                            &self.queue,
                            probe_desc,
                        )
                        .map_err(platform_error)?;
                }
                let probe_volume_desc = EnvironmentProbeVolumeDesc::from_center_extents(
                    [0.0, 0.0, 0.0],
                    [2.25, 2.25, 2.25],
                )
                .with_blend_distance(1.0);

                if !is_feature_showcase() && !self.baked_probe_checked {
                    let baked_probe = probe
                        .bake(graphics, probe_desc, Some(probe_volume_desc))
                        .map_err(platform_error)?;
                    let _baked_environment =
                        WgpuEnvironmentTexture::from_baked_probe(graphics, &baked_probe)
                            .map_err(platform_error)?;
                    self.baked_probe_checked = true;
                }
                if is_feature_showcase() {
                    self.baked_probe_checked = true;
                }

                let probe_volume = EnvironmentProbeVolume::new(probe, probe_volume_desc);
                gpu_scene
                    .render_with_environment_probe_volumes(
                        renderer,
                        surface,
                        &self.queue,
                        &[probe_volume],
                    )
                    .map_err(platform_error)?;
            }
        }

        Ok(())
    }
}

fn main() -> PlatformResult<()> {
    WinitPlatform::new().run(Box::new(RenderSmokeApp::new()))
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

fn window_title() -> &'static str {
    if is_feature_showcase() {
        "Neo Render Feature Showcase"
    } else {
        "Neo Render Smoke"
    }
}

fn is_feature_showcase() -> bool {
    env!("CARGO_PKG_NAME") == "render_feature_showcase"
}

fn showcase_texture(file_name: &str, primary: [u8; 4], secondary: [u8; 4]) -> Texture {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(file_name);
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| Texture::from_image_bytes(&path.to_string_lossy(), &bytes).ok())
        .unwrap_or_else(|| {
            Texture::checkerboard_rgba8(TextureSize::new(64, 64), 8, primary, secondary)
        })
}

fn animated_clear_color(time: f32) -> Color {
    let r = wave(time * 0.7 + 0.0);
    let g = wave(time * 0.8 + 2.1);
    let b = wave(time * 0.9 + 4.2);

    Color::rgb(r, g, b)
}

fn animated_instance_transform(time: f32, index: usize) -> Transform {
    let index = index as f32;
    let phase = index * 2.1;
    let x = (index - 1.5) * 0.48;
    let y = (time * 0.9 + phase).sin() * 0.16;
    let z = match index as usize {
        0 => 0.35,
        1 => -0.75,
        2 => 0.05,
        _ => -1.25,
    };
    let scale = 0.46 + wave_f32(time * 1.3 + phase) * 0.1;

    Transform::new_3d(
        [x, y, z],
        [
            time * (0.35 + index * 0.04) + phase * 0.2,
            time * (0.55 + index * 0.08) + phase,
            time * (0.25 + index * 0.05),
        ],
        [scale, scale, scale],
    )
}

fn animated_material(time: f32, index: usize, texture: TextureHandle) -> Material {
    let phase = index as f32 * 2.2;
    let mut tint = [
        0.65 + wave_f32(time * 0.8 + phase) * 0.35,
        0.65 + wave_f32(time * 0.9 + phase + 1.4) * 0.35,
        0.65 + wave_f32(time * 1.0 + phase + 2.8) * 0.35,
        1.0,
    ];

    if index == 0 {
        Material::opaque_textured(tint, texture)
            .with_surface(0.22 + wave_f32(time * 0.6) * 0.18, 0.25)
            .with_clearcoat(0.45 + wave_f32(time * 0.7) * 0.35, 0.18)
            .with_sheen([0.15, 0.12, 0.08], 0.35)
            .with_specular(0.85, [1.0, 0.95, 0.88])
            .with_anisotropy(0.35 + wave_f32(time * 0.45) * 0.25, time * 0.2)
            .with_iridescence(0.28 + wave_f32(time * 0.4) * 0.2, 1.45, 120.0, 380.0)
            .with_iridescence_texture(texture)
            .with_iridescence_thickness_texture(texture)
            .with_dispersion(0.08)
    } else {
        tint[3] = if index == 1 { 0.58 } else { 0.46 };
        let material = Material::alpha_blended_textured(tint, texture)
            .with_surface(0.72, 0.0)
            .with_transmission(if index == 1 { 0.35 } else { 0.18 })
            .with_ior(if index == 1 { 1.45 } else { 1.25 })
            .with_clearcoat(0.25, 0.08)
            .with_specular(0.65, [0.82, 0.9, 1.0])
            .with_anisotropy(0.18, time * 0.15 + index as f32)
            .with_volume(
                if index == 1 { 0.42 } else { 0.28 },
                if index == 1 {
                    [0.76, 0.86, 1.0]
                } else {
                    [1.0, 0.78, 0.72]
                },
                2.8,
            )
            .with_thickness_texture(texture)
            .with_dispersion(if index == 1 { 0.18 } else { 0.1 });
        if index == 2 {
            material.with_unlit(true)
        } else {
            material
        }
    }
}

fn animated_camera(time: f32) -> PerspectiveCamera {
    let mut camera = PerspectiveCamera::default();
    camera.position = [
        time.sin() * 0.18,
        (time * 0.7).cos() * 0.12,
        4.0 + (time * 0.4).sin() * 0.2,
    ];
    camera
}

fn animated_lighting(time: f32, environment_texture: TextureHandle) -> RenderLighting {
    RenderLighting::new(
        [0.9, 0.94, 1.0],
        0.22,
        DirectionalLight::new(
            [(time * 0.35).cos() * 0.45, 0.85, (time * 0.35).sin() * 0.45],
            [1.0, 0.96, 0.9],
            0.85,
        ),
    )
    .with_directional_shadow(
        DirectionalShadow::enabled(1024, 5.5, -5.0, 5.0, 0.45, 0.003).with_cascades(4, 8.0, 0.55),
    )
    .with_environment(
        EnvironmentLight::new([0.72, 0.82, 1.0], 0.18, [1.0, 0.92, 0.82], 0.35)
            .with_texture(environment_texture)
            .with_background_intensity(0.22),
    )
    .with_point_lights(&[
        PointLight::new(
            [
                (time * 0.9).cos() * 1.15,
                0.55,
                1.15 + (time * 0.4).sin() * 0.35,
            ],
            [1.0, 0.5, 0.25],
            1.35,
            2.4,
        )
        .with_shadow(PointShadow::enabled(512, 0.05, 3.2, 0.32, 0.004)),
        PointLight::new(
            [(time * 0.7 + 2.4).cos() * 0.9, -0.55, -0.65],
            [0.35, 0.65, 1.0],
            0.9,
            1.8,
        )
        .with_shadow(PointShadow::enabled(512, 0.05, 2.6, 0.24, 0.004)),
    ])
    .with_spot_lights(&[
        SpotLight::new(
            [(time * 0.45).sin() * 0.65, 1.35, 1.65],
            [(time * 0.55).sin() * 0.2, -0.75, -1.0],
            [0.95, 1.0, 0.72],
            1.4,
            3.2,
            0.28,
            0.7,
        )
        .with_shadow(SpotShadow::enabled(1024, 0.05, 5.0, 0.35, 0.004)),
        SpotLight::new(
            [(time * 0.35 + 1.7).sin() * 0.75, -1.15, 1.15],
            [(time * 0.45 + 1.2).sin() * -0.25, 0.65, -1.0],
            [0.65, 0.78, 1.0],
            0.8,
            2.8,
            0.22,
            0.62,
        )
        .with_shadow(SpotShadow::enabled(1024, 0.05, 4.5, 0.25, 0.004)),
    ])
}

fn aspect_ratio(size: SurfaceSize) -> f32 {
    if size.height == 0 {
        1.0
    } else {
        size.width as f32 / size.height as f32
    }
}

fn wave(value: f32) -> f64 {
    (value.sin() * 0.5 + 0.5) as f64
}

fn wave_f32(value: f32) -> f32 {
    value.sin() * 0.5 + 0.5
}

fn platform_error(error: impl std::error::Error) -> PlatformError {
    PlatformError::BackendError(error.to_string())
}
