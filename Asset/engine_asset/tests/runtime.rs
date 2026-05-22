use engine_asset::prelude::*;

fn server_with_io(io: MemoryAssetIo) -> AssetServer {
    let mut server = AssetServer::new(AssetServerConfig::default());
    server.set_io(io);
    server.register_builtin_loaders();
    server
}

fn texture_bytes(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend(std::iter::repeat(value).take(width as usize * height as usize * 4));
    bytes
}

fn audio_bytes() -> Vec<u8> {
    b"NGA_AUDIO_V1\nsample_rate=48000\nchannels=2\nformat=i16\nsamples=0,1000,-1000,0\nstreaming=false\n"
        .to_vec()
}

fn wav_pcm16_bytes(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4 + (8 + 16) + (8 + data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn wav_float32_bytes(sample_rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 4) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(4 + (8 + 16) + (8 + data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * u32::from(channels) * 4).to_le_bytes());
    bytes.extend_from_slice(&(channels * 4).to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn animation_bytes() -> Vec<u8> {
    b"NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=bone:Root\ntranslation=0:0,0,0\nrotation=0:0,0,0,1\nscale=0:1,1,1\n"
        .to_vec()
}

fn skeleton_bytes() -> Vec<u8> {
    b"NGA_SKELETON_V1\nbone=Root\nbone=Spine;parent=0\n".to_vec()
}

fn font_bytes() -> Vec<u8> {
    b"NGA_FONT_V1\nfamily=Debug Sans\nglyph=char=A;size=2x1;bitmap=0,255\n".to_vec()
}

fn physics_mesh_bytes() -> Vec<u8> {
    b"NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn mesh_bytes() -> Vec<u8> {
    b"v 0 0 0\nv 1 0 0\nv 0 1 0\ni 0 1 2\n".to_vec()
}

fn binary_mesh_bytes() -> Vec<u8> {
    binary_mesh_bytes_with_weights([
        [0.7, 0.2, 0.1, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.25, 0.25, 0.25, 0.25],
    ])
}

fn binary_mesh_bytes_with_weights(weights: [[f32; 4]; 3]) -> Vec<u8> {
    let mut bytes = b"NGA_MESH_BINARY_V1\n".to_vec();
    push_u32(&mut bytes, 3);
    push_u32(&mut bytes, 3);
    push_u32(&mut bytes, 1 | 2 | 8);
    push_u32(&mut bytes, 1);
    for vertex in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        push_f32s(&mut bytes, &vertex);
    }
    for normal in [[0.0, 0.0, 1.0]; 3] {
        push_f32s(&mut bytes, &normal);
    }
    for uv in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]] {
        push_f32s(&mut bytes, &uv);
    }
    for uv in [[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]] {
        push_f32s(&mut bytes, &uv);
    }
    for joint in [
        [0u16, 1u16, 2u16, 3u16],
        [0u16, 0u16, 0u16, 0u16],
        [1u16, 2u16, 3u16, 4u16],
    ] {
        for value in joint {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for weight in weights {
        push_f32s(&mut bytes, &weight);
    }
    for index in [0, 1, 2] {
        push_u32(&mut bytes, index);
    }
    bytes
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f32s<const N: usize>(bytes: &mut Vec<u8>, values: &[f32; N]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn shader_bytes() -> Vec<u8> {
    b"/* ignored comment with braces {} */\n@group(2)\n@binding(3)\nvar<storage, read_write> particles: array<u32>;\n@compute @workgroup_size(1) fn main() {}\n".to_vec()
}

fn scene_bytes() -> Vec<u8> {
    b"NGA_SCENE_V1\nname=level\ndependency=meshes/tri.mesh\ndependency=materials/hero.material\nentity=Root\ncomponent=Transform|translation=0,0,0\nentity=Hero;parent=0\ncomponent=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material\n".to_vec()
}

fn prefab_bytes() -> Vec<u8> {
    b"NGA_PREFAB_V1\ndependency=meshes/tri.mesh\ndependency=materials/hero.material\nroot=Hero\ncomponent=Transform|translation=0,0,0\nchild=Weapon;parent=0\ncomponent=MeshRenderer|mesh=meshes/tri.mesh;material=materials/hero.material\n".to_vec()
}

fn assert_material_decode_error(source: &str, expected_message: &str) {
    let io =
        MemoryAssetIo::new().with_file("materials/broken.material", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let material: Handle<Material> = server.load("materials/broken.material");
    server.update_loading();

    assert_eq!(server.state(&material), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(material.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == material.id())));
}

fn assert_scene_decode_error(source: &str, expected_message: &str) {
    let io = MemoryAssetIo::new().with_file("scenes/broken.scene", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let scene: Handle<SceneAsset> = server.load("scenes/broken.scene");
    server.update_loading();

    assert_eq!(server.state(&scene), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(scene.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == scene.id())));
}

fn assert_prefab_decode_error(source: &str, expected_message: &str) {
    let io = MemoryAssetIo::new().with_file("prefabs/broken.prefab", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let prefab: Handle<Prefab> = server.load("prefabs/broken.prefab");
    server.update_loading();

    assert_eq!(server.state(&prefab), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(prefab.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == prefab.id())));
}

fn assert_skeleton_decode_error(source: &str, expected_message: &str) {
    let io =
        MemoryAssetIo::new().with_file("skeletons/broken.skeleton", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let skeleton: Handle<Skeleton> = server.load("skeletons/broken.skeleton");
    server.update_loading();

    assert_eq!(server.state(&skeleton), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(skeleton.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == skeleton.id())));
}

fn assert_animation_decode_error(source: &str, expected_message: &str) {
    let io =
        MemoryAssetIo::new().with_file("animations/broken.animation", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let animation: Handle<AnimationClip> = server.load("animations/broken.animation");
    server.update_loading();

    assert_eq!(server.state(&animation), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(animation.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == animation.id())));
}

fn assert_font_decode_error(source: &str, expected_message: &str) {
    let io = MemoryAssetIo::new().with_file("fonts/broken.font", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let font: Handle<Font> = server.load("fonts/broken.font");
    server.update_loading();

    assert_eq!(server.state(&font), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(font.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == font.id())));
}

fn assert_physics_mesh_decode_error(source: &str, expected_message: &str) {
    let io = MemoryAssetIo::new().with_file("physics/broken.physics", source.as_bytes().to_vec());
    let mut server = server_with_io(io);

    let mesh: Handle<PhysicsMesh> = server.load("physics/broken.physics");
    server.update_loading();

    assert_eq!(server.state(&mesh), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(mesh.id()),
        Some(AssetError::Decode { message }) if message.contains(expected_message)
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == mesh.id())));
}

fn finish_all_uploads(server: &mut AssetServer) {
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(uploads.into_iter().enumerate().map(|(index, upload)| {
        GpuUploadResult::ok(upload.id, GpuResourceHandle(index as u64 + 100))
    }));
}

#[test]
fn invalid_material_syntax_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\nthis line has no equals\n",
        "invalid material line 2",
    );
}

