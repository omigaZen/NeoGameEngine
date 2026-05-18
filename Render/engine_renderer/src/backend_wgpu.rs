use engine_graphics::{GraphicsError, PresentMode, RenderSurface, SurfaceSize};
use engine_platform::PlatformWindow;
use engine_render::{RenderQueue, RenderScene};
use graphics_wgpu::{wgpu, WgpuGraphics, WgpuGraphicsOptions, WgpuSurface, WgpuSurfaceOptions};
use render_wgpu::{MeshRenderer, WgpuRenderScene};

use crate::{
    BackendPreference, DepthFormat, DeviceStatus, FormatCaps, FrameStats, MemoryStats,
    RenderGraphStats, RendererCaps, RendererConfig, RendererError, RendererFeatures,
    RendererLimits, TextureFormat, VSyncMode, WgpuRhiDevice,
};

pub struct WgpuRendererRuntime {
    config: RendererConfig,
    graphics: WgpuGraphics,
    surface: Option<WgpuSurface>,
    renderer: Option<MeshRenderer>,
    scene: Option<WgpuRenderScene>,
    last_stats: Option<FrameStats>,
    device_status: DeviceStatus,
    frame_index: u64,
}

impl WgpuRendererRuntime {
    pub fn new(config: RendererConfig) -> Result<Self, RendererError> {
        let graphics =
            WgpuGraphics::new(wgpu_options(config.backend)).map_err(map_backend_error)?;
        Ok(Self {
            config,
            graphics,
            surface: None,
            renderer: None,
            scene: None,
            last_stats: None,
            device_status: DeviceStatus::Ok,
            frame_index: 0,
        })
    }

    pub fn with_surface(
        config: RendererConfig,
        window: &dyn PlatformWindow,
    ) -> Result<Self, RendererError> {
        let graphics =
            WgpuGraphics::new(wgpu_options(config.backend)).map_err(map_backend_error)?;
        let size = window.inner_size();
        let mut surface = graphics
            .create_surface_with_options(
                &*window,
                SurfaceSize::new(size.width, size.height),
                surface_options(&config),
            )
            .map_err(map_backend_error)?;
        surface
            .set_present_mode(present_mode_for_vsync(config.vsync))
            .map_err(map_backend_error)?;
        surface
            .set_frame_latency(config.frame_latency)
            .map_err(map_backend_error)?;
        if config.msaa_samples > 1 {
            surface
                .set_sample_count(config.msaa_samples)
                .map_err(map_backend_error)?;
        }
        let renderer = MeshRenderer::new_with_sample_count(
            &graphics,
            surface.format(),
            surface.depth_format(),
            surface.sample_count(),
        )
        .map_err(map_backend_error)?;

        Ok(Self {
            config,
            graphics,
            surface: Some(surface),
            renderer: Some(renderer),
            scene: None,
            last_stats: None,
            device_status: DeviceStatus::Ok,
            frame_index: 0,
        })
    }

    pub fn graphics(&self) -> &WgpuGraphics {
        &self.graphics
    }

    pub fn rhi_device(&self) -> WgpuRhiDevice {
        WgpuRhiDevice::new(&self.graphics)
    }

    pub fn renderer_caps(&self) -> RendererCaps {
        wgpu_renderer_caps(&self.config, &self.graphics)
    }

    pub fn surface(&self) -> Option<&WgpuSurface> {
        self.surface.as_ref()
    }

    pub fn surface_mut(&mut self) -> Option<&mut WgpuSurface> {
        self.surface.as_mut()
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        let Some(surface) = &mut self.surface else {
            return Err(RendererError::Validation(
                "wgpu renderer was created without a surface".to_owned(),
            ));
        };
        surface
            .resize(SurfaceSize::new(width, height))
            .map_err(map_backend_error)
    }

    pub fn set_vsync(&mut self, mode: VSyncMode) -> Result<(), RendererError> {
        if let Some(surface) = &mut self.surface {
            surface
                .set_present_mode(present_mode_for_vsync(mode))
                .map_err(map_backend_error)?;
        }
        self.config.vsync = mode;
        Ok(())
    }

