use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

const QUATERNION_LENGTH_EPSILON: f32 = 0.001;

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClip {
    pub duration: f32,
    pub ticks_per_second: f32,
    pub tracks: Vec<AnimationTrack>,
}

impl Asset for AnimationClip {
    const TYPE_NAME: &'static str = "AnimationClip";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0006);
}

impl AssetMemoryUsage for AnimationClip {
    fn cpu_bytes(&self) -> u64 {
        self.tracks
            .iter()
            .map(|track| {
                ((track.translations.len() * std::mem::size_of::<Keyframe<[f32; 3]>>())
                    + (track.rotations.len() * std::mem::size_of::<Keyframe<[f32; 4]>>())
                    + (track.scales.len() * std::mem::size_of::<Keyframe<[f32; 3]>>()))
                    as u64
            })
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationTrack {
    pub target: AnimationTarget,
    pub translations: Vec<Keyframe<[f32; 3]>>,
    pub rotations: Vec<Keyframe<[f32; 4]>>,
    pub scales: Vec<Keyframe<[f32; 3]>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe<T> {
    pub time: f32,
    pub value: T,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimationTarget {
    NodeName(String),
    NodeIndex(u32),
    BoneName(String),
}

pub struct AnimationLoader;

impl AnimationLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnimationLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for AnimationLoader {
    fn name(&self) -> &'static str {
        "AnimationLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["animation", "anim"]
    }

    fn asset_type(&self) -> AssetTypeId {
        AnimationClip::TYPE_ID
    }

    fn load(
        &self,
        _ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_animation_clip(bytes).map(LoadedAsset::new)
    }
}

pub(crate) fn parse_animation_clip(bytes: &[u8]) -> Result<AnimationClip, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("animation source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_ANIMATION_V1" {
        return Err(AssetError::Decode {
            message: "animation source must start with NGA_ANIMATION_V1".to_owned(),
        });
    }

    let mut duration = None;
    let mut ticks_per_second = None;
    let mut tracks: Vec<AnimationTrack> = Vec::new();
    let mut current_track = None;
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid animation line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match animation_document_key(key).as_str() {
            "duration" | "length" => duration = Some(parse_f32(value, key, line_number)?),
            "tickspersecond" | "tps" | "framerate" | "fps" => {
                ticks_per_second = Some(parse_f32(value, key, line_number)?);
            }
            "track" | "channel" => {
                tracks.push(AnimationTrack {
                    target: parse_animation_target(value, line_number)?,
                    translations: Vec::new(),
                    rotations: Vec::new(),
                    scales: Vec::new(),
                });
                current_track = Some(tracks.len() - 1);
            }
            "translation" | "position" | "location" => {
                let index = current_track_index(current_track, key, line_number)?;
                tracks[index]
                    .translations
                    .push(parse_keyframe_vec3(value, key, line_number)?);
            }
            "rotation" | "quaternion" => {
                let index = current_track_index(current_track, key, line_number)?;
                tracks[index]
                    .rotations
                    .push(parse_keyframe_vec4(value, key, line_number)?);
            }
            "scale" | "scaling" => {
                let index = current_track_index(current_track, key, line_number)?;
                tracks[index]
                    .scales
                    .push(parse_keyframe_vec3(value, key, line_number)?);
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown animation key `{key}` on line {line_number}"),
                })
            }
        }
    }

    let duration = required_positive(duration, "duration")?;
    let ticks_per_second = required_positive(ticks_per_second, "ticks_per_second")?;
    if tracks.is_empty() {
        return Err(AssetError::Decode {
            message: "animation source must contain at least one track".to_owned(),
        });
    }
    validate_animation_tracks(&tracks)?;
    validate_animation_rotation_quaternions(&tracks)?;
    validate_animation_keyframe_times(&tracks, duration)?;
    Ok(AnimationClip {
        duration,
        ticks_per_second,
        tracks,
    })
}

fn parse_animation_target(value: &str, line_number: usize) -> Result<AnimationTarget, AssetError> {
    let Some((kind, target)) = value.split_once(':') else {
        return Err(AssetError::Decode {
            message: format!("invalid animation track target on line {line_number}"),
        });
    };
    let target = target.trim();
    if target.is_empty() {
        return Err(AssetError::Decode {
            message: format!("animation track target is empty on line {line_number}"),
        });
    }
    let kind = kind.trim();
    match animation_document_key(kind).as_str() {
        "node" => Ok(AnimationTarget::NodeName(target.to_owned())),
        "bone" => Ok(AnimationTarget::BoneName(target.to_owned())),
        "nodeindex" => target
            .parse()
            .map(AnimationTarget::NodeIndex)
            .map_err(|error| AssetError::Decode {
                message: format!("invalid animation node_index on line {line_number}: {error}"),
            }),
        _ => Err(AssetError::Decode {
            message: format!("unknown animation track target `{kind}` on line {line_number}"),
        }),
    }
}

