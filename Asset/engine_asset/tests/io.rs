use std::{fs, path::PathBuf};

use engine_asset::prelude::*;

fn test_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/asset_io_tests")
        .join(format!("{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(root: &PathBuf, path: &str, bytes: &[u8]) {
    let mut file = root.clone();
    for part in path.split('/') {
        file.push(part);
    }
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, bytes).unwrap();
}

#[test]
fn filesystem_asset_io_reads_ranges_lists_normalized_paths_and_missing_files() {
    let root = test_dir("filesystem_read_list");
    write_file(&root, "textures/a.texture", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    write_file(&root, "textures/nested/b.texture", &[10, 11, 12]);
    let io = FileSystemAssetIo::new(root);

    assert!(io.exists("textures\\a.texture"));
    assert_eq!(
        io.read_range("textures/a.texture", 2, 4).unwrap(),
        vec![2, 3, 4, 5]
    );
    assert_eq!(
        io.read_range("textures/a.texture", 20, 4).unwrap(),
        Vec::<u8>::new()
    );
    let metadata = io.metadata("textures/a.texture").unwrap();
    assert_eq!(metadata.path, "textures/a.texture");
    assert_eq!(metadata.size, 10);
    assert!(metadata.hash.is_none());

    let mut list = io.list("textures").unwrap();
    list.sort();
    assert_eq!(
        list,
        vec![
            "textures/a.texture".to_owned(),
            "textures/nested/b.texture".to_owned(),
        ]
    );
    let read_error = io.read("textures/missing.texture").unwrap_err();
    assert!(matches!(read_error, AssetIoError::NotFound { .. }));
    assert_eq!(read_error.path(), "textures/missing.texture");
    assert_eq!(read_error.action(), AssetIoAction::Read);

    let range_error = io
        .read_range("textures/missing_range.texture", 0, 4)
        .unwrap_err();
    assert!(matches!(range_error, AssetIoError::NotFound { .. }));
    assert_eq!(range_error.path(), "textures/missing_range.texture");
    assert_eq!(range_error.action(), AssetIoAction::ReadRange);

    let metadata_error = io.metadata("textures/missing_meta.texture").unwrap_err();
    assert!(matches!(metadata_error, AssetIoError::NotFound { .. }));
    assert_eq!(metadata_error.path(), "textures/missing_meta.texture");
    assert_eq!(metadata_error.action(), AssetIoAction::Metadata);
}

#[test]
fn filesystem_asset_io_reports_non_file_read_errors() {
    let root = test_dir("filesystem_non_file");
    fs::create_dir_all(root.join("textures/directory.texture")).unwrap();
    let io = FileSystemAssetIo::new(root);

    assert!(io.exists("textures/directory.texture"));
    let read_error = io.read("textures/directory.texture").unwrap_err();
    assert!(matches!(
        read_error,
        AssetIoError::PermissionDenied { .. } | AssetIoError::ReadFailed { .. }
    ));
    assert_eq!(read_error.path(), "textures/directory.texture");
    assert_eq!(read_error.action(), AssetIoAction::Read);
    assert!(read_error
        .message()
        .is_some_and(|message| !message.is_empty()));
    let read_display = read_error.to_string();
    assert!(read_display.contains("read"));
    assert!(read_display.contains("textures/directory.texture"));

    let list_error = io.list("textures/missing").unwrap_err();
    assert!(matches!(list_error, AssetIoError::ReadFailed { .. }));
    assert_eq!(list_error.path(), "textures/missing");
    assert_eq!(list_error.action(), AssetIoAction::List);
    assert!(list_error
        .message()
        .is_some_and(|message| !message.is_empty()));
}

#[test]
fn composite_asset_io_diagnostics_work_with_filesystem_layers() {
    let patch_root = test_dir("filesystem_composite_patch");
    let base_root = test_dir("filesystem_composite_base");
    write_file(&patch_root, "textures/shared.texture", b"patch");
    write_file(&patch_root, "textures/patch_only.texture", b"patch-only");
    write_file(&base_root, "textures/shared.texture", b"base");
    write_file(&base_root, "textures/base_only.texture", b"base-only");
    let composite = CompositeAssetIo::new()
        .with_named_layer(
            "patch_fs",
            AssetIoLayerKind::FileSystem,
            FileSystemAssetIo::new(patch_root),
        )
        .with_named_layer(
            "base_fs",
            AssetIoLayerKind::FileSystem,
            FileSystemAssetIo::new(base_root),
        );

    let (bytes, resolution) = composite
        .read_with_diagnostics("textures/shared.texture")
        .unwrap();
    assert_eq!(bytes, b"patch");
    assert_eq!(resolution.layer.name, "patch_fs");

    let (metadata, resolution) = composite
        .metadata_with_diagnostics("textures/base_only.texture")
        .unwrap();
    assert_eq!(metadata.size, 9);
    assert_eq!(resolution.layer.name, "base_fs");

    let entries = composite.list_with_diagnostics("textures").unwrap();
    let served_by = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry.layer.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        served_by,
        vec![
            ("textures/base_only.texture", "base_fs"),
            ("textures/patch_only.texture", "patch_fs"),
            ("textures/shared.texture", "patch_fs"),
        ]
    );
}
