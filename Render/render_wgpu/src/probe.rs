use std::sync::mpsc;

use engine_graphics::{Color, GraphicsError, GraphicsResult};
use graphics_wgpu::{wgpu, WgpuGraphics};

use crate::WgpuEnvironmentTexture;

pub const CUBE_FACE_COUNT: usize = 6;
pub const MAX_ENVIRONMENT_PROBE_BLEND: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentProbeDesc {
    pub position: [f32; 3],
    pub near: f32,
    pub far: f32,
    pub clear_color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentProbeVolumeDesc {
    pub position: [f32; 3],
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub blend_distance: f32,
    pub parallax_correction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BakedEnvironmentProbeFormat {
    Rgba8UnormSrgb,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BakedEnvironmentProbeMip {
    pub size: u32,
    pub faces: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BakedEnvironmentProbe {
    pub desc: EnvironmentProbeDesc,
    pub volume: Option<EnvironmentProbeVolumeDesc>,
    pub format: BakedEnvironmentProbeFormat,
    pub size: u32,
    pub mip_level_count: u32,
    pub mips: Vec<BakedEnvironmentProbeMip>,
}

impl BakedEnvironmentProbe {
    pub fn new(
        desc: EnvironmentProbeDesc,
        volume: Option<EnvironmentProbeVolumeDesc>,
        size: u32,
        mips: Vec<BakedEnvironmentProbeMip>,
    ) -> GraphicsResult<Self> {
        let mip_level_count = u32::try_from(mips.len()).map_err(|_| {
            GraphicsError::InvalidResource(
                "baked environment probe has more than u32::MAX mip levels".to_owned(),
            )
        })?;
        let probe = Self {
            desc,
            volume,
            format: BakedEnvironmentProbeFormat::Rgba8UnormSrgb,
            size,
            mip_level_count,
            mips,
        };
        probe.validate()?;
        Ok(probe)
    }

    pub fn validate(&self) -> GraphicsResult<()> {
        if self.size == 0 {
            return Err(GraphicsError::InvalidResource(
                "baked environment probe size must be non-zero".to_owned(),
            ));
        }
        if self.mip_level_count == 0 {
            return Err(GraphicsError::InvalidResource(
                "baked environment probe must contain at least one mip".to_owned(),
            ));
        }
        if self.mip_level_count as usize != self.mips.len() {
            return Err(GraphicsError::InvalidResource(format!(
                "baked environment probe declares {} mips but contains {}",
                self.mip_level_count,
                self.mips.len()
            )));
        }
        let expected_mip_count = cube_mip_level_count(self.size);
        if self.mip_level_count != expected_mip_count {
            return Err(GraphicsError::InvalidResource(format!(
                "baked environment probe size {} requires {} mips but contains {}",
                self.size, expected_mip_count, self.mip_level_count
            )));
        }

        for (mip_level, mip) in self.mips.iter().enumerate() {
            let expected_size = cube_mip_size(self.size, mip_level as u32);
            if mip.size != expected_size {
                return Err(GraphicsError::InvalidResource(format!(
                    "baked environment probe mip {mip_level} has size {} but expected {expected_size}",
                    mip.size
                )));
            }
            if mip.faces.len() != CUBE_FACE_COUNT {
                return Err(GraphicsError::InvalidResource(format!(
                    "baked environment probe mip {mip_level} contains {} faces but expected {CUBE_FACE_COUNT}",
                    mip.faces.len()
                )));
            }
            let expected_len = (expected_size as usize)
                .saturating_mul(expected_size as usize)
                .saturating_mul(4);
            for (face, rgba8) in mip.faces.iter().enumerate() {
                if rgba8.len() != expected_len {
                    return Err(GraphicsError::InvalidResource(format!(
                        "baked environment probe mip {mip_level} face {face} has {} bytes but expected {expected_len}",
                        rgba8.len()
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn face_rgba8(&self, mip_level: u32, face: usize) -> Option<&[u8]> {
        self.mips
            .get(mip_level as usize)
            .and_then(|mip| mip.faces.get(face))
            .map(Vec::as_slice)
    }
}

impl EnvironmentProbeVolumeDesc {
    pub const fn new(position: [f32; 3], bounds_min: [f32; 3], bounds_max: [f32; 3]) -> Self {
        Self {
            position,
            bounds_min,
            bounds_max,
            blend_distance: 1.0,
            parallax_correction: true,
        }
    }

    pub fn from_center_extents(position: [f32; 3], half_extents: [f32; 3]) -> Self {
        Self::new(
            position,
            [
                position[0] - half_extents[0].abs(),
                position[1] - half_extents[1].abs(),
                position[2] - half_extents[2].abs(),
            ],
            [
                position[0] + half_extents[0].abs(),
                position[1] + half_extents[1].abs(),
                position[2] + half_extents[2].abs(),
            ],
        )
    }

    pub const fn with_blend_distance(mut self, blend_distance: f32) -> Self {
        self.blend_distance = blend_distance;
        self
    }

    pub const fn with_parallax_correction(mut self, parallax_correction: bool) -> Self {
        self.parallax_correction = parallax_correction;
        self
    }
}

pub struct EnvironmentProbeVolume<'a> {
    pub probe: &'a WgpuEnvironmentProbe,
    pub desc: EnvironmentProbeVolumeDesc,
}

impl<'a> EnvironmentProbeVolume<'a> {
    pub const fn new(probe: &'a WgpuEnvironmentProbe, desc: EnvironmentProbeVolumeDesc) -> Self {
        Self { probe, desc }
    }
}

#[derive(Clone, Copy)]
pub struct EnvironmentProbeBlend<'a> {
    pub environment: &'a WgpuEnvironmentTexture,
    pub position: [f32; 3],
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub weight: f32,
    pub parallax_correction: bool,
}

impl<'a> EnvironmentProbeBlend<'a> {
    pub fn from_probe(probe: &'a WgpuEnvironmentProbe, desc: EnvironmentProbeVolumeDesc) -> Self {
        Self {
            environment: probe.environment_texture(),
            position: desc.position,
            bounds_min: desc.bounds_min,
            bounds_max: desc.bounds_max,
            weight: 1.0,
            parallax_correction: desc.parallax_correction,
        }
    }

    pub const fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

pub fn select_environment_probe_blend<'a>(
    sample_position: [f32; 3],
    volumes: &[EnvironmentProbeVolume<'a>],
) -> Vec<EnvironmentProbeBlend<'a>> {
    let mut candidates = volumes
        .iter()
        .filter_map(|volume| {
            let weight = probe_volume_weight(sample_position, volume.desc);
            (weight > 0.0).then(|| {
                EnvironmentProbeBlend::from_probe(volume.probe, volume.desc).with_weight(weight)
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        b.weight.total_cmp(&a.weight).then_with(|| {
            distance_squared(sample_position, a.position)
                .total_cmp(&distance_squared(sample_position, b.position))
        })
    });
    candidates.truncate(MAX_ENVIRONMENT_PROBE_BLEND);

    let total_weight = candidates
        .iter()
        .map(|probe| probe.weight.max(0.0))
        .sum::<f32>();
    if total_weight > f32::EPSILON {
        for probe in &mut candidates {
            probe.weight = (probe.weight.max(0.0) / total_weight).clamp(0.0, 1.0);
        }
    }

    candidates
}

impl EnvironmentProbeDesc {
    pub const DEFAULT: Self = Self {
        position: [0.0, 0.0, 0.0],
        near: 0.05,
        far: 50.0,
        clear_color: Color::BLACK,
    };

    pub const fn at(position: [f32; 3]) -> Self {
        Self {
            position,
            ..Self::DEFAULT
        }
    }

    pub const fn with_range(mut self, near: f32, far: f32) -> Self {
        self.near = near;
        self.far = far;
        self
    }

    pub const fn with_clear_color(mut self, clear_color: Color) -> Self {
        self.clear_color = clear_color;
        self
    }
}

fn probe_volume_weight(position: [f32; 3], desc: EnvironmentProbeVolumeDesc) -> f32 {
    let distance = distance_to_aabb(position, desc.bounds_min, desc.bounds_max);
    if distance <= f32::EPSILON {
        return 1.0;
    }

    let blend_distance = desc.blend_distance.max(0.0);
    if blend_distance <= f32::EPSILON || distance >= blend_distance {
        0.0
    } else {
        1.0 - distance / blend_distance
    }
}

fn distance_to_aabb(position: [f32; 3], bounds_min: [f32; 3], bounds_max: [f32; 3]) -> f32 {
    let mut squared = 0.0;
    for axis in 0..3 {
        let min = bounds_min[axis].min(bounds_max[axis]);
        let max = bounds_min[axis].max(bounds_max[axis]);
        let delta = if position[axis] < min {
            min - position[axis]
        } else if position[axis] > max {
            position[axis] - max
        } else {
            0.0
        };
        squared += delta * delta;
    }
    squared.sqrt()
}

fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let x = a[0] - b[0];
    let y = a[1] - b[1];
    let z = a[2] - b[2];
    x * x + y * y + z * z
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadbackChannelOrder {
    Rgba,
    Bgra,
}

fn readback_channel_order(format: wgpu::TextureFormat) -> GraphicsResult<ReadbackChannelOrder> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
            Ok(ReadbackChannelOrder::Rgba)
        }
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            Ok(ReadbackChannelOrder::Bgra)
        }
        other => Err(GraphicsError::InvalidResource(format!(
            "environment probe format {other:?} cannot be baked to RGBA8"
        ))),
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        value
    } else {
        value.div_ceil(alignment) * alignment
    }
}

fn copy_probe_readback_rows_to_rgba8(
    mapped: &[u8],
    size: u32,
    padded_bytes_per_row: u32,
    channel_order: ReadbackChannelOrder,
) -> Vec<u8> {
    let row_bytes = (size * 4) as usize;
    let padded_bytes_per_row = padded_bytes_per_row as usize;
    let mut rgba8 = vec![0; row_bytes * size as usize];

    for row in 0..size as usize {
        let source_start = row * padded_bytes_per_row;
        let source_row = &mapped[source_start..source_start + row_bytes];
        let destination_start = row * row_bytes;
        let destination_row = &mut rgba8[destination_start..destination_start + row_bytes];

        match channel_order {
            ReadbackChannelOrder::Rgba => destination_row.copy_from_slice(source_row),
            ReadbackChannelOrder::Bgra => {
                for (source, destination) in source_row
                    .chunks_exact(4)
                    .zip(destination_row.chunks_exact_mut(4))
                {
                    destination[0] = source[2];
                    destination[1] = source[1];
                    destination[2] = source[0];
                    destination[3] = source[3];
                }
            }
        }
    }

    rgba8
}

impl Default for EnvironmentProbeDesc {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ProbePrefilterUniform {
    face_roughness_size: [f32; 4],
}

unsafe impl bytemuck::Zeroable for ProbePrefilterUniform {}
unsafe impl bytemuck::Pod for ProbePrefilterUniform {}

pub struct WgpuEnvironmentProbe {
    _capture_texture: wgpu::Texture,
    _capture_view: wgpu::TextureView,
    _capture_sampler: wgpu::Sampler,
    capture_face_views: Vec<wgpu::TextureView>,
    _depth_texture: wgpu::Texture,
    depth_face_views: Vec<wgpu::TextureView>,
    environment: WgpuEnvironmentTexture,
    environment_face_mip_views: Vec<Vec<wgpu::TextureView>>,
    prefilter_pipeline: wgpu::RenderPipeline,
    prefilter_bind_group: wgpu::BindGroup,
    prefilter_uniform_buffer: wgpu::Buffer,
    size: u32,
    format: wgpu::TextureFormat,
    depth_format: wgpu::TextureFormat,
}

impl WgpuEnvironmentProbe {
    pub fn new(
        graphics: &WgpuGraphics,
        size: u32,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> GraphicsResult<Self> {
        if size == 0 {
            return Err(GraphicsError::InvalidResource(
                "environment probe size must be non-zero".to_owned(),
            ));
        }

        let device = graphics.device();
        let mip_level_count = cube_mip_level_count(size);
        let capture_texture = create_cube_texture(
            device,
            "Neo Environment Probe Capture Cubemap",
            size,
            1,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let capture_view = capture_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Neo Environment Probe Capture Cubemap View"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(CUBE_FACE_COUNT as u32),
            ..wgpu::TextureViewDescriptor::default()
        });
        let capture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Neo Environment Probe Capture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        let mut capture_face_mip_views = cube_face_views(&capture_texture, 1);
        let capture_face_views = capture_face_mip_views.remove(0);

        let depth_texture = create_cube_texture(
            device,
            "Neo Environment Probe Depth Cubemap",
            size,
            1,
            depth_format,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let mut depth_face_mip_views = cube_face_views(&depth_texture, 1);
        let depth_face_views = depth_face_mip_views.remove(0);

        let environment_texture = create_cube_texture(
            device,
            "Neo Prefiltered Environment Probe Cubemap",
            size,
            mip_level_count,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        );
        let environment_face_mip_views = cube_face_views(&environment_texture, mip_level_count);
        let environment = WgpuEnvironmentTexture::from_texture_resource(
            device,
            environment_texture,
            size,
            mip_level_count,
            "Neo Prefiltered Environment Probe Cubemap View",
            "Neo Prefiltered Environment Probe Cubemap Sampler",
        );

        let prefilter_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Neo Environment Probe Prefilter Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("probe_prefilter.wgsl").into()),
        });
        let prefilter_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Environment Probe Prefilter Uniform Buffer"),
            size: std::mem::size_of::<ProbePrefilterUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let prefilter_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Neo Environment Probe Prefilter Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let prefilter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Neo Environment Probe Prefilter Bind Group"),
            layout: &prefilter_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: prefilter_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&capture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&capture_sampler),
                },
            ],
        });
        let prefilter_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Neo Environment Probe Prefilter Pipeline Layout"),
            bind_group_layouts: &[&prefilter_bind_group_layout],
            push_constant_ranges: &[],
        });
        let prefilter_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Neo Environment Probe Prefilter Pipeline"),
            layout: Some(&prefilter_layout),
            vertex: wgpu::VertexState {
                module: &prefilter_shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &prefilter_shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Ok(Self {
            _capture_texture: capture_texture,
            _capture_view: capture_view,
            _capture_sampler: capture_sampler,
            capture_face_views,
            _depth_texture: depth_texture,
            depth_face_views,
            environment,
            environment_face_mip_views,
            prefilter_pipeline,
            prefilter_bind_group,
            prefilter_uniform_buffer,
            size,
            format,
            depth_format,
        })
    }

    pub fn environment_texture(&self) -> &WgpuEnvironmentTexture {
        &self.environment
    }

    pub const fn size(&self) -> u32 {
        self.size
    }

    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub const fn depth_format(&self) -> wgpu::TextureFormat {
        self.depth_format
    }

    pub fn bake(
        &self,
        graphics: &WgpuGraphics,
        desc: EnvironmentProbeDesc,
        volume: Option<EnvironmentProbeVolumeDesc>,
    ) -> GraphicsResult<BakedEnvironmentProbe> {
        let channel_order = readback_channel_order(self.format)?;
        let mut mips = Vec::with_capacity(self.environment.mip_level_count() as usize);

        for mip_level in 0..self.environment.mip_level_count() {
            let mip_size = cube_mip_size(self.size, mip_level);
            let mut faces = Vec::with_capacity(CUBE_FACE_COUNT);
            for face in 0..CUBE_FACE_COUNT {
                faces.push(self.read_environment_face_mip_rgba8(
                    graphics,
                    mip_level,
                    face,
                    mip_size,
                    channel_order,
                )?);
            }
            mips.push(BakedEnvironmentProbeMip {
                size: mip_size,
                faces,
            });
        }

        BakedEnvironmentProbe::new(desc, volume, self.size, mips)
    }

    pub(crate) fn capture_face_view(&self, face: usize) -> Option<&wgpu::TextureView> {
        self.capture_face_views.get(face)
    }

    pub(crate) fn depth_face_view(&self, face: usize) -> Option<&wgpu::TextureView> {
        self.depth_face_views.get(face)
    }

    fn read_environment_face_mip_rgba8(
        &self,
        graphics: &WgpuGraphics,
        mip_level: u32,
        face: usize,
        mip_size: u32,
        channel_order: ReadbackChannelOrder,
    ) -> GraphicsResult<Vec<u8>> {
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = mip_size * bytes_per_pixel;
        let padded_bytes_per_row =
            align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(mip_size);
        let readback_buffer = graphics.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Neo Baked Environment Probe Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            graphics
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Neo Baked Environment Probe Readback Encoder"),
                });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: self.environment.texture(),
                mip_level,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: face as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(mip_size),
                },
            },
            wgpu::Extent3d {
                width: mip_size,
                height: mip_size,
                depth_or_array_layers: 1,
            },
        );
        graphics.queue().submit(Some(encoder.finish()));

        let buffer_slice = readback_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        graphics.device().poll(wgpu::Maintain::Wait);
        let map_result = receiver.recv().map_err(|_| {
            GraphicsError::Backend("baked environment probe readback was canceled".to_owned())
        })?;
        map_result.map_err(|err| {
            GraphicsError::Backend(format!(
                "baked environment probe readback mapping failed: {err}"
            ))
        })?;

        let mapped = buffer_slice.get_mapped_range();
        let rgba8 = copy_probe_readback_rows_to_rgba8(
            &mapped,
            mip_size,
            padded_bytes_per_row,
            channel_order,
        );
        drop(mapped);
        readback_buffer.unmap();

        Ok(rgba8)
    }

    pub(crate) fn prefilter(&self, graphics: &WgpuGraphics) -> GraphicsResult<()> {
        for mip_level in 0..self.environment.mip_level_count() {
            let mip_size = (self.size >> mip_level).max(1);
            let roughness = mip_level as f32
                / (self.environment.mip_level_count().saturating_sub(1)).max(1) as f32;
            for face in 0..CUBE_FACE_COUNT {
                let target = self
                    .environment_face_mip_views
                    .get(mip_level as usize)
                    .and_then(|faces| faces.get(face))
                    .ok_or_else(|| {
                        GraphicsError::InvalidResource(format!(
                            "environment probe missing face {face} view for mip {mip_level}"
                        ))
                    })?;

                graphics.queue().write_buffer(
                    &self.prefilter_uniform_buffer,
                    0,
                    bytemuck::bytes_of(&ProbePrefilterUniform {
                        face_roughness_size: [
                            face as f32,
                            roughness,
                            mip_size as f32,
                            self.environment.mip_level_count() as f32,
                        ],
                    }),
                );

                let mut encoder =
                    graphics
                        .device()
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Neo Environment Probe Prefilter Encoder"),
                        });
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Neo Environment Probe Prefilter Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: target,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.prefilter_pipeline);
                    pass.set_bind_group(0, &self.prefilter_bind_group, &[]);
                    pass.draw(0..3, 0..1);
                }
                graphics.queue().submit(Some(encoder.finish()));
            }
        }

        Ok(())
    }
}

