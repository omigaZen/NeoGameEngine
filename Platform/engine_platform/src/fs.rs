use std::path::{Path, PathBuf};

use crate::PlatformResult;

pub trait FileSystem {
    fn read(&self, path: &Path) -> PlatformResult<Vec<u8>>;
    fn write(&self, path: &Path, data: &[u8]) -> PlatformResult<()>;

    fn exists(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;

    fn asset_dir(&self) -> Option<PathBuf>;
    fn user_data_dir(&self) -> Option<PathBuf>;
    fn cache_dir(&self) -> Option<PathBuf>;
    fn temp_dir(&self) -> PathBuf;
}