#[test]
fn invalid_material_numeric_property_fails_with_decode_error_and_event() {
    assert_material_decode_error("name=broken\nmetallic=shiny\n", "invalid float on line 2");
}

#[test]
fn invalid_material_bool_property_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ndouble_sided=sometimes\n",
        "invalid bool on line 2",
    );
}

#[test]
fn invalid_material_alpha_mode_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\nalpha_mode=screen\n",
        "invalid material alpha mode `screen` on line 2",
    );
}

#[test]
fn invalid_material_sampler_property_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ntexture.albedo.sampler.address=mirror\n",
        "invalid material sampler address `mirror` on line 2",
    );
}

#[test]
fn invalid_material_texture_transform_property_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ntexture.albedo.transform.scale=1,2\n",
        "expected three values on line 2",
    );
}

#[test]
fn invalid_material_texture_source_channel_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ntexture.albedo.source_channel=alpha\n",
        "invalid material texture source channel `alpha` on line 2",
    );
}

#[test]
fn invalid_material_texture_projection_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ntexture.albedo.projection=octahedral\n",
        "invalid material texture projection `octahedral` on line 2",
    );
}

#[test]
fn invalid_material_texture_resolution_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ntexture.albedo.texture_resolution=0\n",
        "material texture resolution must be greater than zero on line 2",
    );
}

#[test]
fn invalid_material_custom_property_fails_with_decode_error_and_event() {
    assert_material_decode_error(
        "name=broken\ncustom.tint.vec3=1,2\n",
        "expected three values on line 2",
    );
}

#[test]
fn material_load_applies_texture_metadata() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 128));
    io.insert(
        "materials/sampled.material",
        "name=sampled\ntexture.albedo.sampler.address=clamp_to_edge\ntexture.albedo.sampler.filter=nearest\ntexture.albedo.transform.offset=0.25,0.5,0\ntexture.albedo.transform.scale=2,3,1\ntexture.albedo.transform.turbulence=0.01,0.02,0.03\ntexture.albedo.bump_scale=0.3\ntexture.albedo.color_remap=0.1,0.9\ntexture.albedo.source_channel=green\ntexture.albedo.boost=1.5\ntexture.albedo.blend_u=false\ntexture.albedo.blend_v=true\ntexture.albedo.color_correction=true\ntexture.albedo.projection=sphere\ntexture.albedo.texture_resolution=1024\ntexture.albedo=textures/albedo.texture\nemissive=0.1,0.2,0.3\nalpha_cutoff=0.45\nalpha_mode=mask\ndouble_sided=true\ndepth_write=false\ndepth_test=false\n",
    );
    let mut server = server_with_io(io);

    let material: Handle<Material> = server.load("materials/sampled.material");
    for _ in 0..8 {
        server.update_loading();
        finish_all_uploads(&mut server);
        if server.is_ready(&material) {
            break;
        }
    }

    assert!(server.is_ready_with_dependencies(&material));
    let loaded = server.get(&material).unwrap();
    assert_eq!(loaded.textures.len(), 1);
    assert_eq!(loaded.textures[0].name, "albedo");
    assert_eq!(loaded.textures[0].sampler.address, AddressMode::ClampToEdge);
    assert_eq!(loaded.textures[0].sampler.filter, FilterMode::Nearest);
    assert_eq!(
        loaded.textures[0].options.transform.offset,
        [0.25, 0.5, 0.0]
    );
    assert_eq!(loaded.textures[0].options.transform.scale, [2.0, 3.0, 1.0]);
    assert_eq!(
        loaded.textures[0].options.transform.turbulence,
        [0.01, 0.02, 0.03]
    );
    assert_eq!(loaded.textures[0].options.bump_scale, Some(0.3));
    assert_eq!(loaded.textures[0].options.color_remap, Some([0.1, 0.9]));
    assert_eq!(
        loaded.textures[0].options.source_channel,
        Some(MaterialTextureChannel::Green)
    );
    assert_eq!(loaded.textures[0].options.boost, Some(1.5));
    assert_eq!(loaded.textures[0].options.blend_u, Some(false));
    assert_eq!(loaded.textures[0].options.blend_v, Some(true));
    assert_eq!(loaded.textures[0].options.color_correction, Some(true));
    assert_eq!(
        loaded.textures[0].options.projection,
        Some(MaterialTextureProjection::Sphere)
    );
    assert_eq!(loaded.textures[0].options.texture_resolution, Some(1024));
    assert_eq!(loaded.properties.emissive, [0.1, 0.2, 0.3]);
    assert_eq!(loaded.properties.alpha_cutoff, Some(0.45));
    assert_eq!(loaded.render_state.alpha_mode, AlphaMode::Mask);
    assert!(loaded.render_state.double_sided);
    assert!(!loaded.render_state.depth_write);
    assert!(!loaded.render_state.depth_test);
}

