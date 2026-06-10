use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn server_with_textures(paths: &[(&str, u8)]) -> AssetServer {
    let mut io = MemoryAssetIo::new();
    for (path, value) in paths {
        io.insert(*path, texture_bytes(1, 1, *value));
    }
    let mut server = AssetServer::new(AssetServerConfig::default());
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

fn texture_bundle(
    name: &str,
    files: Vec<(&str, Vec<u8>, Vec<AssetId>)>,
) -> (Vec<AssetId>, Vec<u8>) {
    texture_bundle_with_compression(name, CompressionKind::None, files)
}

fn texture_bundle_with_compression(
    name: &str,
    compression: CompressionKind,
    files: Vec<(&str, Vec<u8>, Vec<AssetId>)>,
) -> (Vec<AssetId>, Vec<u8>) {
    texture_bundle_with_options(name, BundleBuildOptions::new(compression), files)
}

fn texture_bundle_with_options(
    name: &str,
    options: BundleBuildOptions,
    files: Vec<(&str, Vec<u8>, Vec<AssetId>)>,
) -> (Vec<AssetId>, Vec<u8>) {
    let mut ids = Vec::with_capacity(files.len());
    let assets = files
        .into_iter()
        .map(|(path, bytes, dependencies)| {
            let id = AssetId::new();
            ids.push(id);
            BundleAsset {
                id,
                asset_type: AssetTypeId::of::<Texture>(),
                path: AssetPath::parse(path),
                bytes,
                dependencies,
            }
        })
        .collect::<Vec<_>>();
    (
        ids,
        BundleWriter::build_bytes_with_options(name, options, assets).unwrap(),
    )
}

#[test]
fn streaming_region_preload_reports_real_progress_and_ready_state() {
    let mut server = server_with_textures(&[("textures/a.texture", 1), ("textures/b.texture", 2)]);
    let region = server
        .register_streaming_region_paths(
            "room",
            LoadPriority::High,
            &[
                AssetPath::parse("textures/a.texture"),
                AssetPath::parse("textures/b.texture"),
            ],
        )
        .unwrap();

    let progress = server.streaming_region_progress(region).unwrap();
    assert_eq!(progress.total_assets, 2);
    assert_eq!(progress.ready_assets, 0);

    let group = server.preload_streaming_region(region).unwrap();
    assert_eq!(group.assets.len(), 2);
    assert_eq!(
        server.streaming_region_state(region).unwrap(),
        AssetLoadState::LoadingBytes
    );
    assert_eq!(
        server
            .streaming_region_progress(region)
            .unwrap()
            .queued_assets,
        2
    );

    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(
        server.streaming_region_state(region).unwrap(),
        AssetLoadState::Ready
    );
    assert_eq!(
        server
            .streaming_region_progress(region)
            .unwrap()
            .ready_assets,
        2
    );
}

#[test]
fn streaming_region_priority_controls_scheduler_order() {
    let mut config = AssetServerConfig::default();
    config.max_io_jobs_per_frame = 1;
    let mut io = MemoryAssetIo::new();
    io.insert("textures/low.texture", texture_bytes(1, 1, 1));
    io.insert("textures/high.texture", texture_bytes(1, 1, 2));
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let low = server
        .register_streaming_region_paths(
            "low",
            LoadPriority::Background,
            &[AssetPath::parse("textures/low.texture")],
        )
        .unwrap();
    let high = server
        .register_streaming_region_paths(
            "high",
            LoadPriority::Immediate,
            &[AssetPath::parse("textures/high.texture")],
        )
        .unwrap();

    server.preload_streaming_region(low).unwrap();
    server.preload_streaming_region(high).unwrap();
    server.update_loading();

    let high_id = server
        .id_from_path(&AssetPath::parse("textures/high.texture"))
        .unwrap();
    let low_id = server
        .id_from_path(&AssetPath::parse("textures/low.texture"))
        .unwrap();
    assert_eq!(server.state_by_id(high_id), AssetLoadState::UploadingGpu);
    assert_eq!(server.state_by_id(low_id), AssetLoadState::Queued);
}

#[test]
fn streaming_region_priority_can_be_updated_before_preload() {
    let mut config = AssetServerConfig::default();
    config.max_io_jobs_per_frame = 1;
    let mut io = MemoryAssetIo::new();
    io.insert("textures/low.texture", texture_bytes(1, 1, 1));
    io.insert("textures/high.texture", texture_bytes(1, 1, 2));
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let low = server
        .register_streaming_region_paths(
            "low",
            LoadPriority::Background,
            &[AssetPath::parse("textures/low.texture")],
        )
        .unwrap();
    let high = server
        .register_streaming_region_paths(
            "high",
            LoadPriority::Background,
            &[AssetPath::parse("textures/high.texture")],
        )
        .unwrap();

    assert_eq!(
        server
            .set_streaming_region_priority(low, LoadPriority::Immediate)
            .unwrap(),
        LoadPriority::Background
    );
    assert_eq!(
        server.streaming_region(low).unwrap().priority,
        LoadPriority::Immediate
    );

    server.preload_streaming_region(low).unwrap();
    server.preload_streaming_region(high).unwrap();
    server.update_loading();

    let high_id = server
        .id_from_path(&AssetPath::parse("textures/high.texture"))
        .unwrap();
    let low_id = server
        .id_from_path(&AssetPath::parse("textures/low.texture"))
        .unwrap();
    assert_eq!(server.state_by_id(low_id), AssetLoadState::UploadingGpu);
    assert_eq!(server.state_by_id(high_id), AssetLoadState::Queued);
}

#[test]
fn streaming_region_priority_can_be_updated_while_requests_are_queued() {
    let mut config = AssetServerConfig::default();
    config.max_io_jobs_per_frame = 1;
    let mut io = MemoryAssetIo::new();
    io.insert("textures/low.texture", texture_bytes(1, 1, 1));
    io.insert("textures/high.texture", texture_bytes(1, 1, 2));
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let low = server
        .register_streaming_region_paths(
            "low",
            LoadPriority::Background,
            &[AssetPath::parse("textures/low.texture")],
        )
        .unwrap();
    let high = server
        .register_streaming_region_paths(
            "high",
            LoadPriority::Background,
            &[AssetPath::parse("textures/high.texture")],
        )
        .unwrap();

    server.preload_streaming_region(low).unwrap();
    server.preload_streaming_region(high).unwrap();
    assert_eq!(
        server
            .set_streaming_region_priority(low, LoadPriority::Immediate)
            .unwrap(),
        LoadPriority::Background
    );
    assert_eq!(
        server.streaming_region(low).unwrap().priority,
        LoadPriority::Immediate
    );

    server.update_loading();

    let high_id = server
        .id_from_path(&AssetPath::parse("textures/high.texture"))
        .unwrap();
    let low_id = server
        .id_from_path(&AssetPath::parse("textures/low.texture"))
        .unwrap();
    assert_eq!(server.state_by_id(low_id), AssetLoadState::UploadingGpu);
    assert_eq!(server.state_by_id(high_id), AssetLoadState::Queued);
}

#[test]
fn streaming_region_residency_protects_assets_from_region_unload() {
    let mut server = server_with_textures(&[("textures/resident.texture", 3)]);
    let path = AssetPath::parse("textures/resident.texture");
    let region = server
        .register_streaming_region_paths("resident", LoadPriority::Normal, &[path.clone()])
        .unwrap();
    let asset_id = server.id_from_path(&path).unwrap();

    server.preload_streaming_region(region).unwrap();
    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(asset_id), AssetLoadState::Ready);

    server.set_streaming_region_resident(region, true).unwrap();
    assert_eq!(server.unload_streaming_region(region).unwrap(), 0);
    assert_eq!(server.state_by_id(asset_id), AssetLoadState::Ready);

    server.set_streaming_region_resident(region, false).unwrap();
    assert_eq!(server.unload_streaming_region(region).unwrap(), 1);
    assert_eq!(server.state_by_id(asset_id), AssetLoadState::Unloaded);
}

