use std::fs;

use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn shader_bytes() -> Vec<u8> {
    b"@fragment fn main() {}\n".to_vec()
}

fn scene_bytes() -> Vec<u8> {
    b"NGA_SCENE_V1\nname=hero_scene\ndependency=textures/albedo.texture\ndependency=materials/hero.material\nentity=Root\ncomponent=Transform|translation=0,0,0\nentity=Hero;parent=0\ncomponent=Tag|value=hero\n".to_vec()
}

fn prefab_bytes() -> Vec<u8> {
    b"NGA_PREFAB_V1\ndependency=textures/albedo.texture\ndependency=materials/hero.material\nroot=Hero\ncomponent=Transform|translation=0,0,0\nchild=Weapon;parent=0\ncomponent=Tag|value=weapon\n".to_vec()
}

fn animation_source_bytes() -> Vec<u8> {
    b"NGA_ANIMATION_SOURCE_V1\nduration=1.0\nticks_per_second=24.0\ntrack=node:Hero\ntranslation=0.0:0,0,0\nrotation=0.0:0,0,0,1\nscale=0.0:1,1,1\n".to_vec()
}

fn animation_runtime_bytes() -> Vec<u8> {
    b"NGA_ANIMATION_V1\nduration=1.0\nticks_per_second=24.0\ntrack=node:Hero\ntranslation=0.0:0,0,0\nrotation=0.0:0,0,0,1\nscale=0.0:1,1,1\n".to_vec()
}

fn skeleton_source_bytes() -> Vec<u8> {
    b"NGA_SKELETON_SOURCE_V1\nbone=Root;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Child;parent=0;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\n".to_vec()
}

fn skeleton_runtime_bytes() -> Vec<u8> {
    b"NGA_SKELETON_V1\nbone=Root;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Child;parent=0;bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\n".to_vec()
}

fn database_config(name: &str) -> AssetDatabaseConfig {
    let root = std::env::temp_dir()
        .join("engine_asset_feature_flags")
        .join(name);
    AssetDatabaseConfig {
        source_root: root.join("source"),
        imported_root: root.join("imported"),
        cooked_root: root.join("cooked"),
        registry_path: root.join("asset_registry.txt"),
    }
}

#[cfg(not(feature = "async_loading"))]
#[test]
fn disabled_async_loading_config_reports_visible_unsupported_diagnostic() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let status = asset_feature_status(AssetFeature::AsyncLoading);
    assert!(!status.enabled);
    assert_eq!(status.name, "async_loading");
    assert_eq!(
        server.set_async_loading_enabled(true),
        Err(AssetError::Unsupported(
            "asset async_loading feature is disabled"
        ))
    );

    server.config_mut().enable_async_loading = true;
    let report = server.loading_policy_report();
    assert_eq!(report.async_loading_feature, status);
    assert_eq!(
        report.parallel_feature,
        asset_feature_status(AssetFeature::Parallel)
    );
    assert_eq!(report.mode, AssetLoadingExecutionMode::Synchronous);
    assert_eq!(report.effective_worker_threads, 0);
    assert_eq!(
        report.first_error(),
        Some(&AssetError::Unsupported(
            "asset async_loading feature is disabled"
        ))
    );
    assert_eq!(
        server.validate_loading_policy(),
        Err(AssetError::Unsupported(
            "asset async_loading feature is disabled"
        ))
    );
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].feature, AssetFeature::AsyncLoading);
    assert!(matches!(
        &report.diagnostics[0].error,
        Some(AssetError::Unsupported(message)) if *message == AssetFeature::AsyncLoading.unsupported_message()
    ));
}

#[cfg(feature = "async_loading")]
#[test]
fn enabled_async_loading_config_reports_worker_execution_mode() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_async_loading_enabled(true).unwrap();
    let status = asset_feature_status(AssetFeature::AsyncLoading);
    assert_eq!(
        status.enabled,
        asset_feature_enabled(AssetFeature::AsyncLoading)
    );
    assert_eq!(status.name, "async_loading");

    let report = server.loading_policy_report();
    assert_eq!(report.async_loading_feature, status);
    assert_eq!(
        report.parallel_feature,
        asset_feature_status(AssetFeature::Parallel)
    );
    assert_eq!(report.mode, AssetLoadingExecutionMode::WorkerAsync);
    assert_eq!(report.requested_async_loading, true);
    assert_eq!(report.effective_worker_threads, 1);
    assert!(report.diagnostics.is_empty());
    assert!(report.first_error().is_none());
    assert_eq!(server.validate_loading_policy(), Ok(()));
    assert_eq!(require_asset_feature(AssetFeature::AsyncLoading), Ok(()));
}

