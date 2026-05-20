use crate::math::{Real, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TriMeshDesc {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexMeshDesc {
    pub points: Vec<Vec3>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HeightFieldDesc {
    pub heights: Vec<Real>,
    pub rows: u32,
    pub cols: u32,
    pub scale: Vec3,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum PhysicsMeshDesc {
    TriMesh(TriMeshDesc),
    Convex(ConvexMeshDesc),
    HeightField(HeightFieldDesc),
}

impl PhysicsMeshDesc {
    pub(crate) fn points(&self) -> Vec<Vec3> {
        match self {
            Self::TriMesh(desc) => desc.vertices.clone(),
            Self::Convex(desc) => desc.points.clone(),
            Self::HeightField(desc) => {
                let mut points = Vec::with_capacity((desc.rows * desc.cols) as usize);
                for row in 0..desc.rows {
                    for col in 0..desc.cols {
                        let index = (row * desc.cols + col) as usize;
                        let height = desc.heights.get(index).copied().unwrap_or(0.0);
                        points.push(Vec3::new(
                            col as Real * desc.scale.x,
                            height * desc.scale.y,
                            row as Real * desc.scale.z,
                        ));
                    }
                }
                points
            }
        }
    }
}
