use crate::id::PhysicsUserData;
use crate::math::{Real, Transform, Vec3};

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct LockedAxes: u32 {
        const TRANSLATION_X = 1 << 0;
        const TRANSLATION_Y = 1 << 1;
        const TRANSLATION_Z = 1 << 2;
        const ROTATION_X    = 1 << 3;
        const ROTATION_Y    = 1 << 4;
        const ROTATION_Z    = 1 << 5;

        const TRANSLATION_ALL = Self::TRANSLATION_X.bits()
            | Self::TRANSLATION_Y.bits()
            | Self::TRANSLATION_Z.bits();
        const ROTATION_ALL = Self::ROTATION_X.bits()
            | Self::ROTATION_Y.bits()
            | Self::ROTATION_Z.bits();
        const ALL = Self::TRANSLATION_ALL.bits() | Self::ROTATION_ALL.bits();
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyKind {
    Dynamic,
    Fixed,
    KinematicPosition,
    KinematicVelocity,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MassDesc {
    Auto,
    Explicit {
        mass: Real,
        center_of_mass: Vec3,
        principal_inertia: Vec3,
    },
    Infinite,
}

impl Default for MassDesc {
    fn default() -> Self {
        Self::Auto
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damping {
    pub linear: Real,
    pub angular: Real,
}

impl Default for Damping {
    fn default() -> Self {
        Self {
            linear: 0.0,
            angular: 0.0,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BodyDesc {
    pub kind: BodyKind,
    pub transform: Transform,
    pub velocity: Velocity,
    pub mass: MassDesc,
    pub damping: Damping,
    pub gravity_scale: Real,
    pub lock_axes: LockedAxes,
    pub ccd_enabled: bool,
    pub can_sleep: bool,
    pub enabled: bool,
    pub user_data: PhysicsUserData,
    pub debug_name: Option<String>,
}

impl BodyDesc {
    pub fn dynamic() -> Self {
        Self::new(BodyKind::Dynamic)
    }

    pub fn fixed() -> Self {
        Self::new(BodyKind::Fixed).with_mass(MassDesc::Infinite)
    }

    pub fn kinematic_position() -> Self {
        Self::new(BodyKind::KinematicPosition).with_mass(MassDesc::Infinite)
    }

    pub fn kinematic_velocity() -> Self {
        Self::new(BodyKind::KinematicVelocity).with_mass(MassDesc::Infinite)
    }

    pub fn new(kind: BodyKind) -> Self {
        Self {
            kind,
            transform: Transform::IDENTITY,
            velocity: Velocity::default(),
            mass: if kind == BodyKind::Dynamic {
                MassDesc::Auto
            } else {
                MassDesc::Infinite
            },
            damping: Damping::default(),
            gravity_scale: 1.0,
            lock_axes: LockedAxes::empty(),
            ccd_enabled: false,
            can_sleep: true,
            enabled: true,
            user_data: PhysicsUserData::default(),
            debug_name: None,
        }
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.transform.translation = translation;
        self
    }

    pub fn with_rotation(mut self, rotation: crate::math::Quat) -> Self {
        self.transform.rotation = rotation;
        self
    }

    pub fn with_velocity(mut self, velocity: Velocity) -> Self {
        self.velocity = velocity;
        self
    }

    pub fn with_linear_velocity(mut self, linear: Vec3) -> Self {
        self.velocity.linear = linear;
        self
    }

    pub fn with_angular_velocity(mut self, angular: Vec3) -> Self {
        self.velocity.angular = angular;
        self
    }

    pub fn with_mass(mut self, mass: MassDesc) -> Self {
        self.mass = mass;
        self
    }

    pub fn with_damping(mut self, damping: Damping) -> Self {
        self.damping = damping;
        self
    }

    pub fn with_gravity_scale(mut self, scale: Real) -> Self {
        self.gravity_scale = scale;
        self
    }

    pub fn with_locked_axes(mut self, locked: LockedAxes) -> Self {
        self.lock_axes = locked;
        self
    }

    pub fn with_ccd(mut self, enabled: bool) -> Self {
        self.ccd_enabled = enabled;
        self
    }

    pub fn with_sleeping(mut self, can_sleep: bool) -> Self {
        self.can_sleep = can_sleep;
        self
    }

    pub fn with_user_data(mut self, user_data: PhysicsUserData) -> Self {
        self.user_data = user_data;
        self
    }

    pub fn with_debug_name(mut self, name: impl Into<String>) -> Self {
        self.debug_name = Some(name.into());
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceMode {
    Force,
    Impulse,
    Acceleration,
    VelocityChange,
}