#[test]
fn editor_feature_status_matches_composed_feature_gate() {
    let status = asset_feature_status(AssetFeature::Editor);
    assert_eq!(status.enabled, asset_feature_enabled(AssetFeature::Editor));
    assert_eq!(status.name, "editor");
    if status.enabled {
        assert_eq!(require_asset_feature(AssetFeature::Editor), Ok(()));
        assert_eq!(
            asset_feature_status(AssetFeature::Importers).enabled,
            status.enabled
        );
        assert_eq!(
            asset_feature_status(AssetFeature::Cookers).enabled,
            status.enabled
        );
    } else {
        assert_eq!(
            require_asset_feature(AssetFeature::Editor),
            Err(AssetError::Unsupported("asset editor feature is disabled"))
        );
    }
}

#[cfg(not(feature = "editor"))]
#[test]
fn disabled_editor_feature_reports_visible_unsupported_diagnostic() {
    let status = asset_feature_status(AssetFeature::Editor);
    assert!(!status.enabled);
    assert_eq!(status.name, "editor");
    assert_eq!(
        require_asset_feature(AssetFeature::Editor),
        Err(AssetError::Unsupported("asset editor feature is disabled"))
    );
}

#[cfg(not(feature = "parallel"))]
#[test]
fn disabled_parallel_worker_config_reports_visible_unsupported_diagnostic() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let status = asset_feature_status(AssetFeature::Parallel);
    assert!(!status.enabled);
    assert_eq!(status.name, "parallel");
    assert_eq!(
        server.set_parallel_worker_threads(2),
        Err(AssetError::Unsupported(
            "asset parallel feature is disabled"
        ))
    );

    server.config_mut().worker_threads = 2;
    let report = server.loading_policy_report();
    assert_eq!(
        report.async_loading_feature,
        asset_feature_status(AssetFeature::AsyncLoading)
    );
    assert_eq!(report.parallel_feature, status);
    assert_eq!(report.requested_worker_threads, 2);
    assert_eq!(report.effective_worker_threads, 0);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.feature == AssetFeature::Parallel
            && diagnostic.error
                == Some(AssetError::Unsupported(
                    "asset parallel feature is disabled",
                ))
    }));
    assert_eq!(
        report.require_supported(),
        Err(AssetError::Unsupported(
            "asset parallel feature is disabled"
        ))
    );
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].feature, AssetFeature::Parallel);
    assert!(matches!(
        &report.diagnostics[0].error,
        Some(AssetError::Unsupported(message)) if *message == AssetFeature::Parallel.unsupported_message()
    ));
}

#[cfg(feature = "parallel")]
#[test]
fn enabled_parallel_config_accepts_worker_count_without_async_loading() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_parallel_worker_threads(4).unwrap();
    let status = asset_feature_status(AssetFeature::Parallel);
    assert_eq!(
        status.enabled,
        asset_feature_enabled(AssetFeature::Parallel)
    );
    assert_eq!(status.name, "parallel");

    let report = server.loading_policy_report();
    assert_eq!(
        report.async_loading_feature,
        asset_feature_status(AssetFeature::AsyncLoading)
    );
    assert_eq!(
        report.parallel_feature,
        asset_feature_status(AssetFeature::Parallel)
    );
    assert_eq!(report.requested_worker_threads, 4);
    assert_eq!(report.effective_worker_threads, 0);
    assert!(report.diagnostics.is_empty());
    assert!(report.first_error().is_none());
    assert_eq!(report.require_supported(), Ok(()));
    assert_eq!(report.mode, AssetLoadingExecutionMode::Synchronous);
    assert_eq!(require_asset_feature(AssetFeature::Parallel), Ok(()));
}

