use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    pub family_name: String,
    pub data: FontData,
}

impl Asset for Font {
    const TYPE_NAME: &'static str = "Font";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_000a);
}

impl AssetMemoryUsage for Font {
    fn cpu_bytes(&self) -> u64 {
        match &self.data {
            FontData::TrueType(bytes) | FontData::OpenType(bytes) => bytes.len() as u64,
            FontData::Bitmap(font) => font
                .glyphs
                .iter()
                .map(|glyph| glyph.bitmap.len() as u64)
                .sum(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontData {
    TrueType(Vec<u8>),
    OpenType(Vec<u8>),
    Bitmap(BitmapFont),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapFont {
    pub glyphs: Vec<BitmapGlyph>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapGlyph {
    pub codepoint: char,
    pub width: u32,
    pub height: u32,
    pub bitmap: Vec<u8>,
}

pub struct FontLoader;

impl FontLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FontLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for FontLoader {
    fn name(&self) -> &'static str {
        "FontLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["font"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Font::TYPE_ID
    }

    fn load(
        &self,
        _ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_font(bytes).map(LoadedAsset::new)
    }
}

fn parse_font(bytes: &[u8]) -> Result<Font, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("font source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_FONT_V1" {
        return Err(AssetError::Decode {
            message: "font source must start with NGA_FONT_V1".to_owned(),
        });
    }

    let mut family_name = None;
    let mut glyphs = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid font line {line_number}"),
            });
        };
        match key.trim() {
            "family" => {
                if value.trim().is_empty() {
                    return Err(AssetError::Decode {
                        message: format!("font family is empty on line {line_number}"),
                    });
                }
                family_name = Some(value.trim().to_owned());
            }
            "glyph" => glyphs.push(parse_bitmap_glyph(value.trim(), line_number)?),
            other => {
                return Err(AssetError::Decode {
                    message: format!("unknown font key `{other}` on line {line_number}"),
                })
            }
        }
    }

    let family_name = family_name.ok_or_else(|| AssetError::Decode {
        message: "font source missing family".to_owned(),
    })?;
    if glyphs.is_empty() {
        return Err(AssetError::Decode {
            message: "font source must contain at least one glyph".to_owned(),
        });
    }
    Ok(Font {
        family_name,
        data: FontData::Bitmap(BitmapFont { glyphs }),
    })
}

fn parse_bitmap_glyph(value: &str, line_number: usize) -> Result<BitmapGlyph, AssetError> {
    let mut codepoint = None;
    let mut size = None;
    let mut bitmap = None;
    for part in value.split(';').map(str::trim) {
        let Some((key, value)) = part.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid font glyph field on line {line_number}"),
            });
        };
        match (key.trim(), value.trim()) {
            ("char", value) => {
                let mut chars = value.chars();
                let Some(character) = chars.next() else {
                    return Err(AssetError::Decode {
                        message: format!("font glyph char is empty on line {line_number}"),
                    });
                };
                if chars.next().is_some() {
                    return Err(AssetError::Decode {
                        message: format!(
                            "font glyph char must be one scalar on line {line_number}"
                        ),
                    });
                }
                codepoint = Some(character);
            }
            ("size", value) => size = Some(parse_glyph_size(value, line_number)?),
            ("bitmap", value) => bitmap = Some(parse_u8_list(value, line_number)?),
            (other, _) => {
                return Err(AssetError::Decode {
                    message: format!("unknown font glyph field `{other}` on line {line_number}"),
                })
            }
        }
    }
    let codepoint = codepoint.ok_or_else(|| AssetError::Decode {
        message: format!("font glyph missing char on line {line_number}"),
    })?;
    let (width, height) = size.ok_or_else(|| AssetError::Decode {
        message: format!("font glyph missing size on line {line_number}"),
    })?;
    let bitmap = bitmap.ok_or_else(|| AssetError::Decode {
        message: format!("font glyph missing bitmap on line {line_number}"),
    })?;
    let expected = width as usize * height as usize;
    if bitmap.len() != expected {
        return Err(AssetError::Decode {
            message: format!(
                "font glyph bitmap on line {line_number} has {} bytes, expected {expected}",
                bitmap.len()
            ),
        });
    }
    Ok(BitmapGlyph {
        codepoint,
        width,
        height,
        bitmap,
    })
}

fn parse_glyph_size(value: &str, line_number: usize) -> Result<(u32, u32), AssetError> {
    let Some((width, height)) = value.split_once('x') else {
        return Err(AssetError::Decode {
            message: format!("invalid font glyph size on line {line_number}"),
        });
    };
    let width = width.trim().parse().map_err(|error| AssetError::Decode {
        message: format!("invalid font glyph width on line {line_number}: {error}"),
    })?;
    let height = height.trim().parse().map_err(|error| AssetError::Decode {
        message: format!("invalid font glyph height on line {line_number}: {error}"),
    })?;
    if width == 0 || height == 0 {
        return Err(AssetError::Decode {
            message: format!("font glyph size must be non-zero on line {line_number}"),
        });
    }
    Ok((width, height))
}

fn parse_u8_list(value: &str, line_number: usize) -> Result<Vec<u8>, AssetError> {
    if value.trim().is_empty() {
        return Err(AssetError::Decode {
            message: format!("font glyph bitmap is empty on line {line_number}"),
        });
    }
    value
        .split(',')
        .map(str::trim)
        .map(|part| {
            part.parse().map_err(|error| AssetError::Decode {
                message: format!("invalid font glyph bitmap value on line {line_number}: {error}"),
            })
        })
        .collect()
}
