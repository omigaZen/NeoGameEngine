use crate::collider::Axis3;
use crate::math::{Real, Transform, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsDebugDrawOptions {
    pub draw_bodies: bool,
    pub draw_colliders: bool,
    pub draw_aabbs: bool,
    pub draw_contacts: bool,
    pub draw_contact_normals: bool,
    pub draw_joints: bool,
    pub draw_sleeping: bool,
    pub draw_query_gizmos: bool,
    pub draw_names: bool,
}

impl Default for PhysicsDebugDrawOptions {
    fn default() -> Self {
        Self {
            draw_bodies: true,
            draw_colliders: true,
            draw_aabbs: false,
            draw_contacts: true,
            draw_contact_normals: true,
            draw_joints: true,
            draw_sleeping: true,
            draw_query_gizmos: true,
            draw_names: true,
        }
    }
}

pub trait PhysicsDebugRenderer {
    fn line(&mut self, from: Vec3, to: Vec3, style: DebugLineStyle);
    fn sphere(&mut self, center: Vec3, radius: Real, style: DebugShapeStyle);
    fn cuboid(&mut self, transform: Transform, half_extents: Vec3, style: DebugShapeStyle);
    fn capsule(
        &mut self,
        transform: Transform,
        axis: Axis3,
        half_height: Real,
        radius: Real,
        style: DebugShapeStyle,
    );
    fn text(&mut self, position: Vec3, text: &str);
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugLineStyle {
    pub category: DebugDrawCategory,
    pub thickness: Real,
}

impl DebugLineStyle {
    pub const fn new(category: DebugDrawCategory) -> Self {
        Self {
            category,
            thickness: 1.0,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugShapeStyle {
    pub category: DebugDrawCategory,
    pub wireframe: bool,
}

impl DebugShapeStyle {
    pub const fn new(category: DebugDrawCategory) -> Self {
        Self {
            category,
            wireframe: true,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugDrawCategory {
    DynamicBody,
    FixedBody,
    KinematicBody,
    Sensor,
    Sleeping,
    Contact,
    Joint,
    Query,
}

#[derive(Default)]
pub struct DebugCollector {
    pub lines: Vec<(Vec3, Vec3, DebugLineStyle)>,
    pub spheres: Vec<(Vec3, Real, DebugShapeStyle)>,
    pub cuboids: Vec<(Transform, Vec3, DebugShapeStyle)>,
    pub capsules: Vec<(Transform, Axis3, Real, Real, DebugShapeStyle)>,
    pub texts: Vec<(Vec3, String)>,
}

impl PhysicsDebugRenderer for DebugCollector {
    fn line(&mut self, from: Vec3, to: Vec3, style: DebugLineStyle) {
        self.lines.push((from, to, style));
    }

    fn sphere(&mut self, center: Vec3, radius: Real, style: DebugShapeStyle) {
        self.spheres.push((center, radius, style));
    }

    fn cuboid(&mut self, transform: Transform, half_extents: Vec3, style: DebugShapeStyle) {
        self.cuboids.push((transform, half_extents, style));
    }

    fn capsule(
        &mut self,
        transform: Transform,
        axis: Axis3,
        half_height: Real,
        radius: Real,
        style: DebugShapeStyle,
    ) {
        self.capsules
            .push((transform, axis, half_height, radius, style));
    }

    fn text(&mut self, position: Vec3, text: &str) {
        self.texts.push((position, text.to_owned()));
    }
}