#[cfg(all(feature = "async_loading", feature = "parallel"))]
#[test]
fn enabled_async_parallel_config_reports_effective_worker_count() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_async_loading_enabled(true).unwrap();
    server.set_parallel_worker_threads(4).unwrap();
    let status = asset_feature_status(AssetFeature::Parallel);
    assert_eq!(
        status.enabled,
        asset_feature_enabled(AssetFeature::Parallel)
    );
    assert_eq!(status.name, "parallel");

    let report = server.loading_policy_report();
    assert_eq!(
        report.async_loading_feature,
        asset_feature_status(AssetFeature::AsyncLoading)
    );
    assert_eq!(
        report.parallel_feature,
        asset_feature_status(AssetFeature::Parallel)
    );
    assert_eq!(report.mode, AssetLoadingExecutionMode::WorkerAsync);
    assert_eq!(report.requested_worker_threads, 4);
    assert_eq!(report.effective_worker_threads, 4);
    assert!(report.diagnostics.is_empty());
    assert!(report.first_error().is_none());
    assert_eq!(report.require_supported(), Ok(()));
    assert_eq!(require_asset_feature(AssetFeature::Parallel), Ok(()));
}

#[test]
fn zstd_feature_status_matches_bundle_codec_report() {
    let status = asset_feature_status(AssetFeature::Zstd);
    let report = BundleCompressionCodecReport::for_compression(CompressionKind::Zstd);
    assert_eq!(status.name, "zstd");
    assert_eq!(status.enabled, report.supported);
    if status.enabled {
        assert!(report.reason.is_none());
        assert_eq!(require_asset_feature(AssetFeature::Zstd), Ok(()));
        let bytes =
            BundleWriter::build_bytes("zstd_empty", CompressionKind::Zstd, Vec::new()).unwrap();
        let reader = BundleReader::from_bytes(&bytes).unwrap();
        assert_eq!(reader.manifest().name, "zstd_empty");
        assert!(reader.manifest().entries.is_empty());
    } else {
        assert_eq!(
            require_asset_feature(AssetFeature::Zstd),
            Err(AssetError::Unsupported("asset zstd feature is disabled"))
        );
        assert!(report.reason.as_deref().unwrap().contains("zstd feature"));
    }
}

#[test]
fn filesystem_feature_gate_matches_filesystem_io_behavior() {
    let io = FileSystemAssetIo::new(std::env::temp_dir());
    let status = asset_feature_status(AssetFeature::Filesystem);
    assert_eq!(
        status.enabled,
        asset_feature_enabled(AssetFeature::Filesystem)
    );
    assert_eq!(status.name, "filesystem");

    if asset_feature_enabled(AssetFeature::Filesystem) {
        assert_eq!(require_asset_feature(AssetFeature::Filesystem), Ok(()));
        let error = io.read("engine_asset_missing_file.texture").unwrap_err();
        assert!(matches!(error, AssetIoError::NotFound { .. }));
        assert_eq!(error.action(), AssetIoAction::Read);
    } else {
        assert_eq!(
            require_asset_feature(AssetFeature::Filesystem),
            Err(AssetError::Unsupported(
                "asset filesystem feature is disabled"
            ))
        );
        assert!(!io.exists("engine_asset_missing_file.texture"));
        let error = io.read("engine_asset_missing_file.texture").unwrap_err();
        assert!(matches!(error, AssetIoError::ReadFailed { .. }));
        assert_eq!(error.action(), AssetIoAction::Read);
        assert_eq!(error.path(), "engine_asset_missing_file.texture");
        assert!(error
            .message()
            .is_some_and(|message| message.contains("filesystem feature is disabled")));

        let mut server = AssetServer::new(AssetServerConfig::default());
        server.register_builtin_loaders();
        let handle: Handle<Texture> = server.load("engine_asset_missing_file.texture");
        server.update_loading();
        assert_eq!(server.state(&handle), AssetLoadState::Failed);
        assert!(matches!(
            server.error_by_id(handle.id()),
            Some(AssetError::Io { message })
                if message.contains("filesystem feature is disabled")
        ));
    }
}

