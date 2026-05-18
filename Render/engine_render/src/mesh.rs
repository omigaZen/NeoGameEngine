use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoredVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub alpha: f32,
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub uv1: [f32; 2],
    pub tangent: [f32; 4],
}

impl ColoredVertex {
    pub const fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            position,
            color,
            alpha: 1.0,
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            uv1: [0.0, 0.0],
            tangent: [1.0, 0.0, 0.0, 1.0],
        }
    }

    pub const fn with_uv(position: [f32; 3], color: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            color,
            alpha: 1.0,
            normal: [0.0, 0.0, 1.0],
            uv,
            uv1: uv,
            tangent: [1.0, 0.0, 0.0, 1.0],
        }
    }

    pub const fn with_normal_uv(
        position: [f32; 3],
        color: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
    ) -> Self {
        Self {
            position,
            color,
            alpha: 1.0,
            normal,
            uv,
            uv1: uv,
            tangent: [1.0, 0.0, 0.0, 1.0],
        }
    }

    pub const fn with_normal_uv_tangent(
        position: [f32; 3],
        color: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        tangent: [f32; 4],
    ) -> Self {
        Self {
            position,
            color,
            alpha: 1.0,
            normal,
            uv,
            uv1: uv,
            tangent,
        }
    }

    pub const fn with_normal_uvs_tangent(
        position: [f32; 3],
        color: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        uv1: [f32; 2],
        tangent: [f32; 4],
    ) -> Self {
        Self {
            position,
            color,
            alpha: 1.0,
            normal,
            uv,
            uv1,
            tangent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl MeshBounds {
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    pub fn from_vertices(vertices: &[ColoredVertex]) -> Option<Self> {
        let first = vertices.first()?;
        let mut min = first.position;
        let mut max = first.position;

        for vertex in vertices.iter().skip(1) {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex.position[axis]);
                max[axis] = max[axis].max(vertex.position[axis]);
            }
        }

        Some(Self { min, max })
    }

    pub fn corners(self) -> [[f32; 3]; 8] {
        let [min_x, min_y, min_z] = self.min;
        let [max_x, max_y, max_z] = self.max;

        [
            [min_x, min_y, min_z],
            [max_x, min_y, min_z],
            [min_x, max_y, min_z],
            [max_x, max_y, min_z],
            [min_x, min_y, max_z],
            [max_x, min_y, max_z],
            [min_x, max_y, max_z],
            [max_x, max_y, max_z],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshLoadError {
    Empty,
    MalformedLine { line: usize, reason: &'static str },
    InvalidNumber { line: usize, value: String },
    InvalidIndex { line: usize, value: String },
    IndexOutOfBounds { line: usize, value: i32, len: usize },
    TooManyVertices { line: usize },
}

impl fmt::Display for MeshLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "OBJ source did not contain any renderable faces"),
            Self::MalformedLine { line, reason } => write!(f, "line {line}: {reason}"),
            Self::InvalidNumber { line, value } => {
                write!(f, "line {line}: invalid number '{value}'")
            }
            Self::InvalidIndex { line, value } => {
                write!(f, "line {line}: invalid index '{value}'")
            }
            Self::IndexOutOfBounds { line, value, len } => {
                write!(f, "line {line}: index {value} is outside 1..={len}")
            }
            Self::TooManyVertices { line } => write!(f, "line {line}: mesh has too many vertices"),
        }
    }
}

impl std::error::Error for MeshLoadError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjMaterialMesh {
    pub material_name: Option<String>,
    pub mesh: Mesh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    vertices: Vec<ColoredVertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new(vertices: impl Into<Vec<ColoredVertex>>) -> Self {
        Self {
            vertices: vertices.into(),
            indices: Vec::new(),
        }
    }

    pub fn with_indices(
        vertices: impl Into<Vec<ColoredVertex>>,
        indices: impl Into<Vec<u32>>,
    ) -> Self {
        Self {
            vertices: vertices.into(),
            indices: indices.into(),
        }
    }

    pub fn colored_triangle() -> Self {
        Self::with_indices(
            [
                ColoredVertex::with_uv([0.0, 0.62, 0.0], [0.96, 0.24, 0.28], [0.5, 0.0]),
                ColoredVertex::with_uv([-0.64, -0.52, 0.0], [0.16, 0.74, 0.52], [0.0, 1.0]),
                ColoredVertex::with_uv([0.64, -0.52, 0.0], [0.24, 0.48, 1.0], [1.0, 1.0]),
            ],
            [0, 1, 2],
        )
        .with_generated_tangents()
    }

