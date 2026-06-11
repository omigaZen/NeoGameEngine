use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    id::AssetTypeId,
    io::stable_hash,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

#[derive(Clone, Debug, PartialEq)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: AudioSamples,
    pub duration_seconds: f32,
    pub streaming: bool,
}

impl Asset for AudioClip {
    const TYPE_NAME: &'static str = "AudioClip";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0005);
}

impl AssetMemoryUsage for AudioClip {
    fn cpu_bytes(&self) -> u64 {
        match &self.samples {
            AudioSamples::I16(samples) => (samples.len() * 2) as u64,
            AudioSamples::F32(samples) => (samples.len() * 4) as u64,
            AudioSamples::Streaming(_) => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioSamples {
    I16(Vec<i16>),
    F32(Vec<f32>),
    Streaming(AudioStreamHandle),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AudioStreamHandle(pub u64);

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioImportSettings {
    pub force_mono: bool,
    pub normalize: bool,
    pub streaming: bool,
    pub compression: AudioCompression,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCompression {
    None,
    Vorbis,
    Opus,
}

pub struct AudioLoader;

impl AudioLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AudioLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for AudioLoader {
    fn name(&self) -> &'static str {
        "AudioLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["audio", "wav", "ogg"]
    }

    fn asset_type(&self) -> AssetTypeId {
        AudioClip::TYPE_ID
    }

    fn load(
        &self,
        _ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_audio_clip(bytes).map(LoadedAsset::new)
    }
}

fn parse_audio_clip(bytes: &[u8]) -> Result<AudioClip, AssetError> {
    if bytes.starts_with(b"RIFF") {
        return parse_wav_audio_clip(bytes);
    }
    if bytes.starts_with(b"OggS") {
        return parse_ogg_audio_clip(bytes);
    }

    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("audio source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_AUDIO_V1" {
        return Err(AssetError::Decode {
            message: "audio source must start with NGA_AUDIO_V1".to_owned(),
        });
    }

    let mut sample_rate = None;
    let mut channels = None;
    let mut format = None;
    let mut samples = None;
    let mut streaming = false;

    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid audio line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "sample_rate" => sample_rate = Some(parse_u32(value, key, line_number)?),
            "channels" => channels = Some(parse_u16(value, key, line_number)?),
            "format" => format = Some(value.to_owned()),
            "samples" => samples = Some(value.to_owned()),
            "streaming" => streaming = parse_bool(value, line_number)?,
            other => {
                return Err(AssetError::Decode {
                    message: format!("unknown audio key `{other}` on line {line_number}"),
                })
            }
        }
    }

    let sample_rate = sample_rate.ok_or_else(|| AssetError::Decode {
        message: "audio source missing sample_rate".to_owned(),
    })?;
    if sample_rate == 0 {
        return Err(AssetError::Decode {
            message: "audio sample_rate must be greater than zero".to_owned(),
        });
    }
    let channels = channels.ok_or_else(|| AssetError::Decode {
        message: "audio source missing channels".to_owned(),
    })?;
    if channels == 0 {
        return Err(AssetError::Decode {
            message: "audio channels must be greater than zero".to_owned(),
        });
    }
    let format = format.ok_or_else(|| AssetError::Decode {
        message: "audio source missing format".to_owned(),
    })?;
    let samples = samples.ok_or_else(|| AssetError::Decode {
        message: "audio source missing samples".to_owned(),
    })?;

    let parsed_samples = match format.as_str() {
        "i16" => AudioSamples::I16(parse_i16_samples(&samples, channels)?),
        "f32" => AudioSamples::F32(parse_f32_samples(&samples, channels)?),
        other => {
            return Err(AssetError::Decode {
                message: format!("unsupported audio format `{other}`"),
            })
        }
    };
    let sample_count = match &parsed_samples {
        AudioSamples::I16(samples) => samples.len(),
        AudioSamples::F32(samples) => samples.len(),
        AudioSamples::Streaming(_) => 0,
    };
    let frames = sample_count as f32 / f32::from(channels);
    Ok(AudioClip {
        sample_rate,
        channels,
        samples: parsed_samples,
        duration_seconds: frames / sample_rate as f32,
        streaming,
    })
}

fn parse_u32(value: &str, key: &str, line_number: usize) -> Result<u32, AssetError> {
    value.parse().map_err(|error| AssetError::Decode {
        message: format!("invalid {key} on line {line_number}: {error}"),
    })
}

fn parse_u16(value: &str, key: &str, line_number: usize) -> Result<u16, AssetError> {
    value.parse().map_err(|error| AssetError::Decode {
        message: format!("invalid {key} on line {line_number}: {error}"),
    })
}

fn parse_bool(value: &str, line_number: usize) -> Result<bool, AssetError> {
    value.parse().map_err(|error| AssetError::Decode {
        message: format!("invalid streaming flag on line {line_number}: {error}"),
    })
}

fn parse_i16_samples(value: &str, channels: u16) -> Result<Vec<i16>, AssetError> {
    let samples = parse_sample_list(value, "i16", |sample| {
        sample.parse::<i16>().map_err(|error| error.to_string())
    })?;
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_f32_samples(value: &str, channels: u16) -> Result<Vec<f32>, AssetError> {
    let samples = parse_sample_list(value, "f32", parse_f32_sample)?;
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_f32_sample(sample: &str) -> Result<f32, String> {
    let value = sample.parse::<f32>().map_err(|error| error.to_string())?;
    if !value.is_finite() {
        return Err("value must be finite".to_owned());
    }
    Ok(value)
}

fn parse_sample_list<T>(
    value: &str,
    format: &str,
    parse: impl Fn(&str) -> Result<T, String>,
) -> Result<Vec<T>, AssetError> {
    if value.trim().is_empty() {
        return Err(AssetError::Decode {
            message: "audio samples must not be empty".to_owned(),
        });
    }
    value
        .split(',')
        .enumerate()
        .map(|(index, sample)| {
            parse(sample.trim()).map_err(|error| AssetError::Decode {
                message: format!("invalid {format} audio sample {index}: {error}"),
            })
        })
        .collect()
}

fn validate_sample_count(sample_count: usize, channels: u16) -> Result<(), AssetError> {
    let channels = usize::from(channels);
    if sample_count == 0 || sample_count % channels != 0 {
        return Err(AssetError::Decode {
            message: format!(
                "audio sample count {sample_count} must be a non-zero multiple of channels {channels}"
            ),
        });
    }
    Ok(())
}

fn parse_ogg_audio_clip(bytes: &[u8]) -> Result<AudioClip, AssetError> {
    let header_size = 27usize;
    if bytes.len() < header_size {
        return Err(AssetError::Decode {
            message: "OGG source must start with OggS and include a complete page header"
                .to_owned(),
        });
    }
    if &bytes[0..4] != b"OggS" {
        return Err(AssetError::Decode {
            message: "audio source must start with OggS".to_owned(),
        });
    }
    if bytes[4] != 0 {
        return Err(AssetError::Decode {
            message: "unsupported OGG version".to_owned(),
        });
    }

    let segment_count = usize::from(bytes[26]);
    let segment_table_end =
        header_size
            .checked_add(segment_count)
            .ok_or_else(|| AssetError::Decode {
                message: "OGG page segment table length overflow".to_owned(),
            })?;
    if bytes.len() < segment_table_end {
        return Err(AssetError::Decode {
            message: "OGG page segment table exceeds source size".to_owned(),
        });
    }

    let mut packet_len = 0usize;
    let mut has_terminal_segment = false;
    for &segment in bytes[header_size..segment_table_end].iter() {
        let segment_len = usize::from(segment);
        packet_len = packet_len
            .checked_add(segment_len)
            .ok_or_else(|| AssetError::Decode {
                message: "OGG page first packet size overflow".to_owned(),
            })?;
        if segment < 255 {
            has_terminal_segment = true;
            break;
        }
    }
    if !has_terminal_segment {
        return Err(AssetError::Decode {
            message: "OGG first packet spans multiple pages; unsupported minimal parser".to_owned(),
        });
    }
    if packet_len == 0 {
        return Err(AssetError::Decode {
            message: "OGG first packet length must be greater than zero".to_owned(),
        });
    }

    let packet_end =
        segment_table_end
            .checked_add(packet_len)
            .ok_or_else(|| AssetError::Decode {
                message: "OGG first packet length overflow".to_owned(),
            })?;
    if bytes.len() < packet_end {
        return Err(AssetError::Decode {
            message: "OGG page data is truncated".to_owned(),
        });
    }
    let packet = &bytes[segment_table_end..packet_end];

    let (channels, sample_rate) = parse_ogg_audio_packet(packet)?;
    Ok(AudioClip {
        sample_rate,
        channels,
        samples: AudioSamples::Streaming(AudioStreamHandle(stable_hash(bytes))),
        duration_seconds: 0.0,
        streaming: true,
    })
}

fn parse_ogg_audio_packet(packet: &[u8]) -> Result<(u16, u32), AssetError> {
    if packet.len() >= 16 && packet.starts_with(b"OpusHead") {
        let version = packet[8];
        if version != 0 && version != 1 {
            return Err(AssetError::Decode {
                message: format!("unsupported OpusHead version {version}"),
            });
        }
        let channels = u16::from(packet[9]);
        let sample_rate = u32::from_le_bytes([packet[12], packet[13], packet[14], packet[15]]);
        if channels == 0 {
            return Err(AssetError::Decode {
                message: "OGG Opus header has zero channels".to_owned(),
            });
        }
        if sample_rate == 0 {
            return Err(AssetError::Decode {
                message: "OGG Opus header has zero sample rate".to_owned(),
            });
        }
        return Ok((channels, sample_rate));
    }

    if packet.len() >= 16 && packet[0] == 0x01 && packet.get(1..7) == Some(b"vorbis") {
        let channels = u16::from(packet[11]);
        let sample_rate = u32::from_le_bytes([packet[12], packet[13], packet[14], packet[15]]);
        if channels == 0 {
            return Err(AssetError::Decode {
                message: "OGG Vorbis header has zero channels".to_owned(),
            });
        }
        if sample_rate == 0 {
            return Err(AssetError::Decode {
                message: "OGG Vorbis header has zero sample rate".to_owned(),
            });
        }
        return Ok((channels, sample_rate));
    }

    Err(AssetError::Decode {
        message: "audio source is OggS but codec header is unsupported for runtime decode"
            .to_owned(),
    })
}

#[derive(Clone, Debug)]
struct WavFormat {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    samples_per_block: Option<u16>,
    ms_adpcm_coefficients: Vec<[i16; 2]>,
}

const WAV_FORMAT_PCM: u16 = 1;
const WAV_FORMAT_MS_ADPCM: u16 = 2;
const WAV_FORMAT_IEEE_FLOAT: u16 = 3;
const WAV_FORMAT_ALAW: u16 = 6;
const WAV_FORMAT_MULAW: u16 = 7;
const WAV_FORMAT_IMA_ADPCM: u16 = 17;
const WAV_FORMAT_EXTENSIBLE: u16 = 0xfffe;
const WAV_EXTENSIBLE_SUBFORMAT_TAIL: [u8; 12] = [
    0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
const WAV_IMA_ADPCM_INDEX_TABLE: [i8; 16] =
    [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
const WAV_IMA_ADPCM_STEP_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const WAV_MS_ADPCM_ADAPTATION_TABLE: [i32; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230,
];

fn parse_wav_audio_clip(bytes: &[u8]) -> Result<AudioClip, AssetError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AssetError::Decode {
            message: "WAV audio source must start with RIFF/WAVE".to_owned(),
        });
    }

    let mut offset = 12;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = read_wav_u32(bytes, offset + 4, "chunk size")? as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start
            .checked_add(chunk_size)
            .ok_or_else(|| AssetError::Decode {
                message: "WAV chunk size overflow".to_owned(),
            })?;
        if chunk_end > bytes.len() {
            return Err(AssetError::Decode {
                message: format!(
                    "WAV chunk `{}` extends past end of file",
                    String::from_utf8_lossy(chunk_id)
                ),
            });
        }

        match chunk_id {
            b"fmt " => format = Some(parse_wav_format(&bytes[chunk_start..chunk_end])?),
            b"data" => data = Some(&bytes[chunk_start..chunk_end]),
            _ => {}
        }

        offset = chunk_end + usize::from(chunk_size % 2 == 1 && chunk_end < bytes.len());
    }

    let format = format.ok_or_else(|| AssetError::Decode {
        message: "WAV audio source missing fmt chunk".to_owned(),
    })?;
    if format.channels == 0 {
        return Err(AssetError::Decode {
            message: "WAV channels must be greater than zero".to_owned(),
        });
    }
    if format.sample_rate == 0 {
        return Err(AssetError::Decode {
            message: "WAV sample_rate must be greater than zero".to_owned(),
        });
    }
    let data = data.ok_or_else(|| AssetError::Decode {
        message: "WAV audio source missing data chunk".to_owned(),
    })?;

    let samples = match (format.audio_format, format.bits_per_sample) {
        (WAV_FORMAT_PCM, 8) => {
            validate_wav_block_align(&format, 1)?;
            AudioSamples::I16(parse_wav_pcm8_samples(data, format.channels)?)
        }
        (WAV_FORMAT_PCM, 16) => {
            validate_wav_block_align(&format, 2)?;
            AudioSamples::I16(parse_wav_pcm16_samples(data, format.channels)?)
        }
        (WAV_FORMAT_PCM, 24) => {
            validate_wav_block_align(&format, 3)?;
            AudioSamples::I16(parse_wav_pcm24_samples(data, format.channels)?)
        }
        (WAV_FORMAT_PCM, 32) => {
            validate_wav_block_align(&format, 4)?;
            AudioSamples::I16(parse_wav_pcm32_samples(data, format.channels)?)
        }
        (WAV_FORMAT_IEEE_FLOAT, 32) => {
            validate_wav_block_align(&format, 4)?;
            AudioSamples::F32(parse_wav_f32_samples(data, format.channels)?)
        }
        (WAV_FORMAT_ALAW, 8) => {
            validate_wav_block_align(&format, 1)?;
            AudioSamples::I16(parse_wav_g711_samples(
                data,
                format.channels,
                decode_wav_alaw_sample,
            )?)
        }
        (WAV_FORMAT_MULAW, 8) => {
            validate_wav_block_align(&format, 1)?;
            AudioSamples::I16(parse_wav_g711_samples(
                data,
                format.channels,
                decode_wav_mulaw_sample,
            )?)
        }
        (WAV_FORMAT_IMA_ADPCM, 4) => {
            AudioSamples::I16(parse_wav_ima_adpcm_samples(data, &format)?)
        }
        (WAV_FORMAT_MS_ADPCM, 4) => {
            AudioSamples::I16(parse_wav_ms_adpcm_samples(data, &format)?)
        }
        (audio_format, bits_per_sample) => {
            return Err(AssetError::Decode {
                message: format!(
                    "unsupported WAV audio format {audio_format} with {bits_per_sample} bits per sample"
                ),
            })
        }
    };

    let sample_count = match &samples {
        AudioSamples::I16(samples) => samples.len(),
        AudioSamples::F32(samples) => samples.len(),
        AudioSamples::Streaming(_) => 0,
    };
    let frames = sample_count as f32 / f32::from(format.channels);
    Ok(AudioClip {
        sample_rate: format.sample_rate,
        channels: format.channels,
        samples,
        duration_seconds: frames / format.sample_rate as f32,
        streaming: false,
    })
}

fn parse_wav_format(bytes: &[u8]) -> Result<WavFormat, AssetError> {
    if bytes.len() < 16 {
        return Err(AssetError::Decode {
            message: "WAV fmt chunk must be at least 16 bytes".to_owned(),
        });
    }

    let audio_format = read_wav_u16(bytes, 0, "audio format")?;
    let bits_per_sample = read_wav_u16(bytes, 14, "bits per sample")?;
    let audio_format = if audio_format == WAV_FORMAT_EXTENSIBLE {
        parse_wav_extensible_audio_format(bytes, bits_per_sample)?
    } else {
        audio_format
    };
    let (samples_per_block, ms_adpcm_coefficients) = match audio_format {
        WAV_FORMAT_IMA_ADPCM => (
            Some(parse_wav_ima_adpcm_samples_per_block(bytes)?),
            Vec::new(),
        ),
        WAV_FORMAT_MS_ADPCM => {
            let (samples_per_block, coefficients) = parse_wav_ms_adpcm_metadata(bytes)?;
            (Some(samples_per_block), coefficients)
        }
        _ => (None, Vec::new()),
    };

    Ok(WavFormat {
        audio_format,
        channels: read_wav_u16(bytes, 2, "channels")?,
        sample_rate: read_wav_u32(bytes, 4, "sample rate")?,
        block_align: read_wav_u16(bytes, 12, "block align")?,
        bits_per_sample,
        samples_per_block,
        ms_adpcm_coefficients,
    })
}

fn parse_wav_ima_adpcm_samples_per_block(bytes: &[u8]) -> Result<u16, AssetError> {
    if bytes.len() < 20 {
        return Err(AssetError::Decode {
            message: "WAV IMA ADPCM fmt chunk must include samples per block".to_owned(),
        });
    }
    let extension_size = read_wav_u16(bytes, 16, "IMA ADPCM extension size")?;
    if extension_size < 2 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV IMA ADPCM fmt chunk extension size {extension_size} must be at least 2"
            ),
        });
    }
    let samples_per_block = read_wav_u16(bytes, 18, "samples per block")?;
    if samples_per_block == 0 {
        return Err(AssetError::Decode {
            message: "WAV IMA ADPCM samples per block must be greater than zero".to_owned(),
        });
    }
    Ok(samples_per_block)
}

