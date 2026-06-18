use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

pub type Mat4 = [[f32; 4]; 4];
const INVERSE_BIND_EPSILON: f32 = 0.001;

#[derive(Clone, Debug, PartialEq)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub inverse_bind_poses: Vec<Mat4>,
}

impl Asset for Skeleton {
    const TYPE_NAME: &'static str = "Skeleton";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_0007);
}

impl AssetMemoryUsage for Skeleton {
    fn cpu_bytes(&self) -> u64 {
        (self.bones.len() * std::mem::size_of::<Bone>()
            + self.inverse_bind_poses.len() * std::mem::size_of::<Mat4>()) as u64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    pub name: String,
    pub parent: Option<u32>,
    pub local_bind_transform: Mat4,
}

pub struct SkeletonLoader;

impl SkeletonLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SkeletonLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for SkeletonLoader {
    fn name(&self) -> &'static str {
        "SkeletonLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["skeleton", "skel"]
    }

    fn asset_type(&self) -> AssetTypeId {
        Skeleton::TYPE_ID
    }

    fn load(
        &self,
        _ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_skeleton(bytes).map(LoadedAsset::new)
    }
}

pub(crate) fn parse_skeleton(bytes: &[u8]) -> Result<Skeleton, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("skeleton source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_SKELETON_V1" {
        return Err(AssetError::Decode {
            message: "skeleton source must start with NGA_SKELETON_V1".to_owned(),
        });
    }

    let mut bones = Vec::new();
    let mut inverse_bind_poses = Vec::new();
    let mut explicit_inverse_bind_lines = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid skeleton line {line_number}"),
            });
        };
        let key = key.trim();
        match skeleton_document_key(key).as_str() {
            "bone" | "joint" => {
                let (bone, inverse_bind_pose, explicit_inverse_bind_line) =
                    parse_bone(value.trim(), line_number, bones.len())?;
                if bones
                    .iter()
                    .any(|existing: &Bone| existing.name == bone.name)
                {
                    return Err(AssetError::Decode {
                        message: format!(
                            "skeleton bone `{}` on line {line_number} duplicates an earlier bone name",
                            bone.name
                        ),
                    });
                }
                bones.push(bone);
                inverse_bind_poses.push(inverse_bind_pose);
                explicit_inverse_bind_lines.push(explicit_inverse_bind_line);
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown skeleton key `{key}` on line {line_number}"),
                })
            }
        }
    }
    if bones.is_empty() {
        return Err(AssetError::Decode {
            message: "skeleton source must contain at least one bone".to_owned(),
        });
    }
    validate_explicit_inverse_bind_poses(
        &bones,
        &inverse_bind_poses,
        &explicit_inverse_bind_lines,
    )?;
    Ok(Skeleton {
        bones,
        inverse_bind_poses,
    })
}

fn parse_bone(
    value: &str,
    line_number: usize,
    bone_count: usize,
) -> Result<(Bone, Mat4, Option<usize>), AssetError> {
    let mut parts = value.split(';').map(str::trim);
    let name = parts.next().unwrap_or("");
    if name.is_empty() {
        return Err(AssetError::Decode {
            message: format!("skeleton bone name is empty on line {line_number}"),
        });
    }
    let mut parent = None;
    let mut local_bind_transform = identity_mat4();
    let mut inverse_bind_pose = identity_mat4();
    let mut explicit_inverse_bind_line = None;
    for part in parts {
        let Some((key, value)) = part.split_once('=') else {
            return Err(AssetError::Decode {
                message: format!("invalid skeleton bone field on line {line_number}"),
            });
        };
        let key = key.trim();
        let value = value.trim();
        match skeleton_document_key(key).as_str() {
            "parent" | "parentindex" | "parentbone" | "parentjoint" => {
                let parent_index = value.parse::<u32>().map_err(|error| AssetError::Decode {
                    message: format!("invalid skeleton bone parent on line {line_number}: {error}"),
                })?;
                if parent_index as usize >= bone_count {
                    return Err(AssetError::Decode {
                        message: format!(
                            "skeleton bone parent {parent_index} on line {line_number} does not reference an earlier bone"
                        ),
                    });
                }
                parent = Some(parent_index);
            }
            "bind" | "localbind" | "localbindtransform" | "bindpose" => {
                local_bind_transform = parse_mat4_field(value, "bind", line_number)?;
            }
            "inversebind" | "inversebindpose" | "invbind" => {
                inverse_bind_pose = parse_mat4_field(value, "inverse_bind", line_number)?;
                explicit_inverse_bind_line = Some(line_number);
            }
            _ => {
                return Err(AssetError::Decode {
                    message: format!("unknown skeleton bone field `{key}` on line {line_number}"),
                })
            }
        }
    }
    Ok((
        Bone {
            name: name.to_owned(),
            parent,
            local_bind_transform,
        },
        inverse_bind_pose,
        explicit_inverse_bind_line,
    ))
}

fn skeleton_document_key(key: &str) -> String {
    key.chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '_' && *character != '-'
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_explicit_inverse_bind_poses(
    bones: &[Bone],
    inverse_bind_poses: &[Mat4],
    explicit_inverse_bind_lines: &[Option<usize>],
) -> Result<(), AssetError> {
    let mut model_bind_poses = Vec::with_capacity(bones.len());
    for (index, bone) in bones.iter().enumerate() {
        let model_bind_pose = if let Some(parent_index) = bone.parent {
            multiply_mat4(
                model_bind_poses
                    .get(parent_index as usize)
                    .expect("skeleton parent indices reference earlier bones"),
                &bone.local_bind_transform,
            )
        } else {
            bone.local_bind_transform
        };
        if let Some(line_number) = explicit_inverse_bind_lines[index] {
            let product = multiply_mat4(&model_bind_pose, &inverse_bind_poses[index]);
            if !mat4_is_identity_approx(&product) {
                return Err(AssetError::Decode {
                    message: format!(
                        "skeleton inverse_bind on line {line_number} does not invert bind pose for bone `{}`",
                        bone.name
                    ),
                });
            }
        }
        model_bind_poses.push(model_bind_pose);
    }
    Ok(())
}

fn multiply_mat4(left: &Mat4, right: &Mat4) -> Mat4 {
    let mut output = [[0.0; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            output[row][column] = left[row][0] * right[0][column]
                + left[row][1] * right[1][column]
                + left[row][2] * right[2][column]
                + left[row][3] * right[3][column];
        }
    }
    output
}

fn mat4_is_identity_approx(matrix: &Mat4) -> bool {
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            let expected = if row_index == column_index { 1.0 } else { 0.0 };
            if (*value - expected).abs() > INVERSE_BIND_EPSILON {
                return false;
            }
        }
    }
    true
}

fn parse_mat4_field(value: &str, field: &str, line_number: usize) -> Result<Mat4, AssetError> {
    let values = value
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let value = part.parse::<f32>().map_err(|error| AssetError::Decode {
                message: format!("invalid skeleton {field} value on line {line_number}: {error}"),
            })?;
            if !value.is_finite() {
                return Err(AssetError::Decode {
                    message: format!("skeleton {field} value must be finite on line {line_number}"),
                });
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != 16 {
        return Err(AssetError::Decode {
            message: format!(
                "skeleton {field} on line {line_number} must contain 16 values, found {}",
                values.len()
            ),
        });
    }
    Ok([
        [values[0], values[1], values[2], values[3]],
        [values[4], values[5], values[6], values[7]],
        [values[8], values[9], values[10], values[11]],
        [values[12], values[13], values[14], values[15]],
    ])
}

fn identity_mat4() -> Mat4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}
