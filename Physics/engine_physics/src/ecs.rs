use crate::body::{BodyDesc, BodyKind};
use crate::collider::ColliderDesc;
use crate::id::{BodyId, ColliderId, JointId};
use crate::joint::JointDesc;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RigidBodyComponent {
    pub body: BodyId,
    pub desc: BodyDesc,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderComponent {
    pub collider: ColliderId,
    pub parent: Option<BodyId>,
    pub desc: ColliderDesc,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct JointComponent {
    pub joint: JointId,
    pub body_a: BodyId,
    pub body_b: BodyId,
    pub desc: JointDesc,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsSyncMode {
    PhysicsToTransform,
    TransformToPhysics,
    Disabled,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsSyncComponent {
    pub mode: PhysicsSyncMode,
    pub interpolate: bool,
}

impl PhysicsSyncComponent {
    pub fn for_body_kind(kind: BodyKind) -> Self {
        match kind {
            BodyKind::Dynamic | BodyKind::KinematicVelocity => Self {
                mode: PhysicsSyncMode::PhysicsToTransform,
                interpolate: true,
            },
            BodyKind::Fixed | BodyKind::KinematicPosition => Self {
                mode: PhysicsSyncMode::TransformToPhysics,
                interpolate: false,
            },
        }
    }
}