fn parse_wav_ms_adpcm_metadata(bytes: &[u8]) -> Result<(u16, Vec<[i16; 2]>), AssetError> {
    if bytes.len() < 22 {
        return Err(AssetError::Decode {
            message: "WAV MS ADPCM fmt chunk must include samples per block and coefficients"
                .to_owned(),
        });
    }
    let extension_size = read_wav_u16(bytes, 16, "MS ADPCM extension size")?;
    if extension_size < 4 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM fmt chunk extension size {extension_size} must be at least 4"
            ),
        });
    }
    let samples_per_block = read_wav_u16(bytes, 18, "samples per block")?;
    if samples_per_block < 2 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM samples per block {samples_per_block} must be at least 2"
            ),
        });
    }
    let coefficient_count = read_wav_u16(bytes, 20, "MS ADPCM coefficient count")?;
    if coefficient_count == 0 {
        return Err(AssetError::Decode {
            message: "WAV MS ADPCM coefficient count must be greater than zero".to_owned(),
        });
    }
    let coefficient_bytes = usize::from(coefficient_count)
        .checked_mul(4)
        .ok_or_else(|| AssetError::Decode {
            message: "WAV MS ADPCM coefficient table size overflow".to_owned(),
        })?;
    let required_extension_size =
        4usize
            .checked_add(coefficient_bytes)
            .ok_or_else(|| AssetError::Decode {
                message: "WAV MS ADPCM extension size overflow".to_owned(),
            })?;
    if usize::from(extension_size) < required_extension_size {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM fmt chunk extension size {extension_size} is too small for {coefficient_count} coefficient pairs"
            ),
        });
    }
    let table_start = 22;
    let table_end = table_start + coefficient_bytes;
    if bytes.len() < table_end {
        return Err(AssetError::Decode {
            message: "WAV MS ADPCM coefficient table is truncated".to_owned(),
        });
    }
    let mut coefficients = Vec::with_capacity(usize::from(coefficient_count));
    for pair in bytes[table_start..table_end].chunks_exact(4) {
        coefficients.push([
            i16::from_le_bytes([pair[0], pair[1]]),
            i16::from_le_bytes([pair[2], pair[3]]),
        ]);
    }
    Ok((samples_per_block, coefficients))
}

