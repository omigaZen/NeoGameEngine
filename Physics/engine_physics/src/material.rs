use crate::math::Real;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombineRule {
    Average,
    Min,
    Max,
    Multiply,
}

impl Default for CombineRule {
    fn default() -> Self {
        Self::Average
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsMaterial {
    pub friction: Real,
    pub restitution: Real,
    pub friction_combine: CombineRule,
    pub restitution_combine: CombineRule,
}

impl Default for PhysicsMaterial {
    fn default() -> Self {
        Self {
            friction: 0.5,
            restitution: 0.0,
            friction_combine: CombineRule::Average,
            restitution_combine: CombineRule::Average,
        }
    }
}

impl PhysicsMaterial {
    pub fn combine_friction(self, rhs: Self) -> Real {
        combine(self.friction_combine, self.friction, rhs.friction)
    }

    pub fn combine_restitution(self, rhs: Self) -> Real {
        combine(self.restitution_combine, self.restitution, rhs.restitution)
    }
}

fn combine(rule: CombineRule, a: Real, b: Real) -> Real {
    match rule {
        CombineRule::Average => (a + b) * 0.5,
        CombineRule::Min => a.min(b),
        CombineRule::Max => a.max(b),
        CombineRule::Multiply => a * b,
    }
}
