use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn server_with_config(config: AssetServerConfig, files: &[(&str, Vec<u8>)]) -> AssetServer {
    let mut io = MemoryAssetIo::new();
    for (path, bytes) in files {
        io.insert(*path, bytes.clone());
    }
    let mut server = AssetServer::new(config);
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
fn strong_handle_protects_ready_asset_from_unused_gc() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1;
    let mut server =
        server_with_config(config, &[("textures/held.texture", texture_bytes(1, 1, 1))]);
    let texture: Handle<Texture> = server.load("textures/held.texture");
    server.update_loading();
    finish_uploads(&mut server);

    server.update_gc(10);

    assert_eq!(server.state(&texture), AssetLoadState::Ready);
    let entry = server
        .storage::<Texture>()
        .unwrap()
        .entry(texture.id())
        .unwrap();
    assert_eq!(entry.strong_count, 1);
    assert_eq!(entry.weak_count, 0);
}

#[test]
fn cloned_and_converted_handles_update_lifecycle_counts_on_drop() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 100;
    let mut server = server_with_config(
        config,
        &[("textures/lifecycle.texture", texture_bytes(1, 1, 7))],
    );
    let texture: Handle<Texture> = server.load("textures/lifecycle.texture");
    let strong_clone = texture.clone();
    let weak_clone = texture.clone_weak();
    let untyped = texture.untyped();
    server.update_loading();
    finish_uploads(&mut server);

    server.update_gc(0);
    let entry = server
        .storage::<Texture>()
        .unwrap()
        .entry(texture.id())
        .unwrap();
    assert_eq!(entry.strong_count, 3);
    assert_eq!(entry.weak_count, 1);

    drop(strong_clone);
    drop(untyped);
    server.update_gc(1);
    let entry = server
        .storage::<Texture>()
        .unwrap()
        .entry(texture.id())
        .unwrap();
    assert_eq!(entry.strong_count, 1);
    assert_eq!(entry.weak_count, 1);

    let promoted = weak_clone.clone_strong();
    server.update_gc(2);
    let entry = server
        .storage::<Texture>()
        .unwrap()
        .entry(texture.id())
        .unwrap();
    assert_eq!(entry.strong_count, 2);
    assert_eq!(entry.weak_count, 1);

    let id = texture.id();
    drop(texture);
    drop(promoted);
    server.update_gc(3);
    let entry = server.storage::<Texture>().unwrap().entry(id).unwrap();
    assert_eq!(entry.strong_count, 0);
    assert_eq!(entry.weak_count, 1);

    drop(weak_clone);
    server.update_gc(4);
    let entry = server.storage::<Texture>().unwrap().entry(id).unwrap();
    assert_eq!(entry.strong_count, 0);
    assert_eq!(entry.weak_count, 0);
}

#[test]
fn promoted_preload_handle_protects_asset_until_last_strong_drop() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1;
    let mut server = server_with_config(
        config,
        &[("textures/preload.texture", texture_bytes(1, 1, 8))],
    );
    let weak: Handle<Texture> = server.preload("textures/preload.texture");
    assert!(weak.is_weak());
    let strong = weak.clone_strong();
    let id = weak.id();
    server.update_loading();
    finish_uploads(&mut server);

    server.update_gc(0);
    let entry = server.storage::<Texture>().unwrap().entry(id).unwrap();
    assert_eq!(entry.strong_count, 1);
    assert_eq!(entry.weak_count, 1);
    assert_eq!(server.state_by_id(id), AssetLoadState::Ready);

    drop(strong);
    server.update_gc(2);

    assert_eq!(server.state_by_id(id), AssetLoadState::Unloaded);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Unloaded { id: event_id } if *event_id == id)));
}

#[test]
fn weak_only_asset_unloads_after_unused_window() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1;
    let mut server =
        server_with_config(config, &[("textures/weak.texture", texture_bytes(1, 1, 2))]);
    let texture: Handle<Texture> = server.load("textures/weak.texture");
    let id = texture.id();
    server.update_loading();
    finish_uploads(&mut server);
    server.update_gc(0);
    drop(texture);

    server.update_gc(2);

    assert_eq!(server.state_by_id(id), AssetLoadState::Unloaded);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Unloaded { id: event_id } if *event_id == id)));
}

#[test]
fn dependency_reference_protects_texture_from_unused_gc() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1;
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 3));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let material: Handle<Material> = server.load("materials/hero.material");
    server.update_loading();
    finish_uploads(&mut server);
    finish_uploads(&mut server);
    assert!(server.is_ready(&material));
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    server.update_gc(10);

    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    let entry = server
        .storage::<Texture>()
        .unwrap()
        .entry(texture_id)
        .unwrap();
    assert!(entry.dependency_ref_count > 0);
}

