use crate::id::{BodyId, ColliderId};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollisionFilter {
    pub groups: u32,
    pub mask: u32,
    pub query_groups: u32,
    pub query_mask: u32,
}

impl CollisionFilter {
    pub const ALL: u32 = u32::MAX;

    pub const fn new(groups: u32, mask: u32) -> Self {
        Self {
            groups,
            mask,
            query_groups: groups,
            query_mask: mask,
        }
    }

    pub fn collides_with(self, rhs: Self) -> bool {
        (self.groups & rhs.mask) != 0 && (rhs.groups & self.mask) != 0
    }

    pub fn query_matches(self, filter: QueryFilter) -> bool {
        (self.query_groups & filter.mask) != 0 && (filter.groups & self.query_mask) != 0
    }
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self::new(Self::ALL, Self::ALL)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueryFilter {
    pub groups: u32,
    pub mask: u32,
    pub include_sensors: bool,
    pub include_dynamic: bool,
    pub include_fixed: bool,
    pub include_kinematic: bool,
    pub exclude_body: Option<BodyId>,
    pub exclude_collider: Option<ColliderId>,
    pub max_results: Option<usize>,
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            groups: CollisionFilter::ALL,
            mask: CollisionFilter::ALL,
            include_sensors: false,
            include_dynamic: true,
            include_fixed: true,
            include_kinematic: true,
            exclude_body: None,
            exclude_collider: None,
            max_results: None,
        }
    }
}