fn animation_document_key(key: &str) -> String {
    key.chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '_' && *character != '-'
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn current_track_index(
    current_track: Option<usize>,
    key: &str,
    line_number: usize,
) -> Result<usize, AssetError> {
    current_track.ok_or_else(|| AssetError::Decode {
        message: format!("animation {key} on line {line_number} has no track"),
    })
}

fn parse_keyframe_vec3(
    value: &str,
    key: &str,
    line_number: usize,
) -> Result<Keyframe<[f32; 3]>, AssetError> {
    let (time, value) = parse_keyframe_parts(value, key, line_number)?;
    Ok(Keyframe {
        time,
        value: parse_f32_array::<3>(value, key, line_number)?,
    })
}

fn parse_keyframe_vec4(
    value: &str,
    key: &str,
    line_number: usize,
) -> Result<Keyframe<[f32; 4]>, AssetError> {
    let (time, value) = parse_keyframe_parts(value, key, line_number)?;
    Ok(Keyframe {
        time,
        value: parse_f32_array::<4>(value, key, line_number)?,
    })
}

fn parse_keyframe_parts<'a>(
    value: &'a str,
    key: &str,
    line_number: usize,
) -> Result<(f32, &'a str), AssetError> {
    let Some((time, value)) = value.split_once(':') else {
        return Err(AssetError::Decode {
            message: format!("invalid animation {key} keyframe on line {line_number}"),
        });
    };
    let time = parse_f32(time.trim(), key, line_number)?;
    if time < 0.0 {
        return Err(AssetError::Decode {
            message: format!(
                "animation {key} keyframe time on line {line_number} must be non-negative"
            ),
        });
    }
    Ok((time, value.trim()))
}

fn validate_animation_tracks(tracks: &[AnimationTrack]) -> Result<(), AssetError> {
    for (track_index, track) in tracks.iter().enumerate() {
        if track.translations.is_empty() && track.rotations.is_empty() && track.scales.is_empty() {
            return Err(AssetError::Decode {
                message: format!(
                    "animation track {track_index} must contain at least one translation, rotation, or scale keyframe"
                ),
            });
        }
        if let Some(previous_index) = tracks[..track_index]
            .iter()
            .position(|previous| previous.target == track.target)
        {
            return Err(AssetError::Decode {
                message: format!(
                    "animation track {track_index} duplicates target `{}` from track {previous_index}",
                    animation_target_label(&track.target)
                ),
            });
        }
    }
    Ok(())
}

fn animation_target_label(target: &AnimationTarget) -> String {
    match target {
        AnimationTarget::NodeName(name) => format!("node:{name}"),
        AnimationTarget::NodeIndex(index) => format!("node_index:{index}"),
        AnimationTarget::BoneName(name) => format!("bone:{name}"),
    }
}

fn validate_animation_keyframe_times(
    tracks: &[AnimationTrack],
    duration: f32,
) -> Result<(), AssetError> {
    for (track_index, track) in tracks.iter().enumerate() {
        validate_keyframe_times(track_index, "translation", &track.translations, duration)?;
        validate_keyframe_times(track_index, "rotation", &track.rotations, duration)?;
        validate_keyframe_times(track_index, "scale", &track.scales, duration)?;
    }
    Ok(())
}

fn validate_animation_rotation_quaternions(tracks: &[AnimationTrack]) -> Result<(), AssetError> {
    for (track_index, track) in tracks.iter().enumerate() {
        for (keyframe_index, keyframe) in track.rotations.iter().enumerate() {
            let [x, y, z, w] = keyframe.value;
            let length_squared = x * x + y * y + z * z + w * w;
            if length_squared <= f32::EPSILON {
                return Err(AssetError::Decode {
                    message: format!(
                        "animation rotation keyframe {keyframe_index} in track {track_index} has zero-length quaternion"
                    ),
                });
            }
            if (length_squared - 1.0).abs() > QUATERNION_LENGTH_EPSILON {
                return Err(AssetError::Decode {
                    message: format!(
                        "animation rotation keyframe {keyframe_index} in track {track_index} quaternion length must be normalized"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_keyframe_times<T>(
    track_index: usize,
    channel: &str,
    keyframes: &[Keyframe<T>],
    duration: f32,
) -> Result<(), AssetError> {
    let mut previous_time = None;
    for (keyframe_index, keyframe) in keyframes.iter().enumerate() {
        if keyframe.time > duration {
            return Err(AssetError::Decode {
                message: format!(
                    "animation {channel} keyframe {keyframe_index} in track {track_index} has time {} beyond duration {duration}",
                    keyframe.time
                ),
            });
        }
        if let Some(previous_time) = previous_time {
            if keyframe.time < previous_time {
                return Err(AssetError::Decode {
                    message: format!(
                        "animation {channel} keyframes in track {track_index} must be sorted by time"
                    ),
                });
            }
        }
        previous_time = Some(keyframe.time);
    }
    Ok(())
}

fn parse_f32_array<const N: usize>(
    value: &str,
    key: &str,
    line_number: usize,
) -> Result<[f32; N], AssetError> {
    let values = value
        .split(',')
        .map(str::trim)
        .map(|part| parse_f32(part, key, line_number))
        .collect::<Result<Vec<_>, _>>()?;
    values
        .try_into()
        .map_err(|values: Vec<f32>| AssetError::Decode {
            message: format!(
                "animation {key} on line {line_number} expected {N} values, got {}",
                values.len()
            ),
        })
}

fn required_positive(value: Option<f32>, key: &str) -> Result<f32, AssetError> {
    let value = value.ok_or_else(|| AssetError::Decode {
        message: format!("animation source missing {key}"),
    })?;
    if value <= 0.0 {
        return Err(AssetError::Decode {
            message: format!("animation {key} must be greater than zero"),
        });
    }
    Ok(value)
}

fn parse_f32(value: &str, key: &str, line_number: usize) -> Result<f32, AssetError> {
    let value = value.parse::<f32>().map_err(|error| AssetError::Decode {
        message: format!("invalid animation {key} on line {line_number}: {error}"),
    })?;
    if !value.is_finite() {
        return Err(AssetError::Decode {
            message: format!("animation {key} on line {line_number} must be finite"),
        });
    }
    Ok(value)
}
