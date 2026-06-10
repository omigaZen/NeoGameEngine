use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use engine_asset::prelude::*;
use engine_physics::prelude::{
    BodyDesc, ColliderDesc, PhysicsConfig, PhysicsWorld, QueryFilter, Ray, TriMeshDesc,
    Vec3 as PhysicsVec3,
};
use engine_render::{
    ColoredVertex as RenderColoredVertex, Material as RenderMaterial, Mesh as RenderMesh,
    OrthographicCamera, RenderQueue, RenderScene, Texture as RenderTexture, TextureSize,
    Transform as RenderTransform,
};
use engine_renderer::prelude::{
    AlphaMode as RendererAlphaMode, Bounds3 as RendererBounds3, IndexData as RendererIndexData,
    MaterialDomain as RendererMaterialDomain, MeshDesc as RendererMeshDesc,
    MeshFlags as RendererMeshFlags, MeshUsage as RendererMeshUsage, Renderer as HeadlessRenderer,
    RendererConfig, ResourceStatus as RendererResourceStatus,
    StandardMaterialDesc as RendererStandardMaterialDesc, TextureDesc as RendererTextureDesc,
    TextureDimension as RendererTextureDimension, TextureFormat as RendererTextureFormat,
    TextureInitialData as RendererTextureInitialData, TextureUsage as RendererTextureUsage,
    Vec3 as RendererVec3, VertexAttribute, VertexData as RendererVertexData, VertexFormat,
    VertexLayout, VertexSemantic, VertexStepMode, VertexStreamLayout,
};

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
    pub render_scene_meshes: usize,
    pub render_scene_textures: usize,
    pub render_scene_materials: usize,
    pub render_scene_instances: usize,
    pub render_queue_items: usize,
    pub render_queue_batches: usize,
    pub render_queue_draw_calls: usize,
    pub render_mesh_vertices: usize,
    pub render_mesh_indices: usize,
    pub render_texture_pixels: usize,
    pub render_material_textured: bool,
    pub renderer_resource_mesh_ready: bool,
    pub renderer_resource_texture_ready: bool,
    pub renderer_resource_material_ready: bool,
    pub renderer_resource_resident_resources: usize,
    pub renderer_resource_resident_bytes: u64,
    pub renderer_resource_mesh_vertices: usize,
    pub renderer_resource_mesh_indices: u32,
    pub renderer_resource_texture_bytes: u64,
    pub physics_world_mesh_ready: bool,
    pub physics_world_collider_ready: bool,
    pub physics_world_ray_hit: bool,
    pub physics_world_triangles: usize,
    pub scene_commands: usize,
    pub prefab_commands: usize,
    pub scene_sink_events: usize,
    pub prefab_sink_events: usize,
    pub scene_typed_entities: usize,
    pub prefab_typed_entities: usize,
    pub scene_typed_components: usize,
    pub prefab_typed_components: usize,
    pub scene_typed_asset_handles: usize,
    pub prefab_typed_asset_handles: usize,
    pub scene_typed_loaded: bool,
    pub prefab_typed_loaded: bool,
    pub scene_trace: Vec<String>,
    pub prefab_trace: Vec<String>,
    pub scene_typed_trace: Vec<String>,
    pub prefab_typed_trace: Vec<String>,
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
    assert!(scene.instantiate(&assets, &mut scene_sink));
    assert!(prefab.instantiate(&assets, &mut prefab_sink));
    assert!(!SceneInstanceComponent {
        scene: scene.scene.clone(),
        loaded: true,
    }
    .instantiate(&assets, &mut scene_sink));
    assert!(!PrefabInstanceComponent {
        prefab: prefab.prefab.clone(),
        loaded: true,
    }
    .instantiate(&assets, &mut prefab_sink));

    let mut scene_typed = scene.clone();
    let mut prefab_typed = prefab.clone();
    let mut scene_typed_sink = RecordingTypedHostInstantiationSink::with_first_entity(100);
    let mut prefab_typed_sink = RecordingTypedHostInstantiationSink::with_first_entity(200);
    let scene_typed_report = scene_typed
        .instantiate_typed_host(&mut assets, &mut scene_typed_sink)
        .unwrap()
        .unwrap();
    let prefab_typed_report = prefab_typed
        .instantiate_typed_host(&mut assets, &mut prefab_typed_sink)
        .unwrap()
        .unwrap();
    assert!(scene_typed
        .instantiate_typed_host(&mut assets, &mut scene_typed_sink)
        .unwrap()
        .is_none());
    assert!(prefab_typed
        .instantiate_typed_host(&mut assets, &mut prefab_typed_sink)
        .unwrap()
        .is_none());

    let physics_bridge = drive_physics_world_from_asset(&assets, &physics);
    let render_bridge = drive_render_scene_from_assets(&assets, &renderer);
    let renderer_resource_bridge = drive_headless_renderer_from_assets(&assets, &renderer);
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
        render_scene_meshes: render_bridge.scene_meshes,
        render_scene_textures: render_bridge.scene_textures,
        render_scene_materials: render_bridge.scene_materials,
        render_scene_instances: render_bridge.scene_instances,
        render_queue_items: render_bridge.queue_items,
        render_queue_batches: render_bridge.queue_batches,
        render_queue_draw_calls: render_bridge.queue_draw_calls,
        render_mesh_vertices: render_bridge.mesh_vertices,
        render_mesh_indices: render_bridge.mesh_indices,
        render_texture_pixels: render_bridge.texture_pixels,
        render_material_textured: render_bridge.material_textured,
        renderer_resource_mesh_ready: renderer_resource_bridge.mesh_ready,
        renderer_resource_texture_ready: renderer_resource_bridge.texture_ready,
        renderer_resource_material_ready: renderer_resource_bridge.material_ready,
        renderer_resource_resident_resources: renderer_resource_bridge.resident_resources,
        renderer_resource_resident_bytes: renderer_resource_bridge.resident_bytes,
        renderer_resource_mesh_vertices: renderer_resource_bridge.mesh_vertices,
        renderer_resource_mesh_indices: renderer_resource_bridge.mesh_indices,
        renderer_resource_texture_bytes: renderer_resource_bridge.texture_bytes,
        physics_world_mesh_ready: physics_bridge.mesh_ready,
        physics_world_collider_ready: physics_bridge.collider_ready,
        physics_world_ray_hit: physics_bridge.ray_hit,
        physics_world_triangles: physics_bridge.triangles,
        scene_commands,
        prefab_commands,
        scene_sink_events: scene_sink.events.len(),
        prefab_sink_events: prefab_sink.events.len(),
        scene_typed_entities: scene_typed_report.entities.len(),
        prefab_typed_entities: prefab_typed_report.entities.len(),
        scene_typed_components: scene_typed_report.attached_component_count,
        prefab_typed_components: prefab_typed_report.attached_component_count,
        scene_typed_asset_handles: scene_typed_sink.asset_handles,
        prefab_typed_asset_handles: prefab_typed_sink.asset_handles,
        scene_typed_loaded: scene_typed.loaded,
        prefab_typed_loaded: prefab_typed.loaded,
        scene_trace: scene_sink.events,
        prefab_trace: prefab_sink.events,
        scene_typed_trace: scene_typed_sink.events,
        prefab_typed_trace: prefab_typed_sink.events,
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
    let mut scene_sink = RecordingInstantiationSink::default();
    let mut prefab_sink = RecordingInstantiationSink::default();
    assert!(scene_instance.instantiate(&assets, &mut scene_sink));
    assert!(prefab_instance.instantiate(&assets, &mut prefab_sink));

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

