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
            FontData::Bitmap(font) => {
                font.atlas_bitmap.len() as u64
                    + font
                        .glyphs
                        .iter()
                        .map(|glyph| glyph.bitmap.len() as u64)
                        .sum::<u64>()
            }
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
    pub kerning_pairs: Vec<BitmapKerningPair>,
    pub line_height: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub atlas_bitmap: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapKerningPair {
    pub left: char,
    pub right: char,
    pub adjustment: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapTextLayout {
    pub glyphs: Vec<PositionedBitmapGlyph>,
    pub advance_width: i32,
    pub height: u32,
    pub line_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionedBitmapGlyph {
    pub codepoint: char,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub atlas_x: u32,
    pub atlas_y: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitmapGlyph {
    pub codepoint: char,
    pub width: u32,
    pub height: u32,
    pub advance_x: i32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub bitmap: Vec<u8>,
}

impl BitmapFont {
    pub fn glyph(&self, codepoint: char) -> Option<&BitmapGlyph> {
        self.glyphs
            .iter()
            .find(|glyph| glyph.codepoint == codepoint)
    }

    pub fn kerning_adjustment(&self, left: char, right: char) -> i32 {
        self.kerning_pairs
            .iter()
            .find(|pair| pair.left == left && pair.right == right)
            .map_or(0, |pair| pair.adjustment)
    }

    pub fn layout_text(&self, text: &str) -> Result<BitmapTextLayout, AssetError> {
        if text.is_empty() {
            return Ok(BitmapTextLayout {
                glyphs: Vec::new(),
                advance_width: 0,
                height: 0,
                line_count: 0,
            });
        }

        let mut positioned = Vec::new();
        let mut pen_x = 0i32;
        let mut line_y = 0i32;
        let mut max_advance_width = 0i32;
        let mut line_count = 1u32;
        let mut previous = None;
        let line_advance = i32::try_from(self.line_height).map_err(|_| AssetError::Decode {
            message: "bitmap font text layout line height exceeds i32".to_owned(),
        })?;

        for codepoint in text.chars() {
            if codepoint == '\r' {
                continue;
            }
            if codepoint == '\n' {
                max_advance_width = max_advance_width.max(pen_x);
                pen_x = 0;
                line_y = line_y
                    .checked_add(line_advance)
                    .ok_or_else(|| AssetError::Decode {
                        message: "bitmap font text layout line position overflows i32".to_owned(),
                    })?;
                line_count = line_count
                    .checked_add(1)
                    .ok_or_else(|| AssetError::Decode {
                        message: "bitmap font text layout line count overflows u32".to_owned(),
                    })?;
                previous = None;
                continue;
            }

            let glyph = self.glyph(codepoint).ok_or_else(|| AssetError::Decode {
                message: format!("bitmap font text layout is missing glyph `{codepoint}`"),
            })?;
            if let Some(left) = previous {
                pen_x = pen_x
                    .checked_add(self.kerning_adjustment(left, codepoint))
                    .ok_or_else(|| AssetError::Decode {
                        message: "bitmap font text layout horizontal position overflows i32"
                            .to_owned(),
                    })?;
            }
            let x = pen_x
                .checked_add(glyph.bearing_x)
                .ok_or_else(|| AssetError::Decode {
                    message: "bitmap font text layout glyph x position overflows i32".to_owned(),
                })?;
            let y = line_y
                .checked_add(glyph.bearing_y)
                .ok_or_else(|| AssetError::Decode {
                    message: "bitmap font text layout glyph y position overflows i32".to_owned(),
                })?;
            positioned.push(PositionedBitmapGlyph {
                codepoint,
                x,
                y,
                width: glyph.width,
                height: glyph.height,
                atlas_x: glyph.atlas_x,
                atlas_y: glyph.atlas_y,
            });
            pen_x = pen_x
                .checked_add(glyph.advance_x)
                .ok_or_else(|| AssetError::Decode {
                    message: "bitmap font text layout horizontal advance overflows i32".to_owned(),
                })?;
            previous = Some(codepoint);
        }

        max_advance_width = max_advance_width.max(pen_x);
        let height =
            line_count
                .checked_mul(self.line_height)
                .ok_or_else(|| AssetError::Decode {
                    message: "bitmap font text layout height overflows u32".to_owned(),
                })?;
        Ok(BitmapTextLayout {
            glyphs: positioned,
            advance_width: max_advance_width,
            height,
            line_count,
        })
    }
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
        &["font", "ttf", "otf"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Font::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_font_from_path(ctx.path(), bytes).map(LoadedAsset::new)
    }
}

pub(crate) fn parse_font_from_path(
    path: &crate::path::AssetPath,
    bytes: &[u8],
) -> Result<Font, AssetError> {
    match path.extension().map(str::to_ascii_lowercase).as_deref() {
        Some("ttf") => parse_binary_font(path, bytes, BinaryFontKind::TrueType),
        Some("otf") => parse_binary_font(path, bytes, BinaryFontKind::OpenType),
        _ => parse_bitmap_font(bytes),
    }
}

fn parse_bitmap_font(bytes: &[u8]) -> Result<Font, AssetError> {
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
    let mut line_height = None;
    let mut glyphs = Vec::new();
    let mut kerning_pairs = Vec::new();
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
        let key = key.trim();
        match font_document_key(key).as_str() {
            "family" | "familyname" | "fontfamily" | "name" => {
                if family_name.is_some() {
                    return Err(AssetError::Decode {
                        message: format!("font source repeats family on line {line_number}"),
                    });
                }
                if value.trim().is_empty() {
                    return Err(AssetError::Decode {
                        message: format!("font family is empty on line {line_number}"),
                    });
                }
                family_name = Some(value.trim().to_owned());
            }
            "lineheight" | "height" => {
                if line_height.is_some() {
                    return Err(AssetError::Decode {
                        message: format!("font source repeats line height on line {line_number}"),
                    });
                }
                line_height = Some(parse_positive_u32(
                    value.trim(),
                    line_number,
                    "font line height",
                )?);
            }
            "glyph" | "character" | "char" => {
                glyphs.push(parse_bitmap_glyph(value.trim(), line_number)?)
            }
            "kerning" | "kern" | "kerningpair" => {
                kerning_pairs.push(parse_bitmap_kerning(value.trim(), line_number)?)
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown font key `{key}` on line {line_number}"),
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
    for (glyph_index, glyph) in glyphs.iter().enumerate() {
        if glyphs[..glyph_index]
            .iter()
            .any(|previous| previous.codepoint == glyph.codepoint)
        {
            return Err(AssetError::Decode {
                message: format!("font source repeats glyph `{}`", glyph.codepoint),
            });
        }
    }
    validate_bitmap_kerning_pairs(&glyphs, &kerning_pairs)?;
    let bitmap_font = build_bitmap_font(glyphs, kerning_pairs, line_height)?;
    Ok(Font {
        family_name,
        data: FontData::Bitmap(bitmap_font),
    })
}

fn build_bitmap_font(
    mut glyphs: Vec<BitmapGlyph>,
    kerning_pairs: Vec<BitmapKerningPair>,
    line_height: Option<u32>,
) -> Result<BitmapFont, AssetError> {
    let atlas_width = glyphs.iter().try_fold(0u32, |width, glyph| {
        width
            .checked_add(glyph.width)
            .ok_or_else(|| AssetError::Decode {
                message: "font atlas width overflows u32".to_owned(),
            })
    })?;
    let atlas_height = glyphs.iter().map(|glyph| glyph.height).max().unwrap_or(0);
    let line_height = line_height.unwrap_or(atlas_height);
    let atlas_len = (atlas_width as usize)
        .checked_mul(atlas_height as usize)
        .ok_or_else(|| AssetError::Decode {
            message: "font atlas dimensions overflow".to_owned(),
        })?;
    let mut atlas_bitmap = vec![0; atlas_len];
    let mut cursor_x = 0u32;
    for glyph in &mut glyphs {
        glyph.atlas_x = cursor_x;
        glyph.atlas_y = 0;
        for row in 0..glyph.height as usize {
            let src_start = row * glyph.width as usize;
            let src_end = src_start + glyph.width as usize;
            let dst_start = row * atlas_width as usize + cursor_x as usize;
            let dst_end = dst_start + glyph.width as usize;
            atlas_bitmap[dst_start..dst_end].copy_from_slice(&glyph.bitmap[src_start..src_end]);
        }
        cursor_x += glyph.width;
    }
    Ok(BitmapFont {
        glyphs,
        kerning_pairs,
        line_height,
        atlas_width,
        atlas_height,
        atlas_bitmap,
    })
}

fn validate_bitmap_kerning_pairs(
    glyphs: &[BitmapGlyph],
    kerning_pairs: &[BitmapKerningPair],
) -> Result<(), AssetError> {
    for (pair_index, pair) in kerning_pairs.iter().enumerate() {
        if !glyphs.iter().any(|glyph| glyph.codepoint == pair.left) {
            return Err(AssetError::Decode {
                message: format!(
                    "font kerning pair {} references missing left glyph `{}`",
                    pair_index, pair.left
                ),
            });
        }
        if !glyphs.iter().any(|glyph| glyph.codepoint == pair.right) {
            return Err(AssetError::Decode {
                message: format!(
                    "font kerning pair {} references missing right glyph `{}`",
                    pair_index, pair.right
                ),
            });
        }
        if kerning_pairs[..pair_index]
            .iter()
            .any(|previous| previous.left == pair.left && previous.right == pair.right)
        {
            return Err(AssetError::Decode {
                message: format!(
                    "font source repeats kerning pair `{}{}`",
                    pair.left, pair.right
                ),
            });
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BinaryFontKind {
    TrueType,
    OpenType,
}

fn parse_binary_font(
    path: &crate::path::AssetPath,
    bytes: &[u8],
    kind: BinaryFontKind,
) -> Result<Font, AssetError> {
    if bytes.len() < 12 {
        return Err(AssetError::Decode {
            message: format!(
                "{} font source is truncated; expected at least 12 bytes",
                binary_font_kind_name(kind)
            ),
        });
    }
    let signature = bytes.get(0..4).unwrap_or_default();
    let valid = match kind {
        BinaryFontKind::TrueType => {
            matches!(signature, b"\x00\x01\x00\x00" | b"true" | b"typ1")
        }
        BinaryFontKind::OpenType => signature == b"OTTO",
    };
    if !valid {
        return Err(AssetError::Decode {
            message: format!(
                "{} font source has unsupported signature {}",
                binary_font_kind_name(kind),
                format_font_signature(signature)
            ),
        });
    }
    let family_name = font_family_from_path(path, kind);
    let data = match kind {
        BinaryFontKind::TrueType => FontData::TrueType(bytes.to_vec()),
        BinaryFontKind::OpenType => FontData::OpenType(bytes.to_vec()),
    };
    Ok(Font { family_name, data })
}

fn binary_font_kind_name(kind: BinaryFontKind) -> &'static str {
    match kind {
        BinaryFontKind::TrueType => "TrueType",
        BinaryFontKind::OpenType => "OpenType",
    }
}

fn font_family_from_path(path: &crate::path::AssetPath, kind: BinaryFontKind) -> String {
    path.path()
        .rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.').map(|(stem, _)| stem).or(Some(name)))
        .filter(|stem| !stem.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} Font", binary_font_kind_name(kind)))
}

fn format_font_signature(signature: &[u8]) -> String {
    let bytes = signature
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("[{bytes}]")
}

pub(crate) fn font_document_key(key: &str) -> String {
    key.chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '_' && *character != '-'
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_bitmap_glyph(value: &str, line_number: usize) -> Result<BitmapGlyph, AssetError> {
    let mut codepoint = None;
    let mut size = None;
    let mut bitmap = None;
    let mut advance_x = None;
    let mut bearing_x = None;
    let mut bearing_y = None;
    for part in value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let Some((key, value)) = part.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid font glyph field on line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match font_document_key(key).as_str() {
            "char" | "character" | "codepoint" | "code" => {
                codepoint = Some(parse_font_scalar(value, line_number, "font glyph char")?);
            }
            "size" | "dimensions" | "dim" => size = Some(parse_glyph_size(value, line_number)?),
            "bitmap" | "pixels" | "alpha" => bitmap = Some(parse_u8_list(value, line_number)?),
            "advance" | "advancex" => {
                advance_x = Some(parse_i32(value, line_number, "font glyph advance")?)
            }
            "bearing" | "offset" => {
                let (x, y) = parse_i32_pair(value, line_number, "font glyph bearing")?;
                bearing_x = Some(x);
                bearing_y = Some(y);
            }
            "bearingx" | "offsetx" => {
                bearing_x = Some(parse_i32(value, line_number, "font glyph bearing x")?)
            }
            "bearingy" | "offsety" => {
                bearing_y = Some(parse_i32(value, line_number, "font glyph bearing y")?)
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown font glyph field `{key}` on line {line_number}"),
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
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| AssetError::Decode {
            message: format!("font glyph bitmap dimensions overflow on line {line_number}"),
        })?;
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
        advance_x: advance_x.unwrap_or(width as i32),
        bearing_x: bearing_x.unwrap_or(0),
        bearing_y: bearing_y.unwrap_or(0),
        atlas_x: 0,
        atlas_y: 0,
        bitmap,
    })
}

fn parse_bitmap_kerning(value: &str, line_number: usize) -> Result<BitmapKerningPair, AssetError> {
    let mut left = None;
    let mut right = None;
    let mut adjustment = None;
    for part in value
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let Some((key, value)) = part.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid font kerning field on line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match font_document_key(key).as_str() {
            "left" | "first" => {
                left = Some(parse_font_scalar(
                    value,
                    line_number,
                    "font kerning left glyph",
                )?)
            }
            "right" | "second" => {
                right = Some(parse_font_scalar(
                    value,
                    line_number,
                    "font kerning right glyph",
                )?)
            }
            "adjust" | "adjustment" | "amount" | "offset" => {
                adjustment = Some(parse_i32(value, line_number, "font kerning adjustment")?)
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown font kerning field `{key}` on line {line_number}"),
                })
            }
        }
    }
    Ok(BitmapKerningPair {
        left: left.ok_or_else(|| AssetError::Decode {
            message: format!("font kerning pair missing left glyph on line {line_number}"),
        })?,
        right: right.ok_or_else(|| AssetError::Decode {
            message: format!("font kerning pair missing right glyph on line {line_number}"),
        })?,
        adjustment: adjustment.ok_or_else(|| AssetError::Decode {
            message: format!("font kerning pair missing adjustment on line {line_number}"),
        })?,
    })
}

fn parse_font_scalar(value: &str, line_number: usize, field: &str) -> Result<char, AssetError> {
    let mut chars = value.chars();
    let Some(character) = chars.next() else {
        return Err(AssetError::Decode {
            message: format!("{field} is empty on line {line_number}"),
        });
    };
    if chars.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("{field} must be one scalar on line {line_number}"),
        });
    }
    Ok(character)
}

fn parse_positive_u32(value: &str, line_number: usize, field: &str) -> Result<u32, AssetError> {
    let parsed = value.parse::<u32>().map_err(|error| AssetError::Decode {
        message: format!("invalid {field} on line {line_number}: {error}"),
    })?;
    if parsed == 0 {
        return Err(AssetError::Decode {
            message: format!("{field} must be non-zero on line {line_number}"),
        });
    }
    Ok(parsed)
}

fn parse_i32(value: &str, line_number: usize, field: &str) -> Result<i32, AssetError> {
    value.parse::<i32>().map_err(|error| AssetError::Decode {
        message: format!("invalid {field} on line {line_number}: {error}"),
    })
}

fn parse_i32_pair(value: &str, line_number: usize, field: &str) -> Result<(i32, i32), AssetError> {
    let normalized = value.replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let Some(x) = parts.next() else {
        return Err(AssetError::Decode {
            message: format!("invalid {field} on line {line_number}"),
        });
    };
    let Some(y) = parts.next() else {
        return Err(AssetError::Decode {
            message: format!("invalid {field} on line {line_number}"),
        });
    };
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("invalid {field} on line {line_number}"),
        });
    }
    Ok((
        parse_i32(x, line_number, field)?,
        parse_i32(y, line_number, field)?,
    ))
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
    let normalized = value.replace(',', " ");
    normalized
        .split_whitespace()
        .map(str::trim)
        .map(|part| {
            part.parse().map_err(|error| AssetError::Decode {
                message: format!("invalid font glyph bitmap value on line {line_number}: {error}"),
            })
        })
        .collect()
}