#[test]
fn importer_feature_gates_match_registration_paths() {
    let config = database_config("importer_feature_gates");
    let mut database = AssetDatabase::new(config.clone());
    database.set_io(
        MemoryAssetIo::new()
            .with_file("shaders/pbr.wgsl", shader_bytes())
            .with_file("textures/albedo.texture", texture_bytes(1, 1, 7))
            .with_file(
                "materials/hero.material",
                b"name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\n"
                    .to_vec(),
            )
            .with_file("animations/hero.animation", animation_source_bytes())
            .with_file("skeletons/hero.skeleton", skeleton_source_bytes())
            .with_file("scenes/hero.scene", scene_bytes())
            .with_file("prefabs/hero.prefab", prefab_bytes()),
    );

    if asset_feature_enabled(AssetFeature::Importers) {
        database.try_register_builtin_importers().unwrap();
    } else {
        assert_eq!(
            database.try_register_builtin_importers(),
            Err(AssetError::Unsupported(
                "asset importers feature is disabled"
            ))
        );
        database.register_builtin_importers();
    }

    if asset_feature_enabled(AssetFeature::TextureImporter) {
        let id = database
            .import_asset_path_with_settings(
                &AssetPath::parse("textures/albedo.texture"),
                &ImporterSettings::default(),
            )
            .unwrap();
        let metadata = database.registry().get(id).unwrap();
        assert_eq!(metadata.asset_type, AssetTypeId::of::<Texture>());
        assert_eq!(metadata.importer.as_deref(), Some("TextureImporter"));
        assert_eq!(metadata.importer_version, 3);
        assert_eq!(
            metadata.source_path.as_ref(),
            Some(&AssetPath::parse("textures/albedo.texture"))
        );
        assert_eq!(
            metadata.cooked_path.as_ref(),
            Some(&AssetPath::parse("textures/albedo.texture"))
        );
        assert_eq!(
            fs::read(config.imported_root.join("textures/albedo.texture")).unwrap(),
            texture_bytes(1, 1, 7)
        );
        let shader_id = database
            .import_asset_path(&AssetPath::parse("shaders/pbr.wgsl"))
            .unwrap();
        let material_id = database
            .import_asset_path(&AssetPath::parse("materials/hero.material"))
            .unwrap();
        let animation_id = database
            .import_asset_path(&AssetPath::parse("animations/hero.animation"))
            .unwrap();
        let skeleton_id = database
            .import_asset_path(&AssetPath::parse("skeletons/hero.skeleton"))
            .unwrap();
        let scene_id = database
            .import_asset_path(&AssetPath::parse("scenes/hero.scene"))
            .unwrap();
        let prefab_id = database
            .import_asset_path(&AssetPath::parse("prefabs/hero.prefab"))
            .unwrap();
        assert_eq!(
            database.registry().get(material_id).unwrap().dependencies,
            vec![shader_id, id]
        );
        assert_eq!(
            database.registry().get(scene_id).unwrap().dependencies,
            vec![id, material_id]
        );
        assert_eq!(
            database.registry().get(prefab_id).unwrap().dependencies,
            vec![id, material_id]
        );
        assert_eq!(
            database.registry().get(animation_id).unwrap().asset_type,
            AssetTypeId::of::<AnimationClip>()
        );
        assert_eq!(
            database.registry().get(skeleton_id).unwrap().asset_type,
            AssetTypeId::of::<Skeleton>()
        );
        assert!(database
            .registry()
            .get(animation_id)
            .unwrap()
            .dependencies
            .is_empty());
        assert!(database
            .registry()
            .get(skeleton_id)
            .unwrap()
            .dependencies
            .is_empty());
    } else {
        assert!(matches!(
            database.import_asset_path_with_settings(
                &AssetPath::parse("textures/albedo.texture"),
                &ImporterSettings::default(),
            ),
            Err(AssetError::Import { message }) if message.contains("no importer registered")
        ));
        assert!(matches!(
            database.import_asset_path(&AssetPath::parse("scenes/hero.scene")),
            Err(AssetError::Import { message }) if message.contains("no importer registered")
        ));
        assert!(matches!(
            database.import_asset_path(&AssetPath::parse("prefabs/hero.prefab")),
            Err(AssetError::Import { message }) if message.contains("no importer registered")
        ));
        assert!(matches!(
            database.import_asset_path(&AssetPath::parse("animations/hero.animation")),
            Err(AssetError::Import { message }) if message.contains("no importer registered")
        ));
        assert!(matches!(
            database.import_asset_path(&AssetPath::parse("skeletons/hero.skeleton")),
            Err(AssetError::Import { message }) if message.contains("no importer registered")
        ));
    }
}

