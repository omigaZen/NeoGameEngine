use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use engine_platform::{FileSystem, PlatformError, PlatformResult};

#[derive(Debug, Default, Clone, Copy)]
pub struct NativeFileSystem;

impl NativeFileSystem {
    pub fn new() -> Self {
        Self
    }

    fn current_child_dir(name: &str) -> Option<PathBuf> {
        let path = env::current_dir().ok()?.join(name);
        path.is_dir().then_some(path)
    }
}

impl FileSystem for NativeFileSystem {
    fn read(&self, path: &Path) -> PlatformResult<Vec<u8>> {
        fs::read(path).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => PlatformError::FileNotFound(path.display().to_string()),
            _ => PlatformError::FileReadFailed(format!("{}: {err}", path.display())),
        })
    }

    fn write(&self, path: &Path, data: &[u8]) -> PlatformResult<()> {
        fs::write(path, data)
            .map_err(|err| PlatformError::FileWriteFailed(format!("{}: {err}", path.display())))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn asset_dir(&self) -> Option<PathBuf> {
        Self::current_child_dir("assets")
    }

    fn user_data_dir(&self) -> Option<PathBuf> {
        Self::current_child_dir("user_data")
    }

    fn cache_dir(&self) -> Option<PathBuf> {
        Self::current_child_dir("cache")
    }

    fn temp_dir(&self) -> PathBuf {
        env::temp_dir()
    }
}
