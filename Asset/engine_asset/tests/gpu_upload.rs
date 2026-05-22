use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn server_with_io(io: MemoryAssetIo) -> AssetServer {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

#[test]
fn initial_gpu_upload_failure_marks_asset_failed_without_ready_storage() {
    let io = MemoryAssetIo::new().with_file("textures/fail.texture", texture_bytes(1, 1, 1));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load("textures/fail.texture");

    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    server.finish_gpu_uploads(vec![GpuUploadResult::failed(texture.id(), "device lost")]);

    assert_eq!(server.state(&texture), AssetLoadState::Failed);
    assert!(server.get(&texture).is_none());
    assert!(matches!(
        server.error_by_id(texture.id()),
        Some(AssetError::GpuUpload { message }) if message == "device lost"
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, error }
            if *id == texture.id() && matches!(error, AssetError::GpuUpload { .. }))));
}

#[test]
fn gpu_upload_drain_respects_per_frame_limit_and_retains_pending_commands() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/a.texture", texture_bytes(1, 1, 2));
    io.insert("textures/b.texture", texture_bytes(1, 1, 3));
    let mut server = AssetServer::new(AssetServerConfig {
        max_gpu_uploads_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();
    let a: Handle<Texture> = server.load("textures/a.texture");
    let b: Handle<Texture> = server.load("textures/b.texture");

    server.update_loading();
    assert_eq!(server.state(&a), AssetLoadState::UploadingGpu);
    assert_eq!(server.state(&b), AssetLoadState::UploadingGpu);

    let first = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(first.len(), 1);
    server.finish_gpu_uploads(
        first
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(1))),
    );
    assert_eq!(server.state(&a), AssetLoadState::Ready);
    assert_eq!(server.state(&b), AssetLoadState::UploadingGpu);

    let second = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(second.len(), 1);
    assert_ne!(second[0].id, a.id());
    server.finish_gpu_uploads(
        second
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(2))),
    );
    assert_eq!(server.state(&b), AssetLoadState::Ready);
    assert!(server.drain_gpu_uploads().next().is_none());
}

#[test]
fn dependency_waiting_asset_fails_when_dependency_gpu_upload_fails() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 4));
    io.insert(
        "materials/hero.material",
        "name=hero\ntexture.albedo=textures/albedo.texture\n",
    );
    let mut server = server_with_io(io);
    let material: Handle<Material> = server.load("materials/hero.material");

    server.update_loading();
    assert_eq!(
        server.state(&material),
        AssetLoadState::WaitingForDependencies
    );
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    server.update_loading();
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::UploadingGpu);
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    server.finish_gpu_uploads(vec![GpuUploadResult::failed(texture_id, "oom")]);

    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Failed);
    assert_eq!(server.state(&material), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(material.id()),
        Some(AssetError::DependencyFailed { dependency, .. }) if *dependency == texture_id
    ));
    assert!(server.events().iter().any(|event| {
        matches!(
            event,
            AssetEvent::DependencyFailed { id, dependency, .. }
                if *id == material.id() && *dependency == texture_id
        )
    }));
}