#[test]
fn cooker_feature_gates_match_registration_paths() {
    let mut database = AssetDatabase::new(database_config("cooker_feature_gates"));
    database.set_io(
        MemoryAssetIo::new()
            .with_file("textures/albedo.texture", texture_bytes(1, 1, 7))
            .with_file("animations/hero.animation", animation_source_bytes())
            .with_file("skeletons/hero.skeleton", skeleton_source_bytes())
            .with_file("scenes/hero.scene", scene_bytes())
            .with_file("prefabs/hero.prefab", prefab_bytes()),
    );

    if asset_feature_enabled(AssetFeature::Cookers) {
        database.try_register_builtin_cookers().unwrap();
    } else {
        assert_eq!(
            database.try_register_builtin_cookers(),
            Err(AssetError::Unsupported("asset cookers feature is disabled"))
        );
        database.register_builtin_cookers();
    }

    let id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_f001);
    database.registry_mut().insert(AssetMetadata::runtime(
        id,
        AssetPath::parse("textures/albedo.texture"),
        Texture::TYPE_ID,
    ));
    let scene_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_f002);
    database.registry_mut().insert(AssetMetadata::runtime(
        scene_id,
        AssetPath::parse("scenes/hero.scene"),
        SceneAsset::TYPE_ID,
    ));
    let prefab_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_f003);
    database.registry_mut().insert(AssetMetadata::runtime(
        prefab_id,
        AssetPath::parse("prefabs/hero.prefab"),
        Prefab::TYPE_ID,
    ));
    let animation_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_f004);
    database.registry_mut().insert(AssetMetadata::runtime(
        animation_id,
        AssetPath::parse("animations/hero.animation"),
        AnimationClip::TYPE_ID,
    ));
    let skeleton_id = AssetId::from_u128(0x4e47_4153_5345_5400_0000_0000_0000_f005);
    database.registry_mut().insert(AssetMetadata::runtime(
        skeleton_id,
        AssetPath::parse("skeletons/hero.skeleton"),
        Skeleton::TYPE_ID,
    ));

    if asset_feature_enabled(AssetFeature::TextureCooker) {
        let output = database.cook_asset(id, TargetPlatform::Windows).unwrap();
        assert_eq!(output.id, id);
        assert_eq!(
            output.metadata.path.as_ref(),
            Some(&AssetPath::parse("textures/albedo.texture"))
        );
        assert_eq!(output.metadata.asset_type, Texture::TYPE_ID);
        assert_eq!(
            output.metadata.cooked_path.as_ref(),
            Some(&AssetPath::parse("textures/albedo.texture"))
        );
        assert_eq!(output.metadata.cooked_hash, Some(output.content_hash));
        assert_eq!(output.metadata.version_hash, Some(VersionHash(2)));
        assert_eq!(output.bytes, texture_bytes(1, 1, 7));
    } else {
        assert!(matches!(
            database.cook_asset(id, TargetPlatform::Windows),
            Err(AssetError::Cook { message }) if message.contains("no cooker registered")
        ));
    }

    if asset_feature_enabled(AssetFeature::Cookers) {
        let scene_output = database
            .cook_asset(scene_id, TargetPlatform::Windows)
            .unwrap();
        let prefab_output = database
            .cook_asset(prefab_id, TargetPlatform::Windows)
            .unwrap();
        let animation_output = database
            .cook_asset(animation_id, TargetPlatform::Windows)
            .unwrap();
        let skeleton_output = database
            .cook_asset(skeleton_id, TargetPlatform::Windows)
            .unwrap();
        assert_eq!(
            scene_output.metadata.path.as_ref(),
            Some(&AssetPath::parse("scenes/hero.scene"))
        );
        assert_eq!(scene_output.metadata.asset_type, SceneAsset::TYPE_ID);
        assert_eq!(
            scene_output.metadata.cooked_path.as_ref(),
            Some(&AssetPath::parse("scenes/hero.scene"))
        );
        assert_eq!(
            scene_output.metadata.cooked_hash,
            Some(scene_output.content_hash)
        );
        assert_eq!(scene_output.metadata.version_hash, Some(VersionHash(1)));
        assert_eq!(
            prefab_output.metadata.path.as_ref(),
            Some(&AssetPath::parse("prefabs/hero.prefab"))
        );
        assert_eq!(prefab_output.metadata.asset_type, Prefab::TYPE_ID);
        assert_eq!(
            prefab_output.metadata.cooked_path.as_ref(),
            Some(&AssetPath::parse("prefabs/hero.prefab"))
        );
        assert_eq!(
            prefab_output.metadata.cooked_hash,
            Some(prefab_output.content_hash)
        );
        assert_eq!(prefab_output.metadata.version_hash, Some(VersionHash(1)));
        assert_eq!(
            animation_output.metadata.path.as_ref(),
            Some(&AssetPath::parse("animations/hero.animation"))
        );
        assert_eq!(animation_output.metadata.asset_type, AnimationClip::TYPE_ID);
        assert_eq!(
            animation_output.metadata.cooked_path.as_ref(),
            Some(&AssetPath::parse("animations/hero.animation"))
        );
        assert_eq!(
            animation_output.metadata.cooked_hash,
            Some(animation_output.content_hash)
        );
        assert_eq!(animation_output.metadata.version_hash, Some(VersionHash(2)));
        assert_eq!(
            skeleton_output.metadata.path.as_ref(),
            Some(&AssetPath::parse("skeletons/hero.skeleton"))
        );
        assert_eq!(skeleton_output.metadata.asset_type, Skeleton::TYPE_ID);
        assert_eq!(
            skeleton_output.metadata.cooked_path.as_ref(),
            Some(&AssetPath::parse("skeletons/hero.skeleton"))
        );
        assert_eq!(
            skeleton_output.metadata.cooked_hash,
            Some(skeleton_output.content_hash)
        );
        assert_eq!(skeleton_output.metadata.version_hash, Some(VersionHash(2)));
        assert_eq!(scene_output.bytes, scene_bytes());
        assert_eq!(prefab_output.bytes, prefab_bytes());
        assert_eq!(animation_output.bytes, animation_runtime_bytes());
        assert_eq!(skeleton_output.bytes, skeleton_runtime_bytes());
    } else {
        assert!(matches!(
            database.cook_asset(scene_id, TargetPlatform::Windows),
            Err(AssetError::Cook { message }) if message.contains("no cooker registered")
        ));
        assert!(matches!(
            database.cook_asset(prefab_id, TargetPlatform::Windows),
            Err(AssetError::Cook { message }) if message.contains("no cooker registered")
        ));
        assert!(matches!(
            database.cook_asset(animation_id, TargetPlatform::Windows),
            Err(AssetError::Cook { message }) if message.contains("no cooker registered")
        ));
        assert!(matches!(
            database.cook_asset(skeleton_id, TargetPlatform::Windows),
            Err(AssetError::Cook { message }) if message.contains("no cooker registered")
        ));
    }
}

