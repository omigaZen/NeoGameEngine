use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use engine_asset::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmokeReport {
    pub render_ready: bool,
    pub audio_ready: bool,
    pub physics_ready: bool,
    pub material_ready_with_dependencies: bool,
    pub group_ready: bool,
    pub group_total_assets: usize,
    pub group_ready_assets: usize,
    pub material_dependencies: usize,
    pub render_handles: usize,
    pub audio_handles: usize,
    pub physics_handles: usize,
    pub events: usize,
    pub ready_events: usize,
    pub dependency_events: usize,
    pub failed_events: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorSmokeReport {
    pub scanned_sources: usize,
    pub imported_assets: usize,
    pub cooked_assets: usize,
    pub bundled_assets: usize,
    pub bundle_group_ready: bool,
    pub material_ready_with_dependencies: bool,
    pub runtime_dependencies: usize,
    pub ready_events: usize,
    pub failed_events: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSmokeReport {
    pub generated_subresources: usize,
    pub bundled_assets: usize,
    pub bundle_group_ready: bool,
    pub mesh_ready: bool,
    pub material_ready_with_dependencies: bool,
    pub skeleton_ready: bool,
    pub animation_ready: bool,
    pub physics_ready: bool,
    pub material_dependencies: usize,
    pub skeleton_root: Option<String>,
    pub animation_target: Option<String>,
    pub physics_vertices: usize,
}

pub fn run_smoke() -> SmokeReport {
    let mut io = MemoryAssetIo::new();
    io.insert("meshes/tri.mesh", mesh_bytes());
    io.insert("textures/checker.texture", texture_bytes(2, 2));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/checker.texture\nbase_color=1,1,1,1\n",
    );
    io.insert("audio/click.audio", audio_bytes());
    io.insert("physics/hero.physics", physics_mesh_bytes());

    let mut assets = AssetServer::new(AssetServerConfig::default());
    assets.set_io(io);
    assets.register_builtin_loaders();

    let group = assets.load_group(&[
        AssetPath::parse("meshes/tri.mesh"),
        AssetPath::parse("materials/hero.material"),
        AssetPath::parse("audio/click.audio"),
        AssetPath::parse("physics/hero.physics"),
    ]);
    let renderer = MeshRendererComponent {
        mesh: assets.load("meshes/tri.mesh"),
        material: assets.load("materials/hero.material"),
    };
    let audio = AudioSourceComponent {
        clip: assets.load("audio/click.audio"),
        looping: false,
        volume: 0.75,
    };
    let physics = PhysicsColliderComponent {
        mesh: assets.load("physics/hero.physics"),
        dynamic: true,
    };

    let mut cursor = AssetEventCursor::default();
    let mut ready_events = 0;
    let mut dependency_events = 0;
    let mut failed_events = 0;
    for frame in 1..=8 {
        assets.update(frame);
        finish_uploads(&mut assets);
        for event in assets.events_since(&mut cursor) {
            match event {
                AssetEvent::Ready { .. } => ready_events += 1,
                AssetEvent::DependencyReady { .. } => dependency_events += 1,
                AssetEvent::Failed { .. } | AssetEvent::DependencyFailed { .. } => {
                    failed_events += 1
                }
                _ => {}
            }
        }
        if renderer.is_ready(&assets)
            && audio.is_ready(&assets)
            && physics.is_ready(&assets)
            && assets.group_state(&group) == AssetLoadState::Ready
        {
            break;
        }
    }

    let group_progress = assets.group_progress(&group);
    SmokeReport {
        render_ready: renderer.is_ready(&assets),
        audio_ready: audio.is_ready(&assets),
        physics_ready: physics.is_ready(&assets),
        material_ready_with_dependencies: assets.is_ready_with_dependencies(&renderer.material),
        group_ready: assets.group_state(&group) == AssetLoadState::Ready,
        group_total_assets: group_progress.total_assets,
        group_ready_assets: group_progress.ready_assets,
        material_dependencies: assets
            .dependency_graph()
            .direct_dependencies(renderer.material.id())
            .len(),
        render_handles: renderer.asset_handles().len(),
        audio_handles: audio.asset_handles().len(),
        physics_handles: physics.asset_handles().len(),
        events: assets.events().len(),
        ready_events,
        dependency_events,
        failed_events,
    }
}

pub fn run_editor_smoke() -> EditorSmokeReport {
    let root = smoke_temp_root("editor");
    let texture_path = AssetPath::parse("textures/editor.texture");
    let material_path = AssetPath::parse("materials/editor.material");
    let material_source =
        b"name=editor\ntexture.albedo=textures/editor.texture\nbase_color=0.25,0.5,1,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(texture_path.path(), texture_bytes(1, 1));
    io.insert(material_path.path(), material_source);

    let mut database = AssetDatabase::new(AssetDatabaseConfig {
        source_root: root.join("source"),
        imported_root: root.join("imported"),
        cooked_root: root.join("cooked"),
        registry_path: root.join("registry.txt"),
    });
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let scanned_sources = database.scan().unwrap().len();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    database
        .cook_asset(texture_id, TargetPlatform::Windows)
        .unwrap();
    database
        .cook_asset(material_id, TargetPlatform::Windows)
        .unwrap();
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "editor_smoke",
            vec![material_id, texture_id],
        ))
        .unwrap();

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut assets = AssetServer::new(AssetServerConfig::default());
    assets.set_io(bundle_io);
    assets.register_builtin_loaders();
    let mounted = assets.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = assets.preload_bundle(&mounted);
    let material: Handle<Material> = assets.load(material_path);
    let mut cursor = AssetEventCursor::default();
    let mut ready_events = 0;
    let mut failed_events = 0;

    for _ in 0..8 {
        assets.update_loading();
        finish_uploads(&mut assets);
        for event in assets.events_since(&mut cursor) {
            match event {
                AssetEvent::Ready { .. } => ready_events += 1,
                AssetEvent::Failed { .. } | AssetEvent::DependencyFailed { .. } => {
                    failed_events += 1
                }
                _ => {}
            }
        }
        if assets.group_state(&group) == AssetLoadState::Ready
            && assets.is_ready_with_dependencies(&material)
        {
            break;
        }
    }

    let report = EditorSmokeReport {
        scanned_sources,
        imported_assets: 2,
        cooked_assets: 2,
        bundled_assets: bundle.asset_count,
        bundle_group_ready: assets.group_state(&group) == AssetLoadState::Ready,
        material_ready_with_dependencies: assets.is_ready_with_dependencies(&material),
        runtime_dependencies: assets
            .dependency_graph()
            .direct_dependencies(material.id())
            .len(),
        ready_events,
        failed_events,
    };
    let _ = fs::remove_dir_all(root);
    report
}

