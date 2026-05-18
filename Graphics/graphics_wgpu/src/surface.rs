use std::sync::Arc;

use engine_graphics::{
    Color, GraphicsError, GraphicsResult, PresentMode, RenderSurface, SurfaceSize,
};
use engine_platform::PlatformWindow;

pub const DEFAULT_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
pub const DEFAULT_SAMPLE_COUNT: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WgpuSurfaceOptions {
    pub preferred_format: Option<wgpu::TextureFormat>,
    pub depth_format: wgpu::TextureFormat,
}

impl Default for WgpuSurfaceOptions {
    fn default() -> Self {
        Self {
            preferred_format: None,
            depth_format: DEFAULT_DEPTH_FORMAT,
        }
    }
}

pub struct WgpuSurface {
    surface: wgpu::Surface<'static>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    depth_format: wgpu::TextureFormat,
    supported_present_modes: Vec<wgpu::PresentMode>,
    msaa_color_target: Option<WgpuColorTarget>,
    depth_target: Option<WgpuDepthTarget>,
    sample_count: u32,
    supported_sample_counts: Vec<u32>,
    size: SurfaceSize,
}

pub struct WgpuFrameContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub resolve_target: Option<&'a wgpu::TextureView>,
    pub depth_view: Option<&'a wgpu::TextureView>,
    pub format: wgpu::TextureFormat,
    pub depth_format: wgpu::TextureFormat,
    pub sample_count: u32,
}