#[test]
fn unloading_dependent_releases_dependency_reference_for_gc() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1;
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 9));
    io.insert(
        "materials/hero.material",
        "name=hero\ntexture.albedo=textures/albedo.texture\n",
    );
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let material: Handle<Material> = server.load("materials/hero.material");
    server.update_loading();
    finish_uploads(&mut server);
    finish_uploads(&mut server);
    let material_id = material.id();
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    server.update_gc(10);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert!(
        server
            .storage::<Texture>()
            .unwrap()
            .entry(texture_id)
            .unwrap()
            .dependency_ref_count
            > 0
    );

    server.unload_by_id(material_id).unwrap();
    assert!(server
        .dependency_graph()
        .direct_dependencies(material_id)
        .is_empty());
    assert_eq!(
        server
            .storage::<Texture>()
            .unwrap()
            .entry(texture_id)
            .unwrap()
            .dependency_ref_count,
        0
    );

    server.update_gc(12);
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Unloaded);
}

#[test]
fn resident_asset_is_protected_from_gc_until_residency_is_cleared() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1;
    let mut server = server_with_config(
        config,
        &[("textures/resident.texture", texture_bytes(1, 1, 4))],
    );
    let texture: Handle<Texture> = server.load("textures/resident.texture");
    let id = texture.id();
    server.update_loading();
    finish_uploads(&mut server);
    server.set_asset_resident(id, true);
    drop(texture);

    server.update_gc(10);
    assert_eq!(server.state_by_id(id), AssetLoadState::Ready);

    server.set_asset_resident(id, false);
    server.update_gc(12);
    assert_eq!(server.state_by_id(id), AssetLoadState::Unloaded);
}

#[test]
fn memory_report_exposes_asset_type_counts_residency_and_dependency_refs() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 100;
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 3));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();
    let material: Handle<Material> = server.load("materials/hero.material");
    let material_weak = material.clone_weak();
    for _ in 0..4 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.is_ready(&material) {
            break;
        }
    }
    assert!(server.is_ready(&material));
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    server.set_asset_resident(texture_id, true);
    server.update_gc(7);

    let material_info = server.memory_info(material.id()).unwrap();
    assert_eq!(material_info.asset_type, Material::TYPE_ID);
    assert_eq!(material_info.state, AssetLoadState::Ready);
    assert_eq!(material_info.strong_count, 1);
    assert_eq!(material_info.weak_count, 1);
    assert_eq!(material_info.last_used_frame, 7);
    assert!(material_info.cpu_bytes > 0);

    let texture_info = server.memory_info(texture_id).unwrap();
    assert_eq!(texture_info.asset_type, Texture::TYPE_ID);
    assert_eq!(texture_info.state, AssetLoadState::Ready);
    assert_eq!(texture_info.cpu_bytes, 4);
    assert_eq!(texture_info.gpu_bytes, 4);
    assert!(texture_info.dependency_ref_count > 0);
    assert!(texture_info.resident);

    let report = server.memory_report();
    assert_eq!(report.asset_count, 3);
    assert_eq!(report.assets.len(), report.asset_count);
    assert_eq!(
        report.total_cpu_bytes,
        report.assets.iter().map(|info| info.cpu_bytes).sum()
    );
    assert_eq!(
        report.total_gpu_bytes,
        report.assets.iter().map(|info| info.gpu_bytes).sum()
    );
    let texture_type = report
        .by_type
        .iter()
        .find(|entry| entry.asset_type == Texture::TYPE_ID)
        .unwrap();
    assert_eq!(texture_type.asset_count, 1);
    assert_eq!(texture_type.cpu_bytes, texture_info.cpu_bytes);
    assert_eq!(texture_type.gpu_bytes, texture_info.gpu_bytes);
    assert_eq!(texture_type.resident_assets, 1);
    assert!(texture_type.dependency_ref_count > 0);
    let material_type = report
        .by_type
        .iter()
        .find(|entry| entry.asset_type == Material::TYPE_ID)
        .unwrap();
    assert_eq!(material_type.strong_count, 1);
    assert_eq!(material_type.weak_count, 1);
    drop(material_weak);
}

