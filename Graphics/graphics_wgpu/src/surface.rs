use std::sync::{mpsc, Arc};

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
    copy_src_supported: bool,
    frame_readback_enabled: bool,
    pending_frame_readback: Option<PendingSurfaceFrameReadback>,
    last_frame_readback: Option<WgpuFrameReadback>,
    last_submission_index: Option<wgpu::SubmissionIndex>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WgpuFrameReadback {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub bytes_per_row: u32,
    pub rows_per_image: u32,
    pub bytes: Vec<u8>,
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

        let copy_src_supported = capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let usage = if copy_src_supported {
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
        } else {
            wgpu::TextureUsages::RENDER_ATTACHMENT
        };

        let mut surface = Self {
            surface,
            device,
            queue,
            config: wgpu::SurfaceConfiguration {
                usage,
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
            copy_src_supported,
            frame_readback_enabled: false,
            pending_frame_readback: None,
            last_frame_readback: None,
            last_submission_index: None,
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

    pub fn last_submission_index(&self) -> Option<wgpu::SubmissionIndex> {
        self.last_submission_index.clone()
    }

    pub fn surface_readback_supported(&self) -> bool {
        self.copy_src_supported
    }

    pub fn frame_readback_enabled(&self) -> bool {
        self.frame_readback_enabled
    }

    pub fn set_frame_readback_enabled(&mut self, enabled: bool) -> GraphicsResult<()> {
        if enabled && !self.copy_src_supported {
            return Err(GraphicsError::SurfaceConfigurationFailed(
                "surface does not support COPY_SRC usage for frame readback".to_owned(),
            ));
        }
        self.frame_readback_enabled = enabled;
        if !enabled {
            self.pending_frame_readback = None;
            self.last_frame_readback = None;
        }
        Ok(())
    }

    pub fn last_frame_readback(&self) -> Option<&WgpuFrameReadback> {
        self.last_frame_readback.as_ref()
    }

    pub fn has_pending_frame_readback(&self) -> bool {
        self.pending_frame_readback.is_some()
    }

    pub fn resolve_pending_frame_readback(&mut self) -> GraphicsResult<bool> {
        let Some(readback) = self.pending_frame_readback.take() else {
            return Ok(false);
        };
        self.last_frame_readback = Some(read_surface_frame_buffer(&self.device, readback)?);
        Ok(true)
    }

    pub fn try_resolve_pending_frame_readback(&mut self) -> GraphicsResult<bool> {
        let Some(readback) = self.pending_frame_readback.take() else {
            return Ok(false);
        };
        match try_read_surface_frame_buffer(&self.device, &readback)? {
            Some(frame) => {
                self.last_frame_readback = Some(frame);
                Ok(true)
            }
            None => {
                self.pending_frame_readback = Some(readback);
                Ok(false)
            }
        }
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

    pub fn device(&self) -> &wgpu::Device {
        self.device.as_ref()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.queue.as_ref()
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

        let pending_readback = if self.frame_readback_enabled {
            prepare_surface_frame_readback(
                &self.device,
                &mut encoder,
                &frame.texture,
                self.config.width,
                self.config.height,
                self.config.format,
            )
            .ok()
        } else {
            None
        };

        let submission = self.queue.submit(Some(encoder.finish()));
        self.last_submission_index = Some(submission.clone());
        frame.present();
        self.pending_frame_readback = pending_readback.map(|mut readback| {
            readback.submission = Some(submission);
            begin_surface_frame_buffer_map(&mut readback);
            readback
        });
        if self.pending_frame_readback.is_some() {
            self.last_frame_readback = None;
        }
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
            self.pending_frame_readback = None;
            self.last_frame_readback = None;
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

struct PendingSurfaceFrameReadback {
    submission: Option<wgpu::SubmissionIndex>,
    map_receiver: Option<mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    row_bytes: u32,
    padded_row_bytes: u32,
}

fn prepare_surface_frame_readback(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> GraphicsResult<PendingSurfaceFrameReadback> {
    let bytes_per_pixel = surface_readback_bytes_per_pixel(format).ok_or_else(|| {
        GraphicsError::InvalidResource(format!(
            "surface format {format:?} is not supported for frame readback"
        ))
    })?;
    let row_bytes = width.checked_mul(bytes_per_pixel).ok_or_else(|| {
        GraphicsError::InvalidResource("surface frame readback row size overflows".to_owned())
    })?;
    let padded_row_bytes = align_to(row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback_size = u64::from(padded_row_bytes)
        .checked_mul(u64::from(height))
        .ok_or_else(|| {
            GraphicsError::InvalidResource(
                "surface frame readback buffer size overflows".to_owned(),
            )
        })?;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Neo Surface Frame Readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    Ok(PendingSurfaceFrameReadback {
        submission: None,
        map_receiver: None,
        buffer,
        width,
        height,
        format,
        row_bytes,
        padded_row_bytes,
    })
}

fn read_surface_frame_buffer(
    device: &wgpu::Device,
    mut readback: PendingSurfaceFrameReadback,
) -> GraphicsResult<WgpuFrameReadback> {
    if readback.map_receiver.is_none() {
        begin_surface_frame_buffer_map(&mut readback);
    }
    if let Some(submission) = readback.submission.clone() {
        let _ = device.poll(wgpu::Maintain::WaitForSubmissionIndex(submission));
    } else {
        let _ = device.poll(wgpu::Maintain::Wait);
    }
    let _ = device.poll(wgpu::Maintain::Wait);
    let receiver = readback.map_receiver.take().ok_or_else(|| {
        GraphicsError::Backend("surface frame readback mapping was not started".to_owned())
    })?;
    match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Err(GraphicsError::Backend(format!(
                "surface frame readback mapping failed: {error}"
            )));
        }
        Err(_) => {
            return Err(GraphicsError::Backend(
                "surface frame readback callback was canceled".to_owned(),
            ));
        }
    }
    copy_surface_frame_readback_bytes(&readback)
}

fn try_read_surface_frame_buffer(
    device: &wgpu::Device,
    readback: &PendingSurfaceFrameReadback,
) -> GraphicsResult<Option<WgpuFrameReadback>> {
    let Some(receiver) = readback.map_receiver.as_ref() else {
        return Err(GraphicsError::Backend(
            "surface frame readback mapping was not started".to_owned(),
        ));
    };
    let _ = device.poll(wgpu::Maintain::Poll);
    match receiver.try_recv() {
        Ok(Ok(())) => copy_surface_frame_readback_bytes(readback).map(Some),
        Ok(Err(error)) => Err(GraphicsError::Backend(format!(
            "surface frame readback mapping failed: {error}"
        ))),
        Err(mpsc::TryRecvError::Empty) => Ok(None),
        Err(mpsc::TryRecvError::Disconnected) => Err(GraphicsError::Backend(
            "surface frame readback callback was canceled".to_owned(),
        )),
    }
}

fn begin_surface_frame_buffer_map(readback: &mut PendingSurfaceFrameReadback) {
    let slice = readback.buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    readback.map_receiver = Some(receiver);
}

fn copy_surface_frame_readback_bytes(
    readback: &PendingSurfaceFrameReadback,
) -> GraphicsResult<WgpuFrameReadback> {
    let slice = readback.buffer.slice(..);
    let mapped = slice.get_mapped_range();
    let mut bytes = Vec::with_capacity(readback.row_bytes as usize * readback.height as usize);
    for row in 0..readback.height as usize {
        let row_start = row * readback.padded_row_bytes as usize;
        let row_end = row_start + readback.row_bytes as usize;
        bytes.extend_from_slice(&mapped[row_start..row_end]);
    }
    drop(mapped);
    readback.buffer.unmap();
    Ok(WgpuFrameReadback {
        width: readback.width,
        height: readback.height,
        format: readback.format,
        bytes_per_row: readback.row_bytes,
        rows_per_image: readback.height,
        bytes,
    })
}

fn surface_readback_bytes_per_pixel(format: wgpu::TextureFormat) -> Option<u32> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb => Some(4),
        wgpu::TextureFormat::Rgba16Float => Some(8),
        wgpu::TextureFormat::Rgba32Float => Some(16),
        _ => None,
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return value;
    }
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
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

    #[test]
    fn surface_readback_layout_supports_public_color_formats() {
        assert_eq!(
            surface_readback_bytes_per_pixel(wgpu::TextureFormat::Rgba8UnormSrgb),
            Some(4)
        );
        assert_eq!(
            surface_readback_bytes_per_pixel(wgpu::TextureFormat::Bgra8UnormSrgb),
            Some(4)
        );
        assert_eq!(
            surface_readback_bytes_per_pixel(wgpu::TextureFormat::Rgba16Float),
            Some(8)
        );
        assert_eq!(
            surface_readback_bytes_per_pixel(wgpu::TextureFormat::Rgba32Float),
            Some(16)
        );
        assert_eq!(
            surface_readback_bytes_per_pixel(wgpu::TextureFormat::Depth32Float),
            None
        );
        assert_eq!(align_to(8, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT), 256);
        assert_eq!(align_to(256, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT), 256);
    }
}