fn create_cube_texture(
    device: &wgpu::Device,
    label: &'static str,
    size: u32,
    mip_level_count: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.max(1),
            height: size.max(1),
            depth_or_array_layers: CUBE_FACE_COUNT as u32,
        },
        mip_level_count: mip_level_count.max(1),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn cube_face_views(texture: &wgpu::Texture, mip_level_count: u32) -> Vec<Vec<wgpu::TextureView>> {
    (0..mip_level_count.max(1))
        .map(|mip_level| {
            (0..CUBE_FACE_COUNT as u32)
                .map(|face| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        label: Some("Neo Environment Probe Cubemap Face View"),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level: mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: face,
                        array_layer_count: Some(1),
                        ..wgpu::TextureViewDescriptor::default()
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn cube_mip_level_count(size: u32) -> u32 {
    let mut size = size.max(1);
    let mut count = 1;

    while size > 1 {
        size = (size / 2).max(1);
        count += 1;
    }

    count
}

fn cube_mip_size(size: u32, mip_level: u32) -> u32 {
    (size.max(1) >> mip_level).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_desc_defaults_to_capture_origin() {
        let desc = EnvironmentProbeDesc::default();

        assert_eq!(desc.position, [0.0, 0.0, 0.0]);
        assert_eq!(desc.near, 0.05);
        assert_eq!(desc.far, 50.0);
        assert_eq!(desc.clear_color, Color::BLACK);
    }

    #[test]
    fn cube_mip_count_reaches_single_texel() {
        assert_eq!(cube_mip_level_count(1), 1);
        assert_eq!(cube_mip_level_count(2), 2);
        assert_eq!(cube_mip_level_count(128), 8);
    }

    #[test]
    fn baked_probe_validates_complete_mip_chain() {
        let mips = [4, 2, 1]
            .into_iter()
            .map(|size| BakedEnvironmentProbeMip {
                size,
                faces: vec![vec![255; (size * size * 4) as usize]; CUBE_FACE_COUNT],
            })
            .collect::<Vec<_>>();

        let probe = BakedEnvironmentProbe::new(
            EnvironmentProbeDesc::default(),
            Some(EnvironmentProbeVolumeDesc::from_center_extents(
                [0.0, 0.0, 0.0],
                [1.0, 1.0, 1.0],
            )),
            4,
            mips,
        )
        .unwrap();

        assert_eq!(probe.mip_level_count, 3);
        assert_eq!(probe.face_rgba8(2, 5).unwrap().len(), 4);
    }

    #[test]
    fn baked_probe_rejects_missing_cube_face() {
        let result = BakedEnvironmentProbe::new(
            EnvironmentProbeDesc::default(),
            None,
            1,
            vec![BakedEnvironmentProbeMip {
                size: 1,
                faces: vec![vec![0; 4]; CUBE_FACE_COUNT - 1],
            }],
        );

        assert!(result.is_err());
    }

    #[test]
    fn probe_readback_rows_convert_bgra_to_rgba_and_remove_padding() {
        let mut mapped = vec![0; 256 * 2];
        mapped[0..8].copy_from_slice(&[3, 2, 1, 4, 7, 6, 5, 8]);
        mapped[256..264].copy_from_slice(&[11, 10, 9, 12, 15, 14, 13, 16]);

        let rgba8 = copy_probe_readback_rows_to_rgba8(&mapped, 2, 256, ReadbackChannelOrder::Bgra);

        assert_eq!(
            rgba8,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn probe_volume_weight_blends_outside_bounds() {
        let desc =
            EnvironmentProbeVolumeDesc::from_center_extents([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
                .with_blend_distance(2.0);

        assert_eq!(probe_volume_weight([0.0, 0.0, 0.0], desc), 1.0);
        assert!((probe_volume_weight([2.0, 0.0, 0.0], desc) - 0.5).abs() < 0.0001);
        assert_eq!(probe_volume_weight([3.0, 0.0, 0.0], desc), 0.0);
    }
}
