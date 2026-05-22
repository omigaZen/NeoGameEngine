use engine_asset::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomAsset {
    source: String,
    dependency_count: usize,
    subresource_count: usize,
    trait_dependencies: Vec<UntypedHandle>,
    ref_dependencies: Vec<AssetRef<Texture>>,
}

impl Asset for CustomAsset {
    const TYPE_NAME: &'static str = "CustomAsset";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x0000_0000_0000_0000_0000_0000_00c0_ffee);
}

impl AssetDependencies for CustomAsset {
    fn visit_dependencies(&self, visitor: &mut dyn FnMut(AssetDependencyReference)) {
        for dependency in &self.trait_dependencies {
            visitor(AssetDependencyReference::from_handle(dependency.clone()));
        }
        for dependency in &self.ref_dependencies {
            dependency.visit_dependency(visitor);
        }
    }
}

struct CustomLoader {
    return_texture: bool,
}

impl CustomLoader {
    fn new() -> Self {
        Self {
            return_texture: false,
        }
    }

    fn mismatched() -> Self {
        Self {
            return_texture: true,
        }
    }
}

impl AssetLoader for CustomLoader {
    fn name(&self) -> &'static str {
        "CustomLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["custom"]
    }

    fn asset_type(&self) -> AssetTypeId {
        CustomAsset::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        if self.return_texture {
            return Ok(LoadedAsset::new(Texture {
                width: 1,
                height: 1,
                format: TextureFormat::Rgba8UnormSrgb,
                mip_count: 1,
                data: vec![255, 255, 255, 255],
                gpu: None,
            }));
        }

        let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Decode {
            message: error.to_string(),
        })?;
        let mut dependency_count = 0;
        let mut subresource_count = 0;
        let mut trait_dependencies = Vec::new();
        let mut ref_dependencies = Vec::new();
        for line in source
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            if let Some(path) = line.strip_prefix("dependency=") {
                ctx.add_dependency(AssetPath::parse(path), AssetTypeId::of::<Texture>());
                dependency_count += 1;
            } else if let Some(path) = line.strip_prefix("subresource=") {
                ctx.add_subresource(AssetPath::parse(path), CustomAsset::TYPE_ID);
                subresource_count += 1;
            } else if let Some(id) = line.strip_prefix("trait_dependency=") {
                let id = id.parse::<u128>().map_err(|error| AssetError::Decode {
                    message: format!("invalid trait dependency id: {error}"),
                })?;
                trait_dependencies.push(UntypedHandle::new(
                    AssetId::from_u128(id),
                    AssetTypeId::of::<Texture>(),
                    HandleStrength::Weak,
                ));
            } else if let Some(value) = line.strip_prefix("ref_dependency=") {
                let (id, path) = value.split_once('|').ok_or_else(|| AssetError::Decode {
                    message: "ref_dependency must be formatted as `<id>|<path>`".to_owned(),
                })?;
                let id = id.parse::<u128>().map_err(|error| AssetError::Decode {
                    message: format!("invalid ref dependency id: {error}"),
                })?;
                ref_dependencies.push(AssetRef::with_fallback(
                    AssetId::from_u128(id),
                    AssetPath::parse(path),
                ));
            }
        }
        Ok(LoadedAsset::new_with_asset_dependencies(CustomAsset {
            source: source.to_owned(),
            dependency_count,
            subresource_count,
            trait_dependencies,
            ref_dependencies,
        }))
    }
}

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
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
fn custom_loader_registers_dependencies_subresources_and_type_fallback() {
    let custom_path = AssetPath::parse("custom/hero.custom");
    let by_type_path = AssetPath::parse("custom/by_type.blob");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let subresource_path = AssetPath::parse("custom/hero.custom#preview");
    let mut io = MemoryAssetIo::new();
    io.insert(
        custom_path.path(),
        "dependency=textures/albedo.texture\nsubresource=custom/hero.custom#preview\n",
    );
    io.insert(by_type_path.path(), "type fallback");
    io.insert(texture_path.path(), texture_bytes(1, 1, 9));
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    server.register_loader(CustomLoader::new());

    let custom: Handle<CustomAsset> = server.load(custom_path.clone());
    server.update_loading();
    assert_eq!(
        server.state(&custom),
        AssetLoadState::WaitingForDependencies
    );
    let texture_id = server.id_from_path(&texture_path).unwrap();
    assert_eq!(
        server.dependency_graph().direct_dependencies(custom.id()),
        &[texture_id]
    );
    assert_eq!(
        server.metadata(custom.id()).unwrap().dependencies,
        vec![texture_id]
    );
    let subresource_id = server.id_from_path(&subresource_path).unwrap();
    assert_eq!(
        server.metadata(subresource_id).unwrap().asset_type,
        CustomAsset::TYPE_ID
    );

    server.update_loading();
    finish_uploads(&mut server);
    assert!(server.is_ready(&custom));
    let loaded = server.get(&custom).unwrap();
    assert_eq!(loaded.dependency_count, 1);
    assert_eq!(loaded.subresource_count, 1);

    let by_type_id = AssetId::new();
    server.registry_mut().insert(AssetMetadata::runtime(
        by_type_id,
        by_type_path,
        CustomAsset::TYPE_ID,
    ));
    let by_type: Handle<CustomAsset> = server.load_by_id(by_type_id);
    server.update_loading();
    assert!(server.is_ready(&by_type));
    assert_eq!(server.get(&by_type).unwrap().source, "type fallback");
}