#[test]
fn material_load_applies_typed_custom_properties() {
    let io = MemoryAssetIo::new().with_file(
        "materials/custom.material",
        b"name=custom
custom.scalar=0.25
custom.clearcoat.float=0.7
custom.uv_scale.vec2=2,3
custom.tint.vec3=0.1,0.2,0.3
custom.clip_plane.vec4=1,0,0,-1
custom.illumination_model.int=4
custom.use_transmission.bool=true
legacy_float=0.5
"
        .to_vec(),
    );
    let mut server = server_with_io(io);

    let material: Handle<Material> = server.load("materials/custom.material");
    for _ in 0..8 {
        server.update_loading();
        finish_all_uploads(&mut server);
        if server.is_ready(&material) {
            break;
        }
    }

    assert!(server.is_ready(&material));
    let loaded = server.get(&material).unwrap();
    assert_eq!(
        loaded.properties.custom.get("scalar"),
        Some(&MaterialPropertyValue::Float(0.25))
    );
    assert_eq!(
        loaded.properties.custom.get("clearcoat"),
        Some(&MaterialPropertyValue::Float(0.7))
    );
    assert_eq!(
        loaded.properties.custom.get("uv_scale"),
        Some(&MaterialPropertyValue::Vec2([2.0, 3.0]))
    );
    assert_eq!(
        loaded.properties.custom.get("tint"),
        Some(&MaterialPropertyValue::Vec3([0.1, 0.2, 0.3]))
    );
    assert_eq!(
        loaded.properties.custom.get("clip_plane"),
        Some(&MaterialPropertyValue::Vec4([1.0, 0.0, 0.0, -1.0]))
    );
    assert_eq!(
        loaded.properties.custom.get("illumination_model"),
        Some(&MaterialPropertyValue::Int(4))
    );
    assert_eq!(
        loaded.properties.custom.get("use_transmission"),
        Some(&MaterialPropertyValue::Bool(true))
    );
    assert_eq!(
        loaded.properties.custom.get("legacy_float"),
        Some(&MaterialPropertyValue::Float(0.5))
    );
}

#[test]
fn shader_load_reaches_ready_after_renderer_upload_handoff_and_selects_compute_stage() {
    let io = MemoryAssetIo::new().with_file("shaders/compute.wgsl", shader_bytes());
    let mut server = server_with_io(io);

    let shader: Handle<Shader> = server.load("shaders/compute.wgsl#compute");
    server.update_loading();

    assert_eq!(server.state(&shader), AssetLoadState::UploadingGpu);
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].id, shader.id());
    assert_eq!(uploads[0].kind, GpuUploadKind::Shader);
    assert_eq!(
        uploads[0].label.as_deref(),
        Some("shaders/compute.wgsl#compute")
    );

    server.finish_gpu_uploads(vec![GpuUploadResult::ok(
        shader.id(),
        GpuResourceHandle(21),
    )]);

    assert!(server.is_ready(&shader));
    let loaded = server.get(&shader).unwrap();
    assert_eq!(loaded.stages.len(), 1);
    assert_eq!(loaded.stages[0].stage, ShaderStage::Compute);
    assert!(matches!(
        &loaded.stages[0].source,
        ShaderSource::Wgsl(source) if source.contains("@compute")
    ));
    let reflection = loaded.reflection.as_ref().unwrap();
    assert_eq!(
        reflection.bind_groups,
        vec!["group=2,binding=3,name=particles"]
    );
    assert!(reflection.vertex_inputs.is_empty());
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(21)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == shader.id())));
}

#[test]
fn shader_load_populates_vertex_reflection_metadata() {
    let source = b"/* ignored comment with braces {} */\nstruct VertexInput {\n  @location(0) position: vec3<f32>,\n  @location(1) uv: vec2<f32>,\n};\n@group(0)\n@binding(1)\nvar<uniform> camera: mat4x4<f32>;\n@vertex fn main(input: VertexInput) -> @builtin(position) vec4<f32> {\n  return camera * vec4<f32>(input.position, 1.0);\n}\n".to_vec();
    let io = MemoryAssetIo::new().with_file("shaders/mesh.wgsl", source);
    let mut server = server_with_io(io);

    let shader: Handle<Shader> = server.load("shaders/mesh.wgsl#vertex");
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    server.finish_gpu_uploads(vec![GpuUploadResult::ok(
        shader.id(),
        GpuResourceHandle(22),
    )]);

    assert!(server.is_ready(&shader));
    let loaded = server.get(&shader).unwrap();
    assert_eq!(loaded.stages[0].stage, ShaderStage::Vertex);
    let reflection = loaded.reflection.as_ref().unwrap();
    assert_eq!(
        reflection.bind_groups,
        vec!["group=0,binding=1,name=camera"]
    );
    assert_eq!(
        reflection.vertex_inputs,
        vec!["location=0,name=position", "location=1,name=uv"]
    );
}

#[test]
fn invalid_shader_payload_fails_with_decode_error_and_event() {
    let cases = vec![
        (
            "shaders/empty.wgsl",
            b"   \n\t".to_vec(),
            "shader source is empty",
        ),
        (
            "shaders/unclosed.wgsl",
            b"@fragment fn main() {\n".to_vec(),
            "shader source has unclosed `{`",
        ),
        (
            "shaders/malformed_binding.wgsl",
            b"@group(0) var<uniform> camera: mat4x4<f32>;\n@fragment fn main() {}\n".to_vec(),
            "shader resource binding on line 1 must include both @group and @binding",
        ),
        (
            "shaders/unclosed_block_comment.wgsl",
            b"/* unclosed shader comment with } brace\n@fragment fn main() {}\n".to_vec(),
            "shader source has unclosed block comment",
        ),
    ];

    for (path, source, expected_message) in cases {
        let io = MemoryAssetIo::new().with_file(path, source);
        let mut server = server_with_io(io);

        let shader: Handle<Shader> = server.load(path);
        server.update_loading();

        assert_eq!(server.state(&shader), AssetLoadState::Failed);
        assert!(matches!(
            server.error_by_id(shader.id()),
            Some(AssetError::Decode { message }) if message.contains(expected_message)
        ));
        assert!(server
            .events()
            .iter()
            .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == shader.id())));
    }
}

#[test]
fn invalid_shader_stage_label_fails_with_decode_error() {
    let io =
        MemoryAssetIo::new().with_file("shaders/pbr.wgsl", b"@fragment fn main() {}\n".to_vec());
    let mut server = server_with_io(io);

    let shader: Handle<Shader> = server.load("shaders/pbr.wgsl#geometry");
    server.update_loading();

    assert_eq!(server.state(&shader), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(shader.id()),
        Some(AssetError::Decode { message })
            if message.contains("unsupported shader stage label `geometry`")
    ));
}

