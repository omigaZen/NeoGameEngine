use crate::{
    asset::{Asset, AssetMemoryUsage},
    error::{AssetError, AssetLoadError},
    id::AssetTypeId,
    loader::{AssetLoader, LoadContext, LoadedAsset, LoaderSettings},
};

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsMesh {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
    pub kind: PhysicsMeshKind,
}

impl Asset for PhysicsMesh {
    const TYPE_NAME: &'static str = "PhysicsMesh";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_000b);
}

impl AssetMemoryUsage for PhysicsMesh {
    fn cpu_bytes(&self) -> u64 {
        (self.vertices.len() * std::mem::size_of::<[f32; 3]>()
            + self.indices.len() * std::mem::size_of::<[u32; 3]>()) as u64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsMeshKind {
    TriMesh,
    ConvexHull,
    HeightField,
}

pub struct PhysicsMeshLoader;

impl PhysicsMeshLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PhysicsMeshLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoader for PhysicsMeshLoader {
    fn name(&self) -> &'static str {
        "PhysicsMeshLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["physics", "physicsmesh", "pmesh"]
    }

    fn asset_type(&self) -> AssetTypeId {
        PhysicsMesh::TYPE_ID
    }

    fn load(
        &self,
        _ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        parse_physics_mesh(bytes).map(LoadedAsset::new)
    }
}

pub(crate) fn parse_physics_mesh(bytes: &[u8]) -> Result<PhysicsMesh, AssetError> {
    let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
        message: format!("physics mesh source must be UTF-8: {error}"),
    })?;
    let mut lines = source.lines();
    let header = lines.next().unwrap_or("").trim();
    if header != "NGA_PHYSICS_MESH_V1" {
        return Err(AssetError::Decode {
            message: "physics mesh source must start with NGA_PHYSICS_MESH_V1".to_owned(),
        });
    }

    let mut kind = None;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "kind" => kind = Some(parse_kind(value.trim(), line_number)?),
                other => {
                    return Err(AssetError::Decode {
                        message: format!(
                            "unknown physics mesh key `{other}` on line {line_number}"
                        ),
                    })
                }
            }
            continue;
        }
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => vertices.push(parse_f32_triplet(parts, line_number)?),
            Some("i") => indices.push(parse_u32_triplet(parts, line_number)?),
            Some(other) => {
                return Err(AssetError::Decode {
                    message: format!(
                        "unknown physics mesh directive `{other}` on line {line_number}"
                    ),
                })
            }
            None => {}
        }
    }
    let kind = kind.ok_or_else(|| AssetError::Decode {
        message: "physics mesh source missing kind".to_owned(),
    })?;
    if vertices.is_empty() {
        return Err(AssetError::Decode {
            message: "physics mesh must contain at least one vertex".to_owned(),
        });
    }
    if kind != PhysicsMeshKind::ConvexHull && indices.is_empty() {
        return Err(AssetError::Decode {
            message: "physics mesh must contain at least one triangle".to_owned(),
        });
    }
    for triangle in &indices {
        for index in triangle {
            if *index as usize >= vertices.len() {
                return Err(AssetError::Decode {
                    message: format!(
                        "physics mesh index {index} references missing vertex; vertex count is {}",
                        vertices.len()
                    ),
                });
            }
        }
    }
    Ok(PhysicsMesh {
        vertices,
        indices,
        kind,
    })
}

fn parse_kind(value: &str, line_number: usize) -> Result<PhysicsMeshKind, AssetError> {
    match value {
        "trimesh" | "tri_mesh" => Ok(PhysicsMeshKind::TriMesh),
        "convex" | "convex_hull" => Ok(PhysicsMeshKind::ConvexHull),
        "heightfield" | "height_field" => Ok(PhysicsMeshKind::HeightField),
        other => Err(AssetError::Decode {
            message: format!("unknown physics mesh kind `{other}` on line {line_number}"),
        }),
    }
}

fn parse_f32_triplet<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<[f32; 3], AssetError> {
    let mut values = [0.0; 3];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing physics mesh vertex value on line {line_number}"),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!(
                    "invalid physics mesh vertex value on line {line_number}: {error}"
                ),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many physics mesh vertex values on line {line_number}"),
        });
    }
    Ok(values)
}

fn parse_u32_triplet<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<[u32; 3], AssetError> {
    let mut values = [0; 3];
    for value in &mut values {
        *value = parts
            .next()
            .ok_or_else(|| AssetError::Decode {
                message: format!("missing physics mesh index value on line {line_number}"),
            })?
            .parse()
            .map_err(|error| AssetError::Decode {
                message: format!("invalid physics mesh index value on line {line_number}: {error}"),
            })?;
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many physics mesh index values on line {line_number}"),
        });
    }
    Ok(values)
}
