use crate::id::{BodyId, ColliderId, PhysicsTick};
use crate::math::{Real, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum PhysicsEvent {
    CollisionStarted(CollisionEvent),
    CollisionStopped(CollisionEvent),
    SensorEntered(SensorEvent),
    SensorExited(SensorEvent),
    ContactForce(ContactForceEvent),
    EventDropped(EventDropped),
}

impl PhysicsEvent {
    pub fn tick(&self) -> PhysicsTick {
        match self {
            Self::CollisionStarted(event) | Self::CollisionStopped(event) => event.tick,
            Self::SensorEntered(event) | Self::SensorExited(event) => event.tick,
            Self::ContactForce(event) => event.tick,
            Self::EventDropped(event) => event.tick,
        }
    }

    pub fn collider_key(&self) -> (u64, u64) {
        match self {
            Self::CollisionStarted(event) | Self::CollisionStopped(event) => {
                ordered_pair(event.a, event.b)
            }
            Self::SensorEntered(event) | Self::SensorExited(event) => {
                ordered_pair(event.sensor, event.other)
            }
            Self::ContactForce(event) => ordered_pair(event.a, event.b),
            Self::EventDropped(_) => (u64::MAX, u64::MAX),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionEvent {
    pub tick: PhysicsTick,
    pub a: ColliderId,
    pub b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SensorEvent {
    pub tick: PhysicsTick,
    pub sensor: ColliderId,
    pub other: ColliderId,
    pub sensor_body: Option<BodyId>,
    pub other_body: Option<BodyId>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ContactForceEvent {
    pub tick: PhysicsTick,
    pub a: ColliderId,
    pub b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub total_force: Vec3,
    pub total_force_magnitude: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EventDropped {
    pub tick: PhysicsTick,
    pub dropped: usize,
    pub max_events_per_tick: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventCursor {
    pub(crate) index: usize,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ContactPoint {
    pub position: Vec3,
    pub normal: Vec3,
    pub penetration: Real,
    pub impulse: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ContactManifold {
    pub a: ColliderId,
    pub b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub contacts: Vec<ContactPoint>,
}

pub(crate) fn ordered_pair(a: ColliderId, b: ColliderId) -> (u64, u64) {
    let ar = a.raw();
    let br = b.raw();
    if ar <= br {
        (ar, br)
    } else {
        (br, ar)
    }
}
