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
    pub height_field: Option<PhysicsHeightField>,
}

impl Asset for PhysicsMesh {
    const TYPE_NAME: &'static str = "PhysicsMesh";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_0000_000b);
}

impl AssetMemoryUsage for PhysicsMesh {
    fn cpu_bytes(&self) -> u64 {
        let mesh_bytes = self.vertices.len() * std::mem::size_of::<[f32; 3]>()
            + self.indices.len() * std::mem::size_of::<[u32; 3]>();
        let height_bytes = self.height_field.as_ref().map_or(0, |height_field| {
            height_field.heights.len() * std::mem::size_of::<f32>()
        });
        (mesh_bytes + height_bytes) as u64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsMeshKind {
    TriMesh,
    ConvexHull,
    HeightField,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsHeightField {
    pub heights: Vec<f32>,
    pub rows: u32,
    pub cols: u32,
    pub scale: [f32; 3],
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
    let mut height_rows = None;
    let mut height_cols = None;
    let mut height_scale = None;
    let mut heights = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            match physics_mesh_document_key(key).as_str() {
                "kind" | "type" | "meshkind" | "collisionkind" => {
                    kind = Some(parse_kind(value.trim(), line_number)?)
                }
                "vertex" | "position" | "point" | "v" => {
                    vertices.push(parse_f32_triplet_text(value.trim(), line_number)?)
                }
                "triangle" | "index" | "indices" | "face" | "i" => {
                    indices.push(parse_u32_triplet_text(value.trim(), line_number)?)
                }
                "rows" | "rowcount" => {
                    height_rows = Some(parse_heightfield_dimension(
                        value.trim(),
                        line_number,
                        "rows",
                    )?)
                }
                "cols" | "columns" | "columncount" => {
                    height_cols = Some(parse_heightfield_dimension(
                        value.trim(),
                        line_number,
                        "cols",
                    )?)
                }
                "scale" | "cellscale" => {
                    height_scale = Some(parse_positive_f32_triplet_text(
                        value.trim(),
                        line_number,
                        "heightfield scale",
                    )?)
                }
                "heights" | "samples" | "heightvalues" => heights.extend(parse_f32_list(
                    value.trim(),
                    line_number,
                    "heightfield height",
                )?),
                _ => {
                    return Err(AssetError::Decode {
                        message: format!("unknown physics mesh key `{key}` on line {line_number}"),
                    })
                }
            }
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let directive = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("").trim();
        match Some(directive) {
            Some(directive)
                if matches!(
                    physics_mesh_document_key(directive).as_str(),
                    "v" | "vertex" | "position" | "point"
                ) =>
            {
                vertices.push(parse_f32_triplet_text(value, line_number)?)
            }
            Some(directive)
                if matches!(
                    physics_mesh_document_key(directive).as_str(),
                    "i" | "triangle" | "index" | "indices" | "face"
                ) =>
            {
                indices.push(parse_u32_triplet_text(value, line_number)?)
            }
            Some(directive)
                if matches!(
                    physics_mesh_document_key(directive).as_str(),
                    "heights" | "samples" | "heightvalues"
                ) =>
            {
                heights.extend(parse_f32_list(value, line_number, "heightfield height")?)
            }
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
    let height_field = if kind == PhysicsMeshKind::HeightField {
        if !vertices.is_empty() || !indices.is_empty() {
            return Err(AssetError::Decode {
                message: "heightfield physics mesh cannot contain vertices or triangles".to_owned(),
            });
        }
        let rows = height_rows.ok_or_else(|| AssetError::Decode {
            message: "heightfield physics mesh missing rows".to_owned(),
        })?;
        let cols = height_cols.ok_or_else(|| AssetError::Decode {
            message: "heightfield physics mesh missing cols".to_owned(),
        })?;
        let scale = height_scale.ok_or_else(|| AssetError::Decode {
            message: "heightfield physics mesh missing scale".to_owned(),
        })?;
        let expected =
            (rows as usize)
                .checked_mul(cols as usize)
                .ok_or_else(|| AssetError::Decode {
                    message: "heightfield physics mesh dimensions overflow".to_owned(),
                })?;
        if heights.len() != expected {
            return Err(AssetError::Decode {
                message: format!(
                    "heightfield physics mesh has {} heights, expected {expected} for {rows}x{cols}",
                    heights.len()
                ),
            });
        }
        Some(PhysicsHeightField {
            heights,
            rows,
            cols,
            scale,
        })
    } else {
        if height_rows.is_some()
            || height_cols.is_some()
            || height_scale.is_some()
            || !heights.is_empty()
        {
            return Err(AssetError::Decode {
                message: "non-heightfield physics mesh cannot contain heightfield metadata"
                    .to_owned(),
            });
        }
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
        if kind == PhysicsMeshKind::ConvexHull {
            validate_convex_hull_vertices(&vertices).map_err(|message| AssetError::Decode {
                message: message.to_owned(),
            })?;
        }
        None
    };
    Ok(PhysicsMesh {
        vertices,
        indices,
        kind,
        height_field,
    })
}

pub(crate) fn validate_convex_hull_vertices(vertices: &[[f32; 3]]) -> Result<(), &'static str> {
    if vertices.len() < 4 {
        return Err("convex physics mesh must contain at least four vertices");
    }

    let mut unique = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        if !unique.contains(vertex) {
            unique.push(*vertex);
        }
    }
    if unique.len() < 4 {
        return Err("convex physics mesh must contain at least four unique vertices");
    }

    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in &unique {
        for axis in 0..3 {
            let value = vertex[axis] as f64;
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    let scale = (0..3)
        .map(|axis| max[axis] - min[axis])
        .fold(0.0_f64, f64::max);
    let line_tolerance = scale.powi(2) * 1.0e-12;
    let area_tolerance = scale.powi(4) * 1.0e-12;
    let volume_tolerance = scale.powi(3) * 1.0e-9;

    let origin = to_f64_point(unique[0]);
    let Some(line_point) = unique
        .iter()
        .skip(1)
        .map(|vertex| to_f64_point(*vertex))
        .find(|vertex| length_squared(subtract(*vertex, origin)) > line_tolerance)
    else {
        return Err("convex physics mesh vertices must span a line");
    };
    let line = subtract(line_point, origin);
    let Some(plane_point) = unique
        .iter()
        .skip(1)
        .map(|vertex| to_f64_point(*vertex))
        .find(|vertex| length_squared(cross(line, subtract(*vertex, origin))) > area_tolerance)
    else {
        return Err("convex physics mesh vertices are collinear");
    };
    let normal = cross(line, subtract(plane_point, origin));
    if unique
        .iter()
        .skip(1)
        .map(|vertex| to_f64_point(*vertex))
        .all(|vertex| dot(normal, subtract(vertex, origin)).abs() <= volume_tolerance)
    {
        return Err("convex physics mesh vertices are coplanar");
    }

    Ok(())
}

fn to_f64_point(vertex: [f32; 3]) -> [f64; 3] {
    [vertex[0] as f64, vertex[1] as f64, vertex[2] as f64]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn length_squared(value: [f64; 3]) -> f64 {
    dot(value, value)
}

fn parse_kind(value: &str, line_number: usize) -> Result<PhysicsMeshKind, AssetError> {
    match physics_mesh_document_key(value).as_str() {
        "trimesh" | "trianglemesh" => Ok(PhysicsMeshKind::TriMesh),
        "convex" | "convexhull" => Ok(PhysicsMeshKind::ConvexHull),
        "heightfield" => Ok(PhysicsMeshKind::HeightField),
        _ => Err(AssetError::Decode {
            message: format!("unknown physics mesh kind `{value}` on line {line_number}"),
        }),
    }
}

pub(crate) fn physics_mesh_document_key(key: &str) -> String {
    key.chars()
        .filter(|character| {
            !character.is_ascii_whitespace() && *character != '_' && *character != '-'
        })
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_f32_triplet_text(value: &str, line_number: usize) -> Result<[f32; 3], AssetError> {
    let normalized = value.replace(',', " ");
    parse_f32_triplet(normalized.split_whitespace(), line_number)
}

fn parse_positive_f32_triplet_text(
    value: &str,
    line_number: usize,
    field: &str,
) -> Result<[f32; 3], AssetError> {
    let values = parse_f32_triplet_text(value, line_number)?;
    if values.iter().any(|value| *value <= 0.0) {
        return Err(AssetError::Decode {
            message: format!("{field} values must be positive on line {line_number}"),
        });
    }
    Ok(values)
}

fn parse_f32_list(value: &str, line_number: usize, field: &str) -> Result<Vec<f32>, AssetError> {
    let normalized = value.replace(',', " ");
    let values = normalized
        .split_whitespace()
        .map(|part| {
            let value = part.parse::<f32>().map_err(|error| AssetError::Decode {
                message: format!("invalid {field} value on line {line_number}: {error}"),
            })?;
            if !value.is_finite() {
                return Err(AssetError::Decode {
                    message: format!("{field} value must be finite on line {line_number}"),
                });
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.is_empty() {
        return Err(AssetError::Decode {
            message: format!("{field} list is empty on line {line_number}"),
        });
    }
    Ok(values)
}

fn parse_heightfield_dimension(
    value: &str,
    line_number: usize,
    field: &str,
) -> Result<u32, AssetError> {
    let value = value.parse::<u32>().map_err(|error| AssetError::Decode {
        message: format!("invalid heightfield {field} on line {line_number}: {error}"),
    })?;
    if value < 2 {
        return Err(AssetError::Decode {
            message: format!("heightfield {field} must be at least 2 on line {line_number}"),
        });
    }
    Ok(value)
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
            .parse::<f32>()
            .map_err(|error| AssetError::Decode {
                message: format!(
                    "invalid physics mesh vertex value on line {line_number}: {error}"
                ),
            })?;
        if !(*value).is_finite() {
            return Err(AssetError::Decode {
                message: format!("physics mesh vertex value must be finite on line {line_number}"),
            });
        }
    }
    if parts.next().is_some() {
        return Err(AssetError::Decode {
            message: format!("too many physics mesh vertex values on line {line_number}"),
        });
    }
    Ok(values)
}

fn parse_u32_triplet_text(value: &str, line_number: usize) -> Result<[u32; 3], AssetError> {
    let normalized = value.replace(',', " ");
    parse_u32_triplet(normalized.split_whitespace(), line_number)
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
