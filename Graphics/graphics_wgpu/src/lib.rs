mod surface;

use std::sync::Arc;

use engine_graphics::{GraphicsError, GraphicsResult};
use engine_platform::PlatformWindow;

pub use surface::{
    WgpuFrameContext, WgpuFrameReadback, WgpuSurface, WgpuSurfaceOptions, DEFAULT_DEPTH_FORMAT,
    DEFAULT_SAMPLE_COUNT,
};
pub use wgpu;

#[derive(Debug, Clone, Copy)]
pub struct WgpuGraphicsOptions {
    pub power_preference: wgpu::PowerPreference,
    pub force_fallback_adapter: bool,
}

impl Default for WgpuGraphicsOptions {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
        }
    }
}

pub struct WgpuGraphics {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl WgpuGraphics {
    pub fn new(options: WgpuGraphicsOptions) -> GraphicsResult<Self> {
        pollster::block_on(Self::new_async(options))
    }

    pub async fn new_async(options: WgpuGraphicsOptions) -> GraphicsResult<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.power_preference,
                compatible_surface: None,
                force_fallback_adapter: options.force_fallback_adapter,
            })
            .await
            .ok_or(GraphicsError::AdapterNotFound)?;

        let mut required_features = wgpu::Features::empty();
        if adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            required_features |=
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }
        if adapter
            .features()
            .contains(wgpu::Features::VERTEX_ATTRIBUTE_64BIT)
        {
            required_features |= wgpu::Features::VERTEX_ATTRIBUTE_64BIT;
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Neo WGPU Device"),
                    required_features,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .map_err(|err| GraphicsError::DeviceCreationFailed(err.to_string()))?;

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    pub fn create_surface(
        &self,
        window: &dyn PlatformWindow,
        size: engine_graphics::SurfaceSize,
    ) -> GraphicsResult<WgpuSurface> {
        self.create_surface_with_options(window, size, WgpuSurfaceOptions::default())
    }

    pub fn create_surface_with_options(
        &self,
        window: &dyn PlatformWindow,
        size: engine_graphics::SurfaceSize,
        options: WgpuSurfaceOptions,
    ) -> GraphicsResult<WgpuSurface> {
        WgpuSurface::new_with_options(
            &self.instance,
            &self.adapter,
            self.device.clone(),
            self.queue.clone(),
            window,
            size,
            options,
        )
    }

    pub fn device(&self) -> &wgpu::Device {
        self.device.as_ref()
    }

    pub fn device_handle(&self) -> Arc<wgpu::Device> {
        self.device.clone()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        self.queue.as_ref()
    }

    pub fn queue_handle(&self) -> Arc<wgpu::Queue> {
        self.queue.clone()
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }
}