#[test]
fn shared_streaming_asset_stays_resident_until_all_regions_release_it() {
    let mut server = server_with_textures(&[("textures/shared.texture", 4)]);
    let path = AssetPath::parse("textures/shared.texture");
    let room_a = server
        .register_streaming_region_paths("room_a", LoadPriority::Normal, &[path.clone()])
        .unwrap();
    let room_b = server
        .register_streaming_region_paths("room_b", LoadPriority::Normal, &[path.clone()])
        .unwrap();
    let asset_id = server.id_from_path(&path).unwrap();

    server.preload_streaming_region(room_a).unwrap();
    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(asset_id), AssetLoadState::Ready);

    server.set_streaming_region_resident(room_a, true).unwrap();
    server.set_streaming_region_resident(room_b, true).unwrap();
    assert!(server.is_asset_resident(asset_id));

    server.set_streaming_region_resident(room_a, false).unwrap();
    assert!(server.is_asset_resident(asset_id));
    assert_eq!(server.unload_streaming_region(room_a).unwrap(), 0);
    assert_eq!(server.state_by_id(asset_id), AssetLoadState::Ready);

    server.set_streaming_region_resident(room_b, false).unwrap();
    assert!(!server.is_asset_resident(asset_id));
    assert_eq!(server.unload_streaming_region(room_b).unwrap(), 1);
    assert_eq!(server.state_by_id(asset_id), AssetLoadState::Unloaded);
}

