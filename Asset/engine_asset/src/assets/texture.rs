use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    gpu_upload::GpuResourceHandle,
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba16Float,
    Rgba32Float,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_count: u32,
    pub data: Vec<u8>,
    pub gpu: Option<GpuResourceHandle>,
}

impl Asset for Texture {
    const TYPE_NAME: &'static str = "Texture";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0001);
}

impl AssetMemoryUsage for Texture {
    fn cpu_bytes(&self) -> u64 {
        self.data.len() as u64
    }

    fn gpu_bytes(&self) -> u64 {
        pixel_size(self.format) * u64::from(self.width) * u64::from(self.height)
    }
}

fn pixel_size(format: TextureFormat) -> u64 {
    match format {
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => 4,
        TextureFormat::Rgba16Float => 8,
        TextureFormat::Rgba32Float => 16,
    }
}

pub struct TextureLoader;

impl TextureLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for TextureLoader {
    fn name(&self) -> &'static str {
        "TextureLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["texture", "tex", "rgba"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Texture::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        let texture = decode_texture(bytes)?;
        Ok(LoadedAsset::new(texture).texture_upload(
            ctx.id(),
            Texture::TYPE_ID,
            Some(ctx.path().display_string()),
            bytes.to_vec(),
        ))
    }
}

fn decode_texture(bytes: &[u8]) -> Result<Texture, AssetError> {
    if bytes.len() < 8 {
        return Err(AssetError::Decode {
            message: "texture bytes must start with little-endian width and height".to_owned(),
        });
    }
    let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if width == 0 || height == 0 {
        return Err(AssetError::Decode {
            message: "texture width and height must be non-zero".to_owned(),
        });
    }
    let data = bytes[8..].to_vec();
    let expected = width as usize * height as usize * 4;
    if data.len() != expected {
        return Err(AssetError::Decode {
            message: format!(
                "texture data length {} did not match expected {expected}",
                data.len()
            ),
        });
    }
    Ok(Texture {
        width,
        height,
        format: TextureFormat::Rgba8UnormSrgb,
        mip_count: 1,
        data,
        gpu: None,
    })
}

pub fn canonical_texture_runtime_bytes(bytes: &[u8]) -> Result<Vec<u8>, AssetError> {
    let texture = decode_texture(bytes)?;
    encode_texture_runtime_bytes(&texture)
}

pub fn encode_texture_runtime_bytes(texture: &Texture) -> Result<Vec<u8>, AssetError> {
    let mut bytes = Vec::with_capacity(8 + texture.data.len());
    bytes.extend_from_slice(&texture.width.to_le_bytes());
    bytes.extend_from_slice(&texture.height.to_le_bytes());
    bytes.extend_from_slice(&texture.data);
    Ok(bytes)
}

pub fn canonical_texture_source_document(source_text: &str) -> Result<Vec<u8>, AssetError> {
    let mut lines = source_text.lines();
    if lines.next().unwrap_or("").trim() != "NGA_TEXTURE_SOURCE_V1" {
        return Err(AssetError::Import {
            message: "texture source must start with NGA_TEXTURE_SOURCE_V1".to_owned(),
        });
    }
    let mut width = None;
    let mut height = None;
    let mut pixels = None;
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Import {
                message: format!("invalid texture source line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "width" => width = Some(parse_texture_dimension(value, "width", line_number)?),
            "height" => height = Some(parse_texture_dimension(value, "height", line_number)?),
            "size" => {
                let Some((width_value, height_value)) = value.split_once('x') else {
                    return Err(AssetError::Import {
                        message: format!("texture source size on line {line_number} must be WxH"),
                    });
                };
                width = Some(parse_texture_dimension(
                    width_value.trim(),
                    "width",
                    line_number,
                )?);
                height = Some(parse_texture_dimension(
                    height_value.trim(),
                    "height",
                    line_number,
                )?);
            }
            "rgba" | "pixels" => pixels = Some(parse_texture_pixels(value, line_number)?),
            other => {
                return Err(AssetError::Import {
                    message: format!("unknown texture source key `{other}` on line {line_number}"),
                })
            }
        }
    }
    let width = width.ok_or_else(|| AssetError::Import {
        message: "texture source missing width or size".to_owned(),
    })?;
    let height = height.ok_or_else(|| AssetError::Import {
        message: "texture source missing height or size".to_owned(),
    })?;
    let pixels = pixels.ok_or_else(|| AssetError::Import {
        message: "texture source missing rgba pixels".to_owned(),
    })?;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| AssetError::Import {
            message: "texture source dimensions overflow pixel count".to_owned(),
        })?;
    if pixels.len() != expected {
        return Err(AssetError::Import {
            message: format!(
                "texture source rgba byte count {} did not match expected {expected}",
                pixels.len()
            ),
        });
    }
    let mut bytes = Vec::with_capacity(8 + pixels.len());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&pixels);
    Ok(bytes)
}

fn parse_texture_dimension(value: &str, name: &str, line_number: usize) -> Result<u32, AssetError> {
    let dimension = value.parse::<u32>().map_err(|error| AssetError::Import {
        message: format!("invalid texture source {name} on line {line_number}: {error}"),
    })?;
    if dimension == 0 {
        return Err(AssetError::Import {
            message: format!("texture source {name} on line {line_number} must be non-zero"),
        });
    }
    Ok(dimension)
}

fn parse_texture_pixels(value: &str, line_number: usize) -> Result<Vec<u8>, AssetError> {
    value
        .replace(';', ",")
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<u8>().map_err(|error| AssetError::Import {
                message: format!(
                    "invalid texture source rgba byte `{part}` on line {line_number}: {error}"
                ),
            })
        })
        .collect()
}