#[test]
fn mesh_load_reaches_ready_after_renderer_upload_handoff() {
    let io = MemoryAssetIo::new().with_file("meshes/tri.mesh", mesh_bytes());
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/tri.mesh");
    server.update_loading();

    assert_eq!(server.state(&mesh), AssetLoadState::UploadingGpu);
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].id, mesh.id());
    assert_eq!(uploads[0].kind, GpuUploadKind::Mesh);
    let GpuUploadMetadata::Mesh(metadata) = &uploads[0].metadata else {
        panic!("mesh upload should expose binary mesh metadata");
    };
    assert_eq!(metadata.layout.vertex_count, 3);
    assert_eq!(metadata.layout.stride, 12);
    assert_eq!(metadata.vertex_buffer_bytes, 36);
    assert_eq!(metadata.index_buffer_bytes, 12);
    assert_eq!(metadata.index_count, 3);
    assert_eq!(
        metadata.layout.attributes,
        vec![MeshVertexAttribute {
            semantic: MeshVertexSemantic::Position,
            format: MeshVertexFormat::Float32x3,
            offset: 0
        }]
    );
    assert_eq!(uploads[0].bytes.len(), 48);
    assert_eq!(
        &uploads[0].bytes[36..],
        &[0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]
    );

    server.finish_gpu_uploads(vec![GpuUploadResult::ok(mesh.id(), GpuResourceHandle(12))]);

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(
        loaded.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert!(loaded.normals.is_empty());
    assert!(loaded.uvs.is_empty());
    assert!(loaded.uv_sets.is_empty());
    assert!(loaded.tangents.is_empty());
    assert!(loaded.joints.is_empty());
    assert!(loaded.weights.is_empty());
    assert_eq!(loaded.indices, vec![0, 1, 2]);
    assert_eq!(loaded.vertex_buffer.layout.vertex_count, 3);
    assert_eq!(loaded.vertex_buffer.layout.stride, 12);
    assert_eq!(loaded.vertex_buffer.bytes.len(), 36);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(12)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == mesh.id())));
}

#[test]
fn mesh_load_preserves_secondary_uvs_and_skinning_attributes() {
    let source = b"v 0 0 0
v 1 0 0
v 0 1 0
uv 0 0
uv 1 0
uv 0 1
uv1 0.25 0.25
uv1 0.75 0.25
uv1 0.25 0.75
j 0 1 2 3
j 0 0 0 0
j 1 2 3 4
w 0.7 0.2 0.1 0
w 1 0 0 0
w 0.25 0.25 0.25 0.25
i 0 1 2
"
    .to_vec();
    let io = MemoryAssetIo::new().with_file("meshes/skinned.mesh", source);
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/skinned.mesh");
    server.update_loading();
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    let GpuUploadMetadata::Mesh(metadata) = &uploads[0].metadata else {
        panic!("mesh upload should expose binary mesh metadata");
    };
    assert_eq!(metadata.layout.vertex_count, 3);
    assert_eq!(metadata.layout.stride, 52);
    assert_eq!(metadata.vertex_buffer_bytes, 156);
    assert_eq!(metadata.index_buffer_bytes, 12);
    assert_eq!(metadata.index_count, 3);
    assert_eq!(
        metadata.layout.attributes,
        vec![
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Position,
                format: MeshVertexFormat::Float32x3,
                offset: 0,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::TexCoord(0),
                format: MeshVertexFormat::Float32x2,
                offset: 12,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::TexCoord(1),
                format: MeshVertexFormat::Float32x2,
                offset: 20,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Joints,
                format: MeshVertexFormat::Uint16x4,
                offset: 28,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Weights,
                format: MeshVertexFormat::Float32x4,
                offset: 36,
            },
        ]
    );
    assert_eq!(uploads[0].bytes.len(), 168);
    server.finish_gpu_uploads(vec![GpuUploadResult::ok(mesh.id(), GpuResourceHandle(13))]);

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(loaded.uvs, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(
        loaded.uv_sets,
        vec![vec![[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]]]
    );
    assert_eq!(
        loaded.joints,
        vec![[0, 1, 2, 3], [0, 0, 0, 0], [1, 2, 3, 4]]
    );
    assert_eq!(
        loaded.weights,
        vec![
            [0.7, 0.2, 0.1, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.25, 0.25, 0.25, 0.25]
        ]
    );
    assert_eq!(loaded.vertex_buffer.layout, metadata.layout);
    assert_eq!(
        loaded.vertex_buffer.bytes.len(),
        metadata.vertex_buffer_bytes as usize
    );
}

#[test]
fn binary_mesh_load_reaches_ready_with_layout_metadata() {
    let io = MemoryAssetIo::new().with_file("meshes/binary.mesh", binary_mesh_bytes());
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/binary.mesh");
    server.update_loading();

    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    let GpuUploadMetadata::Mesh(metadata) = &uploads[0].metadata else {
        panic!("binary mesh upload should expose mesh metadata");
    };
    assert_eq!(metadata.layout.vertex_count, 3);
    assert_eq!(metadata.layout.stride, 64);
    assert_eq!(metadata.vertex_buffer_bytes, 192);
    assert_eq!(metadata.index_buffer_bytes, 12);
    assert_eq!(metadata.index_count, 3);
    assert_eq!(
        metadata.layout.attributes,
        vec![
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Position,
                format: MeshVertexFormat::Float32x3,
                offset: 0,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Normal,
                format: MeshVertexFormat::Float32x3,
                offset: 12,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::TexCoord(0),
                format: MeshVertexFormat::Float32x2,
                offset: 24,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::TexCoord(1),
                format: MeshVertexFormat::Float32x2,
                offset: 32,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Joints,
                format: MeshVertexFormat::Uint16x4,
                offset: 40,
            },
            MeshVertexAttribute {
                semantic: MeshVertexSemantic::Weights,
                format: MeshVertexFormat::Float32x4,
                offset: 48,
            },
        ]
    );

    server.finish_gpu_uploads(vec![GpuUploadResult::ok(mesh.id(), GpuResourceHandle(14))]);

    assert!(server.is_ready(&mesh));
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(
        loaded.vertices,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(loaded.normals, vec![[0.0, 0.0, 1.0]; 3]);
    assert_eq!(loaded.uvs, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(
        loaded.uv_sets,
        vec![vec![[0.25, 0.25], [0.75, 0.25], [0.25, 0.75]]]
    );
    assert_eq!(
        loaded.joints,
        vec![[0, 1, 2, 3], [0, 0, 0, 0], [1, 2, 3, 4]]
    );
    assert_eq!(loaded.indices, vec![0, 1, 2]);
    assert_eq!(loaded.vertex_buffer.layout, metadata.layout);
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(14)));
}

