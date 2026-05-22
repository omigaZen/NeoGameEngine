use std::{
    collections::HashMap,
    fmt,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{
    asset::Asset,
    id::{AssetId, AssetTypeId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleStrength {
    Strong,
    Weak,
}

#[derive(Debug)]
pub(crate) struct HandleInner {
    pub id: AssetId,
    pub asset_type: AssetTypeId,
    pub strength: HandleStrength,
    lifecycle: Option<Arc<HandleLifecycleTracker>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HandleLifecycleCounts {
    pub strong: usize,
    pub weak: usize,
}

#[derive(Debug, Default)]
pub(crate) struct HandleLifecycleTracker {
    counts: Mutex<HashMap<AssetId, HandleLifecycleCounts>>,
}

impl HandleLifecycleTracker {
    pub fn retain(&self, id: AssetId, strength: HandleStrength) {
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let count = counts.entry(id).or_default();
        match strength {
            HandleStrength::Strong => count.strong += 1,
            HandleStrength::Weak => count.weak += 1,
        }
    }

    pub fn release(&self, id: AssetId, strength: HandleStrength) {
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let count = counts.entry(id).or_default();
        match strength {
            HandleStrength::Strong => count.strong = count.strong.saturating_sub(1),
            HandleStrength::Weak => count.weak = count.weak.saturating_sub(1),
        }
    }

    pub fn counts(&self, id: AssetId) -> HandleLifecycleCounts {
        self.counts
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&id)
            .copied()
            .unwrap_or_default()
    }

    pub fn tracked_ids(&self) -> Vec<AssetId> {
        self.counts
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .keys()
            .copied()
            .collect()
    }
}

impl HandleInner {
    fn retain(&self) {
        if let Some(lifecycle) = &self.lifecycle {
            lifecycle.retain(self.id, self.strength);
        }
    }

    fn release(&self) {
        if let Some(lifecycle) = &self.lifecycle {
            lifecycle.release(self.id, self.strength);
        }
    }
}

pub struct Handle<T: Asset> {
    pub(crate) inner: Arc<HandleInner>,
    marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self::from_inner(self.inner.clone())
    }
}

impl<T: Asset> Drop for Handle<T> {
    fn drop(&mut self) {
        self.inner.release();
    }
}

impl<T: Asset> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("id", &self.id())
            .field("asset_type", &self.asset_type())
            .field("strength", &self.strength())
            .finish()
    }
}

impl<T: Asset> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
            && self.asset_type() == other.asset_type()
            && self.strength() == other.strength()
    }
}

impl<T: Asset> Eq for Handle<T> {}

impl<T: Asset> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
        self.asset_type().hash(state);
        self.strength().hash(state);
    }
}

impl<T: Asset> Handle<T> {
    pub fn strong(id: AssetId) -> Self {
        Self::new(id, HandleStrength::Strong)
    }

    pub fn weak(id: AssetId) -> Self {
        Self::new(id, HandleStrength::Weak)
    }

    pub(crate) fn new(id: AssetId, strength: HandleStrength) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id,
            asset_type: T::TYPE_ID,
            strength,
            lifecycle: None,
        }))
    }

    pub(crate) fn new_tracked(
        id: AssetId,
        strength: HandleStrength,
        lifecycle: Arc<HandleLifecycleTracker>,
    ) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id,
            asset_type: T::TYPE_ID,
            strength,
            lifecycle: Some(lifecycle),
        }))
    }

    pub(crate) fn from_inner(inner: Arc<HandleInner>) -> Self {
        inner.retain();
        Self {
            inner,
            marker: PhantomData,
        }
    }

    pub fn id(&self) -> AssetId {
        self.inner.id
    }

    pub fn asset_type(&self) -> AssetTypeId {
        self.inner.asset_type
    }

    pub fn strength(&self) -> HandleStrength {
        self.inner.strength
    }

    pub fn is_strong(&self) -> bool {
        self.strength() == HandleStrength::Strong
    }

    pub fn is_weak(&self) -> bool {
        self.strength() == HandleStrength::Weak
    }

    pub fn clone_weak(&self) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id: self.id(),
            asset_type: self.asset_type(),
            strength: HandleStrength::Weak,
            lifecycle: self.inner.lifecycle.clone(),
        }))
    }

    pub fn clone_strong(&self) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id: self.id(),
            asset_type: self.asset_type(),
            strength: HandleStrength::Strong,
            lifecycle: self.inner.lifecycle.clone(),
        }))
    }

    pub fn untyped(&self) -> UntypedHandle {
        UntypedHandle::from_inner(self.inner.clone())
    }
}

pub struct UntypedHandle {
    pub(crate) inner: Arc<HandleInner>,
}

impl Clone for UntypedHandle {
    fn clone(&self) -> Self {
        Self::from_inner(self.inner.clone())
    }
}

impl Drop for UntypedHandle {
    fn drop(&mut self) {
        self.inner.release();
    }
}

impl fmt::Debug for UntypedHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UntypedHandle")
            .field("id", &self.id())
            .field("asset_type", &self.asset_type())
            .field("strength", &self.strength())
            .finish()
    }
}

impl PartialEq for UntypedHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
            && self.asset_type() == other.asset_type()
            && self.strength() == other.strength()
    }
}

impl Eq for UntypedHandle {}

impl std::hash::Hash for UntypedHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
        self.asset_type().hash(state);
        self.strength().hash(state);
    }
}

impl UntypedHandle {
    pub fn new(id: AssetId, asset_type: AssetTypeId, strength: HandleStrength) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id,
            asset_type,
            strength,
            lifecycle: None,
        }))
    }

    pub(crate) fn new_tracked(
        id: AssetId,
        asset_type: AssetTypeId,
        strength: HandleStrength,
        lifecycle: Arc<HandleLifecycleTracker>,
    ) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id,
            asset_type,
            strength,
            lifecycle: Some(lifecycle),
        }))
    }

    fn from_inner(inner: Arc<HandleInner>) -> Self {
        inner.retain();
        Self { inner }
    }

    pub fn id(&self) -> AssetId {
        self.inner.id
    }

    pub fn asset_type(&self) -> AssetTypeId {
        self.inner.asset_type
    }

    pub fn strength(&self) -> HandleStrength {
        self.inner.strength
    }

    pub fn typed<T: Asset>(&self) -> Option<Handle<T>> {
        (self.asset_type() == T::TYPE_ID).then(|| Handle::from_inner(self.inner.clone()))
    }

    pub fn clone_weak(&self) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id: self.id(),
            asset_type: self.asset_type(),
            strength: HandleStrength::Weak,
            lifecycle: self.inner.lifecycle.clone(),
        }))
    }

    pub fn clone_strong(&self) -> Self {
        Self::from_inner(Arc::new(HandleInner {
            id: self.id(),
            asset_type: self.asset_type(),
            strength: HandleStrength::Strong,
            lifecycle: self.inner.lifecycle.clone(),
        }))
    }
}
