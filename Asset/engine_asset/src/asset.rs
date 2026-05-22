use crate::{
    handle::{HandleStrength, UntypedHandle},
    id::{AssetId, AssetTypeId},
    path::AssetPath,
};

pub trait Asset: Send + Sync + 'static {
    const TYPE_NAME: &'static str;
    const TYPE_ID: AssetTypeId;
}

pub trait AssetMemoryUsage {
    fn cpu_bytes(&self) -> u64 {
        0
    }

    fn gpu_bytes(&self) -> u64 {
        0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetDependencyReference {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub fallback_path: Option<AssetPath>,
}

impl AssetDependencyReference {
    pub fn new(id: AssetId, asset_type: AssetTypeId) -> Self {
        Self {
            id,
            asset_type,
            fallback_path: None,
        }
    }

    pub fn with_fallback_path(
        id: AssetId,
        asset_type: AssetTypeId,
        fallback_path: AssetPath,
    ) -> Self {
        Self {
            id,
            asset_type,
            fallback_path: Some(fallback_path),
        }
    }

    pub fn from_handle(handle: UntypedHandle) -> Self {
        Self {
            id: handle.id(),
            asset_type: handle.asset_type(),
            fallback_path: None,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn asset_type(&self) -> AssetTypeId {
        self.asset_type
    }

    pub fn fallback_path(&self) -> Option<&AssetPath> {
        self.fallback_path.as_ref()
    }

    pub fn to_untyped_handle(&self) -> UntypedHandle {
        UntypedHandle::new(self.id, self.asset_type, HandleStrength::Weak)
    }
}

impl From<UntypedHandle> for AssetDependencyReference {
    fn from(value: UntypedHandle) -> Self {
        Self::from_handle(value)
    }
}

pub trait AssetDependencies {
    fn visit_dependencies(&self, visitor: &mut dyn FnMut(AssetDependencyReference));
}
