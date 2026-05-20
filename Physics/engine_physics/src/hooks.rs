use crate::event::ContactPoint;
use crate::id::{BodyId, ColliderId, PhysicsUserData};
use crate::material::PhysicsMaterial;

pub trait PhysicsHooks: Send + Sync + 'static {
    fn filter_collision_pair(&self, _pair: CollisionPairInfo) -> CollisionDecision {
        CollisionDecision::UseDefault
    }

    fn modify_contacts(&self, _context: &mut ContactModificationContext) {}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionPairInfo {
    pub collider_a: ColliderId,
    pub collider_b: ColliderId,
    pub body_a: Option<BodyId>,
    pub body_b: Option<BodyId>,
    pub user_data_a: PhysicsUserData,
    pub user_data_b: PhysicsUserData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionDecision {
    UseDefault,
    DisableCollision,
    DisableSolver,
}

pub struct ContactModificationContext<'a> {
    pub pair: CollisionPairInfo,
    pub contacts: &'a mut [ContactPoint],
    pub material: &'a mut PhysicsMaterial,
}
