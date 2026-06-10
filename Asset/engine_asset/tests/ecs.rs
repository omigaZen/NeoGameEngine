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

#[derive(Default)]
struct RecordingInstantiationSink {
    events: Vec<String>,
}

impl InstantiationSink for RecordingInstantiationSink {
    fn spawn_entity(&mut self, entity_index: usize, name: Option<&str>, parent: Option<u64>) {
        self.events
            .push(format!("spawn:{entity_index}:{:?}:{:?}", name, parent));
    }

    fn attach_component(&mut self, entity_index: usize, type_name: &str, data: &[u8]) {
        self.events.push(format!(
            "attach:{entity_index}:{type_name}:{}",
            String::from_utf8_lossy(data)
        ));
    }
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
    assert_eq!(
        component.instantiation_commands(&server),
        Some(vec![
            SceneInstantiationCommand::SpawnEntity {
                entity_index: 0,
                name: Some("Root".to_owned()),
                parent: None,
            },
            SceneInstantiationCommand::AttachComponent {
                entity_index: 0,
                type_name: "Transform".to_owned(),
                data: b"translation=0,0,0".to_vec(),
            },
            SceneInstantiationCommand::SpawnEntity {
                entity_index: 1,
                name: Some("Hero".to_owned()),
                parent: Some(0),
            },
            SceneInstantiationCommand::AttachComponent {
                entity_index: 1,
                type_name: "MeshRenderer".to_owned(),
                data: b"mesh=meshes/tri.mesh".to_vec(),
            },
        ])
    );
    let mut sink = RecordingInstantiationSink::default();
    component
        .instantiation_plan(&server)
        .unwrap()
        .apply(server.get(&component.scene).unwrap(), &mut sink);
    assert_eq!(
        sink.events,
        vec![
            "spawn:0:Some(\"Root\"):None".to_owned(),
            "attach:0:Transform:translation=0,0,0".to_owned(),
            "spawn:1:Some(\"Hero\"):Some(0)".to_owned(),
            "attach:1:MeshRenderer:mesh=meshes/tri.mesh".to_owned(),
        ]
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
fn scene_instance_component_can_instantiate_directly_via_sink() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<SceneAsset>();
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
            dependencies: Vec::new(),
        },
    );

    let ready_component = SceneInstanceComponent {
        scene: scene_handle.clone(),
        loaded: false,
    };
    let mut sink = RecordingInstantiationSink::default();
    assert!(ready_component.instantiate(&server, &mut sink));
    assert_eq!(
        sink.events,
        vec![
            "spawn:0:Some(\"Root\"):None".to_owned(),
            "attach:0:Transform:translation=0,0,0".to_owned(),
            "spawn:1:Some(\"Hero\"):Some(0)".to_owned(),
            "attach:1:MeshRenderer:mesh=meshes/tri.mesh".to_owned(),
        ]
    );

    let pending_component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: true,
    };
    let mut second_sink = RecordingInstantiationSink::default();
    assert!(!pending_component.instantiate(&server, &mut second_sink));
    assert!(second_sink.events.is_empty());
}