#[test]
fn bundle_feature_entry_points_match_gate() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let database = AssetDatabase::new(database_config("disabled_bundle_build"));
    let bundle_build = AssetDatabaseBundleBuild::new("empty", Vec::new());
    let bundle_registry_path = database_config("disabled_bundle_registry").registry_path;
    let status = asset_feature_status(AssetFeature::Bundle);
    assert_eq!(status.enabled, asset_feature_enabled(AssetFeature::Bundle));
    assert_eq!(status.name, "bundle");

    if asset_feature_enabled(AssetFeature::Bundle) {
        assert_eq!(require_asset_feature(AssetFeature::Bundle), Ok(()));
        assert!(matches!(
            server.mount_bundle_bytes(b"not a bundle"),
            Err(AssetError::Bundle { .. })
        ));
        let bundle_output = database.build_bundle(&bundle_build).unwrap();
        assert_eq!(bundle_output.asset_count, 0);
        assert_eq!(
            bundle_output.bytes,
            database.build_bundle_bytes(&bundle_build).unwrap()
        );
        let reader = BundleReader::from_bytes(&bundle_output.bytes).unwrap();
        assert_eq!(reader.manifest().name, "empty");
        assert!(reader.manifest().entries.is_empty());

        #[cfg(feature = "bundle")]
        {
            let mounted = server.mount_bundle_bytes(&bundle_output.bytes).unwrap();
            let mounted_path = database_config("enabled_bundle_mounted_registry").registry_path;
            fs::create_dir_all(mounted_path.parent().unwrap()).unwrap();
            server.save_mounted_bundle_registry(&mounted_path).unwrap();
            let mut mounted_server = AssetServer::new(AssetServerConfig::default());
            let restored_mounted = mounted_server
                .load_mounted_bundle_registry(&mounted_path)
                .unwrap();
            assert_eq!(restored_mounted.len(), 1);
            assert_eq!(restored_mounted[0].id, mounted.id);
            assert_eq!(restored_mounted[0].name, "empty");
            assert_eq!(
                mounted_server.mounted_bundle(mounted.id).unwrap().name,
                "empty"
            );

            let package_registry_path =
                database_config("enabled_bundle_package_registry").registry_path;
            fs::create_dir_all(package_registry_path.parent().unwrap()).unwrap();
            server
                .save_asset_package_registry(&package_registry_path)
                .unwrap();
            let mut package_server = AssetServer::new(AssetServerConfig::default());
            let restored_packages = package_server
                .load_asset_package_registry(&package_registry_path)
                .unwrap();
            assert!(restored_packages.is_empty());
            assert!(package_server
                .asset_package_registry()
                .packages()
                .is_empty());
        }
    } else {
        assert_eq!(
            require_asset_feature(AssetFeature::Bundle),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.mount_bundle_bytes(b"not a bundle"),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            database.build_bundle(&bundle_build),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            database.build_bundle_bytes(&bundle_build),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.save_mounted_bundle_registry(&bundle_registry_path),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.load_mounted_bundle_registry(&bundle_registry_path),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.restore_asset_package_registry(AssetPackageRegistry::default()),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.preview_asset_package_update(
                &AssetPackageRegistry::default(),
                AssetPackageUpdatePolicy::default(),
            ),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.activate_asset_package_registry(
                AssetPackageRegistry::default(),
                AssetPackageUpdatePolicy::default(),
            ),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.verify_asset_package_artifacts(
                &AssetPackageRegistry::default(),
                &bundle_registry_path
            ),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.activate_asset_package_registry_from_artifacts(
                AssetPackageRegistry::default(),
                AssetPackageUpdatePolicy::default(),
                &bundle_registry_path,
            ),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.save_asset_package_registry(&bundle_registry_path),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.load_asset_package_registry(&bundle_registry_path),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
        assert_eq!(
            server.unmount_bundle(BundleId(1)),
            Err(AssetError::Unsupported("asset bundle feature is disabled"))
        );
    }
}

