use std::{
    fmt,
    io::{Cursor, Read},
};

use flate2::read::ZlibDecoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    index: usize,
    generation: u32,
}

impl TextureHandle {
    pub(crate) const fn new(index: usize, generation: u32) -> Self {
        Self { index, generation }
    }

    pub const fn index(self) -> usize {
        self.index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSize {
    pub width: u32,
    pub height: u32,
}

impl TextureSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn byte_len(self) -> Option<usize> {
        let pixels = self.width.checked_mul(self.height)?;
        pixels.checked_mul(4).map(|bytes| bytes as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureLoadError {
    UnsupportedExtension { path: String },
    InvalidData(&'static str),
    UnsupportedBmp(&'static str),
    UnsupportedJpeg(&'static str),
    UnsupportedKtx2(&'static str),
    UnsupportedPng(&'static str),
    UnsupportedTga(&'static str),
    Jpeg(String),
    Ktx2(String),
    Png(String),
    Webp(String),
}

impl fmt::Display for TextureLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedExtension { path } => {
                write!(f, "unsupported image extension for '{path}'")
            }
            Self::InvalidData(reason) => write!(f, "invalid image data: {reason}"),
            Self::UnsupportedBmp(reason) => write!(f, "unsupported BMP: {reason}"),
            Self::UnsupportedJpeg(reason) => write!(f, "unsupported JPEG: {reason}"),
            Self::UnsupportedKtx2(reason) => write!(f, "unsupported KTX2: {reason}"),
            Self::UnsupportedPng(reason) => write!(f, "unsupported PNG: {reason}"),
            Self::UnsupportedTga(reason) => write!(f, "unsupported TGA: {reason}"),
            Self::Jpeg(reason) => write!(f, "invalid JPEG data: {reason}"),
            Self::Ktx2(reason) => write!(f, "invalid KTX2 data: {reason}"),
            Self::Png(reason) => write!(f, "invalid PNG data: {reason}"),
            Self::Webp(reason) => write!(f, "invalid WebP data: {reason}"),
        }
    }
}

impl std::error::Error for TextureLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    size: TextureSize,
    rgba8: Vec<u8>,
}

impl Texture {
    pub fn rgba8(size: TextureSize, rgba8: impl Into<Vec<u8>>) -> Option<Self> {
        let rgba8 = rgba8.into();
        if size.width == 0 || size.height == 0 || size.byte_len()? != rgba8.len() {
            return None;
        }

        Some(Self { size, rgba8 })
    }

    pub fn from_image_bytes(path: &str, bytes: &[u8]) -> Result<Self, TextureLoadError> {
        let path = path.to_ascii_lowercase();
        if path.ends_with(".bmp") {
            return Self::from_bmp_bytes(bytes);
        }
        if path.ends_with(".png") {
            return Self::from_png_bytes(bytes);
        }
        if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            return Self::from_jpeg_bytes(bytes);
        }
        if path.ends_with(".webp") {
            return Self::from_webp_bytes(bytes);
        }
        if path.ends_with(".ktx2") {
            return Self::from_ktx2_bytes(bytes);
        }
        if path.ends_with(".tga") || path.ends_with(".targa") {
            return Self::from_tga_bytes(bytes);
        }
        if bytes.starts_with(b"BM") {
            return Self::from_bmp_bytes(bytes);
        }
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Self::from_png_bytes(bytes);
        }
        if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            return Self::from_jpeg_bytes(bytes);
        }
        if is_webp_bytes(bytes) {
            return Self::from_webp_bytes(bytes);
        }
        if is_ktx2_bytes(bytes) {
            return Self::from_ktx2_bytes(bytes);
        }
        if is_tga_bytes(bytes) {
            return Self::from_tga_bytes(bytes);
        }

        Err(TextureLoadError::UnsupportedExtension { path })
    }

    pub fn from_bmp_bytes(bytes: &[u8]) -> Result<Self, TextureLoadError> {
        if bytes.len() < 54 {
            return Err(TextureLoadError::InvalidData(
                "BMP is shorter than its header",
            ));
        }
        if &bytes[0..2] != b"BM" {
            return Err(TextureLoadError::InvalidData("BMP signature must be BM"));
        }

        let pixel_offset = read_u32_le(bytes, 10)? as usize;
        let dib_header_size = read_u32_le(bytes, 14)?;
        if dib_header_size < 40 {
            return Err(TextureLoadError::UnsupportedBmp(
                "DIB header must be at least BITMAPINFOHEADER",
            ));
        }

        let width = read_i32_le(bytes, 18)?;
        let height = read_i32_le(bytes, 22)?;
        let planes = read_u16_le(bytes, 26)?;
        let bits_per_pixel = read_u16_le(bytes, 28)?;
        let compression = read_u32_le(bytes, 30)?;
        let color_count = read_u32_le(bytes, 46)?;

        if planes != 1 {
            return Err(TextureLoadError::UnsupportedBmp("plane count must be 1"));
        }
        if !matches!(bits_per_pixel, 1 | 4 | 8 | 16 | 24 | 32) {
            return Err(TextureLoadError::UnsupportedBmp(
                "only 1-bit paletted, 4-bit paletted, 8-bit paletted, 16-bit RGB555/bitfields, 24-bit, and 32-bit BMP data is supported",
            ));
        }
        let is_rle8 = compression == 1 && bits_per_pixel == 8;
        let is_rle4 = compression == 2 && bits_per_pixel == 4;
        let is_bitfields = compression == 3 && matches!(bits_per_pixel, 16 | 32);
        if compression != 0 && !is_rle8 && !is_rle4 && !is_bitfields {
            return Err(TextureLoadError::UnsupportedBmp(
                "only uncompressed BMP, RLE8 8-bit paletted BMP, RLE4 4-bit paletted BMP, and 16/32-bit bitfields BMP data is supported",
            ));
        }
        if width <= 0 || height == 0 || height == i32::MIN {
            return Err(TextureLoadError::InvalidData("BMP dimensions are invalid"));
        }

        let width = width as u32;
        let height_abs = height.unsigned_abs();
        let top_down = height < 0;
        let bytes_per_pixel = u32::from(bits_per_pixel / 8);
        let palette_entry_count = if matches!(bits_per_pixel, 1 | 4 | 8) {
            if color_count == 0 {
                1usize << bits_per_pixel
            } else {
                usize::try_from(color_count)
                    .map_err(|_| TextureLoadError::InvalidData("BMP palette is too large"))?
            }
        } else {
            0
        };
        let palette_len = palette_entry_count
            .checked_mul(4)
            .ok_or(TextureLoadError::InvalidData("BMP palette is too large"))?;
        let palette = if matches!(bits_per_pixel, 1 | 4 | 8) {
            let palette_start = 14usize
                .checked_add(
                    usize::try_from(dib_header_size)
                        .map_err(|_| TextureLoadError::InvalidData("BMP header is too large"))?,
                )
                .ok_or(TextureLoadError::InvalidData("BMP palette is too large"))?;
            let palette_end = palette_start
                .checked_add(palette_len)
                .ok_or(TextureLoadError::InvalidData("BMP palette is too large"))?;
            if palette_end > pixel_offset {
                return Err(TextureLoadError::InvalidData(
                    "BMP palette overlaps pixel data",
                ));
            }
            bytes
                .get(palette_start..palette_end)
                .ok_or(TextureLoadError::InvalidData(
                    "BMP palette extends past input",
                ))?
        } else {
            &[]
        };
        let row_bytes = if bits_per_pixel == 1 {
            width
                .checked_add(7)
                .ok_or(TextureLoadError::InvalidData("BMP row is too wide"))?
                / 8
        } else if bits_per_pixel == 4 {
            width
                .checked_add(1)
                .ok_or(TextureLoadError::InvalidData("BMP row is too wide"))?
                / 2
        } else {
            width
                .checked_mul(bytes_per_pixel)
                .ok_or(TextureLoadError::InvalidData("BMP row is too wide"))?
        };
        let row_stride = row_bytes
            .checked_add(3)
            .ok_or(TextureLoadError::InvalidData("BMP row is too wide"))?
            & !3;
        let size = TextureSize::new(width, height_abs);
        let mut rgba8 = Vec::with_capacity(
            size.byte_len()
                .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?,
        );

        if is_rle8 || is_rle4 {
            let pixel_data = bytes
                .get(pixel_offset..)
                .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?;
            let indices = if is_rle8 {
                decode_bmp_rle8_indices(pixel_data, width, height_abs, top_down)?
            } else {
                decode_bmp_rle4_indices(pixel_data, width, height_abs, top_down)?
            };
            for &palette_index in &indices {
                let palette_index = usize::from(palette_index);
                if palette_index >= palette_entry_count {
                    return Err(TextureLoadError::InvalidData(
                        "BMP palette index exceeds palette length",
                    ));
                }
                let palette_offset = palette_index * 4;
                rgba8.extend_from_slice(&[
                    palette[palette_offset + 2],
                    palette[palette_offset + 1],
                    palette[palette_offset],
                    255,
                ]);
            }
        } else {
            let data_len = row_stride
                .checked_mul(height_abs)
                .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?;
            let data_end = pixel_offset
                .checked_add(data_len as usize)
                .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?;
            if data_end > bytes.len() {
                return Err(TextureLoadError::InvalidData(
                    "BMP pixel data extends past input",
                ));
            }

            for y in 0..height_abs {
                let source_y = if top_down { y } else { height_abs - 1 - y };
                let row_offset = pixel_offset + (source_y * row_stride) as usize;

                for x in 0..width {
                    let pixel_offset = row_offset + (x * bytes_per_pixel) as usize;
                    let (red, green, blue, alpha) = if matches!(bits_per_pixel, 1 | 4 | 8) {
                        let palette_index = if bits_per_pixel == 1 {
                            let packed = bytes[row_offset + (x / 8) as usize];
                            usize::from((packed >> (7 - (x % 8))) & 0x01)
                        } else if bits_per_pixel == 4 {
                            let packed = bytes[row_offset + (x / 2) as usize];
                            if x % 2 == 0 {
                                usize::from(packed >> 4)
                            } else {
                                usize::from(packed & 0x0f)
                            }
                        } else {
                            usize::from(bytes[pixel_offset])
                        };
                        if palette_index >= palette_entry_count {
                            return Err(TextureLoadError::InvalidData(
                                "BMP palette index exceeds palette length",
                            ));
                        }
                        let palette_offset = palette_index * 4;
                        (
                            palette[palette_offset + 2],
                            palette[palette_offset + 1],
                            palette[palette_offset],
                            255,
                        )
                    } else if is_bitfields {
                        let raw = if bits_per_pixel == 16 {
                            u32::from(read_u16_le(bytes, pixel_offset)?)
                        } else {
                            read_u32_le(bytes, pixel_offset)?
                        };
                        let red_mask = read_u32_le(bytes, 54)?;
                        let green_mask = read_u32_le(bytes, 58)?;
                        let blue_mask = read_u32_le(bytes, 62)?;
                        let alpha_mask = if pixel_offset >= 70 {
                            read_u32_le(bytes, 66).unwrap_or(0)
                        } else {
                            0
                        };
                        (
                            extract_bmp_bitfield_channel(raw, red_mask),
                            extract_bmp_bitfield_channel(raw, green_mask),
                            extract_bmp_bitfield_channel(raw, blue_mask),
                            if alpha_mask != 0 {
                                extract_bmp_bitfield_channel(raw, alpha_mask)
                            } else {
                                255
                            },
                        )
                    } else if bits_per_pixel == 16 {
                        let raw = u32::from(read_u16_le(bytes, pixel_offset)?);
                        (
                            extract_bmp_bitfield_channel(raw, 0x7C00),
                            extract_bmp_bitfield_channel(raw, 0x03E0),
                            extract_bmp_bitfield_channel(raw, 0x001F),
                            255,
                        )
                    } else {
                        let blue = bytes[pixel_offset];
                        let green = bytes[pixel_offset + 1];
                        let red = bytes[pixel_offset + 2];
                        let alpha = if bits_per_pixel == 32 {
                            bytes[pixel_offset + 3]
                        } else {
                            255
                        };
                        (red, green, blue, alpha)
                    };
                    rgba8.extend_from_slice(&[red, green, blue, alpha]);
                }
            }
        }

        Ok(Self { size, rgba8 })
    }

    pub fn from_tga_bytes(bytes: &[u8]) -> Result<Self, TextureLoadError> {
        const HEADER_LEN: usize = 18;
        const TYPE_UNCOMPRESSED_COLOR_MAPPED: u8 = 1;
        const TYPE_UNCOMPRESSED_TRUE_COLOR: u8 = 2;
        const TYPE_UNCOMPRESSED_GRAYSCALE: u8 = 3;
        const TYPE_RLE_COLOR_MAPPED: u8 = 9;
        const TYPE_RLE_TRUE_COLOR: u8 = 10;
        const TYPE_RLE_GRAYSCALE: u8 = 11;

        if bytes.len() < HEADER_LEN {
            return Err(TextureLoadError::InvalidData(
                "TGA is shorter than its header",
            ));
        }

        let id_len = usize::from(bytes[0]);
        let color_map_type = bytes[1];
        let image_type = bytes[2];
        if !matches!(
            image_type,
            TYPE_UNCOMPRESSED_COLOR_MAPPED
                | TYPE_RLE_COLOR_MAPPED
                | TYPE_UNCOMPRESSED_TRUE_COLOR
                | TYPE_UNCOMPRESSED_GRAYSCALE
                | TYPE_RLE_TRUE_COLOR
                | TYPE_RLE_GRAYSCALE
        ) {
            return Err(TextureLoadError::UnsupportedTga(
                "only color-mapped, true-color, and grayscale TGA data is supported",
            ));
        }
        let is_color_mapped = matches!(
            image_type,
            TYPE_UNCOMPRESSED_COLOR_MAPPED | TYPE_RLE_COLOR_MAPPED
        );
        if color_map_type > 1 || (is_color_mapped && color_map_type != 1) {
            return Err(TextureLoadError::UnsupportedTga(
                "indexed TGA data requires one color map",
            ));
        }
        if !is_color_mapped && color_map_type != 0 {
            return Err(TextureLoadError::UnsupportedTga(
                "true-color and grayscale TGA data must not declare a color map",
            ));
        }
        if !matches!(
            image_type,
            TYPE_UNCOMPRESSED_TRUE_COLOR
                | TYPE_UNCOMPRESSED_GRAYSCALE
                | TYPE_UNCOMPRESSED_COLOR_MAPPED
                | TYPE_RLE_TRUE_COLOR
                | TYPE_RLE_COLOR_MAPPED
                | TYPE_RLE_GRAYSCALE
        ) {
            return Err(TextureLoadError::UnsupportedTga(
                "only color-mapped, true-color, and grayscale TGA data is supported",
            ));
        }

        let width = u32::from(read_u16_le(bytes, 12)?);
        let height = u32::from(read_u16_le(bytes, 14)?);
        let bits_per_pixel = bytes[16];
        let descriptor = bytes[17];
        if width == 0 || height == 0 {
            return Err(TextureLoadError::InvalidData("TGA dimensions are invalid"));
        }
        let color_map_origin = usize::from(read_u16_le(bytes, 3)?);
        let color_map_len = usize::from(read_u16_le(bytes, 5)?);
        let color_map_bits = bytes[7];
        let is_true_color = matches!(
            image_type,
            TYPE_UNCOMPRESSED_TRUE_COLOR | TYPE_RLE_TRUE_COLOR
        );
        let is_grayscale = matches!(image_type, TYPE_UNCOMPRESSED_GRAYSCALE | TYPE_RLE_GRAYSCALE);
        let is_rle = matches!(
            image_type,
            TYPE_RLE_COLOR_MAPPED | TYPE_RLE_TRUE_COLOR | TYPE_RLE_GRAYSCALE
        );

        if is_color_mapped && bits_per_pixel != 8 {
            return Err(TextureLoadError::UnsupportedTga(
                "color-mapped TGA data must use 8-bit indices",
            ));
        }
        if is_color_mapped && color_map_len == 0 {
            return Err(TextureLoadError::UnsupportedTga(
                "color-mapped TGA data must define a palette",
            ));
        }
        if is_color_mapped && !matches!(color_map_bits, 24 | 32) {
            return Err(TextureLoadError::UnsupportedTga(
                "color-mapped TGA palettes must be 24-bit or 32-bit",
            ));
        }
        if is_true_color && !matches!(bits_per_pixel, 24 | 32) {
            return Err(TextureLoadError::UnsupportedTga(
                "true-color TGA data must be 24-bit or 32-bit",
            ));
        }
        if is_grayscale && bits_per_pixel != 8 {
            return Err(TextureLoadError::UnsupportedTga(
                "grayscale TGA data must be 8-bit",
            ));
        }

        let bytes_per_pixel = usize::from(bits_per_pixel / 8);
        let palette_bytes_per_entry = usize::from(color_map_bits / 8);
        let palette_offset = HEADER_LEN
            .checked_add(id_len)
            .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
        let palette_len = color_map_len
            .checked_mul(palette_bytes_per_entry)
            .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
        let pixel_offset = palette_offset
            .checked_add(palette_len)
            .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
        let pixel_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
        let source_len = pixel_count
            .checked_mul(bytes_per_pixel)
            .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
        let palette = if is_color_mapped {
            bytes
                .get(palette_offset..pixel_offset)
                .ok_or(TextureLoadError::InvalidData(
                    "TGA color map extends past input",
                ))?
        } else {
            &[]
        };
        let source = if is_rle {
            decode_tga_rle(
                bytes
                    .get(pixel_offset..)
                    .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?,
                bytes_per_pixel,
                pixel_count,
            )?
        } else {
            let pixel_end = pixel_offset
                .checked_add(source_len)
                .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
            bytes
                .get(pixel_offset..pixel_end)
                .ok_or(TextureLoadError::InvalidData(
                    "TGA pixel data extends past input",
                ))?
                .to_vec()
        };

        let size = TextureSize::new(width, height);
        let mut rgba8 = vec![
            0;
            size.byte_len()
                .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?
        ];
        let top_origin = descriptor & 0x20 != 0;
        let right_origin = descriptor & 0x10 != 0;

        for y in 0..height {
            let source_y = if top_origin { y } else { height - 1 - y };
            for x in 0..width {
                let source_x = if right_origin { width - 1 - x } else { x };
                let source_offset = ((source_y * width + source_x) as usize) * bytes_per_pixel;
                let destination_offset = ((y * width + x) as usize) * 4;
                if is_color_mapped {
                    let palette_index = usize::from(source[source_offset]);
                    let palette_index = palette_index.checked_sub(color_map_origin).ok_or(
                        TextureLoadError::InvalidData("TGA color index is below palette origin"),
                    )?;
                    if palette_index >= color_map_len {
                        return Err(TextureLoadError::InvalidData(
                            "TGA color index exceeds palette length",
                        ));
                    }
                    let palette_offset = palette_index * palette_bytes_per_entry;
                    let blue = palette[palette_offset];
                    let green = palette[palette_offset + 1];
                    let red = palette[palette_offset + 2];
                    let alpha = if color_map_bits == 32 {
                        palette[palette_offset + 3]
                    } else {
                        255
                    };
                    rgba8[destination_offset..destination_offset + 4]
                        .copy_from_slice(&[red, green, blue, alpha]);
                } else if is_grayscale {
                    let value = source[source_offset];
                    rgba8[destination_offset..destination_offset + 4]
                        .copy_from_slice(&[value, value, value, 255]);
                } else {
                    let blue = source[source_offset];
                    let green = source[source_offset + 1];
                    let red = source[source_offset + 2];
                    let alpha = if bits_per_pixel == 32 {
                        source[source_offset + 3]
                    } else {
                        255
                    };
                    rgba8[destination_offset..destination_offset + 4]
                        .copy_from_slice(&[red, green, blue, alpha]);
                }
            }
        }

        Ok(Self { size, rgba8 })
    }

    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, TextureLoadError> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder
            .read_info()
            .map_err(|error| TextureLoadError::Png(error.to_string()))?;
        let mut decoded = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut decoded)
            .map_err(|error| TextureLoadError::Png(error.to_string()))?;
        let pixels = decoded
            .get(..info.buffer_size())
            .ok_or(TextureLoadError::InvalidData(
                "PNG decoder returned invalid buffer",
            ))?;
        let size = TextureSize::new(info.width, info.height);
        if size.width == 0 || size.height == 0 {
            return Err(TextureLoadError::InvalidData("PNG dimensions are invalid"));
        }

        let mut rgba8 = Vec::with_capacity(
            size.byte_len()
                .ok_or(TextureLoadError::InvalidData("PNG data is too large"))?,
        );
        match info.color_type {
            png::ColorType::Rgba => rgba8.extend_from_slice(pixels),
            png::ColorType::Rgb => {
                for pixel in pixels.chunks_exact(3) {
                    rgba8.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
            png::ColorType::Grayscale => {
                for &value in pixels {
                    rgba8.extend_from_slice(&[value, value, value, 255]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for pixel in pixels.chunks_exact(2) {
                    rgba8.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
                }
            }
            png::ColorType::Indexed => {
                return Err(TextureLoadError::UnsupportedPng(
                    "indexed PNG data was not expanded by decoder",
                ));
            }
        }

        Self::rgba8(size, rgba8).ok_or(TextureLoadError::InvalidData(
            "PNG pixel data length does not match dimensions",
        ))
    }

    pub fn from_jpeg_bytes(bytes: &[u8]) -> Result<Self, TextureLoadError> {
        let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
        let pixels = decoder
            .decode()
            .map_err(|error| TextureLoadError::Jpeg(error.to_string()))?;
        let info = decoder
            .info()
            .ok_or(TextureLoadError::InvalidData("JPEG metadata is missing"))?;
        let size = TextureSize::new(u32::from(info.width), u32::from(info.height));
        if size.width == 0 || size.height == 0 {
            return Err(TextureLoadError::InvalidData("JPEG dimensions are invalid"));
        }

        let mut rgba8 = Vec::with_capacity(
            size.byte_len()
                .ok_or(TextureLoadError::InvalidData("JPEG data is too large"))?,
        );
        match info.pixel_format {
            jpeg_decoder::PixelFormat::L8 => {
                for &value in &pixels {
                    rgba8.extend_from_slice(&[value, value, value, 255]);
                }
            }
            jpeg_decoder::PixelFormat::RGB24 => {
                for pixel in pixels.chunks_exact(3) {
                    rgba8.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
            jpeg_decoder::PixelFormat::CMYK32 => {
                for pixel in pixels.chunks_exact(4) {
                    let cyan = u16::from(pixel[0]);
                    let magenta = u16::from(pixel[1]);
                    let yellow = u16::from(pixel[2]);
                    let key = u16::from(pixel[3]);
                    rgba8.extend_from_slice(&[
                        255u8.saturating_sub((cyan + key).min(255) as u8),
                        255u8.saturating_sub((magenta + key).min(255) as u8),
                        255u8.saturating_sub((yellow + key).min(255) as u8),
                        255,
                    ]);
                }
            }
            _ => {
                return Err(TextureLoadError::UnsupportedJpeg(
                    "unsupported JPEG pixel format",
                ));
            }
        }

        Self::rgba8(size, rgba8).ok_or(TextureLoadError::InvalidData(
            "JPEG pixel data length does not match dimensions",
        ))
    }

    pub fn from_webp_bytes(bytes: &[u8]) -> Result<Self, TextureLoadError> {
        let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::WebP)
            .map_err(|error| TextureLoadError::Webp(error.to_string()))?
            .to_rgba8();
        let size = TextureSize::new(decoded.width(), decoded.height());
        if size.width == 0 || size.height == 0 {
            return Err(TextureLoadError::InvalidData("WebP dimensions are invalid"));
        }

        Self::rgba8(size, decoded.into_raw()).ok_or(TextureLoadError::InvalidData(
            "WebP pixel data length does not match dimensions",
        ))
    }

    pub fn from_ktx2_bytes(bytes: &[u8]) -> Result<Self, TextureLoadError> {
        const HEADER_LEN: usize = 80;
        const LEVEL_INDEX_LEN: usize = 24;
        const VK_FORMAT_R8_UNORM: u32 = 9;
        const VK_FORMAT_R8_SRGB: u32 = 15;
        const VK_FORMAT_R8G8_UNORM: u32 = 16;
        const VK_FORMAT_R8G8_SRGB: u32 = 22;
        const VK_FORMAT_R8G8B8_UNORM: u32 = 23;
        const VK_FORMAT_R8G8B8_SRGB: u32 = 29;
        const VK_FORMAT_B8G8R8_UNORM: u32 = 30;
        const VK_FORMAT_B8G8R8_SRGB: u32 = 36;
        const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
        const VK_FORMAT_R8G8B8A8_SRGB: u32 = 43;
        const VK_FORMAT_B8G8R8A8_UNORM: u32 = 44;
        const VK_FORMAT_B8G8R8A8_SRGB: u32 = 50;

        if bytes.len() < HEADER_LEN + LEVEL_INDEX_LEN {
            return Err(TextureLoadError::InvalidData(
                "KTX2 is shorter than its header",
            ));
        }
        if !is_ktx2_bytes(bytes) {
            return Err(TextureLoadError::InvalidData("KTX2 signature is invalid"));
        }

        let vk_format = read_u32_le(bytes, 12)?;
        let type_size = read_u32_le(bytes, 16)?;
        let width = read_u32_le(bytes, 20)?;
        let height = read_u32_le(bytes, 24)?;
        let depth = read_u32_le(bytes, 28)?;
        let layer_count = read_u32_le(bytes, 32)?;
        let face_count = read_u32_le(bytes, 36)?;
        let level_count = read_u32_le(bytes, 40)?;
        let supercompression = read_u32_le(bytes, 44)?;

        if width == 0 || height == 0 {
            return Err(TextureLoadError::InvalidData("KTX2 dimensions are invalid"));
        }
        if depth != 0 {
            return Err(TextureLoadError::UnsupportedKtx2(
                "3D textures are not supported",
            ));
        }
        if layer_count > 1 {
            return Err(TextureLoadError::UnsupportedKtx2(
                "array textures are not supported",
            ));
        }
        if face_count != 1 {
            return Err(TextureLoadError::UnsupportedKtx2(
                "cubemap textures are not supported",
            ));
        }
        if level_count == 0 {
            return Err(TextureLoadError::UnsupportedKtx2(
                "implicit mip chains are not supported",
            ));
        }
        if type_size != 1 {
            return Err(TextureLoadError::UnsupportedKtx2(
                "only 8-bit KTX2 formats are supported",
            ));
        }

        let pixel_format = match vk_format {
            VK_FORMAT_R8_UNORM | VK_FORMAT_R8_SRGB => Ktx2PixelFormat::R,
            VK_FORMAT_R8G8_UNORM | VK_FORMAT_R8G8_SRGB => Ktx2PixelFormat::Rg,
            VK_FORMAT_R8G8B8_UNORM | VK_FORMAT_R8G8B8_SRGB => Ktx2PixelFormat::Rgb,
            VK_FORMAT_B8G8R8_UNORM | VK_FORMAT_B8G8R8_SRGB => Ktx2PixelFormat::Bgr,
            VK_FORMAT_R8G8B8A8_UNORM | VK_FORMAT_R8G8B8A8_SRGB => Ktx2PixelFormat::Rgba,
            VK_FORMAT_B8G8R8A8_UNORM | VK_FORMAT_B8G8R8A8_SRGB => Ktx2PixelFormat::Bgra,
            _ => {
                return Err(TextureLoadError::UnsupportedKtx2(
                    "only R/RG/RGB/BGR/RGBA/BGRA 8-bit KTX2 formats are supported",
                ));
            }
        };
        let source_pixel_size = pixel_format.source_pixel_size();
        let pixel_count = width
            .checked_mul(height)
            .ok_or(TextureLoadError::InvalidData("KTX2 data is too large"))?;
        let expected_level_len = pixel_count
            .checked_mul(source_pixel_size)
            .ok_or(TextureLoadError::InvalidData("KTX2 data is too large"))?
            as usize;

        let level_offset = usize::try_from(read_u64_le(bytes, HEADER_LEN)?)
            .map_err(|_| TextureLoadError::InvalidData("KTX2 level offset is too large"))?;
        let level_len = usize::try_from(read_u64_le(bytes, HEADER_LEN + 8)?)
            .map_err(|_| TextureLoadError::InvalidData("KTX2 level length is too large"))?;
        let uncompressed_level_len = usize::try_from(read_u64_le(bytes, HEADER_LEN + 16)?)
            .map_err(|_| TextureLoadError::InvalidData("KTX2 level length is too large"))?;
        let level_data_too_short = match supercompression {
            0 => level_len < expected_level_len,
            3 => false,
            _ => false,
        };
        if level_data_too_short
            || (uncompressed_level_len != 0 && uncompressed_level_len < expected_level_len)
        {
            return Err(TextureLoadError::InvalidData(
                "KTX2 level data is shorter than expected",
            ));
        }
        let level_end =
            level_offset
                .checked_add(level_len)
                .ok_or(TextureLoadError::InvalidData(
                    "KTX2 level data is too large",
                ))?;
        let level = bytes
            .get(level_offset..level_end)
            .ok_or(TextureLoadError::InvalidData(
                "KTX2 level data extends past input",
            ))?;
        let decompressed_level;
        let pixels = match supercompression {
            0 => level
                .get(..expected_level_len)
                .ok_or(TextureLoadError::InvalidData(
                    "KTX2 level data is shorter than expected",
                ))?,
            3 => {
                decompressed_level =
                    decode_ktx2_zlib_level(level, expected_level_len, uncompressed_level_len)?;
                decompressed_level.as_slice()
            }
            1 => {
                return Err(TextureLoadError::UnsupportedKtx2(
                    "BasisLZ supercompressed KTX2 data is not supported",
                ));
            }
            2 => {
                return Err(TextureLoadError::UnsupportedKtx2(
                    "Zstd supercompressed KTX2 data is not supported",
                ));
            }
            _ => {
                return Err(TextureLoadError::UnsupportedKtx2(
                    "unknown KTX2 supercompression scheme",
                ));
            }
        };

        let size = TextureSize::new(width, height);
        let mut rgba8 = Vec::with_capacity(
            size.byte_len()
                .ok_or(TextureLoadError::InvalidData("KTX2 data is too large"))?,
        );
        for pixel in pixels.chunks_exact(source_pixel_size as usize) {
            match pixel_format {
                Ktx2PixelFormat::R => rgba8.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255]),
                Ktx2PixelFormat::Rg => rgba8.extend_from_slice(&[pixel[0], pixel[1], 0, 255]),
                Ktx2PixelFormat::Rgb => {
                    rgba8.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255])
                }
                Ktx2PixelFormat::Bgr => {
                    rgba8.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255])
                }
                Ktx2PixelFormat::Rgba => {
                    rgba8.extend_from_slice(&[pixel[0], pixel[1], pixel[2], pixel[3]])
                }
                Ktx2PixelFormat::Bgra => {
                    rgba8.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]])
                }
            }
        }

        Self::rgba8(size, rgba8).ok_or(TextureLoadError::InvalidData(
            "KTX2 pixel data length does not match dimensions",
        ))
    }

    pub fn white_1x1() -> Self {
        Self::solid_rgba(TextureSize::new(1, 1), [255, 255, 255, 255])
    }

    pub fn solid_rgba(size: TextureSize, rgba: [u8; 4]) -> Self {
        let pixel_count = size.width.saturating_mul(size.height) as usize;
        let mut rgba8 = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            rgba8.extend_from_slice(&rgba);
        }

        Self { size, rgba8 }
    }

    pub fn checkerboard_rgba8(size: TextureSize, cell_size: u32, a: [u8; 4], b: [u8; 4]) -> Self {
        let cell_size = cell_size.max(1);
        let mut rgba8 = Vec::with_capacity(size.byte_len().unwrap_or(0));

        for y in 0..size.height {
            for x in 0..size.width {
                let use_a = (x / cell_size + y / cell_size) % 2 == 0;
                rgba8.extend_from_slice(if use_a { &a } else { &b });
            }
        }

        Self { size, rgba8 }
    }

    pub const fn size(&self) -> TextureSize {
        self.size
    }

    pub fn rgba8_data(&self) -> &[u8] {
        &self.rgba8
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, TextureLoadError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(TextureLoadError::InvalidData("unexpected end of data"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, TextureLoadError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(TextureLoadError::InvalidData("unexpected end of data"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, TextureLoadError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(TextureLoadError::InvalidData("unexpected end of data"))?;
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64_le(bytes: &[u8], offset: usize) -> Result<u64, TextureLoadError> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or(TextureLoadError::InvalidData("unexpected end of data"))?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn is_webp_bytes(bytes: &[u8]) -> bool {
    bytes.get(0..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
}

fn is_ktx2_bytes(bytes: &[u8]) -> bool {
    bytes.get(0..12) == Some(b"\xabKTX 20\xbb\r\n\x1a\n")
}

fn decode_ktx2_zlib_level(
    level: &[u8],
    expected_level_len: usize,
    uncompressed_level_len: usize,
) -> Result<Vec<u8>, TextureLoadError> {
    if uncompressed_level_len != 0 && uncompressed_level_len != expected_level_len {
        return Err(TextureLoadError::InvalidData(
            "KTX2 level data length does not match dimensions",
        ));
    }

    let mut decoder = ZlibDecoder::new(level);
    let mut pixels = Vec::with_capacity(uncompressed_level_len.max(expected_level_len));
    decoder
        .read_to_end(&mut pixels)
        .map_err(|error| TextureLoadError::Ktx2(error.to_string()))?;
    if pixels.len() != expected_level_len {
        return Err(TextureLoadError::InvalidData(
            "KTX2 pixel data length does not match dimensions",
        ));
    }
    Ok(pixels)
}

fn is_tga_bytes(bytes: &[u8]) -> bool {
    const HEADER_LEN: usize = 18;

    if bytes.len() >= 26 && bytes.ends_with(b"TRUEVISION-XFILE.\0") {
        return true;
    }
    if bytes.len() < HEADER_LEN {
        return false;
    }

    let id_len = usize::from(bytes[0]);
    let color_map_type = bytes[1];
    let image_type = bytes[2];
    let color_map_len = u16::from_le_bytes([bytes[5], bytes[6]]) as usize;
    let color_map_bits = bytes[7];
    let (is_color_mapped, is_true_color, is_grayscale, is_rle) = match image_type {
        1 => (true, false, false, false),
        2 => (false, true, false, false),
        3 => (false, false, true, false),
        9 => (true, false, false, true),
        10 => (false, true, false, true),
        11 => (false, false, true, true),
        _ => return false,
    };
    if color_map_type > 1 || (is_color_mapped && color_map_type != 1) {
        return false;
    }
    if !is_color_mapped && color_map_type != 0 {
        return false;
    }

    let width = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    let height = u16::from_le_bytes([bytes[14], bytes[15]]) as usize;
    if width == 0 || height == 0 {
        return false;
    }

    let bits_per_pixel = bytes[16];
    if is_color_mapped && bits_per_pixel != 8 {
        return false;
    }
    if is_color_mapped && (color_map_len == 0 || !matches!(color_map_bits, 24 | 32)) {
        return false;
    }
    if is_true_color && !matches!(bits_per_pixel, 24 | 32) {
        return false;
    }
    if is_grayscale && bits_per_pixel != 8 {
        return false;
    }

    let bytes_per_pixel = usize::from(bits_per_pixel / 8);
    let palette_bytes_per_entry = usize::from(color_map_bits / 8);
    let palette_offset = match HEADER_LEN.checked_add(id_len) {
        Some(offset) => offset,
        None => return false,
    };
    let palette_len = match color_map_len.checked_mul(palette_bytes_per_entry) {
        Some(len) => len,
        None => return false,
    };
    let pixel_offset = match palette_offset.checked_add(palette_len) {
        Some(offset) => offset,
        None => return false,
    };
    let pixel_count = match width.checked_mul(height) {
        Some(count) => count,
        None => return false,
    };

    if is_rle {
        return bytes
            .get(pixel_offset..)
            .is_some_and(|source| decode_tga_rle(source, bytes_per_pixel, pixel_count).is_ok());
    }

    let source_len = match pixel_count.checked_mul(bytes_per_pixel) {
        Some(len) => len,
        None => return false,
    };
    pixel_offset
        .checked_add(source_len)
        .is_some_and(|pixel_end| pixel_end <= bytes.len())
}

fn decode_bmp_rle8_indices(
    source: &[u8],
    width: u32,
    height: u32,
    top_down: bool,
) -> Result<Vec<u8>, TextureLoadError> {
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?;
    let mut output = vec![0u8; pixel_count];
    let mut offset = 0usize;
    let mut x = 0u32;
    let mut y = if top_down { 0i32 } else { height as i32 - 1 };
    let row_step = if top_down { 1i32 } else { -1i32 };

    while offset < source.len() {
        let count = source[offset];
        offset += 1;
        let command = *source.get(offset).ok_or(TextureLoadError::InvalidData(
            "BMP RLE8 stream is truncated",
        ))?;
        offset += 1;

        if count != 0 {
            write_bmp_rle8_run(&mut output, width, height, x, y, count, command)?;
            x = x
                .checked_add(u32::from(count))
                .ok_or(TextureLoadError::InvalidData("BMP RLE8 row is too wide"))?;
            continue;
        }

        match command {
            0 => {
                x = 0;
                y += row_step;
            }
            1 => return Ok(output),
            2 => {
                let dx = *source
                    .get(offset)
                    .ok_or(TextureLoadError::InvalidData("BMP RLE8 delta is truncated"))?;
                let dy = *source
                    .get(offset + 1)
                    .ok_or(TextureLoadError::InvalidData("BMP RLE8 delta is truncated"))?;
                offset += 2;
                x = x
                    .checked_add(u32::from(dx))
                    .ok_or(TextureLoadError::InvalidData("BMP RLE8 row is too wide"))?;
                y += i32::from(dy) * row_step;
            }
            literal_count => {
                let literal_len = usize::from(literal_count);
                let literals = source.get(offset..offset + literal_len).ok_or(
                    TextureLoadError::InvalidData("BMP RLE8 absolute run is truncated"),
                )?;
                for (index, &value) in literals.iter().enumerate() {
                    write_bmp_rle8_pixel(
                        &mut output,
                        width,
                        height,
                        x.checked_add(index as u32)
                            .ok_or(TextureLoadError::InvalidData("BMP RLE8 row is too wide"))?,
                        y,
                        value,
                    )?;
                }
                offset += literal_len;
                if literal_len % 2 != 0 {
                    offset = offset
                        .checked_add(1)
                        .ok_or(TextureLoadError::InvalidData("BMP RLE8 data is too large"))?;
                    if offset > source.len() {
                        return Err(TextureLoadError::InvalidData(
                            "BMP RLE8 absolute run padding is truncated",
                        ));
                    }
                }
                x = x
                    .checked_add(u32::from(literal_count))
                    .ok_or(TextureLoadError::InvalidData("BMP RLE8 row is too wide"))?;
            }
        }
    }

    Ok(output)
}

fn decode_bmp_rle4_indices(
    source: &[u8],
    width: u32,
    height: u32,
    top_down: bool,
) -> Result<Vec<u8>, TextureLoadError> {
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?;
    let mut output = vec![0u8; pixel_count];
    let mut offset = 0usize;
    let mut x = 0u32;
    let mut y = if top_down { 0i32 } else { height as i32 - 1 };
    let row_step = if top_down { 1i32 } else { -1i32 };

    while offset < source.len() {
        let count = source[offset];
        offset += 1;
        let command = *source.get(offset).ok_or(TextureLoadError::InvalidData(
            "BMP RLE4 stream is truncated",
        ))?;
        offset += 1;

        if count != 0 {
            let first = command >> 4;
            let second = command & 0x0f;
            for index in 0..u32::from(count) {
                let value = if index % 2 == 0 { first } else { second };
                write_bmp_rle8_pixel(&mut output, width, height, x + index, y, value)?;
            }
            x = x
                .checked_add(u32::from(count))
                .ok_or(TextureLoadError::InvalidData("BMP RLE4 row is too wide"))?;
            continue;
        }

        match command {
            0 => {
                x = 0;
                y += row_step;
            }
            1 => return Ok(output),
            2 => {
                let dx = *source
                    .get(offset)
                    .ok_or(TextureLoadError::InvalidData("BMP RLE4 delta is truncated"))?;
                let dy = *source
                    .get(offset + 1)
                    .ok_or(TextureLoadError::InvalidData("BMP RLE4 delta is truncated"))?;
                offset += 2;
                x = x
                    .checked_add(u32::from(dx))
                    .ok_or(TextureLoadError::InvalidData("BMP RLE4 row is too wide"))?;
                y += i32::from(dy) * row_step;
            }
            literal_count => {
                let literal_len = usize::from(literal_count);
                let packed_len = literal_len.div_ceil(2);
                let packed = source.get(offset..offset + packed_len).ok_or(
                    TextureLoadError::InvalidData("BMP RLE4 absolute run is truncated"),
                )?;
                for index in 0..literal_len {
                    let byte = packed[index / 2];
                    let value = if index % 2 == 0 {
                        byte >> 4
                    } else {
                        byte & 0x0f
                    };
                    write_bmp_rle8_pixel(
                        &mut output,
                        width,
                        height,
                        x.checked_add(index as u32)
                            .ok_or(TextureLoadError::InvalidData("BMP RLE4 row is too wide"))?,
                        y,
                        value,
                    )?;
                }
                offset += packed_len;
                if packed_len % 2 != 0 {
                    offset = offset
                        .checked_add(1)
                        .ok_or(TextureLoadError::InvalidData("BMP RLE4 data is too large"))?;
                    if offset > source.len() {
                        return Err(TextureLoadError::InvalidData(
                            "BMP RLE4 absolute run padding is truncated",
                        ));
                    }
                }
                x = x
                    .checked_add(u32::from(literal_count))
                    .ok_or(TextureLoadError::InvalidData("BMP RLE4 row is too wide"))?;
            }
        }
    }

    Ok(output)
}

fn write_bmp_rle8_run(
    output: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: i32,
    count: u8,
    value: u8,
) -> Result<(), TextureLoadError> {
    for index in 0..u32::from(count) {
        write_bmp_rle8_pixel(output, width, height, x + index, y, value)?;
    }
    Ok(())
}

fn write_bmp_rle8_pixel(
    output: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: i32,
    value: u8,
) -> Result<(), TextureLoadError> {
    if y < 0 || y >= height as i32 {
        return Err(TextureLoadError::InvalidData(
            "BMP RLE8 row exceeds image bounds",
        ));
    }
    if x >= width {
        return Err(TextureLoadError::InvalidData(
            "BMP RLE8 row exceeds image bounds",
        ));
    }
    let offset = usize::try_from(y as u32)
        .ok()
        .and_then(|y| {
            usize::try_from(width)
                .ok()
                .and_then(|width| y.checked_mul(width))
        })
        .and_then(|row_offset| {
            usize::try_from(x)
                .ok()
                .and_then(|x| row_offset.checked_add(x))
        })
        .ok_or(TextureLoadError::InvalidData("BMP data is too large"))?;
    output[offset] = value;
    Ok(())
}

fn extract_bmp_bitfield_channel(raw: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    let value = (raw & mask) >> shift;
    if max == 0 {
        0
    } else {
        ((value * 255 + max / 2) / max) as u8
    }
}

fn decode_tga_rle(
    source: &[u8],
    bytes_per_pixel: usize,
    pixel_count: usize,
) -> Result<Vec<u8>, TextureLoadError> {
    let output_len = pixel_count
        .checked_mul(bytes_per_pixel)
        .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
    let mut output = Vec::with_capacity(output_len);
    let mut offset = 0usize;

    while output.len() < output_len {
        let header = *source.get(offset).ok_or(TextureLoadError::InvalidData(
            "TGA RLE packet extends past input",
        ))?;
        offset += 1;

        let packet_pixels = usize::from((header & 0x7f) + 1);
        let remaining_pixels = (output_len - output.len()) / bytes_per_pixel;
        if packet_pixels > remaining_pixels {
            return Err(TextureLoadError::InvalidData(
                "TGA RLE packet emits too many pixels",
            ));
        }

        if header & 0x80 != 0 {
            let pixel = source.get(offset..offset + bytes_per_pixel).ok_or(
                TextureLoadError::InvalidData("TGA RLE packet extends past input"),
            )?;
            offset += bytes_per_pixel;
            for _ in 0..packet_pixels {
                output.extend_from_slice(pixel);
            }
        } else {
            let byte_count = packet_pixels
                .checked_mul(bytes_per_pixel)
                .ok_or(TextureLoadError::InvalidData("TGA data is too large"))?;
            let pixels =
                source
                    .get(offset..offset + byte_count)
                    .ok_or(TextureLoadError::InvalidData(
                        "TGA RLE packet extends past input",
                    ))?;
            offset += byte_count;
            output.extend_from_slice(pixels);
        }
    }

    Ok(output)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ktx2PixelFormat {
    R,
    Rg,
    Rgb,
    Bgr,
    Rgba,
    Bgra,
}

impl Ktx2PixelFormat {
    const fn source_pixel_size(self) -> u32 {
        match self {
            Self::R => 1,
            Self::Rg => 2,
            Self::Rgb | Self::Bgr => 3,
            Self::Rgba | Self::Bgra => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bmp_loader_decodes_bottom_up_24_bit_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_24_bottom_up_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,]
        );
    }

    #[test]
    fn bmp_loader_decodes_top_down_32_bit_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_32_top_down_1x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 2));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn bmp_loader_decodes_bottom_up_8_bit_paletted_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_8_bottom_up_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn bmp_loader_decodes_bottom_up_4_bit_paletted_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_4_bottom_up_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn bmp_loader_decodes_bottom_up_1_bit_paletted_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_1_bottom_up_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255]
        );
    }

    #[test]
    fn bmp_loader_decodes_rle8_paletted_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_rle8_bottom_up_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn bmp_loader_decodes_16_bit_565_bitfields_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_16_bitfields_565_top_down_2x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 1));
        assert_eq!(texture.rgba8_data(), &[255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn bmp_loader_decodes_16_bit_rgb555_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_16_rgb555_top_down_2x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 1));
        assert_eq!(texture.rgba8_data(), &[255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn bmp_loader_decodes_32_bit_bitfields_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_32_bitfields_rgba_top_down_2x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn bmp_loader_decodes_rle4_paletted_pixels_to_rgba8() {
        let texture = Texture::from_bmp_bytes(&bmp_rle4_bottom_up_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn tga_loader_decodes_bottom_left_24_bit_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_24_bottom_left_2x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 2));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,]
        );
    }

    #[test]
    fn tga_loader_decodes_top_left_32_bit_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_32_top_left_1x2()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 2));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn tga_loader_decodes_grayscale_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_grayscale_top_left_2x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 1));
        assert_eq!(texture.rgba8_data(), &[10, 10, 10, 255, 200, 200, 200, 255]);
    }

    #[test]
    fn tga_loader_decodes_color_mapped_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_color_mapped_top_left_2x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 1));
        assert_eq!(texture.rgba8_data(), &[255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn tga_loader_decodes_rle_true_color_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_rle_24_top_left_3x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(3, 1));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255]
        );
    }

    #[test]
    fn tga_loader_decodes_rle_grayscale_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_rle_grayscale_top_left_4x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(4, 1));
        assert_eq!(
            texture.rgba8_data(),
            &[10, 10, 10, 255, 20, 20, 20, 255, 200, 200, 200, 255, 200, 200, 200, 255,]
        );
    }

    #[test]
    fn tga_loader_decodes_rle_color_mapped_pixels_to_rgba8() {
        let texture = Texture::from_tga_bytes(&tga_rle_color_mapped_top_left_3x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(3, 1));
        assert_eq!(
            texture.rgba8_data(),
            &[255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 255, 255]
        );
    }

    #[test]
    fn tga_loader_rejects_rle_packets_that_overrun_pixel_count() {
        assert_eq!(
            Texture::from_tga_bytes(&tga_rle_packet_overrun_1x1()),
            Err(TextureLoadError::InvalidData(
                "TGA RLE packet emits too many pixels"
            ))
        );
    }

    #[test]
    fn image_loader_dispatches_by_extension() {
        assert!(Texture::from_image_bytes("albedo.bmp", &bmp_24_bottom_up_2x2()).is_ok());
        assert!(Texture::from_image_bytes("albedo.tga", &tga_24_bottom_left_2x2()).is_ok());
        assert!(Texture::from_image_bytes("albedo.targa", &tga_24_bottom_left_2x2()).is_ok());
        assert!(Texture::from_image_bytes("albedo.png", &png_rgba_2x1()).is_ok());
        assert!(Texture::from_image_bytes("albedo.jpg", &valid_jpeg_red_1x1()).is_ok());
        assert!(Texture::from_image_bytes("albedo.jpeg", &valid_jpeg_red_1x1()).is_ok());
        assert!(Texture::from_image_bytes("albedo.webp", &valid_webp_1x1()).is_ok());
        assert!(Texture::from_image_bytes("albedo.ktx2", &ktx2_rgba8_1x1()).is_ok());
    }

    #[test]
    fn image_loader_falls_back_to_magic_bytes_without_known_extension() {
        assert!(Texture::from_image_bytes("embedded.bin", &bmp_24_bottom_up_2x2()).is_ok());
        assert!(Texture::from_image_bytes("embedded.bin", &png_rgba_2x1()).is_ok());
        assert!(Texture::from_image_bytes("embedded.bin", &valid_jpeg_red_1x1()).is_ok());
        assert!(Texture::from_image_bytes("embedded.bin", &valid_webp_1x1()).is_ok());
        assert!(Texture::from_image_bytes("embedded.bin", &ktx2_rgba8_1x1()).is_ok());
        assert!(Texture::from_image_bytes("embedded.bin", &tga_32_top_left_1x2()).is_ok());
        assert!(
            Texture::from_image_bytes("embedded.bin", &tga_color_mapped_top_left_2x1()).is_ok()
        );
        assert!(Texture::from_image_bytes("embedded.bin", &tga_rle_24_top_left_3x1()).is_ok());
        assert!(
            Texture::from_image_bytes("embedded.bin", &tga_rle_color_mapped_top_left_3x1()).is_ok()
        );
        assert!(
            Texture::from_image_bytes("embedded.bin", &tga_24_bottom_left_2x2_with_footer())
                .is_ok()
        );
    }

    #[test]
    fn png_loader_decodes_rgba8_pixels() {
        let texture = Texture::from_png_bytes(&png_rgba_2x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(2, 1));
        assert_eq!(texture.rgba8_data(), &[255, 0, 0, 255, 0, 128, 255, 64]);
    }

    #[test]
    fn jpeg_loader_decodes_rgb_pixels_to_rgba8() {
        let texture = Texture::from_jpeg_bytes(&valid_jpeg_red_1x1()).unwrap();
        let pixel = texture.rgba8_data();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert!(pixel[0] > 200);
        assert!(pixel[1] < 80);
        assert!(pixel[2] < 80);
        assert_eq!(pixel[3], 255);
    }

    #[test]
    fn webp_loader_decodes_pixels_to_rgba8() {
        let texture = Texture::from_webp_bytes(&valid_webp_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data().len(), 4);
    }

    #[test]
    fn ktx2_loader_decodes_uncompressed_rgba8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_rgba8_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn ktx2_loader_decodes_uncompressed_rgb8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_rgb8_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 255]);
    }

    #[test]
    fn ktx2_loader_decodes_uncompressed_r8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_r8_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 10, 10, 255]);
    }

    #[test]
    fn ktx2_loader_decodes_uncompressed_rg8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_rg8_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 0, 255]);
    }

    #[test]
    fn ktx2_loader_decodes_uncompressed_bgr8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_bgr8_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 255]);
    }

    #[test]
    fn ktx2_loader_decodes_uncompressed_bgra8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_bgra8_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn ktx2_loader_decodes_zlib_supercompressed_bgr8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_bgr8_zlib_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 255]);
    }

    #[test]
    fn ktx2_loader_decodes_zlib_supercompressed_bgra8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_bgra8_zlib_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn ktx2_loader_decodes_zlib_supercompressed_rgba8_pixels() {
        let texture = Texture::from_ktx2_bytes(&ktx2_rgba8_zlib_1x1()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(1, 1));
        assert_eq!(texture.rgba8_data(), &[10, 20, 30, 40]);
    }

    #[test]
    fn ktx2_loader_accepts_zlib_payload_shorter_than_uncompressed_level() {
        let texture = Texture::from_ktx2_bytes(&ktx2_rgba8_zlib_4x4_repeated()).unwrap();

        assert_eq!(texture.size(), TextureSize::new(4, 4));
        assert_eq!(texture.rgba8_data().len(), 4 * 4 * 4);
        assert!(texture
            .rgba8_data()
            .chunks_exact(4)
            .all(|pixel| pixel == [10, 20, 30, 40]));
    }

    #[test]
    fn ktx2_loader_rejects_basislz_supercompressed_payloads() {
        let mut bytes = ktx2_rgba8_1x1();
        bytes[44..48].copy_from_slice(&1u32.to_le_bytes());

        assert_eq!(
            Texture::from_ktx2_bytes(&bytes),
            Err(TextureLoadError::UnsupportedKtx2(
                "BasisLZ supercompressed KTX2 data is not supported"
            ))
        );
    }

    #[test]
    fn ktx2_loader_reports_supported_channel_layouts_in_error_message() {
        let mut bytes = ktx2_rgba8_1x1();
        bytes[12..16].copy_from_slice(&999u32.to_le_bytes());

        assert_eq!(
            Texture::from_ktx2_bytes(&bytes),
            Err(TextureLoadError::UnsupportedKtx2(
                "only R/RG/RGB/BGR/RGBA/BGRA 8-bit KTX2 formats are supported"
            ))
        );
    }

    fn bmp_24_bottom_up_2x2() -> Vec<u8> {
        let mut bytes = bmp_header(2, 2, 24, 16);
        bytes.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0]);
        bytes
    }

    fn bmp_32_top_down_1x2() -> Vec<u8> {
        let mut bytes = bmp_header(1, -2, 32, 8);
        bytes.extend_from_slice(&[30, 20, 10, 40, 70, 60, 50, 80]);
        bytes
    }

    fn bmp_8_bottom_up_2x2() -> Vec<u8> {
        let color_table_len = 16u32;
        let pixel_data_len = 8u32;
        let mut bytes = bmp_header_with_offset(2, 2, 8, pixel_data_len, 54 + color_table_len);
        bytes[46..50].copy_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 255, 0]);
        bytes.extend_from_slice(&[0, 255, 0, 0]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[255, 255, 255, 0]);
        bytes.extend_from_slice(&[2, 3, 0, 0, 0, 1, 0, 0]);
        bytes
    }

    fn bmp_4_bottom_up_2x2() -> Vec<u8> {
        let color_table_len = 16u32;
        let pixel_data_len = 8u32;
        let mut bytes = bmp_header_with_offset(2, 2, 4, pixel_data_len, 54 + color_table_len);
        bytes[46..50].copy_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 255, 0]);
        bytes.extend_from_slice(&[0, 255, 0, 0]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[255, 255, 255, 0]);
        bytes.extend_from_slice(&[0x23, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);
        bytes
    }

    fn bmp_1_bottom_up_2x2() -> Vec<u8> {
        let color_table_len = 8u32;
        let pixel_data_len = 8u32;
        let mut bytes = bmp_header_with_offset(2, 2, 1, pixel_data_len, 54 + color_table_len);
        bytes[46..50].copy_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 255, 0]);
        bytes.extend_from_slice(&[0, 255, 0, 0]);
        bytes.extend_from_slice(&[0b1000_0000, 0, 0, 0, 0b0110_0000, 0, 0, 0]);
        bytes
    }

    fn bmp_rle8_bottom_up_2x2() -> Vec<u8> {
        let color_table_len = 16u32;
        let pixel_data_len = 14u32;
        let mut bytes = bmp_header_with_offset(2, 2, 8, pixel_data_len, 54 + color_table_len);
        bytes[30..34].copy_from_slice(&1u32.to_le_bytes());
        bytes[34..38].copy_from_slice(&pixel_data_len.to_le_bytes());
        bytes[46..50].copy_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 255, 0]);
        bytes.extend_from_slice(&[0, 255, 0, 0]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[255, 255, 255, 0]);
        bytes.extend_from_slice(&[1, 2, 1, 3, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1]);
        bytes
    }

    fn bmp_rle4_bottom_up_2x2() -> Vec<u8> {
        let color_table_len = 16u32;
        let pixel_data_len = 14u32;
        let mut bytes = bmp_header_with_offset(2, 2, 4, pixel_data_len, 54 + color_table_len);
        bytes[30..34].copy_from_slice(&2u32.to_le_bytes());
        bytes[34..38].copy_from_slice(&pixel_data_len.to_le_bytes());
        bytes[46..50].copy_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 255, 0]);
        bytes.extend_from_slice(&[0, 255, 0, 0]);
        bytes.extend_from_slice(&[255, 0, 0, 0]);
        bytes.extend_from_slice(&[255, 255, 255, 0]);
        bytes.extend_from_slice(&[1, 0x20, 1, 0x30, 0, 0, 1, 0x00, 1, 0x10, 0, 0, 0, 1]);
        bytes
    }

    fn bmp_16_bitfields_565_top_down_2x1() -> Vec<u8> {
        let pixel_data_len = 4u32;
        let pixel_offset = 66u32;
        let mut bytes = bmp_header_with_offset(2, -1, 16, pixel_data_len, pixel_offset);
        bytes[30..34].copy_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&0xF800u32.to_le_bytes());
        bytes.extend_from_slice(&0x07E0u32.to_le_bytes());
        bytes.extend_from_slice(&0x001Fu32.to_le_bytes());
        bytes.extend_from_slice(&0xF800u16.to_le_bytes());
        bytes.extend_from_slice(&0x07E0u16.to_le_bytes());
        bytes
    }

    fn bmp_16_rgb555_top_down_2x1() -> Vec<u8> {
        let mut bytes = bmp_header(2, -1, 16, 4);
        bytes.extend_from_slice(&0x7C00u16.to_le_bytes());
        bytes.extend_from_slice(&0x03E0u16.to_le_bytes());
        bytes
    }

    fn bmp_32_bitfields_rgba_top_down_2x1() -> Vec<u8> {
        let pixel_data_len = 8u32;
        let pixel_offset = 70u32;
        let mut bytes = bmp_header_with_offset(2, -1, 32, pixel_data_len, pixel_offset);
        bytes[30..34].copy_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&0x00FF_0000u32.to_le_bytes());
        bytes.extend_from_slice(&0x0000_FF00u32.to_le_bytes());
        bytes.extend_from_slice(&0x0000_00FFu32.to_le_bytes());
        bytes.extend_from_slice(&0xFF00_0000u32.to_le_bytes());
        bytes.extend_from_slice(&0x280A_141Eu32.to_le_bytes());
        bytes.extend_from_slice(&0x5032_3C46u32.to_le_bytes());
        bytes
    }

    fn tga_24_bottom_left_2x2() -> Vec<u8> {
        let mut bytes = tga_header(2, 2, 24, 2, 0, 12);
        bytes.extend_from_slice(&[255, 0, 0, 255, 255, 255, 0, 0, 255, 0, 255, 0]);
        bytes
    }

    fn tga_24_bottom_left_2x2_with_footer() -> Vec<u8> {
        let mut bytes = tga_24_bottom_left_2x2();
        push_tga_footer(&mut bytes);
        bytes
    }

    fn tga_32_top_left_1x2() -> Vec<u8> {
        let mut bytes = tga_header(1, 2, 32, 2, 0x20, 8);
        bytes.extend_from_slice(&[30, 20, 10, 40, 70, 60, 50, 80]);
        bytes
    }

    fn tga_grayscale_top_left_2x1() -> Vec<u8> {
        let mut bytes = tga_header(2, 1, 8, 3, 0x20, 2);
        bytes.extend_from_slice(&[10, 200]);
        bytes
    }

    fn tga_color_mapped_top_left_2x1() -> Vec<u8> {
        let mut bytes = tga_header(2, 1, 8, 1, 0x20, 2);
        bytes[1] = 1;
        bytes[5..7].copy_from_slice(&2u16.to_le_bytes());
        bytes[7] = 24;
        bytes.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
        bytes.extend_from_slice(&[0, 1]);
        bytes
    }

    fn tga_rle_24_top_left_3x1() -> Vec<u8> {
        let mut bytes = tga_header(3, 1, 24, 10, 0x20, 8);
        bytes.extend_from_slice(&[0x81, 0, 0, 255, 0, 255, 0, 0]);
        bytes
    }

    fn tga_rle_color_mapped_top_left_3x1() -> Vec<u8> {
        let mut bytes = tga_header(3, 1, 8, 9, 0x20, 5);
        bytes[1] = 1;
        bytes[5..7].copy_from_slice(&2u16.to_le_bytes());
        bytes[7] = 24;
        bytes.extend_from_slice(&[0, 0, 255, 255, 0, 0]);
        bytes.extend_from_slice(&[0x81, 0, 0x00, 1]);
        bytes
    }

    fn tga_rle_grayscale_top_left_4x1() -> Vec<u8> {
        let mut bytes = tga_header(4, 1, 8, 11, 0x20, 5);
        bytes.extend_from_slice(&[0x01, 10, 20, 0x81, 200]);
        bytes
    }

    fn tga_rle_packet_overrun_1x1() -> Vec<u8> {
        let mut bytes = tga_header(1, 1, 24, 10, 0x20, 4);
        bytes.extend_from_slice(&[0x81, 0, 0, 255]);
        bytes
    }

    fn tga_header(
        width: u16,
        height: u16,
        bits_per_pixel: u8,
        image_type: u8,
        descriptor: u8,
        pixel_data_len: usize,
    ) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(18 + pixel_data_len);
        bytes.extend_from_slice(&[0, 0, image_type]);
        bytes.extend_from_slice(&[0; 5]);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.push(bits_per_pixel);
        bytes.push(descriptor);
        bytes
    }

    fn push_tga_footer(bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(b"TRUEVISION-XFILE.");
        bytes.push(0);
    }

    fn bmp_header(width: i32, height: i32, bits_per_pixel: u16, pixel_data_len: u32) -> Vec<u8> {
        bmp_header_with_offset(width, height, bits_per_pixel, pixel_data_len, 54)
    }

    fn bmp_header_with_offset(
        width: i32,
        height: i32,
        bits_per_pixel: u16,
        pixel_data_len: u32,
        pixel_offset: u32,
    ) -> Vec<u8> {
        let file_size = pixel_offset + pixel_data_len;
        let mut bytes = Vec::with_capacity(file_size as usize);
        bytes.extend_from_slice(b"BM");
        bytes.extend_from_slice(&file_size.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(&pixel_offset.to_le_bytes());
        bytes.extend_from_slice(&40u32.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&bits_per_pixel.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&pixel_data_len.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        bytes
    }

    fn png_rgba_2x1() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        push_png_chunk(&mut bytes, b"IHDR", &ihdr);

        let raw = [0, 255, 0, 0, 255, 0, 128, 255, 64];
        let mut zlib = Vec::new();
        zlib.extend_from_slice(&[0x78, 0x01, 0x01]);
        zlib.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(&raw);
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        push_png_chunk(&mut bytes, b"IDAT", &zlib);
        push_png_chunk(&mut bytes, b"IEND", &[]);

        bytes
    }

    fn push_png_chunk(bytes: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(data);
        let mut crc_input = Vec::with_capacity(kind.len() + data.len());
        crc_input.extend_from_slice(kind);
        crc_input.extend_from_slice(data);
        bytes.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = 0u32.wrapping_sub(crc & 1);
                crc = (crc >> 1) ^ (0xedb8_8320 & mask);
            }
        }
        !crc
    }

    fn adler32(bytes: &[u8]) -> u32 {
        let mut a = 1u32;
        let mut b = 0u32;
        for &byte in bytes {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    #[allow(dead_code)]
    fn jpeg_red_1x1() -> Vec<u8> {
        vec![
            255, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 255, 219, 0,
            67, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 255, 219, 0, 67, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 255, 192, 0, 17, 8, 0, 1,
            0, 1, 3, 1, 17, 0, 2, 17, 1, 3, 17, 1, 255, 196, 0, 31, 0, 0, 1, 5, 1, 1, 1, 1, 1, 1,
            0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 16, 0, 2, 1,
            3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125, 1, 2, 3, 0, 4, 17, 5, 18, 33, 49, 65, 6, 19,
            81, 97, 7, 34, 113, 20, 50, 129, 145, 161, 8, 35, 66, 177, 193, 21, 82, 209, 240, 36,
            51, 98, 114, 130, 9, 10, 22, 23, 24, 25, 26, 37, 38, 39, 40, 41, 42, 52, 53, 54, 55,
            56, 57, 58, 67, 68, 69, 70, 71, 72, 73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100,
            101, 102, 103, 104, 105, 106, 115, 116, 117, 118, 119, 120, 121, 122, 131, 132, 133,
            134, 135, 136, 137, 138, 146, 147, 148, 149, 150, 151, 152, 153, 154, 162, 163, 164,
            165, 166, 167, 168, 169, 170, 178, 179, 180, 181, 182, 183, 184, 185, 186, 194, 195,
            196, 197, 198, 199, 200, 201, 202, 210, 211, 212, 213, 214, 215, 216, 217, 218, 225,
            226, 227, 228, 229, 230, 231, 232, 233, 234, 241, 242, 243, 244, 245, 246, 247, 248,
            249, 250, 255, 196, 0, 31, 1, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 2,
            3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 196, 0, 181, 17, 0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4,
            0, 1, 2, 119, 0, 1, 2, 3, 17, 4, 5, 33, 49, 6, 18, 65, 81, 7, 97, 113, 19, 34, 50, 129,
            8, 20, 66, 145, 161, 177, 193, 9, 35, 51, 82, 240, 21, 98, 114, 209, 10, 22, 36, 52,
            225, 37, 241, 23, 24, 25, 26, 38, 39, 40, 41, 42, 53, 54, 55, 56, 57, 58, 67, 68, 69,
            70, 71, 72, 73, 74, 83, 84, 85, 86, 87, 88, 89, 90, 99, 100, 101, 102, 103, 104, 105,
            106, 115, 116, 117, 118, 119, 120, 121, 122, 130, 131, 132, 133, 134, 135, 136, 137,
            138, 146, 147, 148, 149, 150, 151, 152, 153, 154, 162, 163, 164, 165, 166, 167, 168,
            169, 170, 178, 179, 180, 181, 182, 183, 184, 185, 186, 194, 195, 196, 197, 198, 199,
            200, 201, 202, 210, 211, 212, 213, 214, 215, 216, 217, 218, 226, 227, 228, 229, 230,
            231, 232, 233, 234, 242, 243, 244, 245, 246, 247, 248, 249, 250, 255, 218, 0, 12, 3, 1,
            0, 2, 17, 3, 17, 0, 63, 0, 252, 95, 175, 242, 156, 255, 0, 191, 131, 255, 217,
        ]
    }

    fn valid_jpeg_red_1x1() -> Vec<u8> {
        decode_base64(
            "\
/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQH/\
2wBDAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQH/wAARCAABAAEDAREAAhEBAxEB/\
8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/\
8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/\
8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/\
8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/\
9oADAMBAAIRAxEAPwD8X6/ynP8Av4P/2Q==",
        )
    }

    fn valid_webp_1x1() -> Vec<u8> {
        decode_base64("UklGRiIAAABXRUJQVlA4IBYAAAAwAQCdASoBAAEADsD+JaQAA3AAAAAA")
    }

    fn ktx2_rgba8_1x1() -> Vec<u8> {
        ktx2_1x1(37, &[10, 20, 30, 40])
    }

    fn ktx2_rgb8_1x1() -> Vec<u8> {
        ktx2_1x1(23, &[10, 20, 30])
    }

    fn ktx2_r8_1x1() -> Vec<u8> {
        ktx2_1x1(9, &[10])
    }

    fn ktx2_rg8_1x1() -> Vec<u8> {
        ktx2_1x1(16, &[10, 20])
    }

    fn ktx2_bgr8_1x1() -> Vec<u8> {
        ktx2_1x1(30, &[30, 20, 10])
    }

    fn ktx2_bgra8_1x1() -> Vec<u8> {
        ktx2_1x1(44, &[30, 20, 10, 40])
    }

    fn ktx2_bgr8_zlib_1x1() -> Vec<u8> {
        ktx2_1x1_supercompressed(30, &[30, 20, 10], 3)
    }

    fn ktx2_bgra8_zlib_1x1() -> Vec<u8> {
        ktx2_1x1_supercompressed(44, &[30, 20, 10, 40], 3)
    }

    fn ktx2_rgba8_zlib_1x1() -> Vec<u8> {
        ktx2_1x1_supercompressed(37, &[10, 20, 30, 40], 3)
    }

    fn ktx2_rgba8_zlib_4x4_repeated() -> Vec<u8> {
        let mut pixel_data = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            pixel_data.extend_from_slice(&[10, 20, 30, 40]);
        }
        ktx2_level(
            37,
            4,
            4,
            &zlib_compress(&pixel_data),
            pixel_data.len() as u64,
            3,
        )
    }

    fn ktx2_1x1(vk_format: u32, pixel: &[u8]) -> Vec<u8> {
        ktx2_level(vk_format, 1, 1, pixel, pixel.len() as u64, 0)
    }

    fn ktx2_1x1_supercompressed(vk_format: u32, pixel: &[u8], supercompression: u32) -> Vec<u8> {
        let level = match supercompression {
            3 => zlib_store_block(pixel),
            _ => pixel.to_vec(),
        };
        ktx2_level(
            vk_format,
            1,
            1,
            &level,
            pixel.len() as u64,
            supercompression,
        )
    }

    fn ktx2_level(
        vk_format: u32,
        width: u32,
        height: u32,
        level: &[u8],
        uncompressed_level_len: u64,
        supercompression: u32,
    ) -> Vec<u8> {
        const HEADER_LEN: usize = 80;
        const LEVEL_INDEX_LEN: usize = 24;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\xabKTX 20\xbb\r\n\x1a\n");
        bytes.extend_from_slice(&vk_format.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&supercompression.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        debug_assert_eq!(bytes.len(), HEADER_LEN);
        bytes.extend_from_slice(&((HEADER_LEN + LEVEL_INDEX_LEN) as u64).to_le_bytes());
        bytes.extend_from_slice(&(level.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&uncompressed_level_len.to_le_bytes());
        bytes.extend_from_slice(level);
        bytes
    }

    fn zlib_store_block(raw: &[u8]) -> Vec<u8> {
        let mut zlib = Vec::new();
        zlib.extend_from_slice(&[0x78, 0x01, 0x01]);
        zlib.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(raw);
        zlib.extend_from_slice(&adler32(raw).to_be_bytes());
        zlib
    }

    fn zlib_compress(raw: &[u8]) -> Vec<u8> {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(raw).unwrap();
        encoder.finish().unwrap()
    }

    fn decode_base64(source: &str) -> Vec<u8> {
        let mut output = Vec::new();
        let mut chunk = [0u8; 4];
        let mut chunk_len = 0;

        for byte in source.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
            chunk[chunk_len] = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => 64,
                _ => panic!("invalid base64 test data"),
            };
            chunk_len += 1;

            if chunk_len == 4 {
                output.push((chunk[0] << 2) | (chunk[1] >> 4));
                if chunk[2] != 64 {
                    output.push((chunk[1] << 4) | (chunk[2] >> 2));
                }
                if chunk[3] != 64 {
                    output.push((chunk[2] << 6) | chunk[3]);
                }
                chunk_len = 0;
            }
        }

        output
    }
}
