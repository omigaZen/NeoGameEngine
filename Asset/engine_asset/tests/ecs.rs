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

#[derive(Default)]
struct RecordingHostInstantiationSink {
    next_entity: u64,
    spawn_error_at: Option<usize>,
    attach_error_at: Option<(usize, usize)>,
    spawns: Vec<(usize, u64, Option<String>, Option<u64>)>,
    attachments: Vec<(u64, usize, usize, String, String)>,
}

impl RecordingHostInstantiationSink {
    fn with_first_entity(first_entity: u64) -> Self {
        Self {
            next_entity: first_entity,
            spawn_error_at: None,
            attach_error_at: None,
            spawns: Vec::new(),
            attachments: Vec::new(),
        }
    }

    fn with_errors(
        first_entity: u64,
        spawn_error_at: Option<usize>,
        attach_error_at: Option<(usize, usize)>,
    ) -> Self {
        Self {
            next_entity: first_entity,
            spawn_error_at,
            attach_error_at,
            spawns: Vec::new(),
            attachments: Vec::new(),
        }
    }
}

impl HostInstantiationSink for RecordingHostInstantiationSink {
    type Entity = u64;
    type Error = String;

    fn spawn_entity(
        &mut self,
        entity_index: usize,
        name: Option<&str>,
        parent: Option<&Self::Entity>,
    ) -> Result<Self::Entity, Self::Error> {
        if self.spawn_error_at == Some(entity_index) {
            return Err(format!("spawn:{entity_index}"));
        }
        let entity = self.next_entity;
        self.next_entity += 1;
        self.spawns.push((
            entity_index,
            entity,
            name.map(str::to_owned),
            parent.copied(),
        ));
        Ok(entity)
    }

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        type_name: &str,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        if self.attach_error_at == Some((entity_index, component_index)) {
            return Err(format!("attach:{entity_index}:{component_index}"));
        }
        self.attachments.push((
            *entity,
            entity_index,
            component_index,
            type_name.to_owned(),
            String::from_utf8_lossy(data).to_string(),
        ));
        Ok(())
    }
}

#[derive(Default)]
struct RecordingTypedHostInstantiationSink {
    next_entity: u64,
    spawn_error_at: Option<usize>,
    attach_error_at: Option<(usize, usize)>,
    spawns: Vec<(usize, u64, Option<String>, Option<u64>)>,
    attachments: Vec<(u64, usize, usize, String, Vec<AssetTypeId>)>,
}

impl RecordingTypedHostInstantiationSink {
    fn with_first_entity(first_entity: u64) -> Self {
        Self {
            next_entity: first_entity,
            spawn_error_at: None,
            attach_error_at: None,
            spawns: Vec::new(),
            attachments: Vec::new(),
        }
    }

    fn with_errors(
        first_entity: u64,
        spawn_error_at: Option<usize>,
        attach_error_at: Option<(usize, usize)>,
    ) -> Self {
        Self {
            next_entity: first_entity,
            spawn_error_at,
            attach_error_at,
            spawns: Vec::new(),
            attachments: Vec::new(),
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
        if self.spawn_error_at == Some(entity_index) {
            return Err(format!("spawn:{entity_index}"));
        }
        let entity = self.next_entity;
        self.next_entity += 1;
        self.spawns.push((
            entity_index,
            entity,
            name.map(str::to_owned),
            parent.copied(),
        ));
        Ok(entity)
    }

    fn attach_component(
        &mut self,
        entity: &Self::Entity,
        entity_index: usize,
        component_index: usize,
        component: EcsComponentInstance,
    ) -> Result<(), Self::Error> {
        if self.attach_error_at == Some((entity_index, component_index)) {
            return Err(format!("attach:{entity_index}:{component_index}"));
        }
        self.attachments.push((
            *entity,
            entity_index,
            component_index,
            component.type_name().to_owned(),
            component
                .asset_handles()
                .iter()
                .map(UntypedHandle::asset_type)
                .collect(),
        ));
        Ok(())
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
fn serialized_component_schema_exports_known_asset_fields_and_references() {
    assert!(serialized_component_type_has_asset_fields(
        "SkinnedMeshRenderer"
    ));
    assert_eq!(
        serialized_component_asset_field_type("PhysicsCollider", "physics_mesh"),
        Some(PhysicsMesh::TYPE_ID)
    );
    let fields = serialized_component_asset_fields("SkinnedMeshRenderer");
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.component_type, field.field, field.asset_type_name))
            .collect::<Vec<_>>(),
        vec![
            ("SkinnedMeshRenderer", "mesh", Mesh::TYPE_NAME),
            ("SkinnedMeshRenderer", "skeleton", Skeleton::TYPE_NAME),
            ("SkinnedMeshRenderer", "material", Material::TYPE_NAME),
        ]
    );

