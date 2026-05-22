use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    id::AssetTypeId,
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