#[test]
fn streaming_region_can_add_and_remove_assets() {
    let mut server = server_with_textures(&[
        ("textures/base.texture", 10),
        ("textures/extra.texture", 11),
    ]);
    let region_path = AssetPath::parse("textures/base.texture");
    let extra_path = AssetPath::parse("textures/extra.texture");
    let region = server
        .register_streaming_region_paths("dynamic", LoadPriority::Normal, &[region_path.clone()])
        .unwrap();
    let base_id = server.id_from_path(&region_path).unwrap();
    let region_data = server.streaming_region(region).unwrap();
    assert_eq!(region_data.assets.len(), 1);

    assert!(server
        .add_asset_to_streaming_region(region, &extra_path)
        .unwrap());
    let extra_id = server.id_from_path(&extra_path).unwrap();
    assert_eq!(server.streaming_region(region).unwrap().assets.len(), 2);
    assert!(!server
        .add_asset_to_streaming_region(region, &extra_path)
        .unwrap());

    server.preload_streaming_region(region).unwrap();
    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(extra_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(base_id), AssetLoadState::Ready);

    server.set_streaming_region_resident(region, true).unwrap();
    assert!(server.is_asset_resident(base_id));
    assert!(server.is_asset_resident(extra_id));

    assert!(server
        .remove_streaming_region_asset(region, extra_id)
        .unwrap());
    assert_eq!(server.streaming_region(region).unwrap().assets.len(), 1);
    assert!(!server.is_asset_resident(extra_id));

    server.set_streaming_region_resident(region, false).unwrap();
    assert_eq!(server.unload_streaming_region(region).unwrap(), 1);
    assert_eq!(server.state_by_id(base_id), AssetLoadState::Unloaded);
    assert_eq!(
        server
            .remove_streaming_region_asset(region, extra_id)
            .unwrap(),
        false
    );
}

#[test]
fn streaming_region_add_asset_rejects_unknown_extension() {
    let mut server = server_with_textures(&[]);
    let region = server
        .register_streaming_region_paths("dynamic", LoadPriority::Normal, &[])
        .unwrap();
    let err = server
        .add_asset_to_streaming_region(region, &AssetPath::parse("audio/unknown.bin"))
        .unwrap_err();
    assert!(matches!(
        err,
        AssetError::LoaderNotFound { extension: ref ext } if ext == "bin"
    ));
}