    let component = SerializedComponent {
        type_name: "SkinnedMeshRenderer".to_owned(),
        data: b"mesh=meshes/hero.mesh;skeleton=skeletons/hero.skeleton;material=materials/hero.material"
            .to_vec(),
    };
    let references = serialized_component_asset_references(&component).unwrap();
    assert_eq!(
        references
            .iter()
            .map(|reference| (
                reference.component_type.as_str(),
                reference.field.as_str(),
                reference.path.display_string(),
                reference.asset_type
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "SkinnedMeshRenderer",
                "mesh",
                "meshes/hero.mesh".to_owned(),
                Mesh::TYPE_ID,
            ),
            (
                "SkinnedMeshRenderer",
                "skeleton",
                "skeletons/hero.skeleton".to_owned(),
                Skeleton::TYPE_ID,
            ),
            (
                "SkinnedMeshRenderer",
                "material",
                "materials/hero.material".to_owned(),
                Material::TYPE_ID,
            ),
        ]
    );
}

#[test]
fn serialized_component_materialization_builds_typed_asset_components() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();
    let mesh_renderer = SerializedComponent {
        type_name: "MeshRenderer".to_owned(),
        data: b"mesh=meshes/hero.mesh;material=materials/hero.material".to_vec(),
    };
    let audio_source = SerializedComponent {
        type_name: "AudioSource".to_owned(),
        data: b"clip=audio/click.audio;looping=true;volume=0.25".to_vec(),
    };
    let physics = SerializedComponent {
        type_name: "PhysicsCollider".to_owned(),
        data: b"physics_mesh=physics/hero.physics;dynamic=on".to_vec(),
    };
    let unknown = SerializedComponent {
        type_name: "Transform".to_owned(),
        data: b"translation=0,0,0".to_vec(),
    };

    let materialized_mesh = materialize_serialized_component(&mut server, &mesh_renderer).unwrap();
    let EcsComponentInstance::MeshRenderer(materialized_mesh) = materialized_mesh else {
        panic!("expected MeshRenderer materialization");
    };
    assert_eq!(
        materialized_mesh
            .asset_handles()
            .iter()
            .map(UntypedHandle::asset_type)
            .collect::<Vec<_>>(),
        vec![Mesh::TYPE_ID, Material::TYPE_ID]
    );

    let materialized_audio = materialize_serialized_component(&mut server, &audio_source).unwrap();
    let EcsComponentInstance::AudioSource(materialized_audio) = materialized_audio else {
        panic!("expected AudioSource materialization");
    };
    assert!(materialized_audio.looping);
    assert_eq!(materialized_audio.volume, 0.25);
    assert_eq!(materialized_audio.clip.asset_type(), AudioClip::TYPE_ID);

    let materialized_physics = materialize_serialized_component(&mut server, &physics).unwrap();
    let EcsComponentInstance::PhysicsCollider(materialized_physics) = materialized_physics else {
        panic!("expected PhysicsCollider materialization");
    };
    assert!(materialized_physics.dynamic);
    assert_eq!(materialized_physics.mesh.asset_type(), PhysicsMesh::TYPE_ID);

    let materialized_unknown = materialize_serialized_component(&mut server, &unknown).unwrap();
    let EcsComponentInstance::Unknown { type_name, data } = materialized_unknown else {
        panic!("expected unknown component passthrough");
    };
    assert_eq!(type_name, "Transform");
    assert_eq!(data, b"translation=0,0,0");
}

