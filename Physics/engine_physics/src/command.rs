use crate::body::{ForceMode, Velocity};
use crate::error::PhysicsError;
use crate::id::{BodyId, ColliderId, JointId};
use crate::math::{Transform, Vec3};

#[derive(Clone, Debug, PartialEq)]
pub enum PhysicsCommand {
    SetBodyTransform {
        body: BodyId,
        transform: Transform,
        wake_up: bool,
    },
    SetBodyVelocity {
        body: BodyId,
        velocity: Velocity,
        wake_up: bool,
    },
    AddForce {
        body: BodyId,
        force: Vec3,
        mode: ForceMode,
        wake_up: bool,
    },
    DestroyBody {
        body: BodyId,
        recursive: bool,
    },
    DestroyCollider(ColliderId),
    DestroyJoint(JointId),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhysicsCommandBuffer {
    commands: Vec<PhysicsCommand>,
}

impl PhysicsCommandBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: PhysicsCommand) {
        self.commands.push(command);
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = PhysicsCommand> + '_ {
        self.commands.drain(..)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandApplyReport {
    pub applied: usize,
    pub failed: usize,
    pub errors: Vec<PhysicsError>,
}