fn parse_wav_extensible_audio_format(
    bytes: &[u8],
    bits_per_sample: u16,
) -> Result<u16, AssetError> {
    if bytes.len() < 40 {
        return Err(AssetError::Decode {
            message: "WAV extensible fmt chunk must be at least 40 bytes".to_owned(),
        });
    }

    let extension_size = read_wav_u16(bytes, 16, "extensible extension size")?;
    if extension_size < 22 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV extensible fmt chunk extension size {extension_size} must be at least 22"
            ),
        });
    }

    let subformat = bytes.get(24..40).ok_or_else(|| AssetError::Decode {
        message: "WAV extensible subformat GUID is truncated".to_owned(),
    })?;
    if subformat[2] != 0 || subformat[3] != 0 || subformat[4..] != WAV_EXTENSIBLE_SUBFORMAT_TAIL {
        return Err(AssetError::Decode {
            message: "unsupported WAV extensible subformat GUID".to_owned(),
        });
    }

    let subformat_tag = u16::from_le_bytes([subformat[0], subformat[1]]);
    match subformat_tag {
        WAV_FORMAT_PCM | WAV_FORMAT_IEEE_FLOAT | WAV_FORMAT_ALAW | WAV_FORMAT_MULAW => {
            let valid_bits_per_sample = read_wav_u16(bytes, 18, "valid bits per sample")?;
            if valid_bits_per_sample != 0 && valid_bits_per_sample > bits_per_sample {
                return Err(AssetError::Decode {
                    message: format!(
                        "WAV valid bits per sample {valid_bits_per_sample} exceeds bits per sample {bits_per_sample}"
                    ),
                });
            }
            Ok(subformat_tag)
        }
        WAV_FORMAT_IMA_ADPCM => Ok(subformat_tag),
        other => Err(AssetError::Decode {
            message: format!("unsupported WAV extensible subformat {other}"),
        }),
    }
}

