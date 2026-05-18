use engine_graphics::{GraphicsError, GraphicsResult};
use engine_render::{Texture, TextureSize};
use graphics_wgpu::{wgpu, WgpuGraphics};

use crate::probe::{BakedEnvironmentProbe, BakedEnvironmentProbeFormat, CUBE_FACE_COUNT};

const BRDF_LUT_SAMPLE_COUNT: u32 = 128;
const ENVIRONMENT_PREFILTER_SAMPLE_COUNT: u32 = 32;

pub struct WgpuTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    mip_level_count: u32,
}

pub struct WgpuEnvironmentTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    mip_level_count: u32,
    size: u32,
}

impl WgpuTexture {
    pub fn from_texture(graphics: &WgpuGraphics, texture: &Texture) -> GraphicsResult<Self> {
        Self::from_texture_with_format(graphics, texture, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    pub(crate) fn from_texture_with_format(
        graphics: &WgpuGraphics,
        texture: &Texture,
        format: wgpu::TextureFormat,
    ) -> GraphicsResult<Self> {
        Self::from_texture_with_options(graphics, texture, TextureUploadOptions::material(format))
    }

    pub fn from_environment_texture(
        graphics: &WgpuGraphics,
        texture: &Texture,
    ) -> GraphicsResult<WgpuEnvironmentTexture> {
        WgpuEnvironmentTexture::from_texture(graphics, texture)
    }

    pub(crate) fn brdf_lut(graphics: &WgpuGraphics, size: u32) -> GraphicsResult<Self> {
        if size == 0 {
            return Err(GraphicsError::InvalidResource(
                "BRDF LUT dimensions must be non-zero".to_owned(),
            ));
        }
        let texture_size = TextureSize::new(size, size);
        let texture =
            Texture::rgba8(texture_size, generate_brdf_lut_rgba8(size)).ok_or_else(|| {
                GraphicsError::InvalidResource(
                    "BRDF LUT data length does not match size".to_owned(),
                )
            })?;
        Self::from_texture_with_options(graphics, &texture, TextureUploadOptions::brdf_lut())
    }

    fn from_texture_with_options(
        graphics: &WgpuGraphics,
        texture: &Texture,
        options: TextureUploadOptions,
    ) -> GraphicsResult<Self> {
        let size = texture.size();
        if size.width == 0 || size.height == 0 {
            return Err(GraphicsError::InvalidResource(
                "texture dimensions must be non-zero".to_owned(),
            ));
        }
        let mips = if options.prefilter_environment {
            generate_environment_rgba8_mips(size, texture.rgba8_data())?
        } else if options.generate_mips {
            generate_rgba8_mips(size, texture.rgba8_data())?
        } else {
            vec![Rgba8Mip {
                size,
                rgba8: texture.rgba8_data().to_vec(),
            }]
        };
        let mip_level_count = u32::try_from(mips.len()).map_err(|_| {
            GraphicsError::InvalidResource("texture has more than u32::MAX mip levels".to_owned())
        })?;

        let gpu_size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let gpu_texture = graphics.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(options.label),
            size: gpu_size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: options.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (mip_level, mip) in mips.iter().enumerate() {
            graphics.queue().write_texture(
                wgpu::ImageCopyTexture {
                    texture: &gpu_texture,
                    mip_level: mip_level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &mip.rgba8,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(mip.size.width * 4),
                    rows_per_image: Some(mip.size.height),
                },
                wgpu::Extent3d {
                    width: mip.size.width,
                    height: mip.size.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = graphics.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some(options.sampler_label),
            address_mode_u: options.address_mode_u,
            address_mode_v: options.address_mode_v,
            address_mode_w: options.address_mode_w,
            mag_filter: options.mag_filter,
            min_filter: options.min_filter,
            mipmap_filter: options.mipmap_filter,
            ..wgpu::SamplerDescriptor::default()
        });

        Ok(Self {
            _texture: gpu_texture,
            view,
            _sampler: sampler,
            mip_level_count,
        })
    }

    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub const fn mip_level_count(&self) -> u32 {
        self.mip_level_count
    }
}

impl WgpuEnvironmentTexture {
    pub fn from_baked_probe(
        graphics: &WgpuGraphics,
        probe: &BakedEnvironmentProbe,
    ) -> GraphicsResult<Self> {
        probe.validate()?;

        let format = match probe.format {
            BakedEnvironmentProbeFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        };
        let gpu_texture = graphics.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("Neo Baked Environment Probe Cubemap"),
            size: wgpu::Extent3d {
                width: probe.size,
                height: probe.size,
                depth_or_array_layers: CUBE_FACE_COUNT as u32,
            },
            mip_level_count: probe.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (mip_level, mip) in probe.mips.iter().enumerate() {
            for (face, rgba8) in mip.faces.iter().enumerate() {
                graphics.queue().write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &gpu_texture,
                        mip_level: mip_level as u32,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: face as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    rgba8,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(mip.size * 4),
                        rows_per_image: Some(mip.size),
                    },
                    wgpu::Extent3d {
                        width: mip.size,
                        height: mip.size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        Ok(Self::from_texture_resource(
            graphics.device(),
            gpu_texture,
            probe.size,
            probe.mip_level_count,
            "Neo Baked Environment Probe Cubemap View",
            "Neo Baked Environment Probe Cubemap Sampler",
        ))
    }

    pub fn from_texture(graphics: &WgpuGraphics, texture: &Texture) -> GraphicsResult<Self> {
        let size = texture.size();
        if size.width == 0 || size.height == 0 {
            return Err(GraphicsError::InvalidResource(
                "environment texture dimensions must be non-zero".to_owned(),
            ));
        }
        if size.byte_len() != Some(texture.rgba8_data().len()) {
            return Err(GraphicsError::InvalidResource(
                "environment texture data length does not match dimensions".to_owned(),
            ));
        }

        let mips = generate_environment_cube_rgba8_mips(size, texture.rgba8_data())?;
        let face_size = mips.first().map_or(1, |mip| mip.size).max(1);
        let mip_level_count = u32::try_from(mips.len()).map_err(|_| {
            GraphicsError::InvalidResource(
                "environment texture has more than u32::MAX mip levels".to_owned(),
            )
        })?;
        let gpu_texture = graphics.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("Neo Environment Cubemap"),
            size: wgpu::Extent3d {
                width: face_size,
                height: face_size,
                depth_or_array_layers: CUBE_FACE_COUNT as u32,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (mip_level, mip) in mips.iter().enumerate() {
            for (face, rgba8) in mip.faces.iter().enumerate() {
                graphics.queue().write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &gpu_texture,
                        mip_level: mip_level as u32,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: face as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    rgba8,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(mip.size * 4),
                        rows_per_image: Some(mip.size),
                    },
                    wgpu::Extent3d {
                        width: mip.size,
                        height: mip.size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        Ok(Self::from_texture_resource(
            graphics.device(),
            gpu_texture,
            face_size,
            mip_level_count,
            "Neo Environment Cubemap View",
            "Neo Environment Cubemap Sampler",
        ))
    }

    pub(crate) fn from_texture_resource(
        device: &wgpu::Device,
        texture: wgpu::Texture,
        size: u32,
        mip_level_count: u32,
        view_label: &'static str,
        sampler_label: &'static str,
    ) -> Self {
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(view_label),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count.max(1)),
            base_array_layer: 0,
            array_layer_count: Some(CUBE_FACE_COUNT as u32),
            ..wgpu::TextureViewDescriptor::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(sampler_label),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
            mip_level_count: mip_level_count.max(1),
            size: size.max(1),
        }
    }

    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self._texture
    }

    pub(crate) fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    pub const fn mip_level_count(&self) -> u32 {
        self.mip_level_count
    }

    pub const fn size(&self) -> u32 {
        self.size
    }
}

#[derive(Clone, Copy)]
struct TextureUploadOptions {
    label: &'static str,
    sampler_label: &'static str,
    format: wgpu::TextureFormat,
    generate_mips: bool,
    prefilter_environment: bool,
    address_mode_u: wgpu::AddressMode,
    address_mode_v: wgpu::AddressMode,
    address_mode_w: wgpu::AddressMode,
    mag_filter: wgpu::FilterMode,
    min_filter: wgpu::FilterMode,
    mipmap_filter: wgpu::FilterMode,
}

impl TextureUploadOptions {
    fn material(format: wgpu::TextureFormat) -> Self {
        Self {
            label: "Neo Texture",
            sampler_label: "Neo Texture Sampler",
            format,
            generate_mips: true,
            prefilter_environment: false,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
        }
    }

    fn brdf_lut() -> Self {
        Self {
            label: "Neo Environment BRDF LUT",
            sampler_label: "Neo Environment BRDF LUT Sampler",
            format: wgpu::TextureFormat::Rgba8Unorm,
            generate_mips: false,
            prefilter_environment: false,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
        }
    }
}

struct Rgba8Mip {
    size: TextureSize,
    rgba8: Vec<u8>,
}

struct CubeRgba8Mip {
    size: u32,
    faces: Vec<Vec<u8>>,
}

fn generate_rgba8_mips(size: TextureSize, rgba8: &[u8]) -> GraphicsResult<Vec<Rgba8Mip>> {
    if size.width == 0 || size.height == 0 {
        return Err(GraphicsError::InvalidResource(
            "texture dimensions must be non-zero".to_owned(),
        ));
    }
    if size.byte_len() != Some(rgba8.len()) {
        return Err(GraphicsError::InvalidResource(
            "texture data length does not match dimensions".to_owned(),
        ));
    }

    let mut mips = vec![Rgba8Mip {
        size,
        rgba8: rgba8.to_vec(),
    }];

    while mips
        .last()
        .map(|mip| mip.size.width > 1 || mip.size.height > 1)
        .unwrap_or(false)
    {
        let next_mip = {
            let previous = mips.last().expect("mip chain has at least one level");
            generate_next_rgba8_mip(previous)
        };
        mips.push(next_mip);
    }

    Ok(mips)
}

fn generate_next_rgba8_mip(previous: &Rgba8Mip) -> Rgba8Mip {
    let next_size = TextureSize::new(
        (previous.size.width / 2).max(1),
        (previous.size.height / 2).max(1),
    );
    let mut rgba8 = vec![0; next_size.byte_len().unwrap_or(0)];

    for y in 0..next_size.height {
        let source_y_start = y * previous.size.height / next_size.height;
        let source_y_end = ((y + 1) * previous.size.height / next_size.height)
            .max(source_y_start + 1)
            .min(previous.size.height);

        for x in 0..next_size.width {
            let source_x_start = x * previous.size.width / next_size.width;
            let source_x_end = ((x + 1) * previous.size.width / next_size.width)
                .max(source_x_start + 1)
                .min(previous.size.width);
            let mut channels = [0u32; 4];
            let mut sample_count = 0u32;

            for source_y in source_y_start..source_y_end {
                for source_x in source_x_start..source_x_end {
                    let source_offset = rgba8_offset(previous.size, source_x, source_y);
                    for (channel, value) in channels.iter_mut().enumerate() {
                        *value += u32::from(previous.rgba8[source_offset + channel]);
                    }
                    sample_count += 1;
                }
            }

            let destination_offset = rgba8_offset(next_size, x, y);
            for (channel, sum) in channels.into_iter().enumerate() {
                rgba8[destination_offset + channel] =
                    ((sum + sample_count / 2) / sample_count) as u8;
            }
        }
    }

    Rgba8Mip {
        size: next_size,
        rgba8,
    }
}

fn rgba8_offset(size: TextureSize, x: u32, y: u32) -> usize {
    ((y * size.width + x) * 4) as usize
}

fn generate_environment_rgba8_mips(
    size: TextureSize,
    rgba8: &[u8],
) -> GraphicsResult<Vec<Rgba8Mip>> {
    if size.width == 0 || size.height == 0 {
        return Err(GraphicsError::InvalidResource(
            "texture dimensions must be non-zero".to_owned(),
        ));
    }
    if size.byte_len() != Some(rgba8.len()) {
        return Err(GraphicsError::InvalidResource(
            "texture data length does not match dimensions".to_owned(),
        ));
    }

    let mip_count = mip_level_count(size);
    let mut mips = Vec::with_capacity(mip_count as usize);
    mips.push(Rgba8Mip {
        size,
        rgba8: rgba8.to_vec(),
    });

    for mip_level in 1..mip_count {
        let roughness = mip_level as f32 / (mip_count - 1).max(1) as f32;
        let mip_size = mip_size(size, mip_level);
        mips.push(prefilter_environment_mip(size, rgba8, mip_size, roughness));
    }

    Ok(mips)
}

fn generate_environment_cube_rgba8_mips(
    source_size: TextureSize,
    source_rgba8: &[u8],
) -> GraphicsResult<Vec<CubeRgba8Mip>> {
    if source_size.width == 0 || source_size.height == 0 {
        return Err(GraphicsError::InvalidResource(
            "environment texture dimensions must be non-zero".to_owned(),
        ));
    }
    if source_size.byte_len() != Some(source_rgba8.len()) {
        return Err(GraphicsError::InvalidResource(
            "environment texture data length does not match dimensions".to_owned(),
        ));
    }

    let face_size = environment_cube_face_size(source_size);
    let mip_count = mip_level_count(TextureSize::new(face_size, face_size));
    let mut mips = Vec::with_capacity(mip_count as usize);

    for mip_level in 0..mip_count {
        let size = (face_size >> mip_level).max(1);
        let roughness = mip_level as f32 / (mip_count - 1).max(1) as f32;
        let mut faces = Vec::with_capacity(CUBE_FACE_COUNT);

        for face in 0..CUBE_FACE_COUNT {
            faces.push(prefilter_environment_cube_face_mip(
                source_size,
                source_rgba8,
                size,
                face,
                roughness,
            ));
        }

        mips.push(CubeRgba8Mip { size, faces });
    }

    Ok(mips)
}

fn environment_cube_face_size(size: TextureSize) -> u32 {
    if size.width >= size.height.saturating_mul(2) {
        size.height.max(1)
    } else {
        size.width.min(size.height).max(1)
    }
}

fn prefilter_environment_cube_face_mip(
    source_size: TextureSize,
    source_rgba8: &[u8],
    mip_size: u32,
    face: usize,
    roughness: f32,
) -> Vec<u8> {
    let mut rgba8 = vec![0; (mip_size * mip_size * 4) as usize];

    for y in 0..mip_size {
        for x in 0..mip_size {
            let direction = direction_from_cube_texel(face, mip_size, x, y);
            let color =
                prefilter_environment_direction(source_size, source_rgba8, direction, roughness);
            let offset = ((y * mip_size + x) * 4) as usize;
            rgba8[offset] = linear_to_srgb8(color[0]);
            rgba8[offset + 1] = linear_to_srgb8(color[1]);
            rgba8[offset + 2] = linear_to_srgb8(color[2]);
            rgba8[offset + 3] = 255;
        }
    }

    rgba8
}

fn direction_from_cube_texel(face: usize, size: u32, x: u32, y: u32) -> [f32; 3] {
    let u = ((x as f32 + 0.5) / size.max(1) as f32) * 2.0 - 1.0;
    let v = ((y as f32 + 0.5) / size.max(1) as f32) * 2.0 - 1.0;
    let (forward, up) = cube_face_direction_and_up(face);
    let right = normalize3(cross3(forward, up));

    normalize3([
        forward[0] + right[0] * u - up[0] * v,
        forward[1] + right[1] * u - up[1] * v,
        forward[2] + right[2] * u - up[2] * v,
    ])
}

fn cube_face_direction_and_up(face: usize) -> ([f32; 3], [f32; 3]) {
    match face {
        0 => ([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        1 => ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        2 => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        3 => ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
        4 => ([0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),
        _ => ([0.0, 0.0, -1.0], [0.0, -1.0, 0.0]),
    }
}

fn mip_level_count(size: TextureSize) -> u32 {
    let mut width = size.width.max(1);
    let mut height = size.height.max(1);
    let mut count = 1;

    while width > 1 || height > 1 {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
        count += 1;
    }

    count
}

fn mip_size(base: TextureSize, mip_level: u32) -> TextureSize {
    TextureSize::new(
        (base.width >> mip_level).max(1),
        (base.height >> mip_level).max(1),
    )
}

fn prefilter_environment_mip(
    source_size: TextureSize,
    source_rgba8: &[u8],
    mip_size: TextureSize,
    roughness: f32,
) -> Rgba8Mip {
    let mut rgba8 = vec![0; mip_size.byte_len().unwrap_or(0)];

    for y in 0..mip_size.height {
        for x in 0..mip_size.width {
            let normal = direction_from_equirectangular_texel(mip_size, x, y);
            let color =
                prefilter_environment_direction(source_size, source_rgba8, normal, roughness);
            let offset = rgba8_offset(mip_size, x, y);
            rgba8[offset] = linear_to_srgb8(color[0]);
            rgba8[offset + 1] = linear_to_srgb8(color[1]);
            rgba8[offset + 2] = linear_to_srgb8(color[2]);
            rgba8[offset + 3] = 255;
        }
    }

    Rgba8Mip {
        size: mip_size,
        rgba8,
    }
}

fn prefilter_environment_direction(
    source_size: TextureSize,
    source_rgba8: &[u8],
    normal: [f32; 3],
    roughness: f32,
) -> [f32; 3] {
    let view = normal;
    let mut color = [0.0; 3];
    let mut total_weight = 0.0;

    for sample_index in 0..ENVIRONMENT_PREFILTER_SAMPLE_COUNT {
        let xi = hammersley(sample_index, ENVIRONMENT_PREFILTER_SAMPLE_COUNT);
        let half_dir = importance_sample_ggx_world(xi, roughness, normal);
        let view_dot_half = dot3(view, half_dir).max(0.0);
        let light = normalize3([
            2.0 * view_dot_half * half_dir[0] - view[0],
            2.0 * view_dot_half * half_dir[1] - view[1],
            2.0 * view_dot_half * half_dir[2] - view[2],
        ]);
        let n_dot_light = dot3(normal, light).max(0.0);

        if n_dot_light > 0.0 {
            let sample = sample_equirectangular_linear(source_size, source_rgba8, light);
            color[0] += sample[0] * n_dot_light;
            color[1] += sample[1] * n_dot_light;
            color[2] += sample[2] * n_dot_light;
            total_weight += n_dot_light;
        }
    }

    if total_weight > 0.0 {
        [
            color[0] / total_weight,
            color[1] / total_weight,
            color[2] / total_weight,
        ]
    } else {
        sample_equirectangular_linear(source_size, source_rgba8, normal)
    }
}

fn direction_from_equirectangular_texel(size: TextureSize, x: u32, y: u32) -> [f32; 3] {
    let u = (x as f32 + 0.5) / size.width.max(1) as f32;
    let v = (y as f32 + 0.5) / size.height.max(1) as f32;
    let phi = (u - 0.5) * 2.0 * std::f32::consts::PI;
    let theta = v * std::f32::consts::PI;
    let sin_theta = theta.sin();

    normalize3([phi.cos() * sin_theta, theta.cos(), phi.sin() * sin_theta])
}

fn sample_equirectangular_linear(size: TextureSize, rgba8: &[u8], direction: [f32; 3]) -> [f32; 3] {
    let direction = normalize3(direction);
    let u = direction[2].atan2(direction[0]) / (2.0 * std::f32::consts::PI) + 0.5;
    let v = direction[1].clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
    let x = u * size.width as f32 - 0.5;
    let y = v * size.height as f32 - 0.5;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let x0_wrapped = wrap_index(x0, size.width);
    let x1_wrapped = wrap_index(x0 + 1, size.width);
    let y0_clamped = clamp_index(y0, size.height);
    let y1_clamped = clamp_index(y0 + 1, size.height);
    let top = lerp3(
        texel_linear(size, rgba8, x0_wrapped, y0_clamped),
        texel_linear(size, rgba8, x1_wrapped, y0_clamped),
        tx,
    );
    let bottom = lerp3(
        texel_linear(size, rgba8, x0_wrapped, y1_clamped),
        texel_linear(size, rgba8, x1_wrapped, y1_clamped),
        tx,
    );

    lerp3(top, bottom, ty)
}

fn wrap_index(index: i32, len: u32) -> u32 {
    let len = len.max(1) as i32;
    index.rem_euclid(len) as u32
}

fn clamp_index(index: i32, len: u32) -> u32 {
    index.clamp(0, len.saturating_sub(1) as i32) as u32
}

fn texel_linear(size: TextureSize, rgba8: &[u8], x: u32, y: u32) -> [f32; 3] {
    let offset = rgba8_offset(size, x, y);
    [
        srgb8_to_linear(rgba8[offset]),
        srgb8_to_linear(rgba8[offset + 1]),
        srgb8_to_linear(rgba8[offset + 2]),
    ]
}

fn srgb8_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    float_to_unorm8(srgb)
}

fn generate_brdf_lut_rgba8(size: u32) -> Vec<u8> {
    let mut rgba8 = Vec::with_capacity((size * size * 4) as usize);
    let denominator = (size.saturating_sub(1)).max(1) as f32;

    for y in 0..size {
        let roughness = y as f32 / denominator;
        for x in 0..size {
            let n_dot_v = (x as f32 / denominator).max(0.001);
            let [scale, bias] =
                integrate_environment_brdf(n_dot_v, roughness, BRDF_LUT_SAMPLE_COUNT);
            rgba8.push(float_to_unorm8(scale));
            rgba8.push(float_to_unorm8(bias));
            rgba8.push(0);
            rgba8.push(255);
        }
    }

    rgba8
}

fn integrate_environment_brdf(n_dot_v: f32, roughness: f32, sample_count: u32) -> [f32; 2] {
    let view = normalize3([(1.0 - n_dot_v * n_dot_v).max(0.0).sqrt(), 0.0, n_dot_v]);
    let mut scale = 0.0;
    let mut bias = 0.0;
    let samples = sample_count.max(1);

    for sample_index in 0..samples {
        let xi = hammersley(sample_index, samples);
        let half_dir = importance_sample_ggx(xi, roughness);
        let view_dot_half = dot3(view, half_dir).max(0.0);
        let light = normalize3([
            2.0 * view_dot_half * half_dir[0] - view[0],
            2.0 * view_dot_half * half_dir[1] - view[1],
            2.0 * view_dot_half * half_dir[2] - view[2],
        ]);
        let n_dot_light = light[2].max(0.0);
        let n_dot_half = half_dir[2].max(0.0);

        if n_dot_light > 0.0 && n_dot_half > 0.0 {
            let geometry = geometry_smith_ibl(n_dot_v, n_dot_light, roughness);
            let visibility = (geometry * view_dot_half) / (n_dot_half * n_dot_v).max(0.0001);
            let fresnel = (1.0 - view_dot_half).clamp(0.0, 1.0).powi(5);
            scale += (1.0 - fresnel) * visibility;
            bias += fresnel * visibility;
        }
    }

    [scale / samples as f32, bias / samples as f32]
}

fn hammersley(index: u32, sample_count: u32) -> [f32; 2] {
    [
        index as f32 / sample_count.max(1) as f32,
        radical_inverse_vdc(index),
    ]
}

fn radical_inverse_vdc(mut bits: u32) -> f32 {
    bits = bits.rotate_right(16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    bits as f32 * 2.328_306_4e-10
}

fn importance_sample_ggx(xi: [f32; 2], roughness: f32) -> [f32; 3] {
    let roughness = roughness.max(0.001);
    let alpha = roughness * roughness;
    let phi = 2.0 * std::f32::consts::PI * xi[0];
    let cos_theta = ((1.0 - xi[1]) / (1.0 + (alpha * alpha - 1.0) * xi[1]))
        .max(0.0)
        .sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();

    [phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta]
}

fn importance_sample_ggx_world(xi: [f32; 2], roughness: f32, normal: [f32; 3]) -> [f32; 3] {
    let half_dir = importance_sample_ggx(xi, roughness);
    let normal = normalize3(normal);
    let up = if normal[2].abs() < 0.999 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let tangent = normalize3(cross3(up, normal));
    let bitangent = cross3(normal, tangent);

    normalize3([
        tangent[0] * half_dir[0] + bitangent[0] * half_dir[1] + normal[0] * half_dir[2],
        tangent[1] * half_dir[0] + bitangent[1] * half_dir[1] + normal[1] * half_dir[2],
        tangent[2] * half_dir[0] + bitangent[2] * half_dir[1] + normal[2] * half_dir[2],
    ])
}

fn geometry_smith_ibl(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    geometry_schlick_ggx_ibl(n_dot_v, roughness) * geometry_schlick_ggx_ibl(n_dot_l, roughness)
}

fn geometry_schlick_ggx_ibl(n_dot: f32, roughness: f32) -> f32 {
    let alpha = roughness.max(0.001);
    let k = alpha * alpha * 0.5;
    n_dot / (n_dot * (1.0 - k) + k).max(0.0001)
}

fn normalize3(vector: [f32; 3]) -> [f32; 3] {
    let length_squared = vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2];
    if length_squared <= f32::EPSILON {
        return [0.0, 0.0, 1.0];
    }

    let inverse_length = 1.0 / length_squared.sqrt();
    [
        vector[0] * inverse_length,
        vector[1] * inverse_length,
        vector[2] * inverse_length,
    ]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn lerp3(a: [f32; 3], b: [f32; 3], factor: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * factor,
        a[1] + (b[1] - a[1]) * factor,
        a[2] + (b[2] - a[2]) * factor,
    ]
}

fn float_to_unorm8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_mips_average_all_pixels() {
        let texture = TextureSize::new(2, 2);
        let rgba8 = [
            0, 0, 0, 255, 20, 40, 60, 255, 40, 80, 120, 255, 60, 120, 180, 255,
        ];

        let mips = generate_rgba8_mips(texture, &rgba8).unwrap();

        assert_eq!(mips.len(), 2);
        assert_eq!(mips[1].size, TextureSize::new(1, 1));
        assert_eq!(mips[1].rgba8, vec![30, 60, 90, 255]);
    }

    #[test]
    fn generated_mips_cover_odd_dimensions() {
        let texture = TextureSize::new(3, 1);
        let rgba8 = [10, 20, 30, 255, 30, 60, 90, 255, 50, 100, 150, 255];

        let mips = generate_rgba8_mips(texture, &rgba8).unwrap();

        assert_eq!(mips.len(), 2);
        assert_eq!(mips[1].size, TextureSize::new(1, 1));
        assert_eq!(mips[1].rgba8, vec![30, 60, 90, 255]);
    }

    #[test]
    fn material_texture_options_enable_mipmapped_linear_sampling() {
        let options = TextureUploadOptions::material(wgpu::TextureFormat::Rgba8UnormSrgb);

        assert!(options.generate_mips);
        assert!(!options.prefilter_environment);
        assert_eq!(options.address_mode_u, wgpu::AddressMode::Repeat);
        assert_eq!(options.address_mode_v, wgpu::AddressMode::Repeat);
        assert_eq!(options.mag_filter, wgpu::FilterMode::Linear);
        assert_eq!(options.min_filter, wgpu::FilterMode::Linear);
        assert_eq!(options.mipmap_filter, wgpu::FilterMode::Linear);
    }

    #[test]
    fn environment_prefilter_mips_reach_single_texel() {
        let size = TextureSize::new(4, 2);
        let texture = Texture::checkerboard_rgba8(size, 1, [255, 0, 0, 255], [0, 0, 255, 255]);

        let mips = generate_environment_cube_rgba8_mips(size, texture.rgba8_data()).unwrap();

        assert_eq!(mips.len(), 2);
        assert_eq!(mips[0].size, 2);
        assert_eq!(mips[1].size, 1);
        assert!(mips.iter().all(|mip| mip
            .faces
            .iter()
            .all(|face| face.chunks_exact(4).all(|texel| texel[3] == 255))));
    }

    #[test]
    fn environment_prefilter_preserves_solid_white() {
        let size = TextureSize::new(4, 2);
        let texture = Texture::solid_rgba(size, [255, 255, 255, 255]);

        let mips = generate_environment_cube_rgba8_mips(size, texture.rgba8_data()).unwrap();

        for mip in mips {
            assert!(mip.faces.iter().all(|face| face
                .chunks_exact(4)
                .all(|texel| texel == [255, 255, 255, 255])));
        }
    }

    #[test]
    fn brdf_lut_generation_outputs_rgba8_texels() {
        let lut = generate_brdf_lut_rgba8(4);

        assert_eq!(lut.len(), 4 * 4 * 4);
        assert!(lut.chunks_exact(4).all(|texel| texel[2] == 0));
        assert!(lut.chunks_exact(4).all(|texel| texel[3] == 255));
    }

    #[test]
    fn brdf_integration_keeps_values_in_texture_range() {
        for roughness in [0.0, 0.25, 0.5, 1.0] {
            for n_dot_v in [0.001, 0.25, 0.5, 1.0] {
                let [scale, bias] = integrate_environment_brdf(n_dot_v, roughness, 32);
                assert!((0.0..=1.0).contains(&scale));
                assert!((0.0..=1.0).contains(&bias));
            }
        }
    }
}
