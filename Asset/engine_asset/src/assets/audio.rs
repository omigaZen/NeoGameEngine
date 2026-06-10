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
    let samples = parse_sample_list(value, "f32", |sample| {
        sample.parse::<f32>().map_err(|error| error.to_string())
    })?;
    validate_sample_count(samples.len(), channels)?;
    Ok(samples)
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
            message: "OGG source must start with OggS and include a complete page header".to_owned(),
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
    let segment_table_end = header_size
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

    let packet_end = segment_table_end
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
        message: "audio source is OggS but codec header is unsupported for runtime decode".to_owned(),
    })
}

#[derive(Clone, Copy, Debug)]
struct WavFormat {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

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
        (1, 16) => {
            validate_wav_block_align(format, 2)?;
            AudioSamples::I16(parse_wav_i16_samples(data, format.channels)?)
        }
        (3, 32) => {
            validate_wav_block_align(format, 4)?;
            AudioSamples::F32(parse_wav_f32_samples(data, format.channels)?)
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

    Ok(WavFormat {
        audio_format: read_wav_u16(bytes, 0, "audio format")?,
        channels: read_wav_u16(bytes, 2, "channels")?,
        sample_rate: read_wav_u32(bytes, 4, "sample rate")?,
        block_align: read_wav_u16(bytes, 12, "block align")?,
        bits_per_sample: read_wav_u16(bytes, 14, "bits per sample")?,
    })
}

fn validate_wav_block_align(format: WavFormat, bytes_per_sample: u16) -> Result<(), AssetError> {
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

fn parse_wav_i16_samples(bytes: &[u8], channels: u16) -> Result<Vec<i16>, AssetError> {
    if bytes.len() % 2 != 0 {
        return Err(AssetError::Decode {
            message: "WAV i16 data byte length must be divisible by 2".to_owned(),
        });
    }

    let samples = bytes
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
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
        AudioSamples::F32(samples) => ("f32", canonical_audio_f32_samples(samples)),
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

fn canonical_audio_f32_samples(samples: &[f32]) -> String {
    samples
        .iter()
        .map(|sample| {
            if *sample == 0.0 {
                "0".to_owned()
            } else {
                sample.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}