#[test]
fn custom_asset_dependencies_trait_drives_dependency_graph_and_recursive_load() {
    let custom_path = AssetPath::parse("custom/trait_dependencies.custom");
    let texture_path = AssetPath::parse("textures/from_trait.texture");
    let texture_id = AssetId::new();
    let mut io = MemoryAssetIo::new();
    io.insert(
        custom_path.path(),
        format!("trait_dependency={}\n", texture_id.raw()),
    );
    io.insert(texture_path.path(), texture_bytes(1, 1, 55));
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        max_cpu_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();
    server.register_loader(CustomLoader::new());
    server.registry_mut().insert(AssetMetadata::runtime(
        texture_id,
        texture_path.clone(),
        Texture::TYPE_ID,
    ));

    let custom: Handle<CustomAsset> = server.load(custom_path);
    server.update_loading();

    assert_eq!(
        server.state(&custom),
        AssetLoadState::WaitingForDependencies
    );
    assert_eq!(
        server.dependency_graph().direct_dependencies(custom.id()),
        &[texture_id]
    );
    assert_eq!(
        server.metadata(custom.id()).unwrap().dependencies,
        vec![texture_id]
    );

    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert_eq!(server.state(&custom), AssetLoadState::Ready);
    let loaded = server.get(&custom).unwrap();
    let mut visited = Vec::new();
    loaded.visit_dependencies(&mut |dependency| visited.push(dependency.id()));
    assert_eq!(visited, vec![texture_id]);
}

#[test]
fn custom_asset_ref_dependencies_register_fallback_paths_and_recursive_load() {
    let custom_path = AssetPath::parse("custom/ref_dependencies.custom");
    let texture_path = AssetPath::parse("textures/from_ref.texture");
    let texture_id = AssetId::new();
    let mut io = MemoryAssetIo::new();
    io.insert(
        custom_path.path(),
        format!(
            "ref_dependency={}|{}\n",
            texture_id.raw(),
            texture_path.display_string()
        ),
    );
    io.insert(texture_path.path(), texture_bytes(1, 1, 77));
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        max_cpu_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();
    server.register_loader(CustomLoader::new());

    let custom: Handle<CustomAsset> = server.load(custom_path);
    server.update_loading();

    assert_eq!(
        server.state(&custom),
        AssetLoadState::WaitingForDependencies
    );
    assert_eq!(
        server.dependency_graph().direct_dependencies(custom.id()),
        &[texture_id]
    );
    assert_eq!(server.path_from_id(texture_id), Some(&texture_path));

    server.update_loading();
    finish_uploads(&mut server);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert_eq!(server.state(&custom), AssetLoadState::Ready);
    assert_eq!(
        server.memory_info(texture_id).unwrap().dependency_ref_count,
        1
    );

    let loaded = server.get(&custom).unwrap();
    let mut visited = Vec::new();
    loaded.visit_dependencies(&mut |dependency| {
        visited.push((dependency.id(), dependency.fallback_path().cloned()))
    });
    assert_eq!(visited, vec![(texture_id, Some(texture_path))]);
}