    pub fn render_scene(&mut self, scene: &RenderScene) -> Result<FrameStats, RendererError> {
        let Some(surface) = &mut self.surface else {
            return Err(RendererError::Validation(
                "wgpu renderer was created without a surface".to_owned(),
            ));
        };
        let Some(renderer) = &mut self.renderer else {
            return Err(RendererError::Validation(
                "wgpu mesh renderer is not initialized".to_owned(),
            ));
        };

        let queue = RenderQueue::from_scene(scene);
        let aspect_ratio = aspect_ratio(surface.size());
        match &mut self.scene {
            Some(gpu_scene) => {
                if let Err(error) =
                    gpu_scene.sync(&self.graphics, renderer, scene, &queue, aspect_ratio)
                {
                    return Err(record_backend_error(&mut self.device_status, error));
                }
            }
            None => {
                match WgpuRenderScene::prepare(
                    &self.graphics,
                    renderer,
                    scene,
                    &queue,
                    aspect_ratio,
                ) {
                    Ok(scene) => self.scene = Some(scene),
                    Err(error) => return Err(record_backend_error(&mut self.device_status, error)),
                }
            }
        }

        let gpu_scene = self
            .scene
            .as_ref()
            .expect("gpu scene was prepared before render");
        if let Err(error) = gpu_scene.render(renderer, surface, &queue) {
            return Err(record_backend_error(&mut self.device_status, error));
        }

        let queue_stats = queue.stats();
        let gpu_stats = renderer.last_stats();
        let stats = FrameStats {
            frame_index: self.frame_index,
            draw_calls: gpu_stats.draw_call_count as u32,
            visible_objects: queue_stats.item_count as u32,
            culled_objects: queue_stats.culled_item_count as u32,
            memory: MemoryStats {
                resident_bytes: gpu_stats.instance_buffer_bytes() as u64,
                delayed_destroy_count: 0,
            },
            graph: RenderGraphStats {
                pass_count: default_wgpu_pass_count(scene),
                transient_textures: 0,
                transient_buffers: 1,
                aliased_memory_bytes: 0,
                barriers: 0,
                ..RenderGraphStats::default()
            },
            ..FrameStats::default()
        };
        self.frame_index += 1;
        self.last_stats = Some(stats.clone());
        Ok(stats)
    }

    pub fn last_frame_stats(&self) -> Option<&FrameStats> {
        self.last_stats.as_ref()
    }

    pub fn device_status(&self) -> DeviceStatus {
        self.device_status
    }

    pub fn wait_for_gpu(&self) {
        self.graphics.device().poll(wgpu::Maintain::Wait);
    }

    pub fn config(&self) -> &RendererConfig {
        &self.config
    }
}

fn wgpu_options(backend: BackendPreference) -> WgpuGraphicsOptions {
    WgpuGraphicsOptions {
        power_preference: match backend {
            BackendPreference::Headless => wgpu::PowerPreference::LowPower,
            _ => wgpu::PowerPreference::HighPerformance,
        },
        force_fallback_adapter: matches!(backend, BackendPreference::Headless),
    }
}

fn wgpu_renderer_caps(config: &RendererConfig, graphics: &WgpuGraphics) -> RendererCaps {
    let mut caps = RendererCaps::for_backend(config, "wgpu", &graphics.adapter().get_info().name);
    let limits = graphics.device().limits();
    caps.limits = renderer_limits_from_wgpu(&limits);
    caps.formats = wgpu_format_caps(graphics.adapter());
    let device_features = graphics.device().features();
    if device_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        caps.features = caps.features | RendererFeatures::TIMESTAMP_QUERY;
    } else {
        caps.features = RendererFeatures(caps.features.0 & !RendererFeatures::TIMESTAMP_QUERY.0);
    }
    caps
}

fn wgpu_format_caps(adapter: &wgpu::Adapter) -> FormatCaps {
    let candidates = FormatCaps::default();
    FormatCaps {
        color: candidates
            .color
            .into_iter()
            .filter(|format| {
                format_supports_render_attachment(
                    adapter.get_texture_format_features(wgpu_surface_format(*format)),
                )
            })
            .collect(),
        depth: candidates
            .depth
            .into_iter()
            .filter(|format| {
                format_supports_render_attachment(
                    adapter.get_texture_format_features(wgpu_depth_format(*format)),
                )
            })
            .collect(),
    }
}

fn format_supports_render_attachment(features: wgpu::TextureFormatFeatures) -> bool {
    features
        .allowed_usages
        .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
}

fn renderer_limits_from_wgpu(limits: &wgpu::Limits) -> RendererLimits {
    RendererLimits {
        max_texture_dimension_2d: limits.max_texture_dimension_2d,
        max_texture_array_layers: limits.max_texture_array_layers,
        max_bind_groups: limits.max_bind_groups,
        max_vertex_buffers: limits.max_vertex_buffers,
    }
}

fn present_mode_for_vsync(mode: VSyncMode) -> PresentMode {
    match mode {
        VSyncMode::Off => PresentMode::Immediate,
        VSyncMode::On => PresentMode::Fifo,
        VSyncMode::Adaptive => PresentMode::AutoVsync,
    }
}

fn surface_options(config: &RendererConfig) -> WgpuSurfaceOptions {
    WgpuSurfaceOptions {
        preferred_format: config.surface_format.map(wgpu_surface_format),
        depth_format: wgpu_depth_format(config.depth_format),
    }
}