#[test]
fn invalid_mesh_payload_fails_with_decode_error_and_event() {
    let io = MemoryAssetIo::new().with_file("meshes/broken.mesh", b"v 0 0 0\ni 0 1 2\n".to_vec());
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/broken.mesh");
    server.update_loading();

    assert_eq!(server.state(&mesh), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(mesh.id()),
        Some(AssetError::Decode { message })
            if message.contains("mesh index 1 references missing vertex")
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == mesh.id())));
}

#[test]
fn invalid_binary_mesh_payload_fails_with_decode_error_and_event() {
    let mut bytes = b"NGA_MESH_BINARY_V1\n".to_vec();
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, 3);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    let io = MemoryAssetIo::new().with_file("meshes/broken_binary.mesh", bytes);
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/broken_binary.mesh");
    server.update_loading();

    assert_eq!(server.state(&mesh), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(mesh.id()),
        Some(AssetError::Decode { message })
            if message.contains("mesh binary payload byte length mismatch")
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == mesh.id())));
}

#[test]
fn invalid_mesh_attribute_count_fails_with_decode_error_and_event() {
    let io = MemoryAssetIo::new().with_file(
        "meshes/broken_attributes.mesh",
        b"v 0 0 0\nv 1 0 0\nt 1 0 0 1\ni 0 1 1\n".to_vec(),
    );
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/broken_attributes.mesh");
    server.update_loading();

    assert_eq!(server.state(&mesh), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(mesh.id()),
        Some(AssetError::Decode { message })
            if message.contains("mesh tangent count 1 must match vertex count 2")
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == mesh.id())));
}

#[test]
fn invalid_mesh_skinning_attribute_count_fails_with_decode_error_and_event() {
    let io = MemoryAssetIo::new().with_file(
        "meshes/broken_skin.mesh",
        b"v 0 0 0\nv 1 0 0\nj 0 1 2 3\nw 1 0 0 0\ni 0 1 1\n".to_vec(),
    );
    let mut server = server_with_io(io);

    let mesh: Handle<Mesh> = server.load("meshes/broken_skin.mesh");
    server.update_loading();

    assert_eq!(server.state(&mesh), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(mesh.id()),
        Some(AssetError::Decode { message })
            if message.contains("mesh skin joint count 1 must match vertex count 2")
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == mesh.id())));
}

#[test]
fn invalid_mesh_skin_weight_total_fails_with_decode_error_and_event() {
    let cases = vec![
        (
            "meshes/zero_skin_weight.mesh",
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 2 3\nj 0 0 0 0\nj 1 2 3 4\nw 0 0 0 0\nw 1 0 0 0\nw 0.25 0.25 0.25 0.25\ni 0 1 2\n".to_vec(),
            "mesh skin weight total must be positive",
        ),
        (
            "meshes/unnormalized_skin_weight.mesh",
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nj 0 1 2 3\nj 0 0 0 0\nj 1 2 3 4\nw 2 0 0 0\nw 1 0 0 0\nw 0.25 0.25 0.25 0.25\ni 0 1 2\n".to_vec(),
            "mesh skin weights on line 7 must sum to 1.0",
        ),
        (
            "meshes/binary_unnormalized_skin_weight.mesh",
            binary_mesh_bytes_with_weights([
                [2.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
                [0.25, 0.25, 0.25, 0.25],
            ]),
            "mesh binary skin weights at vertex 0 must sum to 1.0",
        ),
    ];

    for (path, bytes, expected_message) in cases {
        let io = MemoryAssetIo::new().with_file(path, bytes);
        let mut server = server_with_io(io);

        let mesh: Handle<Mesh> = server.load(path);
        server.update_loading();

        assert_eq!(server.state(&mesh), AssetLoadState::Failed);
        assert!(matches!(
            server.error_by_id(mesh.id()),
            Some(AssetError::Decode { message }) if message.contains(expected_message)
        ));
        assert!(server
            .events()
            .iter()
            .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == mesh.id())));
    }
}

#[test]
fn audio_load_reaches_ready_without_renderer_upload() {
    let io = MemoryAssetIo::new().with_file("audio/click.audio", audio_bytes());
    let mut server = server_with_io(io);

    let audio: Handle<AudioClip> = server.load("audio/click.audio");
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 48000);
    assert_eq!(loaded.channels, 2);
    assert_eq!(loaded.duration_seconds, 2.0 / 48000.0);
    assert_eq!(loaded.samples, AudioSamples::I16(vec![0, 1000, -1000, 0]));
    assert!(!loaded.streaming);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == audio.id())));
}

#[test]
fn wav_audio_load_reaches_ready_without_renderer_upload() {
    let wav = wav_pcm16_bytes(44_100, 2, &[0, 1000, -1000, 500]);
    let io = MemoryAssetIo::new().with_file("audio/click.wav", wav);
    let mut server = server_with_io(io);

    let audio: Handle<AudioClip> = server.load("audio/click.wav");
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 44_100);
    assert_eq!(loaded.channels, 2);
    assert_eq!(loaded.duration_seconds, 2.0 / 44_100.0);
    assert_eq!(loaded.samples, AudioSamples::I16(vec![0, 1000, -1000, 500]));
    assert!(!loaded.streaming);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == audio.id())));
}

#[test]
fn wav_float32_audio_load_reaches_ready_without_renderer_upload() {
    let wav = wav_float32_bytes(48_000, 2, &[0.0, 0.5, -0.25, 1.0]);
    let io = MemoryAssetIo::new().with_file("audio/tone.wav", wav);
    let mut server = server_with_io(io);

    let audio: Handle<AudioClip> = server.load("audio/tone.wav");
    server.update_loading();

    assert!(server.is_ready(&audio));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&audio).unwrap();
    assert_eq!(loaded.sample_rate, 48_000);
    assert_eq!(loaded.channels, 2);
    assert_eq!(loaded.duration_seconds, 2.0 / 48_000.0);
    assert_eq!(
        loaded.samples,
        AudioSamples::F32(vec![0.0, 0.5, -0.25, 1.0])
    );
    assert!(!loaded.streaming);
}

#[test]
fn invalid_audio_payload_fails_with_decode_error_and_event() {
    let io = MemoryAssetIo::new().with_file("audio/broken.audio", b"not audio".to_vec());
    let mut server = server_with_io(io);

    let audio: Handle<AudioClip> = server.load("audio/broken.audio");
    server.update_loading();

    assert_eq!(server.state(&audio), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(audio.id()),
        Some(AssetError::Decode { message })
            if message.contains("audio source must start with NGA_AUDIO_V1")
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == audio.id())));
}

