use engine_asset::prelude::*;

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn mesh_bytes() -> Vec<u8> {
    b"v 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn finish_uploads(server: &mut AssetServer) {
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(1))),
    );
}

fn server_with_io(io: MemoryAssetIo) -> AssetServer {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

#[test]
fn dependency_failure_propagates_to_waiting_parent_asset() {
    let mut io = MemoryAssetIo::new();
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/missing_texture.material",
        "name=broken\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/missing.texture\nbase_color=1,1,1,1\n",
    );
    let mut server = server_with_io(io);
    let material: Handle<Material> = server.load("materials/missing_texture.material");

    server.update_loading();

    assert_eq!(server.state(&material), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(material.id()),
        Some(AssetError::DependencyFailed { .. })
    ));
    assert!(server.events().iter().any(|event| {
        matches!(
            event,
            AssetEvent::DependencyFailed { id, .. } if *id == material.id()
        )
    }));
}

#[test]
fn dependency_report_exports_edges_and_topological_order() {
    let mut io = MemoryAssetIo::new();
    let texture_path = "textures/<albedo>&hero.texture";
    io.insert(texture_path, texture_bytes(1, 1, 8));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        format!(
            "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo={texture_path}\nbase_color=1,1,1,1\n"
        ),
    );
    let mut server = server_with_io(io);
    let material: Handle<Material> = server.load("materials/hero.material");
    server.update_loading();

    let shader_id = server
        .id_from_path(&AssetPath::parse("shaders/pbr.wgsl"))
        .unwrap();
    let texture_id = server
        .id_from_path(&AssetPath::parse(texture_path))
        .unwrap();
    let report = server.dependency_report();
    assert!(report.edges.contains(&DependencyEdge {
        asset: material.id(),
        dependency: shader_id,
    }));
    assert!(report.edges.contains(&DependencyEdge {
        asset: material.id(),
        dependency: texture_id,
    }));
    let text = server.dependency_report_text();
    assert!(text.contains("NGA_DEPENDENCY_GRAPH_V1"));
    assert!(text.contains(&format!("asset|{}", material.id().raw())));
    assert!(text.contains(&format!("asset|{}", shader_id.raw())));
    assert!(text.contains(&format!(
        "edge|{}|{}",
        material.id().raw(),
        texture_id.raw()
    )));

    let dot = server.dependency_report_dot();
    assert!(dot.starts_with("digraph AssetDependencies"));
    assert!(dot.contains(&format!(
        "\"{}\" -> \"{}\";",
        material.id().raw(),
        shader_id.raw()
    )));
    let json = server.dependency_report_json();
    assert!(json.starts_with("{\"version\":1,\"assets\":["));
    assert!(json.contains(&format!("\"{}\"", material.id().raw())));
    assert!(json.contains(&format!(
        "{{\"asset\":\"{}\",\"dependency\":\"{}\"}}",
        material.id().raw(),
        texture_id.raw()
    )));
    let shader_edge = json
        .find(&format!(
            "{{\"asset\":\"{}\",\"dependency\":\"{}\"}}",
            material.id().raw(),
            shader_id.raw()
        ))
        .unwrap();
    let texture_edge = json
        .find(&format!(
            "{{\"asset\":\"{}\",\"dependency\":\"{}\"}}",
            material.id().raw(),
            texture_id.raw()
        ))
        .unwrap();
    assert!(shader_edge < texture_edge);

    let html = server.dependency_report_html();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Asset Dependency Graph"));
    assert!(html.contains("Assets: 3"));
    assert!(html.contains("Edges: 2"));
    assert!(html.contains(&format!("data-asset=\"{}\"", material.id().raw())));
    assert!(html.contains("textures/&lt;albedo&gt;&amp;hero.texture"));
    assert!(!html.contains(texture_path));

    let order = server
        .dependency_graph()
        .topological_order(material.id())
        .unwrap();
    assert_eq!(order.last().copied(), Some(material.id()));
    assert!(order.iter().position(|id| *id == shader_id).unwrap() < order.len() - 1);
    assert!(order.iter().position(|id| *id == texture_id).unwrap() < order.len() - 1);
}

