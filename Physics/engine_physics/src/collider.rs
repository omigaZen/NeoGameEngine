use crate::filter::CollisionFilter;
pub use crate::id::PhysicsMeshId;
use crate::id::PhysicsUserData;
use crate::material::PhysicsMaterial;
use crate::math::{Real, Transform, Vec3};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct TriMeshFlags: u32 {
        const DOUBLE_SIDED = 1 << 0;
        const FIX_INTERNAL_EDGES = 1 << 1;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CompoundShapePart {
    pub local_transform: Transform,
    pub shape: Box<ColliderShape>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum ColliderShape {
    Sphere {
        radius: Real,
    },
    Cuboid {
        half_extents: Vec3,
    },
    Capsule {
        axis: Axis3,
        half_height: Real,
        radius: Real,
    },
    Cylinder {
        axis: Axis3,
        half_height: Real,
        radius: Real,
    },
    Cone {
        axis: Axis3,
        half_height: Real,
        radius: Real,
    },
    ConvexHull {
        mesh: PhysicsMeshId,
    },
    TriMesh {
        mesh: PhysicsMeshId,
        flags: TriMeshFlags,
    },
    HeightField {
        mesh: PhysicsMeshId,
    },
    Compound {
        parts: Vec<CompoundShapePart>,
    },
}

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct ActiveEvents: u32 {
        const COLLISION_EVENTS = 1 << 0;
        const SENSOR_EVENTS = 1 << 1;
        const CONTACT_FORCE_EVENTS = 1 << 2;
        const ALL = Self::COLLISION_EVENTS.bits()
            | Self::SENSOR_EVENTS.bits()
            | Self::CONTACT_FORCE_EVENTS.bits();
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ColliderDesc {
    pub shape: ColliderShape,
    pub local_transform: Transform,
    pub material: PhysicsMaterial,
    pub density: Real,
    pub filter: CollisionFilter,
    pub sensor: bool,
    pub enabled: bool,
    pub events: ActiveEvents,
    pub contact_skin: Real,
    pub user_data: PhysicsUserData,
    pub debug_name: Option<String>,
}

impl ColliderDesc {
    pub fn sphere(radius: Real) -> Self {
        Self::new(ColliderShape::Sphere { radius })
    }

    pub fn cuboid(half_extents: Vec3) -> Self {
        Self::new(ColliderShape::Cuboid { half_extents })
    }

    pub fn capsule(axis: Axis3, half_height: Real, radius: Real) -> Self {
        Self::new(ColliderShape::Capsule {
            axis,
            half_height,
            radius,
        })
    }

    pub fn cylinder(axis: Axis3, half_height: Real, radius: Real) -> Self {
        Self::new(ColliderShape::Cylinder {
            axis,
            half_height,
            radius,
        })
    }

    pub fn cone(axis: Axis3, half_height: Real, radius: Real) -> Self {
        Self::new(ColliderShape::Cone {
            axis,
            half_height,
            radius,
        })
    }

    pub fn trimesh(mesh: PhysicsMeshId) -> Self {
        Self::new(ColliderShape::TriMesh {
            mesh,
            flags: TriMeshFlags::empty(),
        })
    }

    pub fn convex_hull(mesh: PhysicsMeshId) -> Self {
        Self::new(ColliderShape::ConvexHull { mesh })
    }

    pub fn heightfield(mesh: PhysicsMeshId) -> Self {
        Self::new(ColliderShape::HeightField { mesh })
    }

    pub fn compound(parts: Vec<CompoundShapePart>) -> Self {
        Self::new(ColliderShape::Compound { parts })
    }

    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            local_transform: Transform::IDENTITY,
            material: PhysicsMaterial::default(),
            density: 1.0,
            filter: CollisionFilter::default(),
            sensor: false,
            enabled: true,
            events: ActiveEvents::ALL,
            contact_skin: 0.0,
            user_data: PhysicsUserData::default(),
            debug_name: None,
        }
    }

    pub fn with_local_transform(mut self, transform: Transform) -> Self {
        self.local_transform = transform;
        self
    }

    pub fn with_material(mut self, material: PhysicsMaterial) -> Self {
        self.material = material;
        self
    }

    pub fn with_density(mut self, density: Real) -> Self {
        self.density = density;
        self
    }

    pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_sensor(mut self, sensor: bool) -> Self {
        self.sensor = sensor;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_events(mut self, events: ActiveEvents) -> Self {
        self.events = events;
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
