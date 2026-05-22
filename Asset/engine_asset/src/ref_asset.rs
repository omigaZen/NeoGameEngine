use std::marker::PhantomData;

use crate::{
    asset::{Asset, AssetDependencyReference},
    handle::{Handle, HandleStrength, UntypedHandle},
    id::AssetId,
    path::AssetPath,
    server::AssetServer,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRef<T: Asset> {
    pub id: AssetId,
    pub fallback_path: Option<AssetPath>,
    #[cfg_attr(feature = "serde", serde(skip))]
    marker: PhantomData<fn() -> T>,
}

impl<T: Asset> AssetRef<T> {
    pub fn new(id: AssetId) -> Self {
        Self {
            id,
            fallback_path: None,
            marker: PhantomData,
        }
    }

    pub fn with_fallback(id: AssetId, path: AssetPath) -> Self {
        Self {
            id,
            fallback_path: Some(path),
            marker: PhantomData,
        }
    }

    pub fn load(&self, assets: &mut AssetServer) -> Handle<T> {
        assets.load_ref(self)
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn dependency(&self) -> AssetDependencyReference {
        AssetDependencyReference {
            id: self.id,
            asset_type: T::TYPE_ID,
            fallback_path: self.fallback_path.clone(),
        }
    }

    pub fn dependency_handle(&self) -> UntypedHandle {
        UntypedHandle::new(self.id, T::TYPE_ID, HandleStrength::Weak)
    }

    pub fn visit_dependency(&self, visitor: &mut dyn FnMut(AssetDependencyReference)) {
        visitor(self.dependency());
    }
}