fn validate_wav_block_align(format: &WavFormat, bytes_per_sample: u16) -> Result<(), AssetError> {
    let expected = format
        .channels
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| AssetError::Decode {
            message: "WAV block_align overflow".to_owned(),
        })?;
    if format.block_align != expected {
        return Err(AssetError::Decode {
            message: format!(
                "WAV block_align {} does not match channels {} and sample size {}",
                format.block_align, format.channels, bytes_per_sample
            ),
        });
    }
    Ok(())
}

fn parse_wav_pcm8_samples(bytes: &[u8], channels: u16) -> Result<Vec<i16>, AssetError> {
    let samples = bytes
        .iter()
        .map(|sample| (i16::from(*sample) - 128) << 8)
        .collect::<Vec<_>>();
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_wav_pcm16_samples(bytes: &[u8], channels: u16) -> Result<Vec<i16>, AssetError> {
    if bytes.len() % 2 != 0 {
        return Err(AssetError::Decode {
            message: "WAV PCM16 data byte length must be divisible by 2".to_owned(),
        });
    }

    let samples = bytes
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
        .collect::<Vec<_>>();
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_wav_pcm24_samples(bytes: &[u8], channels: u16) -> Result<Vec<i16>, AssetError> {
    if bytes.len() % 3 != 0 {
        return Err(AssetError::Decode {
            message: "WAV PCM24 data byte length must be divisible by 3".to_owned(),
        });
    }

    let samples = bytes
        .chunks_exact(3)
        .map(|sample| {
            let signed = i32::from_le_bytes([
                sample[0],
                sample[1],
                sample[2],
                if sample[2] & 0x80 == 0 { 0x00 } else { 0xff },
            ]);
            (signed >> 8) as i16
        })
        .collect::<Vec<_>>();
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_wav_pcm32_samples(bytes: &[u8], channels: u16) -> Result<Vec<i16>, AssetError> {
    if bytes.len() % 4 != 0 {
        return Err(AssetError::Decode {
            message: "WAV PCM32 data byte length must be divisible by 4".to_owned(),
        });
    }

    let samples = bytes
        .chunks_exact(4)
        .map(|sample| {
            let signed = i32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
            (signed >> 16) as i16
        })
        .collect::<Vec<_>>();
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_wav_f32_samples(bytes: &[u8], channels: u16) -> Result<Vec<f32>, AssetError> {
    if bytes.len() % 4 != 0 {
        return Err(AssetError::Decode {
            message: "WAV f32 data byte length must be divisible by 4".to_owned(),
        });
    }

    let mut samples = Vec::with_capacity(bytes.len() / 4);
    for (index, sample) in bytes.chunks_exact(4).enumerate() {
        let value = f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
        if !value.is_finite() {
            return Err(AssetError::Decode {
                message: format!("invalid WAV f32 sample {index}: value must be finite"),
            });
        }
        samples.push(value);
    }
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn parse_wav_g711_samples(
    bytes: &[u8],
    channels: u16,
    decode: impl Fn(u8) -> i16,
) -> Result<Vec<i16>, AssetError> {
    let samples = bytes
        .iter()
        .map(|sample| decode(*sample))
        .collect::<Vec<_>>();
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
}

fn decode_wav_alaw_sample(sample: u8) -> i16 {
    let value = sample ^ 0x55;
    let sign = value & 0x80;
    let exponent = (value & 0x70) >> 4;
    let mantissa = value & 0x0f;
    let magnitude = if exponent == 0 {
        (i16::from(mantissa) << 4) + 8
    } else {
        ((i16::from(mantissa) << 4) + 0x108) << (exponent - 1)
    };
    if sign == 0 {
        -magnitude
    } else {
        magnitude
    }
}

fn decode_wav_mulaw_sample(sample: u8) -> i16 {
    let value = !sample;
    let sign = value & 0x80;
    let exponent = (value & 0x70) >> 4;
    let mantissa = value & 0x0f;
    let magnitude = (((i16::from(mantissa) << 3) + 0x84) << exponent) - 0x84;
    if sign == 0 {
        magnitude
    } else {
        -magnitude
    }
}

fn parse_wav_ima_adpcm_samples(bytes: &[u8], format: &WavFormat) -> Result<Vec<i16>, AssetError> {
    let channels = usize::from(format.channels);
    let block_align = usize::from(format.block_align);
    let samples_per_block =
        usize::from(format.samples_per_block.ok_or_else(|| AssetError::Decode {
            message: "WAV IMA ADPCM fmt chunk missing samples per block".to_owned(),
        })?);
    let header_bytes = channels.checked_mul(4).ok_or_else(|| AssetError::Decode {
        message: "WAV IMA ADPCM block header size overflow".to_owned(),
    })?;
    if block_align <= header_bytes {
        return Err(AssetError::Decode {
            message: format!(
                "WAV IMA ADPCM block_align {block_align} must be greater than channel header bytes {header_bytes}"
            ),
        });
    }
    let chunk_bytes = channels.checked_mul(4).ok_or_else(|| AssetError::Decode {
        message: "WAV IMA ADPCM chunk size overflow".to_owned(),
    })?;
    if (block_align - header_bytes) % chunk_bytes != 0 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV IMA ADPCM block_align {block_align} does not contain whole channel chunks"
            ),
        });
    }
    if bytes.len() % block_align != 0 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV IMA ADPCM data byte length must be divisible by block_align {block_align}"
            ),
        });
    }

    let block_count = bytes.len() / block_align;
    let mut output = Vec::with_capacity(block_count * samples_per_block * channels);
    for (block_index, block) in bytes.chunks_exact(block_align).enumerate() {
        let mut channel_samples = Vec::with_capacity(channels);
        for channel in 0..channels {
            let header = channel * 4;
            let predictor = i16::from_le_bytes([block[header], block[header + 1]]) as i32;
            let step_index = block[header + 2];
            if usize::from(step_index) >= WAV_IMA_ADPCM_STEP_TABLE.len() {
                return Err(AssetError::Decode {
                    message: format!(
                        "WAV IMA ADPCM step index {step_index} for channel {channel} in block {block_index} exceeds 88"
                    ),
                });
            }
            if block[header + 3] != 0 {
                return Err(AssetError::Decode {
                    message: format!(
                        "WAV IMA ADPCM reserved byte for channel {channel} in block {block_index} must be zero"
                    ),
                });
            }
            channel_samples.push(WavImaAdpcmChannel {
                predictor,
                step_index: usize::from(step_index),
                samples: vec![predictor as i16],
            });
        }

        let mut offset = header_bytes;
        while offset < block_align {
            for channel in 0..channels {
                let chunk = &block[offset..offset + 4];
                for byte in chunk {
                    decode_wav_ima_adpcm_nibble(byte & 0x0f, &mut channel_samples[channel]);
                    decode_wav_ima_adpcm_nibble(byte >> 4, &mut channel_samples[channel]);
                }
                offset += 4;
            }
        }

        for (channel, samples) in channel_samples.iter().enumerate() {
            if samples.samples.len() < samples_per_block {
                return Err(AssetError::Decode {
                    message: format!(
                        "WAV IMA ADPCM block {block_index} channel {channel} has {} samples but fmt declares {samples_per_block}",
                        samples.samples.len()
                    ),
                });
            }
        }
        for frame in 0..samples_per_block {
            for samples in &channel_samples {
                output.push(samples.samples[frame]);
            }
        }
    }
    validate_sample_count(output.len(), format.channels)?;
    Ok(output)
}