fn wgpu_surface_format(format: TextureFormat) -> wgpu::TextureFormat {
    match format {
        TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        TextureFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
    }
}

fn wgpu_depth_format(format: DepthFormat) -> wgpu::TextureFormat {
    match format {
        DepthFormat::D16Unorm => wgpu::TextureFormat::Depth16Unorm,
        DepthFormat::D24Plus => wgpu::TextureFormat::Depth24Plus,
        DepthFormat::D24PlusStencil8 => wgpu::TextureFormat::Depth24PlusStencil8,
        DepthFormat::D32Float => wgpu::TextureFormat::Depth32Float,
    }
}

fn aspect_ratio(size: SurfaceSize) -> f32 {
    if size.height == 0 {
        1.0
    } else {
        size.width as f32 / size.height as f32
    }
}

fn default_wgpu_pass_count(scene: &RenderScene) -> u32 {
    let lighting = scene.lighting();
    let mut count = 1;
    if lighting.directional_shadow.enabled {
        count += 1;
    }
    count += lighting
        .point_lights()
        .iter()
        .filter(|light| light.shadow.enabled)
        .count() as u32;
    count += lighting
        .spot_lights()
        .iter()
        .filter(|light| light.shadow.enabled)
        .count() as u32;
    if lighting.environment.background_intensity > 0.0 {
        count += 1;
    }
    count
}

fn map_backend_error(error: GraphicsError) -> RendererError {
    match error {
        GraphicsError::SurfaceOutOfMemory => {
            RendererError::OutOfMemory("surface is out of memory".to_owned())
        }
        GraphicsError::Backend(message)
            if message.contains("device was lost") || message.contains("surface was lost") =>
        {
            RendererError::DeviceLost { reason: message }
        }
        error => RendererError::Backend(error.to_string()),
    }
}

fn record_backend_error(device_status: &mut DeviceStatus, error: GraphicsError) -> RendererError {
    let error = map_backend_error(error);
    if matches!(error, RendererError::DeviceLost { .. }) {
        *device_status = DeviceStatus::Lost;
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsync_modes_map_to_wgpu_present_modes() {
        assert_eq!(
            present_mode_for_vsync(VSyncMode::Off),
            PresentMode::Immediate
        );
        assert_eq!(present_mode_for_vsync(VSyncMode::On), PresentMode::Fifo);
        assert_eq!(
            present_mode_for_vsync(VSyncMode::Adaptive),
            PresentMode::AutoVsync
        );
    }

    #[test]
    fn renderer_config_maps_to_wgpu_surface_options() {
        let options = surface_options(&RendererConfig {
            surface_format: Some(TextureFormat::Bgra8UnormSrgb),
            depth_format: DepthFormat::D24Plus,
            ..RendererConfig::default()
        });

        assert_eq!(
            options.preferred_format,
            Some(wgpu::TextureFormat::Bgra8UnormSrgb)
        );
        assert_eq!(options.depth_format, wgpu::TextureFormat::Depth24Plus);
    }

    #[test]
    fn backend_errors_map_to_renderer_status_errors() {
        assert!(matches!(
            map_backend_error(GraphicsError::SurfaceOutOfMemory),
            RendererError::OutOfMemory(_)
        ));
        assert!(matches!(
            map_backend_error(GraphicsError::Backend("surface was lost".to_owned())),
            RendererError::DeviceLost { .. }
        ));

        let mut status = DeviceStatus::Ok;
        let error = record_backend_error(
            &mut status,
            GraphicsError::Backend("device was lost".to_owned()),
        );
        assert!(matches!(error, RendererError::DeviceLost { .. }));
        assert_eq!(status, DeviceStatus::Lost);
    }

    #[test]
    fn wgpu_limits_map_to_renderer_limits() {
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 4096,
            max_texture_array_layers: 128,
            max_bind_groups: 6,
            max_vertex_buffers: 12,
            ..wgpu::Limits::downlevel_defaults()
        };

        assert_eq!(
            renderer_limits_from_wgpu(&limits),
            RendererLimits {
                max_texture_dimension_2d: 4096,
                max_texture_array_layers: 128,
                max_bind_groups: 6,
                max_vertex_buffers: 12,
            }
        );
    }

    #[test]
    fn wgpu_format_support_requires_render_attachment_usage() {
        assert!(format_supports_render_attachment(
            wgpu::TextureFormatFeatures {
                allowed_usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
                flags: wgpu::TextureFormatFeatureFlags::empty(),
            }
        ));
        assert!(!format_supports_render_attachment(
            wgpu::TextureFormatFeatures {
                allowed_usages: wgpu::TextureUsages::TEXTURE_BINDING,
                flags: wgpu::TextureFormatFeatureFlags::empty(),
            }
        ));
    }
}
