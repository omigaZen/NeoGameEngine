use crate::math::{Real, Transform, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum JointDesc {
    Fixed(FixedJointDesc),
    Ball(BallJointDesc),
    Hinge(HingeJointDesc),
    Prismatic(PrismaticJointDesc),
    Distance(DistanceJointDesc),
    Generic(GenericJointDesc),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointAnchor {
    pub local_anchor_a: Vec3,
    pub local_anchor_b: Vec3,
    pub local_axis_a: Vec3,
    pub local_axis_b: Vec3,
}

impl Default for JointAnchor {
    fn default() -> Self {
        Self {
            local_anchor_a: Vec3::ZERO,
            local_anchor_b: Vec3::ZERO,
            local_axis_a: Vec3::X,
            local_axis_b: Vec3::X,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct FixedJointDesc {
    pub local_frame_a: Transform,
    pub local_frame_b: Transform,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BallJointDesc {
    pub anchors: JointAnchor,
    pub limits: Option<JointLimits>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HingeJointDesc {
    pub anchors: JointAnchor,
    pub limits: Option<JointLimits>,
    pub motor: Option<JointMotor>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PrismaticJointDesc {
    pub anchors: JointAnchor,
    pub limits: Option<JointLimits>,
    pub motor: Option<JointMotor>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DistanceJointDesc {
    pub local_anchor_a: Vec3,
    pub local_anchor_b: Vec3,
    pub min_distance: Real,
    pub max_distance: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct GenericJointDesc {
    pub local_frame_a: Transform,
    pub local_frame_b: Transform,
    pub locked_axes: JointLockedAxes,
    pub limits: Vec<JointAxisLimit>,
    pub motors: Vec<JointAxisMotor>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointLimits {
    pub min: Real,
    pub max: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointMotor {
    pub target_velocity: Real,
    pub target_position: Option<Real>,
    pub stiffness: Real,
    pub damping: Real,
    pub max_force: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JointAxis {
    X,
    Y,
    Z,
    AngularX,
    AngularY,
    AngularZ,
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct JointLockedAxes: u32 {
        const LIN_X = 1 << 0;
        const LIN_Y = 1 << 1;
        const LIN_Z = 1 << 2;
        const ANG_X = 1 << 3;
        const ANG_Y = 1 << 4;
        const ANG_Z = 1 << 5;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointAxisLimit {
    pub axis: JointAxis,
    pub min: Real,
    pub max: Real,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointAxisMotor {
    pub axis: JointAxis,
    pub motor: JointMotor,
}

impl JointDesc {
    pub fn fixed() -> Self {
        Self::Fixed(FixedJointDesc {
            local_frame_a: Transform::IDENTITY,
            local_frame_b: Transform::IDENTITY,
        })
    }

    pub fn distance(min_distance: Real, max_distance: Real) -> Self {
        Self::Distance(DistanceJointDesc {
            local_anchor_a: Vec3::ZERO,
            local_anchor_b: Vec3::ZERO,
            min_distance,
            max_distance,
        })
    }
}
