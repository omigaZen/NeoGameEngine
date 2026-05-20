use crate::body::{BodyDesc, Velocity};
use crate::character::CharacterControllerDesc;
use crate::collider::ColliderDesc;
use crate::config::PhysicsConfig;
use crate::id::{BodyId, CharacterControllerId, ColliderId, JointId, PhysicsMeshId, PhysicsTick};
use crate::joint::JointDesc;
use crate::math::Transform;
use crate::mesh::PhysicsMeshDesc;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsSnapshot {
    pub tick: PhysicsTick,
    pub frame_index: u64,
    pub accumulator: crate::math::Real,
    pub config: PhysicsConfig,
    pub bodies: Vec<BodySnapshot>,
    pub colliders: Vec<ColliderSnapshot>,
    pub joints: Vec<JointSnapshot>,
    pub meshes: Vec<PhysicsMeshSnapshot>,
    pub character_controllers: Vec<CharacterControllerSnapshot>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BodySnapshot {
    pub id: BodyId,
    pub desc: BodyDesc,
    pub transform: Transform,
    pub previous_transform: Transform,
    pub velocity: Velocity,
    pub sleeping: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderSnapshot {
    pub id: ColliderId,
    pub parent: Option<BodyId>,
    pub desc: ColliderDesc,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct JointSnapshot {
    pub id: JointId,
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub desc: JointDesc,
    pub enabled: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsMeshSnapshot {
    pub id: PhysicsMeshId,
    pub desc: PhysicsMeshDesc,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterControllerSnapshot {
    pub id: CharacterControllerId,
    pub desc: CharacterControllerDesc,
}
