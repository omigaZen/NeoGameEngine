use engine_asset::prelude::*;

fn texture(width: u32, value: u8) -> Texture {
    Texture {
        width,
        height: 1,
        format: TextureFormat::Rgba8UnormSrgb,
        mip_count: 1,
        data: vec![value; width as usize * 4],
        gpu: None,
    }
}

#[test]
fn unload_registered_but_never_loaded_asset_reports_not_loaded_without_event() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<Texture>();
    let id = AssetId::new();
    server.registry_mut().insert(AssetMetadata::runtime(
        id,
        AssetPath::parse("textures/not_loaded.texture"),
        AssetTypeId::of::<Texture>(),
    ));

    assert_eq!(server.unload_by_id(id), Err(AssetError::NotLoaded { id }));
    assert_eq!(server.state_by_id(id), AssetLoadState::Unloaded);
    assert!(server
        .events()
        .iter()
        .all(|event| !matches!(event, AssetEvent::Unloaded { id: unloaded } if *unloaded == id)));
}

#[test]
fn bundle_compression_and_parse_errors_are_stable() {
    #[cfg(feature = "zstd")]
    {
        assert!(BundleCompressionCodecReport::for_compression(CompressionKind::Zstd).supported);
        assert!(BundleWriter::build_bytes("zstd_empty", CompressionKind::Zstd, Vec::new()).is_ok());
    }
    #[cfg(not(feature = "zstd"))]
    {
        assert_eq!(
            BundleWriter::build_bytes("bad", CompressionKind::Zstd, Vec::new()),
            Err(AssetError::Unsupported(
                "asset zstd feature is disabled for bundle writer"
            ))
        );
    }
    assert_eq!(
        asset_feature_enabled(AssetFeature::Zstd),
        BundleCompressionCodecReport::for_compression(CompressionKind::Zstd).supported
    );
    assert!(matches!(
        BundleReader::from_bytes(b"not a bundle"),
        Err(AssetError::Bundle { message }) if message.contains("missing DATA")
    ));
}

#[test]
fn already_loaded_error_remains_displayable_and_comparable() {
    let id = AssetId::new();
    let error = AssetError::AlreadyLoaded { id };

    assert_eq!(error, AssetError::AlreadyLoaded { id });
    assert!(error.to_string().contains("already loaded"));
}

#[test]
fn insert_loaded_produces_ready_asset_and_rejects_live_replacement() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let path = AssetPath::parse("textures/inserted.texture");

    let handle = server.insert_loaded(path.clone(), texture(1, 7)).unwrap();
    let id = handle.id();

    assert_eq!(server.state(&handle), AssetLoadState::Ready);
    assert_eq!(server.get(&handle).unwrap().data, vec![7; 4]);
    assert_eq!(
        server
            .metadata(id)
            .and_then(|metadata| metadata.path.as_ref()),
        Some(&path)
    );
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::LoadedCpu { id: loaded } if *loaded == id)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id: ready } if *ready == id)));

    let info = server.memory_info(id).unwrap();
    assert_eq!(info.cpu_bytes, 4);
    assert_eq!(info.gpu_bytes, 4);
    assert_eq!(info.strong_count, 1);

    assert_eq!(
        server
            .insert_loaded(path.clone(), texture(2, 9))
            .unwrap_err(),
        AssetError::AlreadyLoaded { id }
    );
    assert_eq!(server.get(&handle).unwrap().width, 1);

    drop(handle);
    server.unload_by_id(id).unwrap();
    let replacement = server.insert_loaded(path, texture(2, 3)).unwrap();

    assert_eq!(replacement.id(), id);
    assert_eq!(server.get(&replacement).unwrap().width, 2);
    assert_eq!(server.get(&replacement).unwrap().data, vec![3; 8]);
}

#[test]
fn insert_loaded_rejects_queued_asset_before_replacement() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let path = AssetPath::parse("textures/queued.texture");
    let handle: Handle<Texture> = server.load(path.clone());

    assert_eq!(server.state(&handle), AssetLoadState::Queued);
    assert_eq!(
        server.insert_loaded(path, texture(1, 5)).unwrap_err(),
        AssetError::AlreadyLoaded { id: handle.id() }
    );
    assert_eq!(server.state(&handle), AssetLoadState::Queued);
    assert!(server
        .events()
        .iter()
        .all(|event| !matches!(event, AssetEvent::Failed { id, .. } if *id == handle.id())));
}
