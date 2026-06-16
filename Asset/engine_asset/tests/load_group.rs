use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn io_source_hash(io: &MemoryAssetIo, path: &str) -> ContentHash {
    io.metadata(path).unwrap().hash.unwrap()
}

fn server_with_textures(paths: &[(&str, u8)]) -> AssetServer {
    let mut io = MemoryAssetIo::new();
    for (path, value) in paths {
        io.insert(*path, texture_bytes(1, 1, *value));
    }
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

fn finish_uploads(server: &mut AssetServer) {
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(1))),
    );
}

#[test]
fn group_progress_reports_mixed_ready_failed_cancelled_and_queued_states() {
    let mut server = server_with_textures(&[
        ("textures/cancel.texture", 1),
        ("textures/ready.texture", 2),
        ("textures/queued.texture", 3),
    ]);
    let group = server.load_group(&[
        AssetPath::parse("textures/cancel.texture"),
        AssetPath::parse("textures/missing.texture"),
        AssetPath::parse("textures/ready.texture"),
        AssetPath::parse("textures/queued.texture"),
    ]);

    assert!(server.cancel_load_by_id(group.assets[0].id()));
    server.update_loading();
    assert_eq!(
        server.state_by_id(group.assets[1].id()),
        AssetLoadState::Failed
    );
    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(
        server.state_by_id(group.assets[2].id()),
        AssetLoadState::Ready
    );
    assert_eq!(
        server.state_by_id(group.assets[3].id()),
        AssetLoadState::Queued
    );

    let progress = server.group_progress(&group);
    assert_eq!(progress.total_assets, 4);
    assert_eq!(progress.queued_assets, 1);
    assert_eq!(progress.loading_assets, 0);
    assert_eq!(progress.ready_assets, 1);
    assert_eq!(progress.failed_assets, 1);
    assert_eq!(progress.cancelled_assets, 1);
    assert_eq!(progress.bytes_loaded, 8);
    assert_eq!(progress.bytes_total, 8);
    assert_eq!(server.group_state(&group), AssetLoadState::Failed);
}

#[test]
fn group_progress_tracks_loading_and_ready_memory_bytes() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/a.texture", texture_bytes(1, 1, 4));
    io.insert("textures/b.texture", texture_bytes(1, 1, 5));
    let a_source_hash = io_source_hash(&io, "textures/a.texture");
    let b_source_hash = io_source_hash(&io, "textures/b.texture");
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();

    let group = server.load_group(&[
        AssetPath::parse("textures/a.texture"),
        AssetPath::parse("textures/b.texture"),
    ]);

    let queued = server.group_progress(&group);
    assert_eq!(queued.total_assets, 2);
    assert_eq!(queued.queued_assets, 2);
    assert_eq!(queued.bytes_total, 0);
    assert_eq!(queued.bytes_loaded, 0);

    server.update_loading();
    let uploading = server.group_progress(&group);
    assert_eq!(uploading.loading_assets, 1);
    assert_eq!(uploading.queued_assets, 1);
    assert_eq!(uploading.bytes_total, 0);

    finish_uploads(&mut server);
    let one_ready = server.group_progress(&group);
    assert_eq!(one_ready.ready_assets, 1);
    assert_eq!(one_ready.queued_assets, 1);
    assert_eq!(one_ready.bytes_loaded, 8);
    assert_eq!(one_ready.bytes_total, 8);

    server.update_loading();
    finish_uploads(&mut server);
    let all_ready = server.group_progress(&group);
    assert_eq!(all_ready.ready_assets, 2);
    assert_eq!(all_ready.bytes_loaded, 16);
    assert_eq!(all_ready.bytes_total, 16);
    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(
        server.metadata(group.assets[0].id()).unwrap().source_hash,
        Some(a_source_hash)
    );
    assert_eq!(
        server.metadata(group.assets[1].id()).unwrap().source_hash,
        Some(b_source_hash)
    );
}

#[test]
fn release_group_drops_tracking_without_cancelling_queued_loads() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/a.texture", texture_bytes(1, 1, 6));
    io.insert("textures/b.texture", texture_bytes(1, 1, 7));
    let a_source_hash = io_source_hash(&io, "textures/a.texture");
    let b_source_hash = io_source_hash(&io, "textures/b.texture");
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();

    let group = server.load_group(&[
        AssetPath::parse("textures/a.texture"),
        AssetPath::parse("textures/b.texture"),
    ]);
    let released = group.clone();

    assert!(server.is_group_tracked(group.id));
    server.release_group(group);
    assert!(!server.is_group_tracked(released.id));

    server.update_loading();
    finish_uploads(&mut server);
    server.update_loading();
    finish_uploads(&mut server);

    assert_eq!(server.group_state(&released), AssetLoadState::Ready);
    assert_eq!(server.group_progress(&released).ready_assets, 2);
    assert_eq!(
        server
            .metadata(released.assets[0].id())
            .unwrap()
            .source_hash,
        Some(a_source_hash)
    );
    assert_eq!(
        server
            .metadata(released.assets[1].id())
            .unwrap()
            .source_hash,
        Some(b_source_hash)
    );
}