fn parse_wav_ms_adpcm_samples(bytes: &[u8], format: &WavFormat) -> Result<Vec<i16>, AssetError> {
    let channels = usize::from(format.channels);
    if channels > 2 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM supports only mono or stereo payloads, got {channels} channels"
            ),
        });
    }
    let block_align = usize::from(format.block_align);
    let samples_per_block =
        usize::from(format.samples_per_block.ok_or_else(|| AssetError::Decode {
            message: "WAV MS ADPCM fmt chunk missing samples per block".to_owned(),
        })?);
    let header_bytes = channels.checked_mul(7).ok_or_else(|| AssetError::Decode {
        message: "WAV MS ADPCM block header size overflow".to_owned(),
    })?;
    if block_align < header_bytes {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM block_align {block_align} must be at least channel header bytes {header_bytes}"
            ),
        });
    }
    let payload_nibbles =
        (block_align - header_bytes)
            .checked_mul(2)
            .ok_or_else(|| AssetError::Decode {
                message: "WAV MS ADPCM payload nibble count overflow".to_owned(),
            })?;
    if payload_nibbles % channels != 0 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM block_align {block_align} does not contain whole channel nibbles"
            ),
        });
    }
    let available_samples_per_channel = 2 + payload_nibbles / channels;
    if samples_per_block > available_samples_per_channel {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM samples per block {samples_per_block} exceeds block capacity {available_samples_per_channel}"
            ),
        });
    }
    if bytes.len() % block_align != 0 {
        return Err(AssetError::Decode {
            message: format!(
                "WAV MS ADPCM data byte length must be divisible by block_align {block_align}"
            ),
        });
    }

    let block_count = bytes.len() / block_align;
    let mut output = Vec::with_capacity(block_count * samples_per_block * channels);
    for (block_index, block) in bytes.chunks_exact(block_align).enumerate() {
        let mut channel_samples = Vec::with_capacity(channels);
        for channel in 0..channels {
            let predictor = usize::from(block[channel]);
            let coefficients =
                format
                    .ms_adpcm_coefficients
                    .get(predictor)
                    .ok_or_else(|| AssetError::Decode {
                        message: format!(
                            "WAV MS ADPCM predictor {predictor} for channel {channel} in block {block_index} exceeds coefficient count {}",
                            format.ms_adpcm_coefficients.len()
                        ),
                    })?;
            let delta_offset = channels + channel * 2;
            let delta = i32::from(i16::from_le_bytes([
                block[delta_offset],
                block[delta_offset + 1],
            ]));
            if delta < 16 {
                return Err(AssetError::Decode {
                    message: format!(
                        "WAV MS ADPCM delta {delta} for channel {channel} in block {block_index} must be at least 16"
                    ),
                });
            }
            let sample1_offset = channels + channels * 2 + channel * 2;
            let sample1 = i16::from_le_bytes([block[sample1_offset], block[sample1_offset + 1]]);
            let sample2_offset = channels + channels * 4 + channel * 2;
            let sample2 = i16::from_le_bytes([block[sample2_offset], block[sample2_offset + 1]]);
            channel_samples.push(WavMsAdpcmChannel {
                coefficient_1: i32::from(coefficients[0]),
                coefficient_2: i32::from(coefficients[1]),
                delta,
                sample_1: i32::from(sample1),
                sample_2: i32::from(sample2),
                samples: vec![sample2, sample1],
            });
        }

        let mut offset = header_bytes;
        while offset < block_align {
            let byte = block[offset];
            if channels == 1 {
                decode_wav_ms_adpcm_nibble(byte >> 4, &mut channel_samples[0]);
                decode_wav_ms_adpcm_nibble(byte & 0x0f, &mut channel_samples[0]);
            } else {
                decode_wav_ms_adpcm_nibble(byte >> 4, &mut channel_samples[0]);
                decode_wav_ms_adpcm_nibble(byte & 0x0f, &mut channel_samples[1]);
            }
            offset += 1;
        }

        for (channel, samples) in channel_samples.iter().enumerate() {
            if samples.samples.len() < samples_per_block {
                return Err(AssetError::Decode {
                    message: format!(
                        "WAV MS ADPCM block {block_index} channel {channel} has {} samples but fmt declares {samples_per_block}",
                        samples.samples.len()
                    ),
                });
            }
        }
        for frame in 0..samples_per_block {
            for samples in &channel_samples {
                output.push(samples.samples[frame]);
            }
        }
    }
    validate_sample_count(output.len(), format.channels)?;
    Ok(output)
}