#[test]
fn hot_reload_feature_entry_points_match_gate() {
    let path = AssetPath::parse("textures/albedo.texture");
    let io = MemoryAssetIo::new().with_file(path.path(), texture_bytes(1, 1, 10));
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    let status = asset_feature_status(AssetFeature::HotReload);
    assert_eq!(
        status.enabled,
        asset_feature_enabled(AssetFeature::HotReload)
    );
    assert_eq!(status.name, "hot_reload");

    if asset_feature_enabled(AssetFeature::HotReload) {
        assert_eq!(require_asset_feature(AssetFeature::HotReload), Ok(()));
        server.watch_hot_reload_path(path.clone()).unwrap();
        let watch = server.hot_reload_watch(&path).unwrap();
        assert_eq!(watch.backend, HotReloadWatchBackend::PollingMetadata);
        assert_eq!(watch.path, path);
        assert_eq!(watch.last_metadata.size, 12);
        assert_eq!(server.hot_reload_watches().count(), 1);

        let report = server.hot_reload_policy_report();
        assert_eq!(report.watched_paths(), 1);
        assert_eq!(report.queued_changes(), 0);
        assert_eq!(report.watch_backend, HotReloadWatchBackend::PollingMetadata);
        assert_eq!(report.watches.len(), 1);
        assert_eq!(report.watches[0].path, path);
        assert_eq!(report.watch_statuses.len(), 1);
        assert_eq!(report.watch_statuses[0].path, path);
        assert_eq!(
            report.watch_statuses[0].backend,
            HotReloadWatchBackend::PollingMetadata
        );
        assert!(!report.watch_statuses[0].queued);
        assert!(report.watch_statuses[0].last_error.is_none());
        assert_eq!(report.last_poll, HotReloadPollReport::default());
        assert_eq!(
            server.hot_reload_async_watch_report(),
            HotReloadAsyncWatchReport::default()
        );

        assert!(matches!(
            server.queue_hot_reload_id(AssetId::new()),
            Err(AssetError::AssetNotFound { .. })
        ));
        assert!(matches!(
            server.hot_reload_dependency_plan_by_id(
                AssetId::new(),
                HotReloadDependencyPolicy::Direct,
            ),
            Err(AssetError::AssetNotFound { .. })
        ));
    } else {
        assert_eq!(
            require_asset_feature(AssetFeature::HotReload),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.queue_hot_reload_id(AssetId::new()),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.try_queue_hot_reload_path("textures/albedo.texture"),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.watch_hot_reload_path("textures/albedo.texture"),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.watch_hot_reload_path_with_backend(
                "textures/albedo.texture",
                HotReloadWatchBackend::AsyncNotification,
            ),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.start_hot_reload_async_watch_backend(),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.stop_hot_reload_async_watch_backend(),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.notify_hot_reload_async_watch_change("textures/albedo.texture"),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(
            server.poll_hot_reload_watches(),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
        assert_eq!(server.hot_reload_watches().count(), 0);
        assert!(server
            .hot_reload_watch(&AssetPath::parse("textures/albedo.texture"))
            .is_none());
        let hot_reload_report = server.hot_reload_policy_report();
        assert_eq!(hot_reload_report.watched_paths(), 0);
        assert_eq!(hot_reload_report.queued_changes(), 0);
        assert_eq!(
            hot_reload_report.async_watch.lifecycle,
            HotReloadAsyncWatchLifecycle::Stopped
        );
        assert_eq!(
            server.hot_reload_async_watch_report(),
            HotReloadAsyncWatchReport::default()
        );
        assert_eq!(
            server.hot_reload_dependency_plan_by_id(
                AssetId::new(),
                HotReloadDependencyPolicy::Direct,
            ),
            Err(AssetError::Unsupported(
                "asset hot_reload feature is disabled"
            ))
        );
    }
}

#[test]
fn streaming_feature_entry_points_match_gate() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    let _missing_region = StreamingRegionId(42);
    let status = asset_feature_status(AssetFeature::Streaming);
    assert_eq!(
        status.enabled,
        asset_feature_enabled(AssetFeature::Streaming)
    );
    assert_eq!(status.name, "streaming");

    #[cfg(feature = "streaming")]
    {
        assert_eq!(require_asset_feature(AssetFeature::Streaming), Ok(()));
        let region = server
            .register_streaming_region_paths("empty", LoadPriority::Low, &[])
            .unwrap();
        let registered = server.streaming_region(region).unwrap();
        assert_eq!(registered.id, region);
        assert_eq!(registered.name, "empty");
        assert_eq!(registered.priority, LoadPriority::Low);
        assert!(registered.assets.is_empty());
        assert!(!registered.resident);
        assert_eq!(
            server.streaming_region_state(region),
            Ok(AssetLoadState::Ready)
        );
        assert_eq!(
            server.streaming_region_progress(region),
            Ok(LoadProgress::default())
        );
    }

    #[cfg(not(feature = "streaming"))]
    {
        assert_eq!(
            require_asset_feature(AssetFeature::Streaming),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.register_streaming_region_paths("empty", LoadPriority::Low, &[]),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.register_streaming_region_bundle("empty", LoadPriority::Low, BundleId(1)),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.register_streaming_region_bundle_subset(
                "empty",
                LoadPriority::Low,
                BundleId(1),
                &[]
            ),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.set_streaming_region_resident(_missing_region, true),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.set_streaming_region_priority(_missing_region, LoadPriority::Immediate),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert!(matches!(
            server.preload_streaming_region(_missing_region),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        ));
        assert_eq!(
            server.unload_streaming_region(_missing_region),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.streaming_region_progress(_missing_region),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.streaming_region_state(_missing_region),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
        assert_eq!(
            server.remove_streaming_region(_missing_region),
            Err(AssetError::Unsupported(
                "asset streaming feature is disabled"
            ))
        );
    }
}
