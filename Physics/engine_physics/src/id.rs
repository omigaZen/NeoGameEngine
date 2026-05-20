#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicsTick(pub u64);

macro_rules! define_id {
    ($name:ident) => {
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            pub const INVALID: Self = Self(0);

            pub const fn is_valid(self) -> bool {
                self.0 != 0
            }

            pub const fn raw(self) -> u64 {
                self.0
            }

            pub(crate) const fn from_parts(index: u32, generation: u32) -> Self {
                Self(((generation as u64) << 32) | index as u64)
            }

            pub(crate) const fn index(self) -> u32 {
                self.0 as u32
            }

            pub(crate) const fn generation(self) -> u32 {
                (self.0 >> 32) as u32
            }
        }
    };
}

define_id!(BodyId);
define_id!(ColliderId);
define_id!(JointId);
define_id!(CharacterControllerId);
define_id!(PhysicsMeshId);

pub type EntityId = u64;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicsUserData {
    pub entity: Option<EntityId>,
    pub layer_tag: u32,
    pub gameplay_tag: u32,
    pub payload: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct Slot<T> {
    pub generation: u32,
    pub value: Option<T>,
}

impl<T> Slot<T> {
    pub fn occupied(generation: u32, value: T) -> Self {
        Self {
            generation,
            value: Some(value),
        }
    }
}
