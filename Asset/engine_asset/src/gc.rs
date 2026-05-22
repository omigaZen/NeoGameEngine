pub use crate::config::{AssetGcConfig, AssetTypeMemoryBudget};
pub use crate::server::{
    AssetMemoryInfo, AssetMemoryReport, AssetMemoryStats, AssetTypeMemoryReport,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetGarbageCollectionReport {
    pub unloaded_assets: usize,
    pub freed_cpu_bytes: u64,
    pub freed_gpu_bytes: u64,
}