struct RenderBridgeReport {
    scene_meshes: usize,
    scene_textures: usize,
    scene_materials: usize,
    scene_instances: usize,
    queue_items: usize,
    queue_batches: usize,
    queue_draw_calls: usize,
    mesh_vertices: usize,
    mesh_indices: usize,
    texture_pixels: usize,
    material_textured: bool,
}

fn drive_render_scene_from_assets(
    assets: &AssetServer,
    renderer: &MeshRendererComponent,
) -> RenderBridgeReport {
    let mesh = assets
        .get(&renderer.mesh)
        .expect("smoke mesh should be ready before render bridge");
    let material = assets
        .get(&renderer.material)
        .expect("smoke material should be ready before render bridge");
    let texture_binding = material
        .textures
        .iter()
        .find(|binding| binding.name == "albedo" || binding.name == "base_color")
        .or_else(|| material.textures.first())
        .expect("smoke material should expose a texture binding");
    let texture = assets
        .get(&texture_binding.texture)
        .expect("smoke material texture should be ready before render bridge");

    let render_mesh = render_mesh_from_asset(mesh);
    let mesh_vertices = render_mesh.vertex_count();
    let mesh_indices = render_mesh.index_count();
    let render_texture = RenderTexture::rgba8(
        TextureSize::new(texture.width, texture.height),
        texture.data.clone(),
    )
    .expect("asset texture should convert to engine_render rgba8 texture");
    let texture_pixels = usize::try_from(texture.width)
        .ok()
        .and_then(|width| {
            usize::try_from(texture.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .unwrap_or_default();

    let mut scene = RenderScene::new(OrthographicCamera::new_2d(2.0));
    let mesh_handle = scene.add_mesh(render_mesh);
    let texture_handle = scene.add_texture(render_texture);
    let render_material = RenderMaterial::textured(material.properties.base_color, texture_handle);
    let material_textured = render_material.base_color_texture.is_some();
    let material_handle = scene.add_material(render_material);
    scene.add_instance_with_material(mesh_handle, material_handle, RenderTransform::IDENTITY);

    let queue = RenderQueue::from_scene(&scene);
    let stats = queue.stats();
    RenderBridgeReport {
        scene_meshes: scene.mesh_entries().count(),
        scene_textures: scene.texture_entries().count(),
        scene_materials: scene.material_entries().count(),
        scene_instances: scene.instance_count(),
        queue_items: stats.item_count,
        queue_batches: stats.batch_count,
        queue_draw_calls: stats.draw_call_count,
        mesh_vertices,
        mesh_indices,
        texture_pixels,
        material_textured,
    }
}

fn render_mesh_from_asset(mesh: &Mesh) -> RenderMesh {
    let vertices = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let normal = mesh.normals.get(index).copied().unwrap_or([0.0, 0.0, 1.0]);
            let uv = mesh.uvs.get(index).copied().unwrap_or([0.0, 0.0]);
            let uv1 = mesh
                .uv_sets
                .first()
                .and_then(|uvs| uvs.get(index))
                .copied()
                .unwrap_or(uv);
            let tangent = mesh
                .tangents
                .get(index)
                .copied()
                .unwrap_or([1.0, 0.0, 0.0, 1.0]);
            RenderColoredVertex::with_normal_uvs_tangent(
                *position,
                [1.0, 1.0, 1.0],
                normal,
                uv,
                uv1,
                tangent,
            )
        })
        .collect::<Vec<_>>();
    RenderMesh::with_indices(vertices, mesh.indices.clone())
}

struct HeadlessRendererBridgeReport {
    mesh_ready: bool,
    texture_ready: bool,
    material_ready: bool,
    resident_resources: usize,
    resident_bytes: u64,
    mesh_vertices: usize,
    mesh_indices: u32,
    texture_bytes: u64,
}

fn drive_headless_renderer_from_assets(
    assets: &AssetServer,
    renderer: &MeshRendererComponent,
) -> HeadlessRendererBridgeReport {
    let mesh = assets
        .get(&renderer.mesh)
        .expect("smoke mesh should be ready before renderer resource bridge");
    let material = assets
        .get(&renderer.material)
        .expect("smoke material should be ready before renderer resource bridge");
    let texture_binding = material
        .textures
        .iter()
        .find(|binding| binding.name == "albedo" || binding.name == "base_color")
        .or_else(|| material.textures.first())
        .expect("smoke material should expose a texture binding");
    let texture = assets
        .get(&texture_binding.texture)
        .expect("smoke material texture should be ready before renderer resource bridge");

    let mut renderer = HeadlessRenderer::new_headless(RendererConfig::default());
    let vertex_bytes = renderer_mesh_vertex_bytes(mesh);
    let bounds = renderer_mesh_bounds(mesh);
    let renderer_mesh = renderer
        .create_mesh(RendererMeshDesc {
            label: Some("asset_smoke_mesh"),
            vertex_layout: renderer_mesh_vertex_layout(),
            vertices: RendererVertexData::Interleaved(&vertex_bytes),
            indices: Some(RendererIndexData::U32(&mesh.indices)),
            submeshes: vec![engine_renderer::prelude::SubMeshDesc {
                index_range: 0..u32::try_from(mesh.indices.len()).unwrap_or_default(),
                vertex_range: 0..u32::try_from(mesh.vertices.len()).unwrap_or_default(),
                material_slot: 0,
                bounds,
            }],
            bounds,
            usage: RendererMeshUsage::STATIC,
            flags: RendererMeshFlags::default(),
            skin: None,
            morph_targets: Vec::new(),
            meshlets: None,
        })
        .expect("asset mesh should create a headless renderer mesh resource");
    let renderer_texture = renderer
        .create_texture(RendererTextureDesc {
            label: Some("asset_smoke_texture"),
            dimension: RendererTextureDimension::D2,
            width: texture.width,
            height: texture.height,
            depth_or_layers: 1,
            mip_levels: 1,
            samples: 1,
            format: RendererTextureFormat::Rgba8UnormSrgb,
            usage: RendererTextureUsage::SAMPLED | RendererTextureUsage::COPY_DST,
            initial_data: Some(RendererTextureInitialData {
                bytes: &texture.data,
                bytes_per_row: texture.width.saturating_mul(4),
                rows_per_image: texture.height,
            }),
        })
        .expect("asset texture should create a headless renderer texture resource");
    let renderer_material = renderer
        .create_standard_material(RendererStandardMaterialDesc {
            label: Some("asset_smoke_material".to_owned()),
            domain: RendererMaterialDomain::Opaque,
            base_color: engine_graphics::Color::rgba(
                f64::from(material.properties.base_color[0]),
                f64::from(material.properties.base_color[1]),
                f64::from(material.properties.base_color[2]),
                f64::from(material.properties.base_color[3]),
            ),
            base_color_texture: Some(renderer_texture),
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            metallic: material.properties.metallic,
            roughness: material.properties.roughness,
            emissive: RendererVec3::new(
                material.properties.emissive[0],
                material.properties.emissive[1],
                material.properties.emissive[2],
            ),
            alpha_mode: RendererAlphaMode::Opaque,
            double_sided: material.render_state.double_sided,
            receive_shadows: true,
            cast_shadows: true,
        })
        .expect("asset material should create a headless renderer material resource");

    let memory = renderer.memory_stats();
    HeadlessRendererBridgeReport {
        mesh_ready: renderer.resource_status(renderer_mesh) == Some(RendererResourceStatus::Ready),
        texture_ready: renderer.resource_status(renderer_texture)
            == Some(RendererResourceStatus::Ready),
        material_ready: renderer.resource_status(renderer_material)
            == Some(RendererResourceStatus::Ready),
        resident_resources: memory.resident_resources,
        resident_bytes: memory.resident_bytes,
        mesh_vertices: mesh.vertices.len(),
        mesh_indices: renderer
            .mesh_info(renderer_mesh)
            .map(|info| info.index_count)
            .unwrap_or_default(),
        texture_bytes: renderer
            .texture_info(renderer_texture)
            .map(|info| info.subresource_byte_len())
            .unwrap_or_default(),
    }
}

fn renderer_mesh_vertex_layout() -> VertexLayout {
    VertexLayout {
        streams: vec![VertexStreamLayout {
            stride: 32,
            step: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    semantic: VertexSemantic::Position,
                    format: VertexFormat::Float32x3,
                    offset: 0,
                },
                VertexAttribute {
                    semantic: VertexSemantic::Normal,
                    format: VertexFormat::Float32x3,
                    offset: 12,
                },
                VertexAttribute {
                    semantic: VertexSemantic::TexCoord(0),
                    format: VertexFormat::Float32x2,
                    offset: 24,
                },
            ],
        }],
    }
}