#[test]
fn memory_budget_evicts_oldest_unprotected_assets_first() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1000;
    let mut server = server_with_config(
        config,
        &[
            ("textures/old.texture", texture_bytes(1, 1, 5)),
            ("textures/new.texture", texture_bytes(1, 1, 6)),
        ],
    );
    let old: Handle<Texture> = server.load("textures/old.texture");
    server.update_loading();
    finish_uploads(&mut server);
    let old_id = old.id();
    server
        .storage_mut::<Texture>()
        .unwrap()
        .mark_used(old_id, 1);
    drop(old);

    let new: Handle<Texture> = server.load("textures/new.texture");
    server.update_loading();
    finish_uploads(&mut server);
    let new_id = new.id();
    server
        .storage_mut::<Texture>()
        .unwrap()
        .mark_used(new_id, 2);
    drop(new);

    server.config_mut().gc.memory_budget_bytes = Some(8);
    server.collect_until_budget();

    assert_eq!(server.state_by_id(old_id), AssetLoadState::Unloaded);
    assert_eq!(server.state_by_id(new_id), AssetLoadState::Ready);
    let report = server.memory_report();
    assert_eq!(report.asset_count, 1);
    assert!(report.assets.iter().any(|info| info.id == new_id));
    assert!(report.total_cpu_bytes + report.total_gpu_bytes <= 8);
    let stats = server.memory_stats();
    assert_eq!(stats.assets, report.asset_count);
    assert_eq!(stats.cpu_bytes, report.total_cpu_bytes);
    assert_eq!(stats.gpu_bytes, report.total_gpu_bytes);
}

#[test]
fn per_type_memory_budget_evicts_oldest_asset_without_touching_other_types() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1000;
    config
        .gc
        .type_memory_budgets
        .push(AssetTypeMemoryBudget::total(Texture::TYPE_ID, 8));
    let mut server = server_with_config(
        config,
        &[
            ("textures/old.texture", texture_bytes(1, 1, 5)),
            ("textures/new.texture", texture_bytes(1, 1, 6)),
            ("shaders/keep.wgsl", b"@fragment fn main() {}".to_vec()),
        ],
    );
    let old: Handle<Texture> = server.load("textures/old.texture");
    server.update_loading();
    finish_uploads(&mut server);
    let old_id = old.id();
    server
        .storage_mut::<Texture>()
        .unwrap()
        .mark_used(old_id, 1);
    drop(old);

    let new: Handle<Texture> = server.load("textures/new.texture");
    server.update_loading();
    finish_uploads(&mut server);
    let new_id = new.id();
    server
        .storage_mut::<Texture>()
        .unwrap()
        .mark_used(new_id, 2);
    drop(new);

    let shader: Handle<Shader> = server.load("shaders/keep.wgsl");
    server.update_loading();
    finish_uploads(&mut server);
    let shader_id = shader.id();
    drop(shader);

    server.collect_until_budget();

    assert_eq!(server.state_by_id(old_id), AssetLoadState::Unloaded);
    assert_eq!(server.state_by_id(new_id), AssetLoadState::Ready);
    assert_eq!(server.state_by_id(shader_id), AssetLoadState::Ready);
    let report = server.memory_report();
    let texture_type = report
        .by_type
        .iter()
        .find(|entry| entry.asset_type == Texture::TYPE_ID)
        .unwrap();
    assert_eq!(texture_type.asset_count, 1);
    assert!(texture_type.cpu_bytes + texture_type.gpu_bytes <= 8);
    assert!(report.assets.iter().any(|info| info.id == shader_id));
}

#[test]
fn per_type_memory_budget_respects_strong_dependency_and_resident_protection() {
    let mut config = AssetServerConfig::default();
    config.gc.unload_after_unused_frames = 1000;
    config
        .gc
        .type_memory_budgets
        .push(AssetTypeMemoryBudget::total(Texture::TYPE_ID, 0));
    let mut io = MemoryAssetIo::new();
    io.insert("textures/held.texture", texture_bytes(1, 1, 7));
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 8));
    io.insert(
        "materials/hero.material",
        "name=hero\ntexture.albedo=textures/albedo.texture\n",
    );
    let mut server = AssetServer::new(config);
    server.set_io(io);
    server.register_builtin_loaders();

    let held: Handle<Texture> = server.load("textures/held.texture");
    server.update_loading();
    finish_uploads(&mut server);
    let held_id = held.id();
    server.collect_until_budget();
    assert_eq!(server.state_by_id(held_id), AssetLoadState::Ready);
    drop(held);
    server.collect_until_budget();
    assert_eq!(server.state_by_id(held_id), AssetLoadState::Unloaded);

    let material: Handle<Material> = server.load("materials/hero.material");
    for _ in 0..4 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.is_ready(&material) {
            break;
        }
    }
    let material_id = material.id();
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    server.collect_until_budget();
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert!(server.memory_info(texture_id).unwrap().dependency_ref_count > 0);

    server.unload_by_id(material_id).unwrap();
    server.set_asset_resident(texture_id, true);
    server.collect_until_budget();
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Ready);
    assert!(server.memory_info(texture_id).unwrap().resident);

    server.set_asset_resident(texture_id, false);
    server.collect_until_budget();
    assert_eq!(server.state_by_id(texture_id), AssetLoadState::Unloaded);
}