    pub fn textured_quad(width: f32, height: f32, color: [f32; 3]) -> Self {
        let half_width = width * 0.5;
        let half_height = height * 0.5;

        Self::with_indices(
            [
                ColoredVertex::with_uv([-half_width, half_height, 0.0], color, [0.0, 0.0]),
                ColoredVertex::with_uv([-half_width, -half_height, 0.0], color, [0.0, 1.0]),
                ColoredVertex::with_uv([half_width, -half_height, 0.0], color, [1.0, 1.0]),
                ColoredVertex::with_uv([half_width, half_height, 0.0], color, [1.0, 0.0]),
            ],
            [0, 1, 2, 0, 2, 3],
        )
        .with_generated_tangents()
    }

    pub fn textured_cube(size: f32, color: [f32; 3]) -> Self {
        let half = size * 0.5;
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        let faces = [
            (
                [0.0, 0.0, 1.0],
                [
                    [-half, half, half],
                    [-half, -half, half],
                    [half, -half, half],
                    [half, half, half],
                ],
            ),
            (
                [0.0, 0.0, -1.0],
                [
                    [half, half, -half],
                    [half, -half, -half],
                    [-half, -half, -half],
                    [-half, half, -half],
                ],
            ),
            (
                [1.0, 0.0, 0.0],
                [
                    [half, half, half],
                    [half, -half, half],
                    [half, -half, -half],
                    [half, half, -half],
                ],
            ),
            (
                [-1.0, 0.0, 0.0],
                [
                    [-half, half, -half],
                    [-half, -half, -half],
                    [-half, -half, half],
                    [-half, half, half],
                ],
            ),
            (
                [0.0, 1.0, 0.0],
                [
                    [-half, half, -half],
                    [-half, half, half],
                    [half, half, half],
                    [half, half, -half],
                ],
            ),
            (
                [0.0, -1.0, 0.0],
                [
                    [-half, -half, half],
                    [-half, -half, -half],
                    [half, -half, -half],
                    [half, -half, half],
                ],
            ),
        ];

        for (normal, positions) in faces {
            let base = vertices.len() as u32;
            vertices.extend([
                ColoredVertex::with_normal_uv(positions[0], color, normal, [0.0, 0.0]),
                ColoredVertex::with_normal_uv(positions[1], color, normal, [0.0, 1.0]),
                ColoredVertex::with_normal_uv(positions[2], color, normal, [1.0, 1.0]),
                ColoredVertex::with_normal_uv(positions[3], color, normal, [1.0, 0.0]),
            ]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        Self::with_indices(vertices, indices).with_generated_tangents()
    }

    pub fn from_obj_str(source: &str) -> Result<Self, MeshLoadError> {
        Self::from_obj_str_with_color(source, [1.0, 1.0, 1.0])
    }

    pub fn from_obj_str_with_color(source: &str, color: [f32; 3]) -> Result<Self, MeshLoadError> {
        let mut positions = Vec::new();
        let mut uvs = Vec::new();
        let mut normals = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.split_once('#').map_or(raw_line, |(line, _)| line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(kind) = parts.next() else {
                continue;
            };

            match kind {
                "v" => positions.push(parse_vec3(parts, line_number)?),
                "vt" => uvs.push(parse_vec2(parts, line_number)?),
                "vn" => normals.push(normalize_or(
                    parse_vec3(parts, line_number)?,
                    [0.0, 0.0, 1.0],
                )),
                "f" => {
                    let face = parts
                        .map(|part| {
                            ObjFaceVertex::parse(
                                part,
                                line_number,
                                positions.len(),
                                uvs.len(),
                                normals.len(),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if face.len() < 3 {
                        return Err(MeshLoadError::MalformedLine {
                            line: line_number,
                            reason: "face must contain at least three vertices",
                        });
                    }

                    for index in 1..face.len() - 1 {
                        let triangle = [face[0], face[index], face[index + 1]];
                        let flat_normal = triangle_normal(
                            positions[triangle[0].position],
                            positions[triangle[1].position],
                            positions[triangle[2].position],
                        );

                        for vertex in triangle {
                            let mesh_index = u32::try_from(vertices.len()).map_err(|_| {
                                MeshLoadError::TooManyVertices { line: line_number }
                            })?;
                            vertices.push(ColoredVertex::with_normal_uv(
                                positions[vertex.position],
                                color,
                                vertex.normal.map_or(flat_normal, |index| normals[index]),
                                vertex.uv.map_or([0.0, 0.0], |index| uvs[index]),
                            ));
                            indices.push(mesh_index);
                        }
                    }
                }
                _ => {}
            }
        }

        if vertices.is_empty() {
            return Err(MeshLoadError::Empty);
        }

        Ok(Self::with_indices(vertices, indices).with_generated_tangents())
    }

    pub fn from_obj_str_by_material(
        source: &str,
        color: [f32; 3],
    ) -> Result<Vec<ObjMaterialMesh>, MeshLoadError> {
        let mut positions = Vec::new();
        let mut uvs = Vec::new();
        let mut normals = Vec::new();
        let mut builders = Vec::new();
        let mut current_material = None;

        for (line_index, raw_line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line.split_once('#').map_or(raw_line, |(line, _)| line);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let Some(kind) = parts.next() else {
                continue;
            };

            match kind {
                "v" => positions.push(parse_vec3(parts, line_number)?),
                "vt" => uvs.push(parse_vec2(parts, line_number)?),
                "vn" => normals.push(normalize_or(
                    parse_vec3(parts, line_number)?,
                    [0.0, 0.0, 1.0],
                )),
                "usemtl" => {
                    let name = parts.collect::<Vec<_>>().join(" ");
                    if name.is_empty() {
                        return Err(MeshLoadError::MalformedLine {
                            line: line_number,
                            reason: "usemtl must include a material name",
                        });
                    }
                    current_material = Some(name);
                }
                "f" => {
                    let face = parts
                        .map(|part| {
                            ObjFaceVertex::parse(
                                part,
                                line_number,
                                positions.len(),
                                uvs.len(),
                                normals.len(),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if face.len() < 3 {
                        return Err(MeshLoadError::MalformedLine {
                            line: line_number,
                            reason: "face must contain at least three vertices",
                        });
                    }

                    let builder = obj_mesh_builder(&mut builders, &current_material);
                    for index in 1..face.len() - 1 {
                        let triangle = [face[0], face[index], face[index + 1]];
                        let flat_normal = triangle_normal(
                            positions[triangle[0].position],
                            positions[triangle[1].position],
                            positions[triangle[2].position],
                        );

                        for vertex in triangle {
                            let mesh_index =
                                u32::try_from(builder.vertices.len()).map_err(|_| {
                                    MeshLoadError::TooManyVertices { line: line_number }
                                })?;
                            builder.vertices.push(ColoredVertex::with_normal_uv(
                                positions[vertex.position],
                                color,
                                vertex.normal.map_or(flat_normal, |index| normals[index]),
                                vertex.uv.map_or([0.0, 0.0], |index| uvs[index]),
                            ));
                            builder.indices.push(mesh_index);
                        }
                    }
                }
                _ => {}
            }
        }

        let meshes = builders
            .into_iter()
            .filter(|builder| !builder.vertices.is_empty())
            .map(|builder| ObjMaterialMesh {
                material_name: builder.material_name,
                mesh: Mesh::with_indices(builder.vertices, builder.indices)
                    .with_generated_tangents(),
            })
            .collect::<Vec<_>>();

        if meshes.is_empty() {
            return Err(MeshLoadError::Empty);
        }

        Ok(meshes)
    }

    pub fn vertices(&self) -> &[ColoredVertex] {
        &self.vertices
    }

    pub fn bounds(&self) -> Option<MeshBounds> {
        MeshBounds::from_vertices(&self.vertices)
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    pub fn is_indexed(&self) -> bool {
        !self.indices.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn generate_normals(&mut self) {
        generate_mesh_normals(&mut self.vertices, &self.indices);
    }

    pub fn with_generated_normals(mut self) -> Self {
        self.generate_normals();
        self
    }

    pub fn generate_tangents(&mut self) {
        generate_mesh_tangents(&mut self.vertices, &self.indices);
    }

    pub fn with_generated_tangents(mut self) -> Self {
        self.generate_tangents();
        self
    }
}

#[derive(Debug, Clone)]
struct ObjMeshBuilder {
    material_name: Option<String>,
    vertices: Vec<ColoredVertex>,
    indices: Vec<u32>,
}

fn obj_mesh_builder<'a>(
    builders: &'a mut Vec<ObjMeshBuilder>,
    material_name: &Option<String>,
) -> &'a mut ObjMeshBuilder {
    if let Some(index) = builders
        .iter()
        .position(|builder| &builder.material_name == material_name)
    {
        return &mut builders[index];
    }

    builders.push(ObjMeshBuilder {
        material_name: material_name.clone(),
        vertices: Vec::new(),
        indices: Vec::new(),
    });
    builders.last_mut().expect("builder was just pushed")
}

#[derive(Debug, Clone, Copy)]
struct ObjFaceVertex {
    position: usize,
    uv: Option<usize>,
    normal: Option<usize>,
}

impl ObjFaceVertex {
    fn parse(
        source: &str,
        line: usize,
        position_len: usize,
        uv_len: usize,
        normal_len: usize,
    ) -> Result<Self, MeshLoadError> {
        let mut parts = source.split('/');
        let position = parts.next().unwrap_or_default();
        let uv = parts.next();
        let normal = parts.next();

        if parts.next().is_some() || position.is_empty() {
            return Err(MeshLoadError::MalformedLine {
                line,
                reason: "face vertex must be v, v/vt, v//vn, or v/vt/vn",
            });
        }

        Ok(Self {
            position: parse_obj_index(position, position_len, line)?,
            uv: parse_optional_obj_index(uv, uv_len, line)?,
            normal: parse_optional_obj_index(normal, normal_len, line)?,
        })
    }
}

fn parse_vec3<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[f32; 3], MeshLoadError> {
    let x = parse_number(required_part(parts.next(), line, "expected x")?, line)?;
    let y = parse_number(required_part(parts.next(), line, "expected y")?, line)?;
    let z = parse_number(required_part(parts.next(), line, "expected z")?, line)?;

    Ok([x, y, z])
}

fn parse_vec2<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[f32; 2], MeshLoadError> {
    let u = parse_number(required_part(parts.next(), line, "expected u")?, line)?;
    let v = parse_number(required_part(parts.next(), line, "expected v")?, line)?;

    Ok([u, v])
}

fn required_part<'a>(
    part: Option<&'a str>,
    line: usize,
    reason: &'static str,
) -> Result<&'a str, MeshLoadError> {
    part.ok_or(MeshLoadError::MalformedLine { line, reason })
}

fn parse_number(source: &str, line: usize) -> Result<f32, MeshLoadError> {
    let value = source
        .parse::<f32>()
        .map_err(|_| MeshLoadError::InvalidNumber {
            line,
            value: source.to_owned(),
        })?;

    if value.is_finite() {
        Ok(value)
    } else {
        Err(MeshLoadError::InvalidNumber {
            line,
            value: source.to_owned(),
        })
    }
}

fn parse_optional_obj_index(
    source: Option<&str>,
    len: usize,
    line: usize,
) -> Result<Option<usize>, MeshLoadError> {
    match source {
        Some(source) if !source.is_empty() => parse_obj_index(source, len, line).map(Some),
        _ => Ok(None),
    }
}

fn parse_obj_index(source: &str, len: usize, line: usize) -> Result<usize, MeshLoadError> {
    let value = source
        .parse::<i32>()
        .map_err(|_| MeshLoadError::InvalidIndex {
            line,
            value: source.to_owned(),
        })?;
    if value == 0 {
        return Err(MeshLoadError::InvalidIndex {
            line,
            value: source.to_owned(),
        });
    }

    let index = if value > 0 {
        value - 1
    } else {
        len as i32 + value
    };

    if index < 0 || index as usize >= len {
        return Err(MeshLoadError::IndexOutOfBounds { line, value, len });
    }

    Ok(index as usize)
}

fn generate_mesh_normals(vertices: &mut [ColoredVertex], indices: &[u32]) {
    if vertices.is_empty() {
        return;
    }

    let mut normals = vec![[0.0; 3]; vertices.len()];
    if indices.is_empty() {
        let mut index = 0;
        while index + 2 < vertices.len() {
            accumulate_triangle_normal(vertices, &mut normals, index, index + 1, index + 2);
            index += 3;
        }
    } else {
        for triangle in indices.chunks(3) {
            if let &[i0, i1, i2] = triangle {
                let Some(i0) = usize::try_from(i0).ok() else {
                    continue;
                };
                let Some(i1) = usize::try_from(i1).ok() else {
                    continue;
                };
                let Some(i2) = usize::try_from(i2).ok() else {
                    continue;
                };
                accumulate_triangle_normal(vertices, &mut normals, i0, i1, i2);
            }
        }
    }

    for (vertex, normal) in vertices.iter_mut().zip(normals) {
        vertex.normal = normalize_or(normal, [0.0, 0.0, 1.0]);
    }
}

fn accumulate_triangle_normal(
    vertices: &[ColoredVertex],
    normals: &mut [[f32; 3]],
    i0: usize,
    i1: usize,
    i2: usize,
) {
    let (Some(v0), Some(v1), Some(v2)) = (vertices.get(i0), vertices.get(i1), vertices.get(i2))
    else {
        return;
    };
    let normal = cross(
        subtract(v1.position, v0.position),
        subtract(v2.position, v0.position),
    );
    if normal == [0.0, 0.0, 0.0] {
        return;
    }

    for index in [i0, i1, i2] {
        normals[index] = add(normals[index], normal);
    }
}

fn generate_mesh_tangents(vertices: &mut [ColoredVertex], indices: &[u32]) {
    if vertices.len() < 3 {
        for vertex in vertices {
            let normal = normalize_or(vertex.normal, [0.0, 0.0, 1.0]);
            let tangent = orthogonal_tangent(normal);
            vertex.normal = normal;
            vertex.tangent = [tangent[0], tangent[1], tangent[2], 1.0];
        }
        return;
    }

    let mut tangents = vec![[0.0; 3]; vertices.len()];
    let mut bitangents = vec![[0.0; 3]; vertices.len()];

    if indices.is_empty() {
        let mut index = 0;
        while index + 2 < vertices.len() {
            accumulate_triangle_tangent(
                vertices,
                &mut tangents,
                &mut bitangents,
                index,
                index + 1,
                index + 2,
            );
            index += 3;
        }
    } else {
        for triangle in indices.chunks(3) {
            if let &[i0, i1, i2] = triangle {
                let Some(i0) = usize::try_from(i0).ok() else {
                    continue;
                };
                let Some(i1) = usize::try_from(i1).ok() else {
                    continue;
                };
                let Some(i2) = usize::try_from(i2).ok() else {
                    continue;
                };
                accumulate_triangle_tangent(vertices, &mut tangents, &mut bitangents, i0, i1, i2);
            }
        }
    }

    for (index, vertex) in vertices.iter_mut().enumerate() {
        let normal = normalize_or(vertex.normal, [0.0, 0.0, 1.0]);
        let tangent = subtract(tangents[index], scale(normal, dot(normal, tangents[index])));
        let tangent = normalize_or(tangent, orthogonal_tangent(normal));
        let handedness = if dot(cross(normal, tangent), bitangents[index]) < 0.0 {
            -1.0
        } else {
            1.0
        };

        vertex.normal = normal;
        vertex.tangent = [tangent[0], tangent[1], tangent[2], handedness];
    }
}

fn accumulate_triangle_tangent(
    vertices: &[ColoredVertex],
    tangents: &mut [[f32; 3]],
    bitangents: &mut [[f32; 3]],
    i0: usize,
    i1: usize,
    i2: usize,
) {
    let (Some(v0), Some(v1), Some(v2)) = (vertices.get(i0), vertices.get(i1), vertices.get(i2))
    else {
        return;
    };

    let edge1 = subtract(v1.position, v0.position);
    let edge2 = subtract(v2.position, v0.position);
    let uv1 = [v1.uv[0] - v0.uv[0], v1.uv[1] - v0.uv[1]];
    let uv2 = [v2.uv[0] - v0.uv[0], v2.uv[1] - v0.uv[1]];
    let denominator = uv1[0] * uv2[1] - uv1[1] * uv2[0];
    if denominator.abs() <= f32::EPSILON {
        return;
    }

    let inverse = 1.0 / denominator;
    let tangent = scale(
        subtract(scale(edge1, uv2[1]), scale(edge2, uv1[1])),
        inverse,
    );
    let bitangent = scale(
        subtract(scale(edge2, uv1[0]), scale(edge1, uv2[0])),
        inverse,
    );

    for index in [i0, i1, i2] {
        tangents[index] = add(tangents[index], tangent);
        bitangents[index] = add(bitangents[index], bitangent);
    }
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    normalize_or(cross(subtract(b, a), subtract(c, a)), [0.0, 0.0, 1.0])
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(value: [f32; 3], scale: f32) -> [f32; 3] {
    [value[0] * scale, value[1] * scale, value[2] * scale]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn orthogonal_tangent(normal: [f32; 3]) -> [f32; 3] {
    let reference = if normal[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    normalize_or(cross(reference, normal), [1.0, 0.0, 0.0])
}

fn normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length_squared = value[0] * value[0] + value[1] * value[1] + value[2] * value[2];
    if length_squared > f32::EPSILON {
        let length = length_squared.sqrt();
        [value[0] / length, value[1] / length, value[2] / length]
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textured_quad_builds_two_indexed_triangles() {
        let mesh = Mesh::textured_quad(2.0, 4.0, [1.0, 1.0, 1.0]);

        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.indices(), &[0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.vertices()[0].position, [-1.0, 2.0, 0.0]);
        assert_eq!(mesh.vertices()[0].uv, [0.0, 0.0]);
        assert_eq!(mesh.vertices()[0].tangent, [1.0, 0.0, 0.0, -1.0]);
        assert_eq!(mesh.vertices()[2].position, [1.0, -2.0, 0.0]);
        assert_eq!(mesh.vertices()[2].uv, [1.0, 1.0]);
    }

    #[test]
    fn textured_cube_builds_indexed_faces_with_normals() {
        let mesh = Mesh::textured_cube(2.0, [1.0, 1.0, 1.0]);

        assert_eq!(mesh.vertex_count(), 24);
        assert_eq!(mesh.index_count(), 36);
        assert_eq!(mesh.vertices()[0].position, [-1.0, 1.0, 1.0]);
        assert_eq!(mesh.vertices()[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(mesh.vertices()[0].tangent, [1.0, 0.0, 0.0, -1.0]);
        assert_eq!(mesh.vertices()[4].normal, [0.0, 0.0, -1.0]);
    }

    #[test]
    fn mesh_bounds_capture_vertex_extents() {
        let mesh = Mesh::textured_cube(2.0, [1.0, 1.0, 1.0]);

        assert_eq!(
            mesh.bounds(),
            Some(MeshBounds::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]))
        );
    }

    #[test]
    fn obj_loader_imports_positions_uvs_and_normals() {
        let mesh = Mesh::from_obj_str(
            "\
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 1/1/1 2/2/1 3/3/1
",
        )
        .unwrap();

        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.indices(), &[0, 1, 2]);
        assert_eq!(mesh.vertices()[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(mesh.vertices()[1].uv, [1.0, 0.0]);
        assert_eq!(mesh.vertices()[1].normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn obj_loader_triangulates_quads_and_generates_flat_normals() {
        let mesh = Mesh::from_obj_str(
            "\
v -1 -1 0
v 1 -1 0
v 1 1 0
v -1 1 0
f 1 2 3 4
",
        )
        .unwrap();

        assert_eq!(mesh.vertex_count(), 6);
        assert_eq!(mesh.indices(), &[0, 1, 2, 3, 4, 5]);
        assert!(mesh
            .vertices()
            .iter()
            .all(|vertex| vertex.normal == [0.0, 0.0, 1.0]));
    }

    #[test]
    fn obj_loader_supports_negative_indices() {
        let mesh = Mesh::from_obj_str(
            "\
v 0 0 0
v 1 0 0
v 0 1 0
f -3 -2 -1
",
        )
        .unwrap();

        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.vertices()[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(mesh.vertices()[2].position, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn obj_loader_rejects_out_of_bounds_indices() {
        let error = Mesh::from_obj_str(
            "\
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 4
",
        )
        .unwrap_err();

        assert_eq!(
            error,
            MeshLoadError::IndexOutOfBounds {
                line: 4,
                value: 4,
                len: 3,
            }
        );
    }

    #[test]
    fn obj_loader_splits_meshes_by_material_name() {
        let meshes = Mesh::from_obj_str_by_material(
            "\
v 0 0 0
v 1 0 0
v 0 1 0
v 1 1 0
usemtl warm
f 1 2 3
usemtl cool
f 2 4 3
usemtl warm
f 1 2 3
",
            [1.0, 1.0, 1.0],
        )
        .unwrap();

        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].material_name.as_deref(), Some("warm"));
        assert_eq!(meshes[0].mesh.vertex_count(), 6);
        assert_eq!(meshes[1].material_name.as_deref(), Some("cool"));
        assert_eq!(meshes[1].mesh.vertex_count(), 3);
    }
}