struct WgpuColorTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct WgpuDepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl WgpuSurface {
    pub fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        window: &dyn PlatformWindow,
        size: SurfaceSize,
    ) -> GraphicsResult<Self> {
        Self::new_with_options(
            instance,
            adapter,
            device,
            queue,
            window,
            size,
            WgpuSurfaceOptions::default(),
        )
    }

    pub fn new_with_options(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        window: &dyn PlatformWindow,
        size: SurfaceSize,
        options: WgpuSurfaceOptions,
    ) -> GraphicsResult<Self> {
        let raw_display_handle = window
            .display_handle()
            .map_err(|err| GraphicsError::SurfaceCreationFailed(err.to_string()))?
            .as_raw();
        let raw_window_handle = window
            .window_handle()
            .map_err(|err| GraphicsError::SurfaceCreationFailed(err.to_string()))?
            .as_raw();
        let surface_target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle,
            raw_window_handle,
        };

        // The caller creates this surface from a live PlatformWindow and must drop
        // the surface before destroying that window.
        let surface = unsafe { instance.create_surface_unsafe(surface_target) }
            .map_err(|err| GraphicsError::SurfaceCreationFailed(err.to_string()))?;

        let capabilities = surface.get_capabilities(adapter);
        let format = choose_surface_format(&capabilities.formats, options.preferred_format)?;
        let present_mode = choose_present_mode(&capabilities, PresentMode::Fifo);
        let alpha_mode = capabilities.alpha_modes.first().copied().ok_or_else(|| {
            GraphicsError::SurfaceConfigurationFailed(
                "surface reported no compatible alpha modes".to_owned(),
            )
        })?;
        let supported_sample_counts =
            supported_surface_sample_counts(adapter, format, options.depth_format);

        let mut surface = Self {
            surface,
            device,
            queue,
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
            depth_format: options.depth_format,
            supported_present_modes: capabilities.present_modes,
            msaa_color_target: None,
            depth_target: None,
            sample_count: DEFAULT_SAMPLE_COUNT,
            supported_sample_counts,
            size,
        };

        surface.configure();
        Ok(surface)
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }

    pub fn depth_view(&self) -> Option<&wgpu::TextureView> {
        self.depth_target.as_ref().map(|target| &target.view)
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn set_present_mode(&mut self, present_mode: PresentMode) -> GraphicsResult<()> {
        let present_mode =
            choose_present_mode_from_modes(&self.supported_present_modes, present_mode);
        if self.config.present_mode == present_mode {
            return Ok(());
        }

        self.config.present_mode = present_mode;
        if !self.size.is_empty() {
            self.configure();
        }
        Ok(())
    }

    pub fn set_frame_latency(&mut self, frame_latency: u32) -> GraphicsResult<()> {
        if frame_latency == 0 {
            return Err(GraphicsError::SurfaceConfigurationFailed(
                "surface frame latency must be non-zero".to_owned(),
            ));
        }
        if self.config.desired_maximum_frame_latency == frame_latency {
            return Ok(());
        }

        self.config.desired_maximum_frame_latency = frame_latency;
        if !self.size.is_empty() {
            self.configure();
        }
        Ok(())
    }

    pub fn supported_sample_counts(&self) -> &[u32] {
        &self.supported_sample_counts
    }

    pub fn set_sample_count(&mut self, sample_count: u32) -> GraphicsResult<()> {
        if !self.supported_sample_counts.contains(&sample_count) {
            return Err(GraphicsError::SurfaceConfigurationFailed(format!(
                "sample count {sample_count} is not supported for surface format {:?} and depth format {:?}; supported counts: {:?}",
                self.config.format,
                self.depth_format,
                self.supported_sample_counts
            )));
        }

        if self.sample_count == sample_count {
            return Ok(());
        }

        self.sample_count = sample_count;
        if !self.size.is_empty() {
            self.configure();
        }
        Ok(())
    }

    pub fn render_frame<F>(&mut self, label: &str, mut encode: F) -> GraphicsResult<()>
    where
        F: FnMut(WgpuFrameContext<'_>),
    {
        if self.size.is_empty() {
            return Ok(());
        }

        match self.render_current_frame(label, &mut encode) {
            Ok(()) => Ok(()),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.configure();
                self.render_current_frame(label, &mut encode)
                    .map_err(map_surface_error)
            }
            Err(error) => Err(map_surface_error(error)),
        }
    }

    fn configure(&mut self) {
        self.config.width = self.size.width.max(1);
        self.config.height = self.size.height.max(1);
        self.surface.configure(&self.device, &self.config);
        let size = SurfaceSize::new(self.config.width, self.config.height);
        self.msaa_color_target = (self.sample_count > 1).then(|| {
            WgpuColorTarget::new(&self.device, size, self.config.format, self.sample_count)
        });
        self.depth_target = Some(WgpuDepthTarget::new(
            &self.device,
            size,
            self.depth_format,
            self.sample_count,
        ));
    }

    fn render_current_frame<F>(
        &mut self,
        label: &str,
        encode: &mut F,
    ) -> Result<(), wgpu::SurfaceError>
    where
        F: FnMut(WgpuFrameContext<'_>),
    {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_view = self
            .msaa_color_target
            .as_ref()
            .map_or(&view, |target| &target.view);
        let resolve_target = self.msaa_color_target.as_ref().map(|_| &view);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        let depth_view = self.depth_view();

        encode(WgpuFrameContext {
            device: &self.device,
            queue: &self.queue,
            encoder: &mut encoder,
            view: render_view,
            resolve_target,
            depth_view,
            format: self.config.format,
            depth_format: self.depth_format,
            sample_count: self.sample_count,
        });

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn clear_current_frame(&mut self, color: Color) -> GraphicsResult<()> {
        self.render_frame("Neo Clear Encoder", |frame| {
            let _pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Neo Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: frame.view,
                        resolve_target: frame.resolve_target,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu_color(color)),
                            store: store_op_for_resolve(frame.resolve_target),
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
        })
    }
}

impl RenderSurface for WgpuSurface {
    fn size(&self) -> SurfaceSize {
        self.size
    }

    fn resize(&mut self, size: SurfaceSize) -> GraphicsResult<()> {
        self.size = size;

        if !size.is_empty() {
            self.configure();
        } else {
            self.depth_target = None;
            self.msaa_color_target = None;
        }

        Ok(())
    }

    fn clear(&mut self, color: Color) -> GraphicsResult<()> {
        self.clear_current_frame(color)
    }
}

impl WgpuColorTarget {
    fn new(
        device: &wgpu::Device,
        size: SurfaceSize,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Neo Surface MSAA Color Texture"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _texture: texture,
            view,
        }
    }
}

impl WgpuDepthTarget {
    fn new(
        device: &wgpu::Device,
        size: SurfaceSize,
        format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Neo Surface Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _texture: texture,
            view,
        }
    }
}

fn supported_surface_sample_counts(
    adapter: &wgpu::Adapter,
    color_format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
) -> Vec<u32> {
    let color_flags = adapter.get_texture_format_features(color_format).flags;
    let depth_flags = adapter.get_texture_format_features(depth_format).flags;
    supported_surface_sample_counts_from_flags(color_flags, depth_flags)
}