#[test]
fn invalid_wav_audio_payload_fails_with_decode_error_and_event() {
    let io =
        MemoryAssetIo::new().with_file("audio/broken.wav", b"RIFF\x04\x00\x00\x00WAVE".to_vec());
    let mut server = server_with_io(io);

    let audio: Handle<AudioClip> = server.load("audio/broken.wav");
    server.update_loading();

    assert_eq!(server.state(&audio), AssetLoadState::Failed);
    assert!(matches!(
        server.error_by_id(audio.id()),
        Some(AssetError::Decode { message })
            if message.contains("WAV audio source missing fmt chunk")
    ));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == audio.id())));
}

#[test]
fn skeleton_load_reaches_ready_without_renderer_upload() {
    let io = MemoryAssetIo::new().with_file("skeletons/hero.skeleton", skeleton_bytes());
    let mut server = server_with_io(io);

    let skeleton: Handle<Skeleton> = server.load("skeletons/hero.skeleton");
    server.update_loading();

    assert!(server.is_ready(&skeleton));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&skeleton).unwrap();
    assert_eq!(loaded.bones.len(), 2);
    assert_eq!(loaded.bones[0].name, "Root");
    assert_eq!(loaded.bones[0].parent, None);
    assert_eq!(loaded.bones[1].name, "Spine");
    assert_eq!(loaded.bones[1].parent, Some(0));
    assert_eq!(loaded.inverse_bind_poses.len(), 2);
    assert_eq!(loaded.inverse_bind_poses[0][0][0], 1.0);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == skeleton.id())));
}

#[test]
fn skeleton_load_preserves_explicit_bind_pose_matrices() {
    let source = b"NGA_SKELETON_V1\nbone=Root;bind=1,0,0,2,0,1,0,3,0,0,1,4,0,0,0,1;inverse_bind=1,0,0,-2,0,1,0,-3,0,0,1,-4,0,0,0,1\n".to_vec();
    let io = MemoryAssetIo::new().with_file("skeletons/bind_pose.skeleton", source);
    let mut server = server_with_io(io);

    let skeleton: Handle<Skeleton> = server.load("skeletons/bind_pose.skeleton");
    server.update_loading();

    assert!(server.is_ready(&skeleton));
    let loaded = server.get(&skeleton).unwrap();
    assert_eq!(loaded.bones[0].local_bind_transform[0][3], 2.0);
    assert_eq!(loaded.bones[0].local_bind_transform[1][3], 3.0);
    assert_eq!(loaded.bones[0].local_bind_transform[2][3], 4.0);
    assert_eq!(loaded.inverse_bind_poses[0][0][3], -2.0);
    assert_eq!(loaded.inverse_bind_poses[0][1][3], -3.0);
    assert_eq!(loaded.inverse_bind_poses[0][2][3], -4.0);
}

#[test]
fn invalid_skeleton_payload_fails_with_decode_error_and_event() {
    assert_skeleton_decode_error(
        "NGA_SKELETON_V1\nbone=Spine;parent=0\n",
        "does not reference an earlier bone",
    );
    assert_skeleton_decode_error(
        "NGA_SKELETON_V1\nbone=Root;bind=1,0\n",
        "skeleton bind on line 2 must contain 16 values",
    );
    assert_skeleton_decode_error(
        "NGA_SKELETON_V1\nbone=Root\nbone=Root;parent=0\n",
        "duplicates an earlier bone name",
    );
    assert_skeleton_decode_error(
        "NGA_SKELETON_V1\nbone=Root;bind=1,0,0,2,0,1,0,3,0,0,1,4,0,0,0,1;inverse_bind=1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1\n",
        "inverse_bind on line 2 does not invert bind pose for bone `Root`",
    );
    assert_skeleton_decode_error(
        "NGA_SKELETON_V1\nbone=Root;bind=1,0,0,2,0,1,0,0,0,0,1,0,0,0,0,1;inverse_bind=1,0,0,-2,0,1,0,0,0,0,1,0,0,0,0,1\nbone=Child;parent=0;bind=1,0,0,3,0,1,0,0,0,0,1,0,0,0,0,1;inverse_bind=1,0,0,-3,0,1,0,0,0,0,1,0,0,0,0,1\n",
        "inverse_bind on line 3 does not invert bind pose for bone `Child`",
    );
}

#[test]
fn animation_load_reaches_ready_without_renderer_upload() {
    let io = MemoryAssetIo::new().with_file("animations/idle.animation", animation_bytes());
    let mut server = server_with_io(io);

    let animation: Handle<AnimationClip> = server.load("animations/idle.animation");
    server.update_loading();

    assert!(server.is_ready(&animation));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&animation).unwrap();
    assert_eq!(loaded.duration, 1.0);
    assert_eq!(loaded.ticks_per_second, 60.0);
    assert_eq!(loaded.tracks.len(), 1);
    assert_eq!(
        loaded.tracks[0].target,
        AnimationTarget::BoneName("Root".to_owned())
    );
    assert_eq!(loaded.tracks[0].translations[0].value, [0.0, 0.0, 0.0]);
    assert_eq!(loaded.tracks[0].rotations[0].value, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(loaded.tracks[0].scales[0].value, [1.0, 1.0, 1.0]);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == animation.id())));
}

#[test]
fn invalid_animation_payload_fails_with_decode_error_and_event() {
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntranslation=0:0,0,0\n",
        "animation translation on line 4 has no track",
    );
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=NaN\nticks_per_second=60\ntrack=bone:Root\ntranslation=0:0,0,0\n",
        "animation duration on line 2 must be finite",
    );
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=bone:Root\ntranslation=-0.1:0,0,0\n",
        "animation translation keyframe time on line 5 must be non-negative",
    );
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=bone:Root\ntranslation=1.25:0,0,0\n",
        "animation translation keyframe 0 in track 0 has time 1.25 beyond duration 1",
    );
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=bone:Root\ntranslation=0.5:0,0,0\ntranslation=0.25:1,0,0\n",
        "animation translation keyframes in track 0 must be sorted by time",
    );
}