#[test]
fn scoped_dependency_report_exports_root_subgraph_and_missing_root_errors() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 8));
    io.insert("textures/unrelated.texture", texture_bytes(1, 1, 16));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert("meshes/tri.mesh", mesh_bytes());
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    io.insert(
        "scenes/level.scene",
        "NGA_SCENE_V1\nname=level\ndependency=meshes/tri.mesh\ndependency=materials/hero.material\n",
    );
    let mut server = server_with_io(io);
    let scene: Handle<SceneAsset> = server.load("scenes/level.scene");
    let unrelated: Handle<Texture> = server.load("textures/unrelated.texture");
    for _ in 0..8 {
        server.update_loading();
        finish_uploads(&mut server);
        if server.is_ready(&scene) && server.is_ready(&unrelated) {
            break;
        }
    }

    let mesh_id = server
        .id_from_path(&AssetPath::parse("meshes/tri.mesh"))
        .unwrap();
    let material_id = server
        .id_from_path(&AssetPath::parse("materials/hero.material"))
        .unwrap();
    let shader_id = server
        .id_from_path(&AssetPath::parse("shaders/pbr.wgsl"))
        .unwrap();
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    let scene_scope = server.scoped_dependency_report(scene.id()).unwrap();
    assert_eq!(scene_scope.root, scene.id());
    assert_eq!(scene_scope.direct_dependencies, vec![mesh_id, material_id]);
    let mut expected_transitive = vec![mesh_id, material_id, shader_id, texture_id];
    expected_transitive.sort();
    assert_eq!(scene_scope.transitive_dependencies, expected_transitive);
    assert!(scene_scope.direct_dependents.is_empty());
    assert!(scene_scope.transitive_dependents.is_empty());
    assert!(scene_scope.graph.edges.contains(&DependencyEdge {
        asset: scene.id(),
        dependency: material_id,
    }));
    assert!(scene_scope.graph.edges.contains(&DependencyEdge {
        asset: material_id,
        dependency: texture_id,
    }));

    let texture_scope = server.scoped_dependency_report(texture_id).unwrap();
    assert_eq!(texture_scope.direct_dependents, vec![material_id]);
    assert_eq!(
        texture_scope.transitive_dependents,
        vec![scene.id(), material_id]
    );
    assert!(texture_scope.graph.edges.contains(&DependencyEdge {
        asset: scene.id(),
        dependency: material_id,
    }));
    let text = server.scoped_dependency_report_text(scene.id()).unwrap();
    assert!(text.starts_with("NGA_DEPENDENCY_SCOPE_V1"));
    assert!(text.contains(&format!("root={}", scene.id().raw())));
    let dot = server.scoped_dependency_report_dot(scene.id()).unwrap();
    assert!(dot.contains(&format!(
        "\"{}\" -> \"{}\";",
        scene.id().raw(),
        material_id.raw()
    )));
    let json = server.scoped_dependency_report_json(scene.id()).unwrap();
    assert!(json.contains(&format!("\"root\":\"{}\"", scene.id().raw())));
    assert!(json.contains("\"direct_dependencies\""));
    assert!(json.contains("\"transitive_dependents\""));
    assert!(json.contains("\"graph\":{\"version\":1"));
    let html = server.scoped_dependency_report_html(scene.id()).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Asset Dependency Scope"));
    assert!(html.contains(&format!("data-root=\"{}\"", scene.id().raw())));
    assert!(html.contains(&format!("<code>{}</code>", material_id.raw())));
    assert!(!html.contains(&unrelated.id().raw().to_string()));

    let text_path = std::env::temp_dir().join("asset_smoke_scoped_dependency.txt");
    let dot_path = std::env::temp_dir().join("asset_smoke_scoped_dependency.dot");
    let json_path = std::env::temp_dir().join("asset_smoke_scoped_dependency.json");
    let html_path = std::env::temp_dir().join("asset_smoke_scoped_dependency.html");
    server
        .save_scoped_dependency_report_text(scene.id(), &text_path)
        .unwrap();
    server
        .save_scoped_dependency_report_dot(scene.id(), &dot_path)
        .unwrap();
    server
        .save_scoped_dependency_report_json(scene.id(), &json_path)
        .unwrap();
    server
        .save_scoped_dependency_report_html(scene.id(), &html_path)
        .unwrap();
    assert_eq!(std::fs::read_to_string(text_path).unwrap(), text);
    assert_eq!(std::fs::read_to_string(dot_path).unwrap(), dot);
    assert_eq!(std::fs::read_to_string(json_path).unwrap(), json);
    assert_eq!(std::fs::read_to_string(html_path).unwrap(), html);

    assert!(matches!(
        server.scoped_dependency_report(AssetId::from_u128(0xdead_beef)),
        Err(AssetError::AssetNotFound { .. })
    ));
    assert!(matches!(
        server.scoped_dependency_report_html(AssetId::from_u128(0xdead_beef)),
        Err(AssetError::AssetNotFound { .. })
    ));
}

#[derive(Clone, Debug)]
struct CyclicA;

impl Asset for CyclicA {
    const TYPE_NAME: &'static str = "CyclicA";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0001);
}

#[derive(Clone, Debug)]
struct CyclicB;

impl Asset for CyclicB {
    const TYPE_NAME: &'static str = "CyclicB";
    const TYPE_ID: AssetTypeId = AssetTypeId::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0002);
}

struct CyclicALoader;

impl AssetLoader for CyclicALoader {
    fn name(&self) -> &'static str {
        "CyclicALoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["cyca"]
    }

    fn asset_type(&self) -> AssetTypeId {
        CyclicA::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        _bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        ctx.dependency::<CyclicB>("cycles/b.cycb");
        Ok(LoadedAsset::new(CyclicA))
    }
}

struct CyclicBLoader;

impl AssetLoader for CyclicBLoader {
    fn name(&self) -> &'static str {
        "CyclicBLoader"
    }

    fn extensions(&self) -> &[&'static str] {
        &["cycb"]
    }

    fn asset_type(&self) -> AssetTypeId {
        CyclicB::TYPE_ID
    }

    fn load(
        &self,
        ctx: &mut LoadContext<'_>,
        _bytes: &[u8],
        _settings: &LoaderSettings,
    ) -> Result<LoadedAsset, AssetLoadError> {
        ctx.dependency::<CyclicA>("cycles/a.cyca");
        Ok(LoadedAsset::new(CyclicB))
    }
}

#[test]
fn cyclic_dependencies_fail_with_user_visible_cycle_error() {
    let io = MemoryAssetIo::new()
        .with_file("cycles/a.cyca", b"a".to_vec())
        .with_file("cycles/b.cycb", b"b".to_vec());
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_asset_type::<CyclicA>();
    server.register_asset_type::<CyclicB>();
    server.register_loader(CyclicALoader);
    server.register_loader(CyclicBLoader);

    let asset: Handle<CyclicA> = server.load("cycles/a.cyca");
    server.update_loading();

    let dependency_id = server
        .id_from_path(&AssetPath::parse("cycles/b.cycb"))
        .unwrap();
    assert_eq!(server.state_by_id(dependency_id), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(dependency_id),
        Some(AssetError::CyclicDependency)
    ));
    assert_eq!(server.state(&asset), AssetLoadState::Failed);
    assert!(server.dependency_graph().has_cycle_from(asset.id()));
}
