use crate::filter::QueryFilter;
use crate::id::{BodyId, CharacterControllerId, ColliderId};
use crate::math::{Real, Transform, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterControllerDesc {
    pub up: Vec3,
    pub offset: Real,
    pub max_slope_angle: Real,
    pub step_height: Real,
    pub snap_to_ground_distance: Real,
    pub enable_slide: bool,
    pub enable_auto_step: bool,
    pub enable_snap_to_ground: bool,
    pub apply_impulses_to_dynamic_bodies: bool,
    pub max_iterations: u32,
}

impl Default for CharacterControllerDesc {
    fn default() -> Self {
        Self {
            up: Vec3::Y,
            offset: 0.02,
            max_slope_angle: 50.0_f32.to_radians(),
            step_height: 0.35,
            snap_to_ground_distance: 0.2,
            enable_slide: true,
            enable_auto_step: true,
            enable_snap_to_ground: true,
            apply_impulses_to_dynamic_bodies: false,
            max_iterations: 4,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterMoveInput {
    pub controller: CharacterControllerId,
    pub body: BodyId,
    pub collider: ColliderId,
    pub desired_translation: Vec3,
    pub dt: Real,
    pub filter: QueryFilter,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterMoveOutput {
    pub requested_translation: Vec3,
    pub corrected_translation: Vec3,
    pub final_transform: Transform,
    pub grounded: bool,
    pub ground_collider: Option<ColliderId>,
    pub ground_body: Option<BodyId>,
    pub ground_normal: Vec3,
    pub hit_wall: bool,
    pub hit_ceiling: bool,
    pub collisions: Vec<CharacterCollision>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterCollision {
    pub collider: ColliderId,
    pub body: Option<BodyId>,
    pub point: Vec3,
    pub normal: Vec3,
    pub translation_remaining: Vec3,
}