#[test]
fn serialized_component_materialization_reports_missing_and_invalid_fields() {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();

    let missing = SerializedComponent {
        type_name: "MeshRenderer".to_owned(),
        data: b"mesh=meshes/hero.mesh".to_vec(),
    };
    assert_eq!(
        materialize_serialized_component(&mut server, &missing).unwrap_err(),
        EcsComponentMaterializationError::MissingField {
            component_type: "MeshRenderer".to_owned(),
            field: "material".to_owned(),
        }
    );

    let duplicate = SerializedComponent {
        type_name: "AudioSource".to_owned(),
        data: b"clip=audio/a.audio;volume=1;volume=0.5".to_vec(),
    };
    assert_eq!(
        materialize_serialized_component(&mut server, &duplicate).unwrap_err(),
        EcsComponentMaterializationError::DuplicateField {
            component_type: "AudioSource".to_owned(),
            field: "volume".to_owned(),
        }
    );

    let wrong_type = SerializedComponent {
        type_name: "MeshRenderer".to_owned(),
        data: b"mesh=materials/hero.material;material=materials/hero.material".to_vec(),
    };
    assert!(matches!(
        materialize_serialized_component(&mut server, &wrong_type).unwrap_err(),
        EcsComponentMaterializationError::AssetReference {
            component_type,
            error: AssetError::Decode { .. },
        } if component_type == "MeshRenderer"
    ));
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
    assert_eq!(
        component.instantiation_asset_references(&server).unwrap(),
        Some(vec![InstantiationAssetReference {
            entity_index: 1,
            component_index: 0,
            component_type: "MeshRenderer".to_owned(),
            field: "mesh".to_owned(),
            path: AssetPath::parse("meshes/tri.mesh"),
            asset_type: Mesh::TYPE_ID,
        }])
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
    assert_eq!(
        loaded_component
            .instantiation_asset_references(&server)
            .unwrap(),
        None
    );
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
fn scene_instance_component_instantiates_host_entities_and_marks_loaded() {
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

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingHostInstantiationSink::with_first_entity(10);
    let report = component
        .instantiate_host(&server, &mut host)
        .unwrap()
        .unwrap();

    assert_eq!(
        report,
        HostInstantiationReport {
            source: scene_id,
            entities: vec![10, 11],
            root_entities: vec![10],
            attached_component_count: 2,
        }
    );
    assert!(component.loaded);
    assert_eq!(
        host.spawns,
        vec![
            (0, 10, Some("Root".to_owned()), None),
            (1, 11, Some("Hero".to_owned()), Some(10)),
        ]
    );
    assert_eq!(
        host.attachments,
        vec![
            (
                10,
                0,
                0,
                "Transform".to_owned(),
                "translation=0,0,0".to_owned()
            ),
            (
                11,
                1,
                0,
                "MeshRenderer".to_owned(),
                "mesh=meshes/tri.mesh".to_owned()
            ),
        ]
    );
    assert_eq!(
        component.instantiate_host(&server, &mut host).unwrap(),
        None
    );
    assert_eq!(host.spawns.len(), 2);
}

#[test]
fn scene_instance_component_instantiates_typed_host_components() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();
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
                    components: vec![
                        SerializedComponent {
                            type_name: "MeshRenderer".to_owned(),
                            data: b"mesh=meshes/hero.mesh;material=materials/hero.material"
                                .to_vec(),
                        },
                        SerializedComponent {
                            type_name: "AudioSource".to_owned(),
                            data: b"clip=audio/click.audio;looping=true;volume=0.5".to_vec(),
                        },
                    ],
                },
            ],
            dependencies: Vec::new(),
        },
    );

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_first_entity(30);
    let report = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap()
        .unwrap();

    assert_eq!(
        report,
        HostInstantiationReport {
            source: scene_id,
            entities: vec![30, 31],
            root_entities: vec![30],
            attached_component_count: 3,
        }
    );
    assert!(component.loaded);
    assert_eq!(
        host.spawns,
        vec![
            (0, 30, Some("Root".to_owned()), None),
            (1, 31, Some("Hero".to_owned()), Some(30)),
        ]
    );
    assert_eq!(
        host.attachments,
        vec![
            (30, 0, 0, "Transform".to_owned(), vec![]),
            (
                31,
                1,
                0,
                "MeshRenderer".to_owned(),
                vec![Mesh::TYPE_ID, Material::TYPE_ID],
            ),
            (31, 1, 1, "AudioSource".to_owned(), vec![AudioClip::TYPE_ID],),
        ]
    );
}