#[test]
fn custom_asset_dependencies_trait_failure_propagates_to_waiting_asset() {
    let custom_path = AssetPath::parse("custom/missing_trait_dependency.custom");
    let texture_path = AssetPath::parse("textures/missing_from_trait.texture");
    let texture_id = AssetId::new();
    let io = MemoryAssetIo::new().with_file(
        custom_path.path(),
        format!("trait_dependency={}\n", texture_id.raw()),
    );
    let mut server = AssetServer::new(AssetServerConfig {
        max_io_jobs_per_frame: 1,
        max_cpu_jobs_per_frame: 1,
        ..AssetServerConfig::default()
    });
    server.set_io(io);
    server.register_builtin_loaders();
    server.register_loader(CustomLoader::new());
    server.registry_mut().insert(AssetMetadata::runtime(
        texture_id,
        texture_path,
        Texture::TYPE_ID,
    ));

    let custom: Handle<CustomAsset> = server.load(custom_path);
    server.update_loading();
    assert_eq!(
        server.state(&custom),
        AssetLoadState::WaitingForDependencies
    );

    server.update_loading();
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Failed);
    assert_eq!(server.state(&custom), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(custom.id()),
        Some(AssetError::DependencyFailed { dependency, .. }) if *dependency == texture_id
    ));
}

#[test]
fn custom_asset_dependencies_trait_unknown_id_fails_without_indefinite_wait() {
    let custom_path = AssetPath::parse("custom/unknown_trait_dependency.custom");
    let texture_id = AssetId::from_u128(0xfeed_face);
    let io = MemoryAssetIo::new().with_file(
        custom_path.path(),
        format!("trait_dependency={}\n", texture_id.raw()),
    );
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    server.register_loader(CustomLoader::new());

    let custom: Handle<CustomAsset> = server.load(custom_path);
    server.update_loading();

    assert_eq!(
        server.dependency_graph().direct_dependencies(custom.id()),
        &[texture_id]
    );
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(texture_id),
        Some(AssetError::AssetNotFound { id }) if *id == texture_id
    ));
    assert_eq!(server.state(&custom), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(custom.id()),
        Some(AssetError::DependencyFailed { asset, dependency })
            if *asset == custom.id() && *dependency == texture_id
    ));
    assert!(server.events().iter().any(|event| {
        matches!(
            event,
            AssetEvent::DependencyFailed {
                id,
                dependency,
                error: AssetError::AssetNotFound { id: missing }
            } if *id == custom.id() && *dependency == texture_id && *missing == texture_id
        )
    }));
}

#[test]
fn custom_loader_type_mismatch_fails_visibly() {
    let path = AssetPath::parse("custom/mismatch.custom");
    let io = MemoryAssetIo::new().with_file(path.path(), "mismatch");
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_loader(CustomLoader::mismatched());
    let handle: Handle<CustomAsset> = server.load(path);

    server.update_loading();

    assert_eq!(server.state(&handle), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(handle.id()),
        Some(AssetError::TypeMismatch { expected, actual })
            if expected == "CustomAsset" && actual == "Texture"
    ));
}

#[test]
fn missing_loader_diagnostics_cover_extension_and_type_paths() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(MemoryAssetIo::new().with_file("custom/missing.blob", "payload"));
    let untyped = server.load_untyped("custom/missing.unknown");
    assert_eq!(server.state_by_id(untyped.id()), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(untyped.id()),
        Some(AssetError::LoaderNotFound { extension }) if extension == "unknown"
    ));

    let id = AssetId::new();
    server.registry_mut().insert(AssetMetadata::runtime(
        id,
        AssetPath::parse("custom/missing.blob"),
        CustomAsset::TYPE_ID,
    ));
    let typed: Handle<CustomAsset> = server.load_by_id(id);
    server.update_loading();

    assert_eq!(server.state(&typed), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(id),
        Some(AssetError::LoaderForTypeNotFound { asset_type })
            if *asset_type == CustomAsset::TYPE_ID
    ));
}
