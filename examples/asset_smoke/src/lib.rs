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
    pub scene_ready: bool,
    pub prefab_ready: bool,
    pub material_ready_with_dependencies: bool,
    pub group_ready: bool,
    pub group_total_assets: usize,
    pub group_ready_assets: usize,
    pub material_dependencies: usize,
    pub scene_commands: usize,
    pub prefab_commands: usize,
    pub scene_sink_events: usize,
    pub prefab_sink_events: usize,
    pub scene_trace: Vec<String>,
    pub prefab_trace: Vec<String>,
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
    pub scene_ready_with_dependencies: bool,
    pub prefab_ready_with_dependencies: bool,
    pub runtime_dependencies: usize,
    pub scene_dependencies: usize,
    pub prefab_dependencies: usize,
    pub scene_commands: usize,
    pub prefab_commands: usize,
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
    io.insert(
        "scenes/hero.scene",
        "NGA_SCENE_V1\nname=hero_scene\ndependency=meshes/tri.mesh\ndependency=materials/hero.material\nentity=Root\ncomponent=Transform|translation=0,0,0\nentity=Hero;parent=0\ncomponent=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material\n",
    );
    io.insert(
        "prefabs/hero.prefab",
        "NGA_PREFAB_V1\ndependency=meshes/tri.mesh\ndependency=materials/hero.material\nroot=Hero\ncomponent=Transform|translation=0,0,0\nchild=Weapon;parent=0\ncomponent=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material\n",
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
    let scene = SceneInstanceComponent {
        scene: assets.load("scenes/hero.scene"),
        loaded: false,
    };
    let prefab = PrefabInstanceComponent {
        prefab: assets.load("prefabs/hero.prefab"),
        loaded: false,
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
            && scene.can_instantiate(&assets)
            && prefab.can_instantiate(&assets)
            && assets.group_state(&group) == AssetLoadState::Ready
        {
            break;
        }
    }

    let mut scene_sink = RecordingInstantiationSink::default();
    let mut prefab_sink = RecordingInstantiationSink::default();
    let scene_commands = scene
        .instantiation_commands(&assets)
        .map(|commands| commands.len())
        .unwrap_or_default();
    let prefab_commands = prefab
        .instantiation_commands(&assets)
        .map(|commands| commands.len())
        .unwrap_or_default();
    if let Some(plan) = scene.instantiation_plan(&assets) {
        let scene_asset = assets.get(&scene.scene).unwrap();
        plan.apply(scene_asset, &mut scene_sink);
    }
    if let Some(plan) = prefab.instantiation_plan(&assets) {
        let prefab_asset = assets.get(&prefab.prefab).unwrap();
        plan.apply(prefab_asset, &mut prefab_sink);
    }

    let group_progress = assets.group_progress(&group);
    SmokeReport {
        render_ready: renderer.is_ready(&assets),
        audio_ready: audio.is_ready(&assets),
        physics_ready: physics.is_ready(&assets),
        scene_ready: scene.can_instantiate(&assets),
        prefab_ready: prefab.can_instantiate(&assets),
        material_ready_with_dependencies: assets.is_ready_with_dependencies(&renderer.material),
        group_ready: assets.group_state(&group) == AssetLoadState::Ready,
        group_total_assets: group_progress.total_assets,
        group_ready_assets: group_progress.ready_assets,
        material_dependencies: assets
            .dependency_graph()
            .direct_dependencies(renderer.material.id())
            .len(),
        scene_commands,
        prefab_commands,
        scene_sink_events: scene_sink.events.len(),
        prefab_sink_events: prefab_sink.events.len(),
        scene_trace: scene_sink.events,
        prefab_trace: prefab_sink.events,
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
    let mesh_path = AssetPath::parse("meshes/editor.mesh");
    let material_path = AssetPath::parse("materials/editor.material");
    let scene_path = AssetPath::parse("scenes/editor.scene");
    let prefab_path = AssetPath::parse("prefabs/editor.prefab");
    let material_source =
        b"name=editor\ntexture.albedo=textures/editor.texture\nbase_color=0.25,0.5,1,1\n".to_vec();
    let mut io = MemoryAssetIo::new();
    io.insert(texture_path.path(), texture_bytes(1, 1));
    io.insert(mesh_path.path(), mesh_bytes());
    io.insert(material_path.path(), material_source);
    io.insert(
        scene_path.path(),
        b"NGA_SCENE_V1\nname=editor_scene\ndependency=textures/editor.texture\ndependency=materials/editor.material\nentity=Root\ncomponent=Transform|translation=0,0,0\nentity=Child;parent=0\ncomponent=MeshRenderer|mesh=meshes/editor.mesh;material=materials/editor.material\n"
            .to_vec(),
    );
    io.insert(
        prefab_path.path(),
        b"NGA_PREFAB_V1\ndependency=textures/editor.texture\ndependency=materials/editor.material\nroot=EditorRoot\ncomponent=Transform|translation=1,0,0\nchild=EditorChild;parent=0\ncomponent=MeshRenderer|mesh=meshes/editor.mesh;material=materials/editor.material\n"
            .to_vec(),
    );

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
    let mesh_id = database.import_asset_path(&mesh_path).unwrap();
    let material_id = database.import_asset_path(&material_path).unwrap();
    let scene_id = database.import_asset_path(&scene_path).unwrap();
    let prefab_id = database.import_asset_path(&prefab_path).unwrap();
    for id in [texture_id, mesh_id, material_id, scene_id, prefab_id] {
        database.cook_asset(id, TargetPlatform::Windows).unwrap();
    }
    let bundle = database
        .build_bundle(&AssetDatabaseBundleBuild::new(
            "editor_smoke",
            vec![material_id, mesh_id, prefab_id, scene_id, texture_id],
        ))
        .unwrap();

    let bundle_io = BundleAssetIo::from_bytes(&bundle.bytes).unwrap();
    let mut assets = AssetServer::new(AssetServerConfig::default());
    assets.set_io(bundle_io);
    assets.register_builtin_loaders();
    let mounted = assets.mount_bundle_bytes(&bundle.bytes).unwrap();
    let group = assets.preload_bundle(&mounted);
    let material: Handle<Material> = assets.load(material_path);
    let scene: Handle<SceneAsset> = assets.load(scene_path);
    let prefab: Handle<Prefab> = assets.load(prefab_path);
    let scene_instance = SceneInstanceComponent {
        scene: scene.clone(),
        loaded: false,
    };
    let prefab_instance = PrefabInstanceComponent {
        prefab: prefab.clone(),
        loaded: false,
    };
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
            && assets.is_ready_with_dependencies(&scene)
            && assets.is_ready_with_dependencies(&prefab)
        {
            break;
        }
    }

    let scene_commands = scene_instance
        .instantiation_commands(&assets)
        .map(|commands| commands.len())
        .unwrap_or_default();
    let prefab_commands = prefab_instance
        .instantiation_commands(&assets)
        .map(|commands| commands.len())
        .unwrap_or_default();

    let report = EditorSmokeReport {
        scanned_sources,
        imported_assets: 5,
        cooked_assets: 5,
        bundled_assets: bundle.asset_count,
        bundle_group_ready: assets.group_state(&group) == AssetLoadState::Ready,
        material_ready_with_dependencies: assets.is_ready_with_dependencies(&material),
        scene_ready_with_dependencies: assets.is_ready_with_dependencies(&scene),
        prefab_ready_with_dependencies: assets.is_ready_with_dependencies(&prefab),
        runtime_dependencies: assets
            .dependency_graph()
            .direct_dependencies(material.id())
            .len(),
        scene_dependencies: assets
            .dependency_graph()
            .direct_dependencies(scene.id())
            .len(),
        prefab_dependencies: assets
            .dependency_graph()
            .direct_dependencies(prefab.id())
            .len(),
        scene_commands,
        prefab_commands,
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

#[derive(Default)]
struct RecordingInstantiationSink {
    events: Vec<String>,
}

impl InstantiationSink for RecordingInstantiationSink {
    fn spawn_entity(&mut self, entity_index: usize, name: Option<&str>, parent: Option<u64>) {
        self.events
            .push(format!("spawn:{entity_index}:{name:?}:{parent:?}"));
    }

    fn attach_component(&mut self, entity_index: usize, type_name: &str, data: &[u8]) {
        self.events.push(format!(
            "attach:{entity_index}:{type_name}:{}",
            String::from_utf8_lossy(data)
        ));
    }
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
        assert!(report.scene_ready);
        assert!(report.prefab_ready);
        assert!(report.material_ready_with_dependencies);
        assert!(report.group_ready);
        assert_eq!(report.group_total_assets, 4);
        assert_eq!(report.group_ready_assets, report.group_total_assets);
        assert_eq!(report.material_dependencies, 2);
        assert_eq!(report.scene_commands, 4);
        assert_eq!(report.prefab_commands, 4);
        assert_eq!(report.scene_sink_events, 4);
        assert_eq!(report.prefab_sink_events, 4);
        assert_eq!(
            report.scene_trace,
            vec![
                "spawn:0:Some(\"Root\"):None".to_owned(),
                "attach:0:Transform:translation=0,0,0".to_owned(),
                "spawn:1:Some(\"Hero\"):Some(0)".to_owned(),
                "attach:1:MeshRenderer:mesh=meshes/tri.mesh;material=materials/hero.material"
                    .to_owned(),
            ]
        );
        assert_eq!(
            report.prefab_trace,
            vec![
                "spawn:0:Some(\"Hero\"):None".to_owned(),
                "attach:0:Transform:translation=0,0,0".to_owned(),
                "spawn:1:Some(\"Weapon\"):Some(0)".to_owned(),
                "attach:1:MeshRenderer:mesh=meshes/tri.mesh;material=materials/hero.material"
                    .to_owned(),
            ]
        );
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

        assert_eq!(report.scanned_sources, 5);
        assert_eq!(report.imported_assets, 5);
        assert_eq!(report.cooked_assets, 5);
        assert_eq!(report.bundled_assets, 5);
        assert!(report.bundle_group_ready);
        assert!(report.material_ready_with_dependencies);
        assert!(report.scene_ready_with_dependencies);
        assert!(report.prefab_ready_with_dependencies);
        assert_eq!(report.runtime_dependencies, 1);
        assert_eq!(report.scene_dependencies, 3);
        assert_eq!(report.prefab_dependencies, 3);
        assert_eq!(report.scene_commands, 4);
        assert_eq!(report.prefab_commands, 4);
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