#[test]
fn removing_resident_streaming_regions_releases_shared_residency_counts() {
    let mut server = server_with_textures(&[("textures/shared.texture", 5)]);
    let path = AssetPath::parse("textures/shared.texture");
    let room_a = server
        .register_streaming_region_paths("room_a", LoadPriority::Normal, &[path.clone()])
        .unwrap();
    let room_b = server
        .register_streaming_region_paths("room_b", LoadPriority::Normal, &[path.clone()])
        .unwrap();
    let asset_id = server.id_from_path(&path).unwrap();

    server.set_streaming_region_resident(room_a, true).unwrap();
    server.set_streaming_region_resident(room_b, true).unwrap();
    assert!(server.is_asset_resident(asset_id));

    server.remove_streaming_region(room_a).unwrap();
    assert!(server.is_asset_resident(asset_id));
    server.remove_streaming_region(room_b).unwrap();
    assert!(!server.is_asset_resident(asset_id));
}

#[test]
fn memory_budget_eviction_skips_resident_streaming_assets_until_residency_clears() {
    let mut config = AssetServerConfig::default();
    config.gc.memory_budget_bytes = Some(0);
    let mut io = MemoryAssetIo::new();
    io.insert("textures/resident.texture", texture_bytes(1, 1, 6));
    io.insert("textures/evictable.texture", texture_bytes(1, 1, 7));
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let resident_path = AssetPath::parse("textures/resident.texture");
    let evictable_path = AssetPath::parse("textures/evictable.texture");
    let resident_region = server
        .register_streaming_region_paths("resident", LoadPriority::Normal, &[resident_path.clone()])
        .unwrap();
    let evictable_region = server
        .register_streaming_region_paths(
            "evictable",
            LoadPriority::Normal,
            &[evictable_path.clone()],
        )
        .unwrap();
    let resident_id = server.id_from_path(&resident_path).unwrap();
    let evictable_id = server.id_from_path(&evictable_path).unwrap();

    server.preload_streaming_region(resident_region).unwrap();
    server.preload_streaming_region(evictable_region).unwrap();
    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(resident_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(evictable_id), AssetLoadState::Ready);

    server
        .set_streaming_region_resident(resident_region, true)
        .unwrap();
    server.update_gc(10);
    assert_eq!(server.state_by_id(resident_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(evictable_id), AssetLoadState::Unloaded);

    server
        .set_streaming_region_resident(resident_region, false)
        .unwrap();
    server.update_gc(11);
    assert_eq!(server.state_by_id(resident_id), AssetLoadState::Unloaded);
}

#[test]
fn streaming_region_can_preload_and_unload_mounted_bundle_manifest_assets() {
    let (ids, bundle) = texture_bundle(
        "level_stream",
        vec![
            ("textures/a.texture", texture_bytes(1, 1, 8), Vec::new()),
            ("textures/b.texture", texture_bytes(1, 1, 9), Vec::new()),
        ],
    );
    let bundle_io = BundleAssetIo::from_bytes(&bundle).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle).unwrap();

    let region = server
        .register_streaming_region_bundle("bundle_region", LoadPriority::High, mounted.id)
        .unwrap();
    let region_data = server.streaming_region(region).unwrap();
    assert_eq!(region_data.assets.len(), 2);
    assert_eq!(
        region_data
            .assets
            .iter()
            .map(UntypedHandle::id)
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        server.path_from_id(ids[0]),
        Some(&AssetPath::parse("textures/a.texture"))
    );

    let group = server.preload_streaming_region(region).unwrap();
    assert_eq!(group.assets.len(), 2);
    assert_eq!(
        server
            .streaming_region_progress(region)
            .unwrap()
            .queued_assets,
        2
    );
    server.update_loading();
    finish_uploads(&mut server);

    assert_eq!(
        server.streaming_region_state(region).unwrap(),
        AssetLoadState::Ready
    );
    assert_eq!(server.unload_streaming_region(region).unwrap(), 2);
    assert_eq!(server.state_by_id(ids[0]), AssetLoadState::Unloaded);
    assert_eq!(server.state_by_id(ids[1]), AssetLoadState::Unloaded);
}

