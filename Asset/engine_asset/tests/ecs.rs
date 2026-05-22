use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn mesh_bytes() -> &'static [u8] {
    b"v 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n"
}

fn physics_mesh_bytes() -> &'static [u8] {
    b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n"
}

fn finish_uploads(server: &mut AssetServer) {
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(uploads.into_iter().enumerate().map(|(index, upload)| {
        GpuUploadResult::ok(upload.id, GpuResourceHandle(index as u64 + 1))
    }));
}

#[test]
fn ecs_system_descriptors_match_documented_order() {
    let descriptors = asset_system_descriptors();
    let order = descriptors
        .iter()
        .map(|descriptor| descriptor.stage)
        .collect::<Vec<_>>();

    assert_eq!(order, ASSET_SYSTEM_ORDER);
    assert!(validate_asset_system_order(&order));
    assert!(stage_runs_before(
        AssetSystemStage::AssetRequest,
        AssetSystemStage::AssetServerUpdate
    ));
    assert!(stage_runs_before(
        AssetSystemStage::GpuUploadPrepare,
        AssetSystemStage::RenderPrepare
    ));
    assert!(stage_runs_before(
        AssetSystemStage::AudioPrepare,
        AssetSystemStage::AssetGc
    ));
    assert_eq!(descriptors[0].name, "AssetRequestSystem");
    assert_eq!(descriptors[7].name, "AssetGcSystem");
}

#[test]
fn mesh_renderer_component_readiness_uses_asset_server_state_and_dependencies() {
    let mut io = MemoryAssetIo::new();
    io.insert("meshes/tri.mesh", mesh_bytes().to_vec());
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 1));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();

    let component = MeshRendererComponent {
        mesh: server.load("meshes/tri.mesh"),
        material: server.load("materials/hero.material"),
    };
    let handles = component.asset_handles();
    assert_eq!(handles.len(), 2);
    assert!(!component.is_ready(&server));

    server.update_loading();
    finish_uploads(&mut server);
    finish_uploads(&mut server);

    assert!(component.is_ready(&server));
    assert!(handles.iter().all(|handle| {
        handle.id() == component.mesh.id() || handle.id() == component.material.id()
    }));
}

#[test]
fn scene_instance_component_reports_instantiation_readiness_without_mutating_lifecycle() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let dependency_id = AssetId::new();
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<SceneAsset>();
    let pending_component = SceneInstanceComponent {
        scene: scene_handle.clone(),
        loaded: false,
    };
    assert!(!pending_component.can_instantiate(&server));
    assert!(pending_component.instantiation_plan(&server).is_none());

    server.storage_mut::<SceneAsset>().unwrap().insert(
        scene_id,
        SceneAsset {
            name: "level".to_owned(),
            entities: vec![
                SerializedEntity {
                    name: Some("Root".to_owned()),
                    parent: None,
                    components: vec![SerializedComponent {
                        type_name: "Transform".to_owned(),
                        data: b"translation=0,0,0".to_vec(),
                    }],
                },
                SerializedEntity {
                    name: Some("Hero".to_owned()),
                    parent: Some(0),
                    components: vec![SerializedComponent {
                        type_name: "MeshRenderer".to_owned(),
                        data: b"mesh=meshes/tri.mesh".to_vec(),
                    }],
                },
            ],
            dependencies: vec![UntypedHandle::new(
                dependency_id,
                Texture::TYPE_ID,
                HandleStrength::Weak,
            )],
        },
    );

    let component = SceneInstanceComponent {
        scene: scene_handle.clone(),
        loaded: false,
    };

    assert!(component.is_scene_asset_ready(&server));
    assert!(component.can_instantiate(&server));
    assert_eq!(
        component.instantiation_plan(&server),
        Some(SceneInstantiationPlan {
            scene: scene_id,
            entity_count: 2,
            component_count: 2,
            dependency_count: 1,
        })
    );
    assert_eq!(component.asset_handles()[0].id(), scene_id);
    assert_eq!(server.state_by_id(scene_id), AssetLoadState::Ready);

    let loaded_component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: true,
    };
    assert!(loaded_component.instantiation_plan(&server).is_none());
    assert_eq!(server.state_by_id(scene_id), AssetLoadState::Ready);
}

#[test]
fn audio_source_component_exposes_handle_readiness_without_owning_audio_resource() {
    let clip_id = AssetId::new();
    let clip = Handle::<AudioClip>::strong(clip_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<AudioClip>();
    let component = AudioSourceComponent {
        clip: clip.clone(),
        looping: true,
        volume: 0.5,
    };
    assert_eq!(component.asset_handles()[0].id(), clip_id);
    assert!(!component.is_ready(&server));

    server.storage_mut::<AudioClip>().unwrap().insert(
        clip_id,
        AudioClip {
            sample_rate: 48_000,
            channels: 2,
            samples: AudioSamples::F32(vec![0.0; 4]),
            duration_seconds: 0.000_041,
            streaming: false,
        },
    );

    assert!(component.is_ready(&server));
    assert!(server.get(&clip).is_some());
}

#[test]
fn physics_collider_component_exposes_handle_readiness_without_owning_resource() {
    let mut io = MemoryAssetIo::new();
    io.insert("physics/hero.physics", physics_mesh_bytes().to_vec());
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();

    let component = PhysicsColliderComponent {
        mesh: server.load("physics/hero.physics"),
        dynamic: true,
    };
    assert_eq!(component.asset_handles()[0].id(), component.mesh.id());
    assert!(!component.is_ready(&server));

    server.update_loading();

    assert!(component.is_ready(&server));
    assert!(server.get(&component.mesh).is_some());
}