struct WavImaAdpcmChannel {
    predictor: i32,
    step_index: usize,
    samples: Vec<i16>,
}

struct WavMsAdpcmChannel {
    coefficient_1: i32,
    coefficient_2: i32,
    delta: i32,
    sample_1: i32,
    sample_2: i32,
    samples: Vec<i16>,
}

fn decode_wav_ima_adpcm_nibble(code: u8, channel: &mut WavImaAdpcmChannel) {
    let step = i32::from(WAV_IMA_ADPCM_STEP_TABLE[channel.step_index]);
    let mut difference = step >> 3;
    if code & 0x01 != 0 {
        difference += step >> 2;
    }
    if code & 0x02 != 0 {
        difference += step >> 1;
    }
    if code & 0x04 != 0 {
        difference += step;
    }
    if code & 0x08 != 0 {
        channel.predictor -= difference;
    } else {
        channel.predictor += difference;
    }
    channel.predictor = channel
        .predictor
        .clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    let next_index =
        channel.step_index as i32 + i32::from(WAV_IMA_ADPCM_INDEX_TABLE[usize::from(code & 0x0f)]);
    channel.step_index = next_index.clamp(0, (WAV_IMA_ADPCM_STEP_TABLE.len() - 1) as i32) as usize;
    channel.samples.push(channel.predictor as i16);
}