#[test]
fn invalid_animation_track_shape_fails_with_decode_error_and_event() {
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=bone:Root\n",
        "animation track 0 must contain at least one translation, rotation, or scale keyframe",
    );
    assert_animation_decode_error(
        "NGA_ANIMATION_V1\nduration=1\nticks_per_second=60\ntrack=node:Root\ntranslation=0:0,0,0\ntrack=node:Root\nrotation=0:0,0,0,1\n",
        "animation track 1 duplicates target `node:Root` from track 0",
    );
}

#[test]
fn font_load_reaches_ready_without_renderer_upload() {
    let io = MemoryAssetIo::new().with_file("fonts/debug.font", font_bytes());
    let mut server = server_with_io(io);

    let font: Handle<Font> = server.load("fonts/debug.font");
    server.update_loading();

    assert!(server.is_ready(&font));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&font).unwrap();
    assert_eq!(loaded.family_name, "Debug Sans");
    let FontData::Bitmap(bitmap) = &loaded.data else {
        panic!("expected bitmap font");
    };
    assert_eq!(bitmap.glyphs.len(), 1);
    assert_eq!(bitmap.glyphs[0].codepoint, 'A');
    assert_eq!((bitmap.glyphs[0].width, bitmap.glyphs[0].height), (2, 1));
    assert_eq!(bitmap.glyphs[0].bitmap, vec![0, 255]);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == font.id())));
}

#[test]
fn invalid_font_payload_fails_with_decode_error_and_event() {
    assert_font_decode_error(
        "NGA_FONT_V1\nfamily=Debug\nglyph=char=A;size=2x2;bitmap=0,255\n",
        "expected 4",
    );
}

#[test]
fn physics_mesh_load_reaches_ready_without_renderer_upload() {
    let io = MemoryAssetIo::new().with_file("physics/hero.physics", physics_mesh_bytes());
    let mut server = server_with_io(io);

    let mesh: Handle<PhysicsMesh> = server.load("physics/hero.physics");
    server.update_loading();

    assert!(server.is_ready(&mesh));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&mesh).unwrap();
    assert_eq!(loaded.kind, PhysicsMeshKind::TriMesh);
    assert_eq!(loaded.vertices.len(), 3);
    assert_eq!(loaded.indices, vec![[0, 1, 2]]);
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == mesh.id())));
}

#[test]
fn invalid_physics_mesh_payload_fails_with_decode_error_and_event() {
    assert_physics_mesh_decode_error(
        "NGA_PHYSICS_MESH_V1\nkind=trimesh\nv 0 0 0\ni 0 1 2\n",
        "physics mesh index 1 references missing vertex",
    );
}

#[test]
fn texture_load_reaches_ready_after_renderer_upload_handoff() {
    let io = MemoryAssetIo::new().with_file("textures/checker.texture", texture_bytes(2, 2, 255));
    let mut server = server_with_io(io);

    let texture: Handle<Texture> = server.load("textures/checker.texture");
    assert_eq!(server.state(&texture), AssetLoadState::Queued);

    server.update_loading();
    assert_eq!(server.state(&texture), AssetLoadState::UploadingGpu);
    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].id, texture.id());
    assert_eq!(uploads[0].kind, GpuUploadKind::Texture);

    server.finish_gpu_uploads(vec![GpuUploadResult::ok(
        texture.id(),
        GpuResourceHandle(7),
    )]);

    assert!(server.is_ready(&texture));
    let loaded = server.get(&texture).unwrap();
    assert_eq!((loaded.width, loaded.height), (2, 2));
    assert_eq!(loaded.gpu, Some(GpuResourceHandle(7)));
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == texture.id())));
}

#[test]
fn material_load_waits_for_shader_and_texture_dependencies() {
    let mut io = MemoryAssetIo::new();
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 128));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    let mut server = server_with_io(io);

    let material: Handle<Material> = server.load("materials/hero.material");
    server.update_loading();

    assert_eq!(
        server.state(&material),
        AssetLoadState::WaitingForDependencies
    );
    let material_id = material.id();
    let dependencies = server.dependency_graph().direct_dependencies(material_id);
    assert_eq!(dependencies.len(), 2);
    let shader_id = server
        .id_from_path(&AssetPath::parse("shaders/pbr.wgsl"))
        .unwrap();
    let texture_id = server
        .id_from_path(&AssetPath::parse("textures/albedo.texture"))
        .unwrap();
    assert!(dependencies.contains(&shader_id));
    assert!(dependencies.contains(&texture_id));

    finish_all_uploads(&mut server);
    assert_eq!(server.state(&material), AssetLoadState::UploadingGpu);
    finish_all_uploads(&mut server);

    assert!(server.is_ready_with_dependencies(&material));
    let loaded = server.get(&material).unwrap();
    assert_eq!(loaded.name.as_deref(), Some("hero"));
    assert_eq!(loaded.shader.as_ref().unwrap().id(), shader_id);
    assert_eq!(loaded.textures[0].texture.id(), texture_id);
}

#[test]
fn scene_load_waits_for_dependency_paths_and_exposes_handles() {
    let mut io = MemoryAssetIo::new();
    io.insert("meshes/tri.mesh", mesh_bytes());
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 128));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    io.insert("scenes/level.scene", scene_bytes());
    let mut server = server_with_io(io);

    let scene: Handle<SceneAsset> = server.load("scenes/level.scene");
    server.update_loading();

    assert_eq!(server.state(&scene), AssetLoadState::WaitingForDependencies);
    let mesh_id = server
        .id_from_path(&AssetPath::parse("meshes/tri.mesh"))
        .unwrap();
    let material_id = server
        .id_from_path(&AssetPath::parse("materials/hero.material"))
        .unwrap();
    assert_eq!(
        server.dependency_graph().direct_dependencies(scene.id()),
        &[mesh_id, material_id]
    );

    finish_all_uploads(&mut server);
    assert_eq!(server.state(&scene), AssetLoadState::WaitingForDependencies);
    finish_all_uploads(&mut server);

    assert!(server.is_ready_with_dependencies(&scene));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&scene).unwrap();
    assert_eq!(loaded.name, "level");
    assert_eq!(loaded.entities.len(), 2);
    assert_eq!(loaded.entities[0].name.as_deref(), Some("Root"));
    assert_eq!(loaded.entities[0].components[0].type_name, "Transform");
    assert_eq!(loaded.entities[0].components[0].data, b"translation=0,0,0");
    assert_eq!(loaded.entities[1].name.as_deref(), Some("Hero"));
    assert_eq!(loaded.entities[1].parent, Some(0));
    assert_eq!(loaded.dependencies.len(), 2);
    assert!(loaded.dependencies.iter().any(|dependency| {
        dependency.id() == mesh_id
            && dependency.asset_type() == Mesh::TYPE_ID
            && dependency.strength() == HandleStrength::Weak
    }));
    assert!(loaded.dependencies.iter().any(|dependency| {
        dependency.id() == material_id
            && dependency.asset_type() == Material::TYPE_ID
            && dependency.strength() == HandleStrength::Weak
    }));
    let mut visited = Vec::new();
    loaded.visit_dependencies(&mut |dependency| visited.push(dependency.id()));
    assert_eq!(visited, vec![mesh_id, material_id]);
}