fn supported_surface_sample_counts_from_flags(
    color_flags: wgpu::TextureFormatFeatureFlags,
    depth_flags: wgpu::TextureFormatFeatureFlags,
) -> Vec<u32> {
    [1, 2, 4, 8, 16]
        .into_iter()
        .filter(|sample_count| {
            *sample_count == 1
                || (color_flags.sample_count_supported(*sample_count)
                    && color_flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
                    && depth_flags.sample_count_supported(*sample_count))
        })
        .collect()
}

fn choose_surface_format(
    supported_formats: &[wgpu::TextureFormat],
    preferred: Option<wgpu::TextureFormat>,
) -> GraphicsResult<wgpu::TextureFormat> {
    if let Some(preferred) = preferred {
        if supported_formats.contains(&preferred) {
            return Ok(preferred);
        }
        return Err(GraphicsError::SurfaceConfigurationFailed(format!(
            "surface format {preferred:?} is not supported; supported formats: {supported_formats:?}"
        )));
    }
    supported_formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| supported_formats.first().copied())
        .ok_or_else(|| {
            GraphicsError::SurfaceConfigurationFailed(
                "surface reported no compatible texture formats".to_owned(),
            )
        })
}

fn store_op_for_resolve(resolve_target: Option<&wgpu::TextureView>) -> wgpu::StoreOp {
    if resolve_target.is_some() {
        wgpu::StoreOp::Discard
    } else {
        wgpu::StoreOp::Store
    }
}

fn wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn choose_present_mode(
    capabilities: &wgpu::SurfaceCapabilities,
    requested: PresentMode,
) -> wgpu::PresentMode {
    choose_present_mode_from_modes(&capabilities.present_modes, requested)
}

fn choose_present_mode_from_modes(
    present_modes: &[wgpu::PresentMode],
    requested: PresentMode,
) -> wgpu::PresentMode {
    let requested = match requested {
        PresentMode::Fifo => wgpu::PresentMode::Fifo,
        PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
        PresentMode::Immediate => wgpu::PresentMode::Immediate,
        PresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
        PresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
    };

    if present_modes.contains(&requested) {
        requested
    } else {
        wgpu::PresentMode::Fifo
    }
}

fn map_surface_error(error: wgpu::SurfaceError) -> GraphicsError {
    match error {
        wgpu::SurfaceError::OutOfMemory => GraphicsError::SurfaceOutOfMemory,
        wgpu::SurfaceError::Timeout => GraphicsError::SurfaceTimeout,
        wgpu::SurfaceError::Lost => GraphicsError::Backend("surface was lost".to_owned()),
        wgpu::SurfaceError::Outdated => GraphicsError::Backend("surface is outdated".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_counts_require_color_resolve_and_depth_support() {
        let color_flags = wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2
            | wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4
            | wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE;
        let depth_flags = wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4;

        assert_eq!(
            supported_surface_sample_counts_from_flags(color_flags, depth_flags),
            vec![1, 4]
        );
    }

    #[test]
    fn sample_counts_fall_back_to_single_sample_without_resolve() {
        let color_flags = wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4;
        let depth_flags = wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4;

        assert_eq!(
            supported_surface_sample_counts_from_flags(color_flags, depth_flags),
            vec![1]
        );
    }

    #[test]
    fn present_mode_selection_honors_supported_modes_and_falls_back_to_fifo() {
        assert_eq!(
            choose_present_mode_from_modes(
                &[wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate],
                PresentMode::Immediate,
            ),
            wgpu::PresentMode::Immediate
        );
        assert_eq!(
            choose_present_mode_from_modes(&[wgpu::PresentMode::Fifo], PresentMode::Mailbox),
            wgpu::PresentMode::Fifo
        );
    }

    #[test]
    fn surface_format_selection_honors_preference_and_default_srgb() {
        let formats = [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ];
        assert_eq!(
            choose_surface_format(&formats, None).unwrap(),
            wgpu::TextureFormat::Bgra8UnormSrgb
        );
        assert_eq!(
            choose_surface_format(&formats, Some(wgpu::TextureFormat::Rgba8Unorm)).unwrap(),
            wgpu::TextureFormat::Rgba8Unorm
        );
        assert!(matches!(
            choose_surface_format(&formats, Some(wgpu::TextureFormat::Rgba16Float)),
            Err(GraphicsError::SurfaceConfigurationFailed(_))
        ));
    }
}