fn decode_wav_ms_adpcm_nibble(code: u8, channel: &mut WavMsAdpcmChannel) {
    let signed_code = if code & 0x08 != 0 {
        i32::from(code) - 16
    } else {
        i32::from(code)
    };
    let predicted = ((channel.sample_1 * channel.coefficient_1
        + channel.sample_2 * channel.coefficient_2)
        >> 8)
        + signed_code * channel.delta;
    let sample = predicted.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    let adaptation = WAV_MS_ADPCM_ADAPTATION_TABLE[usize::from(code & 0x0f)];
    channel.delta = ((adaptation * channel.delta) >> 8).max(16);
    channel.sample_2 = channel.sample_1;
    channel.sample_1 = sample;
    channel.samples.push(sample as i16);
}

fn read_wav_u16(bytes: &[u8], offset: usize, field: &str) -> Result<u16, AssetError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| AssetError::Decode {
            message: format!("WAV {field} is truncated"),
        })?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_wav_u32(bytes: &[u8], offset: usize, field: &str) -> Result<u32, AssetError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| AssetError::Decode {
            message: format!("WAV {field} is truncated"),
        })?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

pub fn canonical_audio_runtime_bytes(bytes: &[u8]) -> Result<Vec<u8>, AssetError> {
    if bytes.starts_with(b"OggS") {
        parse_ogg_audio_clip(bytes)?;
        return Ok(bytes.to_vec());
    }
    let clip = parse_audio_clip(bytes)?;
    encode_audio_clip_runtime_bytes(&clip)
}