pub fn run_model_smoke() -> ModelSmokeReport {
    let root = smoke_temp_root("model");
    let shader_path = AssetPath::parse("shaders/pbr.wgsl");
    let texture_path = AssetPath::parse("textures/albedo.texture");
    let model_path = AssetPath::parse("models/hero.model");
    let mesh_path = AssetPath::parse("models/hero.Mesh0.mesh");
    let material_path = AssetPath::parse("models/hero.Material_Hero.material");
    let skeleton_path = AssetPath::parse("models/hero.Skeleton.skeleton");
    let animation_path = AssetPath::parse("models/hero.Animation_Idle.animation");
    let physics_path = AssetPath::parse("models/hero.Collision.physics");
    let mut io = MemoryAssetIo::new();
    io.insert(shader_path.path(), shader_bytes());
    io.insert(texture_path.path(), texture_bytes(1, 1));
    io.insert(model_path.path(), model_manifest_bytes());

    let mut database = AssetDatabase::new(AssetDatabaseConfig {
        source_root: root.join("source"),
        imported_root: root.join("imported"),
        cooked_root: root.join("cooked"),
        registry_path: root.join("registry.txt"),
    });
    database.set_io(io);
    database.register_builtin_importers();
    database.register_builtin_cookers();

    let shader_id = database.import_asset_path(&shader_path).unwrap();
    let texture_id = database.import_asset_path(&texture_path).unwrap();
    database.import_asset_path(&model_path).unwrap();
    let mesh_id = database.registry().metadata_by_path(&mesh_path).unwrap().id;
    let material_id = database
        .registry()
        .metadata_by_path(&material_path)
        .unwrap()
        .id;
    let skeleton_id = database
        .registry()
        .metadata_by_path(&skeleton_path)
        .unwrap()
        .id;
    let animation_id = database
        .registry()
        .metadata_by_path(&animation_path)
        .unwrap()
        .id;
    let physics_id = database
        .registry()
        .metadata_by_path(&physics_path)
        .unwrap()
        .id;
    for id in [
        shader_id,
        texture_id,
        mesh_id,
        material_id,
        skeleton_id,
        animation_id,
        physics_id,
    ] {
        database.cook_asset(id, TargetPlatform::Windows).unwrap();
    }
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "model_smoke",
            vec![
                mesh_id,
                material_id,
                skeleton_id,
                animation_id,
                physics_id,
                shader_id,
                texture_id,
            ],
        ))
        .unwrap();

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut assets = AssetServer::new(AssetServerConfig::default());
    assets.set_io(bundle_io);
    assets.register_builtin_loaders();
    let mounted = assets.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = assets.preload_bundle(&mounted);
    for _ in 0..8 {
        assets.update_loading();
        finish_uploads(&mut assets);
        if assets.group_state(&group) == AssetLoadState::Ready {
            break;
        }
    }

    let animation_target = assets
        .get_by_id::<AnimationClip>(animation_id)
        .and_then(|animation| animation.tracks.first())
        .map(|track| match &track.target {
            AnimationTarget::NodeName(name) => name.clone(),
            AnimationTarget::NodeIndex(index) => index.to_string(),
            AnimationTarget::BoneName(name) => name.clone(),
        });
    let physics_vertices = assets
        .get_by_id::<PhysicsMesh>(physics_id)
        .map(|physics| physics.vertices.len())
        .unwrap_or_default();
    let report = ModelSmokeReport {
        generated_subresources: 5,
        bundled_assets: bundle.asset_count,
        bundle_group_ready: assets.group_state(&group) == AssetLoadState::Ready,
        mesh_ready: assets.state_by_id(mesh_id) == AssetLoadState::Ready,
        material_ready_with_dependencies: assets
            .is_ready_with_dependencies(&Handle::<Material>::strong(material_id)),
        skeleton_ready: assets.state_by_id(skeleton_id) == AssetLoadState::Ready,
        animation_ready: assets.state_by_id(animation_id) == AssetLoadState::Ready,
        physics_ready: assets.state_by_id(physics_id) == AssetLoadState::Ready,
        material_dependencies: assets
            .dependency_graph()
            .direct_dependencies(material_id)
            .len(),
        skeleton_root: assets
            .get_by_id::<Skeleton>(skeleton_id)
            .and_then(|skeleton| skeleton.bones.first())
            .map(|bone| bone.name.clone()),
        animation_target,
        physics_vertices,
    };
    let _ = fs::remove_dir_all(root);
    report
}