#[test]
fn typed_host_instantiation_materialization_error_does_not_spawn_or_mark_loaded() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();
    server.storage_mut::<SceneAsset>().unwrap().insert(
        scene_id,
        SceneAsset {
            name: "level".to_owned(),
            entities: vec![SerializedEntity {
                name: Some("Hero".to_owned()),
                parent: None,
                components: vec![SerializedComponent {
                    type_name: "MeshRenderer".to_owned(),
                    data: b"mesh=meshes/hero.mesh".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_first_entity(50);
    let error = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap_err();

    assert_eq!(
        error,
        TypedHostInstantiationError::Component {
            entity_index: 0,
            component_index: 0,
            error: EcsComponentMaterializationError::MissingField {
                component_type: "MeshRenderer".to_owned(),
                field: "material".to_owned(),
            },
        }
    );
    assert!(!component.loaded);
    assert!(host.spawns.is_empty());
    assert!(host.attachments.is_empty());
}

#[test]
fn scene_typed_host_instantiation_reports_missing_parent_without_marking_loaded() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<SceneAsset>();
    server.storage_mut::<SceneAsset>().unwrap().insert(
        scene_id,
        SceneAsset {
            name: "level".to_owned(),
            entities: vec![SerializedEntity {
                name: Some("Orphan".to_owned()),
                parent: Some(99),
                components: Vec::new(),
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_first_entity(60);
    let error = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap_err();

    assert_eq!(
        error,
        TypedHostInstantiationError::MissingParent {
            entity_index: 0,
            parent_index: 99,
        }
    );
    assert!(!component.loaded);
    assert!(host.spawns.is_empty());
    assert!(host.attachments.is_empty());
}

#[test]
fn scene_typed_host_instantiation_propagates_sink_attach_error_without_marking_loaded() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();
    server.storage_mut::<SceneAsset>().unwrap().insert(
        scene_id,
        SceneAsset {
            name: "level".to_owned(),
            entities: vec![SerializedEntity {
                name: Some("Hero".to_owned()),
                parent: None,
                components: vec![SerializedComponent {
                    type_name: "MeshRenderer".to_owned(),
                    data: b"mesh=meshes/hero.mesh;material=materials/hero.material".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_errors(80, None, Some((0, 0)));
    let error = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap_err();

    assert_eq!(
        error,
        TypedHostInstantiationError::Sink("attach:0:0".to_owned())
    );
    assert!(!component.loaded);
    assert_eq!(host.spawns.len(), 1);
    assert!(host.attachments.is_empty());
}

#[test]
fn scene_host_instantiation_reports_missing_parent_without_marking_loaded() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<SceneAsset>();
    server.storage_mut::<SceneAsset>().unwrap().insert(
        scene_id,
        SceneAsset {
            name: "level".to_owned(),
            entities: vec![SerializedEntity {
                name: Some("Orphan".to_owned()),
                parent: Some(99),
                components: Vec::new(),
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingHostInstantiationSink::with_first_entity(10);
    let error = component.instantiate_host(&server, &mut host).unwrap_err();

    assert_eq!(
        error,
        HostInstantiationError::MissingParent {
            entity_index: 0,
            parent_index: 99,
        }
    );
    assert!(!component.loaded);
    assert!(host.spawns.is_empty());
    assert!(host.attachments.is_empty());
}

#[test]
fn scene_host_instantiation_propagates_sink_attach_error_without_marking_loaded() {
    let scene_id = AssetId::new();
    let scene_handle = Handle::<SceneAsset>::strong(scene_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_asset_type::<SceneAsset>();
    server.storage_mut::<SceneAsset>().unwrap().insert(
        scene_id,
        SceneAsset {
            name: "level".to_owned(),
            entities: vec![SerializedEntity {
                name: Some("Hero".to_owned()),
                parent: None,
                components: vec![SerializedComponent {
                    type_name: "Transform".to_owned(),
                    data: b"translation=0,0,0".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = SceneInstanceComponent {
        scene: scene_handle,
        loaded: false,
    };
    let mut host = RecordingHostInstantiationSink::with_errors(11, None, Some((0, 0)));
    let error = component.instantiate_host(&server, &mut host).unwrap_err();

    assert_eq!(error, HostInstantiationError::Sink("attach:0:0".to_owned()));
    assert!(!component.loaded);
    assert_eq!(host.spawns.len(), 1);
    assert!(host.attachments.is_empty());
}

#[test]
fn prefab_typed_host_instantiation_propagates_sink_spawn_error_without_marking_loaded() {
    let prefab_id = AssetId::new();
    let prefab_handle = Handle::<Prefab>::strong(prefab_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();
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
                    data: b"mesh=meshes/weapon.mesh;material=materials/weapon.material".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_errors(90, Some(1), None);
    let error = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap_err();

    assert_eq!(
        error,
        TypedHostInstantiationError::Sink("spawn:1".to_owned())
    );
    assert!(!component.loaded);
    assert_eq!(host.spawns.len(), 1);
    assert_eq!(host.attachments.len(), 1);
}

#[test]
fn prefab_host_instantiation_propagates_sink_spawn_error_without_marking_loaded() {
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
                    data: b"mesh=meshes/weapon.mesh;material=materials/weapon.material".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: false,
    };
    let mut host = RecordingHostInstantiationSink::with_errors(21, Some(1), None);
    let error = component.instantiate_host(&server, &mut host).unwrap_err();

    assert_eq!(error, HostInstantiationError::Sink("spawn:1".to_owned()));
    assert!(!component.loaded);
    assert_eq!(host.spawns.len(), 1);
    assert_eq!(host.attachments.len(), 1);
}

#[test]
fn prefab_typed_host_instantiation_reports_missing_parent_without_marking_loaded() {
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
                parent: Some(99),
                components: vec![SerializedComponent {
                    type_name: "MeshRenderer".to_owned(),
                    data: b"mesh=meshes/weapon.mesh;material=materials/weapon.material".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_first_entity(70);
    let error = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap_err();

    assert_eq!(
        error,
        TypedHostInstantiationError::MissingParent {
            entity_index: 1,
            parent_index: 99,
        }
    );
    assert!(!component.loaded);
    assert!(host.spawns.is_empty());
    assert!(host.attachments.is_empty());
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
fn prefab_instance_component_instantiates_host_entities_and_marks_loaded() {
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

    let mut component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: false,
    };
    let mut host = RecordingHostInstantiationSink::with_first_entity(20);
    let report = component
        .instantiate_host(&server, &mut host)
        .unwrap()
        .unwrap();

    assert_eq!(
        report,
        HostInstantiationReport {
            source: prefab_id,
            entities: vec![20, 21],
            root_entities: vec![20],
            attached_component_count: 2,
        }
    );
    assert!(component.loaded);
    assert_eq!(
        host.spawns,
        vec![
            (0, 20, Some("Hero".to_owned()), None),
            (1, 21, Some("Weapon".to_owned()), Some(20)),
        ]
    );
    assert_eq!(
        component.instantiate_host(&server, &mut host).unwrap(),
        None
    );
}

#[test]
fn prefab_instance_component_instantiates_typed_host_components() {
    let prefab_id = AssetId::new();
    let prefab_handle = Handle::<Prefab>::strong(prefab_id);
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.register_builtin_loaders();
    server.storage_mut::<Prefab>().unwrap().insert(
        prefab_id,
        Prefab {
            root: SerializedEntity {
                name: Some("Hero".to_owned()),
                parent: None,
                components: vec![SerializedComponent {
                    type_name: "SceneInstance".to_owned(),
                    data: b"scene=scenes/level.scene".to_vec(),
                }],
            },
            children: vec![SerializedEntity {
                name: Some("Collider".to_owned()),
                parent: Some(0),
                components: vec![SerializedComponent {
                    type_name: "PhysicsCollider".to_owned(),
                    data: b"mesh=physics/hero.physics;dynamic=false".to_vec(),
                }],
            }],
            dependencies: Vec::new(),
        },
    );

    let mut component = PrefabInstanceComponent {
        prefab: prefab_handle,
        loaded: false,
    };
    let mut host = RecordingTypedHostInstantiationSink::with_first_entity(40);
    let report = component
        .instantiate_typed_host(&mut server, &mut host)
        .unwrap()
        .unwrap();

    assert_eq!(
        report,
        HostInstantiationReport {
            source: prefab_id,
            entities: vec![40, 41],
            root_entities: vec![40],
            attached_component_count: 2,
        }
    );
    assert!(component.loaded);
    assert_eq!(
        host.attachments,
        vec![
            (
                40,
                0,
                0,
                "SceneInstance".to_owned(),
                vec![SceneAsset::TYPE_ID],
            ),
            (
                41,
                1,
                0,
                "PhysicsCollider".to_owned(),
                vec![PhysicsMesh::TYPE_ID],
            ),
        ]
    );
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
    assert_eq!(
        component.instantiation_asset_references(&server).unwrap(),
        Some(vec![InstantiationAssetReference {
            entity_index: 1,
            component_index: 0,
            component_type: "MeshRenderer".to_owned(),
            field: "mesh".to_owned(),
            path: AssetPath::parse("meshes/weapon.mesh"),
            asset_type: Mesh::TYPE_ID,
        }])
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
    assert_eq!(
        loaded_component
            .instantiation_asset_references(&server)
            .unwrap(),
        None
    );
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