#[test]
fn invalid_scene_payload_fails_with_decode_error_and_event() {
    assert_scene_decode_error(
        "NGA_SCENE_V1\nname=broken\ncomponent=Transform|translation=0,0,0\n",
        "scene component on line 3 has no entity",
    );
}

#[test]
fn prefab_load_waits_for_dependency_paths_and_exposes_handles() {
    let mut io = MemoryAssetIo::new();
    io.insert("meshes/tri.mesh", mesh_bytes());
    io.insert("textures/albedo.texture", texture_bytes(1, 1, 128));
    io.insert("shaders/pbr.wgsl", "@fragment fn main() {}");
    io.insert(
        "materials/hero.material",
        "name=hero\nshader=shaders/pbr.wgsl\ntexture.albedo=textures/albedo.texture\nbase_color=1,1,1,1\n",
    );
    io.insert("prefabs/hero.prefab", prefab_bytes());
    let mut server = server_with_io(io);

    let prefab: Handle<Prefab> = server.load("prefabs/hero.prefab");
    server.update_loading();

    assert_eq!(
        server.state(&prefab),
        AssetLoadState::WaitingForDependencies
    );
    let mesh_id = server
        .id_from_path(&AssetPath::parse("meshes/tri.mesh"))
        .unwrap();
    let material_id = server
        .id_from_path(&AssetPath::parse("materials/hero.material"))
        .unwrap();
    assert_eq!(
        server.dependency_graph().direct_dependencies(prefab.id()),
        &[mesh_id, material_id]
    );

    finish_all_uploads(&mut server);
    assert_eq!(
        server.state(&prefab),
        AssetLoadState::WaitingForDependencies
    );
    finish_all_uploads(&mut server);

    assert!(server.is_ready_with_dependencies(&prefab));
    assert!(server.drain_gpu_uploads().next().is_none());
    let loaded = server.get(&prefab).unwrap();
    assert_eq!(loaded.root.name.as_deref(), Some("Hero"));
    assert_eq!(loaded.root.components[0].type_name, "Transform");
    assert_eq!(loaded.root.components[0].data, b"translation=0,0,0");
    assert_eq!(loaded.children.len(), 1);
    assert_eq!(loaded.children[0].name.as_deref(), Some("Weapon"));
    assert_eq!(loaded.children[0].parent, Some(0));
    assert_eq!(loaded.dependencies.len(), 2);
    assert!(loaded.dependencies.iter().any(|dependency| {
        dependency.id() == mesh_id
            && dependency.asset_type() == Mesh::TYPE_ID
            && dependency.strength() == HandleStrength::Weak
    }));
    assert!(loaded.dependencies.iter().any(|dependency| {
        dependency.id() == material_id
            && dependency.asset_type() == Material::TYPE_ID
            && dependency.strength() == HandleStrength::Weak
    }));
    let mut visited = Vec::new();
    loaded.visit_dependencies(&mut |dependency| visited.push(dependency.id()));
    assert_eq!(visited, vec![mesh_id, material_id]);
}

#[test]
fn invalid_prefab_payload_fails_with_decode_error_and_event() {
    assert_prefab_decode_error(
        "NGA_PREFAB_V1\ncomponent=Transform|translation=0,0,0\n",
        "prefab component on line 2 has no entity",
    );
}

#[test]
fn missing_path_fails_with_visible_state_error_and_event() {
    let io = MemoryAssetIo::new();
    let mut server = server_with_io(io);

    let texture: Handle<Texture> = server.load("textures/missing.texture");
    server.update_loading();

    assert_eq!(server.state(&texture), AssetLoadState::Failed);
    match server.error_by_id(texture.id()) {
        Some(AssetError::Io { message }) => {
            assert!(message.contains("read"));
            assert!(message.contains("textures/missing.texture"));
        }
        other => panic!("expected io error, got {other:?}"),
    }
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Failed { id, .. } if *id == texture.id())));
}

#[test]
fn unload_by_id_removes_ready_asset_and_emits_event() {
    let io = MemoryAssetIo::new().with_file("textures/checker.texture", texture_bytes(1, 1, 9));
    let mut server = server_with_io(io);
    let texture: Handle<Texture> = server.load("textures/checker.texture");
    server.update_loading();
    finish_all_uploads(&mut server);
    assert!(server.is_ready(&texture));

    server.unload_by_id(texture.id()).unwrap();

    assert_eq!(server.state(&texture), AssetLoadState::Unloaded);
    assert!(server.get(&texture).is_none());
    assert!(server
        .events()
        .iter()
        .any(|event| matches!(event, AssetEvent::Unloaded { id } if *id == texture.id())));
}

#[test]
fn event_cursor_only_returns_new_events() {
    let io = MemoryAssetIo::new().with_file("textures/checker.texture", texture_bytes(1, 1, 1));
    let mut server = server_with_io(io);
    let mut cursor = AssetEventCursor::default();

    let texture: Handle<Texture> = server.load("textures/checker.texture");
    assert_eq!(server.events_since(&mut cursor).len(), 1);
    server.update_loading();
    assert!(!server.events_since(&mut cursor).is_empty());
    assert!(server.events_since(&mut cursor).is_empty());

    let uploads = server.drain_gpu_uploads().collect::<Vec<_>>();
    server.finish_gpu_uploads(
        uploads
            .into_iter()
            .map(|upload| GpuUploadResult::ok(upload.id, GpuResourceHandle(42))),
    );
    assert!(server
        .events_since(&mut cursor)
        .iter()
        .any(|event| matches!(event, AssetEvent::Ready { id } if *id == texture.id())));
}
