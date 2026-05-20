use crate::id::{BodyId, CharacterControllerId, ColliderId, JointId, PhysicsMeshId};

pub type PhysicsResult<T> = Result<T, PhysicsError>;

#[derive(Clone, Debug, thiserror::Error)]
pub enum PhysicsError {
    #[error("body not found: {0:?}")]
    BodyNotFound(BodyId),

    #[error("collider not found: {0:?}")]
    ColliderNotFound(ColliderId),

    #[error("joint not found: {0:?}")]
    JointNotFound(JointId),

    #[error("character controller not found: {0:?}")]
    CharacterControllerNotFound(CharacterControllerId),

    #[error("physics mesh not found: {0:?}")]
    MeshNotFound(PhysicsMeshId),

    #[error("invalid shape: {reason}")]
    InvalidShape { reason: String },

    #[error("invalid transform")]
    InvalidTransform,

    #[error("invalid parent body: {0:?}")]
    InvalidParent(BodyId),

    #[error("object already exists")]
    AlreadyExists,

    #[error("unsupported physics capability: {0}")]
    Unsupported(&'static str),

    #[error("snapshot validation failed: {0}")]
    InvalidSnapshot(String),

    #[error("backend error: {0}")]
    Backend(String),
}