#[test]
fn prefab_instance_component_can_instantiate_directly_via_sink() {
    let prefab_id = AssetId::new();
    let prefab_handle = Handle::<Prefab>::strong(prefab_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<Prefab>();
    server.storage_mut::<Prefab>().unwrap().insert(
        prefab_id,
        Prefab {
            root: SerializedEntity {
                name: Some("Hero".to_owned()),
                parent: None,
                components: vec![SerializedComponent {
                    type_name: "Transform".to_owned(),
                    data: b"translation=0,0,0".to_vec(),
                }],
            },
            children: vec![SerializedEntity {
                name: Some("Weapon".to_owned()),
                parent: Some(0),
                components: vec![SerializedComponent {
                    type_name: "MeshRenderer".to_owned(),
                    data: b"mesh=meshes/weapon.mesh".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let ready_component = PrefabInstanceComponent {
        prefab: prefab_handle.clone(),
        loaded: false,
    };
    let mut sink = RecordingInstantiationSink::default();
    assert!(ready_component.instantiate(&server, &mut sink));
    assert_eq!(
        sink.events,
        vec![
            "spawn:0:Some(\"Hero\"):None".to_owned(),
            "attach:0:Transform:translation=0,0,0".to_owned(),
            "spawn:1:Some(\"Weapon\"):Some(0)".to_owned(),
            "attach:1:MeshRenderer:mesh=meshes/weapon.mesh".to_owned(),
        ]
    );

    let pending_component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: true,
    };
    let mut second_sink = RecordingInstantiationSink::default();
    assert!(!pending_component.instantiate(&server, &mut second_sink));
    assert!(second_sink.events.is_empty());
}

#[test]
fn prefab_instance_component_exports_stable_spawn_and_attach_commands() {
    let prefab_id = AssetId::new();
    let dependency_id = AssetId::new();
    let prefab_handle = Handle::<Prefab>::strong(prefab_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<Prefab>();
    let pending_component = PrefabInstanceComponent {
        prefab: prefab_handle.clone(),
        loaded: false,
    };
    assert!(!pending_component.can_instantiate(&server));
    assert!(pending_component.instantiation_plan(&server).is_none());
    assert!(pending_component.instantiation_commands(&server).is_none());

    server.storage_mut::<Prefab>().unwrap().insert(
        prefab_id,
        Prefab {
            root: SerializedEntity {
                name: Some("Hero".to_owned()),
                parent: None,
                components: vec![SerializedComponent {
                    type_name: "Transform".to_owned(),
                    data: b"translation=0,0,0".to_vec(),
                }],
            },
            children: vec![
                SerializedEntity {
                    name: Some("Weapon".to_owned()),
                    parent: Some(0),
                    components: vec![SerializedComponent {
                        type_name: "MeshRenderer".to_owned(),
                        data: b"mesh=meshes/weapon.mesh".to_vec(),
                    }],
                },
                SerializedEntity {
                    name: Some("Light".to_owned()),
                    parent: Some(0),
                    components: vec![],
                },
            ],
            dependencies: vec![UntypedHandle::new(
                dependency_id,
                Mesh::TYPE_ID,
                HandleStrength::Weak,
            )],
        },
    );

    let component = PrefabInstanceComponent {
        prefab: prefab_handle.clone(),
        loaded: false,
    };

    assert!(component.is_prefab_asset_ready(&server));
    assert!(component.can_instantiate(&server));
    assert_eq!(
        component.instantiation_plan(&server),
        Some(PrefabInstantiationPlan {
            prefab: prefab_id,
            entity_count: 3,
            component_count: 2,
            dependency_count: 1,
        })
    );
    assert_eq!(
        component.instantiation_commands(&server),
        Some(vec![
            PrefabInstantiationCommand::SpawnEntity {
                entity_index: 0,
                name: Some("Hero".to_owned()),
                parent: None,
            },
            PrefabInstantiationCommand::AttachComponent {
                entity_index: 0,
                type_name: "Transform".to_owned(),
                data: b"translation=0,0,0".to_vec(),
            },
            PrefabInstantiationCommand::SpawnEntity {
                entity_index: 1,
                name: Some("Weapon".to_owned()),
                parent: Some(0),
            },
            PrefabInstantiationCommand::AttachComponent {
                entity_index: 1,
                type_name: "MeshRenderer".to_owned(),
                data: b"mesh=meshes/weapon.mesh".to_vec(),
            },
            PrefabInstantiationCommand::SpawnEntity {
                entity_index: 2,
                name: Some("Light".to_owned()),
                parent: Some(0),
            },
        ])
    );
    let mut sink = RecordingInstantiationSink::default();
    component
        .instantiation_plan(&server)
        .unwrap()
        .apply(server.get(&component.prefab).unwrap(), &mut sink);
    assert_eq!(
        sink.events,
        vec![
            "spawn:0:Some(\"Hero\"):None".to_owned(),
            "attach:0:Transform:translation=0,0,0".to_owned(),
            "spawn:1:Some(\"Weapon\"):Some(0)".to_owned(),
            "attach:1:MeshRenderer:mesh=meshes/weapon.mesh".to_owned(),
            "spawn:2:Some(\"Light\"):Some(0)".to_owned(),
        ]
    );
    assert_eq!(component.asset_handles()[0].id(), prefab_id);
    assert_eq!(server.state_by_id(prefab_id), AssetLoadState::Ready);

    let loaded_component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: true,
    };
    assert!(loaded_component.instantiation_plan(&server).is_none());
    assert!(loaded_component.instantiation_commands(&server).is_none());
    assert_eq!(server.state_by_id(prefab_id), AssetLoadState::Ready);
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
