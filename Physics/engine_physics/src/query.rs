use crate::collider::ColliderShape;
use crate::filter::QueryFilter;
use crate::id::{BodyId, ColliderId, PhysicsUserData};
use crate::math::{Aabb, Real, Transform, Vec3};
use crate::world::PhysicsWorld;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub max_toi: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RayHit {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub point: Vec3,
    pub normal: Vec3,
    pub toi: Real,
    pub user_data: PhysicsUserData,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeCastInput {
    pub shape: ColliderShape,
    pub transform: Transform,
    pub translation: Vec3,
    pub max_toi: Real,
    pub stop_at_penetration: bool,
    pub target_distance: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeCastHit {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub toi: Real,
    pub point1: Vec3,
    pub point2: Vec3,
    pub normal1: Vec3,
    pub normal2: Vec3,
    pub user_data: PhysicsUserData,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OverlapInput {
    pub shape: ColliderShape,
    pub transform: Transform,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OverlapHit {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub user_data: PhysicsUserData,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointProjection {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub point: Vec3,
    pub is_inside: bool,
    pub distance: Real,
    pub user_data: PhysicsUserData,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsQuerySnapshot {
    pub hits: Vec<OverlapHit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryGizmoKind {
    Ray,
    ShapeCast,
    Overlap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryGizmo {
    pub kind: QueryGizmoKind,
    pub from: Vec3,
    pub to: Vec3,
    pub hit: Option<Vec3>,
}

pub struct PhysicsQuery<'a> {
    world: &'a PhysicsWorld,
}

impl<'a> PhysicsQuery<'a> {
    pub(crate) fn new(world: &'a PhysicsWorld) -> Self {
        Self { world }
    }

    pub fn cast_ray(&self, ray: Ray, filter: QueryFilter) -> Option<RayHit> {
        self.world.cast_ray_internal(ray, filter)
    }

    pub fn cast_ray_all(&self, ray: Ray, filter: QueryFilter, out: &mut Vec<RayHit>) -> usize {
        self.world.cast_ray_all_internal(ray, filter, out)
    }

    pub fn cast_ray_predicate<F>(
        &self,
        ray: Ray,
        filter: QueryFilter,
        predicate: F,
    ) -> Option<RayHit>
    where
        F: Fn(ColliderId, PhysicsUserData) -> bool,
    {
        let mut hits = Vec::new();
        self.cast_ray_all(ray, filter, &mut hits);
        hits.into_iter()
            .find(|hit| predicate(hit.collider, hit.user_data))
    }

    pub fn cast_shape(&self, input: ShapeCastInput, filter: QueryFilter) -> Option<ShapeCastHit> {
        self.world.cast_shape_internal(input, filter)
    }

    pub fn cast_shape_all(
        &self,
        input: ShapeCastInput,
        filter: QueryFilter,
        out: &mut Vec<ShapeCastHit>,
    ) -> usize {
        self.world.cast_shape_all_internal(input, filter, out)
    }

    pub fn overlap_shape(
        &self,
        input: OverlapInput,
        filter: QueryFilter,
        out: &mut Vec<OverlapHit>,
    ) -> usize {
        self.world.overlap_shape_internal(input, filter, out)
    }

    pub fn overlap_aabb(
        &self,
        aabb: Aabb,
        filter: QueryFilter,
        out: &mut Vec<OverlapHit>,
    ) -> usize {
        self.world.overlap_aabb_internal(aabb, filter, out)
    }

    pub fn contains_point(
        &self,
        point: Vec3,
        filter: QueryFilter,
        out: &mut Vec<OverlapHit>,
    ) -> usize {
        self.world.contains_point_internal(point, filter, out)
    }

    pub fn project_point(
        &self,
        point: Vec3,
        max_distance: Real,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<PointProjection> {
        self.world
            .project_point_internal(point, max_distance, solid, filter)
    }
}