pub fn encode_audio_clip_runtime_bytes(audio: &AudioClip) -> Result<Vec<u8>, AssetError> {
    let (sample_format, samples) = match &audio.samples {
        AudioSamples::I16(samples) => ("i16", canonical_audio_i16_samples(samples)),
        AudioSamples::F32(samples) => ("f32", canonical_audio_f32_samples(samples)?),
        AudioSamples::Streaming(_) => {
            return Err(AssetError::Decode {
                message: "audio streaming samples cannot be encoded to runtime bytes".to_owned(),
            })
        }
    };
    Ok(format!(
        "NGA_AUDIO_V1\nsample_rate={}\nchannels={}\nformat={sample_format}\nsamples={samples}\nstreaming={}\n",
        audio.sample_rate,
        audio.channels,
        audio.streaming
    )
    .into_bytes())
}

fn canonical_audio_i16_samples(samples: &[i16]) -> String {
    samples
        .iter()
        .map(|sample| sample.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn canonical_audio_f32_samples(samples: &[f32]) -> Result<String, AssetError> {
    let mut canonical = Vec::with_capacity(samples.len());
    for (index, sample) in samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(AssetError::Decode {
                message: format!("invalid f32 audio sample {index}: value must be finite"),
            });
        }
        canonical.push(if *sample == 0.0 {
            "0".to_owned()
        } else {
            sample.to_string()
        });
    }
    Ok(canonical.join(","))
}