fn finish_uploads(assets: &mut AssetServer) {
    let uploads = assets.drain_gpu_uploads().collect::<Vec<_>>();
    assets.finish_gpu_uploads(uploads.into_iter().enumerate().map(|(index, upload)| {
        GpuUploadResult::ok(upload.id, GpuResourceHandle(index as u64 + 1))
    }));
}

fn texture_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(255).take(width as usize * height as usize * 4));
    bytes
}

fn mesh_bytes() -> Vec<u8> {
    b"v 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn shader_bytes() -> Vec<u8> {
    b"@fragment fn main() {}\n".to_vec()
}

fn model_manifest_bytes() -> Vec<u8> {
    b"NGA_MODEL_V1\nmesh=Mesh0|v 0 0 0;v 1 0 0;v 0 1 0;i 0 1 2\nmaterial=Material/Hero|name=hero;shader=shaders/pbr.wgsl;texture.albedo=textures/albedo.texture;base_color=1,1,1,1\nskeleton=Skeleton|NGA_SKELETON_V1;bone=Root\nanimation=Animation/Idle|NGA_ANIMATION_V1;duration=1;ticks_per_second=60;track=bone:Root;translation=0:0,0,0;rotation=0:0,0,0,1;scale=0:1,1,1\nphysics_mesh=Collision|NGA_PHYSICS_MESH_V1;kind=trimesh;v 0 0 0;v 1 0 0;v 0 1 0;i 0 1 2\n".to_vec()
}

fn audio_bytes() -> Vec<u8> {
    b"NGA_AUDIO_V1\nsample_rate=48000\nchannels=2\nformat=i16\nsamples=0,1000,-1000,0\nstreaming=false\n"
        .to_vec()
}

fn physics_mesh_bytes() -> Vec<u8> {
    b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn smoke_temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let root = std::env::temp_dir().join(format!(
        "neo_asset_smoke_{label}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_consumers_observe_ready_assets_without_owning_resources() {
        let report = run_smoke();

        assert!(report.render_ready);
        assert!(report.audio_ready);
        assert!(report.physics_ready);
        assert!(report.material_ready_with_dependencies);
        assert!(report.group_ready);
        assert_eq!(report.group_total_assets, 4);
        assert_eq!(report.group_ready_assets, report.group_total_assets);
        assert_eq!(report.material_dependencies, 2);
        assert_eq!(report.render_handles, 2);
        assert_eq!(report.audio_handles, 1);
        assert_eq!(report.physics_handles, 1);
        assert!(report.events >= 6);
        assert!(report.ready_events >= 6);
        assert!(report.dependency_events >= 2);
        assert_eq!(report.failed_events, 0);
    }

    #[test]
    fn editor_smoke_imports_cooks_bundles_and_loads_runtime_output() {
        let report = run_editor_smoke();

        assert_eq!(report.scanned_sources, 2);
        assert_eq!(report.imported_assets, 2);
        assert_eq!(report.cooked_assets, 2);
        assert_eq!(report.bundled_assets, 2);
        assert!(report.bundle_group_ready);
        assert!(report.material_ready_with_dependencies);
        assert_eq!(report.runtime_dependencies, 1);
        assert!(report.ready_events >= 2);
        assert_eq!(report.failed_events, 0);
    }

    #[test]
    fn model_smoke_imports_generated_subresources_and_loads_bundle() {
        let report = run_model_smoke();

        assert_eq!(report.generated_subresources, 5);
        assert_eq!(report.bundled_assets, 7);
        assert!(report.bundle_group_ready);
        assert!(report.mesh_ready);
        assert!(report.material_ready_with_dependencies);
        assert!(report.skeleton_ready);
        assert!(report.animation_ready);
        assert!(report.physics_ready);
        assert_eq!(report.material_dependencies, 2);
        assert_eq!(report.skeleton_root.as_deref(), Some("Root"));
        assert_eq!(report.animation_target.as_deref(), Some("Root"));
        assert_eq!(report.physics_vertices, 3);
    }
}
