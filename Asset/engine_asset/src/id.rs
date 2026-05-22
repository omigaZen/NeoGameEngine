use std::sync::atomic::{AtomicU64, Ordering};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId(u128);

impl AssetId {
    pub const NIL: Self = Self(0);

    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed) as u128;
        Self(0x4e47_4153_5345_5400_0000_0000_0000_0000 | sequence)
    }

    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u128 {
        self.0
    }

    pub const fn is_nil(self) -> bool {
        self.0 == 0
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetTypeId(u128);

impl AssetTypeId {
    pub const NIL: Self = Self(0);

    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u128 {
        self.0
    }

    pub fn of<T: crate::asset::Asset>() -> Self {
        T::TYPE_ID
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetTypeName(pub String);

impl AssetTypeName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for AssetTypeName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for AssetTypeName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ContentHash(pub u64);

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VersionHash(pub u64);