fn renderer_mesh_vertex_bytes(mesh: &Mesh) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(mesh.vertices.len() * 32);
    for (index, position) in mesh.vertices.iter().enumerate() {
        let normal = mesh.normals.get(index).copied().unwrap_or([0.0, 0.0, 1.0]);
        let uv = mesh.uvs.get(index).copied().unwrap_or([0.0, 0.0]);
        for value in *position {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in normal {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in uv {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

fn renderer_mesh_bounds(mesh: &Mesh) -> RendererBounds3 {
    let first = mesh.vertices.first().copied().unwrap_or([0.0, 0.0, 0.0]);
    let mut min = first;
    let mut max = first;
    for vertex in mesh.vertices.iter().skip(1) {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    RendererBounds3::new(
        RendererVec3::new(min[0], min[1], min[2]),
        RendererVec3::new(max[0], max[1], max[2]),
    )
}

struct PhysicsBridgeReport {
    mesh_ready: bool,
    collider_ready: bool,
    ray_hit: bool,
    triangles: usize,
}

fn drive_physics_world_from_asset(
    assets: &AssetServer,
    collider: &PhysicsColliderComponent,
) -> PhysicsBridgeReport {
    let mesh = assets
        .get(&collider.mesh)
        .expect("smoke physics mesh should be ready before physics bridge");
    assert_eq!(mesh.kind, PhysicsMeshKind::TriMesh);

    let mut world = PhysicsWorld::new(PhysicsConfig::default());
    let physics_mesh = world
        .create_trimesh(TriMeshDesc {
            vertices: mesh
                .vertices
                .iter()
                .map(|vertex| PhysicsVec3::new(vertex[0], vertex[1], vertex[2]))
                .collect(),
            indices: mesh.indices.clone(),
        })
        .expect("asset physics mesh should convert to engine_physics trimesh");
    let body = world
        .create_body(BodyDesc::fixed().with_debug_name("Asset Physics Mesh"))
        .expect("fixed body should be valid");
    let physics_collider = world
        .create_collider_with_parent(
            body,
            ColliderDesc::trimesh(physics_mesh).with_debug_name("Asset Physics Collider"),
        )
        .expect("asset physics mesh collider should be valid");
    let ray_hit = world
        .query()
        .cast_ray(
            Ray {
                origin: PhysicsVec3::new(0.25, 0.25, -1.0),
                direction: PhysicsVec3::Z,
                max_toi: 2.0,
            },
            QueryFilter::default(),
        )
        .is_some();

    PhysicsBridgeReport {
        mesh_ready: world.contains_mesh(physics_mesh),
        collider_ready: world.contains_collider(physics_collider),
        ray_hit,
        triangles: mesh.indices.len(),
    }
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

struct RecordingTypedHostInstantiationSink {
    next_entity: u64,
    events: Vec<String>,
    asset_handles: usize,
}

impl RecordingTypedHostInstantiationSink {
    fn with_first_entity(first_entity: u64) -> Self {
        Self {
            next_entity: first_entity,
            events: Vec::new(),
            asset_handles: 0,
        }
    }
}

impl TypedHostInstantiationSink for RecordingTypedHostInstantiationSink {
    type Entity = u64;
    type Error = String;

    fn spawn_entity(
        &mut self,
        entity_index: usize,
        name: Option<&str>,
        parent: Option<&Self::Entity>,
    ) -> Result<Self::Entity, Self::Error> {
        let entity = self.next_entity;
        self.next_entity += 1;
        self.events
            .push(format!("spawn:{entity_index}:{entity}:{name:?}:{parent:?}"));
        Ok(entity)
    }

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        component: EcsComponentInstance,
    ) -> Result<(), Self::Error> {
        let handle_count = component.asset_handles().len();
        self.asset_handles += handle_count;
        self.events.push(format!(
            "attach:{entity}:{entity_index}:{component_index}:{}:{handle_count}",
            component.type_name()
        ));
        Ok(())
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
        assert_eq!(report.render_scene_meshes, 1);
        assert_eq!(report.render_scene_textures, 1);
        assert_eq!(report.render_scene_materials, 2);
        assert_eq!(report.render_scene_instances, 1);
        assert_eq!(report.render_queue_items, 1);
        assert_eq!(report.render_queue_batches, 1);
        assert_eq!(report.render_queue_draw_calls, 1);
        assert_eq!(report.render_mesh_vertices, 3);
        assert_eq!(report.render_mesh_indices, 3);
        assert_eq!(report.render_texture_pixels, 4);
        assert!(report.render_material_textured);
        assert!(report.renderer_resource_mesh_ready);
        assert!(report.renderer_resource_texture_ready);
        assert!(report.renderer_resource_material_ready);
        assert!(report.renderer_resource_resident_resources >= 3);
        assert!(report.renderer_resource_resident_bytes >= 60);
        assert_eq!(report.renderer_resource_mesh_vertices, 3);
        assert_eq!(report.renderer_resource_mesh_indices, 3);
        assert_eq!(report.renderer_resource_texture_bytes, 16);
        assert!(report.physics_world_mesh_ready);
        assert!(report.physics_world_collider_ready);
        assert!(report.physics_world_ray_hit);
        assert_eq!(report.physics_world_triangles, 1);
        assert_eq!(report.scene_commands, 4);
        assert_eq!(report.prefab_commands, 4);
        assert_eq!(report.scene_sink_events, 4);
        assert_eq!(report.prefab_sink_events, 4);
        assert_eq!(report.scene_typed_entities, 2);
        assert_eq!(report.prefab_typed_entities, 2);
        assert_eq!(report.scene_typed_components, 2);
        assert_eq!(report.prefab_typed_components, 2);
        assert_eq!(report.scene_typed_asset_handles, 2);
        assert_eq!(report.prefab_typed_asset_handles, 2);
        assert!(report.scene_typed_loaded);
        assert!(report.prefab_typed_loaded);
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
        assert_eq!(
            report.scene_typed_trace,
            vec![
                "spawn:0:100:Some(\"Root\"):None".to_owned(),
                "attach:100:0:0:Transform:0".to_owned(),
                "spawn:1:101:Some(\"Hero\"):Some(100)".to_owned(),
                "attach:101:1:0:MeshRenderer:2".to_owned(),
            ]
        );
        assert_eq!(
            report.prefab_typed_trace,
            vec![
                "spawn:0:200:Some(\"Hero\"):None".to_owned(),
                "attach:200:0:0:Transform:0".to_owned(),
                "spawn:1:201:Some(\"Weapon\"):Some(200)".to_owned(),
                "attach:201:1:0:MeshRenderer:2".to_owned(),
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