#[test]
fn streaming_region_preloads_rle_compressed_mounted_bundle_assets() {
    let first = texture_bytes(8, 8, 42);
    let second = texture_bytes(8, 8, 43);
    let (ids, bundle) = texture_bundle_with_options(
        "compressed_stream",
        BundleBuildOptions::new(CompressionKind::Rle).with_chunk_policy(
            BundleChunkPartitionPolicy::MaxUncompressedBytes(first.len() + 1),
        ),
        vec![
            ("textures/compressed.texture", first, Vec::new()),
            ("textures/compressed_b.texture", second, Vec::new()),
        ],
    );
    let bundle_io = BundleAssetIo::from_bytes_with_loading_policy(
        &bundle,
        BundleChunkLoadingPolicy::OnDemandCached,
    )
    .unwrap();
    assert_eq!(
        bundle_io.manifest().chunks[0].compression,
        CompressionKind::Rle
    );
    assert_eq!(bundle_io.manifest().chunks.len(), 2);
    assert_eq!(bundle_io.chunk_cache_stats().decoded_chunks, 0);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io.clone());
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle).unwrap();

    let region = server
        .register_streaming_region_bundle("compressed_region", LoadPriority::High, mounted.id)
        .unwrap();
    let group = server.preload_streaming_region(region).unwrap();
    server.update_loading();
    finish_uploads(&mut server);

    assert_eq!(
        server.streaming_region_state(region).unwrap(),
        AssetLoadState::Ready
    );
    assert_eq!(server.group_state(&group), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(ids[0]), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(ids[1]), AssetLoadState::Ready);
    assert_eq!(bundle_io.chunk_cache_stats().decoded_chunks, 2);
    assert_eq!(
        server
            .get_by_id::<Texture>(ids[0])
            .unwrap()
            .data
            .first()
            .copied(),
        Some(42)
    );
}

#[test]
fn streaming_region_bundle_subset_preserves_manifest_metadata_and_validates_ids() {
    let dependency = AssetId::new();
    let (ids, bundle) = texture_bundle(
        "subset_stream",
        vec![
            ("textures/base.texture", texture_bytes(1, 1, 10), Vec::new()),
            (
                "textures/subset.texture",
                texture_bytes(1, 1, 11),
                vec![dependency],
            ),
        ],
    );
    let bundle_io = BundleAssetIo::from_bytes(&bundle).unwrap();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(bundle_io);
    server.register_builtin_loaders();
    let mounted = server.mount_bundle_bytes(&bundle).unwrap();

    let missing_id = AssetId::new();
    assert!(matches!(
        server.register_streaming_region_bundle_subset(
            "missing",
            LoadPriority::Normal,
            mounted.id,
            &[missing_id],
        ),
        Err(AssetError::AssetNotFound { id }) if id == missing_id
    ));

    let region = server
        .register_streaming_region_bundle_subset(
            "subset",
            LoadPriority::Immediate,
            mounted.id,
            &[ids[1]],
        )
        .unwrap();
    let region_data = server.streaming_region(region).unwrap();
    assert_eq!(region_data.assets.len(), 1);
    assert_eq!(region_data.assets[0].id(), ids[1]);
    assert_eq!(
        server.metadata(ids[1]).unwrap().dependencies,
        vec![dependency]
    );
    assert_eq!(server.state_by_id(ids[0]), AssetLoadState::Unloaded);

    server.preload_streaming_region(region).unwrap();
    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(ids[1]), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(ids[0]), AssetLoadState::Unloaded);

    assert!(matches!(
        server.register_streaming_region_bundle("unmounted", LoadPriority::Normal, BundleId(999)),
        Err(AssetError::Bundle { .. })
    ));
}
