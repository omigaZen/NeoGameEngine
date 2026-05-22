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
